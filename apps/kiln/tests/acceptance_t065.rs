//! T-065 acceptance tests for the criterion that runs in the gate (AC-1): the command line of
//! `kiln [--root <dir>] [--script <file>] [<path>…]` and its exit when no window can be opened
//! (`tasks/T-065-window.md`, Interface "Command line", Pinned (a), (b), (c), AC-1).
//!
//! The gate has no display, so every `kiln` here runs with `DISPLAY`, `WAYLAND_DISPLAY` and
//! `WAYLAND_SOCKET` removed from its environment (one test sets `DISPLAY` to a display nothing
//! listens on, which the spec counts as "a display that cannot be reached"); no test needs a
//! display and none opens a window. AC-2 and AC-3 (a real window under Xvfb) are run by the
//! reviewer and the evaluator by the procedure in the task file's test-author log.
//!
//! Without a display every start that passes the earlier checks ends in exit 2, so the tests
//! tell the stages of Pinned (c) apart by what the spec says each prints: a usage error prints
//! "a message and the usage" on stderr (the usage being what `kiln --help` prints on stdout); a
//! bad root's message names the root as given; "no window" is exactly one stderr line without
//! `panicked`. No test pins a message text or the usage text itself (Pinned (f); golden rule 9).
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
#[cfg(target_os = "linux")] // used by the Linux-only no-display tests only
use std::net::{SocketAddr, TcpStream};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Longest any spawned process may run (a debug build linking the toolkit, on a loaded machine).
const TIMEOUT: Duration = Duration::from_secs(120);

/// `src/a.rs` of Pinned (g).
const A_RS: &str = "fn main() {\n    let x = 1;\n}\n";

/// An X display number nothing listens on (checked by [`assert_display_unused`]).
#[cfg(target_os = "linux")] // used by the Linux-only no-display tests only
const UNUSED_DISPLAY: u16 = 4093;

// ---------------------------------------------------------------------------------------------
// Processes and directories.
// ---------------------------------------------------------------------------------------------

/// What a finished process left.
struct Out {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Out {
    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

fn wait_with_timeout(child: &mut std::process::Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what} did not finish within {TIMEOUT:?} (a window opened, or a hang)");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// How the display variables are set for one run.
#[derive(Clone, Copy)]
enum Display {
    /// `DISPLAY`, `WAYLAND_DISPLAY` and `WAYLAND_SOCKET` removed.
    None,
    /// As `None`, then `WAYLAND_DISPLAY` set to a socket name that does not exist.
    #[cfg(target_os = "linux")] // used by the Linux-only no-display tests only
    WaylandOnly,
    /// As `None`, then `DISPLAY` set to [`UNUSED_DISPLAY`].
    #[cfg(target_os = "linux")] // used by the Linux-only no-display tests only
    Unreachable,
}

/// `kiln <args>` in `cwd` with `HOME` set to `home`, stdin closed, under the hang detector.
fn kiln_in<A: AsRef<OsStr>>(cwd: &Path, home: &Path, display: Display, args: &[A]) -> Out {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(args)
        .current_dir(cwd)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env("HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match display {
        Display::None => {}
        #[cfg(target_os = "linux")]
        Display::WaylandOnly => {
            cmd.env("WAYLAND_DISPLAY", "kiln-t065-no-such-wayland-socket");
        }
        #[cfg(target_os = "linux")]
        Display::Unreachable => {
            cmd.env("DISPLAY", format!(":{UNUSED_DISPLAY}"));
        }
    }
    let what = format!("kiln {}", shown(args));
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    let mut out = child.stdout.take().expect("stdout pipe");
    let mut err = child.stderr.take().expect("stderr pipe");
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
    let status = wait_with_timeout(&mut child, &what);
    let code = status
        .code()
        .unwrap_or_else(|| panic!("{what} ended by a signal ({status})"));
    Out {
        code,
        stdout: out_reader.join().expect("stdout reader"),
        stderr: err_reader.join().expect("stderr reader"),
    }
}

/// The arguments as one line for messages (not UTF-8 shown lossily).
fn shown<A: AsRef<OsStr>>(args: &[A]) -> String {
    args.iter()
        .map(|a| a.as_ref().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// One case directory: `<dir>/d` with `src/a.rs` (the root), `<dir>/home` (an empty `HOME`),
/// `<dir>/s.jsonl` (a readable script). Returns `dir`.
fn case_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t065-acceptance")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("d/src")).expect("create root");
    fs::create_dir_all(d.join("home")).expect("create home");
    fs::write(d.join("d/src/a.rs"), A_RS).expect("write a.rs");
    fs::write(
        d.join("s.jsonl"),
        "{\"id\":\"s\",\"ui\":\"snapshot\"}\n{\"id\":1,\"command\":\"doc.list\"}\n",
    )
    .expect("write s.jsonl");
    d
}

/// Every entry under `root`, by relative path: file bytes, or `<dir>`. Symlinks not followed.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .expect("read_dir")
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            let rel = p
                .strip_prefix(base)
                .expect("under base")
                .to_string_lossy()
                .into_owned();
            let meta = fs::symlink_metadata(&p).expect("symlink_metadata");
            if meta.is_dir() {
                out.insert(rel, b"<dir>".to_vec());
                walk(base, &p, out);
            } else if meta.file_type().is_symlink() {
                let target = fs::read_link(&p).expect("read_link");
                out.insert(rel, format!("<link {}>", target.display()).into_bytes());
            } else {
                out.insert(rel, fs::read(&p).expect("read file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Panics unless nothing can be listening on X display [`UNUSED_DISPLAY`]: no socket file, no
/// abstract socket of that name, no TCP listener on its port. A precondition, not a skip.
#[cfg(target_os = "linux")] // used by the Linux-only no-display tests only
fn assert_display_unused() {
    let name = format!("/tmp/.X11-unix/X{UNUSED_DISPLAY}");
    assert!(
        fs::symlink_metadata(&name).is_err(),
        "precondition: {name} exists, so display :{UNUSED_DISPLAY} may be served here"
    );
    let unix = fs::read_to_string("/proc/net/unix").expect("precondition: read /proc/net/unix");
    assert!(
        !unix.contains(&name),
        "precondition: a socket named {name} is listening (display :{UNUSED_DISPLAY} is served)"
    );
    let port = 6000 + UNUSED_DISPLAY;
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("address");
    assert!(
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_err(),
        "precondition: something listens on {addr} (X display :{UNUSED_DISPLAY} over TCP)"
    );
}

// ---------------------------------------------------------------------------------------------
// Oracles.
// ---------------------------------------------------------------------------------------------

/// The usage, as `kiln --help` prints it on stdout (Interface: exit 0, usage on stdout).
fn usage(cwd: &Path, home: &Path) -> String {
    let out = kiln_in(cwd, home, Display::None, &["--help"]);
    assert_eq!(out.code, 0, "kiln --help: {}", out.stderr_text());
    let text = String::from_utf8(out.stdout).expect("usage is UTF-8");
    let text = text.trim().to_owned();
    assert!(!text.is_empty(), "kiln --help printed no usage on stdout");
    text
}

/// Exit 2, nothing on stdout, and no panic.
fn assert_refused_start(out: &Out, what: &str) {
    assert_eq!(
        out.code,
        2,
        "{what}: exit status (stderr: {})",
        out.stderr_text()
    );
    assert!(
        out.stdout.is_empty(),
        "{what}: wrote on stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !out.stderr_text().contains("panicked"),
        "{what}: panicked: {}",
        out.stderr_text()
    );
}

/// "No window": exit 2, nothing on stdout, exactly one stderr line (one non-empty line, with or
/// without its final line break), which does not contain `panicked`.
#[cfg(target_os = "linux")] // used by the Linux-only no-display tests only
fn assert_no_window(out: &Out, what: &str) {
    assert_refused_start(out, what);
    let text = out.stderr_text();
    let body = text.strip_suffix('\n').unwrap_or(&text);
    assert!(
        !body.trim().is_empty() && !body.contains('\n'),
        "{what}: stderr is not exactly one line: {text:?}"
    );
}

/// A usage error: exit 2, nothing on stdout, a message and the usage on stderr.
fn assert_usage_error(out: &Out, usage: &str, what: &str) {
    assert_refused_start(out, what);
    let text = out.stderr_text();
    assert!(
        text.contains(usage),
        "{what}: the usage is not on stderr (a usage error prints a message and the usage): {text:?}"
    );
    assert!(
        text.trim().len() > usage.len(),
        "{what}: no message besides the usage on stderr: {text:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-1: no window.
// ---------------------------------------------------------------------------------------------

/// AC-1: `kiln --root d src/a.rs` without a display exits 2, stdout empty, one stderr line
/// without `panicked`; and (Pinned (b)) the start writes no file: the root, the current
/// directory and `HOME` are as before.
#[test]
#[cfg(target_os = "linux")] // X11's no-window behaviour; on macOS a window opens (xtask/platform-tests.toml)
fn ac1_no_display_exits_2_with_one_line() {
    let dir = case_dir("no-display");
    let home = dir.join("home");
    // Something in each place to compare against (a negative test shows X exists first).
    fs::write(home.join(".profile"), "# home of the test\n").expect("write home file");
    let before_root = tree(&dir.join("d"));
    let before_cwd = tree(&dir);
    assert!(before_root.contains_key("src/a.rs") && before_cwd.contains_key("home/.profile"));
    let out = kiln_in(&dir, &home, Display::None, &["--root", "d", "src/a.rs"]);
    assert_no_window(&out, "kiln --root d src/a.rs (no display)");
    assert_eq!(tree(&dir.join("d")), before_root, "the root changed");
    assert_eq!(
        tree(&dir),
        before_cwd,
        "a file was written in the current directory or HOME"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// AC-1 over the other command-line forms that reach the window system: bare `kiln` (root: the
/// current directory), options after the paths, a file called `script` that is not the first
/// argument (a path, not the subcommand), a start path that does not exist (start paths come
/// after the window system, Pinned (c): no exit 3), several paths, and `--script` with a
/// readable script (its file unchanged).
#[test]
#[cfg(target_os = "linux")] // X11's no-window behaviour; on macOS a window opens (xtask/platform-tests.toml)
fn ac1_no_display_every_window_form() {
    let dir = case_dir("forms");
    let home = dir.join("home");
    fs::write(dir.join("d/script"), "a file called script\n").expect("write d/script");
    fs::write(dir.join("d/src/b.rs"), "fn b() {}\n").expect("write b.rs");
    let script_before = fs::read(dir.join("s.jsonl")).expect("s.jsonl");
    let before_root = tree(&dir.join("d"));
    let cases: [(&str, &[&str]); 8] = [
        ("d", &[]),
        ("d", &["src/a.rs"]),
        (".", &["src/a.rs", "--root", "d"]),
        (".", &["--root", "d", "script"]),
        (".", &["--root", "d", "missing.rs"]),
        (".", &["--root", "d", "src/a.rs", "src/b.rs", "missing.rs"]),
        (".", &["--root", "d", "--script", "s.jsonl", "src/a.rs"]),
        (".", &["src/a.rs", "--script", "s.jsonl", "--root", "d"]),
    ];
    for (cwd, args) in cases {
        let out = kiln_in(&dir.join(cwd), &home, Display::None, args);
        assert_no_window(
            &out,
            &format!("(in {cwd}) kiln {} (no display)", args.join(" ")),
        );
    }
    assert_eq!(
        fs::read(dir.join("s.jsonl")).expect("s.jsonl"),
        script_before,
        "the script changed"
    );
    assert_eq!(tree(&dir.join("d")), before_root, "the root changed");
    let _ = fs::remove_dir_all(&dir);
}

/// AC-1 / Interface "No window": X11 only, so `WAYLAND_DISPLAY` without `DISPLAY` gives no
/// window; and `DISPLAY` naming a display nothing serves ("a display that cannot be reached").
#[test]
#[cfg(target_os = "linux")] // X11's no-window behaviour; on macOS a window opens (xtask/platform-tests.toml)
fn ac1_wayland_only_or_unreachable_display_exits_2() {
    let dir = case_dir("unreachable");
    let home = dir.join("home");
    let out = kiln_in(
        &dir,
        &home,
        Display::WaylandOnly,
        &["--root", "d", "src/a.rs"],
    );
    assert_no_window(&out, "kiln --root d src/a.rs (WAYLAND_DISPLAY only)");
    assert_display_unused();
    let out = kiln_in(
        &dir,
        &home,
        Display::Unreachable,
        &["--root", "d", "src/a.rs"],
    );
    assert_no_window(
        &out,
        &format!("kiln --root d src/a.rs (DISPLAY=:{UNUSED_DISPLAY}, nothing listening)"),
    );
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-1: bad root, checked before the window system.
// ---------------------------------------------------------------------------------------------

/// AC-1: `kiln --root <missing>` exits 2 with nothing on stdout and a stderr naming the root as
/// given; also a root that is a file, with the root after the paths, and with a readable
/// `--script` (script readable → root → window system, Pinned (c)).
#[test]
fn ac1_bad_root_is_named_before_the_window() {
    let dir = case_dir("bad-root");
    let home = dir.join("home");
    let cases: [&[&str]; 5] = [
        &["--root", "no-such-root-t065"],
        &["--root", "no-such-root-t065", "src/a.rs"],
        &["src/a.rs", "--root", "no-such-root-t065"],
        &["--root", "no-such-root-t065", "--script", "s.jsonl"],
        &["--root", "d/src/a.rs", "src/a.rs"],
    ];
    for args in cases {
        let what = format!("kiln {} (no display)", args.join(" "));
        let out = kiln_in(&dir, &home, Display::None, args);
        assert_refused_start(&out, &what);
        let named = args[args.iter().position(|a| *a == "--root").expect("--root") + 1];
        assert!(
            out.stderr_text().contains(named),
            "{what}: stderr does not name the root `{named}`: {}",
            out.stderr_text()
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// AC-1: usage errors and `--script` unreadable.
// ---------------------------------------------------------------------------------------------

/// AC-1: `kiln --bogus`, `kiln --root d --root d`, `kiln --log l.jsonl` and the other usage
/// errors of the Interface (an option twice or without its value; `--log`, `--memory`, `--size`
/// without `script`) exit 2 with nothing on stdout, a message and the usage on stderr; `--log`
/// creates no file. A usage error comes before the root and the script check (Pinned (c)).
#[test]
fn ac1_usage_errors_exit_2() {
    let dir = case_dir("usage");
    let home = dir.join("home");
    let usage = usage(&dir, &home);
    let cases: [&[&str]; 18] = [
        &["--bogus"],
        &["--bogus", "--root", "d", "src/a.rs"],
        &["--root", "d", "src/a.rs", "--bogus"],
        &["--root", "d", "--root", "d"],
        &["--root", "d", "src/a.rs", "--root", "d"],
        &["--log", "l.jsonl"],
        &["--root", "d", "src/a.rs", "--log", "l.jsonl"],
        &["--memory"],
        &["--root", "d", "--memory"],
        &["--size", "800x600"],
        &["--root", "d", "src/a.rs", "--size", "800x600"],
        &["--root"],
        &["src/a.rs", "--root"],
        &["--root", "d", "--script"],
        &["--root", "d", "--script", "s.jsonl", "--script", "s.jsonl"],
        // Usage before the root and before the script (Pinned (c)).
        &["--bogus", "--root", "no-such-root-t065"],
        &["--root", "no-such-root-t065", "--root", "d"],
        &["--bogus", "--root", "d", "--script", "missing.jsonl"],
    ];
    for args in cases {
        let out = kiln_in(&dir, &home, Display::None, args);
        assert_usage_error(&out, &usage, &format!("kiln {}", args.join(" ")));
        assert!(
            !dir.join("l.jsonl").exists() && !dir.join("d/l.jsonl").exists(),
            "kiln {}: created the --log file",
            args.join(" ")
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// AC-1: `kiln --root d --script missing.jsonl` exits 2 with nothing on stdout; so does a
/// `--script` that names a directory. The script is checked before the root (Pinned (c)): with
/// both bad, the message is not the bad root's (it does not name the root).
#[test]
fn ac1_unreadable_script_exits_2() {
    let dir = case_dir("script-unreadable");
    let home = dir.join("home");
    let before_root = tree(&dir.join("d"));
    let cases: [&[&str]; 4] = [
        &["--root", "d", "--script", "missing.jsonl"],
        &["--root", "d", "--script", "missing.jsonl", "src/a.rs"],
        &["src/a.rs", "--script", "d", "--root", "d"],
        &["--script", "missing.jsonl", "--root", "d"],
    ];
    for args in cases {
        let what = format!("kiln {} (no display)", args.join(" "));
        let out = kiln_in(&dir, &home, Display::None, args);
        assert_refused_start(&out, &what);
    }
    let what = "kiln --root no-such-root-t065 --script missing.jsonl (no display)";
    let out = kiln_in(
        &dir,
        &home,
        Display::None,
        &["--root", "no-such-root-t065", "--script", "missing.jsonl"],
    );
    assert_refused_start(&out, what);
    assert!(
        !out.stderr_text().contains("no-such-root-t065"),
        "{what}: the root was checked before the script: {}",
        out.stderr_text()
    );
    assert_eq!(tree(&dir.join("d")), before_root, "the root changed");
    let _ = fs::remove_dir_all(&dir);
}

/// b3 (D-165): an argument that is not UTF-8 is a usage error, exit 2, a message and the usage
/// on stderr (ruling a1), nothing on stdout, no panic, in the window form and in `kiln script`,
/// whether it is a path, an option, or an option's value. (On base `std::env::args` panics.)
#[test]
fn ac1_non_utf8_argument_is_a_usage_error() {
    let dir = case_dir("non-utf8");
    let home = dir.join("home");
    let usage = usage(&dir, &home);
    let bad = OsStr::from_bytes(b"src/\xffa.rs");
    let bad_opt = OsStr::from_bytes(b"--r\xffoot");
    let os = |s: &str| OsString::from(s);
    let cases: [Vec<OsString>; 9] = [
        vec![os("--root"), os("d"), bad.into()],
        vec![bad.into()],
        vec![os("--root"), bad.into(), os("src/a.rs")],
        vec![bad_opt.into(), os("d")],
        vec![os("--root"), os("d"), os("--script"), bad.into()],
        vec![os("script"), bad.into(), os("--root"), os("d")],
        vec![
            os("script"),
            os("s.jsonl"),
            os("--root"),
            os("d"),
            bad.into(),
        ],
        vec![os("script"), os("s.jsonl"), os("--root"), bad.into()],
        vec![os("script"), os("s.jsonl"), os("--log"), bad.into()],
    ];
    let before = tree(&dir);
    for args in cases {
        let out = kiln_in(&dir, &home, Display::None, &args);
        assert_usage_error(&out, &usage, &format!("kiln {}", shown(&args)));
    }
    assert_eq!(tree(&dir), before, "a refused start wrote a file");
    let _ = fs::remove_dir_all(&dir);
}
