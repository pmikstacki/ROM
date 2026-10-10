//! Physical layout required by the versioned index profile.
use rom::{Error, Result};
use rusqlite::{Connection, types::ValueRef};

pub(super) fn validate(c: &Connection) -> Result<()> {
    table(
        c,
        "query_keys",
        true,
        &[
            ("kind", "TEXT", true, 1),
            ("field", "TEXT", true, 3),
            ("id", "TEXT", true, 2),
            ("encoded", "BLOB", true, 0),
        ],
    )?;
    table(
        c,
        "query_kinds",
        true,
        &[
            ("kind", "TEXT", true, 1),
            ("row_count", "INTEGER", true, 0),
            ("row_bytes", "INTEGER", true, 0),
            ("canonical_row_bytes", "INTEGER", true, 0),
            ("live_count", "INTEGER", true, 0),
            ("generation", "BLOB", true, 0),
        ],
    )?;
    table(
        c,
        "query_profile",
        false,
        &[
            ("id", "INTEGER", false, 1),
            ("encoding_version", "INTEGER", true, 0),
            ("profile_version", "INTEGER", true, 0),
            ("store", "TEXT", true, 0),
        ],
    )?;
    indexes(
        c,
        "query_keys",
        Some((&["kind", "id", "field"], &["encoded"])),
        true,
    )?;
    indexes(
        c,
        "query_kinds",
        Some((
            &["kind"],
            &[
                "row_count",
                "row_bytes",
                "canonical_row_bytes",
                "live_count",
                "generation",
            ],
        )),
        false,
    )?;
    indexes(c, "query_profile", None, false)
}

pub(crate) fn table(
    c: &Connection,
    name: &str,
    without_rowid: bool,
    columns: &[(&str, &str, bool, i64)],
) -> Result<()> {
    let mut statement=c.prepare("SELECT name,type,\"notnull\",dflt_value,pk,hidden FROM pragma_table_xinfo(?) ORDER BY cid").map_err(|_|Error::Storage)?;
    let mut rows = statement.query([name]).map_err(|_| Error::Storage)?;
    for (column, kind, required, primary) in columns {
        let row = rows
            .next()
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        let text = |i| {
            row.get_ref(i)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)
        };
        let integer = |i| {
            row.get_ref(i)
                .map_err(|_| Error::Storage)?
                .as_i64()
                .map_err(|_| Error::Storage)
        };
        if text(0)? != *column
            || !text(1)?.eq_ignore_ascii_case(kind)
            || integer(2)? != i64::from(*required)
            || !matches!(row.get_ref(3).map_err(|_| Error::Storage)?, ValueRef::Null)
            || integer(4)? != *primary
            || integer(5)? != 0
        {
            return Err(Error::Storage);
        }
    }
    if rows.next().map_err(|_| Error::Storage)?.is_some() {
        return Err(Error::Storage);
    }
    let layout: (i64, i64, String) = c
        .query_row(
            "SELECT wr,ncol,type FROM pragma_table_list WHERE schema='main' AND name=?",
            [name],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| Error::Storage)?;
    if layout.0 != i64::from(without_rowid)
        || layout.1 != i64::try_from(columns.len()).map_err(|_| Error::TooLarge)?
        || layout.2 != "table"
    {
        return Err(Error::Storage);
    }
    Ok(())
}

pub(crate) type Primary<'a> = (&'a [&'a str], &'a [&'a str]);
pub(crate) fn indexes(
    c: &Connection,
    table: &str,
    primary: Option<Primary<'_>>,
    secondary: bool,
) -> Result<()> {
    let mut s = c
        .prepare("SELECT name,\"unique\",origin,partial FROM pragma_index_list(?)")
        .map_err(|_| Error::Storage)?;
    let mut rows = s.query([table]).map_err(|_| Error::Storage)?;
    let (mut found_primary, mut found_secondary) = (false, false);
    while let Some(row) = rows.next().map_err(|_| Error::Storage)? {
        let name = row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let unique: i64 = row.get(1).map_err(|_| Error::Storage)?;
        let origin = row
            .get_ref(2)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?;
        let partial: i64 = row.get(3).map_err(|_| Error::Storage)?;
        if partial != 0 {
            return Err(Error::Storage);
        }
        match origin {
            "pk" if !found_primary && unique == 1 => {
                let (keys, auxiliary) = primary.ok_or(Error::Storage)?;
                index_columns(c, name, keys, auxiliary)?;
                found_primary = true;
            }
            "c" if secondary && !found_secondary && unique == 0 && name == "query_keys_value" => {
                index_columns(c, name, &["kind", "field", "encoded", "id"], &[])?;
                found_secondary = true;
            }
            _ => return Err(Error::Storage),
        }
    }
    if found_primary != primary.is_some() || found_secondary != secondary {
        return Err(Error::Storage);
    }
    Ok(())
}

fn index_columns(c: &Connection, name: &str, keys: &[&str], auxiliary: &[&str]) -> Result<()> {
    let mut s = c
        .prepare("SELECT name,\"desc\",coll,\"key\" FROM pragma_index_xinfo(?) ORDER BY seqno")
        .map_err(|_| Error::Storage)?;
    let mut rows = s.query([name]).map_err(|_| Error::Storage)?;
    for (expected, key) in keys
        .iter()
        .map(|name| (name, 1))
        .chain(auxiliary.iter().map(|name| (name, 0)))
    {
        let row = rows
            .next()
            .map_err(|_| Error::Storage)?
            .ok_or(Error::Storage)?;
        if row
            .get_ref(0)
            .map_err(|_| Error::Storage)?
            .as_str()
            .map_err(|_| Error::Storage)?
            != *expected
            || row.get::<_, i64>(1).map_err(|_| Error::Storage)? != 0
            || row
                .get_ref(2)
                .map_err(|_| Error::Storage)?
                .as_str()
                .map_err(|_| Error::Storage)?
                != "BINARY"
            || row.get::<_, i64>(3).map_err(|_| Error::Storage)? != key
        {
            return Err(Error::Storage);
        }
    }
    if rows.next().map_err(|_| Error::Storage)?.is_some() {
        return Err(Error::Storage);
    }
    Ok(())
}
