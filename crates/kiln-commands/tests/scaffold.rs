//! T-015 implementer tests: every `Context` starts with external tools off (`NoTools`), and
//! `set_tools` replaces the runner without changing what commands do.

use kiln_commands::{Context, standard_registry};
use kiln_workspace::{MemoryFs, Tool, ToolError, ToolOutput, ToolRequest, ToolRunner};
use serde_json::json;

#[derive(Debug)]
struct Fixed;

impl ToolRunner for Fixed {
    fn run(&mut self, _request: &ToolRequest) -> Result<ToolOutput, ToolError> {
        Err(ToolError::NotFound { tool: Tool::Cargo })
    }
}

#[test]
fn every_constructor_starts_with_no_tools() {
    let contexts = vec![
        Context::new(),
        Context::default(),
        Context::with_text("abc"),
        Context::with_files(&[("a.rs", b"fn a() {}\n")]).expect("files"),
        Context::with_vfs(Box::new(MemoryFs::new())),
    ];
    for ctx in contexts {
        assert_eq!(format!("{:?}", ctx.tools()), "NoTools");
    }
}

#[test]
fn set_tools_replaces_the_runner_and_can_restore_no_tools() {
    let mut ctx = Context::with_text("x");
    ctx.set_tools(Box::new(Fixed));
    assert_eq!(format!("{:?}", ctx.tools()), "Fixed");
    ctx.set_tools(Box::new(kiln_workspace::NoTools));
    assert_eq!(format!("{:?}", ctx.tools()), "NoTools");
}

#[test]
fn installing_a_runner_does_not_change_commands() {
    let r = standard_registry().expect("registry");
    let run = |ctx: &mut Context| {
        [
            r.execute(ctx, "doc.open", json!({"path": "a.rs"})),
            r.execute(ctx, "doc.open", json!({"path": "missing"})),
            r.execute(ctx, "buffer.text", json!({})),
        ]
        .map(|res| res.map_err(|e| e.to_json()))
    };
    let files: &[(&str, &[u8])] = &[("a.rs", b"fn a() {}\n")];
    let mut plain = Context::with_files(files).expect("ctx");
    let mut with_runner = Context::with_files(files).expect("ctx");
    with_runner.set_tools(Box::new(Fixed));
    assert_eq!(run(&mut plain), run(&mut with_runner));
    assert_eq!(plain.log(), with_runner.log());
}
