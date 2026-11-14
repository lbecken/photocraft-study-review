//! T-038 guards through the built `kiln-cli`. Written by the test author from
//! `tasks/T-038-error-kinds-match-commands.md` alone.
//!
//! AC-5 `disk_refusals_through_control`: the ten C pairs of the spec's table (the *disk* kinds
//! a deterministic session on disk produces) through `kiln-cli control --root`, each answered
//! `rejected` with its kind, never `internal`. After T-038 these refusals are declared *disk*;
//! the test fails if the execute check turns one of them into `internal`/`other`.
//!
//! AC-7 `situations_of_the_changed_when_texts`: the situations the five new `when` texts name
//! give their kinds (in memory through `kiln-cli run --memory-seed`, and on disk through
//! `control --root`). The wording of the texts is not asserted (ruling Q-9, D-128: the reviewer
//! checks it, and the restated merged pins hold it).
//!
//! Both pass on base. Each case runs in its own fixture (a fresh directory removed on drop,
//! failed runs included) and, on disk, its own session; every case runs even when an earlier
//! one fails, and the test then names every failed case. Each fixture change is checked through
//! `std::fs` before the request (a precondition that fails the case, never skips it). Runs as
//! the test's own identity: no case needs an unprivileged user (Pinned (d)). No message, text
//! or time is asserted; every child process and every response has a 10 s timeout that only
//! detects hangs.

#![cfg(test)]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, symlink};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Hang detector for every child process and every response; never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);
const SECRET: &[u8] = b"outside secret\n";

// ---------------------------------------------------------------------------------------------
// Fixture.

/// A fresh `work/` under the system temp dir: `work/root/` (the session's root, holding `a.txt`
/// and `p/y.txt`) and `work/outside/` (holding `secret.txt`). Removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn new(name: &str) -> Work {
        let dir = std::env::temp_dir().join(format!("k38g-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dir = fs::canonicalize(&dir).unwrap();
        let w = Work { dir };
        w.file("root/a.txt", b"alpha\n");
        w.file("root/p/y.txt", b"yankee\n");
        w.file("outside/secret.txt", SECRET);
        w
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }

    fn file(&self, rel: &str, bytes: &[u8]) {
        let p = self.at(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, bytes).unwrap();
    }

    fn link(&self, rel: &str, target: &str) {
        symlink(target, self.at(rel)).unwrap();
        let meta = fs::symlink_metadata(self.at(rel)).unwrap();
        assert!(
            meta.file_type().is_symlink(),
            "precondition: {rel} is a link"
        );
    }

    /// Two links in `dir` (root-relative) pointing at each other: `l1 -> l2`, `l2 -> l1`.
    fn loop_links(&self, dir: &str, l1: &str, l2: &str) {
        self.link(&format!("{dir}/{l1}"), l2);
        self.link(&format!("{dir}/{l2}"), l1);
        let err = fs::metadata(self.at(&format!("{dir}/{l1}"))).unwrap_err();
        assert_eq!(
            err.raw_os_error(),
            Some(40), // ELOOP
            "precondition: {dir}/{l1} is a link loop: {err}"
        );
    }

    /// Rename `root/` away and put a new, empty directory in its place.
    fn replace_root(&self) {
        fs::rename(self.at("root"), self.at("old-root")).unwrap();
        fs::create_dir(self.at("root")).unwrap();
        assert!(self.at("old-root/a.txt").is_file() && !self.at("root/a.txt").exists());
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn mkfifo(path: &Path) {
    let mut child = Command::new("mkfifo")
        .arg(path)
        .stdin(Stdio::null())
        .spawn()
        .expect("spawn mkfifo");
    assert!(
        wait_or_kill(&mut child, "mkfifo").success(),
        "mkfifo failed"
    );
    assert!(
        fs::symlink_metadata(path).unwrap().file_type().is_fifo(),
        "precondition: {} is a FIFO",
        path.display()
    );
}

/// A chain of `levels` nested directories with 250-byte names at `work/root/<name>/<name>/…`,
/// built with short paths only (each level is made at `work/`, then the chain so far is moved
/// into it), so that the host path of the deepest directory exceeds 4,096 bytes. Returns that
/// host path's length in bytes.
fn deep_chain(w: &Work, levels: usize) -> usize {
    let name = "d".repeat(250);
    let chain = w.at("chain");
    let parent = w.at("parent");
    fs::create_dir(&chain).unwrap();
    for _ in 1..levels {
        fs::create_dir(&parent).unwrap();
        fs::rename(&chain, parent.join(&name)).unwrap();
        fs::rename(&parent, &chain).unwrap();
    }
    fs::rename(&chain, w.at("root").join(&name)).unwrap();
    w.at("root").as_os_str().len() + levels * (name.len() + 1)
}

// ---------------------------------------------------------------------------------------------
// Running `kiln-cli`.

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

/// `kiln-cli control --root root`, run in `work/`.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    out: Receiver<String>,
    next_id: u64,
}

impl Session {
    fn start(w: &Work) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
            .args(["control", "--root", "root"])
            .current_dir(&w.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn kiln-cli control");
        let stdout = child.stdout.take().unwrap();
        let (tx, out) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stdin = child.stdin.take();
        Session {
            child,
            stdin,
            out,
            next_id: 1,
        }
    }

    /// Send one request and wait (at most [`TIMEOUT`], a hang detector) for its response.
    fn ask(&mut self, command: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"id": id, "command": command, "params": params});
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        match self.out.recv_timeout(TIMEOUT) {
            Ok(line) => {
                let r: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(r["id"], json!(id), "response to {request}: {r}");
                r
            }
            Err(e) => {
                let _ = self.child.kill();
                panic!("hang detector: no response to {request} within {TIMEOUT:?}: {e}");
            }
        }
    }

    fn ok(&mut self, command: &str, params: Value, case: &str) -> Value {
        let r = self.ask(command, params.clone());
        assert_eq!(r["ok"], json!(true), "{case}: {command} {params}: {r}");
        r
    }

    fn finish(mut self) {
        drop(self.stdin.take());
        let status = wait_or_kill(&mut self.child, "kiln-cli control");
        assert!(
            status.success(),
            "control exits 0 at end of input: {status:?}"
        );
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The response is `rejected` with `kind` (so never `internal`).
fn assert_refused(r: &Value, kind: &str, case: &str) {
    assert!(
        r["ok"] == json!(false)
            && r["error"]["code"] == json!("rejected")
            && r["error"]["kind"] == json!(kind),
        "{case}: want rejected/{kind}, got {r}"
    );
}

/// One named case.
type Case<'a> = (&'a str, &'a dyn Fn(&str));

/// Run every case; fail naming every case that failed.
fn run_cases(cases: &[Case]) {
    let mut failed = Vec::new();
    for (case, run) in cases {
        if catch_unwind(AssertUnwindSafe(|| run(case))).is_err() {
            failed.push(*case);
        }
    }
    assert!(
        failed.is_empty(),
        "failed cases (each one's message is above): {failed:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-5 cases (each: fixture, session, change, request).

fn open_link_outside(case: &str) {
    let w = Work::new(case);
    w.link("root/out.txt", "../outside/secret.txt");
    assert!(
        w.at("root/out.txt").is_file(),
        "precondition: the link leads to a file"
    );
    let mut s = Session::start(&w);
    let r = s.ask("doc.open", json!({"path": "out.txt"}));
    assert_refused(&r, "outside_root", case);
    s.finish();
}

fn open_link_loop(case: &str) {
    let w = Work::new(case);
    w.loop_links("root", "l1", "l2");
    let mut s = Session::start(&w);
    let r = s.ask("doc.open", json!({"path": "l1"}));
    assert_refused(&r, "io_error", case);
    s.finish();
}

fn reopen_link_outside(case: &str) {
    let w = Work::new(case);
    let mut s = Session::start(&w);
    s.ok("doc.open", json!({"path": "a.txt"}), case);
    s.ok(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "E"}),
        case,
    );
    fs::remove_file(w.at("root/a.txt")).unwrap();
    w.link("root/a.txt", "../outside/secret.txt");
    let r = s.ask("doc.open", json!({"path": "a.txt"}));
    assert_refused(&r, "outside_root", case);
    s.finish();
}

/// `doc.open p/y.txt` and an edit, then `change`, then `doc.save` → `kind`.
fn save_after(case: &str, change: &dyn Fn(&Work), kind: &str) {
    let w = Work::new(case);
    let mut s = Session::start(&w);
    s.ok("doc.open", json!({"path": "p/y.txt"}), case);
    s.ok(
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "E"}),
        case,
    );
    change(&w);
    let r = s.ask("doc.save", json!({"doc": 1}));
    assert_refused(&r, kind, case);
    assert_eq!(
        fs::read(w.at("outside/secret.txt")).unwrap(),
        SECRET,
        "{case}: the outside file is unchanged"
    );
    s.finish();
}

fn save_parent_replaced_by_file(case: &str) {
    save_after(
        case,
        &|w| {
            fs::remove_dir_all(w.at("root/p")).unwrap();
            fs::write(w.at("root/p"), b"now a file\n").unwrap();
            assert!(w.at("root/p").is_file(), "precondition: p is a file");
        },
        "not_found",
    );
}

fn save_file_replaced_by_dir(case: &str) {
    save_after(
        case,
        &|w| {
            fs::remove_file(w.at("root/p/y.txt")).unwrap();
            fs::create_dir(w.at("root/p/y.txt")).unwrap();
            assert!(
                w.at("root/p/y.txt").is_dir(),
                "precondition: y.txt is a directory"
            );
        },
        "not_a_file",
    );
}

fn save_file_replaced_by_link_outside(case: &str) {
    save_after(
        case,
        &|w| {
            fs::remove_file(w.at("root/p/y.txt")).unwrap();
            w.link("root/p/y.txt", "../../outside/secret.txt");
            assert!(
                w.at("root/p/y.txt").is_file(),
                "precondition: the link leads to a file"
            );
        },
        "outside_root",
    );
}

fn save_file_replaced_by_link_loop(case: &str) {
    save_after(
        case,
        &|w| {
            fs::remove_file(w.at("root/p/y.txt")).unwrap();
            w.loop_links("root/p", "y.txt", "z.txt");
        },
        "io_error",
    );
}

fn tree_link_outside(case: &str) {
    let w = Work::new(case);
    w.link("root/outdir", "../outside");
    assert!(
        w.at("root/outdir").is_dir(),
        "precondition: the link leads to a directory"
    );
    let mut s = Session::start(&w);
    let r = s.ask("project.tree", json!({"path": "outdir"}));
    assert_refused(&r, "outside_root", case);
    s.finish();
}

fn tree_link_loop(case: &str) {
    let w = Work::new(case);
    w.loop_links("root", "l1", "l2");
    let mut s = Session::start(&w);
    let r = s.ask("project.tree", json!({"path": "l1"}));
    assert_refused(&r, "io_error", case);
    s.finish();
}

fn symbols_root_replaced(case: &str) {
    let w = Work::new(case);
    let mut s = Session::start(&w);
    s.ok("nav.workspace-symbols", json!({"query": "a"}), case);
    w.replace_root();
    let r = s.ask("nav.workspace-symbols", json!({"query": "a"}));
    assert_refused(&r, "outside_root", case);
    s.finish();
}

fn symbols_deep_chain(case: &str) {
    let w = Work::new(case);
    let mut s = Session::start(&w);
    s.ok("nav.workspace-symbols", json!({"query": "a"}), case);
    let len = deep_chain(&w, 18);
    assert!(
        len > 4096,
        "precondition: the deepest directory's host path is {len} bytes, not beyond 4,096"
    );
    let r = s.ask("nav.workspace-symbols", json!({"query": "a"}));
    assert_refused(&r, "io_error", case);
    s.finish();
}

/// AC-5: the ten C pairs through `kiln-cli control --root`: `doc.open` of a link to an outside
/// file → `outside_root`, of a two-link loop → `io_error`, a re-open after the open (edited)
/// file was replaced by a link outside → `outside_root`; `doc.save` after the parent directory
/// was replaced by a regular file → `not_found`, after the file was replaced by a directory →
/// `not_a_file`, by a link outside → `outside_root`, by a link loop → `io_error`;
/// `project.tree` with `path` a link to an outside directory → `outside_root`, a link loop →
/// `io_error`; `nav.workspace-symbols` after the root was replaced by a new directory →
/// `outside_root`, with a chain of nested directories whose host path exceeds 4,096 bytes →
/// `io_error`. Each answer `rejected` with that kind. The two `nav.workspace-symbols` cases
/// first succeed before the change.
#[test]
fn disk_refusals_through_control() {
    run_cases(&[
        ("open-link-outside", &open_link_outside),
        ("open-link-loop", &open_link_loop),
        ("reopen-link-outside", &reopen_link_outside),
        ("save-parent-is-file", &save_parent_replaced_by_file),
        ("save-file-is-dir", &save_file_replaced_by_dir),
        ("save-link-outside", &save_file_replaced_by_link_outside),
        ("save-link-loop", &save_file_replaced_by_link_loop),
        ("tree-link-outside", &tree_link_outside),
        ("tree-link-loop", &tree_link_loop),
        ("symbols-root-replaced", &symbols_root_replaced),
        ("symbols-deep-chain", &symbols_deep_chain),
    ]);
}

// ---------------------------------------------------------------------------------------------
// AC-7

/// Run `requests` (command, params, expected: `Some(kind)` for a refusal, `None` for ok) as one
/// script through `kiln-cli run s.jsonl --memory-seed m.json` on `files`.
fn memory_session(case: &str, files: Value, requests: &[(&str, Value, Option<&str>)]) {
    let w = Work::new(case);
    fs::write(w.at("m.json"), json!({ "files": files }).to_string()).unwrap();
    let script: String = requests
        .iter()
        .enumerate()
        .map(|(i, (c, p, _))| format!("{}\n", json!({"id": i, "command": c, "params": p})))
        .collect();
    fs::write(w.at("s.jsonl"), script).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(["run", "s.jsonl", "--memory-seed", "m.json"])
        .current_dir(&w.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn kiln-cli run");
    let mut so = child.stdout.take().unwrap();
    let reader = thread::spawn(move || {
        let mut s = String::new();
        let _ = so.read_to_string(&mut s);
        s
    });
    wait_or_kill(&mut child, "kiln-cli run");
    let stdout = reader.join().unwrap();
    let responses: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        responses.len(),
        requests.len(),
        "{case}: one response per request"
    );
    let mut wrong = Vec::new();
    for (i, ((c, p, want), r)) in requests.iter().zip(&responses).enumerate() {
        assert_eq!(r["id"], json!(i), "{case}: response order");
        let fits = match want {
            None => r["ok"] == json!(true),
            Some(kind) => {
                r["ok"] == json!(false)
                    && r["error"]["code"] == json!("rejected")
                    && r["error"]["kind"] == json!(kind)
            }
        };
        if !fits {
            let p = p.to_string();
            let p: String = p.chars().take(120).collect();
            wrong.push(format!("{c} {p}: want {want:?}, got {r}"));
        }
    }
    assert!(wrong.is_empty(), "{case}:\n{}", wrong.join("\n"));
}

fn changed_texts_in_memory(case: &str) {
    let groups = |n: usize| "(a)".repeat(n);
    memory_session(
        case,
        json!([
            {"path": "f.txt", "text": "abc\r\ndef\n"},
            {"path": "a.rs", "text": "fn main() {}\n"},
        ]),
        &[
            // not_found: a component that must be a directory is a file, or is missing.
            ("doc.open", json!({"path": "f.txt/x"}), Some("not_found")),
            (
                "doc.open",
                json!({"path": "nodir/x.txt"}),
                Some("not_found"),
            ),
            // not_a_file: a file where a directory is needed.
            ("project.tree", json!({"path": "f.txt"}), Some("not_a_file")),
            ("doc.open", json!({"path": "f.txt"}), None), // doc 1: "abc", "def", ""
            // out_of_range: a column beyond its line, a line beyond the last line.
            (
                "doc.position",
                json!({"doc": 1, "at": {"line_col": {"line": 0, "col": 4}}}),
                Some("out_of_range"),
            ),
            (
                "doc.position",
                json!({"doc": 1, "at": {"line_col": {"line": 0, "col": 3}}}),
                None,
            ),
            (
                "doc.line",
                json!({"doc": 1, "line": 3}),
                Some("out_of_range"),
            ),
            ("doc.line", json!({"doc": 1, "line": 2}), None),
            // invalid_pattern: longer than 1,000 chars; more than 100 groups; `$3` of 2 groups.
            (
                "search.find",
                json!({"doc": 1, "query": "a".repeat(1001)}),
                Some("invalid_pattern"),
            ),
            (
                "search.find",
                json!({"doc": 1, "query": "a".repeat(1000)}),
                None,
            ),
            (
                "search.find",
                json!({"doc": 1, "query": groups(101), "regex": true}),
                Some("invalid_pattern"),
            ),
            (
                "search.find",
                json!({"doc": 1, "query": groups(100), "regex": true}),
                None,
            ),
            (
                "search.replace",
                json!({"doc": 1, "query": "(a)(b)", "regex": true, "replacement": "$3"}),
                Some("invalid_pattern"),
            ),
            // out_of_range: an outline item index beyond the last item (one item: `main`).
            ("doc.open", json!({"path": "a.rs"}), None), // doc 2
            (
                "nav.goto-symbol",
                json!({"doc": 2, "index": 5}),
                Some("out_of_range"),
            ),
            ("nav.goto-symbol", json!({"doc": 2, "index": 0}), None),
        ],
    );
}

fn tree_of_a_fifo(case: &str) {
    let w = Work::new(case);
    mkfifo(&w.at("root/fifo"));
    let mut s = Session::start(&w);
    let r = s.ask("project.tree", json!({"path": "fifo"}));
    assert_refused(&r, "not_a_file", case);
    s.finish();
}

fn tree_after_root_replaced(case: &str) {
    let w = Work::new(case);
    let mut s = Session::start(&w);
    s.ok("project.tree", json!({}), case);
    w.replace_root();
    let r = s.ask("project.tree", json!({}));
    assert_refused(&r, "outside_root", case);
    s.finish();
}

/// AC-7: `doc.open f.txt/x` and `nodir/x.txt` → `not_found` (memory); `project.tree {"path":
/// "f.txt"}` (a file) → `not_a_file` (memory) and of a FIFO → `not_a_file` (disk);
/// `doc.position` with a column beyond its line, `doc.line` past the last line,
/// `nav.goto-symbol` with an index beyond the outline → `out_of_range`; `search.find` with a
/// 1,001-char query and with a regex of 101 groups, `search.replace` with `$3` for a 2-group
/// regex → `invalid_pattern`; `project.tree {}` after the root was replaced → `outside_root`.
/// Next to each bound, the request just inside it succeeds, so the refusal is the bound's.
#[test]
fn situations_of_the_changed_when_texts() {
    run_cases(&[
        ("when-memory", &changed_texts_in_memory),
        ("when-tree-fifo", &tree_of_a_fifo),
        ("when-root-replaced", &tree_after_root_replaced),
    ]);
}
