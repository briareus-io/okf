//! A reader that closes okf's stdout early (`okf … | head`) ends the run quietly.
#![cfg(unix)]

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

fn okf() -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("okf"));
    command
        .current_dir(std::env::temp_dir())
        .env_remove("OKF_BUNDLE")
        .env_remove("OKF_CATALOG")
        .env_remove("OKF_CATALOG_OVERRIDES")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn reader_closing_stdout_early_exits_zero_without_message() {
    // `schema` prints far more than a pipe buffer holds, so okf is still writing
    // when the reader goes away.
    let mut child = okf().arg("schema").spawn().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(!line.is_empty());
    drop(stdout);
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
    assert_eq!(stderr, "");
}

#[test]
fn help_to_a_closed_pipe_exits_zero_without_panicking() {
    for args in [&["--help"][..], &["list", "--help"], &["--version"]] {
        let mut child = okf().args(args).spawn().unwrap();
        drop(child.stdout.take());
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(0), "{args:?} stderr: {stderr}");
        assert!(!stderr.contains("panicked"), "{args:?} stderr: {stderr}");
    }
}
