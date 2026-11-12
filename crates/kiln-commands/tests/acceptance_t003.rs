//! Acceptance tests for T-003 (documents through schema-validated `doc.*` commands, the
//! command log and `Registry::replay`), written from the task specification alone by the test
//! author.
//!
//! Every test dispatches commands by name with JSON params through
//! `standard_registry().execute` and observes results, `doc.*` queries, `Context::vfs()` and
//! `Context::log()`. Random sequences use a fixed-seed generator, so runs are deterministic.
//! Tests that need no file-backed context live in `acceptance_t003_registry.rs`.

use kiln_commands::{CommandError, Context, LogEntry, Registry, ReplayError, standard_registry};
use kiln_text::{Buffer, LineEnding, LineEndings};
use kiln_workspace::RelPath;
use serde_json::{Value, json};
use support::*;

#[cfg(test)]
mod support {
    use super::*;
    use std::collections::BTreeMap;

    pub fn reg() -> Registry {
        standard_registry().expect("standard registry builds")
    }

    pub fn ctx_files(files: &[(&str, &[u8])]) -> Context {
        Context::with_files(files).unwrap_or_else(|e| panic!("with_files({files:?}): {e:?}"))
    }

    pub fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
        r.execute(ctx, name, params.clone())
            .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
    }

    pub fn code(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> String {
        match r.execute(ctx, name, params.clone()) {
            Ok(v) => panic!("{name} {params} succeeded with {v}, expected an error"),
            Err(e) => e.to_json()["code"]
                .as_str()
                .unwrap_or_else(|| panic!("error without code: {e:?}"))
                .to_owned(),
        }
    }

    /// Assert that `name params` fails with `want` and changes nothing observable: the log,
    /// `doc.list` and, for every listed document, `doc.text` and `doc.info`.
    pub fn assert_fails_unchanged(
        r: &Registry,
        ctx: &mut Context,
        name: &str,
        params: Value,
        want: &str,
    ) {
        let before = observe(r, ctx);
        let log_len = ctx.log().len();
        let log_before = ctx.log().to_vec();
        let got = code(r, ctx, name, params.clone());
        assert_eq!(got, want, "{name} {params}");
        assert_eq!(
            ctx.log().len(),
            log_len,
            "failed {name} {params} was logged"
        );
        assert_eq!(
            ctx.log(),
            &log_before[..],
            "failed {name} {params} changed the log"
        );
        let after = observe(r, ctx);
        assert_eq!(before, after, "failed {name} {params} changed state");
    }

    /// `doc.list` plus `doc.text` and `doc.info` of every listed document (through commands).
    pub fn observe(r: &Registry, ctx: &mut Context) -> Value {
        let list = ok(r, ctx, "doc.list", json!({}));
        let mut docs = Vec::new();
        for d in list["docs"].as_array().expect("docs array") {
            let id = d["doc"].clone();
            let text = ok(r, ctx, "doc.text", json!({ "doc": id }));
            let info = ok(r, ctx, "doc.info", json!({ "doc": id }));
            docs.push(json!({"text": text, "info": info}));
        }
        json!({"list": list, "docs": docs})
    }

    pub fn rp(s: &str) -> RelPath {
        RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}): {e:?}"))
    }

    pub fn file(ctx: &Context, path: &str) -> Vec<u8> {
        ctx.vfs()
            .read(&rp(path))
            .unwrap_or_else(|e| panic!("read {path}: {e:?}"))
    }

    pub fn text_of(r: &Registry, ctx: &mut Context, doc: u64) -> String {
        ok(r, ctx, "doc.text", json!({ "doc": doc }))["text"]
            .as_str()
            .expect("text is a string")
            .to_owned()
    }

    pub fn info(r: &Registry, ctx: &mut Context, doc: u64) -> Value {
        ok(r, ctx, "doc.info", json!({ "doc": doc }))
    }

    pub fn open_id(r: &Registry, ctx: &mut Context, path: &str) -> u64 {
        ok(r, ctx, "doc.open", json!({ "path": path }))["doc"]
            .as_u64()
            .expect("doc id is an integer")
    }

    pub fn ending_name(b: &Buffer) -> &'static str {
        match b.dominant_line_ending() {
            None => "none",
            Some(LineEnding::Lf) => "lf",
            Some(LineEnding::Crlf) => "crlf",
            Some(LineEnding::Cr) => "cr",
        }
    }

    pub fn mixed(b: &Buffer) -> bool {
        b.line_endings() == LineEndings::Mixed
    }

    /// Small deterministic PRNG (SplitMix64).
    pub struct Rng(pub u64);
    impl Rng {
        pub fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        pub fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
            &xs[self.below(xs.len() as u64) as usize]
        }
    }

    pub const SNIPPETS: &[&str] = &[
        "a",
        "b",
        "é",
        "😀",
        " ",
        "\n",
        "\r\n",
        "\r",
        "xy",
        "fn x() {}",
        "é😀\r\n",
        "",
    ];

    /// The initial files F of the log/replay tests.
    pub fn replay_files() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("a.txt", b"hello\nworld\n"),
            ("b.txt", b"b\r\nb\r\n"),
            ("src/c.rs", "fn c() { \"é😀\" }\r".as_bytes()),
            ("bom.txt", b"\xEF\xBB\xBFbom\n"),
            ("bad.bin", b"\xff"),
        ]
    }

    pub const REPLAY_PATHS: &[&str] = &[
        "a.txt",
        "b.txt",
        "src/c.rs",
        "bom.txt",
        "bad.bin",
        "missing.txt",
        "../x",
        "src",
        "",
    ];

    /// One generated command: name and params. Tracks ids and lengths seen in results so
    /// most commands hit an open document in range; some deliberately fail.
    pub struct Gen {
        pub rng: Rng,
        pub lens: BTreeMap<u64, u64>,
    }

    impl Gen {
        pub fn new(seed: u64) -> Self {
            Self {
                rng: Rng(seed),
                lens: BTreeMap::new(),
            }
        }

        fn doc(&mut self) -> u64 {
            if self.lens.is_empty() || self.rng.below(10) == 0 {
                self.rng.below(8)
            } else {
                let ids: Vec<u64> = self.lens.keys().copied().collect();
                *self.rng.pick(&ids)
            }
        }

        fn len_of(&self, doc: u64) -> u64 {
            self.lens.get(&doc).copied().unwrap_or(5)
        }

        fn offset(&mut self, doc: u64) -> u64 {
            let len = self.len_of(doc);
            self.rng.below(len + 3)
        }

        pub fn command(&mut self) -> (String, Value) {
            let k = self.rng.below(100);
            let (name, params) = match k {
                0..=11 => {
                    let p = *self.rng.pick(REPLAY_PATHS);
                    ("doc.open", json!({ "path": p }))
                }
                12..=33 => {
                    let d = self.doc();
                    let o = self.offset(d);
                    let t = *self.rng.pick(SNIPPETS);
                    ("doc.insert", json!({"doc": d, "offset": o, "text": t}))
                }
                34..=41 => {
                    let d = self.doc();
                    let (a, b) = (self.offset(d), self.offset(d));
                    let (s, e) = if self.rng.below(8) == 0 {
                        (b, a)
                    } else {
                        (a.min(b), a.max(b))
                    };
                    ("doc.delete", json!({"doc": d, "start": s, "end": e}))
                }
                42..=49 => {
                    let d = self.doc();
                    let (a, b) = (self.offset(d), self.offset(d));
                    let t = *self.rng.pick(SNIPPETS);
                    (
                        "doc.replace",
                        json!({"doc": d, "start": a.min(b), "end": a.max(b), "text": t}),
                    )
                }
                50..=59 => ("doc.undo", json!({"doc": self.doc()})),
                60..=65 => ("doc.redo", json!({"doc": self.doc()})),
                66..=71 => ("doc.save", json!({"doc": self.doc()})),
                72..=76 => {
                    let d = self.doc();
                    match self.rng.below(3) {
                        0 => ("doc.close", json!({"doc": d})),
                        1 => ("doc.close", json!({"doc": d, "discard": true})),
                        _ => ("doc.close", json!({"doc": d, "discard": false})),
                    }
                }
                77..=81 => ("doc.text", json!({"doc": self.doc()})),
                82..=85 => ("doc.info", json!({"doc": self.doc()})),
                86..=88 => ("doc.list", json!({})),
                89..=90 => {
                    let t = *self.rng.pick(SNIPPETS);
                    let o = self.rng.below(4);
                    ("buffer.insert", json!({"offset": o, "text": t}))
                }
                91 => ("buffer.text", json!({})),
                92..=94 => {
                    // Schema violations: missing field, wrong type, unknown field.
                    let d = self.doc();
                    match self.rng.below(3) {
                        0 => ("doc.insert", json!({"doc": d, "offset": 0})),
                        1 => ("doc.text", json!({"doc": d.to_string()})),
                        _ => ("doc.undo", json!({"doc": d, "extra": 1})),
                    }
                }
                95 => ("doc.nope", json!({})),
                _ => ("doc.list", json!({"all": true})),
            };
            (name.to_owned(), params)
        }

        /// Learn ids and lengths from a successful result.
        pub fn observe(&mut self, name: &str, params: &Value, result: &Value) {
            if name == "doc.close" {
                if let Some(d) = params["doc"].as_u64() {
                    self.lens.remove(&d);
                }
                return;
            }
            let doc = result["doc"].as_u64().or_else(|| params["doc"].as_u64());
            if let (Some(d), Some(len)) = (doc, result["len_chars"].as_u64()) {
                self.lens.insert(d, len);
            }
        }
    }

    /// Run `n` generated commands on `ctx`; return each command and its outcome.
    pub fn run_random(
        r: &Registry,
        ctx: &mut Context,
        seed: u64,
        n: usize,
    ) -> Vec<(String, Value, Result<Value, CommandError>)> {
        let mut g = Gen::new(seed);
        let mut out = Vec::new();
        for _ in 0..n {
            let (name, params) = g.command();
            let res = r.execute(ctx, &name, params.clone());
            if let Ok(v) = &res {
                g.observe(&name, &params, v);
            }
            out.push((name, params, res));
        }
        out
    }

    /// Name → declared `mutating` flag, from `describe()`.
    pub fn mutating_flags(r: &Registry) -> BTreeMap<String, bool> {
        r.describe()["commands"]
            .as_array()
            .expect("commands array")
            .iter()
            .map(|c| {
                (
                    c["name"].as_str().expect("name").to_owned(),
                    c["mutating"]
                        .as_bool()
                        .unwrap_or_else(|| panic!("mutating not a bool in {}", c["name"])),
                )
            })
            .collect()
    }

    /// Everything the replay criterion compares, read without going through commands except
    /// for the documents: VFS contents of every path in F, scratch buffer, `doc.list`, and
    /// `doc.text` of each listed document. Call on both contexts in the same way.
    pub fn final_state(r: &Registry, ctx: &mut Context, files: &[(&str, &[u8])]) -> Value {
        let vfs: Vec<Value> = files
            .iter()
            .map(|(p, _)| json!([p, ctx.vfs().read(&rp(p)).ok()]))
            .collect();
        let buffer = ctx.buffer().text();
        let list = ok(r, ctx, "doc.list", json!({}));
        let mut texts = Vec::new();
        for d in list["docs"].as_array().expect("docs") {
            texts.push(ok(r, ctx, "doc.text", json!({"doc": d["doc"]})));
        }
        json!({"vfs": vfs, "buffer": buffer, "list": list, "texts": texts})
    }
}

// ---------------------------------------------------------------------------------------------
// AC-1 (slice)

#[test]
fn open_edit_save_slice_exact_values() {
    let r = reg();
    let mut ctx = ctx_files(&[("a.txt", b"hello\n")]);
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"})),
        json!({"doc": 1, "already_open": false, "path": "a.txt", "encoding": "utf-8",
               "line_ending": "lf", "mixed_line_endings": false, "len_chars": 6})
    );
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 5, "text": " world"})
        ),
        json!({"len_chars": 12, "dirty": true})
    );
    // Not yet saved: the file is untouched.
    assert_eq!(file(&ctx, "a.txt"), b"hello\n");
    assert_eq!(
        ok(&r, &mut ctx, "doc.save", json!({"doc": 1})),
        json!({"doc": 1, "bytes_written": 12, "dirty": false})
    );
    assert_eq!(file(&ctx, "a.txt"), b"hello world\n");
    assert_eq!(
        info(&r, &mut ctx, 1),
        json!({"doc": 1, "path": "a.txt", "encoding": "utf-8", "line_ending": "lf",
               "mixed_line_endings": false, "len_chars": 12,
               "len_lines": Buffer::from_text("hello world\n").len_lines(),
               "dirty": false, "can_undo": true, "can_redo": false})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.text", json!({"doc": 1})),
        json!({"text": "hello world\n", "len_chars": 12})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": false}]})
    );
}

#[test]
fn typed_doc_open_example_from_the_spec() {
    let r = reg();
    let mut ctx = ctx_files(&[("a.rs", b"fn a() {}\r\n")]);
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.rs"})),
        json!({"already_open": false, "doc": 1, "encoding": "utf-8", "len_chars": 11,
               "line_ending": "crlf", "mixed_line_endings": false, "path": "a.rs"})
    );
}

// ---------------------------------------------------------------------------------------------
// AC-2 (edits differential against kiln_text::Buffer)

#[test]
fn edit_commands_match_buffer_model() {
    let r = reg();
    let initial = "fn main() {\n    let s = \"héllo 😀\";\r\n}\n";
    let mut ctx = ctx_files(&[("m.rs", initial.as_bytes())]);
    assert_eq!(open_id(&r, &mut ctx, "m.rs"), 1);
    let mut model = Buffer::from_text(initial);
    let mut saved = initial.to_owned();
    let mut rng = Rng(0x7003_0002);
    let mut rejected = 0;
    let mut commands = 0;
    for step in 0..700 {
        let len = model.len_chars() as u64;
        // Mostly in range; sometimes beyond the end (rejected).
        let off = |rng: &mut Rng| {
            if rng.below(12) == 0 {
                len + 1 + rng.below(3) + if rng.below(4) == 0 { 1_000_000 } else { 0 }
            } else {
                rng.below(len + 1)
            }
        };
        let k = rng.below(100);
        let (name, params) = match k {
            0..=39 => {
                let o = off(&mut rng);
                let t = *rng.pick(SNIPPETS);
                ("doc.insert", json!({"doc": 1, "offset": o, "text": t}))
            }
            40..=54 => {
                let (a, b) = (off(&mut rng), off(&mut rng));
                let (s, e) = if rng.below(10) == 0 {
                    (a.max(b), a.min(b))
                } else {
                    (a.min(b), a.max(b))
                };
                ("doc.delete", json!({"doc": 1, "start": s, "end": e}))
            }
            55..=69 => {
                let (a, b) = (off(&mut rng), off(&mut rng));
                let t = *rng.pick(SNIPPETS);
                (
                    "doc.replace",
                    json!({"doc": 1, "start": a.min(b), "end": a.max(b), "text": t}),
                )
            }
            70..=84 => ("doc.undo", json!({"doc": 1})),
            85..=96 => ("doc.redo", json!({"doc": 1})),
            _ => ("doc.save", json!({"doc": 1})),
        };
        commands += 1;
        // Apply to the model.
        let as_usize = |v: &Value| v.as_u64().expect("u64") as usize;
        let expected: Result<Value, ()> = match name {
            "doc.insert" => {
                let t = params["text"].as_str().expect("text");
                model.insert(as_usize(&params["offset"]), t).map(|()| {
                    json!({"len_chars": model.len_chars(), "dirty": model.text() != saved})
                }).map_err(|_| ())
            }
            "doc.delete" => model
                .delete(as_usize(&params["start"])..as_usize(&params["end"]))
                .map(|removed| {
                    json!({"removed": removed, "len_chars": model.len_chars(),
                           "dirty": model.text() != saved})
                })
                .map_err(|_| ()),
            "doc.replace" => {
                let t = params["text"].as_str().expect("text");
                model
                    .replace(as_usize(&params["start"])..as_usize(&params["end"]), t)
                    .map(|removed| {
                        json!({"removed": removed, "len_chars": model.len_chars(),
                               "dirty": model.text() != saved})
                    })
                    .map_err(|_| ())
            }
            "doc.undo" => {
                let u = model.undo();
                Ok(
                    json!({"undone": u, "len_chars": model.len_chars(), "dirty": model.text() != saved}),
                )
            }
            "doc.redo" => {
                let u = model.redo();
                Ok(
                    json!({"redone": u, "len_chars": model.len_chars(), "dirty": model.text() != saved}),
                )
            }
            "doc.save" => {
                model.break_undo_group();
                saved = model.text();
                Ok(json!({"doc": 1, "bytes_written": saved.len(), "dirty": false}))
            }
            _ => unreachable!(),
        };
        match expected {
            Ok(want) => {
                let got = ok(&r, &mut ctx, name, params.clone());
                assert_eq!(got, want, "step {step}: {name} {params}");
            }
            Err(()) => {
                rejected += 1;
                assert_fails_unchanged(&r, &mut ctx, name, params.clone(), "rejected");
            }
        }
        assert_eq!(
            ok(&r, &mut ctx, "doc.text", json!({"doc": 1})),
            json!({"text": model.text(), "len_chars": model.len_chars()}),
            "step {step}: text after {name} {params}"
        );
        if step % 7 == 0 || name == "doc.save" {
            assert_eq!(
                info(&r, &mut ctx, 1),
                json!({"doc": 1, "path": "m.rs", "encoding": "utf-8",
                       "line_ending": ending_name(&model), "mixed_line_endings": mixed(&model),
                       "len_chars": model.len_chars(), "len_lines": model.len_lines(),
                       "dirty": model.text() != saved, "can_undo": model.can_undo(),
                       "can_redo": model.can_redo()}),
                "step {step}: info after {name} {params}"
            );
        }
        if name == "doc.save" {
            assert_eq!(
                file(&ctx, "m.rs"),
                saved.as_bytes(),
                "step {step}: file after save"
            );
        }
    }
    assert!(commands >= 500);
    assert!(
        rejected >= 20,
        "too few rejected commands exercised: {rejected}"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-3 (several documents, ws.documents)

#[test]
fn documents_have_independent_text_history_and_dirty_flag() {
    let r = reg();
    let mut ctx = ctx_files(&[("a.txt", b"alpha\n"), ("b.txt", b"beta\r\n")]);
    assert_eq!(open_id(&r, &mut ctx, "a.txt"), 1);
    assert_eq!(open_id(&r, &mut ctx, "b.txt"), 2);
    let doc2_text = ok(&r, &mut ctx, "doc.text", json!({"doc": 2}));
    let doc2_info = info(&r, &mut ctx, 2);
    assert_eq!(doc2_info["dirty"], false);
    assert_eq!(doc2_info["can_undo"], false);
    let check_doc2 = |ctx: &mut Context, when: &str| {
        assert_eq!(
            ok(&r, ctx, "doc.text", json!({"doc": 2})),
            doc2_text,
            "{when}"
        );
        assert_eq!(info(&r, ctx, 2), doc2_info, "{when}");
        assert_eq!(file(ctx, "b.txt"), b"beta\r\n", "{when}");
    };
    let steps: Vec<(&str, Value)> = vec![
        ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
        ("doc.insert", json!({"doc": 1, "offset": 1, "text": "y"})),
        ("doc.delete", json!({"doc": 1, "start": 0, "end": 1})),
        (
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": 1, "text": "ZZ"}),
        ),
        ("doc.undo", json!({"doc": 1})),
        ("doc.redo", json!({"doc": 1})),
        ("doc.save", json!({"doc": 1})),
        ("doc.insert", json!({"doc": 1, "offset": 2, "text": "\n"})),
    ];
    for (name, params) in steps {
        ok(&r, &mut ctx, name, params.clone());
        check_doc2(&mut ctx, &format!("after {name} {params}"));
    }
    assert_eq!(text_of(&r, &mut ctx, 1), "ZZ\nalpha\n");
    assert_eq!(file(&ctx, "a.txt"), b"ZZalpha\n");
    // Doc 2 has no history of its own even though doc 1 has.
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 2})),
        json!({"undone": false, "len_chars": 6, "dirty": false})
    );
    check_doc2(&mut ctx, "after doc.undo on doc 2");
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": true},
                        {"doc": 2, "path": "b.txt", "dirty": false}]})
    );
    // And the other direction: editing doc 2 leaves doc 1 alone.
    let doc1_text = ok(&r, &mut ctx, "doc.text", json!({"doc": 1}));
    let doc1_info = info(&r, &mut ctx, 1);
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 4, "text": "!"}),
    );
    assert_eq!(ok(&r, &mut ctx, "doc.text", json!({"doc": 1})), doc1_text);
    assert_eq!(info(&r, &mut ctx, 1), doc1_info);
    assert_eq!(text_of(&r, &mut ctx, 2), "beta!\r\n");
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": true},
                        {"doc": 2, "path": "b.txt", "dirty": true}]})
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
    assert_eq!(file(&ctx, "b.txt"), b"beta!\r\n");
    assert_eq!(file(&ctx, "a.txt"), b"ZZalpha\n");
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": true},
                        {"doc": 2, "path": "b.txt", "dirty": false}]})
    );
}

#[test]
fn close_and_reopen_gives_new_id_and_fresh_history() {
    let r = reg();
    let mut ctx = ctx_files(&[("a.txt", b"alpha\n"), ("b.txt", b"beta\n"), ("c.txt", b"")]);
    assert_eq!(open_id(&r, &mut ctx, "a.txt"), 1);
    assert_eq!(open_id(&r, &mut ctx, "b.txt"), 2);
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 5, "text": "!"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(info(&r, &mut ctx, 1)["can_undo"], true);
    assert_eq!(
        ok(&r, &mut ctx, "doc.close", json!({"doc": 1})),
        json!({"doc": 1, "closed": true})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 2, "path": "b.txt", "dirty": false}]})
    );
    for (name, params) in [
        ("doc.text", json!({"doc": 1})),
        ("doc.info", json!({"doc": 1})),
        ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
        ("doc.delete", json!({"doc": 1, "start": 0, "end": 1})),
        (
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": 1, "text": "x"}),
        ),
        ("doc.undo", json!({"doc": 1})),
        ("doc.redo", json!({"doc": 1})),
        ("doc.save", json!({"doc": 1})),
        ("doc.close", json!({"doc": 1})),
        ("doc.close", json!({"doc": 1, "discard": true})),
    ] {
        assert_fails_unchanged(&r, &mut ctx, name, params, "rejected");
    }
    assert_eq!(file(&ctx, "a.txt"), b"alpha!\n");
    // Re-open: a new id, the saved text, no history.
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"})),
        json!({"doc": 3, "already_open": false, "path": "a.txt", "encoding": "utf-8",
               "line_ending": "lf", "mixed_line_endings": false, "len_chars": 7})
    );
    let i = info(&r, &mut ctx, 3);
    assert_eq!(i["can_undo"], false, "{i}");
    assert_eq!(i["can_redo"], false, "{i}");
    assert_eq!(i["dirty"], false, "{i}");
    assert_eq!(text_of(&r, &mut ctx, 3), "alpha!\n");
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 3})),
        json!({"undone": false, "len_chars": 7, "dirty": false})
    );
    // Doc 1 stays closed; ids are never reused.
    assert_fails_unchanged(&r, &mut ctx, "doc.text", json!({"doc": 1}), "rejected");
    ok(&r, &mut ctx, "doc.close", json!({"doc": 2}));
    assert_eq!(open_id(&r, &mut ctx, "c.txt"), 4);
    assert_eq!(open_id(&r, &mut ctx, "b.txt"), 5);
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 3, "path": "a.txt", "dirty": false},
                        {"doc": 4, "path": "c.txt", "dirty": false},
                        {"doc": 5, "path": "b.txt", "dirty": false}]})
    );
}

// ---------------------------------------------------------------------------------------------
// AC-4 (dirty, ws.dirty-tracking)

#[test]
fn dirty_after_edit_clean_after_save_and_after_full_undo() {
    let r = reg();
    let mut ctx = ctx_files(&[("d.txt", b"abc")]);
    assert_eq!(open_id(&r, &mut ctx, "d.txt"), 1);
    let dirty = |ctx: &mut Context| info(&r, ctx, 1)["dirty"].clone();
    let listed = |ctx: &mut Context| ok(&r, ctx, "doc.list", json!({}))["docs"][0]["dirty"].clone();
    assert_eq!(dirty(&mut ctx), false, "after open");
    assert_eq!(listed(&mut ctx), false);
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 3, "text": "X"})
        ),
        json!({"len_chars": 4, "dirty": true})
    );
    assert_eq!(dirty(&mut ctx), true, "after edit");
    assert_eq!(listed(&mut ctx), true);
    assert_eq!(
        ok(&r, &mut ctx, "doc.save", json!({"doc": 1})),
        json!({"doc": 1, "bytes_written": 4, "dirty": false})
    );
    assert_eq!(dirty(&mut ctx), false, "after save");
    assert_eq!(listed(&mut ctx), false);
    // Two edits after the save, then undo both: clean again.
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": 1, "text": "Q"})
        ),
        json!({"removed": "a", "len_chars": 4, "dirty": true})
    );
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 4, "text": "\n"})
        ),
        json!({"len_chars": 5, "dirty": true})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 4, "dirty": true})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 4, "dirty": false})
    );
    assert_eq!(
        dirty(&mut ctx),
        false,
        "after undoing every edit since the save"
    );
    assert_eq!(listed(&mut ctx), false);
    // Redo makes it dirty again, undo clean again.
    assert_eq!(
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1})),
        json!({"redone": true, "len_chars": 4, "dirty": true})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 4, "dirty": false})
    );
    // Content model: an edit and its inverse edit (not undo) give the saved text: clean.
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "zz"})
        ),
        json!({"len_chars": 6, "dirty": true})
    );
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.delete",
            json!({"doc": 1, "start": 0, "end": 2})
        ),
        json!({"removed": "zz", "len_chars": 4, "dirty": false})
    );
    assert_eq!(dirty(&mut ctx), false, "text equals the saved text");
    assert_eq!(text_of(&r, &mut ctx, 1), "abcX");
    assert_eq!(file(&ctx, "d.txt"), b"abcX");
}

#[test]
fn undo_past_save_point_is_dirty() {
    let r = reg();
    let mut ctx = ctx_files(&[("u.txt", b"one\n")]);
    assert_eq!(open_id(&r, &mut ctx, "u.txt"), 1);
    ok(
        &r,
        &mut ctx,
        "doc.replace",
        json!({"doc": 1, "start": 0, "end": 3, "text": "ONE"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(file(&ctx, "u.txt"), b"ONE\n");
    assert_eq!(info(&r, &mut ctx, 1)["dirty"], false);
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 4, "dirty": true})
    );
    assert_eq!(text_of(&r, &mut ctx, 1), "one\n");
    let i = info(&r, &mut ctx, 1);
    assert_eq!(i["dirty"], true, "{i}");
    assert_eq!(i["can_redo"], true, "{i}");
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 1, "path": "u.txt", "dirty": true}]})
    );
    // Dirty: closing without discard is refused.
    assert_fails_unchanged(&r, &mut ctx, "doc.close", json!({"doc": 1}), "rejected");
    assert_eq!(
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1})),
        json!({"redone": true, "len_chars": 4, "dirty": false})
    );
    assert_eq!(info(&r, &mut ctx, 1)["dirty"], false);
    // Undo past the save point again, then save: the older text is the new save point.
    ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
    assert_eq!(
        ok(&r, &mut ctx, "doc.save", json!({"doc": 1})),
        json!({"doc": 1, "bytes_written": 4, "dirty": false})
    );
    assert_eq!(file(&ctx, "u.txt"), b"one\n");
    assert_eq!(
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1})),
        json!({"redone": true, "len_chars": 4, "dirty": true})
    );
}

// ---------------------------------------------------------------------------------------------
// AC-5 (line endings, io.crlf)

#[test]
fn lf_and_crlf_detected_on_open_and_kept_on_save() {
    let r = reg();
    let files: &[(&str, &[u8], &str, u64)] = &[
        ("lf.txt", b"a\nb\n", "lf", 4),
        ("crlf.txt", b"a\r\nb\r\n", "crlf", 6),
        ("cr.txt", b"a\rb", "cr", 3),
        ("none.txt", b"abc", "none", 3),
    ];
    let pairs: Vec<(&str, &[u8])> = files.iter().map(|(p, b, _, _)| (*p, *b)).collect();
    let mut ctx = ctx_files(&pairs);
    for (i, (path, bytes, ending, len)) in files.iter().enumerate() {
        assert_eq!(
            ok(&r, &mut ctx, "doc.open", json!({"path": path})),
            json!({"doc": i + 1, "already_open": false, "path": path, "encoding": "utf-8",
                   "line_ending": ending, "mixed_line_endings": false, "len_chars": len}),
            "{path}"
        );
        let i = info(&r, &mut ctx, i as u64 + 1);
        assert_eq!(i["line_ending"], *ending, "{path}: {i}");
        assert_eq!(i["mixed_line_endings"], false, "{path}: {i}");
        // Unedited save writes the original bytes.
        assert_eq!(
            ok(&r, &mut ctx, "doc.save", json!({"doc": i["doc"]})),
            json!({"doc": i["doc"], "bytes_written": bytes.len(), "dirty": false}),
            "{path}"
        );
        assert_eq!(file(&ctx, path), *bytes, "{path}: unedited save");
    }
    // Edit and save: each file keeps its own line breaks.
    for (doc, path, want) in [
        (1u64, "lf.txt", &b"aX\nb\n"[..]),
        (2, "crlf.txt", &b"aX\r\nb\r\n"[..]),
        (3, "cr.txt", &b"aX\rb"[..]),
        (4, "none.txt", &b"aXbc"[..]),
    ] {
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": doc, "offset": 1, "text": "X"}),
        );
        ok(&r, &mut ctx, "doc.save", json!({"doc": doc}));
        assert_eq!(file(&ctx, path), want, "{path}");
    }
    // An inserted "\n" in the CRLF file is saved as "\n" (no normalisation).
    let len = ok(&r, &mut ctx, "doc.text", json!({"doc": 2}))["len_chars"].clone();
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": len, "text": "\n"}),
    );
    let i = info(&r, &mut ctx, 2);
    assert_eq!(i["line_ending"], "crlf", "{i}");
    assert_eq!(i["mixed_line_endings"], true, "{i}");
    ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
    assert_eq!(file(&ctx, "crlf.txt"), b"aX\r\nb\r\n\n");
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 2, "text": "\n"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
    assert_eq!(file(&ctx, "crlf.txt"), b"aX\n\r\nb\r\n\n");
    // Line ending is that of the current text: the "none" file gains a CRLF.
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 4, "offset": 4, "text": "\r\n"}),
    );
    let i = info(&r, &mut ctx, 4);
    assert_eq!(i["line_ending"], "crlf", "{i}");
    assert_eq!(i["mixed_line_endings"], false, "{i}");
    ok(&r, &mut ctx, "doc.undo", json!({"doc": 4}));
    assert_eq!(info(&r, &mut ctx, 4)["line_ending"], "none");
}

#[test]
fn bom_and_utf16_files_round_trip_through_commands() {
    let r = reg();
    let utf16le: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("a\r\nb".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let mut ctx = ctx_files(&[("bom.txt", b"\xEF\xBB\xBFx\r\n"), ("w.txt", &utf16le)]);
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "bom.txt"})),
        json!({"doc": 1, "already_open": false, "path": "bom.txt", "encoding": "utf-8-bom",
               "line_ending": "crlf", "mixed_line_endings": false, "len_chars": 3})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "w.txt"})),
        json!({"doc": 2, "already_open": false, "path": "w.txt", "encoding": "utf-16le",
               "line_ending": "crlf", "mixed_line_endings": false, "len_chars": 4})
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
    assert_eq!(file(&ctx, "bom.txt"), b"\xEF\xBB\xBFx\r\n");
    assert_eq!(file(&ctx, "w.txt"), utf16le);
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 4, "text": "é"}),
    );
    let want: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("a\r\nbé".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    assert_eq!(
        ok(&r, &mut ctx, "doc.save", json!({"doc": 2})),
        json!({"doc": 2, "bytes_written": want.len(), "dirty": false})
    );
    assert_eq!(file(&ctx, "w.txt"), want);
    assert_eq!(info(&r, &mut ctx, 2)["encoding"], "utf-16le");
}

#[test]
fn mixed_line_endings_reported() {
    let r = reg();
    let mut ctx = ctx_files(&[
        ("mix.txt", b"a\r\nb\r\nc\n"),
        ("mix2.txt", b"a\rb\nc\n"),
        ("lf.txt", b"a\nb\n"),
    ]);
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "mix.txt"})),
        json!({"doc": 1, "already_open": false, "path": "mix.txt", "encoding": "utf-8",
               "line_ending": "crlf", "mixed_line_endings": true, "len_chars": 8})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "mix2.txt"})),
        json!({"doc": 2, "already_open": false, "path": "mix2.txt", "encoding": "utf-8",
               "line_ending": "lf", "mixed_line_endings": true, "len_chars": 6})
    );
    let i = info(&r, &mut ctx, 1);
    assert_eq!(
        (i["line_ending"].clone(), i["mixed_line_endings"].clone()),
        (json!("crlf"), json!(true))
    );
    // Unedited save keeps every break byte-for-byte.
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(file(&ctx, "mix.txt"), b"a\r\nb\r\nc\n");
    ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
    assert_eq!(file(&ctx, "mix2.txt"), b"a\rb\nc\n");
    // Edited save keeps the other breaks byte-for-byte.
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "X"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(file(&ctx, "mix.txt"), b"aX\r\nb\r\nc\n");
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 8, "text": "Y"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(file(&ctx, "mix.txt"), b"aX\r\nb\r\ncY\n");
    // A pure-LF document becomes mixed by an edit and pure again by undo.
    assert_eq!(open_id(&r, &mut ctx, "lf.txt"), 3);
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 3, "offset": 4, "text": "c\r\n"}),
    );
    let i = info(&r, &mut ctx, 3);
    assert_eq!(
        (i["line_ending"].clone(), i["mixed_line_endings"].clone()),
        (json!("lf"), json!(true))
    );
    ok(&r, &mut ctx, "doc.undo", json!({"doc": 3}));
    let i = info(&r, &mut ctx, 3);
    assert_eq!(
        (i["line_ending"].clone(), i["mixed_line_endings"].clone()),
        (json!("lf"), json!(false))
    );
}

// ---------------------------------------------------------------------------------------------
// AC-6 (errors and re-open)

#[test]
fn doc_command_errors_are_rejected_or_invalid_params() {
    let r = reg();
    let mut ctx = ctx_files(&[
        ("a.txt", b"hello\n"),
        ("bad.txt", b"\xff"),
        ("bad16.txt", b"\xFF\xFE\x00"),
        ("dir/f.txt", b"f"),
    ]);
    // Failed opens: rejected, nothing listed, nothing logged, no id consumed.
    for path in [
        "missing.txt",
        "bad.txt",
        "bad16.txt",
        "../x",
        "dir",
        "",
        "/a.txt",
        "a\\b",
        "./a.txt",
        "dir/../a.txt",
    ] {
        assert_fails_unchanged(&r, &mut ctx, "doc.open", json!({"path": path}), "rejected");
        assert_eq!(
            ok(&r, &mut ctx, "doc.list", json!({})),
            json!({"docs": []}),
            "{path}"
        );
    }
    assert_eq!(
        open_id(&r, &mut ctx, "a.txt"),
        1,
        "failed opens must not consume ids"
    );
    for path in ["missing.txt", "bad.txt", "../x"] {
        assert_fails_unchanged(&r, &mut ctx, "doc.open", json!({"path": path}), "rejected");
        assert_eq!(
            ok(&r, &mut ctx, "doc.list", json!({})),
            json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": false}]})
        );
    }
    // Dirty close without discard is refused; the document stays.
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "x"}),
    );
    assert_fails_unchanged(&r, &mut ctx, "doc.close", json!({"doc": 1}), "rejected");
    assert_fails_unchanged(
        &r,
        &mut ctx,
        "doc.close",
        json!({"doc": 1, "discard": false}),
        "rejected",
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.list", json!({})),
        json!({"docs": [{"doc": 1, "path": "a.txt", "dirty": true}]})
    );
    // Out-of-range and inverted ranges, unknown docs: rejected, unchanged.
    for (name, params) in [
        ("doc.insert", json!({"doc": 1, "offset": 8, "text": "x"})),
        (
            "doc.insert",
            json!({"doc": 1, "offset": u64::MAX, "text": "x"}),
        ),
        ("doc.delete", json!({"doc": 1, "start": 3, "end": 9})),
        ("doc.delete", json!({"doc": 1, "start": 3, "end": 2})),
        (
            "doc.replace",
            json!({"doc": 1, "start": 8, "end": 8, "text": "x"}),
        ),
        (
            "doc.replace",
            json!({"doc": 1, "start": 5, "end": 1, "text": "x"}),
        ),
        ("doc.insert", json!({"doc": 0, "offset": 0, "text": "x"})),
        ("doc.insert", json!({"doc": 2, "offset": 0, "text": "x"})),
        ("doc.text", json!({"doc": 99})),
        ("doc.info", json!({"doc": 2})),
        ("doc.undo", json!({"doc": 2})),
        ("doc.redo", json!({"doc": 2})),
        ("doc.save", json!({"doc": 2})),
        ("doc.close", json!({"doc": 2, "discard": true})),
    ] {
        assert_fails_unchanged(&r, &mut ctx, name, params, "rejected");
    }
    // Schema violations: invalid_params, unchanged.
    for (name, params) in [
        ("doc.insert", json!({"doc": 1, "offset": 0})),
        ("doc.insert", json!({"doc": "1", "offset": 0, "text": "x"})),
        ("doc.insert", json!({"doc": 1, "offset": -1, "text": "x"})),
        ("doc.insert", json!({"doc": 1, "offset": 0.5, "text": "x"})),
        ("doc.insert", json!({"doc": 1, "offset": 0, "text": 7})),
        (
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "x", "extra": null}),
        ),
        ("doc.delete", json!({"doc": 1, "start": 0})),
        ("doc.replace", json!({"doc": 1, "start": 0, "end": 1})),
        ("doc.text", json!({"doc": -1})),
        ("doc.text", json!({})),
        ("doc.text", json!(null)),
        ("doc.text", json!([1])),
        ("doc.info", json!({"doc": 1, "verbose": true})),
        ("doc.undo", json!({"doc": 1, "count": 2})),
        ("doc.redo", json!({"doc": "1"})),
        ("doc.save", json!({"doc": 1, "path": "b.txt"})),
        ("doc.close", json!({"doc": 1, "discard": "true"})),
        ("doc.close", json!({"doc": 1, "force": true})),
        ("doc.open", json!({})),
        ("doc.open", json!({"path": 1})),
        ("doc.open", json!({"path": "a.txt", "encoding": "utf-8"})),
        ("doc.list", json!({"doc": 1})),
    ] {
        assert_fails_unchanged(&r, &mut ctx, name, params, "invalid_params");
    }
    // Discarding close: closed, file unchanged.
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.close",
            json!({"doc": 1, "discard": true})
        ),
        json!({"doc": 1, "closed": true})
    );
    assert_eq!(file(&ctx, "a.txt"), b"hello\n");
    assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), json!({"docs": []}));
    // A clean document closes with discard:false too.
    assert_eq!(open_id(&r, &mut ctx, "dir/f.txt"), 2);
    assert_eq!(
        ok(
            &r,
            &mut ctx,
            "doc.close",
            json!({"doc": 2, "discard": false})
        ),
        json!({"doc": 2, "closed": true})
    );
}

#[test]
fn reopening_open_path_returns_existing_id() {
    let r = reg();
    let mut ctx = ctx_files(&[("a.txt", b"hello\n"), ("b.txt", b"b")]);
    assert_eq!(open_id(&r, &mut ctx, "a.txt"), 1);
    assert_eq!(open_id(&r, &mut ctx, "b.txt"), 2);
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 5, "text": "!!"}),
    );
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 0, "text": "\r\n"}),
    );
    ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
    let before = observe(&r, &mut ctx);
    let info_before = info(&r, &mut ctx, 1);
    assert_eq!(info_before["can_undo"], true);
    assert_eq!(info_before["can_redo"], true);
    let log_len = ctx.log().len();
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"})),
        json!({"doc": 1, "already_open": true, "path": "a.txt", "encoding": "utf-8",
               "line_ending": "lf", "mixed_line_endings": false, "len_chars": 8})
    );
    // Logged like any successful command.
    assert_eq!(ctx.log().len(), log_len + 1);
    let last = ctx.log().last().expect("logged").clone();
    assert_eq!(last.name, "doc.open");
    assert_eq!(last.params, json!({"path": "a.txt"}));
    assert!(last.mutating, "doc.open is a mutating command");
    // Changes nothing.
    assert_eq!(observe(&r, &mut ctx), before);
    assert_eq!(info(&r, &mut ctx, 1), info_before);
    assert_eq!(text_of(&r, &mut ctx, 1), "hello!!\n");
    // Redo still works (history kept), and the next new open gets the next id.
    assert_eq!(
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1})),
        json!({"redone": true, "len_chars": 10, "dirty": true})
    );
    // Re-open does not re-read the file and reports the current (CRLF-mixed) state.
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"})),
        json!({"doc": 1, "already_open": true, "path": "a.txt", "encoding": "utf-8",
               "line_ending": "lf", "mixed_line_endings": true, "len_chars": 10})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "b.txt"})),
        json!({"doc": 2, "already_open": true, "path": "b.txt", "encoding": "utf-8",
               "line_ending": "none", "mixed_line_endings": false, "len_chars": 1})
    );
    ok(&r, &mut ctx, "doc.close", json!({"doc": 2}));
    assert_eq!(open_id(&r, &mut ctx, "b.txt"), 3);
}

// ---------------------------------------------------------------------------------------------
// AC-7 (grouping through commands)

#[test]
fn typing_groups_across_insert_commands_and_save_breaks_group() {
    let r = reg();
    let mut ctx = ctx_files(&[("g.txt", b"xyz"), ("h.txt", b"xyz")]);
    assert_eq!(open_id(&r, &mut ctx, "g.txt"), 1);
    for (i, c) in ["a", "b", "c"].iter().enumerate() {
        ok(
            &r,
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": i, "text": c}),
        );
    }
    assert_eq!(text_of(&r, &mut ctx, 1), "abcxyz");
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 3, "dirty": false})
    );
    assert_eq!(text_of(&r, &mut ctx, 1), "xyz");
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": false, "len_chars": 3, "dirty": false})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1})),
        json!({"redone": true, "len_chars": 6, "dirty": true})
    );
    assert_eq!(text_of(&r, &mut ctx, 1), "abcxyz");

    // A save between "b" and "c" ends the group.
    assert_eq!(open_id(&r, &mut ctx, "h.txt"), 2);
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 0, "text": "a"}),
    );
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 1, "text": "b"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 2}));
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 2, "text": "c"}),
    );
    assert_eq!(text_of(&r, &mut ctx, 2), "abcxyz");
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 2})),
        json!({"undone": true, "len_chars": 5, "dirty": false})
    );
    assert_eq!(text_of(&r, &mut ctx, 2), "abxyz");
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 2})),
        json!({"undone": true, "len_chars": 3, "dirty": true})
    );
    assert_eq!(text_of(&r, &mut ctx, 2), "xyz");
    // Neither a query on another document nor a rejected insert on this one breaks doc 1's
    // typing group (a failed command changes nothing): "1","2","3" undo as one step.
    let mut model = Buffer::from_text("abcxyz");
    for (o, t) in [(6, "1"), (7, "2"), (99, "x"), (8, "3")] {
        let m = model.insert(o, t);
        let res = r.execute(
            &mut ctx,
            "doc.insert",
            json!({"doc": 1, "offset": o, "text": t}),
        );
        assert_eq!(m.is_ok(), res.is_ok(), "insert {o} {t}: {res:?}");
        ok(&r, &mut ctx, "doc.text", json!({"doc": 2}));
    }
    assert_eq!(text_of(&r, &mut ctx, 1), "abcxyz123");
    assert!(model.undo());
    assert!(!model.undo(), "the model groups 1,2,3 into one step");
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 6, "dirty": true})
    );
    assert_eq!(text_of(&r, &mut ctx, 1), model.text());
    // The earlier "abc" group is next.
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1})),
        json!({"undone": true, "len_chars": 3, "dirty": false})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}))["undone"],
        false
    );
}

// ---------------------------------------------------------------------------------------------
// AC-8 (log and replay, cmd.log-replay)

#[test]
fn log_lists_successful_commands_with_mutating_flag() {
    let r = reg();
    let flags = mutating_flags(&r);
    assert_eq!(flags.get("doc.insert"), Some(&true));
    assert_eq!(flags.get("doc.text"), Some(&false));
    let files = replay_files();
    for seed in [1u64, 2, 3, 0x7003] {
        let mut ctx = ctx_files(&files);
        assert!(ctx.log().is_empty(), "a fresh context has an empty log");
        let mut expected = Vec::new();
        let mut failures = 0;
        for (name, params, res) in run_random(&r, &mut ctx, seed, 300) {
            match res {
                Ok(_) => expected.push(LogEntry {
                    mutating: *flags
                        .get(&name)
                        .unwrap_or_else(|| panic!("{name} not described")),
                    name,
                    params,
                }),
                Err(_) => failures += 1,
            }
        }
        assert_eq!(ctx.log(), &expected[..], "seed {seed}");
        assert!(
            failures >= 30,
            "seed {seed}: only {failures} failing commands"
        );
        assert!(
            expected.len() >= 100,
            "seed {seed}: only {} successes",
            expected.len()
        );
    }
    // Hand-written sequence with exact entries.
    let mut ctx = ctx_files(&[("a.txt", b"a")]);
    ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    let _ = r.execute(&mut ctx, "doc.open", json!({"path": "nope.txt"}));
    let _ = r.execute(&mut ctx, "doc.insert", json!({"doc": 1, "offset": 0}));
    let _ = r.execute(&mut ctx, "no.such", json!({}));
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "b"}),
    );
    ok(&r, &mut ctx, "doc.text", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.list", json!({}));
    ok(
        &r,
        &mut ctx,
        "buffer.insert",
        json!({"offset": 0, "text": "s"}),
    );
    ok(&r, &mut ctx, "buffer.text", json!({}));
    ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.redo", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    ok(&r, &mut ctx, "doc.close", json!({"doc": 1}));
    let e = |name: &str, params: Value, mutating: bool| LogEntry {
        name: name.to_owned(),
        params,
        mutating,
    };
    assert_eq!(
        ctx.log(),
        &[
            e("doc.open", json!({"path": "a.txt"}), true),
            e(
                "doc.insert",
                json!({"doc": 1, "offset": 1, "text": "b"}),
                true
            ),
            e("doc.text", json!({"doc": 1}), false),
            e("doc.info", json!({"doc": 1}), false),
            e("doc.list", json!({}), false),
            e("buffer.insert", json!({"offset": 0, "text": "s"}), true),
            e("buffer.text", json!({}), false),
            e("doc.undo", json!({"doc": 1}), true),
            e("doc.redo", json!({"doc": 1}), true),
            e("doc.save", json!({"doc": 1}), true),
            e("doc.close", json!({"doc": 1}), true),
        ][..]
    );
    // LogEntry serializes with exactly its three fields and round-trips.
    let v = serde_json::to_value(&ctx.log()[1]).expect("serialize");
    assert_eq!(
        v,
        json!({"name": "doc.insert", "params": {"doc": 1, "offset": 1, "text": "b"}, "mutating": true})
    );
    let back: LogEntry = serde_json::from_value(v).expect("deserialize");
    assert_eq!(back, ctx.log()[1]);
}

#[test]
fn log_replay_reproduces_documents_and_files() {
    let r = reg();
    let files = replay_files();
    for seed in [11u64, 12, 13, 14, 15, 0xBEEF] {
        let mut a = ctx_files(&files);
        let results: Vec<Value> = run_random(&r, &mut a, seed, 250)
            .into_iter()
            .filter_map(|(_, _, res)| res.ok())
            .collect();
        let log = a.log().to_vec();
        assert_eq!(log.len(), results.len());
        assert!(
            log.iter().any(|e| e.name == "doc.save"),
            "seed {seed}: no save exercised"
        );
        let mut b = ctx_files(&files);
        let replayed = r
            .replay(&mut b, &log)
            .unwrap_or_else(|e| panic!("seed {seed}: replay failed: {e:?}"));
        assert_eq!(replayed, results, "seed {seed}: replay results");
        assert_eq!(b.log(), &log[..], "seed {seed}: replayed log");
        let sa = final_state(&r, &mut a, &files);
        let sb = final_state(&r, &mut b, &files);
        assert_eq!(sa, sb, "seed {seed}: state after replay");
        assert_eq!(
            a.log(),
            b.log(),
            "seed {seed}: logs after identical queries"
        );
    }
}

#[test]
fn replay_of_replayed_log_is_identical() {
    let r = reg();
    let files = replay_files();
    for seed in [21u64, 22, 23] {
        let mut a = ctx_files(&files);
        run_random(&r, &mut a, seed, 250);
        let mut b = ctx_files(&files);
        let rb = r.replay(&mut b, a.log()).expect("first replay");
        let mut c = ctx_files(&files);
        let rc = r.replay(&mut c, b.log()).expect("second replay");
        assert_eq!(rb, rc, "seed {seed}");
        assert_eq!(
            c.log(),
            b.log(),
            "seed {seed}: replaying a replayed log gives the same log"
        );
        assert_eq!(c.log(), a.log(), "seed {seed}");
        assert_eq!(
            final_state(&r, &mut b, &files),
            final_state(&r, &mut c, &files),
            "seed {seed}"
        );
    }
    // Replaying an empty log changes nothing and returns no results.
    let mut d = ctx_files(&files);
    assert_eq!(
        r.replay(&mut d, &[]).expect("empty replay"),
        Vec::<Value>::new()
    );
    assert!(d.log().is_empty());
}

#[test]
fn replay_stops_at_first_failing_entry_without_rollback() {
    let r = reg();
    let entry = |name: &str, params: Value, mutating: bool| LogEntry {
        name: name.to_owned(),
        params,
        mutating,
    };
    let log = vec![
        entry("doc.open", json!({"path": "a.txt"}), true),
        entry(
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "Z"}),
            true,
        ),
        entry("doc.open", json!({"path": "missing.txt"}), true),
        entry(
            "doc.insert",
            json!({"doc": 1, "offset": 0, "text": "Y"}),
            true,
        ),
    ];
    let mut ctx = ctx_files(&[("a.txt", b"a")]);
    let err: ReplayError = r.replay(&mut ctx, &log).expect_err("entry 2 fails");
    assert_eq!(err.index, 2, "{err:?}");
    assert!(
        matches!(err.error, CommandError::Rejected { .. }),
        "{err:?}"
    );
    assert!(!err.to_string().is_empty());
    let _: &dyn std::error::Error = &err;
    // Entries before the failure stay applied (D-020 ruling 3); the failing one is not logged.
    assert_eq!(ctx.log(), &log[..2]);
    assert_eq!(text_of(&r, &mut ctx, 1), "Za");

    // Unknown command and invalid params fail with their own codes at their index.
    let mut ctx = ctx_files(&[("a.txt", b"a")]);
    let err = r
        .replay(
            &mut ctx,
            &[
                entry("doc.list", json!({}), false),
                entry("doc.gone", json!({}), true),
            ],
        )
        .expect_err("unknown command");
    assert_eq!(err.index, 1);
    assert!(
        matches!(err.error, CommandError::UnknownCommand { .. }),
        "{err:?}"
    );
    let mut ctx = ctx_files(&[("a.txt", b"a")]);
    let err = r
        .replay(&mut ctx, &[entry("doc.text", json!({"doc": "1"}), false)])
        .expect_err("invalid params");
    assert_eq!(err.index, 0);
    assert!(
        matches!(err.error, CommandError::InvalidParams { .. }),
        "{err:?}"
    );
    assert!(ctx.log().is_empty());

    // Replay on a context whose files differ from F fails where the state diverges.
    let mut a = ctx_files(&[("a.txt", b"abc")]);
    ok(&r, &mut a, "doc.open", json!({"path": "a.txt"}));
    ok(
        &r,
        &mut a,
        "doc.delete",
        json!({"doc": 1, "start": 1, "end": 3}),
    );
    let mut b = ctx_files(&[("a.txt", b"a")]);
    let err = r
        .replay(&mut b, a.log())
        .expect_err("diverging initial state");
    assert_eq!(err.index, 1);
    assert!(
        matches!(err.error, CommandError::Rejected { .. }),
        "{err:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Constructors named in the Interface

#[test]
fn context_constructors_hold_the_given_files_and_text() {
    let r = reg();
    let ctx = ctx_files(&[("x/y.txt", b"\x00\xffraw"), ("z.txt", b"")]);
    assert_eq!(file(&ctx, "x/y.txt"), b"\x00\xffraw");
    assert_eq!(file(&ctx, "z.txt"), b"");
    assert!(ctx.vfs().read(&rp("nope")).is_err());
    assert!(ctx.log().is_empty());
    assert_eq!(ctx.buffer().text(), "");
    assert!(
        Context::with_files(&[("../bad", b"x")]).is_err(),
        "invalid path must be refused"
    );

    let mut ctx = Context::new();
    assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), json!({"docs": []}));
    assert_eq!(ctx.buffer().text(), "");
    let mut ctx = Context::with_text("scratch");
    assert_eq!(
        ok(&r, &mut ctx, "buffer.text", json!({})),
        json!({"text": "scratch", "len_chars": 7})
    );
    assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), json!({"docs": []}));

    let mut fs = kiln_workspace::MemoryFs::new();
    fs.insert(&rp("v.txt"), b"vfs\r\n");
    let mut ctx = Context::with_vfs(Box::new(fs));
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "v.txt"})),
        json!({"doc": 1, "already_open": false, "path": "v.txt", "encoding": "utf-8",
               "line_ending": "crlf", "mixed_line_endings": false, "len_chars": 5})
    );
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 3, "text": "!"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(file(&ctx, "v.txt"), b"vfs!\r\n");
    let _ = format!("{:?}", Context::default());
}
