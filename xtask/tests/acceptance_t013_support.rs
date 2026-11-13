//! Shared helpers of the T-013 acceptance tests (included with `#[path]`; as its own test target
//! it holds no tests). It reuses T-005's fixture (`acceptance_t005_support.rs`: toy repositories,
//! environment hygiene) and adds: a runner with a timeout for every process these tests spawn
//! (D-028 (8)), UTC clock readings for bracketing `<now>`, a light toy repository for the
//! `metrics` commands (no cargo build needed), and refusal assertions.
#![allow(dead_code)]

#[path = "acceptance_t005_support.rs"]
mod shared;

pub(crate) use shared::support::*;

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Seconds a spawned `xtask` may take (queue and publish build and gate a toy workspace).
pub const XTASK_TIMEOUT_S: u64 = 900;

/// Run `c` (with T-005's environment hygiene) and kill it after `secs` seconds.
pub fn run_to(c: &mut Command, secs: u64) -> Out {
    hygiene(c);
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap_or_else(|e| panic!("spawn {c:?}: {e}"));
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
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&t1.join().unwrap()).into_owned(),
        stderr: String::from_utf8_lossy(&t2.join().unwrap()).into_owned(),
    }
}

/// The `xtask` binary under test, in `dir`, with a timeout.
pub fn xt(dir: &Path, args: &[&str]) -> Out {
    run_to(Command::new(XTASK).args(args).current_dir(dir), XTASK_TIMEOUT_S)
}

/// A shell command in `dir`, with a timeout.
pub fn sh(dir: &Path, script: &str) -> Out {
    run_to(
        Command::new("sh").args(["-c", script]).current_dir(dir),
        XTASK_TIMEOUT_S,
    )
}

// ---- clock -----------------------------------------------------------------------------------

/// The system clock in whole Unix seconds.
pub fn now_s() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DDTHH:MM:SSZ`.
pub fn ts(secs: i64) -> String {
    let (y, m, d) = civil_from_days(secs.div_euclid(86400));
    let r = secs.rem_euclid(86400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        r / 3600,
        r % 3600 / 60,
        r % 60
    )
}

/// Parse exactly `YYYY-MM-DDTHH:MM:SSZ`; panics otherwise (the record format is pinned).
pub fn parse_ts(s: &str) -> i64 {
    let b = s.as_bytes();
    assert!(
        b.len() == 20
            && b[4] == b'-'
            && b[7] == b'-'
            && b[10] == b'T'
            && b[13] == b':'
            && b[16] == b':'
            && b[19] == b'Z',
        "`{s}` is not YYYY-MM-DDTHH:MM:SSZ"
    );
    let n = |a: usize, z: usize| -> i64 { s[a..z].parse().unwrap() };
    days_from_civil(n(0, 4), n(5, 7), n(8, 10)) * 86400 + n(11, 13) * 3600 + n(14, 16) * 60 + n(17, 19)
}

/// Assert `t` (a record time) lies in the clock bracket `[before, after]`.
pub fn assert_in_bracket(t: &str, before: i64, after: i64, what: &str) {
    let v = parse_ts(t);
    assert!(
        before <= v && v <= after,
        "{what}: time {t} is outside the clock bracket [{}, {}]",
        ts(before),
        ts(after)
    );
}

// ---- refusals --------------------------------------------------------------------------------

/// Assert a refusal: exit 1 and stderr exactly one line starting with `prefix`. Returns it.
pub fn assert_refusal(o: &Out, prefix: &str, what: &str) -> String {
    let lines: Vec<&str> = o.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        o.code == 1 && lines.len() == 1 && lines[0].starts_with(prefix),
        "{what}: expected exit 1 and one stderr line `{prefix}…`; got exit {}\nstdout:\n{}\nstderr:\n{}",
        o.code,
        o.stdout,
        o.stderr
    );
    lines[0].to_owned()
}

/// Assert success (exit 0).
pub fn assert_ok(o: &Out, what: &str) {
    assert_eq!(
        o.code, 0,
        "{what}: expected exit 0\nstdout:\n{}\nstderr:\n{}",
        o.stdout, o.stderr
    );
}

/// The last `n` non-empty lines of `s`.
pub fn last_lines(s: &str, n: usize) -> Vec<String> {
    let v: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    v[v.len().saturating_sub(n)..]
        .iter()
        .map(|l| (*l).to_owned())
        .collect()
}

// ---- toy task files and records --------------------------------------------------------------

/// A v1.1 task file (front matter only plus a log heading).
pub struct TaskSpec<'a> {
    pub id: &'a str,
    pub tier: &'a str,
    pub state: &'a str,
    pub iteration: u64,
    pub budget: Option<u64>,
    pub found_by: &'a str,
    pub origin: Option<&'a str>,
    pub touches: &'a [&'a str],
}

impl<'a> TaskSpec<'a> {
    pub fn new(id: &'a str, tier: &'a str, iteration: u64) -> Self {
        TaskSpec {
            id,
            tier,
            state: "ready",
            iteration,
            budget: Some(500_000),
            found_by: "planner",
            origin: None,
            touches: &["crates/a"],
        }
    }

    pub fn md(&self) -> String {
        let mut t = String::new();
        for p in self.touches {
            t.push_str(&format!("  - {p}\n"));
        }
        let budget = self
            .budget
            .map(|b| format!("budget_tokens: {b}\n"))
            .unwrap_or_default();
        format!(
            "---\nid: {id}\ntitle: \"Toy task {id}\"\ntier: {tier}\nstate: {state}\nowner_role: director\nscorecard_items: []\ndepends_on: []\ntouches:\n{t}{budget}iteration: {it}\nfound_by: {fb}\norigin_task: {origin}\n---\n\n# {id} Toy task\n\n## Log\n### implementer\n<!-- end implementer log -->\n",
            id = self.id,
            tier = self.tier,
            state = self.state,
            it = self.iteration,
            fb = self.found_by,
            origin = self.origin.unwrap_or("null"),
        )
    }
}

/// A complete `run` record (METHODOLOGY §13 fields).
pub fn run_rec(task: &str, tier: Option<&str>, iteration: u64, tokens: Option<u64>, at: &str) -> Value {
    serde_json::json!({
        "kind": "run", "task": task, "iteration": iteration, "run": "R-900",
        "role": if task.starts_with("iter-") { "planner" } else { "implementer" },
        "tier": tier, "started_at": at, "ended_at": at, "wall_s": 0,
        "tokens": tokens, "tool_calls": 1, "gate_attempts": 0, "outcome": "completed",
        "missing_context": "none"
    })
}

/// A `transition` record.
pub fn transition_rec(task: &str, from: &str, to: &str, at: &str) -> Value {
    serde_json::json!({
        "kind": "transition", "task": task, "iteration": 1, "at": at, "from": from, "to": to
    })
}

/// A `join` record.
pub fn join_rec13(
    task: &str,
    at: &str,
    sha: &str,
    fail_first: u64,
    defects: Option<u64>,
    green: bool,
) -> Value {
    serde_json::json!({
        "kind": "join", "task": task, "at": at, "acc_total": 3, "acc_fail_base": 3,
        "base_fail_kind": "fails", "acc_fail_first_join": fail_first, "acc_defects": defects,
        "impl_tests_green": green, "gate_green": fail_first == 0, "tests_sha": sha,
        "joined_sha": sha, "result": if fail_first == 0 { "joined" } else { "acceptance_failed" }
    })
}

/// A 40-hex sha made from `n`.
pub fn sha(n: u64) -> String {
    format!("{n:040x}")
}

// ---- light repository for `metrics` ----------------------------------------------------------

/// A git repository with a `[workspace]` manifest (enough for `xtask metrics`), task files and
/// `docs/decisions.md`; no cargo build.
pub struct Lite {
    pub root: PathBuf,
}

impl Lite {
    pub fn new(name: &str) -> Lite {
        let root = scratch(&format!("t013-{name}")).join("repo");
        write(&root, "Cargo.toml", "[workspace]\nmembers = []\n");
        write(&root, "docs/decisions.md", DECISIONS);
        write(&root, "README.md", "# toy\n");
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "toy base"]);
        Lite { root }
    }

    pub fn task(&self, t: &TaskSpec) {
        write(&self.root, &format!("tasks/{}-toy.md", t.id), &t.md());
    }

    pub fn x(&self, args: &[&str]) -> Out {
        xt(&self.root, args)
    }

    pub fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.root.join("metrics/tasks.jsonl")).unwrap_or_default()
    }

    pub fn records(&self) -> Vec<Value> {
        String::from_utf8(self.bytes())
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Append records directly (fixture data, as earlier tooling or the director wrote them).
    pub fn seed(&self, recs: &[Value]) {
        let mut b = self.bytes();
        for r in recs {
            b.extend_from_slice(serde_json::to_string(r).unwrap().as_bytes());
            b.push(b'\n');
        }
        std::fs::create_dir_all(self.root.join("metrics")).unwrap();
        std::fs::write(self.root.join("metrics/tasks.jsonl"), b).unwrap();
    }

    pub fn queue_merged(&self, tasks: &[&str]) {
        let mut s = String::new();
        for t in tasks {
            s.push_str(&format!(
                "{{\"task\":\"{t}\",\"attempt\":1,\"started_at\":\"2026-11-12T00:00:00Z\",\"ended_at\":\"2026-11-12T01:00:00Z\",\"base_sha\":\"{b}\",\"head_sha\":\"{b}\",\"result\":\"merged\",\"failed_step\":null,\"gate_seconds\":1.0,\"gate_xtask\":\"main\",\"gate_xtask_sha\":\"{b}\",\"regenerated\":[]}}\n",
                b = sha(7)
            ));
        }
        write(&self.root, "metrics/queue.jsonl", &s);
    }

    /// Run `args` and assert a `metrics: refused: ` refusal that leaves the file byte-identical.
    pub fn refused(&self, args: &[&str], what: &str) -> String {
        let before = self.bytes();
        let o = self.x(args);
        let line = assert_refusal(&o, "metrics: refused: ", what);
        assert_eq!(
            self.bytes(),
            before,
            "{what}: the refusal changed metrics/tasks.jsonl"
        );
        line
    }

    /// Run `args`, assert exit 0 and exactly one appended record; return it and the output.
    pub fn appended(&self, args: &[&str], what: &str) -> (Value, Out) {
        let before = self.bytes();
        let o = self.x(args);
        assert_ok(&o, what);
        let after = self.bytes();
        assert!(
            after.starts_with(&before),
            "{what}: earlier bytes of metrics/tasks.jsonl changed"
        );
        let added = String::from_utf8(after[before.len()..].to_vec()).unwrap();
        let lines: Vec<&str> = added.lines().collect();
        assert_eq!(lines.len(), 1, "{what}: expected one appended line, got {added:?}");
        assert!(added.ends_with('\n'), "{what}: appended line has no newline");
        (serde_json::from_str(lines[0]).unwrap(), o)
    }
}

/// `metrics add` arguments: `tokens` `None` means `--tokens-unknown`.
pub fn add_args(task: &str, role: &str, tokens: Option<u64>, extra: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = [
        "metrics",
        "add",
        "--task",
        task,
        "--run",
        "R-100",
        "--role",
        role,
        "--tool-calls",
        "3",
        "--gate-attempts",
        "1",
        "--outcome",
        "completed",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    match tokens {
        Some(n) => a.extend(["--tokens".to_owned(), n.to_string()]),
        None => a.push("--tokens-unknown".to_owned()),
    }
    if !extra.contains(&"--started-at") {
        a.extend(["--started-at".to_owned(), ts(now_s() - 7200)]);
    }
    a.extend(extra.iter().map(|s| (*s).to_owned()));
    a
}

pub fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// This checkout's root (the workspace above the xtask package).
pub fn checkout_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
