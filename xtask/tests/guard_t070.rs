//! T-070 guard tests (test author; ruling m9, D-189): the window procedure of record,
//! `.github/scripts/window-procedure.sh`, is the test author's file and is on the tests branch,
//! so these tests pass on the base and on the joined branch (moved unchanged in what they assert
//! from `acceptance_t070.rs`).
//!
//! - AC-3 (ruling Q-3 B, D-180: structure in the gate, the run by the reviewer and the
//!   `window-linux` job): every line of the R-251 block of `tasks/T-065-window.md` in the
//!   procedure file in order, except the lines named in `R251_REPLACED` (Interface (3); ruling
//!   m2, D-182), which must be absent; the usage line naming the file; and the exit rule for a
//!   run in which no verdict can pass (no `kiln`, no tools on `PATH`; no display, no network).
//!
//! No Unix-only API is used (the files run on macOS and Windows once T-070 has merged).
//! A missing precondition (no `sh`, the T-065 block not found) is a panic that says so.
#![cfg(test)]

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ---- files ---------------------------------------------------------------------------------

/// The repository root (the parent of the `xtask` package).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

const PROCEDURE: &str = ".github/scripts/window-procedure.sh";

/// The text of a file under the root; the file not existing is an assertion failure (it is
/// the subject of the test, not a precondition).
fn subject(rel: &str) -> String {
    let p = root().join(rel);
    assert!(p.is_file(), "{rel} does not exist (T-070 adds it)");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// A file the tests read as an oracle; missing is a precondition failure.
fn oracle(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("precondition: cannot read {rel} (the oracle): {e}"))
}

/// A fresh, empty scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t070-guard")
        .join(name);
    if d.exists() {
        std::fs::remove_dir_all(&d).unwrap();
    }
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---- processes (every one with a timeout, D-028 (8)) ----------------------------------------

struct Out {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
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
        std::thread::sleep(Duration::from_millis(10));
    };
    Out {
        code: status.code(),
        stdout: t1.join().unwrap(),
        stderr: String::from_utf8_lossy(&t2.join().unwrap()).into_owned(),
    }
}

/// The `sh` on `PATH`, as an absolute path (the exit-rule run empties `PATH`).
fn find_sh() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for d in std::env::split_paths(&path) {
        for name in ["sh", "sh.exe"] {
            let p = d.join(name);
            if p.is_file() {
                return p;
            }
        }
    }
    panic!("precondition: no `sh` on PATH (the window procedure is a POSIX sh script)");
}

/// A path as `sh` takes it on every platform (forward slashes; Git Bash reads `C:/…`).
fn sh_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

// ---- AC-3 ------------------------------------------------------------------------------------

/// The R-251 block of `tasks/T-065-window.md`: the ```` ```sh ```` fence after "Window
/// procedure (replaces R-248's in full)" up to its closing fence.
fn r251_block() -> Vec<String> {
    let t = oracle("tasks/T-065-window.md");
    let lines: Vec<&str> = t.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains("Window procedure (replaces R-248's in full)"))
        .unwrap_or_else(|| panic!("precondition: the R-251 entry is not in tasks/T-065-window.md"));
    let open = (at..lines.len())
        .find(|&i| lines[i] == "```sh")
        .unwrap_or_else(|| panic!("precondition: no ```sh fence after the R-251 heading"));
    let close = (open + 1..lines.len())
        .find(|&i| lines[i] == "```")
        .unwrap_or_else(|| panic!("precondition: the R-251 block is not closed"));
    let block: Vec<String> = lines[open + 1..close]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let cases = block.iter().filter(|l| l.starts_with("case_ ")).count();
    assert_eq!(cases, 6, "precondition: the R-251 block has six case lines");
    block
}

/// The lines of the R-251 block the procedure file replaces, and nothing else: the usage line
/// (Interface (3)), and the three lines that take a window run's exit status from `xvfb-run`
/// (ruling m2, D-182: the status is `kiln`'s own, recorded inside the wrapped command): the two
/// window lines of `case_` and AC-3's.
const R251_REPLACED: [&str; 4] = [
    "Usage: sh ac2.sh",
    r#"    xv sh -c 'exec "$0" "$@" 2>"w_'"$n"'.err"' "$K" --root "w_$n" "$@" --script "$s" > "w_$n.out"; wc=$?"#,
    r#"    (cd "w_$n" && xv sh -c 'exec "$0" "$@" 2>"../w_'"$n"'.err"' "$K" "$@" --script "../$s" > "../w_$n.out"); wc=$?"#,
    r#"    xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$K' --root n a.rs 2>n.err" > n.out; e=$?"#,
];

#[test]
fn ac3_procedure_holds_every_line_of_r251_in_order() {
    let block = r251_block();
    let text = subject(PROCEDURE);
    let file: Vec<&str> = text.lines().collect();
    for r in R251_REPLACED {
        assert_eq!(
            block.iter().filter(|l| l.contains(r)).count(),
            1,
            "precondition: R-251's block has the replaced line {r:?} once"
        );
        assert!(
            !file.iter().any(|l| l.contains(r)),
            "{PROCEDURE} still has R-251's line {r:?} (replaced: Interface (3), ruling m2)"
        );
    }
    let mut from = 0;
    for want in block
        .iter()
        .filter(|l| !R251_REPLACED.iter().any(|r| l.contains(r)))
    {
        match (from..file.len()).find(|&i| file[i] == want) {
            Some(i) => from = i + 1,
            None => {
                panic!("{PROCEDURE}: R-251's line is missing, changed or out of order: {want:?}")
            }
        }
    }
    for case in ["ac2", "ac2x3", "dblclick", "scroll100", "ctrlq", "noroot"] {
        assert!(
            file.iter()
                .any(|l| l.starts_with(&format!("case_ {case} "))),
            "{PROCEDURE}: no case {case}"
        );
    }
}

#[test]
fn ac3_usage_line_names_the_file() {
    let text = subject(PROCEDURE);
    assert!(
        text.lines()
            .any(|l| l.contains("Usage:") && l.contains("window-procedure.sh")),
        "{PROCEDURE}: no usage line naming window-procedure.sh"
    );
    assert!(!text.contains("ac2.sh"), "{PROCEDURE} still names ac2.sh");
}

#[test]
fn ac3_exit_status_is_1_when_no_verdict_passes() {
    // No kiln (a path that does not exist) and an empty PATH: nothing the procedure calls exists,
    // so no case can pass, nothing reaches a display or the network, and the script must still
    // run to its end and exit 1 (R-251's block always exits 0).
    let text = subject(PROCEDURE);
    assert!(!text.is_empty());
    let sh = find_sh();
    let d = scratch("exit-rule");
    let empty_path = d.join("empty-path");
    let work = d.join("work");
    std::fs::create_dir_all(&empty_path).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let o = run_to(
        Command::new(&sh)
            .arg(sh_path(&root().join(PROCEDURE)))
            .arg(sh_path(&d.join("no-such-kiln")))
            .arg(sh_path(&work))
            .env("PATH", &empty_path)
            .current_dir(&d),
        600,
    );
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.contains(&"WINDOW CASES FAIL")
            && lines.iter().any(|l| l.starts_with("PLAIN NOT RUN")),
        "the procedure did not run to its end (no `WINDOW CASES FAIL` and `PLAIN NOT RUN`):\n{out}\n{}",
        o.stderr
    );
    for pass in ["WINDOW CASES PASS", "AC-3 PASS"] {
        assert!(
            !lines.contains(&pass),
            "printed {pass} without a kiln:\n{out}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.starts_with("PLAIN PASS")),
        "printed PLAIN PASS without a kiln"
    );
    assert_eq!(
        o.code,
        Some(1),
        "exit status with every verdict failing:\n{out}"
    );
    std::fs::remove_dir_all(&d).unwrap();
}
