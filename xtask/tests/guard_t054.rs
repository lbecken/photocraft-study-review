//! T-054 guard tests (test author): AC-2, file and line are kept in a panic and in a backtrace.
//! Both pass on base (Cargo's default, full debug info) and with the Interface's profile (line
//! tables for the workspace's crates).
//!
//! - The backtrace test captures `std::backtrace::Backtrace::force_capture()` in its own body and
//!   asserts that the text names this file with the line of that call (oracle: `file!()` and
//!   `line!()`). With `debug = false` for the workspace the `at <file>:<line>` lines are gone
//!   (R-270, D0), so it fails then: it guards against a later, lighter setting.
//! - The panic test raises a panic inside `std::panic::catch_unwind`, with a hook that records
//!   `PanicHookInfo::location()` for this test's thread only (other threads' panics go to the
//!   previous hook), and asserts the file is `file!()` and the line the panicking line.
//!
//! Only file name and line are read from the backtrace (Pinned (f)); its path prefix (`./`,
//! absolute, `\` on Windows) is not pinned. On failure the message prints the platform
//! (`std::env::consts::OS`), what was looked for and the backtrace text (truncated), so that the
//! result can be read per platform on `ci/T-054` (ruling Q-4 A, D-200).
//!
//! No Unix-only API and no process is used (runs on Linux, macOS and Windows).
#![cfg(test)]

use std::backtrace::{Backtrace, BacktraceStatus};
use std::panic;
use std::sync::{Arc, Mutex};

/// This file as a backtrace names it, from `file!()`: the last two components
/// (`tests/guard_t054.rs`) with `/` as the separator.
fn this_file_tail() -> String {
    let f = file!().replace('\\', "/");
    let parts: Vec<&str> = f.rsplitn(3, '/').collect();
    assert!(
        parts.len() >= 2,
        "precondition: file!() = {f:?} has a directory"
    );
    format!("{}/{}", parts[1], parts[0])
}

/// At most `max` characters of `s`, marked when cut.
fn truncated(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}\n… (truncated)")
    }
}

#[test]
fn ac2_backtrace_names_this_file_and_line() {
    let bt = Backtrace::force_capture();
    let line = line!() - 1;
    let text = bt.to_string();
    let tail = this_file_tail();
    let want = format!("{tail}:{line}");
    let os = std::env::consts::OS;
    assert_eq!(
        bt.status(),
        BacktraceStatus::Captured,
        "platform {os}: force_capture() captured no backtrace (looked for `{want}`)"
    );
    // A frame line names the file and line as `<path>:<line>:<column>` (or `<path>:<line>`).
    let found = text.lines().any(|l| {
        let l = l.replace('\\', "/");
        match l.find(&want) {
            Some(i) => {
                let before_ok =
                    i == 0 || !l[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_');
                let rest = &l[i + want.len()..];
                let after_ok = !rest.starts_with(|c: char| c.is_ascii_digit());
                before_ok && after_ok
            }
            None => false,
        }
    });
    assert!(
        found,
        "platform {os}: the backtrace does not name this test's file and line: looked for \
         `{want}` (file!() = {:?}, line of force_capture() = {line}, `\\` read as `/`) in a \
         frame line; backtrace text:\n{}",
        file!(),
        truncated(&text, 4000)
    );
}

#[test]
fn ac2_panic_location_is_this_file_and_line() {
    type Hook = Box<dyn Fn(&panic::PanicHookInfo<'_>) + Sync + Send + 'static>;
    let me = std::thread::current().id();
    let seen: Arc<Mutex<Vec<(String, u32)>>> = Arc::new(Mutex::new(Vec::new()));
    let prev: Arc<Hook> = Arc::new(panic::take_hook());
    {
        let seen = Arc::clone(&seen);
        let prev = Arc::clone(&prev);
        panic::set_hook(Box::new(move |info| {
            if std::thread::current().id() == me {
                let loc = info
                    .location()
                    .map(|l| (l.file().to_owned(), l.line()))
                    .unwrap_or_else(|| ("<no location>".to_owned(), 0));
                seen.lock().unwrap_or_else(|e| e.into_inner()).push(loc);
            } else {
                prev(info);
            }
        }));
    }
    let line = line!() + 1;
    let caught = panic::catch_unwind(|| panic!("T-054 guard: a panic raised on purpose"));
    // Restore a hook that forwards to the previous one.
    let _ = panic::take_hook();
    {
        let prev = Arc::clone(&prev);
        panic::set_hook(Box::new(move |info| prev(info)));
    }
    let os = std::env::consts::OS;
    assert!(
        caught.is_err(),
        "platform {os}: catch_unwind caught no panic"
    );
    let seen = seen.lock().unwrap_or_else(|e| e.into_inner()).clone();
    assert_eq!(
        seen,
        vec![(file!().to_owned(), line)],
        "platform {os}: the panic hook's location(s), looked for this file ({:?}) at the \
         panicking line {line}",
        file!()
    );
}
