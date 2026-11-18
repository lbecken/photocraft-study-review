//! T-096 acceptance test (test author), written from the task file alone.
//!
//! AC-1: the unit test `queue::tests::main_xtask_is_rebuilt_from_base_on_every_run` passes when
//! the machine's default toolchain cannot build, because its tiny workspace carries this
//! checkout's `rust-toolchain.toml`. The test builds a scratch `RUSTUP_HOME` in a fresh
//! directory under the system temporary directory (no toolchain file above it): the pinned
//! toolchain under its own name (a symlink to the real one) and a default toolchain `fake`
//! whose `cargo` and `rustc` print a marker and exit 97. It first shows that this setup is
//! effective (outside any toolchain file `cargo --version` fails with the marker; in the
//! checkout it prints the pinned channel), then runs the unit test in the checkout with that
//! `RUSTUP_HOME` and a scratch `CARGO_TARGET_DIR` (both only for the spawned processes) and
//! reads its result line. The machine's real `RUSTUP_HOME` and default are never written.
//!
//! On Windows the unit test is not built (registry: `unix`, counterpart `none`; ruling Q-2,
//! D-259): the test asserts instead that the binary's `--list` has no line for it. The platform
//! is chosen at run time with `cfg!`, never with a cfg attribute; both branches assert.
//!
//! Cost: one nested `cargo test` that builds `xtask`'s unit-test binary from scratch into the
//! scratch target (about half a minute on the director's machine), no network (`--offline`).
//! Every spawned process has a timeout (D-028 (8)); the scratch directory is removed on drop.
#![cfg(test)]

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const UNIT_TEST: &str = "queue::tests::main_xtask_is_rebuilt_from_base_on_every_run";
/// Printed by the fake default toolchain's `cargo` and `rustc`.
const MARKER: &str = "T096-FAKE-DEFAULT-TOOLCHAIN-CANNOT-BUILD";
/// The nested build of `xtask`'s unit-test binary and its run.
const NESTED_SECS: u64 = 1200;
const SHORT_SECS: u64 = 120;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// A fresh directory under the system temporary directory, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = std::env::temp_dir().join(format!("kiln-acc-t096-{}-{nanos}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Out {
    fn text(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

fn run_to(c: &mut Command, secs: u64) -> Out {
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot start {c:?}: {e}"));
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t1 = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = so.read_to_end(&mut v);
        v
    });
    let t2 = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = se.read_to_end(&mut v);
        v
    });
    let deadline = Instant::now() + Duration::from_secs(secs);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("timed out after {secs}s: {c:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Out {
        code: status.code(),
        stdout: String::from_utf8_lossy(&t1.join().unwrap()).into_owned(),
        stderr: String::from_utf8_lossy(&t2.join().unwrap()).into_owned(),
    }
}

/// A child with nothing inherited from the outer `cargo test` (target directory, package
/// variables, the active toolchain), so the toolchain is chosen by rustup's own rules.
fn clean(program: &str, dir: &Path) -> Command {
    let mut c = Command::new(program);
    for (k, _) in std::env::vars_os() {
        let Some(k) = k.to_str() else { continue };
        if (k.starts_with("CARGO") && k != "CARGO_HOME")
            || k.starts_with("__CARGO")
            || k == "RUSTUP_TOOLCHAIN"
        {
            c.env_remove(k);
        }
    }
    c.current_dir(dir);
    c
}

/// The `channel` of this checkout's `rust-toolchain.toml`.
fn pinned_channel() -> String {
    let path = checkout().join("rust-toolchain.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("precondition: read {}: {e}", path.display()));
    let v: toml::Value = toml::from_str(&text).expect("rust-toolchain.toml is TOML");
    v.get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(|c| c.as_str())
        .expect("rust-toolchain.toml has [toolchain] channel")
        .to_owned()
}

/// `cargo test --offline --locked -p xtask --bin xtask <args>` in the checkout.
fn nested_cargo_test(target: &Path, args: &[&str], rustup_home: Option<&Path>) -> Out {
    let mut c = clean("cargo", &checkout());
    c.args([
        "test",
        "--offline",
        "--locked",
        "-p",
        "xtask",
        "--bin",
        "xtask",
    ])
    .args(args)
    .env("CARGO_TARGET_DIR", target)
    .env("RUST_BACKTRACE", "0");
    if let Some(h) = rustup_home {
        c.env("RUSTUP_HOME", h).env("RUSTUP_AUTO_INSTALL", "0");
    }
    run_to(&mut c, NESTED_SECS)
}

/// The counts of the `test result:` lines in `text`: (lines, passed, failed).
fn result_counts(text: &str) -> (usize, u64, u64) {
    let num = |line: &str, word: &str| -> u64 {
        let i = line
            .find(&format!(" {word}"))
            .unwrap_or_else(|| panic!("no `{word}` in {line:?}"));
        line[..i]
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("no count before `{word}` in {line:?}"))
    };
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("test result:"))
        .collect();
    let passed = lines.iter().map(|l| num(l, "passed")).sum();
    let failed = lines.iter().map(|l| num(l, "failed")).sum();
    (lines.len(), passed, failed)
}

/// Each `panicked at` line of `text` with the five lines after it.
fn panics(text: &str) -> String {
    let v: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in v.iter().enumerate() {
        if l.contains("panicked at") {
            out.extend_from_slice(&v[i..(i + 6).min(v.len())]);
        }
    }
    out.join("\n")
}

fn tail(s: &str, n: usize) -> String {
    let v: Vec<&str> = s.lines().collect();
    v[v.len().saturating_sub(n)..].join("\n")
}

#[test]
fn rebuild_from_base_unit_test_passes_when_the_default_toolchain_cannot_build() {
    if cfg!(unix) {
        unix_branch();
    } else {
        windows_branch();
    }
}

/// AC-1 on Linux and macOS.
fn unix_branch() {
    let scratch = Scratch::new();
    let channel = pinned_channel();

    // rustup is needed: the toolchain file is honoured by its proxies (Pinned (d)).
    let v = run_to(
        &mut clean("rustup", &checkout()).arg("--version"),
        SHORT_SECS,
    );
    assert_eq!(
        v.code,
        Some(0),
        "this test needs `rustup` and its proxies on PATH (CI installs the toolchain with \
         `rustup toolchain install`); `rustup --version` failed:\n{}",
        v.text()
    );

    // The real pinned toolchain, as this checkout selects it.
    let s = run_to(
        &mut clean("rustc", &checkout()).args(["--print", "sysroot"]),
        SHORT_SECS,
    );
    assert_eq!(s.code, Some(0), "rustc --print sysroot:\n{}", s.text());
    let sysroot = PathBuf::from(s.stdout.trim());
    let name = sysroot
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| panic!("sysroot without a name: {}", sysroot.display()))
        .to_owned();
    assert!(
        name.starts_with(&format!("{channel}-")),
        "the checkout's toolchain {name} is not the pinned channel {channel}"
    );

    // Scratch RUSTUP_HOME: the pinned toolchain under its own name, a default that cannot build.
    let home = scratch.0.join("rustup");
    let toolchains = home.join("toolchains");
    let fake_bin = toolchains.join("fake").join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    let ln = run_to(
        Command::new("ln")
            .arg("-s")
            .arg(&sysroot)
            .arg(toolchains.join(&name)),
        SHORT_SECS,
    );
    assert_eq!(ln.code, Some(0), "ln -s:\n{}", ln.text());
    let script = format!("#!/bin/sh\necho {MARKER}\necho {MARKER} >&2\nexit 97\n");
    for tool in ["cargo", "rustc"] {
        std::fs::write(fake_bin.join(tool), &script).unwrap();
    }
    let chmod = run_to(
        Command::new("chmod")
            .arg("+x")
            .arg(fake_bin.join("cargo"))
            .arg(fake_bin.join("rustc")),
        SHORT_SECS,
    );
    assert_eq!(chmod.code, Some(0), "chmod:\n{}", chmod.text());
    std::fs::write(
        home.join("settings.toml"),
        "version = \"12\"\ndefault_toolchain = \"fake\"\nprofile = \"minimal\"\nauto_self_update = \"disable\"\n\n[overrides]\n",
    )
    .unwrap();

    // Self-check (METHODOLOGY §6): the forbidden target exists. Outside any toolchain file the
    // default answers and cannot build ...
    let outside = scratch.0.join("no-toolchain-file");
    std::fs::create_dir_all(&outside).unwrap();
    let mut probe = |dir: &Path| {
        run_to(
            clean("cargo", dir)
                .arg("--version")
                .env("RUSTUP_HOME", &home)
                .env("RUSTUP_AUTO_INSTALL", "0"),
            SHORT_SECS,
        )
    };
    let d = probe(&outside);
    assert!(
        d.code != Some(0) && d.text().contains(MARKER),
        "setup not effective: `cargo --version` without a toolchain file should fail with the \
         marker (exit {:?}):\n{}",
        d.code,
        d.text()
    );
    // ... and in the checkout its toolchain file selects the pinned toolchain.
    let p = probe(&checkout());
    assert_eq!(
        p.code,
        Some(0),
        "`cargo --version` in the checkout:\n{}",
        p.text()
    );
    assert!(
        p.stdout.starts_with(&format!("cargo {channel} ")) && !p.text().contains(MARKER),
        "`cargo --version` in the checkout should print the pinned {channel}:\n{}",
        p.text()
    );

    // AC-1: the unit test passes with that RUSTUP_HOME.
    let started = Instant::now();
    let o = nested_cargo_test(
        &scratch.0.join("target"),
        &[UNIT_TEST, "--", "--exact"],
        Some(&home),
    );
    let (lines, passed, failed) = result_counts(&o.stdout);
    let shown = format!(
        "exit {:?}, {lines} result line(s), {passed} passed, {failed} failed, {:.1}s\n\
         --- inner panics ---\n{}\n--- stdout (tail) ---\n{}\n--- stderr (tail) ---\n{}",
        o.code,
        started.elapsed().as_secs_f64(),
        panics(&o.text()),
        tail(&o.stdout, 40),
        tail(&o.stderr, 40)
    );
    assert_eq!(
        (o.code, lines, passed, failed),
        (Some(0), 1, 1, 0),
        "the unit test with a default toolchain that cannot build: {shown}"
    );
    println!("AC-1 (unix): {shown}");
}

/// AC-1's counterpart on Windows: the unit test is not built there (registry `unix`).
fn windows_branch() {
    let scratch = Scratch::new();
    let o = nested_cargo_test(&scratch.0.join("target"), &["--", "--list"], None);
    assert_eq!(
        o.code,
        Some(0),
        "cargo test --bin xtask -- --list:\n{}",
        o.text()
    );
    let listed: Vec<&str> = o.stdout.lines().filter(|l| l.ends_with(": test")).collect();
    assert!(
        listed.iter().any(|l| l.starts_with("queue::tests::")),
        "the list holds no queue unit test at all, so absence would prove nothing:\n{}",
        o.stdout
    );
    assert!(
        !listed
            .iter()
            .any(|l| l.trim_end_matches(": test") == UNIT_TEST),
        "{UNIT_TEST} is registered `unix` but is built on this platform:\n{}",
        o.stdout
    );
    println!(
        "AC-1 (windows counterpart): {UNIT_TEST} is not in the unit-test list ({} tests listed); \
         registry `unix`, counterpart `none`",
        listed.len()
    );
}
