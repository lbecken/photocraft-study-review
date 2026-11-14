//! T-036 AC-1 through `kiln-cli`: a `--root` the process cannot open for reading (modes 0311
//! and 0000) is refused at start by `exec`, `run` and `control`: exit 2, nothing on stdout, a
//! stderr that names the root as given and is not a JSON error object, no request executed and
//! nothing changed (Interface 1, Pinned (a)). Written by the test author from
//! `tasks/T-036-root-identity-follow-ups.md` alone. AC-6 is not a test (D-114).
//!
//! The gate runs as uid 0, where permission bits do not apply, so `kiln-cli` always runs as the
//! unprivileged identity U: uid/gid 65534 through `CommandExt::{uid, gid}` when the test's
//! effective uid is 0 (the fixture is then given to U with `lchown`), otherwise the test's own
//! identity. A positive control (U opens a file under a readable root) and a negative control
//! (U cannot open a mode-0000 file it owns) run first; if either fails, the test fails naming
//! the precondition. Both identity branches run the whole scenario; nothing is skipped.
//!
//! The binary runs in the parent of `work/`, and the root is given as `<work>/root`, so stderr
//! can be checked to name the root as given; no stderr text is otherwise asserted. Every child
//! has a 10 s timeout, which only detects hangs.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt, lchown};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const SECRET: &[u8] = b"s3cr3t-content\n";
const F_BYTES: &[u8] = b"f readable\n";
/// Hang detector for every child process (AC "Processes and time"); never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);
/// The unprivileged identity used when the test runs as uid 0 (Definitions, "U").
const NOBODY: u32 = 65534;

// ---------------------------------------------------------------------------------------------
// Identity and fixture.

/// The identity the process under test runs as.
#[derive(Clone, Copy, Debug)]
struct Identity {
    /// True when the test runs as uid 0 and the child is dropped to 65534.
    drop: bool,
    uid: u32,
    gid: u32,
}

impl Identity {
    /// U, found from the owner of a file just created in `dir` (Definitions).
    fn find(dir: &Path) -> Identity {
        let probe = dir.join(".euid-probe");
        fs::write(&probe, b"").unwrap();
        let meta = fs::metadata(&probe).unwrap();
        fs::remove_file(&probe).unwrap();
        if meta.uid() == 0 {
            Identity {
                drop: true,
                uid: NOBODY,
                gid: NOBODY,
            }
        } else {
            Identity {
                drop: false,
                uid: meta.uid(),
                gid: meta.gid(),
            }
        }
    }

    fn apply(&self, cmd: &mut Command) {
        if self.drop {
            cmd.uid(self.uid).gid(self.gid);
        }
    }

    /// Give `path` and everything below it (links not followed) to U.
    fn give(&self, path: &Path) {
        if self.drop {
            lchown(path, Some(self.uid), Some(self.gid)).unwrap();
            if fs::symlink_metadata(path).unwrap().is_dir() {
                for e in fs::read_dir(path).unwrap() {
                    self.give(&e.unwrap().path());
                }
            }
        }
    }
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A fresh `work/` (mode 0755) under `std::env::temp_dir()`, with `root/`, `outside/` and
/// `outside/secret.txt`; removed on drop, failed runs included (modes are restored first).
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k36c-{}-{name}", std::process::id()));
        if dir.exists() {
            unlock(&dir);
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        set_mode(&dir, 0o755);
        let dir = fs::canonicalize(&dir).unwrap();
        fs::create_dir(dir.join("root")).unwrap();
        fs::create_dir(dir.join("outside")).unwrap();
        fs::write(dir.join("outside/secret.txt"), SECRET).unwrap();
        Work { dir }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn file(&self, at: &str, bytes: &[u8], mode: u32) {
        let p = self.at(at);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
        set_mode(&p, mode);
    }

    /// The parent of `work/`, where the binary runs.
    fn parent(&self) -> &Path {
        self.dir.parent().unwrap()
    }

    /// `work/<rel>` as given to the binary: relative to [`Work::parent`].
    fn given(&self, rel: &str) -> String {
        format!("{}/{rel}", self.dir.file_name().unwrap().to_string_lossy())
    }
}

/// Make every directory under `dir` (and `dir`) listable again, links not followed.
fn unlock(dir: &Path) {
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if fs::symlink_metadata(&p).is_ok_and(|m| m.is_dir()) {
                unlock(&p);
            }
        }
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        unlock(&self.dir);
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Every entry under `dir`, recursively, links not followed: type, link target and bytes.
fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
            let ft = fs::symlink_metadata(&p).unwrap().file_type();
            let desc = if ft.is_symlink() {
                format!("link -> {}", fs::read_link(&p).unwrap().display())
            } else if ft.is_dir() {
                walk(base, &p, out);
                "dir".to_owned()
            } else if ft.is_file() {
                format!("file {}", fs::read(&p).unwrap().escape_ascii())
            } else {
                "other".to_owned()
            };
            out.insert(rel, desc);
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

// ---------------------------------------------------------------------------------------------
// Running `kiln-cli` as U.

/// What one `kiln-cli` invocation gave.
struct Ran {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: String,
}

impl Ran {
    fn describe(&self) -> String {
        format!(
            "exit {:?}, stdout {:?}, stderr {:?}",
            self.status.code(),
            String::from_utf8_lossy(&self.stdout),
            self.stderr
        )
    }
}

fn wait_or_kill(child: &mut Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("hang detector: {what} did not finish within {TIMEOUT:?}; killed");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Run `kiln-cli args` as `id` in `cwd`, with `stdin` written to its stdin and then closed.
fn kiln(id: Identity, cwd: &Path, args: &[&str], stdin: &[u8]) -> Ran {
    let what = format!("kiln-cli {}", args.join(" "));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    id.apply(&mut cmd);
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("precondition: cannot spawn {what} as uid {}: {e}", id.uid));
    let mut input = child.stdin.take().unwrap();
    let bytes = stdin.to_vec();
    let feed = thread::spawn(move || {
        let _ = input.write_all(&bytes);
    });
    let drain = |r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut r = r;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let status = wait_or_kill(&mut child, &what);
    let _ = feed.join();
    Ran {
        status,
        stdout: out.join().unwrap(),
        stderr: String::from_utf8_lossy(&err.join().unwrap()).into_owned(),
    }
}

/// True iff some stderr line is a JSON object (what a refused *request* prints).
fn has_json_object_line(stderr: &str) -> bool {
    stderr
        .lines()
        .any(|l| serde_json::from_str::<Value>(l.trim()).is_ok_and(|v| v.is_object()))
}

// ---------------------------------------------------------------------------------------------
// AC-1 through kiln-cli.

#[test]
fn unreadable_root_is_refused_at_start() {
    let w = Work::new("start");
    let id = Identity::find(&w.dir);
    w.file("open/f.txt", F_BYTES, 0o644);
    w.file("open/locked.txt", F_BYTES, 0o000);
    w.file("root/f.txt", F_BYTES, 0o644);
    let request = json!({"id":1,"command":"doc.open","params":{"path":"f.txt"}});
    fs::write(w.at("s.jsonl"), format!("{request}\n")).unwrap();
    id.give(&w.dir);
    set_mode(&w.at("open"), 0o711);
    set_mode(&w.at("root"), 0o755);
    let cwd = w.parent().to_path_buf();
    let open = w.given("open");
    let root = w.given("root");
    let script = w.given("s.jsonl");
    let f_params = r#"{"path":"f.txt"}"#;

    // Controls, as U: U can run the binary and reach the fixture, and has no override.
    let pos = kiln(
        id,
        &cwd,
        &["exec", "doc.open", f_params, "--root", &open],
        b"",
    );
    assert!(
        pos.status.code() == Some(0) && !pos.stdout.is_empty(),
        "precondition (positive control): as uid {}, `exec doc.open f.txt --root work/open` did \
         not succeed, so U cannot run kiln-cli or reach the fixture: {}",
        id.uid,
        pos.describe()
    );
    let neg = kiln(
        id,
        &cwd,
        &[
            "exec",
            "doc.open",
            r#"{"path":"locked.txt"}"#,
            "--root",
            &open,
        ],
        b"",
    );
    assert!(
        neg.status.code() == Some(2) && neg.stdout.is_empty(),
        "precondition (negative control): as uid {}, `exec doc.open locked.txt` (mode 0000) did \
         not fail, so U overrides permission checks: {}",
        id.uid,
        neg.describe()
    );

    // The protected file exists and holds its bytes.
    assert_eq!(fs::read(w.at("root/f.txt")).unwrap(), F_BYTES);
    let before = snapshot(&w.dir);

    // Every case runs before anything is asserted, so a failure reports each of them.
    let mut failures = Vec::new();
    for mode in [0o311, 0o000] {
        let m = format!("mode {mode:04o}");
        set_mode(&w.at("root"), mode);
        let exec = kiln(
            id,
            &cwd,
            &["exec", "doc.open", f_params, "--root", &root],
            b"",
        );
        let run = kiln(id, &cwd, &["run", &script, "--root", &root], b"");
        let control = kiln(
            id,
            &cwd,
            &["control", "--root", &root],
            format!("{request}\n").as_bytes(),
        );
        set_mode(&w.at("root"), 0o755);

        if !(exec.status.code() == Some(2)
            && exec.stdout.is_empty()
            && exec.stderr.contains(&root)
            && !has_json_object_line(&exec.stderr))
        {
            failures.push(format!(
                "{m}: exec: expected exit 2, empty stdout, a stderr naming `{root}` that is not \
                 a JSON object; got {}",
                exec.describe()
            ));
        }
        if !(run.status.code() == Some(2) && run.stdout.is_empty()) {
            failures.push(format!(
                "{m}: run: expected exit 2 and empty stdout; got {}",
                run.describe()
            ));
        }
        if !(control.status.code() == Some(2) && control.stdout.is_empty()) {
            failures.push(format!(
                "{m}: control: expected exit 2 and no line on stdout; got {}",
                control.describe()
            ));
        }
        if snapshot(&w.dir) != before {
            failures.push(format!("{m}: the work/ snapshot changed"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
