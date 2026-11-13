//! T-011 implementer's integration tests for `--memory-seed` through the built binary: hostile
//! manifests (deep, large, long reasons) end in a structured exit, the `--log` of a seeded run
//! replays to the same responses, and `--memory-seed` lists like `--root` on an equal tree.
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("memory_seed")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `kiln-cli args` in `cwd`; stdout/stderr go to files in `out_dir` so no pipe can block.
fn kiln(args: &[&str], cwd: &Path, out_dir: &Path) -> Output {
    let (o, e) = (out_dir.join("stdout"), out_dir.join("stderr"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&o).unwrap())
        .stderr(std::fs::File::create(&e).unwrap())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(st) = child.try_wait().unwrap() {
            break st;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli did not exit within {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    Output {
        status,
        stdout: std::fs::read(o).unwrap(),
        stderr: std::fs::read(e).unwrap(),
    }
}

fn setup(name: &str) -> (PathBuf, PathBuf) {
    let work = fresh(name);
    let out = fresh(&format!("{name}-out"));
    (work, out)
}

#[test]
fn hostile_manifests_exit_2_without_panic() {
    let (work, out_dir) = setup("hostile");
    let long_key = "k".repeat(10_000);
    let cases = [
        format!(
            "{{\"files\":{}{}}}",
            "[".repeat(1_000_000),
            "]".repeat(1_000_000)
        ),
        "{\"a\":".repeat(100_000),
        format!("{{\"files\":[],\"{long_key}\":1}}"),
        format!(
            "{{\"files\":[{{\"path\":\"{}\",\"text\":\"\"}}]}}",
            "a/".repeat(5_000)
        ),
    ];
    for (i, manifest) in cases.iter().enumerate() {
        let name = format!("h{i}.json");
        std::fs::write(work.join(&name), manifest).unwrap();
        let out = kiln(
            &["exec", "doc.list", "{}", "--memory-seed", &name],
            &work,
            &out_dir,
        );
        assert_eq!(out.status.code(), Some(2), "case {i}");
        assert!(out.stdout.is_empty());
        let err = String::from_utf8(out.stderr).unwrap();
        let first = err.lines().next().unwrap();
        assert!(
            first.starts_with(&format!("kiln-cli: bad --memory-seed {name}: ")),
            "{first:.300}"
        );
        // D-031: stderr lines are capped.
        assert!(err.lines().all(|l| l.chars().count() <= 200), "case {i}");
    }
}

#[test]
fn many_files_are_seeded() {
    let (work, out_dir) = setup("many");
    let entries: Vec<String> = (0..5_000)
        .map(|i| format!("{{\"path\":\"d{}/f{i}.txt\",\"text\":\"{i}\"}}", i % 37))
        .collect();
    std::fs::write(
        work.join("m.json"),
        format!("{{\"files\":[{}]}}", entries.join(",")),
    )
    .unwrap();
    let out = kiln(
        &[
            "exec",
            "doc.open",
            r#"{"path":"d4/f4999.txt"}"#,
            "--memory-seed",
            "m.json",
        ],
        &work,
        &out_dir,
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["len_chars"], 4);
}

#[test]
fn seeded_log_replays_to_the_same_responses() {
    let (work, out_dir) = setup("log");
    std::fs::write(
        work.join("m.json"),
        r#"{"files":[{"path":"a.txt","text":"one\r\n"},{"path":"b/c.txt","hex":"feff0041"}]}"#,
    )
    .unwrap();
    let script = concat!(
        r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
        "\n",
        r#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":3,"text":" two"}}"#,
        "\n",
        r#"{"id":3,"command":"doc.save","params":{"doc":1}}"#,
        "\n",
        r#"{"id":4,"command":"doc.open","params":{"path":"b/c.txt"}}"#,
        "\n",
        r#"{"id":5,"command":"doc.open","params":{"path":"nope"}}"#,
        "\n",
        r#"{"id":6,"command":"doc.info","params":{"doc":2}}"#,
        "\n",
    );
    std::fs::write(work.join("s.jsonl"), script).unwrap();
    let first = kiln(
        &[
            "run",
            "s.jsonl",
            "--memory-seed",
            "m.json",
            "--log",
            "l.jsonl",
        ],
        &work,
        &out_dir,
    );
    assert_eq!(first.status.code(), Some(3));
    let replay = kiln(
        &["run", "l.jsonl", "--memory-seed", "m.json"],
        &work,
        &out_dir,
    );
    assert_eq!(replay.status.code(), Some(0));
    let ok_lines: Vec<String> = String::from_utf8(first.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.contains(r#""ok":true"#))
        .map(|l| {
            let mut v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["id"] = serde_json::Value::Null;
            v.to_string()
        })
        .collect();
    let replayed: Vec<String> = String::from_utf8(replay.stdout)
        .unwrap()
        .lines()
        .map(|l| {
            let mut v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["id"] = serde_json::Value::Null;
            v.to_string()
        })
        .collect();
    assert_eq!(ok_lines, replayed);
    assert_eq!(replayed.len(), 5);
}

#[test]
fn doc_list_and_open_match_root() {
    let (work, out_dir) = setup("match");
    let root = work.join("root");
    std::fs::create_dir_all(root.join("x/y")).unwrap();
    std::fs::write(root.join("x/y/z.rs"), "fn z() {}\n").unwrap();
    std::fs::write(root.join("top"), b"\xef\xbb\xbfT\r").unwrap();
    std::fs::write(
        work.join("m.json"),
        r#"{"files":[{"path":"top","hex":"EFBBBF540D"},{"path":"x/y/z.rs","text":"fn z() {}\n"}]}"#,
    )
    .unwrap();
    let script = concat!(
        r#"{"id":1,"command":"doc.open","params":{"path":"x/y/z.rs"}}"#,
        "\n",
        r#"{"id":2,"command":"doc.open","params":{"path":"top"}}"#,
        "\n",
        r#"{"id":3,"command":"doc.open","params":{"path":"x/y"}}"#,
        "\n",
        r#"{"id":4,"command":"doc.list"}"#,
        "\n",
    );
    std::fs::write(work.join("s.jsonl"), script).unwrap();
    let mem = kiln(
        &["run", "s.jsonl", "--memory-seed", "m.json"],
        &work,
        &out_dir,
    );
    let disk = kiln(&["run", "s.jsonl", "--root", "root"], &work, &out_dir);
    assert_eq!(mem.status.code(), disk.status.code());
    assert_eq!(
        String::from_utf8(mem.stdout).unwrap(),
        String::from_utf8(disk.stdout).unwrap()
    );
}
