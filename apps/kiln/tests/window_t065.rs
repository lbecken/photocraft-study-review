//! T-065, the implementer's tests of bare `kiln` (the window) through the built binary, with
//! no display: every display variable (`DISPLAY`, `WAYLAND_DISPLAY`, `WAYLAND_SOCKET`) is
//! removed, so no test needs a window system. The real window is observed under Xvfb by the
//! reviewer and the evaluator (AC-2, AC-3), not here.
//!
//! No test pins a message text or the usage text: they compare with what `kiln --help` prints,
//! count lines, and look for the names given on the command line (rulings a1, a2).
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

/// `kiln <args>` in `dir`, display variables removed, under [`TIMEOUT`].
fn kiln(dir: &Path, args: &[&str]) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .args(args)
        .current_dir(dir)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln");
    let mut out = child.stdout.take().expect("stdout");
    let mut err = child.stderr.take().expect("stderr");
    let out_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let err_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln {args:?} did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    Run {
        code: status
            .code()
            .unwrap_or_else(|| panic!("kiln ended by a signal ({status})")),
        stdout: String::from_utf8(out_reader.join().expect("stdout reader")).expect("utf-8"),
        stderr: String::from_utf8(err_reader.join().expect("stderr reader")).expect("utf-8"),
    }
}

/// A fresh directory with a root `d` holding `src/a.rs`, and a script `s.jsonl` beside it.
fn fixture(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("window_t065-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("d/src")).expect("mkdir");
    fs::write(dir.join("d/src/a.rs"), "fn main() {}\n").expect("write a.rs");
    fs::write(dir.join("s.jsonl"), "{\"id\":1,\"ui\":\"snapshot\"}\n").expect("write script");
    dir
}

/// No window: exit 2, nothing on stdout, exactly one stderr line without `panicked`.
fn assert_no_window(run: &Run, what: &str) {
    assert_eq!(run.code, 2, "{what}: stderr {:?}", run.stderr);
    assert!(run.stdout.is_empty(), "{what}: stdout {:?}", run.stdout);
    assert_eq!(run.stderr.lines().count(), 1, "{what}: {:?}", run.stderr);
    assert!(run.stderr.ends_with('\n'), "{what}: {:?}", run.stderr);
    assert!(!run.stderr.contains("panicked"), "{what}: {:?}", run.stderr);
}

fn help(dir: &Path) -> String {
    let run = kiln(dir, &["--help"]);
    assert_eq!(run.code, 0);
    assert!(run.stderr.is_empty());
    assert!(!run.stdout.is_empty());
    run.stdout
}

#[test]
fn without_a_display_the_window_exits_2_with_one_line() {
    let dir = fixture("no-display");
    for args in [
        &["--root", "d", "src/a.rs"][..],
        &["--root", "d"],
        &["src/a.rs", "--root", "d"],
        // Start paths come after the window system: a missing one changes nothing here.
        &["--root", "d", "missing.rs", "src/a.rs"],
        // The script is read, then the window system fails: no response is printed.
        &["--root", "d", "--script", "s.jsonl", "src/a.rs"],
        &["src/a.rs", "--script", "s.jsonl", "--root", "d"],
        // A file named `script` after an option is a start path, not the subcommand.
        &["--root", "d", "script"],
    ] {
        assert_no_window(&kiln(&dir, args), &format!("{args:?}"));
    }
    // The default root is the current directory.
    assert_no_window(&kiln(&dir.join("d"), &["src/a.rs"]), "default root");
}

#[test]
fn the_root_is_checked_before_the_window_system() {
    let dir = fixture("bad-root");
    for args in [
        &["--root", "no-such-root"][..],
        &["--root", "no-such-root", "src/a.rs", "--script", "s.jsonl"],
    ] {
        let run = kiln(&dir, args);
        assert_eq!(run.code, 2, "{args:?}");
        assert!(run.stdout.is_empty());
        assert!(run.stderr.contains("no-such-root"), "{:?}", run.stderr);
    }
    // A file as the root is refused too.
    let run = kiln(&dir, &["--root", "s.jsonl"]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("s.jsonl"), "{:?}", run.stderr);
}

#[test]
fn an_unreadable_script_is_reported_before_the_root() {
    let dir = fixture("bad-script");
    let run = kiln(&dir, &["--root", "d", "--script", "missing.jsonl"]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("missing.jsonl"), "{:?}", run.stderr);
    // Ruling a2: with a bad root as well, the script is what is reported.
    let run = kiln(
        &dir,
        &["--root", "no-such-root", "--script", "missing.jsonl"],
    );
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("missing.jsonl"), "{:?}", run.stderr);
    assert!(!run.stderr.contains("no-such-root"), "{:?}", run.stderr);
    // A directory is not a readable script.
    let run = kiln(&dir, &["--root", "d", "--script", "d"]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
}

#[test]
fn usage_errors_print_a_message_and_the_usage() {
    let dir = fixture("usage");
    let usage = help(&dir);
    for args in [
        &["--bogus"][..],
        &["--root", "d", "--root", "d"],
        &["--script", "s.jsonl", "--script", "s.jsonl"],
        &["--root"],
        &["--root", "d", "--script"],
        &["--log", "l.jsonl"],
        &["--memory"],
        &["--size", "800x600"],
        &["--root", "d", "src/a.rs", "--log", "l.jsonl"],
        // Usage comes first: before the script, the root and the window system.
        &[
            "--root",
            "no-such-root",
            "--script",
            "missing.jsonl",
            "--bogus",
        ],
    ] {
        let run = kiln(&dir, args);
        assert_eq!(run.code, 2, "{args:?}");
        assert!(run.stdout.is_empty(), "{args:?}");
        // Ruling a1: a message and the usage exactly as `--help` prints it.
        assert!(run.stderr.contains(&usage), "{args:?}: {:?}", run.stderr);
        assert!(run.stderr.len() > usage.len(), "{args:?}: no message");
        assert!(!run.stderr.contains("panicked"));
    }
    assert!(!dir.join("l.jsonl").exists(), "a usage error writes no log");
}

#[test]
fn help_prints_the_usage_on_stdout() {
    let dir = fixture("help");
    let usage = help(&dir);
    for args in [
        &["-h"][..],
        &["script", "--help"],
        &["--root", "d", "--help"],
    ] {
        let run = kiln(&dir, args);
        assert_eq!(run.code, 0, "{args:?}");
        assert_eq!(run.stdout, usage, "{args:?}");
        assert!(run.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn the_subcommand_script_still_needs_no_display() {
    let dir = fixture("script");
    let run = kiln(&dir, &["script", "s.jsonl", "--root", "d", "src/a.rs"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout.lines().count(), 1);
    assert!(run.stdout.contains("src/a.rs — Kiln"), "{}", run.stdout);
    // `script` as the first argument is always the subcommand: here without its script.
    let run = kiln(&dir, &["script"]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(
        run.stderr.lines().count() > 1,
        "a usage error: {:?}",
        run.stderr
    );
}
