//! Checked index counters and physical profile metadata.
use rom::{Error, Result, Row};
use rusqlite::{Connection, OptionalExtension, params};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Counters {
    pub rows: u64,
    pub bytes: u64,
    pub canonical_bytes: u64,
    pub live: u64,
    pub generation: u64,
}

fn delta(total: u64, before: u64, after: u64) -> Result<u64> {
    let value = total
        .checked_sub(before)
        .ok_or(Error::Storage)?
        .checked_add(after)
        .ok_or(Error::TooLarge)?;
    if value > i64::MAX as u64 {
        return Err(Error::TooLarge);
    }
    Ok(value)
}

impl Counters {
    pub(super) fn replace(
        &self,
        old: Option<&Row>,
        new: &Row,
        old_bytes: Option<usize>,
    ) -> Result<Self> {
        if old.is_some() != old_bytes.is_some() {
            return Err(Error::Storage);
        }
        let canonical = |row: &Row| -> Result<u64> {
            u64::try_from(serde_json::to_vec(row).map_err(|_| Error::Storage)?.len())
                .map_err(|_| Error::TooLarge)
        };
        let old_canonical = old.map(canonical).transpose()?.unwrap_or(0);
        let new_canonical = canonical(new)?;
        Ok(Self {
            rows: delta(self.rows, u64::from(old.is_some()), 1)?,
            bytes: delta(
                self.bytes,
                u64::try_from(old_bytes.unwrap_or(0)).map_err(|_| Error::TooLarge)?,
                new_canonical,
            )?,
            canonical_bytes: delta(self.canonical_bytes, old_canonical, new_canonical)?,
            live: delta(
                self.live,
                u64::from(old.is_some_and(|r| r.value.is_some())),
                u64::from(new.value.is_some()),
            )?,
            generation: self.generation.checked_add(1).ok_or(Error::TooLarge)?,
        })
    }

    pub(super) fn add(&mut self, row: &Row, raw_bytes: usize) -> Result<()> {
        let canonical = serde_json::to_vec(row).map_err(|_| Error::Storage)?.len();
        self.rows = delta(self.rows, 0, 1)?;
        self.bytes = delta(
            self.bytes,
            0,
            u64::try_from(raw_bytes).map_err(|_| Error::TooLarge)?,
        )?;
        self.canonical_bytes = delta(
            self.canonical_bytes,
            0,
            u64::try_from(canonical).map_err(|_| Error::TooLarge)?,
        )?;
        self.live = delta(self.live, 0, u64::from(row.value.is_some()))?;
        Ok(())
    }
}

pub(super) fn load(c: &Connection, kind: &str) -> Result<Option<Counters>> {
    load_admitted(c, kind, None)
}
pub(super) fn load_admitted(
    c: &Connection,
    kind: &str,
    reader: Option<&crate::native_work::Reader<'_>>,
) -> Result<Option<Counters>> {
    let mut s=c.prepare("SELECT row_count,row_bytes,canonical_row_bytes,live_count,generation FROM query_kinds WHERE kind=?").map_err(|_|Error::Storage)?;
    let mut rows = s.query([kind]).map_err(|_| Error::Storage)?;
    let Some(row) = rows.next().map_err(|_| Error::Storage)? else {
        return Ok(None);
    };
    if let Some(reader) = reader {
        let generation_bytes = match row.get_ref(4).map_err(|_| Error::Storage)? {
            rusqlite::types::ValueRef::Blob(raw) | rusqlite::types::ValueRef::Text(raw) => {
                raw.len()
            }
            rusqlite::types::ValueRef::Integer(_) | rusqlite::types::ValueRef::Real(_) => 8,
            rusqlite::types::ValueRef::Null => 0,
        };
        reader.charge(
            kind.len()
                .checked_add(32)
                .and_then(|n| n.checked_add(generation_bytes))
                .ok_or(Error::TooLarge)?,
            1,
        )?;
    }
    let result = decode(row, 0)?;
    if rows.next().map_err(|_| Error::Storage)?.is_some() {
        return Err(Error::Storage);
    }
    Ok(Some(result))
}

pub(super) fn decode(row: &rusqlite::Row<'_>, offset: usize) -> Result<Counters> {
    let number = |i| -> Result<u64> {
        u64::try_from(
            row.get_ref(i)
                .map_err(|_| Error::Storage)?
                .as_i64()
                .map_err(|_| Error::Storage)?,
        )
        .map_err(|_| Error::Storage)
    };
    let generation = row
        .get_ref(offset + 4)
        .map_err(|_| Error::Storage)?
        .as_blob()
        .map_err(|_| Error::Storage)?;
    let result = Counters {
        rows: number(offset)?,
        bytes: number(offset + 1)?,
        canonical_bytes: number(offset + 2)?,
        live: number(offset + 3)?,
        generation: u64::from_be_bytes(generation.try_into().map_err(|_| Error::Storage)?),
    };
    if result.live > result.rows {
        return Err(Error::Storage);
    }
    Ok(result)
}

pub(super) fn insert(c: &Connection, kind: &str, counts: &Counters) -> Result<()> {
    c.execute("INSERT INTO query_kinds(kind,row_count,row_bytes,canonical_row_bytes,live_count,generation) VALUES (?,?,?,?,?,?)",
        params![kind,i64::try_from(counts.rows).map_err(|_|Error::TooLarge)?,i64::try_from(counts.bytes).map_err(|_|Error::TooLarge)?,i64::try_from(counts.canonical_bytes).map_err(|_|Error::TooLarge)?,i64::try_from(counts.live).map_err(|_|Error::TooLarge)?,counts.generation.to_be_bytes().as_slice()]).map_err(|_|Error::NotCommitted)?;
    Ok(())
}

pub(super) fn update(c: &Connection, kind: &str, counts: &Counters) -> Result<()> {
    let changed=c.execute("UPDATE query_kinds SET row_count=?,row_bytes=?,canonical_row_bytes=?,live_count=?,generation=? WHERE kind=?",
        params![i64::try_from(counts.rows).map_err(|_|Error::TooLarge)?,i64::try_from(counts.bytes).map_err(|_|Error::TooLarge)?,i64::try_from(counts.canonical_bytes).map_err(|_|Error::TooLarge)?,i64::try_from(counts.live).map_err(|_|Error::TooLarge)?,counts.generation.to_be_bytes().as_slice(),kind]).map_err(|_|Error::NotCommitted)?;
    if changed != 1 {
        return Err(Error::Storage);
    }
    Ok(())
}

pub(super) fn profile(c: &Connection) -> Result<String> {
    let mut s = c
        .prepare("SELECT id,encoding_version,profile_version,store FROM query_profile")
        .map_err(|_| Error::Storage)?;
    let mut rows = s.query([]).map_err(|_| Error::Storage)?;
    let row = rows
        .next()
        .map_err(|_| Error::Storage)?
        .ok_or(Error::Storage)?;
    for (i, expected) in [
        1,
        i64::from(rom::QUERY_ENCODING_VERSION),
        i64::from(rom::QUERY_PROFILE_VERSION),
    ]
    .into_iter()
    .enumerate()
    {
        if row
            .get_ref(i)
            .map_err(|_| Error::Storage)?
            .as_i64()
            .map_err(|_| Error::Storage)?
            != expected
        {
            return Err(Error::Storage);
        }
    }
    let store = row
        .get_ref(3)
        .map_err(|_| Error::Storage)?
        .as_str()
        .map_err(|_| Error::Storage)?;
    if store.is_empty() || store.len() > 256 {
        return Err(Error::Storage);
    }
    let store = store.to_owned();
    if rows.next().map_err(|_| Error::Storage)?.is_some() {
        return Err(Error::Storage);
    }
    Ok(store)
}

pub(super) fn descriptor(c: &Connection, kind: &str) -> Result<rom::Descriptor> {
    let text: Option<String> = c
        .query_row("SELECT data FROM schemas WHERE kind=?", [kind], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| Error::Storage)?;
    let descriptor: rom::Descriptor =
        serde_json::from_str(&text.ok_or(Error::Unregistered)?).map_err(|_| Error::Storage)?;
    if descriptor.kind != kind || descriptor.canonical().map_err(|_| Error::Storage)? != descriptor
    {
        return Err(Error::Storage);
    }
    Ok(descriptor)
}

pub(super) fn reset_profile(c: &Connection) -> Result<()> {
    c.execute("DELETE FROM query_profile", [])
        .map_err(|_| Error::NotCommitted)?;
    c.execute("INSERT INTO query_profile(id,encoding_version,profile_version,store) VALUES(1,?,?,lower(hex(randomblob(16))))",params![rom::QUERY_ENCODING_VERSION,rom::QUERY_PROFILE_VERSION]).map_err(|_|Error::NotCommitted)?;
    Ok(())
}
