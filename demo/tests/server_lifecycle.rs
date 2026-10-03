#![cfg(unix)]

use rom_demo::smoke;
#[path = "server_lifecycle/support.rs"]
mod server_lifecycle_support;
use server_lifecycle_support::run;

#[test]
fn sqlite_immediate_sigterm_drains_and_reopens() {
    run("sqlite", "TERM");
}
#[test]
fn redb_immediate_sigterm_drains_and_reopens() {
    run("redb", "TERM");
}
#[test]
fn sqlite_immediate_sigint_drains_and_reopens() {
    run("sqlite", "INT");
}
#[test]
fn redb_immediate_sigint_drains_and_reopens() {
    run("redb", "INT");
}
