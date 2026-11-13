//! T-019 guards through `kiln-cli exec doc.open` (pass on the base and on the joined branch):
//! on AC-4's fixture (links to the outside, to `.`, a link loop, a FIFO and a `.gitignore`
//! linked to an outside file) `doc.open` keeps T-008's results, so adding `Vfs::list_dir` left
//! the open path confined. Refusals are checked by `code` and `kind` only, never by message
//! text (Pinned (f)). Every spawned process is killed after a timeout (D-028 (8)); the 5 s
//! timeout detects a hang on the FIFO, it is not a speed claim.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const HANG: Duration = Duration::from_secs(5);
const SECRET: &[u8] = b"s3cr3t\n";

fn spawn_with_timeout(mut cmd: Command, timeout: Duration) -> Output {
    let what = format!("{cmd:?}");
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {what}: {e}"));
    let drain = |mut r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{what} did not finish within {timeout:?} (hang)");
        }
        thread::sleep(Duration::from_millis(2));
    };
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

/// A fresh temporary directory with AC-4's fixture, removed on drop.
struct Work {
    dir: PathBuf,
}

impl Work {
    fn ac4(tag: &str) -> Work {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "k19g-{}-{}-{tag}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let w = Work {
            dir: fs::canonicalize(&dir).unwrap(),
        };
        fs::create_dir_all(w.at("root/sub")).unwrap();
        fs::create_dir_all(w.at("outside")).unwrap();
        fs::write(w.at("outside/secret.txt"), SECRET).unwrap();
        fs::write(w.at("outside/.gitignore"), b"*\n").unwrap();
        fs::write(w.at("outside-ignore"), b"*\n").unwrap();
        symlink("../outside", w.at("root/out")).unwrap();
        symlink(".", w.at("root/self")).unwrap();
        symlink("loop2", w.at("root/loop1")).unwrap();
        symlink("loop1", w.at("root/loop2")).unwrap();
        symlink("../outside-ignore", w.at("root/.gitignore")).unwrap();
        fs::write(w.at("root/keep.rs"), b"fn keep() {}\n").unwrap();
        fs::write(w.at("root/sub/x.rs"), b"fn x() {}\n").unwrap();
        let mut mkfifo = Command::new("mkfifo");
        mkfifo.arg(w.at("root/p"));
        assert!(spawn_with_timeout(mkfifo, HANG).status.success());
        assert!(
            fs::symlink_metadata(w.at("root/p"))
                .unwrap()
                .file_type()
                .is_fifo()
        );
        // The escapes are real.
        assert_eq!(fs::read(w.at("root/out/secret.txt")).unwrap(), SECRET);
        assert_eq!(fs::read(w.at("root/.gitignore")).unwrap(), b"*\n");
        w
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Every entry under `dir`, symlinks not followed, FIFOs not opened.
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
                "special".to_owned()
            };
            out.insert(rel, desc);
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// `kiln-cli exec doc.open {"path": path} --root root` under the hang timeout:
/// `Ok(result)` on exit 0, `Err(error object)` on exit 2.
fn open(w: &Work, path: &str) -> Result<Value, Value> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln-cli"));
    cmd.args([
        "exec",
        "doc.open",
        &json!({ "path": path }).to_string(),
        "--root",
        "root",
    ])
    .current_dir(&w.dir);
    let out = spawn_with_timeout(cmd, HANG);
    match out.status.code() {
        Some(0) => Ok(serde_json::from_slice(&out.stdout).unwrap()),
        Some(2) => {
            let text = String::from_utf8_lossy(&out.stderr).into_owned();
            Err(serde_json::from_str(text.lines().next().unwrap_or("")).unwrap())
        }
        other => panic!("doc.open {path}: exit {other:?}"),
    }
}

fn assert_refused(w: &Work, path: &str, kind: &str) {
    match open(w, path) {
        Ok(v) => panic!("doc.open {path} succeeded, expected rejected/{kind}: {v}"),
        Err(e) => {
            assert_eq!(e["code"], json!("rejected"), "doc.open {path}: {e}");
            assert_eq!(e["kind"], json!(kind), "doc.open {path}: {e}");
        }
    }
}

#[test]
fn doc_open_stays_confined_on_the_tree_fixture() {
    let w = Work::ac4("open");
    let before = snapshot(&w.dir);
    assert_refused(&w, "out/secret.txt", "outside_root");
    assert_refused(&w, "out", "outside_root");
    assert_refused(&w, ".gitignore", "outside_root");
    let ok = open(&w, "self/keep.rs").unwrap_or_else(|e| panic!("doc.open self/keep.rs: {e}"));
    assert_eq!(ok["path"], json!("self/keep.rs"), "{ok}");
    assert_refused(&w, "p", "not_a_file");
    assert_refused(&w, "loop1", "io_error");
    assert_eq!(snapshot(&w.dir), before);
    assert_eq!(fs::read(w.at("outside/secret.txt")).unwrap(), SECRET);
}

#[test]
fn doc_open_of_a_directory_is_still_not_a_file() {
    let w = Work::ac4("dir");
    assert!(w.at("root/sub").is_dir());
    assert_refused(&w, "sub", "not_a_file");
    assert_refused(&w, "self", "not_a_file");
    assert_refused(&w, "self/sub", "not_a_file");
}
