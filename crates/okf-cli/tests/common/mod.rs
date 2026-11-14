//! Helpers shared by integration test crates.
#![allow(dead_code)] // each test crate uses a subset of these helpers

use assert_cmd::Command;

/// The `okf` binary under test.
pub fn okf() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("okf"))
}
