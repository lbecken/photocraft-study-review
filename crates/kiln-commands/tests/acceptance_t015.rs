//! T-015 AC-2 acceptance tests (no process by default, Rust API), written from the task
//! specification alone by the test author: every `Context` constructor installs `NoTools`,
//! `set_tools` replaces the runner and nothing else, and no command swaps the runner. On the
//! base these fail to compile (`Context::tools`/`set_tools` do not exist yet): weak evidence by
//! design (task file, "AC-2 to AC-4 fail on base only at compile time").
#![cfg(test)]

use std::path::Path;

use kiln_commands::{Context, Registry, standard_registry};
use kiln_workspace::{
    DiskFs, MemoryFs, NoTools, RelPath, ToolError, ToolOutput, ToolRequest, ToolRunner,
};
use serde_json::{Value, json};

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
    r.execute(ctx, name, params.clone())
        .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
}

fn tools_of(ctx: &Context) -> String {
    format!("{:?}", ctx.tools())
}

/// A fresh, empty directory unique to `name` below the integration-test temp dir.
fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t015")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A test runner whose `Debug` names it and its label.
#[derive(Debug)]
struct Fake {
    label: &'static str,
}

impl ToolRunner for Fake {
    fn run(&mut self, _request: &ToolRequest) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput {
            status: Some(0),
            stdout: self.label.to_owned(),
            stderr: String::new(),
        })
    }
}

/// Every command a session could send today, on `ctx` holding `a.txt`.
fn exercise(r: &Registry, ctx: &mut Context) {
    ok(r, ctx, "buffer.insert", json!({"offset": 0, "text": "x"}));
    ok(r, ctx, "buffer.text", json!({}));
    let doc = ok(r, ctx, "doc.open", json!({"path": "a.txt"}))["doc"].clone();
    ok(
        r,
        ctx,
        "doc.insert",
        json!({"doc": doc, "offset": 0, "text": "> "}),
    );
    ok(
        r,
        ctx,
        "doc.delete",
        json!({"doc": doc, "start": 0, "end": 1}),
    );
    ok(
        r,
        ctx,
        "doc.replace",
        json!({"doc": doc, "start": 0, "end": 1, "text": "<"}),
    );
    ok(r, ctx, "doc.undo", json!({"doc": doc}));
    ok(r, ctx, "doc.redo", json!({"doc": doc}));
    ok(r, ctx, "doc.info", json!({"doc": doc}));
    ok(r, ctx, "doc.text", json!({"doc": doc}));
    ok(r, ctx, "doc.list", json!({}));
    ok(r, ctx, "doc.save", json!({"doc": doc}));
    ok(r, ctx, "doc.close", json!({"doc": doc}));
    // Refused requests too.
    assert!(r.execute(ctx, "doc.text", json!({"doc": 99})).is_err());
    assert!(r.execute(ctx, "doc.open", json!({"path": "../x"})).is_err());
    assert!(r.execute(ctx, "no.such", json!({})).is_err());
}

/// AC-2: `format!("{:?}", ctx.tools()) == "NoTools"` for every constructor, and it stays so
/// after every existing command has run.
#[test]
fn contexts_start_without_tools() {
    let disk_root = fresh_dir("disk");
    std::fs::write(disk_root.join("a.txt"), b"hello\n").unwrap();
    let mut memory_with_a = MemoryFs::new();
    memory_with_a.insert(&RelPath::new("a.txt").unwrap(), b"hello\n");

    let contexts: Vec<(&str, Context)> = vec![
        ("new", Context::new()),
        ("default", Context::default()),
        ("with_text empty", Context::with_text("")),
        ("with_text", Context::with_text("abc\r\né😀")),
        ("with_files empty", Context::with_files(&[]).unwrap()),
        (
            "with_files",
            Context::with_files(&[("a.txt", b"hello\n"), ("sub/b.rs", b"fn b() {}\n")]).unwrap(),
        ),
        (
            "with_vfs memory",
            Context::with_vfs(Box::new(MemoryFs::new())),
        ),
        (
            "with_vfs memory a.txt",
            Context::with_vfs(Box::new(memory_with_a)),
        ),
        (
            "with_vfs disk",
            Context::with_vfs(Box::new(DiskFs::new(&disk_root).unwrap())),
        ),
    ];
    let r = reg();
    for (what, mut ctx) in contexts {
        assert_eq!(tools_of(&ctx), "NoTools", "{what}");
        // `tools()` is read-only, so the runner's behaviour cannot be driven from here; its
        // `Debug` is the observable (the task's oracle).
        let runner: &dyn ToolRunner = ctx.tools();
        assert_eq!(format!("{runner:?}"), format!("{NoTools:?}"), "{what}");
        if ctx.vfs().read(&RelPath::new("a.txt").unwrap()).is_ok() {
            exercise(&r, &mut ctx);
            assert!(!ctx.log().is_empty(), "{what}: commands ran");
        } else {
            ok(&r, &mut ctx, "buffer.text", json!({}));
            ok(&r, &mut ctx, "doc.list", json!({}));
        }
        assert_eq!(tools_of(&ctx), "NoTools", "{what} after commands");
    }
    let _ = std::fs::remove_dir_all(&disk_root);
}

/// AC-2: after `set_tools(Box::new(R))`, `tools()` is `R`'s `Debug`; a later `set_tools`
/// replaces it again (also back to `NoTools`); and replacing the runner changes nothing else
/// in the session (buffer, documents, files, log, document ids).
#[test]
fn set_tools_replaces_the_runner() {
    let r = reg();
    let mut ctx = Context::with_files(&[("a.txt", b"hello\n"), ("b.txt", b"b\n")]).unwrap();
    ok(
        &r,
        &mut ctx,
        "buffer.insert",
        json!({"offset": 0, "text": "scratch"}),
    );
    let a = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(a["doc"], json!(1));
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 5, "text": "!"}),
    );
    let before_list = ok(&r, &mut ctx, "doc.list", json!({}));
    let before_log = ctx.log().to_vec();
    let before_buffer = ctx.buffer().text();

    ctx.set_tools(Box::new(Fake { label: "one" }));
    assert_eq!(tools_of(&ctx), r#"Fake { label: "one" }"#);
    assert_eq!(
        ctx.log(),
        before_log.as_slice(),
        "set_tools is not a command"
    );
    assert_eq!(ctx.buffer().text(), before_buffer);
    assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), before_list);
    assert_eq!(
        ok(&r, &mut ctx, "doc.text", json!({"doc": 1}))["text"],
        json!("hello!\n")
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.info", json!({"doc": 1}))["dirty"],
        json!(true)
    );
    // Document ids continue where they were: the next one is 2.
    assert_eq!(
        ok(&r, &mut ctx, "doc.open", json!({"path": "b.txt"}))["doc"],
        json!(2)
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 1}));
    assert_eq!(
        ctx.vfs().read(&RelPath::new("a.txt").unwrap()).unwrap(),
        b"hello!\n"
    );
    exercise(&r, &mut ctx);
    assert_eq!(
        tools_of(&ctx),
        r#"Fake { label: "one" }"#,
        "commands keep the runner"
    );

    ctx.set_tools(Box::new(Fake { label: "two" }));
    assert_eq!(tools_of(&ctx), r#"Fake { label: "two" }"#);
    ctx.set_tools(Box::new(NoTools));
    assert_eq!(tools_of(&ctx), "NoTools");

    // Every constructor accepts a replacement.
    for mut ctx in [
        Context::new(),
        Context::default(),
        Context::with_text("t"),
        Context::with_files(&[]).unwrap(),
        Context::with_vfs(Box::new(MemoryFs::new())),
    ] {
        ctx.set_tools(Box::new(Fake { label: "c" }));
        assert_eq!(tools_of(&ctx), r#"Fake { label: "c" }"#);
    }

    // Contexts do not share a runner.
    let mut first = Context::new();
    let second = Context::new();
    first.set_tools(Box::new(Fake {
        label: "only first",
    }));
    assert_eq!(tools_of(&first), r#"Fake { label: "only first" }"#);
    assert_eq!(tools_of(&second), "NoTools");
}
