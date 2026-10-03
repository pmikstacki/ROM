//! File space and Linux process RSS; neither is an allocator-specific measurement.
use crate::Failure;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub(crate) struct Files {
    main_bytes: Option<u64>,
    wal_bytes: Option<u64>,
    shm_bytes: Option<u64>,
}
pub(crate) fn files(path: &Path) -> Files {
    let length = |path: &Path| std::fs::metadata(path).ok().map(|m| m.len());
    let sidecar = |suffix: &str| {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        length(Path::new(&name))
    };
    Files {
        main_bytes: length(path),
        wal_bytes: sidecar("-wal"),
        shm_bytes: sidecar("-shm"),
    }
}
#[derive(Serialize)]
pub(crate) struct Space {
    files: Files,
    sqlite_page_count: u64,
    sqlite_page_bytes: u64,
    sqlite_free_pages: u64,
    query_index_page_bytes: Option<u64>,
}
pub(crate) fn capture(path: &Path) -> Result<Space, Failure> {
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let number = |sql: &str| -> Result<u64, Failure> {
        Ok(u64::try_from(
            c.query_row(sql, [], |r| r.get::<_, i64>(0))?,
        )?)
    };
    let index=c.query_row("SELECT COALESCE(SUM(pgsize),0) FROM dbstat WHERE name IN ('query_keys','query_keys_value','query_kinds','query_profile')",[],|r|r.get::<_,i64>(0)).ok().and_then(|n|u64::try_from(n).ok());
    Ok(Space {
        files: files(path),
        sqlite_page_count: number("PRAGMA page_count")?,
        sqlite_page_bytes: number("PRAGMA page_size")?,
        sqlite_free_pages: number("PRAGMA freelist_count")?,
        query_index_page_bytes: index,
    })
}

#[cfg(feature = "queries")]
#[derive(Serialize)]
pub(crate) struct Rss {
    current_kib: Option<u64>,
    process_high_water_kib: Option<u64>,
}
#[cfg(feature = "queries")]
pub(crate) fn rss() -> Rss {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        text.lines().find_map(|line| {
            line.strip_prefix(name)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
    };
    Rss {
        current_kib: field("VmRSS:"),
        process_high_water_kib: field("VmHWM:"),
    }
}
