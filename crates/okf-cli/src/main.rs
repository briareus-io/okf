//! okf-cli: the `okf` binary. Arg parsing, output rendering, exit-code mapping.
//!
//! Thin layer over okf-core (the deterministic "hands"). See knowledge/contributing/architecture/overview.md.
//!
//! SIGPIPE stays ignored (Rust's default) so a git child that exits early surfaces
//! as an `EPIPE` error instead of killing okf. A reader that closes okf's own stdout
//! (`okf … | head`) is handled in `exit` as a quiet exit 0.
#![recursion_limit = "256"]

mod cli;
mod commands;
mod exit;
mod output;
mod structured;

fn main() {
    let reads = okf_core::ports::git_read::ReadSession::new();
    let code = exit::run();
    drop(reads);
    std::process::exit(code);
}
