//! Compile only against the independently identified previous release source.
use std::path::Path;
fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    assert_eq!(arguments.len(), 4, "usage: old-reader-probe sqlite|redb PATH accept|reject");
    let path = Path::new(&arguments[2]);
    assert!(path.is_file(), "probe must never create a missing database");
    let result = match arguments[1].as_str() {
        "sqlite" => rom_sqlite::Sqlite::open(path).map(|_| ()),
        "redb" => rom_redb::Redb::open(path).map(|_| ()),
        _ => panic!("unsupported adapter"),
    };
    match arguments[3].as_str() {
        "accept" => assert!(result.is_ok(), "old-reader positive control failed: {result:?}"),
        "reject" => assert!(matches!(result, Err(rom::Error::Unsupported(_))), "old reader must reject unsupported marker: {result:?}"),
        _ => panic!("unsupported expected outcome"),
    }
    println!("old-reader {} {} {}", arguments[1], path.display(), arguments[3]);
}
