//! T-093: what case folding changes, through `kiln-cli` on disk (`tasks/T-093-macos-document-sentences.md`,
//! rows 3 and 4; ruling Q-5 A). The expected result is chosen by platform with `cfg!`, never by a
//! run-time probe, so a pass on a platform shows that platform's branch:
//! - Linux: the file system keeps names that differ only by case apart.
//! - macOS: the default volume folds case (a case-sensitive volume fails here, naming why).
//! - Windows: as macOS (NTFS's default; an inference, not run until `kiln-cli`'s test targets build there).
//!
//! Portable `std` only, no platform cfg, so the file compiles everywhere. Fixtures live under
//! `CARGO_TARGET_TMPDIR` and are removed on drop. Every spawned process is killed after a timeout.
//! Refusals are checked by `code` and `kind`, never by message text.
#![cfg(test)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// Hang detector only, not a speed claim.
const TIMEOUT: Duration = Duration::from_secs(30);

/// A fresh directory under `CARGO_TARGET_TMPDIR`, removed on drop.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("macos-t093")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("root")).unwrap();
        Fixture { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// `kiln-cli args`: (exit status, stdout, stderr), killed after [`TIMEOUT`].
fn kiln(args: &[&str]) -> (ExitStatus, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn kiln-cli {args:?}: {e}"));
    let drain = |mut r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            String::from_utf8_lossy(&buf).into_owned()
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let status = wait_or_kill(&mut child, args);
    (status, out.join().unwrap(), err.join().unwrap())
}

fn wait_or_kill(child: &mut Child, args: &[&str]) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli {args:?} did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// Fails, naming why, unless the file system at `root` folds case exactly where ruling Q-5 A
/// expects it to: `a.rs` exists there, and `A.rs` is found as it on a folding file system.
fn assert_case_folding_as_expected(root: &Path) {
    let found = root.join("A.rs").exists();
    if cfg!(target_os = "linux") {
        assert!(
            !found,
            "`A.rs` is found as `a.rs` at {}: this test expects Linux's case-sensitive file system",
            root.display()
        );
    } else {
        assert!(
            found,
            "`A.rs` is not found as `a.rs` at {}: this test expects the platform's default \
             case-folding volume (macOS: case-insensitive APFS), not a case-sensitive one",
            root.display()
        );
    }
}

/// The paths of `project.tree`'s `entries`, in the order listed.
fn tree_paths(root: &Path) -> (Vec<String>, Value) {
    let (status, out, err) = kiln(&[
        "exec",
        "project.tree",
        "{}",
        "--root",
        root.to_str().unwrap(),
    ]);
    assert_eq!(status.code(), Some(0), "project.tree: {out}{err}");
    let v: Value = serde_json::from_str(out.trim_end()).expect("one JSON value");
    let paths = v["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["path"].as_str().expect("path").to_owned())
        .collect();
    (paths, v)
}

/// Row 3: a name written a second time with other case. On Linux both names exist and are
/// listed in byte order (`A.rs` before `a.rs`); on a case-folding file system the second write
/// reaches the first file, so the listing holds one entry, under the name first created.
#[test]
fn tree_lists_names_differing_only_by_case_as_the_file_system_holds_them() {
    let f = Fixture::new("tree");
    let root = f.root();
    fs::write(root.join("a.rs"), b"first\n").unwrap();
    assert_case_folding_as_expected(&root);
    fs::write(root.join("A.rs"), b"second\n").unwrap();

    let (paths, v) = tree_paths(&root);
    assert_eq!(v["skipped"], 0, "{v}");
    if cfg!(target_os = "linux") {
        assert_eq!(paths, ["A.rs", "a.rs"], "{v}");
        assert_eq!(fs::read(root.join("a.rs")).unwrap(), b"first\n");
    } else {
        assert_eq!(paths, ["a.rs"], "{v}");
        assert_eq!(fs::read(root.join("a.rs")).unwrap(), b"second\n");
    }
}

/// Row 4: a path already open is recognised by its text only. After `a.rs` is open, `doc.open`
/// of `a.rs` returns the open document (`already_open: true`) everywhere; `doc.open` of `A.rs`
/// is `not_found` on Linux (no such file), and on a case-folding file system opens a second
/// document of the same file: a save through it changes what `a.rs` holds.
#[test]
fn reopen_with_other_case_is_a_new_document_where_case_folds() {
    let f = Fixture::new("reopen");
    let root = f.root();
    fs::write(root.join("a.rs"), b"x\n").unwrap();
    assert_case_folding_as_expected(&root);
    let script = f.dir.join("s.jsonl");
    fs::write(
        &script,
        concat!(
            r#"{"id":1,"command":"doc.open","params":{"path":"a.rs"}}"#,
            "\n",
            r#"{"id":2,"command":"doc.open","params":{"path":"a.rs"}}"#,
            "\n",
            r#"{"id":3,"command":"doc.open","params":{"path":"A.rs"}}"#,
            "\n",
            r#"{"id":4,"command":"doc.insert","params":{"doc":2,"offset":0,"text":"y"}}"#,
            "\n",
            r#"{"id":5,"command":"doc.save","params":{"doc":2}}"#,
            "\n",
            r#"{"id":6,"command":"doc.list","params":{}}"#,
            "\n",
        ),
    )
    .unwrap();
    let (status, out, err) = kiln(&[
        "run",
        script.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
    ]);
    let lines: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).expect("one JSON value per line"))
        .collect();
    assert_eq!(lines.len(), 6, "{out}{err}");

    assert_eq!(lines[0]["result"]["doc"], 1, "{}", lines[0]);
    assert_eq!(lines[0]["result"]["already_open"], false, "{}", lines[0]);
    assert_eq!(lines[1]["result"]["doc"], 1, "{}", lines[1]);
    assert_eq!(lines[1]["result"]["already_open"], true, "{}", lines[1]);

    let docs = lines[5]["result"]["docs"]
        .as_array()
        .unwrap_or_else(|| panic!("doc.list: {}", lines[5]));
    if cfg!(target_os = "linux") {
        assert_eq!(lines[2]["ok"], false, "{}", lines[2]);
        assert_eq!(lines[2]["error"]["code"], "rejected", "{}", lines[2]);
        assert_eq!(lines[2]["error"]["kind"], "not_found", "{}", lines[2]);
        assert_eq!(lines[4]["ok"], false, "{}", lines[4]);
        assert_eq!(docs.len(), 1, "{}", lines[5]);
        assert_eq!(status.code(), Some(3), "{out}{err}");
        assert_eq!(fs::read(root.join("a.rs")).unwrap(), b"x\n");
    } else {
        assert_eq!(lines[2]["ok"], true, "{}", lines[2]);
        assert_eq!(lines[2]["result"]["doc"], 2, "{}", lines[2]);
        assert_eq!(lines[2]["result"]["already_open"], false, "{}", lines[2]);
        assert_eq!(lines[4]["ok"], true, "{}", lines[4]);
        assert_eq!(docs.len(), 2, "{}", lines[5]);
        assert_eq!(status.code(), Some(0), "{out}{err}");
        assert_eq!(fs::read(root.join("a.rs")).unwrap(), b"yx\n");
    }
}
