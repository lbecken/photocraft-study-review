//! T-032 regression guards (D-010, D-021), Rust API of `kiln-commands`: structured reasons are
//! added *beside* every message, so the message, the `code`, the `Display` text and the order of
//! checks of every refusal stay exactly as on base (Pinned (a), (c)). Every expected message
//! below was recorded on base (`main` with T-015 merged); these tests pass there by design.
//! Written from the task specification alone by the test author.
//!
//! Patterns match `CommandError::Rejected { message, .. }`, so this file compiles both on base
//! and once `Rejected` gains its `reason` field.
#![cfg(test)]

use kiln_commands::{CommandError, Context, Registry, standard_registry};
use kiln_workspace::{MemoryFs, RelPath};
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

/// (what, files, fail writes, scratch text, setup, command, params, base message).
type Case = (
    &'static str,
    &'static [(&'static str, &'static [u8])],
    bool,
    &'static str,
    Vec<(&'static str, Value)>,
    &'static str,
    Value,
    &'static str,
);

const ABC: &[(&str, &[u8])] = &[("a.txt", b"abc")];

/// Stands for the message of a refusal that opens a directory: not pinned, because T-008 rewords
/// it (ruling r6, D-052); its error class, code and reason are still checked.
const UNPINNED: &str = "<directory refusal: message not pinned (D-052)>";

fn cases() -> Vec<Case> {
    let open_a = || ("doc.open", json!({"path": "a.txt"}));
    vec![
        (
            "invalid path",
            &[],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "../x"}),
            "invalid relative path \"../x\": path has a '..' segment",
        ),
        (
            "absolute path",
            &[],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "/abs.txt"}),
            "invalid relative path \"/abs.txt\": path starts with '/'",
        ),
        (
            "no such document (doc.text)",
            &[],
            false,
            "",
            vec![],
            "doc.text",
            json!({"doc": 9}),
            "no open document 9",
        ),
        (
            "no such document (doc.save)",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.save",
            json!({"doc": 4}),
            "no open document 4",
        ),
        (
            "no such document (doc.close)",
            &[],
            false,
            "",
            vec![],
            "doc.close",
            json!({"doc": 2}),
            "no open document 2",
        ),
        (
            "no such document before the offset check (precedence)",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.insert",
            json!({"doc": 9, "offset": 99, "text": "x"}),
            "no open document 9",
        ),
        (
            "dirty close",
            ABC,
            false,
            "",
            vec![
                open_a(),
                ("doc.insert", json!({"doc": 1, "offset": 0, "text": "x"})),
            ],
            "doc.close",
            json!({"doc": 1}),
            "document 1 has unsaved changes; save it or close with discard",
        ),
        (
            "missing file",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "missing.txt"}),
            "cannot open missing.txt: not found",
        ),
        (
            "directory",
            &[("d/x", b"x")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "d"}),
            UNPINNED,
        ),
        (
            "undecodable bytes",
            &[("bad.bin", b"\xff")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "bad.bin"}),
            "cannot open bad.bin: invalid utf-8 at byte offset 0",
        ),
        (
            "doc.insert out of range",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.insert",
            json!({"doc": 1, "offset": 99, "text": "x"}),
            "char index 99 is out of bounds (buffer has 3 chars)",
        ),
        (
            "doc.delete inverted",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.delete",
            json!({"doc": 1, "start": 2, "end": 1}),
            "range start 2 is after range end 1",
        ),
        (
            "doc.replace out of range",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.replace",
            json!({"doc": 1, "start": 1, "end": 9, "text": "q"}),
            "char index 9 is out of bounds (buffer has 3 chars)",
        ),
        (
            "buffer.insert out of range",
            &[],
            false,
            "abc",
            vec![],
            "buffer.insert",
            json!({"offset": 99, "text": "!"}),
            "char index 99 is out of bounds (buffer has 3 chars)",
        ),
        (
            "missing file named like a refusal",
            ABC,
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "no open document 1"}),
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
            UNPINNED,
        ),
        (
            "undecodable file named like a missing one",
            &[("m: not found", b"ok\xff")],
            false,
            "",
            vec![],
            "doc.open",
            json!({"path": "m: not found"}),
            "cannot open m: not found: invalid utf-8 at byte offset 2",
        ),
        (
            "doc.replace inverted",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.replace",
            json!({"doc": 1, "start": 3, "end": 1, "text": "q"}),
            "range start 3 is after range end 1",
        ),
        (
            "no such document (doc.info)",
            ABC,
            false,
            "",
            vec![open_a()],
            "doc.info",
            json!({"doc": 3}),
            "no open document 3",
        ),
        (
            "no such document (doc.undo)",
            &[],
            false,
            "",
            vec![],
            "doc.undo",
            json!({"doc": 5}),
            "no open document 5",
        ),
        (
            "failed write",
            ABC,
            true,
            "",
            vec![
                open_a(),
                ("doc.insert", json!({"doc": 1, "offset": 3, "text": "d"})),
            ],
            "doc.save",
            json!({"doc": 1}),
            "cannot save a.txt: I/O error: simulated write failure",
        ),
    ]
}

/// Every refusal keeps its base message, `code`, `Display` text and log, and leaves the
/// scratch buffer, the open documents and the files as they were (the order of checks is pinned
/// by the precedence case).
#[test]
fn refusal_messages_and_codes_are_unchanged() {
    let r = reg();
    for (what, files, fail, text, setup, name, params, message) in cases() {
        let mut ctx = files_ctx(files, fail);
        if !text.is_empty() {
            ok(
                &r,
                &mut ctx,
                "buffer.insert",
                json!({"offset": 0, "text": text}),
            );
        }
        for (n, p) in setup {
            ok(&r, &mut ctx, n, p);
        }
        let list = ok(&r, &mut ctx, "doc.list", json!({}));
        let buffer = ok(&r, &mut ctx, "buffer.text", json!({}));
        let read_all = |ctx: &Context| -> Vec<Option<Vec<u8>>> {
            files
                .iter()
                .map(|(p, _)| ctx.vfs().read(&RelPath::new(p).expect("path")).ok())
                .collect()
        };
        let bytes = read_all(&ctx);
        let log = ctx.log().to_vec();
        let err = r.execute(&mut ctx, name, params.clone()).expect_err(what);
        match &err {
            CommandError::Rejected { message: m, .. } => {
                if message != UNPINNED {
                    assert_eq!(m, message, "{what}");
                }
            }
            other => panic!("{what}: expected a refusal, got {other:?}"),
        }
        assert_eq!(err.to_json()["code"], "rejected", "{what}");
        if message != UNPINNED {
            assert_eq!(err.to_string(), message, "{what}: Display");
            assert_eq!(err.to_json()["message"], message, "{what}");
        }
        assert_eq!(ctx.log(), log.as_slice(), "{what}: the refusal was logged");
        assert_eq!(read_all(&ctx), bytes, "{what}: files changed");
        assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), list, "{what}");
        assert_eq!(ok(&r, &mut ctx, "buffer.text", json!({})), buffer, "{what}");
    }
}

/// `to_json` of the two codes that gain no reason is unchanged: exactly the base keys.
#[test]
fn request_level_errors_keep_their_json() {
    let r = reg();
    let mut ctx = Context::new();
    let unknown = r
        .execute(&mut ctx, "zz.no-such-command", json!({}))
        .expect_err("unknown command");
    assert_eq!(
        unknown.to_json(),
        json!({"code": "unknown_command", "message": "unknown command `zz.no-such-command`"})
    );
    let invalid = r
        .execute(&mut ctx, "doc.insert", json!({"doc": 1}))
        .expect_err("invalid params");
    assert_eq!(
        invalid.to_json(),
        json!({
            "code": "invalid_params",
            "message": "invalid params for `doc.insert`",
            "details": ["\"offset\" is a required property", "\"text\" is a required property"],
        })
    );
    assert_eq!(
        invalid.to_string(),
        "invalid params for `doc.insert`: \"offset\" is a required property; \
         \"text\" is a required property"
    );
}

/// A save whose directory was removed on disk keeps its base message.
#[test]
fn save_into_a_removed_directory_keeps_its_message() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t032")
        .join("removed-dir");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("sub")).expect("mkdir");
    std::fs::write(root.join("sub/a.txt"), b"abc").expect("write");
    let fs = kiln_workspace::DiskFs::new(&root).expect("disk fs");
    let mut ctx = Context::with_vfs(Box::new(fs));
    let r = reg();
    ok(&r, &mut ctx, "doc.open", json!({"path": "sub/a.txt"}));
    std::fs::remove_dir_all(root.join("sub")).expect("remove sub");
    match r.execute(&mut ctx, "doc.save", json!({"doc": 1})) {
        Err(CommandError::Rejected { message, .. }) => {
            assert_eq!(message, "cannot save sub/a.txt: not found");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(!root.join("sub").exists(), "the save created the directory");
    let _ = std::fs::remove_dir_all(&root);
}
