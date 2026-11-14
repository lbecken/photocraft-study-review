//! `kiln script` through the built binary (implementer's tests of T-055), against temporary
//! roots under `CARGO_TARGET_TMPDIR`. Nodes are found by role, label or value; no test pins a
//! whole snapshot, a message text or the usage text.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// Every spawned `kiln` is killed and the test fails after this long (D-028 (8)).
const TIMEOUT: Duration = Duration::from_secs(120);

/// The root of ADR-0008 / T-055 (g): `src/a.rs`, plus `b.txt` and a directory `dir`.
const A_RS: &[u8] = b"fn main() {\n    let x = 1;\n}\n";

/// A fresh, empty directory unique to `name`.
#[cfg(test)]
fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("kiln-script-t055")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A fresh root below `dir` (at `dir/<sub>`).
#[cfg(test)]
fn project(dir: &Path, sub: &str) -> PathBuf {
    let root = dir.join(sub);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("dir")).unwrap();
    fs::write(root.join("src/a.rs"), A_RS).unwrap();
    fs::write(root.join("b.txt"), b"b\n").unwrap();
    root
}

#[cfg(test)]
fn wait_or_kill(child: &mut Child) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if child.try_wait().unwrap().is_some() {
            return;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// Run `kiln` with `args` in `cwd` (stdout and stderr to files, so no pipe can fill up).
#[cfg(test)]
fn kiln_in(cwd: &Path, args: &[&str], envs_removed: &[&str]) -> Output {
    let out_path = cwd.join(".stdout");
    let err_path = cwd.join(".stderr");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kiln"));
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(fs::File::create(&out_path).unwrap())
        .stderr(fs::File::create(&err_path).unwrap());
    for name in envs_removed {
        cmd.env_remove(name);
    }
    let mut child = cmd.spawn().unwrap();
    wait_or_kill(&mut child);
    let status = child.wait().unwrap();
    let out = Output {
        status,
        stdout: fs::read(&out_path).unwrap(),
        stderr: fs::read(&err_path).unwrap(),
    };
    fs::remove_file(out_path).unwrap();
    fs::remove_file(err_path).unwrap();
    out
}

/// `kiln script s.jsonl --root <root> --log l.jsonl` in `dir`; (output, log).
#[cfg(test)]
fn script(dir: &Path, root: &str, text: &str) -> (Output, String) {
    fs::write(dir.join("s.jsonl"), text).unwrap();
    let out = kiln_in(
        dir,
        &["script", "s.jsonl", "--root", root, "--log", "l.jsonl"],
        &[],
    );
    let log = fs::read_to_string(dir.join("l.jsonl")).unwrap_or_default();
    (out, log)
}

#[cfg(test)]
fn lines(out: &Output) -> Vec<Value> {
    String::from_utf8(out.stdout.clone())
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The response with this `id`.
#[cfg(test)]
fn by_id<'a>(responses: &'a [Value], id: &str) -> &'a Value {
    responses
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no response {id}"))
}

/// Every node of a snapshot's tree, in pre-order.
#[cfg(test)]
fn nodes(node: &Value) -> Vec<&Value> {
    let mut out = vec![node];
    for child in node["children"].as_array().unwrap() {
        out.extend(nodes(child));
    }
    out
}

/// The one node of `snapshot` (a response) with this label.
#[cfg(test)]
fn labelled<'a>(snapshot: &'a Value, label: &str) -> &'a Value {
    let found: Vec<&Value> = nodes(&snapshot["result"]["root"])
        .into_iter()
        .filter(|n| n["label"] == label)
        .collect();
    assert_eq!(found.len(), 1, "nodes labelled {label}");
    found[0]
}

#[cfg(test)]
fn child_values(node: &Value) -> Vec<String> {
    node["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["value"].as_str().unwrap_or("").to_owned())
        .collect()
}

/// The text of a node's painted rows, one string per row.
#[cfg(test)]
fn row_texts(node: &Value) -> Vec<String> {
    node["rows"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .unwrap()
                        .iter()
                        .map(|run| run["text"].as_str().unwrap().to_owned())
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The responses of `kiln script` for `text` on `root` (in `dir`), as parsed lines.
#[cfg(test)]
fn replay(dir: &Path, root: &str, text: &str) -> Vec<Value> {
    let (out, _) = script(dir, root, text);
    lines(&out)
}

#[test]
fn control_requests_get_control_responses_and_a_replayable_log() {
    let dir = fresh("control");
    project(&dir, "proj");
    project(&dir, "copy");
    let text = concat!(
        r#"{"id":1,"command":"doc.list"}"#,
        "\n",
        r#"{"id":2,"command":"doc.open","params":{"path":"src/a.rs"}}"#,
        "\n",
        r#"{"id":3,"command":"doc.open","params":{"path":"src/missing.rs"}}"#,
        "\n\n   \n",
        "not json\n[1]\n",
        r#"{"id":1.5,"ui":"snapshot"}"#,
        "\n",
        r#"{"id":"x","command":"doc.insert","params":{"doc":1}}"#,
        "\n",
        r#"{"id":4,"command":"doc.insert","params":{"doc":1,"offset":0,"text":"// hi\n"}}"#,
        "\n",
        r#"{"id":5,"command":"syntax.highlight","params":{"doc":1}}"#,
        "\n",
        r#"{"id":6,"command":"doc.save","params":{"doc":1}}"#,
        "\n",
        r#"{"id":7,"command":"doc.list"}"#,
    );
    let (out, log) = script(&dir, "proj", text);
    assert_eq!(out.status.code(), Some(3));
    // One response per non-blank line, in order; kinds as the control protocol gives them
    // (the byte-for-byte comparison with `kiln_control::Host` is kiln-ui's unit test).
    let r = lines(&out);
    let kinds: Vec<&str> = r
        .iter()
        .map(|v| v["error"]["kind"].as_str().unwrap_or("ok"))
        .collect();
    assert_eq!(
        kinds,
        [
            "ok",
            "ok",
            "not_found",
            "syntax",
            "not_an_object",
            "bad_id",
            "schema",
            "ok",
            "ok",
            "ok",
            "ok"
        ]
    );
    let logged: Vec<String> = log
        .lines()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["command"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        logged,
        [
            "doc.list",
            "doc.open",
            "doc.insert",
            "syntax.highlight",
            "doc.save",
            "doc.list"
        ]
    );
    let mut saved = b"// hi\n".to_vec();
    saved.extend_from_slice(A_RS);
    assert_eq!(fs::read(dir.join("proj/src/a.rs")).unwrap(), saved);
    // The log run again on a copy of the initial root writes the same file and the same log.
    let (again, again_log) = script(&dir, "copy", &log);
    assert_eq!(again.status.code(), Some(0));
    assert_eq!(again_log, log);
    assert_eq!(fs::read(dir.join("copy/src/a.rs")).unwrap(), saved);

    // Only successful requests: exit 0.
    let (out, _) = script(&dir, "proj", r#"{"command":"doc.list"}"#);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn the_shell_shows_documents_opened_by_control_requests() {
    let dir = fresh("shell");
    project(&dir, "proj");
    let (out, _) = script(
        &dir,
        "proj",
        concat!(
            r#"{"id":"s0","ui":"snapshot"}"#,
            "\n",
            r#"{"command":"doc.open","params":{"path":"src/a.rs"}}"#,
            "\n",
            r#"{"command":"doc.open","params":{"path":"b.txt"}}"#,
            "\n",
            r#"{"id":"s1","ui":"snapshot"}"#,
        ),
    );
    assert_eq!(out.status.code(), Some(0));
    let r = lines(&out);
    let s0 = by_id(&r, "s0");
    assert_eq!(labelled(s0, "Path")["role"], "text_input");
    assert_eq!(labelled(s0, "Open")["role"], "button");
    assert!(child_values(labelled(s0, "Documents")).is_empty());
    assert!(
        labelled(s0, "Message")["value"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );
    let s1 = by_id(&r, "s1");
    assert_eq!(
        child_values(labelled(s1, "Documents")),
        ["src/a.rs", "b.txt"]
    );
    assert!(s1["result"]["frames"].as_u64() > s0["result"]["frames"].as_u64());
}

/// Open `path` through the UI: focus `Path`, type, then click `Open` or press Enter.
#[cfg(test)]
fn open_through_ui(path: &str, enter: bool) -> String {
    let last = if enter {
        r#"{"id":"go","ui":"key","key":"Enter"}"#.to_owned()
    } else {
        r#"{"id":"go","ui":"click","target":{"label":"Open"}}"#.to_owned()
    };
    format!(
        "{}\n{}\n{last}\n{}\n{}\n",
        r#"{"id":"f","ui":"focus","target":{"label":"Path"}}"#,
        serde_json::json!({"id": "t", "ui": "type", "text": path}),
        r#"{"id":"s","ui":"snapshot"}"#,
        r#"{"id":"l","command":"doc.list"}"#,
    )
}

#[test]
fn the_ui_opens_through_the_session_by_click_and_by_enter() {
    let dir = fresh("open");
    project(&dir, "proj");
    project(&dir, "replay");
    for enter in [false, true] {
        let (out, log) = script(&dir, "proj", &open_through_ui("src/a.rs", enter));
        assert_eq!(out.status.code(), Some(0), "enter: {enter}");
        let r = lines(&out);
        for id in ["f", "t", "go"] {
            let result = &by_id(&r, id)["result"];
            assert_eq!(result["settled"], true);
            assert!(result["frames"].as_u64().unwrap() <= 60);
        }
        let s = by_id(&r, "s");
        assert_eq!(child_values(labelled(s, "Documents")), ["src/a.rs"]);
        let docs = &by_id(&r, "l")["result"]["docs"];
        assert_eq!(docs[0]["path"], "src/a.rs");
        assert_eq!(docs[0]["doc"], 1);
        let first: Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
        assert_eq!(first["command"], "doc.open");
        assert_eq!(first["params"]["path"], "src/a.rs");
        // The log replayed on a copy of the initial root gives the same list.
        let replayed = replay(
            &dir,
            "replay",
            &format!("{log}{{\"id\":\"l\",\"command\":\"doc.list\"}}\n"),
        );
        assert_eq!(replayed.last().unwrap()["result"]["docs"], *docs);
    }
}

#[test]
fn refused_opens_are_shown_with_the_kind_and_not_logged() {
    let dir = fresh("refused");
    let root = project(&dir, "proj");
    fs::write(dir.join("outside.txt"), b"o\n").unwrap();
    std::os::unix::fs::symlink(dir.join("outside.txt"), root.join("link.txt")).unwrap();
    for (path, kind) in [
        ("src/missing.rs", "not_found"),
        ("dir", "not_a_file"),
        ("link.txt", "outside_root"),
        ("../x", "invalid_path"),
    ] {
        // The same path as a control request reports this kind.
        let control = replay(
            &dir,
            "proj",
            &serde_json::json!({"command": "doc.open", "params": {"path": path}}).to_string(),
        );
        assert_eq!(control[0]["error"]["kind"], kind, "{path}");

        let (out, log) = script(&dir, "proj", &open_through_ui(path, false));
        assert_eq!(out.status.code(), Some(0), "{path}");
        let r = lines(&out);
        let s = by_id(&r, "s");
        assert!(child_values(labelled(s, "Documents")).is_empty(), "{path}");
        let message = labelled(s, "Message")["value"].as_str().unwrap().to_owned();
        assert!(message.starts_with(kind), "{path}: {message} / {kind}");
        assert!(!log.contains("doc.open"), "{path}");
    }
    // A later successful open clears the message.
    let text = format!(
        "{}{}\n{}\n{}\n{}\n{}\n",
        open_through_ui("nope", false),
        r#"{"ui":"focus","target":{"label":"Path"}}"#,
        r#"{"ui":"key","key":"A","modifiers":["ctrl"]}"#,
        r#"{"ui":"type","text":"b.txt"}"#,
        r#"{"ui":"click","target":{"label":"Open"}}"#,
        r#"{"id":"after","ui":"snapshot"}"#,
    );
    let (out, _) = script(&dir, "proj", &text);
    let r = lines(&out);
    assert!(
        labelled(by_id(&r, "after"), "Message")["value"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );
}

#[test]
fn idle_frames_change_nothing() {
    let dir = fresh("idle");
    project(&dir, "proj");
    let base = concat!(
        r#"{"command":"doc.open","params":{"path":"src/a.rs"}}"#,
        "\n",
        r#"{"id":"a","ui":"snapshot"}"#,
        "\n"
    );
    let (out1, log1) = script(&dir, "proj", base);
    let (out2, log2) = script(
        &dir,
        "proj",
        &format!(
            "{base}{}\n{}\n",
            r#"{"id":"idle","ui":"frames","count":600}"#, r#"{"id":"b","ui":"snapshot"}"#
        ),
    );
    assert_eq!(log1, log2);
    let r1 = lines(&out1);
    let r2 = lines(&out2);
    assert_eq!(by_id(&r2, "idle")["result"]["frames"], 600);
    assert_eq!(by_id(&r2, "idle")["result"]["settled"], true);
    assert_eq!(by_id(&r1, "a"), by_id(&r2, "a"));
    assert_eq!(
        by_id(&r2, "a")["result"]["root"],
        by_id(&r2, "b")["result"]["root"]
    );
}

#[cfg(test)]
fn mixed_script() -> String {
    let mut text = String::new();
    for i in 0..60 {
        text.push_str(&format!(
            "{{\"command\":\"doc.open\",\"params\":{{\"path\":\"f{i:02}.txt\"}}}}\n"
        ));
    }
    text.push_str(&open_through_ui("src/a.rs", true));
    for line in [
        r#"{"id":"at","ui":"click","target":{"label":"Path"},"at":{"row":0,"col":3}}"#,
        r#"{"ui":"key","key":"ArrowRight","modifiers":["shift"]}"#,
        r#"{"id":"sel","ui":"snapshot"}"#,
        r#"{"id":"sc","ui":"scroll","target":{"value":"f00.txt"},"direction":"down","steps":3}"#,
        r#"{"id":"scrolled","ui":"snapshot"}"#,
        r#"{"ui":"click","target":{"label":"Open"},"button":"secondary"}"#,
        r#"{"ui":"frames","count":5}"#,
        r#"{"command":"doc.list"}"#,
        r#"{"id":"end","ui":"snapshot"}"#,
    ] {
        text.push_str(line);
        text.push('\n');
    }
    text
}

#[test]
fn the_same_script_on_copies_prints_the_same_bytes() {
    let dir = fresh("determinism");
    let mut outputs = Vec::new();
    for sub in ["one", "deeper/two"] {
        let root = project(&dir, sub);
        for i in 0..60 {
            fs::write(root.join(format!("f{i:02}.txt")), b"x\n").unwrap();
        }
        let (out, log) = script(&dir, sub, &mixed_script());
        assert_eq!(out.status.code(), Some(0));
        outputs.push((out.stdout, log));
    }
    assert_eq!(outputs[0], outputs[1]);

    let r: Vec<Value> = String::from_utf8(outputs[0].0.clone())
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // The click with `at` put the caret before char 3; shift+right selected one char.
    let path = labelled(by_id(&r, "sel"), "Path");
    assert_eq!(path["selection"]["anchor"], 3);
    assert_eq!(path["selection"]["head"], 4);
    assert_eq!(path["focused"], true);
    // Scrolling moved the first document out of the visible part of the list.
    let first_visible = |snapshot: &Value| {
        labelled(snapshot, "Documents")["children"]
            .as_array()
            .unwrap()
            .iter()
            .position(|c| c.get("offscreen").is_none())
    };
    let before = first_visible(by_id(&r, "sel")).unwrap();
    let after = first_visible(by_id(&r, "scrolled")).unwrap();
    assert!(after > before, "{before} -> {after}");
    let docs = labelled(by_id(&r, "scrolled"), "Documents");
    let painted: Vec<String> = docs["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c.get("offscreen").is_none())
        .flat_map(row_texts)
        .collect();
    assert!(!painted.iter().any(|t| t == "f00.txt"));
    // Every action other than `frames` settled within the cap of 60 frames.
    for resp in &r {
        let result = &resp["result"];
        if result.get("settled").is_some() && result["frames"] != 5 {
            assert_eq!(result["settled"], true, "{resp}");
            assert!(result["frames"].as_u64().unwrap() <= 60, "{resp}");
        }
    }
}

#[test]
fn ui_refusals_have_their_kind_and_change_nothing() {
    let dir = fresh("refusals");
    project(&dir, "proj");
    let setup = concat!(
        r#"{"command":"doc.open","params":{"path":"src/a.rs"}}"#,
        "\n",
        r#"{"command":"doc.open","params":{"path":"b.txt"}}"#,
        "\n",
        r#"{"id":"before","ui":"snapshot"}"#,
        "\n"
    );
    let cases = [
        ("bad_request", r#"{"ui":"snapshot","extra":1}"#),
        ("bad_request", r#"{"ui":"focus"}"#),
        ("bad_request", r#"{"ui":"frames","count":0}"#),
        ("bad_request", r#"{"ui":"frames","count":601}"#),
        (
            "bad_request",
            r#"{"ui":"key","key":"A","modifiers":["hyper"]}"#,
        ),
        ("bad_request", r#"{"ui":"key","key":"NoSuchKey"}"#),
        ("bad_request", r#"{"ui":"focus","target":{}}"#),
        (
            "bad_request",
            r#"{"ui":"scroll","target":{"label":"Documents"},"direction":"in"}"#,
        ),
        ("bad_request", r#"{"ui":"zoom","extra":1}"#),
        ("unknown_action", r#"{"ui":"zoom"}"#),
        (
            "no_target",
            r#"{"ui":"click","target":{"label":"Nothing"}}"#,
        ),
        (
            "ambiguous_target",
            r#"{"ui":"click","target":{"value_contains":"."}}"#,
        ),
        ("not_actionable", r#"{"ui":"type","text":"x"}"#),
        (
            "not_actionable",
            r#"{"ui":"focus","target":{"label":"Open"}}"#,
        ),
        (
            "not_actionable",
            r#"{"ui":"click","target":{"label":"Open"},"at":{"row":9,"col":0}}"#,
        ),
        (
            "not_actionable",
            r#"{"ui":"scroll","target":{"label":"Open"},"direction":"down"}"#,
        ),
    ];
    let mut text = setup.to_owned();
    for (i, (_, line)) in cases.iter().enumerate() {
        let mut v: Value = serde_json::from_str(line).unwrap();
        v["id"] = Value::from(i);
        text.push_str(&format!("{v}\n"));
    }
    text.push_str(r#"{"id":"after","ui":"snapshot"}"#);
    let (out, log) = script(&dir, "proj", &text);
    let (_, log_without) = script(&dir, "proj", setup);
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(log, log_without);
    let r = lines(&out);
    for (i, (kind, line)) in cases.iter().enumerate() {
        let resp = by_id_n(&r, i);
        assert_eq!(resp["ok"], false, "{line}");
        assert_eq!(resp["error"]["code"], "ui", "{line}");
        assert_eq!(resp["error"]["kind"], *kind, "{line}");
    }
    assert_eq!(by_id(&r, "before")["result"], by_id(&r, "after")["result"]);
}

#[cfg(test)]
fn by_id_n(responses: &[Value], id: usize) -> &Value {
    responses.iter().find(|r| r["id"] == id).unwrap()
}

#[test]
fn command_line_errors_exit_2_with_nothing_on_stdout() {
    let dir = fresh("cli");
    project(&dir, "proj");
    fs::write(dir.join("s.jsonl"), r#"{"command":"doc.list"}"#).unwrap();
    for args in [
        &["script", "s.jsonl", "--bogus"][..],
        &["script", "s.jsonl", "--root", "proj", "--memory"],
        &["script", "s.jsonl", "--size", "199x800"],
        &["script", "s.jsonl", "--size", "1280x8193"],
        &["script", "s.jsonl", "--size", "1280*800"],
        &["script", "s.jsonl", "--log", "a", "--log", "b"],
        &["script"],
        &["script", "s.jsonl", "t.jsonl"],
        &["script", "missing.jsonl", "--root", "proj"],
        &["script", "s.jsonl", "--root", "nowhere"],
        &[
            "script",
            "s.jsonl",
            "--root",
            "proj",
            "--log",
            "nodir/l.jsonl",
        ],
    ] {
        let out = kiln_in(&dir, args, &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!out.stderr.is_empty(), "{args:?}");
    }
    for args in [
        &["--help"][..],
        &["-h"],
        &["script", "--help"],
        &["script", "s.jsonl", "-h"],
    ] {
        let out = kiln_in(&dir, args, &[]);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert!(!out.stdout.is_empty(), "{args:?}");
    }
    // Options after or before the positional; --memory and --size accepted.
    let out = kiln_in(
        &dir,
        &["script", "--memory", "--size", "200x8192", "s.jsonl"],
        &[],
    );
    assert_eq!(out.status.code(), Some(0));
    // A bad root is checked before the log file is created.
    let out = kiln_in(
        &dir,
        &[
            "script",
            "s.jsonl",
            "--root",
            "nowhere",
            "--log",
            "made.jsonl",
        ],
        &[],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(!dir.join("made.jsonl").exists());
}

#[test]
fn no_display_is_needed_and_every_control_is_named() {
    let dir = fresh("display");
    project(&dir, "proj");
    let text = format!(
        "{}{}\n",
        open_through_ui("src/a.rs", false),
        r#"{"id":"s2","ui":"snapshot"}"#
    );
    fs::write(dir.join("s.jsonl"), &text).unwrap();
    let args = ["script", "s.jsonl", "--root", "proj"];
    let with = kiln_in(&dir, &args, &[]);
    let without = kiln_in(&dir, &args, &["DISPLAY", "WAYLAND_DISPLAY"]);
    assert_eq!(with.status.code(), Some(0));
    assert_eq!(with.stdout, without.stdout);
    for resp in lines(&without) {
        if resp["result"].get("root").is_none() {
            continue;
        }
        for node in nodes(&resp["result"]["root"]) {
            if node["role"] == "button" || node["role"] == "text_input" {
                assert!(!node["label"].as_str().unwrap_or("").is_empty());
            }
        }
    }
}
