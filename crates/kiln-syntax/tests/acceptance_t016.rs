//! Independent acceptance tests for T-016 (tree-sitter parse contract in `kiln-syntax`).
//!
//! Written from the task file's Interface, Pinned section and acceptance criteria only. Oracles:
//! the pinned corpus under `tests/corpus/` (FNV-1a 64 hashes in `MANIFEST.toml`, the recorded
//! rustc run in `corpus-rustc-run.toml`, cross-checked here with `syn`), a from-scratch parse
//! (AC-4), the live sources of this repository (AC-2), hand-written exact expectations (AC-5)
//! and properties (AC-3, AC-6). Every generator is fixed-seed (SplitMix64); no test spawns a
//! process.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use kiln_syntax::{ErrorKind, ErrorSpan, ParseError, ParseMode, SyntaxTree};
use syn::visit::{self, Visit};

// ---------------------------------------------------------------------------------------------
// Corpus access
// ---------------------------------------------------------------------------------------------

const PINNED_COMMAND: &str = "rustc +1.97.0 --edition 2024 --crate-type lib --emit metadata \
--crate-name corpus --cap-lints allow --out-dir <scratch>/out <file>";

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("corpus")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// FNV-1a 64 over raw bytes, as pinned (offset basis 0xcbf29ce484222325, prime 0x100000001b3).
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Every regular file under `dir`, as `/`-separated paths relative to `base`, sorted.
fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        let ty = entry.file_type().expect("file type");
        if ty.is_dir() {
            walk(base, &path, out);
        } else {
            let rel = path.strip_prefix(base).expect("path under base");
            let parts: Vec<String> =
                rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            out.push(parts.join("/"));
        }
    }
    out.sort();
}

#[derive(Debug, Clone)]
struct Entry {
    path: String,
    set: String,
    bytes: i64,
    lines: i64,
    fnv1a64: String,
    origin: String,
    license: Option<String>,
    gap: Option<String>,
}

fn read_toml(name: &str) -> toml::Table {
    let p = corpus_dir().join(name);
    let s = fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
    toml::from_str::<toml::Table>(&s).unwrap_or_else(|e| panic!("{name} is not valid TOML: {e}"))
}

fn str_field(t: &toml::Table, key: &str, ctx: &str) -> String {
    t.get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{ctx}: missing string field `{key}`"))
        .to_owned()
}

fn int_field(t: &toml::Table, key: &str, ctx: &str) -> i64 {
    t.get(key)
        .and_then(toml::Value::as_integer)
        .unwrap_or_else(|| panic!("{ctx}: missing integer field `{key}`"))
}

fn file_tables<'a>(t: &'a toml::Table, ctx: &str) -> Vec<&'a toml::Table> {
    t.get("file")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{ctx}: no [[file]] array"))
        .iter()
        .map(|v| v.as_table().unwrap_or_else(|| panic!("{ctx}: [[file]] entry is not a table")))
        .collect()
}

fn manifest() -> Vec<Entry> {
    let t = read_toml("MANIFEST.toml");
    file_tables(&t, "MANIFEST.toml")
        .into_iter()
        .map(|f| {
            let path = str_field(f, "path", "MANIFEST.toml entry");
            let ctx = format!("MANIFEST.toml entry {path}");
            Entry {
                set: str_field(f, "set", &ctx),
                bytes: int_field(f, "bytes", &ctx),
                lines: int_field(f, "lines", &ctx),
                fnv1a64: str_field(f, "fnv1a64", &ctx),
                origin: str_field(f, "origin", &ctx),
                license: f.get("license").and_then(toml::Value::as_str).map(str::to_owned),
                gap: f.get("gap").and_then(toml::Value::as_str).map(str::to_owned),
                path,
            }
        })
        .collect()
}

/// The `(path, text)` pairs of one manifest set; panics (with a message) if the set is empty.
fn corpus_set(set: &str) -> Vec<(String, String)> {
    let files: Vec<(String, String)> = manifest()
        .into_iter()
        .filter(|e| e.set == set)
        .map(|e| {
            let p = corpus_dir().join(&e.path);
            let text = fs::read_to_string(&p)
                .unwrap_or_else(|err| panic!("corpus file {} is not readable UTF-8: {err}", e.path));
            (e.path, text)
        })
        .collect();
    assert!(!files.is_empty(), "the corpus has no `{set}` files; the manifest or the corpus is missing");
    files
}

// ---------------------------------------------------------------------------------------------
// Tree helpers
// ---------------------------------------------------------------------------------------------

fn parse(text: &str) -> SyntaxTree {
    match SyntaxTree::parse(text) {
        Ok(t) => t,
        Err(e) => panic!("parse returned Err({e:?}) for a {}-char text", text.chars().count()),
    }
}

fn len_chars(s: &str) -> usize {
    s.chars().count()
}

/// Spans are ordered and inside `0..=len`; `has_error` agrees with `errors`.
fn check_spans(what: &str, tree: &SyntaxTree, len: usize) {
    let errs = tree.errors();
    for e in &errs {
        assert!(e.start <= e.end && e.end <= len, "{what}: span {e:?} outside 0..={len}");
    }
    assert_eq!(
        tree.has_error(),
        !errs.is_empty(),
        "{what}: has_error() disagrees with errors() ({} spans)",
        errs.len()
    );
}

/// Every `@S..E` in the sexp has S <= E <= len.
fn check_sexp_offsets(what: &str, sexp: &str, len: usize) {
    let b = sexp.as_bytes();
    let mut i = 0;
    let mut seen = 0usize;
    while i < b.len() {
        if b[i] == b'@' {
            let num = |j: &mut usize| -> Option<usize> {
                let s = *j;
                while *j < b.len() && b[*j].is_ascii_digit() {
                    *j += 1;
                }
                sexp[s..*j].parse().ok()
            };
            let mut j = i + 1;
            if let Some(start) = num(&mut j)
                && sexp[j..].starts_with("..")
            {
                j += 2;
                let end = num(&mut j).unwrap_or_else(|| panic!("{what}: malformed offsets at byte {i}"));
                assert!(start <= end && end <= len, "{what}: sexp offsets {start}..{end} outside 0..={len}");
                seen += 1;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    assert!(seen >= 1, "{what}: sexp has no `@S..E` offsets: {:?}", &sexp[..sexp.len().min(80)]);
}

/// `assert_eq!` for long strings: reports the first differing char with some context.
fn same_text(what: &str, field: &str, got: &str, want: &str) {
    if got != want {
        let at = got.chars().zip(want.chars()).take_while(|(a, b)| a == b).count();
        let ctx = |s: &str| s.chars().skip(at.saturating_sub(60)).take(160).collect::<String>();
        panic!(
            "{what}: {field} after update differs from a fresh parse at char {at}\n  update: …{}…\n  fresh:  …{}…",
            ctx(got),
            ctx(want)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// AC-1: compiles => no error nodes, pinned corpus
// ---------------------------------------------------------------------------------------------

#[test]
fn corpus_is_pinned_and_compiles() {
    let dir = corpus_dir();
    assert!(dir.is_dir(), "corpus directory {} is missing", dir.display());
    let mut on_disk = Vec::new();
    walk(&dir, &dir, &mut on_disk);
    let on_disk: Vec<String> =
        on_disk.into_iter().filter(|p| p != "MANIFEST.toml" && p != "corpus-rustc-run.toml").collect();
    let entries = manifest();

    // (i) one entry per file, one file per entry, bytes/lines/hash match, no `.rs` file.
    let mut by_path: BTreeMap<String, Entry> = BTreeMap::new();
    for e in &entries {
        assert!(by_path.insert(e.path.clone(), e.clone()).is_none(), "duplicate manifest entry {}", e.path);
    }
    let disk_set: BTreeSet<&String> = on_disk.iter().collect();
    let man_set: BTreeSet<&String> = by_path.keys().collect();
    let unlisted: Vec<_> = disk_set.difference(&man_set).collect();
    let missing: Vec<_> = man_set.difference(&disk_set).collect();
    assert!(unlisted.is_empty(), "files under corpus/ without a manifest entry: {unlisted:?}");
    assert!(missing.is_empty(), "manifest entries without a file: {missing:?}");
    for p in &on_disk {
        assert!(!p.ends_with(".rs"), "corpus file {p} has the extension .rs");
    }
    for e in &entries {
        let data = fs::read(dir.join(&e.path)).unwrap_or_else(|err| panic!("read {}: {err}", e.path));
        assert_eq!(e.bytes, data.len() as i64, "{}: `bytes` does not match the file", e.path);
        let nl = data.iter().filter(|&&b| b == b'\n').count() as i64;
        assert_eq!(e.lines, nl, "{}: `lines` does not match the number of \\n bytes", e.path);
        assert!(
            e.fnv1a64.len() == 16 && e.fnv1a64.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "{}: fnv1a64 {:?} is not 16 lowercase hex digits",
            e.path,
            e.fnv1a64
        );
        assert_eq!(e.fnv1a64, format!("{:016x}", fnv1a64(&data)), "{}: bytes do not match the pinned hash", e.path);
        assert!(data.ends_with(b"\n"), "{}: does not end with \\n", e.path);
        assert!(e.path.ends_with(".rs.txt"), "{}: corpus sources are stored as <name>.rs.txt", e.path);
        let want_set = match e.path.split('/').next() {
            Some("rust") => "valid",
            Some("rust-gaps") => "gap",
            Some("rust-invalid") => "invalid",
            other => panic!("{}: not under rust/, rust-gaps/ or rust-invalid/ ({other:?})", e.path),
        };
        assert_eq!(e.set, want_set, "{}: `set` does not match its directory", e.path);
        if e.set == "valid" {
            assert!(std::str::from_utf8(&data).is_ok(), "{}: valid file is not UTF-8", e.path);
            assert!(!data.starts_with(&[0xEF, 0xBB, 0xBF]), "{}: valid file has a BOM", e.path);
        }
        if e.set == "gap" {
            assert!(e.gap.as_deref().is_some_and(|g| !g.trim().is_empty()), "{}: gap file without `gap`", e.path);
        }
        if e.origin != "written for kiln" {
            assert!(e.origin.starts_with("adapted from ") && e.origin.contains(" at "), "{}: bad origin {:?}", e.path, e.origin);
            let lic = e.license.as_deref().unwrap_or_else(|| panic!("{}: adapted file without `license`", e.path));
            assert!(
                ["MIT", "Apache-2.0", "MIT OR Apache-2.0", "Apache-2.0 OR MIT"].contains(&lic),
                "{}: licence {lic:?} is not compatible with MIT OR Apache-2.0",
                e.path
            );
        }
    }

    // (ii) counts and the line floor.
    let count = |s: &str| entries.iter().filter(|e| e.set == s).count();
    let valid_lines: i64 = entries.iter().filter(|e| e.set == "valid").map(|e| e.lines).sum();
    assert!(count("valid") >= 30, "only {} valid files; at least 30 are pinned", count("valid"));
    assert!(valid_lines >= 4_000, "valid files hold {valid_lines} lines; at least 4,000 are pinned");
    assert!(count("invalid") >= 8, "only {} invalid files; at least 8 are pinned", count("invalid"));
    assert!(count("gap") >= 1, "no gap file; the corpus records the known `safe fn` gap");

    // (iii) the recorded rustc run matches the manifest.
    let run = read_toml("corpus-rustc-run.toml");
    assert_eq!(str_field(&run, "command", "run record"), PINNED_COMMAND, "run record: `command` is not the pinned string");
    let tc = str_field(&run, "toolchain", "run record");
    assert!(tc.starts_with("rustc 1.97.0 "), "run record: toolchain {tc:?} is not rustc 1.97.0");
    let date = str_field(&run, "date", "run record");
    let db = date.as_bytes();
    assert!(
        db.len() == 10
            && db[4] == b'-'
            && db[7] == b'-'
            && db.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()),
        "run record: date {date:?} is not YYYY-MM-DD"
    );
    let mut recorded: BTreeMap<String, (String, i64)> = BTreeMap::new();
    for f in file_tables(&run, "run record") {
        let path = str_field(f, "path", "run record entry");
        let ctx = format!("run record entry {path}");
        let rec = (str_field(f, "fnv1a64", &ctx), int_field(f, "exit", &ctx));
        assert!(recorded.insert(path.clone(), rec).is_none(), "run record lists {path} twice");
    }
    let rec_set: BTreeSet<&String> = recorded.keys().collect();
    assert_eq!(rec_set, man_set, "the run record does not list exactly the manifest's paths");
    for e in &entries {
        let (h, exit) = &recorded[&e.path];
        assert_eq!(h, &e.fnv1a64, "{}: the run record compiled other bytes than the manifest pins", e.path);
        match e.set.as_str() {
            "valid" | "gap" => assert_eq!(*exit, 0, "{}: rustc did not accept a {} file", e.path, e.set),
            _ => assert_ne!(*exit, 0, "{}: rustc accepted an invalid file", e.path),
        }
    }

    // (iv) the spawn-free cross-check with syn.
    for e in &entries {
        let text = fs::read_to_string(dir.join(&e.path)).unwrap_or_else(|err| panic!("{}: {err}", e.path));
        let r = syn::parse_file(&text);
        match e.set.as_str() {
            "valid" | "gap" => assert!(r.is_ok(), "{}: syn::parse_file failed: {:?}", e.path, r.err().map(|x| x.to_string())),
            _ => assert!(r.is_err(), "{}: syn::parse_file accepted an invalid file", e.path),
        }
    }
}

#[test]
fn compiling_corpus_parses_without_errors() {
    let mut bad = Vec::new();
    for (path, text) in corpus_set("valid") {
        let t = parse(&text);
        let errs = t.errors();
        if t.has_error() || !errs.is_empty() {
            bad.push(format!("{path}: has_error={} errors={:?}", t.has_error(), &errs[..errs.len().min(3)]));
        }
        check_spans(&path, &t, len_chars(&text));
    }
    assert!(bad.is_empty(), "valid (compiling) corpus files with error nodes:\n{}", bad.join("\n"));

    for (path, text) in corpus_set("gap") {
        let t = parse(&text);
        assert!(t.has_error(), "{path}: a recorded grammar gap parses without errors; move it to rust/");
        check_spans(&path, &t, len_chars(&text));
    }
}

// ---------------------------------------------------------------------------------------------
// AC-2: compiles => no error nodes, live corpus
// ---------------------------------------------------------------------------------------------

#[test]
fn workspace_sources_parse_without_errors() {
    let root = repo_root();
    let mut roots: Vec<PathBuf> = Vec::new();
    for group in ["crates", "apps"] {
        let dir = root.join(group);
        let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
        for entry in entries {
            let src = entry.expect("entry").path().join("src");
            if src.is_dir() {
                roots.push(src);
            }
        }
    }
    roots.push(root.join("xtask").join("src"));
    roots.sort();
    assert!(roots.len() >= 8, "found only {} source roots under crates/, apps/ and xtask/", roots.len());

    let mut checked = 0usize;
    let mut bad = Vec::new();
    for src in &roots {
        let mut files = Vec::new();
        walk(src, src, &mut files);
        for rel in files.iter().filter(|f| f.ends_with(".rs")) {
            let p = src.join(rel);
            let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            let t = parse(&text);
            let errs = t.errors();
            if t.has_error() || !errs.is_empty() {
                bad.push(format!("{}: {:?}", p.display(), &errs[..errs.len().min(3)]));
            }
            checked += 1;
        }
    }
    assert!(checked >= 40, "only {checked} .rs files found under the source roots; the walk is broken");
    assert!(
        bad.is_empty(),
        "compiling workspace sources hit a grammar gap (stop with BLOCKED:, see AC-2):\n{}",
        bad.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// AC-3: errors are found
// ---------------------------------------------------------------------------------------------

#[test]
fn invalid_corpus_reports_errors() {
    let files = corpus_set("invalid");
    assert!(files.len() >= 8, "only {} invalid files", files.len());
    for (path, text) in files {
        let t = parse(&text);
        let errs = t.errors();
        assert!(!errs.is_empty(), "{path}: a file with a syntax error gives no ErrorSpan");
        assert!(t.has_error(), "{path}: errors() is non-empty but has_error() is false");
        let len = len_chars(&text);
        for e in &errs {
            assert!(e.start <= e.end && e.end <= len, "{path}: span {e:?} outside 0..={len}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// AC-4: incremental == from scratch
// ---------------------------------------------------------------------------------------------

/// SplitMix64: tiny, fixed-seed, no dependency.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
}

const SPECIALS: &[char] = &['{', '}', '(', ')', ';', '"', '\'', '/', '\r', '\n', 'é', '😀'];
const EDITS_PER_FILE: usize = 200;
const CHAINED_EDITS_PER_FILE: usize = 50;

/// Draw 0..=20 chars: a slice of the file's own text, or single chars from the file and the
/// special set.
fn draw(rng: &mut Rng, own: &[char]) -> Vec<char> {
    let n = rng.below(21);
    if rng.below(2) == 0 && own.len() > n {
        let at = rng.below(own.len() - n + 1);
        own[at..at + n].to_vec()
    } else {
        (0..n)
            .map(|_| if rng.below(2) == 0 { own[rng.below(own.len())] } else { SPECIALS[rng.below(SPECIALS.len())] })
            .collect()
    }
}

#[test]
fn incremental_update_equals_fresh_parse() {
    let mut files = corpus_set("valid");
    files.extend(corpus_set("invalid"));
    let (mut total, mut unchanged, mut incremental, mut full, mut erroneous) = (0usize, 0usize, 0usize, 0usize, 0usize);
    for (path, original) in &files {
        let own: Vec<char> = original.chars().collect();
        let mut rng = Rng(fnv1a64(path.as_bytes()) ^ 0x7016_0000_0000_0001);
        let base = parse(original);
        let mut tree = base.clone();
        let mut cur: Vec<char> = own.clone();
        for k in 0..EDITS_PER_FILE + CHAINED_EDITS_PER_FILE {
            // The first EDITS_PER_FILE edits each start from the pinned file (one edit on a
            // compiling or pinned-invalid text); the rest form one chain of edits on edits.
            if k <= EDITS_PER_FILE {
                tree = base.clone();
                cur.clone_from(&own);
            }
            let before: String = cur.iter().collect();
            assert_eq!(tree.text(), before, "{path} edit {k}: tree text is not the text before the edit");
            let pos = rng.below(cur.len() + 1);
            let op = rng.below(3);
            let (del, ins) = match op {
                0 => (0, draw(&mut rng, &own)),
                1 => (rng.below(21).min(cur.len() - pos), Vec::new()),
                _ => (rng.below(21).min(cur.len() - pos), draw(&mut rng, &own)),
            };
            cur.splice(pos..pos + del, ins.iter().copied());
            let new: String = cur.iter().collect();
            let what = format!("{path} edit {k} (op {op} at char {pos}, delete {del}, insert {:?})", ins.iter().collect::<String>());

            let mode = match tree.update(&new) {
                Ok(m) => m,
                Err(e) => panic!("{what}: update returned Err({e:?})"),
            };
            let fresh = parse(&new);
            total += 1;
            assert_eq!(tree.text(), new, "{what}: text() is not the new text");
            same_text(&what, "sexp", &tree.sexp(), &fresh.sexp());
            assert_eq!(tree.errors(), fresh.errors(), "{what}: errors after update differ from a fresh parse");
            assert_eq!(tree.node_count(), fresh.node_count(), "{what}: node_count after update differs");
            assert_eq!(tree.has_error(), fresh.has_error(), "{what}: has_error after update differs");
            if new == before {
                unchanged += 1;
                assert_eq!(mode, ParseMode::Unchanged, "{what}: text unchanged, mode must be Unchanged");
            } else if !fresh.has_error() {
                incremental += 1;
                assert_eq!(mode, ParseMode::Incremental, "{what}: changed text, fresh tree without errors");
            } else {
                erroneous += 1;
                assert!(
                    matches!(mode, ParseMode::Incremental | ParseMode::Full),
                    "{what}: changed text, mode {mode:?} is neither Incremental nor Full"
                );
                if mode == ParseMode::Full {
                    full += 1;
                }
            }
        }
    }
    eprintln!("AC-4: {total} edits: {unchanged} unchanged, {incremental} error-free, {erroneous} with errors ({full} Full)");
    assert_eq!(total, files.len() * (EDITS_PER_FILE + CHAINED_EDITS_PER_FILE), "edit count");
    assert!(unchanged >= 1, "no edit left the text unchanged; the Unchanged rule was not exercised");
    assert!(incremental >= 100, "only {incremental} edits gave an error-free fresh tree");
    assert!(erroneous >= 100, "only {erroneous} edits gave a fresh tree with errors (full: {full})");
}

// ---------------------------------------------------------------------------------------------
// AC-5: format
// ---------------------------------------------------------------------------------------------

#[test]
fn sexp_form_is_exact() {
    // The worked example (Interface), exact.
    let t = parse("fn f() {}");
    assert_eq!(
        t.sexp(),
        "(source_file@0..9 (function_item@0..9 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..9)))"
    );
    // All nodes, named and anonymous, root included: source_file function_item `fn` identifier
    // parameters `(` `)` block `{` `}`.
    assert_eq!(t.node_count(), 10, "node_count of `fn f() {{}}`");
    assert!(!t.has_error() && t.errors().is_empty(), "`fn f() {{}}` has no errors");

    // The three MISSING forms of AC-5, as substrings, plus the full sexp and errors().
    let cases: [(&str, &str, &str, Vec<ErrorSpan>); 3] = [
        (
            "fn f() { let x = 1 }",
            r#"value:(integer_literal@17..18) (MISSING ";"@18..18)"#,
            r#"(source_file@0..20 (function_item@0..20 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..20 (let_declaration@9..18 pattern:(identifier@13..14) value:(integer_literal@17..18) (MISSING ";"@18..18)))))"#,
            vec![ErrorSpan { kind: ErrorKind::Missing, start: 18, end: 18 }],
        ),
        (
            "fn f() { 1 + }",
            "right:(MISSING identifier@12..12)",
            "(source_file@0..14 (function_item@0..14 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..14 (binary_expression@9..12 left:(integer_literal@9..10) right:(MISSING identifier@12..12)))))",
            vec![ErrorSpan { kind: ErrorKind::Missing, start: 12, end: 12 }],
        ),
        (
            r##"fn f() { let s = r#"abc"; }"##,
            r#"value:(string_literal@23..24 (MISSING "\""@24..24))"#,
            r#"(source_file@0..27 (function_item@0..27 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..27 (let_declaration@9..25 pattern:(identifier@13..14) (ERROR@17..23 (identifier@20..23)) value:(string_literal@23..24 (MISSING "\""@24..24))))))"#,
            vec![
                ErrorSpan { kind: ErrorKind::Error, start: 17, end: 23 },
                ErrorSpan { kind: ErrorKind::Missing, start: 24, end: 24 },
            ],
        ),
    ];
    for (src, part, whole, errs) in cases {
        let t = parse(src);
        let s = t.sexp();
        assert!(s.contains(part), "sexp of {src:?} lacks {part:?}; got {s:?}");
        assert_eq!(s, whole, "sexp of {src:?}");
        assert_eq!(t.errors(), errs, "errors() of {src:?} (preorder, char offsets)");
        assert!(t.has_error(), "has_error() of {src:?}");
    }

    // An ERROR node prints as `ERROR`; Pinned (g): `fn a() { (` has an error.
    let t = parse("fn a() { (");
    assert_eq!(t.sexp(), "(source_file@0..10 (ERROR@0..10 (identifier@3..4) (parameters@4..6)))");
    assert_eq!(t.errors(), vec![ErrorSpan { kind: ErrorKind::Error, start: 0, end: 10 }]);

    // Offsets are chars, not bytes (Pinned (d)): `é` is 2 bytes, `😀` is 4.
    let t = parse("fn f() { \"é😀\"; }");
    assert_eq!(
        t.sexp(),
        "(source_file@0..16 (function_item@0..16 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..16 (expression_statement@9..14 (string_literal@9..13 (string_content@10..12))))))"
    );
    let t = parse("fn é() { \"😀\" + }");
    assert_eq!(
        t.sexp(),
        "(source_file@0..16 (function_item@0..16 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..16 (binary_expression@9..14 left:(string_literal@9..12 (string_content@10..11)) right:(MISSING identifier@14..14)))))"
    );
    assert_eq!(t.errors(), vec![ErrorSpan { kind: ErrorKind::Missing, start: 14, end: 14 }]);

    // CRLF counts as two chars.
    let t = parse("fn f() {}\r\nfn g() {}\r\n");
    assert_eq!(
        t.sexp(),
        "(source_file@0..22 (function_item@0..9 name:(identifier@3..4) parameters:(parameters@4..6) body:(block@7..9)) (function_item@11..20 name:(identifier@14..15) parameters:(parameters@15..17) body:(block@18..20)))"
    );

    // Pinned (g): has_error values of the worked examples.
    for ok in [
        "fn f(x: Option<i32>) { if let Some(a) = x && a > 1 { } }",
        "fn g() -> impl Sized + use<> { 0 }",
        "fn h() { let Some(x) = Some(1) else { return }; }",
    ] {
        let t = parse(ok);
        assert!(!t.has_error() && t.errors().is_empty(), "{ok:?} must parse without errors: {:?}", t.errors());
    }
    let t = parse("unsafe extern \"C\" { safe fn x(); }");
    assert!(t.has_error(), "the spike's grammar gap `safe fn` parses with an error");

    // The empty text.
    let t = parse("");
    assert_eq!(t.sexp(), "(source_file@0..0)");
    assert_eq!(t.node_count(), 1);
    assert!(t.errors().is_empty() && !t.has_error());
}

// ---------------------------------------------------------------------------------------------
// AC-6: robustness
// ---------------------------------------------------------------------------------------------

#[test]
fn hostile_inputs_parse() {
    let mut rng = Rng(0x7016_7016_7016_7016);
    let random: Vec<u8> = (0..1_000_000).map(|_| rng.next_u64() as u8).collect();
    let nested = format!("fn f() {{ {}1{} }}", "(".repeat(10_000), ")".repeat(10_000));
    let inputs: Vec<(&str, String)> = vec![
        ("empty", String::new()),
        ("NUL", String::from("\0")),
        ("lone CR", String::from("\r")),
        ("1 MB of (", "(".repeat(1_000_000)),
        ("1 MB random bytes, lossy", String::from_utf8_lossy(&random).into_owned()),
        ("10,000 nested balanced parens", nested),
        ("not Rust", String::from("SELECT * FROM t WHERE x = 'é' -- \u{0}\u{1}\u{7f} </html> \r\r\n")),
    ];
    for (what, text) in &inputs {
        let len = len_chars(text);
        let t = parse(text);
        let s = t.sexp();
        let n = t.node_count();
        assert!(s.starts_with('('), "{what}: sexp does not start with a node: {:?}", &s[..s.len().min(60)]);
        assert!(s.ends_with(')'), "{what}: sexp is not closed");
        check_sexp_offsets(what, &s, len);
        check_spans(what, &t, len);
        assert!(n >= 1, "{what}: node_count is 0");
        assert_eq!(t.text(), text.as_str(), "{what}: text() differs from the input");
    }

    // 1 MB of `(` is not valid Rust.
    assert!(parse(&inputs[3].1).has_error(), "1 MB of ( parses without errors");

    // Deep valid nesting: R-067 found 0 errors at depth about 10,000; every level is rendered and
    // counted (each level is a named node plus `(` and `)`).
    let deep = parse(&inputs[5].1);
    assert!(deep.errors().is_empty() && !deep.has_error(), "deep balanced nesting reports errors: {:?}", &deep.errors()[..deep.errors().len().min(3)]);
    let levels = deep.sexp().matches("(parenthesized_expression@").count();
    assert_eq!(levels, 10_000, "sexp renders {levels} of 10,000 nested parenthesized expressions");
    assert!(deep.node_count() >= 30_000, "node_count {} does not count every nested level", deep.node_count());
}

// ---------------------------------------------------------------------------------------------
// Interface: traits, Debug, Display, Unchanged, Clone independence
// ---------------------------------------------------------------------------------------------

fn assert_traits<T: Clone + std::fmt::Debug + Send + 'static>() {}
fn assert_error<E: std::error::Error + Copy + PartialEq>() {}

#[test]
fn interface_contract_holds() {
    assert_traits::<SyntaxTree>();
    assert_traits::<ErrorSpan>();
    assert_error::<ParseError>();
    assert_eq!(ParseError::NoTree.to_string(), "tree-sitter returned no tree");

    // Debug prints the char length and has_error, not the text.
    let text = "fn zq_marker_name() { \"ééé\" }";
    let t = parse(text);
    let dbg = format!("{t:?}");
    assert!(!dbg.contains("zq_marker_name"), "Debug prints the text: {dbg}");
    assert!(dbg.contains(&len_chars(text).to_string()), "Debug lacks the char length {}: {dbg}", len_chars(text));

    // `update` to the same text is Unchanged and keeps the tree.
    let mut t = parse("fn f() { let x = 1; }");
    let before = t.sexp();
    assert_eq!(t.update("fn f() { let x = 1; }"), Ok(ParseMode::Unchanged));
    assert_eq!(t.sexp(), before);
    // A change is Incremental when the fresh tree has no error.
    assert_eq!(t.update("fn f() { let xy = 1; }"), Ok(ParseMode::Incremental));
    assert_eq!(t.sexp(), parse("fn f() { let xy = 1; }").sexp());
    // Changing a clone leaves the original alone.
    let mut c = t.clone();
    let mode = c.update("fn g() {}");
    assert!(matches!(mode, Ok(ParseMode::Incremental)), "clone update: {mode:?}");
    assert_eq!(t.text(), "fn f() { let xy = 1; }");
    assert_eq!(t.sexp(), parse("fn f() { let xy = 1; }").sexp());
    assert_eq!(c.sexp(), parse("fn g() {}").sexp());
    // Edits next to multi-byte chars and CRLF.
    let steps = ["fn é() {}\r\n", "fn é😀() {}\r\n", "fn é😀() { \"\r\n\" }\r\n", "fn 😀() { \"\r\n\" }\r\n", "fn 😀() {}"];
    let mut t = parse(steps[0]);
    for s in &steps[1..] {
        let mode = t.update(s);
        assert!(mode.is_ok(), "update to {s:?}: {mode:?}");
        let fresh = parse(s);
        assert_eq!((t.sexp(), t.errors(), t.node_count()), (fresh.sexp(), fresh.errors(), fresh.node_count()), "update to {s:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// AC-7: corpus coverage
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct Census {
    found: BTreeSet<&'static str>,
    or_depth: usize,
}

impl Census {
    fn hit(&mut self, what: &'static str) {
        self.found.insert(what);
    }
}

fn let_in_and_chain(e: &syn::Expr) -> bool {
    match e {
        syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::And(_)) => {
            matches!(*b.left, syn::Expr::Let(_))
                || matches!(*b.right, syn::Expr::Let(_))
                || let_in_and_chain(&b.left)
                || let_in_and_chain(&b.right)
        }
        syn::Expr::Paren(p) => let_in_and_chain(&p.expr),
        _ => false,
    }
}

impl<'a> Visit<'a> for Census {
    fn visit_expr_if(&mut self, e: &'a syn::ExprIf) {
        if let_in_and_chain(&e.cond) {
            self.hit("ExprLet in && in if");
        }
        visit::visit_expr_if(self, e);
    }
    fn visit_local(&mut self, l: &'a syn::Local) {
        if l.init.as_ref().is_some_and(|i| i.diverge.is_some()) {
            self.hit("let-else");
        }
        visit::visit_local(self, l);
    }
    fn visit_type_param_bound(&mut self, b: &'a syn::TypeParamBound) {
        if matches!(b, syn::TypeParamBound::PreciseCapture(_)) {
            self.hit("use<> precise capture");
        }
        visit::visit_type_param_bound(self, b);
    }
    fn visit_expr_async(&mut self, e: &'a syn::ExprAsync) {
        self.hit("ExprAsync");
        visit::visit_expr_async(self, e);
    }
    fn visit_expr_await(&mut self, e: &'a syn::ExprAwait) {
        self.hit("ExprAwait");
        visit::visit_expr_await(self, e);
    }
    fn visit_expr_closure(&mut self, e: &'a syn::ExprClosure) {
        self.hit(if e.capture.is_some() { "closure with move" } else { "closure without move" });
        visit::visit_expr_closure(self, e);
    }
    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        if p.subpat.is_some() {
            self.hit("binding @ subpattern");
        }
        visit::visit_pat_ident(self, p);
    }
    fn visit_pat_or(&mut self, p: &'a syn::PatOr) {
        if self.or_depth > 0 {
            self.hit("nested or-pattern");
        }
        self.or_depth += 1;
        visit::visit_pat_or(self, p);
        self.or_depth -= 1;
    }
    fn visit_expr_break(&mut self, e: &'a syn::ExprBreak) {
        if e.label.is_some() && e.expr.is_some() {
            self.hit("break 'label value");
        }
        visit::visit_expr_break(self, e);
    }
    fn visit_lit(&mut self, l: &'a syn::Lit) {
        match l {
            syn::Lit::CStr(_) => self.hit("Lit::CStr"),
            syn::Lit::ByteStr(_) => self.hit("Lit::ByteStr"),
            syn::Lit::Byte(_) => self.hit("Lit::Byte"),
            syn::Lit::Char(_) => self.hit("Lit::Char"),
            syn::Lit::Int(i) if !i.suffix().is_empty() => self.hit("suffixed LitInt"),
            syn::Lit::Float(f) if !f.suffix().is_empty() => self.hit("suffixed LitFloat"),
            syn::Lit::Str(s) if s.token().to_string().starts_with("r#") => self.hit("raw string r#\"\"#"),
            _ => {}
        }
        if let syn::Lit::Int(i) = l
            && i.to_string().contains('_')
        {
            self.hit("numeric separator");
        }
        visit::visit_lit(self, l);
    }
    fn visit_item_macro(&mut self, m: &'a syn::ItemMacro) {
        if m.mac.path.is_ident("macro_rules") {
            self.hit("macro_rules!");
        } else {
            self.hit("macro in item position");
        }
        visit::visit_item_macro(self, m);
    }
    fn visit_stmt_macro(&mut self, m: &'a syn::StmtMacro) {
        self.hit("macro in statement position");
        visit::visit_stmt_macro(self, m);
    }
    fn visit_expr_macro(&mut self, m: &'a syn::ExprMacro) {
        self.hit("macro in expression position");
        visit::visit_expr_macro(self, m);
    }
    fn visit_item_foreign_mod(&mut self, m: &'a syn::ItemForeignMod) {
        self.hit("ItemForeignMod");
        if m.unsafety.is_some() {
            self.hit("unsafe extern block");
        }
        visit::visit_item_foreign_mod(self, m);
    }
    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        if f.sig.unsafety.is_some() {
            self.hit("unsafe fn");
        }
        if f.sig.asyncness.is_some() {
            self.hit("async fn");
        }
        visit::visit_item_fn(self, f);
    }
    fn visit_expr_unsafe(&mut self, e: &'a syn::ExprUnsafe) {
        self.hit("ExprUnsafe");
        visit::visit_expr_unsafe(self, e);
    }
    fn visit_expr_try(&mut self, e: &'a syn::ExprTry) {
        self.hit("ExprTry");
        visit::visit_expr_try(self, e);
    }
    fn visit_expr_range(&mut self, e: &'a syn::ExprRange) {
        self.hit("ExprRange");
        visit::visit_expr_range(self, e);
    }
    fn visit_ident(&mut self, i: &'a proc_macro2::Ident) {
        if i.to_string().starts_with("r#") {
            self.hit("raw identifier");
        }
        if !i.to_string().is_ascii() {
            self.hit("non-ASCII identifier");
        }
        visit::visit_ident(self, i);
    }
    fn visit_fields_unnamed(&mut self, f: &'a syn::FieldsUnnamed) {
        self.hit("tuple struct fields");
        visit::visit_fields_unnamed(self, f);
    }
    fn visit_variant(&mut self, v: &'a syn::Variant) {
        if v.discriminant.is_some() {
            self.hit("enum discriminant");
        }
        visit::visit_variant(self, v);
    }
    fn visit_type_trait_object(&mut self, t: &'a syn::TypeTraitObject) {
        self.hit("TypeTraitObject");
        if t.dyn_token.is_some() {
            self.hit("dyn");
        }
        visit::visit_type_trait_object(self, t);
    }
    fn visit_type_impl_trait(&mut self, t: &'a syn::TypeImplTrait) {
        self.hit("TypeImplTrait");
        visit::visit_type_impl_trait(self, t);
    }
    fn visit_const_param(&mut self, c: &'a syn::ConstParam) {
        self.hit("const generic");
        visit::visit_const_param(self, c);
    }
    fn visit_lifetime_param(&mut self, l: &'a syn::LifetimeParam) {
        self.hit("lifetime parameter");
        visit::visit_lifetime_param(self, l);
    }
    fn visit_where_clause(&mut self, w: &'a syn::WhereClause) {
        self.hit("where clause");
        for p in &w.predicates {
            if let syn::WherePredicate::Type(t) = p
                && matches!(&t.bounded_ty, syn::Type::Path(tp) if tp.path.is_ident("Self"))
                && t.bounds.iter().any(|b| matches!(b, syn::TypeParamBound::Trait(tb) if tb.path.is_ident("Sized")))
            {
                self.hit("where Self: Sized");
            }
        }
        visit::visit_where_clause(self, w);
    }
    fn visit_trait_item_type(&mut self, t: &'a syn::TraitItemType) {
        self.hit("associated type");
        visit::visit_trait_item_type(self, t);
    }
    fn visit_impl_item_type(&mut self, t: &'a syn::ImplItemType) {
        self.hit("associated type");
        visit::visit_impl_item_type(self, t);
    }
    fn visit_trait_item_const(&mut self, t: &'a syn::TraitItemConst) {
        self.hit("associated const");
        visit::visit_trait_item_const(self, t);
    }
    fn visit_impl_item_const(&mut self, t: &'a syn::ImplItemConst) {
        self.hit("associated const");
        visit::visit_impl_item_const(self, t);
    }
    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        if i.trait_.is_some() && !i.generics.params.is_empty() {
            self.hit("generic trait impl");
        }
        visit::visit_item_impl(self, i);
    }
    fn visit_item_mod(&mut self, m: &'a syn::ItemMod) {
        if m.content.is_some() {
            self.hit("inline mod");
        }
        visit::visit_item_mod(self, m);
    }
    fn visit_attribute(&mut self, a: &'a syn::Attribute) {
        let doc = a.path().is_ident("doc");
        match (a.style, doc) {
            (syn::AttrStyle::Inner(_), false) => self.hit("inner attribute"),
            (syn::AttrStyle::Outer, false) => self.hit("outer attribute"),
            _ => {}
        }
        visit::visit_attribute(self, a);
    }
    fn visit_expr_while(&mut self, e: &'a syn::ExprWhile) {
        if e.label.is_some() {
            self.hit("labeled loop");
        }
        visit::visit_expr_while(self, e);
    }
    fn visit_expr_loop(&mut self, e: &'a syn::ExprLoop) {
        if e.label.is_some() {
            self.hit("labeled loop");
        }
        visit::visit_expr_loop(self, e);
    }
    fn visit_expr_for_loop(&mut self, e: &'a syn::ExprForLoop) {
        if e.label.is_some() {
            self.hit("labeled loop");
        }
        visit::visit_expr_for_loop(self, e);
    }
    fn visit_pat_slice(&mut self, p: &'a syn::PatSlice) {
        self.hit("slice pattern");
        visit::visit_pat_slice(self, p);
    }
    fn visit_expr_array(&mut self, e: &'a syn::ExprArray) {
        self.hit("array expression");
        visit::visit_expr_array(self, e);
    }
    fn visit_type_slice(&mut self, t: &'a syn::TypeSlice) {
        self.hit("slice type");
        visit::visit_type_slice(self, t);
    }
    fn visit_expr_match(&mut self, m: &'a syn::ExprMatch) {
        if m.arms.iter().any(|a| a.guard.is_some()) {
            self.hit("match guard");
        }
        visit::visit_expr_match(self, m);
    }
}

/// A `/*` that opens a comment containing another `/*` before its first `*/`.
fn has_nested_block_comment(text: &str) -> bool {
    let mut rest = text;
    while let Some(i) = rest.find("/*") {
        let after = &rest[i + 2..];
        match (after.find("/*"), after.find("*/")) {
            (Some(open), Some(close)) if open < close => return true,
            _ => rest = after,
        }
    }
    false
}

#[test]
fn corpus_covers_checklist() {
    let files = corpus_set("valid");
    let mut census = Census::default();
    let mut text_hits: BTreeSet<&'static str> = BTreeSet::new();
    let mut crlf_files = Vec::new();
    let mut non_ascii_string_files = BTreeSet::new();
    for (path, text) in &files {
        let ast = syn::parse_file(text).unwrap_or_else(|e| panic!("{path}: syn::parse_file failed: {e}"));
        let mut per_file = Census::default();
        per_file.visit_file(&ast);
        if per_file.found.contains("non-ASCII identifier") {
            struct Strs(bool);
            impl<'a> Visit<'a> for Strs {
                fn visit_lit_str(&mut self, s: &'a syn::LitStr) {
                    if !s.value().is_ascii() {
                        self.0 = true;
                    }
                }
            }
            let mut s = Strs(false);
            s.visit_file(&ast);
            if s.0 {
                non_ascii_string_files.insert(path.clone());
            }
        }
        census.found.extend(per_file.found);
        for (needle, name) in [("///", "///"), ("//!", "//!"), ("/**", "/**"), ("\r\n", "\\r\\n")] {
            if text.contains(needle) {
                text_hits.insert(name);
            }
        }
        if has_nested_block_comment(text) {
            text_hits.insert("nested /* /* */ */");
        }
        let b = text.as_bytes();
        let lf = b.iter().filter(|&&c| c == b'\n').count();
        let crlf = b.windows(2).filter(|w| w == b"\r\n").count();
        if lf > 0 && lf == crlf {
            crlf_files.push(path.clone());
        }
    }

    let required = [
        "ExprLet in && in if",
        "let-else",
        "use<> precise capture",
        "ExprAsync",
        "ExprAwait",
        "closure with move",
        "closure without move",
        "binding @ subpattern",
        "nested or-pattern",
        "break 'label value",
        "Lit::CStr",
        "Lit::ByteStr",
        "Lit::Byte",
        "Lit::Char",
        "suffixed LitInt",
        "suffixed LitFloat",
        "macro_rules!",
        "ItemForeignMod",
        "unsafe extern block",
        "unsafe fn",
        "ExprUnsafe",
        "ExprTry",
        "ExprRange",
        "raw identifier",
        "tuple struct fields",
        "enum discriminant",
        "TypeTraitObject",
        "TypeImplTrait",
        "const generic",
        "associated type",
        "associated const",
        // The rest of the Pinned (b) checklist.
        "lifetime parameter",
        "where clause",
        "where Self: Sized",
        "async fn",
        "match guard",
        "labeled loop",
        "raw string r#\"\"#",
        "numeric separator",
        "inner attribute",
        "outer attribute",
        "inline mod",
        "macro in item position",
        "macro in statement position",
        "macro in expression position",
        "slice pattern",
        "array expression",
        "slice type",
        "generic trait impl",
        "dyn",
        "non-ASCII identifier",
    ];
    let missing: Vec<&str> = required.iter().copied().filter(|r| !census.found.contains(r)).collect();
    assert!(missing.is_empty(), "the valid corpus lacks: {missing:?}");
    let text_required = ["///", "//!", "/**", "nested /* /* */ */", "\\r\\n"];
    let missing: Vec<&str> = text_required.iter().copied().filter(|r| !text_hits.contains(r)).collect();
    assert!(missing.is_empty(), "text scan of the valid corpus lacks: {missing:?}");
    assert!(!crlf_files.is_empty(), "no valid file uses CRLF line endings throughout");
    assert!(
        !non_ascii_string_files.is_empty(),
        "no valid file has both non-ASCII identifiers and non-ASCII strings"
    );
}
