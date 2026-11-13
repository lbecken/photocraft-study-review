//! T-011 acceptance tests: `kiln-cli --memory-seed <manifest>` seeds the in-memory file system.
//! Written from the task specification before the implementation (experiment arm, METHODOLOGY
//! §6). Every test drives the built `kiln-cli` binary as a subprocess; on the base the option
//! does not exist, so each test fails there by assertion (usage error instead of the expected
//! behaviour), not by compilation.
//!
//! Oracles: AC-1 differential (the same script with `--memory-seed` and with `--root` on an
//! equal directory), AC-2 byte comparison (reopened text and encoding, encoded by the naive
//! encoder below, against the disk run's files), AC-3 / AC-4 exact values. Refusal message
//! texts are not pinned except where the spec pins them (the `bad --memory-seed` prefix and
//! the `--root`/`--memory-seed` usage message); protocol errors are compared by `code`/`kind`
//! or differentially.
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)). Generated data comes from
//! fixed seeds.
#![cfg(test)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

// ---------------------------------------------------------------------------------------------
// Process and file helpers.

const TIMEOUT: Duration = Duration::from_secs(60);

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_kiln-cli")
}

/// A fresh, empty directory unique to `name` (and this process) under `CARGO_TARGET_TMPDIR`.
fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t011")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wait_or_kill(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli did not exit within {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Run `kiln-cli args` in `cwd` with stdin closed, killed after [`TIMEOUT`].
fn kiln(args: &[&str], cwd: &Path) -> Output {
    let mut child = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let drain = |r: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut r) = r {
                let _ = r.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out_t = drain(child.stdout.take().map(|r| Box::new(r) as _));
    let err_t = drain(child.stderr.take().map(|r| Box::new(r) as _));
    let status = wait_or_kill(&mut child);
    Output {
        status,
        stdout: out_t.join().unwrap(),
        stderr: err_t.join().unwrap(),
    }
}

fn code(out: &Output) -> i32 {
    out.status
        .code()
        .expect("kiln-cli exited normally (no signal)")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn first_stderr_line(out: &Output) -> String {
    stderr(out).lines().next().unwrap_or("").to_owned()
}

fn write_tree(root: &Path, files: &[(String, Vec<u8>)]) {
    for (path, bytes) in files {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, bytes).unwrap();
    }
}

/// Every regular file under `root` (relative `/`-separated path → bytes).
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap();
                let rel: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.insert(rel.join("/"), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn hex(bytes: &[u8], upper: bool) -> String {
    bytes
        .iter()
        .map(|b| {
            if upper {
                format!("{b:02X}")
            } else {
                format!("{b:02x}")
            }
        })
        .collect()
}

/// A manifest for `files`: `text` when `use_text(i)` and the bytes are UTF-8, else `hex`.
fn manifest(files: &[(String, Vec<u8>)], use_text: impl Fn(usize) -> bool) -> String {
    let entries: Vec<Value> = files
        .iter()
        .enumerate()
        .map(|(i, (path, bytes))| match std::str::from_utf8(bytes) {
            Ok(text) if use_text(i) => json!({"path": path, "text": text}),
            _ => json!({"path": path, "hex": hex(bytes, i % 2 == 1)}),
        })
        .collect();
    json!({ "files": entries }).to_string()
}

fn parse_lines(s: &str) -> Vec<Value> {
    s.lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect()
}

/// The response whose `id` is `id`.
fn by_id(responses: &[Value], id: u64) -> &Value {
    responses
        .iter()
        .find(|r| r["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response with id {id}"))
}

// ---------------------------------------------------------------------------------------------
// Deterministic generator (splitmix64) and the naive encoder.

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

const ENCODINGS: [&str; 4] = ["utf-8", "utf-8-bom", "utf-16le", "utf-16be"];

/// The bytes of `text` saved as `encoding` (BOM first, then the text in that encoding).
fn naive_encode(text: &str, encoding: &str) -> Vec<u8> {
    match encoding {
        "utf-8" => text.as_bytes().to_vec(),
        "utf-8-bom" => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        "utf-16le" => {
            let mut out = vec![0xFF, 0xFE];
            text.encode_utf16()
                .for_each(|u| out.extend_from_slice(&u.to_le_bytes()));
            out
        }
        "utf-16be" => {
            let mut out = vec![0xFE, 0xFF];
            text.encode_utf16()
                .for_each(|u| out.extend_from_slice(&u.to_be_bytes()));
            out
        }
        other => panic!("unknown encoding {other}"),
    }
}

/// Pieces of generated text: ASCII, multi-byte, astral (surrogate pairs in UTF-16), no U+FEFF.
const WORDS: [&str; 8] = ["fn", "x", "é", "日本", "😀", "let a = 1;", "ü", " "];

fn gen_text(rng: &mut Rng, eol: &str) -> String {
    let lines = rng.below(5);
    let mut out = String::new();
    for _ in 0..lines {
        for _ in 0..rng.below(4) {
            out.push_str(rng.pick(&WORDS));
        }
        out.push_str(match eol {
            "mixed" => rng.pick(&["\n", "\r\n", "\r"]),
            e => e,
        });
    }
    if rng.chance(50) {
        out.push_str(rng.pick(&WORDS));
    }
    out
}

/// One generated file: its bytes and whether it is decodable.
struct GenFile {
    path: String,
    bytes: Vec<u8>,
    encoding: Option<&'static str>,
}

/// What the generated sets covered (so the generator cannot silently lose a case).
#[derive(Default)]
struct Coverage {
    encodings: BTreeMap<&'static str, usize>,
    eols: BTreeMap<&'static str, usize>,
    nested: usize,
    undecodable: usize,
}

fn gen_files(rng: &mut Rng, set: usize, cov: &mut Coverage) -> Vec<GenFile> {
    const DIRS: [&str; 5] = ["", "", "src/", "src/deep/", "docs/a/b/"];
    // 1–5 files per set (AC-1), the undecodable one included where there is one.
    let n = 1 + rng.below(if set.is_multiple_of(3) { 4 } else { 5 });
    let mut files = Vec::new();
    for i in 0..n {
        let dir = rng.pick(&DIRS);
        if !dir.is_empty() {
            cov.nested += 1;
        }
        let path = format!("{dir}f{i}.txt");
        let encoding = ENCODINGS[(set + i) % 4];
        let eol = *rng.pick(&["\n", "\r\n", "\r", "mixed"]);
        *cov.encodings.entry(encoding).or_default() += 1;
        *cov.eols
            .entry(match eol {
                "\n" => "lf",
                "\r\n" => "crlf",
                "\r" => "cr",
                _ => "mixed",
            })
            .or_default() += 1;
        let text = gen_text(rng, eol);
        files.push(GenFile {
            path,
            bytes: naive_encode(&text, encoding),
            encoding: Some(encoding),
        });
    }
    // One undecodable file in every third set (never starting with a BOM).
    if set.is_multiple_of(3) {
        cov.undecodable += 1;
        let bytes: &[u8] = rng.pick(&[&b"\xff"[..], b"ab\xc3(cd", b"x\r\n\xfe\x80"]);
        files.push(GenFile {
            path: format!("{}bad.bin", rng.pick(&DIRS)),
            bytes: bytes.to_vec(),
            encoding: None,
        });
    }
    files
}

/// The request ids of the final `doc.info` / `doc.text` of each decodable file (by path), and
/// of the final `doc.open` of the undecodable one.
struct Finals {
    info_text: Vec<(String, u64, u64)>,
    bad_open: Option<(String, u64)>,
}

/// A script that opens every decodable file, edits (insert/delete/replace), saves, undoes and
/// closes with or without `discard`; then closes, reopens and reads every file. Doc ids are
/// predicted: 1.. for the first opens, then one new id per reopen (ids are never reused).
fn gen_script(rng: &mut Rng, files: &[GenFile]) -> (String, Finals) {
    let mut lines = Vec::new();
    let mut next_id = 0u64;
    let mut req = |lines: &mut Vec<String>, command: &str, params: Value| -> u64 {
        next_id += 1;
        lines.push(json!({"id": next_id, "command": command, "params": params}).to_string());
        next_id
    };
    let good: Vec<&GenFile> = files.iter().filter(|f| f.encoding.is_some()).collect();
    // Phase 1: open; char lengths tracked while they are known.
    let mut lens: Vec<Option<usize>> = Vec::new();
    for f in &good {
        req(&mut lines, "doc.open", json!({"path": f.path}));
        let text = decode_naive(&f.bytes, f.encoding.unwrap());
        lens.push(Some(text.chars().count()));
    }
    // Phase 2: per document, edits, save, undo, close.
    for (i, len) in lens.iter_mut().enumerate() {
        let doc = (i + 1) as u64;
        let edit = |rng: &mut Rng,
                    lines: &mut Vec<String>,
                    len: &mut Option<usize>,
                    req: &mut dyn FnMut(&mut Vec<String>, &str, Value) -> u64| {
            let Some(l) = *len else {
                req(
                    lines,
                    "doc.insert",
                    json!({"doc": doc, "offset": 0, "text": "Z"}),
                );
                return;
            };
            let piece = format!(
                "{}{}",
                rng.pick(&WORDS),
                rng.pick(&["", "\n", "\r\n", "\r"])
            );
            let plen = piece.chars().count();
            match rng.below(3) {
                0 => {
                    let offset = rng.below(l + 1);
                    req(
                        lines,
                        "doc.insert",
                        json!({"doc": doc, "offset": offset, "text": piece}),
                    );
                    *len = Some(l + plen);
                }
                1 if l > 0 => {
                    let start = rng.below(l);
                    let end = start + 1 + rng.below(l - start);
                    req(
                        lines,
                        "doc.delete",
                        json!({"doc": doc, "start": start, "end": end}),
                    );
                    *len = Some(l - (end - start));
                }
                _ => {
                    let start = rng.below(l + 1);
                    let end = start + rng.below(l - start + 1);
                    req(
                        lines,
                        "doc.replace",
                        json!({"doc": doc, "start": start, "end": end, "text": piece}),
                    );
                    *len = Some(l - (end - start) + plen);
                }
            }
        };
        for _ in 0..1 + rng.below(3) {
            edit(rng, &mut lines, len, &mut req);
        }
        if rng.chance(70) {
            req(&mut lines, "doc.save", json!({"doc": doc}));
        }
        if rng.chance(50) {
            edit(rng, &mut lines, len, &mut req);
            req(&mut lines, "doc.undo", json!({"doc": doc}));
            *len = None;
            if rng.chance(50) {
                req(&mut lines, "doc.save", json!({"doc": doc}));
            }
        }
        if rng.chance(40) {
            edit(rng, &mut lines, len, &mut req);
        }
        match rng.below(5) {
            0 | 1 => {
                req(
                    &mut lines,
                    "doc.close",
                    json!({"doc": doc, "discard": true}),
                );
            }
            2 | 3 => {
                req(&mut lines, "doc.close", json!({"doc": doc}));
            }
            _ => {}
        }
    }
    // Phase 3: close (discarding) whatever is still open, reopen and read every file.
    let mut finals = Finals {
        info_text: Vec::new(),
        bad_open: None,
    };
    for i in 0..good.len() {
        req(
            &mut lines,
            "doc.close",
            json!({"doc": i + 1, "discard": true}),
        );
    }
    for (i, f) in good.iter().enumerate() {
        let doc = (good.len() + i + 1) as u64;
        req(&mut lines, "doc.open", json!({"path": f.path}));
        let info = req(&mut lines, "doc.info", json!({"doc": doc}));
        let text = req(&mut lines, "doc.text", json!({"doc": doc}));
        finals.info_text.push((f.path.clone(), info, text));
    }
    // Phase 4: the undecodable file, last, so that its failed open cannot shift any id.
    if let Some(bad) = files.iter().find(|f| f.encoding.is_none()) {
        let id = req(&mut lines, "doc.open", json!({"path": bad.path}));
        finals.bad_open = Some((bad.path.clone(), id));
    }
    req(&mut lines, "doc.list", json!({}));
    (lines.join("\n") + "\n", finals)
}

/// Test-side decoder for the generator's own files (BOM decides; strict).
fn decode_naive(bytes: &[u8], encoding: &str) -> String {
    match encoding {
        "utf-8" => String::from_utf8(bytes.to_vec()).unwrap(),
        "utf-8-bom" => String::from_utf8(bytes[3..].to_vec()).unwrap(),
        "utf-16le" | "utf-16be" => {
            let units: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|c| {
                    if encoding == "utf-16le" {
                        u16::from_le_bytes([c[0], c[1]])
                    } else {
                        u16::from_be_bytes([c[0], c[1]])
                    }
                })
                .collect();
            String::from_utf16(&units).unwrap()
        }
        other => panic!("unknown encoding {other}"),
    }
}

/// One generated set run both ways.
struct SetRun {
    files: Vec<GenFile>,
    finals: Finals,
    memory: Output,
    disk: Output,
    disk_after: BTreeMap<String, Vec<u8>>,
}

const SETS: usize = 24;

/// AC-1's runs: [`SETS`] fixed-seed file sets, each with a fixed-seed script, run with
/// `--memory-seed` and with `--root` on an equal directory.
fn run_generated_sets(name: &str) -> (Vec<SetRun>, Coverage) {
    let work = fresh(name);
    let mut rng = Rng(0x7011_5EED);
    let mut cov = Coverage::default();
    let mut runs = Vec::new();
    for set in 0..SETS {
        let files = gen_files(&mut rng, set, &mut cov);
        let (script, finals) = gen_script(&mut rng, &files);
        let dir = work.join(format!("set{set}"));
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let pairs: Vec<(String, Vec<u8>)> = files
            .iter()
            .map(|f| (f.path.clone(), f.bytes.clone()))
            .collect();
        write_tree(&root, &pairs);
        std::fs::write(dir.join("m.json"), manifest(&pairs, |i| (i + set) % 2 == 0)).unwrap();
        std::fs::write(dir.join("s.jsonl"), &script).unwrap();
        let memory = kiln(&["run", "s.jsonl", "--memory-seed", "m.json"], &dir);
        let disk = kiln(&["run", "s.jsonl", "--root", "root"], &dir);
        let disk_after = snapshot(&root);
        runs.push(SetRun {
            files,
            finals,
            memory,
            disk,
            disk_after,
        });
    }
    (runs, cov)
}

/// The generator does what AC-1 asks for, checked on the disk runs (which work on the base).
fn check_generator(runs: &[SetRun], cov: &Coverage) {
    assert!(runs.len() >= 20);
    for e in ENCODINGS {
        assert!(
            cov.encodings.get(e).copied().unwrap_or(0) > 0,
            "{e} covered"
        );
    }
    for e in ["lf", "crlf", "cr", "mixed"] {
        assert!(cov.eols.get(e).copied().unwrap_or(0) > 0, "{e} covered");
    }
    assert!(cov.nested > 0 && cov.undecodable > 0);
    let mut changed = 0;
    for (n, run) in runs.iter().enumerate() {
        assert!((1..=5).contains(&run.files.len()));
        let disk = parse_lines(&stdout(&run.disk));
        for (path, info, text) in &run.finals.info_text {
            let info = by_id(&disk, *info);
            assert_eq!(info["ok"], json!(true), "set {n}: generator: {info}");
            assert_eq!(
                info["result"]["path"],
                json!(path),
                "set {n}: id prediction"
            );
            assert_eq!(by_id(&disk, *text)["ok"], json!(true), "set {n}");
        }
        for f in &run.files {
            if run.disk_after.get(&f.path) != Some(&f.bytes) {
                changed += 1;
            }
        }
    }
    assert!(changed >= 10, "saves changed only {changed} files");
}

// ---------------------------------------------------------------------------------------------
// AC-1, AC-2.

/// AC-1: `run --memory-seed` and `run --root` print byte-identical stdout and exit alike.
#[test]
fn ac1_memory_seed_and_root_runs_print_identical_stdout() {
    let (runs, cov) = run_generated_sets("ac1");
    check_generator(&runs, &cov);
    for (n, run) in runs.iter().enumerate() {
        assert_eq!(
            stdout(&run.memory),
            stdout(&run.disk),
            "set {n}: memory vs disk stdout; memory stderr: {}",
            stderr(&run.memory)
        );
        assert_eq!(code(&run.memory), code(&run.disk), "set {n}: exit code");
        assert!(
            run.memory.stderr.is_empty(),
            "set {n}: {}",
            stderr(&run.memory)
        );
    }
}

/// AC-2: in the memory runs, each file's reopened text and encoding, encoded naively, equal
/// that file's bytes in the disk directory after the disk run; the undecodable file still fails
/// with `decode_error` in memory and is unchanged on disk.
#[test]
fn ac2_reopened_memory_files_equal_disk_files_after_the_run() {
    let (runs, cov) = run_generated_sets("ac2");
    check_generator(&runs, &cov);
    for (n, run) in runs.iter().enumerate() {
        let memory = parse_lines(&stdout(&run.memory));
        assert!(
            !memory.is_empty(),
            "set {n}: memory run printed nothing: {}",
            first_stderr_line(&run.memory)
        );
        for (path, info, text) in &run.finals.info_text {
            let info = by_id(&memory, *info);
            let text = by_id(&memory, *text);
            assert_eq!(info["ok"], json!(true), "set {n} {path}: {info}");
            assert_eq!(text["ok"], json!(true), "set {n} {path}: {text}");
            assert_eq!(info["result"]["path"], json!(path));
            let encoding = info["result"]["encoding"].as_str().unwrap();
            let bytes = naive_encode(text["result"]["text"].as_str().unwrap(), encoding);
            assert_eq!(
                Some(&bytes),
                run.disk_after.get(path),
                "set {n} {path}: memory file vs disk file"
            );
        }
        if let Some((path, id)) = &run.finals.bad_open {
            let r = by_id(&memory, *id);
            assert_eq!(
                (&r["ok"], &r["error"]["code"], &r["error"]["kind"]),
                (&json!(false), &json!("rejected"), &json!("decode_error")),
                "set {n} {path}"
            );
            let original = &run.files.iter().find(|f| &f.path == path).unwrap().bytes;
            assert_eq!(run.disk_after.get(path), Some(original));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// AC-3.

/// The spec's example manifest files, plus UTF-16 and nested ones.
fn example_files() -> Vec<(String, Vec<u8>)> {
    vec![
        ("src/main.rs".into(), b"fn main() {}\r\n".to_vec()),
        ("bom.txt".into(), b"\xef\xbb\xbfhi".to_vec()),
        ("bad.txt".into(), b"\xff".to_vec()),
        ("a.txt".into(), b"one\ntwo\r\nthree\r".to_vec()),
        (
            "d/e/le.txt".into(),
            naive_encode("x\u{e9}\u{1F600}\r\n", "utf-16le"),
        ),
        ("d/be.txt".into(), naive_encode("b\n", "utf-16be")),
    ]
}

/// AC-3: `exec … --memory-seed` gives the same exit code, stdout and stderr as `--root` on an
/// equal directory; for `doc.open` of a seeded decodable file that is exit 0 and the result.
#[test]
fn ac3_exec_with_memory_seed_matches_root() {
    let work = fresh("ac3");
    let files = example_files();
    write_tree(&work.join("root"), &files);
    std::fs::write(work.join("m.json"), manifest(&files, |i| i % 2 == 0)).unwrap();

    let seeded = kiln(
        &[
            "exec",
            "doc.open",
            r#"{"path":"a.txt"}"#,
            "--memory-seed",
            "m.json",
        ],
        &work,
    );
    assert_eq!(code(&seeded), 0, "{}", stderr(&seeded));
    let v: Value = serde_json::from_str(&stdout(&seeded)).unwrap();
    assert_eq!(
        (
            &v["path"],
            &v["encoding"],
            &v["len_chars"],
            &v["already_open"]
        ),
        (&json!("a.txt"), &json!("utf-8"), &json!(15), &json!(false))
    );

    let mut cases: Vec<Vec<String>> = Vec::new();
    for (path, _) in &files {
        cases.push(vec!["doc.open".into(), json!({ "path": path }).to_string()]);
    }
    for extra in [
        vec![
            "doc.open".to_owned(),
            r#"{"path":"missing.txt"}"#.to_owned(),
        ],
        vec!["doc.open".into(), r#"{"path":"d"}"#.into()],
        vec!["doc.open".into(), r#"{"path":"d/e"}"#.into()],
        vec!["doc.list".into(), "{}".into()],
        vec![
            "buffer.text".into(),
            "{}".into(),
            "--text".into(),
            "abc\r\n".into(),
        ],
    ] {
        cases.push(extra);
    }
    for case in cases {
        let mut mem: Vec<&str> = vec!["exec"];
        mem.extend(case.iter().map(String::as_str));
        let mut disk = mem.clone();
        mem.extend(["--memory-seed", "m.json"]);
        disk.extend(["--root", "root"]);
        let (m, d) = (kiln(&mem, &work), kiln(&disk, &work));
        assert_eq!(
            (code(&m), stdout(&m), stderr(&m)),
            (code(&d), stdout(&d), stderr(&d)),
            "{case:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// AC-4.

/// A `run` script that would change `a.txt` (and saves it).
const CHANGING_SCRIPT: &str = concat!(
    r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
    "\n",
    r#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":0,"text":"CHANGED"}}"#,
    "\n",
    r#"{"id":3,"command":"doc.save","params":{"doc":1}}"#,
    "\n",
);

/// Every refusal case of the Interface, as manifest bytes.
fn bad_manifests() -> Vec<(&'static str, Vec<u8>)> {
    let deep = format!(
        "{{\"files\":{}{}}}",
        "[".repeat(100_000),
        "]".repeat(100_000)
    );
    let cases: Vec<(&str, String)> = vec![
        // invalid JSON
        ("empty file", String::new()),
        ("truncated", r#"{"files":["#.into()),
        ("not json", "files: []".into()),
        ("trailing garbage", r#"{"files":[]} x"#.into()),
        ("raw control character", "{\"files\":[{\"path\":\"a\",\"text\":\"\u{1}\"}]}".into()),
        ("deeply nested", deep),
        // top level
        ("top-level array", "[]".into()),
        ("top-level string", r#""files""#.into()),
        ("top-level null", "null".into()),
        ("no files key", "{}".into()),
        ("extra top-level key", r#"{"files":[],"x":1}"#.into()),
        ("files not an array", r#"{"files":{}}"#.into()),
        ("files null", r#"{"files":null}"#.into()),
        // entries
        ("entry not an object", r#"{"files":[1]}"#.into()),
        ("entry array", r#"{"files":[["a","x"]]}"#.into()),
        ("extra entry key", r#"{"files":[{"path":"a","text":"","mode":1}]}"#.into()),
        ("no path", r#"{"files":[{"text":"x"}]}"#.into()),
        ("path not a string", r#"{"files":[{"path":1,"text":"x"}]}"#.into()),
        ("both text and hex", r#"{"files":[{"path":"a","text":"x","hex":"78"}]}"#.into()),
        ("neither text nor hex", r#"{"files":[{"path":"a"}]}"#.into()),
        ("text not a string", r#"{"files":[{"path":"a","text":1}]}"#.into()),
        ("hex not a string", r#"{"files":[{"path":"a","hex":12}]}"#.into()),
        ("odd hex", r#"{"files":[{"path":"a","hex":"abc"}]}"#.into()),
        ("non-hex digit", r#"{"files":[{"path":"a","hex":"zz"}]}"#.into()),
        ("hex with 0x", r#"{"files":[{"path":"a","hex":"0x41"}]}"#.into()),
        ("hex with space", r#"{"files":[{"path":"a","hex":"41 42"}]}"#.into()),
        ("non-ASCII hex", r#"{"files":[{"path":"a","hex":"4é"}]}"#.into()),
        // duplicates and prefix conflicts
        (
            "duplicate path",
            r#"{"files":[{"path":"a","text":"1"},{"path":"a","hex":"32"}]}"#.into(),
        ),
        (
            "file is a prefix directory",
            r#"{"files":[{"path":"d","text":""},{"path":"d/x","text":""}]}"#.into(),
        ),
        (
            "prefix directory listed after",
            r#"{"files":[{"path":"d/e/x","text":""},{"path":"d/e","text":""}]}"#.into(),
        ),
        (
            "deeper prefix",
            r#"{"files":[{"path":"d","text":""},{"path":"q","text":""},{"path":"d/e/f","text":""}]}"#
                .into(),
        ),
    ];
    let mut cases: Vec<(&str, Vec<u8>)> = cases
        .into_iter()
        .map(|(w, s)| (w, s.into_bytes()))
        .collect();
    cases.push((
        "not UTF-8",
        b"{\"files\":[{\"path\":\"a\",\"text\":\"\xff\"}]}".to_vec(),
    ));
    // invalid paths (the RelPath rules of "Paths")
    for (why, path) in [
        ("empty path", ""),
        ("absolute path", "/a"),
        ("dot-dot segment", "a/../b"),
        ("dot-dot", ".."),
        ("dot segment", "./a"),
        ("empty segment", "a//b"),
        ("trailing slash", "a/"),
        ("backslash", "a\\b"),
        ("NUL", "a\u{0}b"),
    ] {
        cases.push((
            why,
            json!({"files": [{"path": "ok.txt", "text": "x"}, {"path": path, "text": "x"}]})
                .to_string()
                .into_bytes(),
        ));
    }
    cases
}

/// Assert `out` is a `bad --memory-seed` refusal for manifest argument `arg`.
fn assert_bad_manifest(out: &Output, arg: &str, why: &str) {
    let first = first_stderr_line(out);
    assert_eq!(code(out), 2, "{why}: {first}");
    assert!(out.stdout.is_empty(), "{why}: nothing on stdout");
    let prefix = format!("kiln-cli: bad --memory-seed {arg}: ");
    assert!(
        first.starts_with(&prefix),
        "{why}: first stderr line: {first:?}"
    );
    assert!(
        first.len() > prefix.len(),
        "{why}: a reason follows the prefix"
    );
    assert!(!stderr(out).contains("usage:"), "{why}: no usage text");
}

/// AC-4: every bad manifest is refused (exit 2, empty stdout, exact first-line prefix, no usage
/// text) before any request runs: the changing script's effects and its `--log` never appear.
#[test]
fn ac4_bad_manifests_are_refused_before_any_request() {
    let work = fresh("ac4");
    std::fs::write(work.join("s.jsonl"), CHANGING_SCRIPT).unwrap();
    std::fs::create_dir(work.join("adir")).unwrap();
    // Positive control: a good manifest with the same script runs it (exit 0, three responses).
    std::fs::write(
        work.join("good.json"),
        r#"{"files":[{"path":"a.txt","text":"x"}]}"#,
    )
    .unwrap();
    let ok = kiln(
        &[
            "run",
            "s.jsonl",
            "--memory-seed",
            "good.json",
            "--log",
            "l.jsonl",
        ],
        &work,
    );
    assert_eq!(code(&ok), 0, "{}", first_stderr_line(&ok));
    assert_eq!(parse_lines(&stdout(&ok)).len(), 3);
    std::fs::remove_file(work.join("l.jsonl")).unwrap();

    // Unreadable manifest: missing, or a directory.
    for (why, arg) in [("missing file", "missing.json"), ("a directory", "adir")] {
        let before = snapshot(&work);
        for args in [
            vec!["run", "s.jsonl", "--memory-seed", arg, "--log", "l.jsonl"],
            vec!["exec", "doc.list", "{}", "--memory-seed", arg],
        ] {
            assert_bad_manifest(&kiln(&args, &work), arg, why);
            assert_eq!(snapshot(&work), before, "{why}: nothing written (no log)");
        }
    }
    for (i, (why, bytes)) in bad_manifests().into_iter().enumerate() {
        let name = format!("m{i}.json");
        std::fs::write(work.join(&name), &bytes).unwrap();
        let before = snapshot(&work);
        for args in [
            vec!["run", "s.jsonl", "--memory-seed", &name, "--log", "l.jsonl"],
            vec!["exec", "doc.list", "{}", "--memory-seed", &name],
        ] {
            assert_bad_manifest(&kiln(&args, &work), &name, why);
            assert_eq!(snapshot(&work), before, "{why}: nothing written (no log)");
        }
    }
    // Precedence: the manifest is checked before the script is read.
    std::fs::write(
        work.join("odd.json"),
        r#"{"files":[{"path":"a","hex":"abc"}]}"#,
    )
    .unwrap();
    let out = kiln(
        &["run", "no-such-script.jsonl", "--memory-seed", "odd.json"],
        &work,
    );
    assert_bad_manifest(&out, "odd.json", "bad manifest before unreadable script");
    // Precedence: option parsing (usage errors) comes before the manifest check.
    let out = kiln(
        &["run", "s.jsonl", "--memory-seed", "odd.json", "--bogus"],
        &work,
    );
    assert_eq!(code(&out), 2);
    assert!(out.stdout.is_empty() && stderr(&out).contains("usage:"));
    assert!(!first_stderr_line(&out).starts_with("kiln-cli: bad --memory-seed"));
}

/// Ruling u3 (D-067): for `exec`, params that are not JSON are reported (`invalid_json`, exit 2)
/// before the manifest is read, as with `--root`; with valid params the manifest is checked.
#[test]
fn ac4_exec_reports_invalid_params_json_before_the_manifest() {
    let work = fresh("ac4u3");
    std::fs::write(
        work.join("odd.json"),
        r#"{"files":[{"path":"a","hex":"abc"}]}"#,
    )
    .unwrap();
    for manifest in ["missing.json", "odd.json"] {
        let out = kiln(&["exec", "doc.open", "{", "--memory-seed", manifest], &work);
        assert_eq!(code(&out), 2, "{manifest}");
        assert!(out.stdout.is_empty(), "{manifest}");
        let err: Value = serde_json::from_str(first_stderr_line(&out).as_str())
            .unwrap_or_else(|e| panic!("{manifest}: not a JSON error ({e}): {}", stderr(&out)));
        assert_eq!(err["code"], json!("invalid_json"), "{manifest}");
        // Positive control: valid params, so the manifest is read and refused.
        let out = kiln(
            &[
                "exec",
                "doc.open",
                r#"{"path":"a"}"#,
                "--memory-seed",
                manifest,
            ],
            &work,
        );
        assert_bad_manifest(&out, manifest, "valid params");
    }
}

/// Pinned (d): the manifest path is resolved against the current directory (relative or
/// absolute), and a refusal names it as given on the command line.
#[test]
fn ac4_manifest_path_is_resolved_against_the_current_directory() {
    let work = fresh("ac4cwd");
    std::fs::create_dir(work.join("sub")).unwrap();
    std::fs::write(
        work.join("sub/m.json"),
        r#"{"files":[{"path":"a.txt","text":"in sub"}]}"#,
    )
    .unwrap();
    std::fs::write(
        work.join("m.json"),
        r#"{"files":[{"path":"a.txt","text":"in work"}]}"#,
    )
    .unwrap();
    let text = |cwd: &Path, arg: &str| {
        let out = kiln(
            &[
                "exec",
                "doc.open",
                r#"{"path":"a.txt"}"#,
                "--memory-seed",
                arg,
            ],
            cwd,
        );
        assert_eq!(code(&out), 0, "{}", first_stderr_line(&out));
        serde_json::from_str::<Value>(&stdout(&out)).unwrap()["len_chars"].clone()
    };
    assert_eq!(text(&work, "m.json"), json!(7));
    assert_eq!(text(&work, "sub/m.json"), json!(6));
    assert_eq!(text(&work.join("sub"), "m.json"), json!(6));
    let abs = work.join("m.json");
    assert_eq!(text(&work.join("sub"), abs.to_str().unwrap()), json!(7));
    // Missing relative to the cwd although it exists elsewhere.
    let out = kiln(
        &["exec", "doc.list", "{}", "--memory-seed", "sub/m.json"],
        &work.join("sub"),
    );
    assert_bad_manifest(&out, "sub/m.json", "relative to cwd");
}

/// AC-4: `--memory-seed` with `--root` or `--memory` (either order), or twice, is a usage
/// error (exit 2, usage text) for `run` and `exec`, even when the manifest is bad.
#[test]
fn ac4_memory_seed_conflicts_are_usage_errors() {
    let work = fresh("ac4usage");
    std::fs::write(work.join("s.jsonl"), "{\"command\":\"doc.list\"}\n").unwrap();
    std::fs::write(work.join("m.json"), r#"{"files":[]}"#).unwrap();
    std::fs::write(work.join("bad.json"), "{").unwrap();
    std::fs::create_dir(work.join("root")).unwrap();
    // Positive controls: each option alone is accepted.
    for opt in [
        vec!["--memory-seed", "m.json"],
        vec!["--root", "root"],
        vec!["--memory"],
    ] {
        let mut args = vec!["run", "s.jsonl"];
        args.extend(&opt);
        let out = kiln(&args, &work);
        assert_eq!(code(&out), 0, "{opt:?}: {}", first_stderr_line(&out));
        let mut args = vec!["exec", "doc.list", "{}"];
        args.extend(&opt);
        assert_eq!(code(&kiln(&args, &work)), 0, "{opt:?}");
    }
    for manifest in ["m.json", "bad.json", "missing.json"] {
        let cases: Vec<(Vec<&str>, Option<&str>)> = vec![
            (
                vec!["--memory-seed", manifest, "--root", "root"],
                Some("kiln-cli: options `--root` and `--memory-seed` cannot be combined"),
            ),
            (
                vec!["--root", "root", "--memory-seed", manifest],
                Some("kiln-cli: options `--root` and `--memory-seed` cannot be combined"),
            ),
            (vec!["--memory-seed", manifest, "--memory"], None),
            (vec!["--memory", "--memory-seed", manifest], None),
            (
                vec!["--memory-seed", manifest, "--memory-seed", "m.json"],
                None,
            ),
        ];
        for (opts, exact) in cases {
            for sub in [vec!["run", "s.jsonl"], vec!["exec", "doc.list", "{}"]] {
                let mut args = sub.clone();
                args.extend(&opts);
                let out = kiln(&args, &work);
                let first = first_stderr_line(&out);
                assert_eq!(code(&out), 2, "{args:?}");
                assert!(out.stdout.is_empty(), "{args:?}");
                assert!(stderr(&out).contains("usage:"), "{args:?}: usage text");
                assert!(first.contains("`--memory-seed`"), "{args:?}: {first}");
                assert!(!first.contains("unknown option"), "{args:?}: {first}");
                match exact {
                    Some(line) => assert_eq!(first, line, "{args:?}"),
                    None if opts.contains(&"--memory") => {
                        assert!(
                            first.contains("`--memory`") && first.contains("cannot be combined"),
                            "{args:?}: {first}"
                        );
                    }
                    None => {}
                }
            }
        }
    }
}

/// Valid edge cases of the manifest are accepted and give exactly the files: empty `files`,
/// empty `text`/`hex`, upper-case hex, siblings in one directory, and a file `d` next to a
/// file `dx/y`: `d` prefixes `dx` as a string but is not a directory prefix at a `/` boundary,
/// so it is no conflict (ruling u1, D-067); both are seeded and read back.
#[test]
fn ac4_valid_manifest_edge_cases_are_accepted() {
    let work = fresh("ac4ok");
    let script = concat!(
        r#"{"id":1,"command":"doc.open","params":{"path":"c/x"}}"#,
        "\n",
        r#"{"id":2,"command":"doc.text","params":{"doc":1}}"#,
        "\n",
        r#"{"id":3,"command":"doc.open","params":{"path":"c/y"}}"#,
        "\n",
        r#"{"id":4,"command":"doc.text","params":{"doc":2}}"#,
        "\n",
        r#"{"id":5,"command":"doc.open","params":{"path":"dx/y"}}"#,
        "\n",
        r#"{"id":6,"command":"doc.text","params":{"doc":3}}"#,
        "\n",
        r#"{"id":7,"command":"doc.open","params":{"path":"d"}}"#,
        "\n",
        r#"{"id":8,"command":"doc.text","params":{"doc":4}}"#,
        "\n",
        r#"{"id":9,"command":"doc.open","params":{"path":"dx"}}"#,
        "\n",
        r#"{"id":10,"command":"doc.open","params":{"path":"other"}}"#,
        "\n",
    );
    std::fs::write(work.join("s.jsonl"), script).unwrap();
    std::fs::write(
        work.join("m.json"),
        r#"{"files":[{"path":"c/x","hex":"C3a961"},{"path":"c/y","text":""},{"path":"dx/y","hex":""},{"path":"d","text":"é\r\n"}]}"#,
    )
    .unwrap();
    let out = kiln(&["run", "s.jsonl", "--memory-seed", "m.json"], &work);
    assert_eq!(
        code(&out),
        3,
        "the last two opens fail: {}",
        first_stderr_line(&out)
    );
    let r = parse_lines(&stdout(&out));
    assert_eq!(r.len(), 10);
    for (id, text) in [(2, "éa"), (4, ""), (6, ""), (8, "é\r\n")] {
        assert_eq!(by_id(&r, id)["result"]["text"], json!(text), "id {id}");
    }
    assert_eq!(by_id(&r, 9)["error"]["kind"], json!("not_a_file"));
    assert_eq!(by_id(&r, 10)["error"]["kind"], json!("not_found"));

    // An empty `files` array: the same as `--memory`.
    std::fs::write(work.join("empty.json"), r#"{ "files" : [ ] }"#).unwrap();
    let seeded = kiln(&["run", "s.jsonl", "--memory-seed", "empty.json"], &work);
    let empty = kiln(&["run", "s.jsonl", "--memory"], &work);
    assert_eq!(code(&seeded), 3, "{}", first_stderr_line(&seeded));
    assert_eq!(
        (code(&seeded), stdout(&seeded)),
        (code(&empty), stdout(&empty))
    );
}

// ---------------------------------------------------------------------------------------------
// Pinned (b), (d); Documentation.

/// Pinned (b): a seeded session writes nothing to disk; saves live in memory only (a reopen in
/// the same session shows them; the next session starts again from the manifest).
#[test]
fn memory_seed_writes_nothing_to_disk() {
    let work = fresh("nowrite");
    let manifest =
        r#"{"files":[{"path":"a.txt","text":"a\r\n"},{"path":"n/b.txt","hex":"fffe6200"}]}"#;
    std::fs::write(work.join("m.json"), manifest).unwrap();
    let script = concat!(
        r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
        "\n",
        r#"{"id":2,"command":"doc.insert","params":{"doc":1,"offset":0,"text":"new "}}"#,
        "\n",
        r#"{"id":3,"command":"doc.save","params":{"doc":1}}"#,
        "\n",
        r#"{"id":4,"command":"doc.open","params":{"path":"n/b.txt"}}"#,
        "\n",
        r#"{"id":5,"command":"doc.insert","params":{"doc":2,"offset":1,"text":"é"}}"#,
        "\n",
        r#"{"id":6,"command":"doc.save","params":{"doc":2}}"#,
        "\n",
        r#"{"id":7,"command":"doc.close","params":{"doc":1}}"#,
        "\n",
        r#"{"id":8,"command":"doc.close","params":{"doc":2}}"#,
        "\n",
        r#"{"id":9,"command":"doc.open","params":{"path":"a.txt"}}"#,
        "\n",
        r#"{"id":10,"command":"doc.text","params":{"doc":3}}"#,
        "\n",
        r#"{"id":11,"command":"doc.open","params":{"path":"n/b.txt"}}"#,
        "\n",
        r#"{"id":12,"command":"doc.info","params":{"doc":4}}"#,
        "\n",
        r#"{"id":13,"command":"doc.text","params":{"doc":4}}"#,
        "\n",
    );
    std::fs::write(work.join("s.jsonl"), script).unwrap();
    let before = snapshot(&work);
    let out = kiln(&["run", "s.jsonl", "--memory-seed", "m.json"], &work);
    assert_eq!(code(&out), 0, "{}", first_stderr_line(&out));
    let r = parse_lines(&stdout(&out));
    assert_eq!(by_id(&r, 10)["result"]["text"], json!("new a\r\n"));
    assert_eq!(by_id(&r, 12)["result"]["encoding"], json!("utf-16le"));
    assert_eq!(by_id(&r, 13)["result"]["text"], json!("bé"));
    assert_eq!(
        snapshot(&work),
        before,
        "nothing written under the current directory"
    );
    assert!(!work.join("a.txt").exists() && !work.join("n").exists());
    // A second session starts from the manifest again: a read-only script sees the
    // manifest's contents, not the first session's saves.
    let reader = concat!(
        r#"{"id":1,"command":"doc.open","params":{"path":"a.txt"}}"#,
        "\n",
        r#"{"id":2,"command":"doc.text","params":{"doc":1}}"#,
        "\n",
        r#"{"id":3,"command":"doc.open","params":{"path":"n/b.txt"}}"#,
        "\n",
        r#"{"id":4,"command":"doc.text","params":{"doc":2}}"#,
        "\n",
    );
    std::fs::write(work.join("read.jsonl"), reader).unwrap();
    let out = kiln(&["run", "read.jsonl", "--memory-seed", "m.json"], &work);
    assert_eq!(code(&out), 0, "{}", first_stderr_line(&out));
    let r = parse_lines(&stdout(&out));
    assert_eq!(by_id(&r, 2)["result"]["text"], json!("a\r\n"));
    assert_eq!(by_id(&r, 4)["result"]["text"], json!("b"));
}

/// Documentation: `kiln-cli --help` (and `run --help`, `exec --help`) lists `--memory-seed`.
#[test]
fn help_lists_memory_seed() {
    let work = fresh("help");
    for args in [vec!["--help"], vec!["run", "--help"], vec!["exec", "-h"]] {
        let out = kiln(&args, &work);
        assert_eq!(code(&out), 0, "{args:?}");
        let text = stdout(&out);
        assert!(text.contains("usage"), "{args:?}");
        assert!(text.contains("--memory-seed"), "{args:?}: {text}");
    }
}
