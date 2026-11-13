//! T-032 AC-1 (and the `internal` half of the reason contract) through JSON only: every refusal
//! raised by `standard_registry().execute` serializes with exactly its pinned `"reason"` beside
//! the unchanged `code` and `message` (`to_json()["reason"] == as_str()`, Pinned (a)). Written
//! from the task specification alone by the test author.
//!
//! This file uses only the base API (`to_json`, `Command`, `Registry`), so it compiles on base
//! and fails there by assertion: base `to_json` has no `"reason"`. The typed checks of the same
//! criteria are in `acceptance_t032.rs`.
#![cfg(test)]

use kiln_commands::{Command, CommandError, Context, Example, Registry, standard_registry};
use kiln_workspace::{MemoryFs, RelPath};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Value, json};

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
    r.execute(ctx, name, params.clone())
        .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
}

fn files_ctx(files: &[(&str, &[u8])], fail_writes: bool) -> Context {
    let mut fs = MemoryFs::new();
    for (p, b) in files {
        fs.insert(&RelPath::new(p).expect("valid fixture path"), b);
    }
    fs.set_fail_writes(fail_writes);
    Context::with_vfs(Box::new(fs))
}

/// One refusal: (what, files, fail writes, scratch text, setup, command, params, reason, message).
type Case = (
    &'static str,
    &'static [(&'static str, &'static [u8])],
    bool,
    &'static str,
    Vec<(&'static str, Value)>,
    &'static str,
    Value,
    &'static str,
    &'static str,
);

const ABC: &[(&str, &[u8])] = &[("a.txt", b"abc")];

/// The AC-1 list, then further refusal sites and hostile names whose messages look like another
/// kind's message (the reason must come from where the refusal is raised, not from text).
fn cases() -> Vec<Case> {
    let open_a = || ("doc.open", json!({"path": "a.txt"}));
    let dirty = || {
        vec![
            open_a(),
            ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
        ]
    };
    vec![
        // AC-1, in its order.
        (
            "doc.open ../x",
            &[],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "../x"}),
            "invalid_path",
            "invalid relative path \"../x\": path has a '..' segment",
        ),
        (
            "doc.text 9",
            &[],
            false,
            "",
            vec![],
            "doc.text",
            json!({"doc": 9}),
            "no_such_document",
            "no open document 9",
        ),
        (
            "dirty doc.close",
            ABC,
            false,
            "",
            dirty(),
            "doc.close",
            json!({"doc": 1}),
            "unsaved_changes",
            "document 1 has unsaved changes; save it or close with discard",
        ),
        (
            "doc.open missing",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "missing.txt"}),
            "not_found",
            "cannot open missing.txt: not found",
        ),
        (
            "doc.open directory",
            &[("d/x", b"x")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "d"}),
            "not_a_file",
            "cannot open d: is a directory, not a file",
        ),
        (
            "doc.open \\xff",
            &[("bad.bin", b"\xff")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "bad.bin"}),
            "decode_error",
            "cannot open bad.bin: invalid utf-8 at byte offset 0",
        ),
        (
            "doc.insert at 99",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.insert",
            json!({"doc": 1, "offset": 99, "text": "x"}),
            "out_of_range",
            "char index 99 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.delete 2..1",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.delete",
            json!({"doc": 1, "start": 2, "end": 1}),
            "inverted_range",
            "range start 2 is after range end 1",
        ),
        (
            "buffer.insert at 99",
            &[],
            false,
            "abc",
            vec![],
            "buffer.insert",
            json!({"offset": 99, "text": "!"}),
            "out_of_range",
            "char index 99 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.save, writes fail",
            ABC,
            true,
            "",
            dirty(),
            "doc.save",
            json!({"doc": 1}),
            "io_error",
            "cannot save a.txt: I/O error: simulated write failure",
        ),
        // Further sites of the same reasons.
        (
            "doc.open /abs.txt",
            &[],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "/abs.txt"}),
            "invalid_path",
            "invalid relative path \"/abs.txt\": path starts with '/'",
        ),
        (
            "doc.save 4",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.save",
            json!({"doc": 4}),
            "no_such_document",
            "no open document 4",
        ),
        (
            "doc.close 2",
            &[],
            false,
            "",
            vec![],
            "doc.close",
            json!({"doc": 2}),
            "no_such_document",
            "no open document 2",
        ),
        (
            "doc.info 3",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.info",
            json!({"doc": 3}),
            "no_such_document",
            "no open document 3",
        ),
        (
            "doc.undo 5",
            &[],
            false,
            "",
            vec![],
            "doc.undo",
            json!({"doc": 5}),
            "no_such_document",
            "no open document 5",
        ),
        (
            "doc.insert 9 at 99 (document checked first)",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.insert",
            json!({"doc": 9, "offset": 99, "text": "x"}),
            "no_such_document",
            "no open document 9",
        ),
        (
            "doc.replace 1..9",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.replace",
            json!({"doc": 1, "start": 1, "end": 9, "text": "q"}),
            "out_of_range",
            "char index 9 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.replace 3..1",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.replace",
            json!({"doc": 1, "start": 3, "end": 1, "text": "q"}),
            "inverted_range",
            "range start 3 is after range end 1",
        ),
        // Hostile names: each message reads like another kind's.
        (
            "missing file named like a refusal",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "no open document 1"}),
            "not_found",
            "cannot open no open document 1: not found",
        ),
        (
            "missing file whose name ends like a directory refusal",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "x: is a directory, not a file"}),
            "not_found",
            "cannot open x: is a directory, not a file: not found",
        ),
        (
            "directory named like an I/O error",
            &[("e: I/O error: z/f", b"x")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "e: I/O error: z"}),
            "not_a_file",
            "cannot open e: I/O error: z: is a directory, not a file",
        ),
        (
            "undecodable file named like a missing one",
            &[("m: not found", b"ok\xff")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "m: not found"}),
            "decode_error",
            "cannot open m: not found: invalid utf-8 at byte offset 2",
        ),
    ]
}

fn run_case(r: &Registry, case: &Case) -> CommandError {
    let (what, files, fail, text, setup, name, params, _, _) = case;
    let mut ctx = files_ctx(files, *fail);
    if !text.is_empty() {
        ok(
            r,
            &mut ctx,
            "buffer.insert",
            json!({"offset": 0, "text": text}),
        );
    }
    for (n, p) in setup {
        ok(r, &mut ctx, n, p.clone());
    }
    r.execute(&mut ctx, name, params.clone()).expect_err(what)
}

/// AC-1: `to_json()` of each refusal is exactly `{"code":"rejected","message":…,"reason":…}`.
#[test]
fn refusals_serialize_their_reason() {
    let r = reg();
    for case in cases() {
        let (what, .., reason, message) = case;
        let err = run_case(&r, &case);
        assert_eq!(
            err.to_json(),
            json!({"code": "rejected", "message": message, "reason": reason}),
            "{what}"
        );
        assert_eq!(err.to_json()["reason"], reason, "{what}");
        assert_eq!(err.to_string(), message, "{what}: Display");
    }
}

// --------------------------------------------------------------------------------------------
// Internal errors raised by the registry itself.

/// No params.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NoParams {}

/// Declared as an integer, serialized as a string: violates its own result schema.
#[derive(Debug, JsonSchema)]
struct Lying(u64);

impl Serialize for Lying {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("not a number: {}", self.0))
    }
}

/// Refuses to serialize.
#[derive(Debug, JsonSchema)]
struct Unserializable(u64);

impl Serialize for Unserializable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(format!("boom {}", self.0)))
    }
}

fn example() -> Vec<Example> {
    vec![Example {
        description: "never run",
        initial_text: "",
        files: vec![],
        setup: vec![],
        params: json!({}),
        result: json!(0),
    }]
}

struct SchemaViolation;

impl Command for SchemaViolation {
    const NAME: &'static str = "t.schema";
    const DOC: &'static str = "Returns a result that violates its schema.";
    const MUTATING: bool = false;
    type Params = NoParams;
    type Output = Lying;
    fn examples() -> Vec<Example> {
        example()
    }
    fn run(_: &mut Context, _: NoParams) -> Result<Lying, CommandError> {
        Ok(Lying(7))
    }
}

struct NoSerialize;

impl Command for NoSerialize {
    const NAME: &'static str = "t.noser";
    const DOC: &'static str = "Returns a result that does not serialize.";
    const MUTATING: bool = false;
    type Params = NoParams;
    type Output = Unserializable;
    fn examples() -> Vec<Example> {
        example()
    }
    fn run(_: &mut Context, _: NoParams) -> Result<Unserializable, CommandError> {
        Ok(Unserializable(7))
    }
}

fn custom_registry() -> Registry {
    let mut r = Registry::new();
    r.register::<SchemaViolation>().expect("register t.schema");
    r.register::<NoSerialize>().expect("register t.noser");
    r
}

/// A result-schema violation is `internal` with reason `result_schema`; a result that does not
/// serialize is `internal` with reason `other`. Messages unchanged.
#[test]
fn internal_errors_serialize_their_reason() {
    let r = custom_registry();
    let mut ctx = Context::new();
    let schema = r
        .execute(&mut ctx, "t.schema", json!({}))
        .expect_err("schema violation");
    let message = schema.to_string();
    assert!(
        message.starts_with("result of `t.schema` violates its schema: "),
        "{message}"
    );
    assert_eq!(
        schema.to_json(),
        json!({"code": "internal", "message": message, "reason": "result_schema"})
    );
    let noser = r
        .execute(&mut ctx, "t.noser", json!({}))
        .expect_err("does not serialize");
    assert_eq!(
        noser.to_json(),
        json!({
            "code": "internal",
            "message": "result of `t.noser` does not serialize: boom 7",
            "reason": "other",
        })
    );
    assert!(ctx.log().is_empty(), "a failed command was logged");
}

/// A save whose directory was removed on disk after the open is `not_found` (the save site maps
/// the VFS error, it does not assume `io_error`).
#[test]
fn save_into_a_removed_directory_is_not_found() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t032_json")
        .join("removed-dir");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("sub")).expect("mkdir");
    std::fs::write(root.join("sub/a.txt"), b"abc").expect("write");
    let fs = kiln_workspace::DiskFs::new(&root).expect("disk fs");
    let mut ctx = Context::with_vfs(Box::new(fs));
    let r = reg();
    ok(&r, &mut ctx, "doc.open", json!({"path": "sub/a.txt"}));
    std::fs::remove_dir_all(root.join("sub")).expect("remove sub");
    assert!(
        !root.join("sub").exists(),
        "precondition: directory removed"
    );
    let err = r
        .execute(&mut ctx, "doc.save", json!({"doc": 1}))
        .expect_err("save into a removed directory");
    assert_eq!(
        err.to_json(),
        json!({"code": "rejected", "message": "cannot save sub/a.txt: not found", "reason": "not_found"})
    );
    assert!(!root.join("sub").exists(), "the save created the directory");
    let _ = std::fs::remove_dir_all(&root);
}
