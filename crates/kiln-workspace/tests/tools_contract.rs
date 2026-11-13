//! T-015 implementer tests: the external-tool contract (`Tool`, `ToolError`, `NoTools`) and
//! the crate-root re-exports.

use kiln_workspace::{
    DiskFs, Document, Encoding, MemoryFs, NoTools, OpenError, RelPath, Tool, ToolError, ToolOutput,
    ToolRequest, ToolRunner, Vfs, VfsError, decode, encode,
};

fn req(tool: Tool) -> ToolRequest {
    ToolRequest {
        tool,
        args: vec!["--version".into()],
        stdin: Some("x".into()),
    }
}

#[test]
fn tool_names_are_exact() {
    assert_eq!(Tool::Cargo.as_str(), "cargo");
    assert_eq!(Tool::Rustfmt.as_str(), "rustfmt");
    assert!(Tool::Cargo < Tool::Rustfmt);
}

#[test]
fn no_tools_refuses_every_request() {
    let mut runner = NoTools;
    for tool in [Tool::Cargo, Tool::Rustfmt] {
        assert_eq!(runner.run(&req(tool)), Err(ToolError::Unavailable));
        let empty = ToolRequest {
            tool,
            args: vec![],
            stdin: None,
        };
        assert_eq!(runner.run(&empty), Err(ToolError::Unavailable));
    }
    let mut boxed: Box<dyn ToolRunner> = Box::new(NoTools);
    assert_eq!(boxed.run(&req(Tool::Cargo)), Err(ToolError::Unavailable));
    assert_eq!(format!("{boxed:?}"), "NoTools");
    assert_eq!(NoTools::default(), NoTools);
}

#[test]
fn tool_error_texts_are_exact() {
    let cases = [
        (
            ToolError::Unavailable,
            "external tools are off for this session (start with --tools real to run them)",
        ),
        (
            ToolError::NotFound { tool: Tool::Cargo },
            "`cargo` was not found",
        ),
        (
            ToolError::Timeout {
                tool: Tool::Rustfmt,
                seconds: 30,
            },
            "`rustfmt` timed out after 30 s",
        ),
        (
            ToolError::Failed {
                tool: Tool::Cargo,
                message: "exit 101".into(),
            },
            "`cargo` failed: exit 101",
        ),
        (
            ToolError::NoRecording {
                tool: Tool::Rustfmt,
            },
            "no recorded output for this `rustfmt` request",
        ),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
        let dyn_err: &dyn std::error::Error = &err;
        assert!(dyn_err.source().is_none());
    }
}

#[test]
fn tool_output_is_plain_data() {
    let out = ToolOutput {
        status: None,
        stdout: "a".into(),
        stderr: "b".into(),
    };
    assert_eq!(out.clone(), out);
}

#[test]
fn reexports_resolve_at_the_crate_root_and_in_tools() {
    let _: fn(&MemoryFs) -> &dyn Vfs = |fs| fs;
    let _: Option<VfsError> = None;
    let _: Option<OpenError> = None;
    let _: Option<Document> = None;
    let _: Option<DiskFs> = None;
    let _: Option<Encoding> = None;
    let _ = (decode, encode);
    assert!(RelPath::new("a/b.rs").is_ok());
    assert_eq!(kiln_workspace::tools::NoTools, NoTools);
    let _: Option<kiln_workspace::tools::ToolRequest> = Some(req(Tool::Cargo));
}
