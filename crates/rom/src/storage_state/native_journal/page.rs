use super::*;
pub fn journal_page(
    reader: &impl JournalRead,
    kind: &str,
    after: Option<&JournalCursor>,
    max_rows: usize,
    max_bytes: usize,
) -> Result<JournalPage> {
    if max_rows == 0 || max_bytes == 0 {
        return Err(Error::TooLarge);
    }
    let fence = reader.coherence();
    let header = reader.header()?;
    let parts = header.parts();
    let position = after.map_or(0, |c| c.position);
    if position < parts.floor
        || position > parts.head
        || after.is_some_and(|c| c.kind != kind || c.generation != parts.generation)
    {
        return Err(Error::HistoryGap);
    }
    let mut cursor = JournalCursor {
        generation: parts.generation,
        kind: kind.into(),
        position,
    };
    let mut events = vec![];
    let mut bytes = 0usize;
    let mut position = position;
    while position < parts.head {
        position = position.checked_add(1).ok_or(Error::TooLarge)?;
        let entry = reader.entry(position)?.ok_or(Error::Storage)?;
        read::validate_entry(&header, position, &entry)?;
        if entry.event.row.key.kind == kind {
            let next = bytes
                .checked_add(entry.encoded_bytes)
                .ok_or(Error::TooLarge)?;
            if next > max_bytes {
                if events.is_empty() {
                    return Err(Error::TooLarge);
                }
                break;
            }
            if events.len() == max_rows {
                break;
            }
            bytes = next;
            events.push(entry.event);
        }
        cursor.position = position;
    }
    if !reader.coherence().same_context(&fence) {
        return Err(Error::Conflict);
    }
    Ok(JournalPage { events, cursor })
}
