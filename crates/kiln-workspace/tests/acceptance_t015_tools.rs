//! T-015 acceptance tests for the external-tool contract and the `kiln-workspace` re-exports
//! (AC-2 `NoTools`, AC-3 `ToolError`/`Tool`, AC-4 re-exports), written from the task
//! specification alone by the test author. On the base these fail to compile (the types do
//! not exist yet): weak evidence by design (task file, "AC-2 to AC-4 fail on base only at
//! compile time").
#![cfg(test)]

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use kiln_workspace::{
    DiskFs, Document, Encoding, MemoryFs, NoTools, OpenError, RelPath, Tool, ToolError, ToolOutput,
    ToolRequest, ToolRunner, Vfs, VfsError, decode, encode,
};

/// Every process a test spawns is killed after this long (D-028 (8)).
const TIMEOUT: Duration = Duration::from_secs(60);

const BOTH: [Tool; 2] = [Tool::Cargo, Tool::Rustfmt];

fn requests() -> Vec<ToolRequest> {
    let mut out = Vec::new();
    for tool in BOTH {
        out.push(ToolRequest {
            tool,
            args: Vec::new(),
            stdin: None,
        });
        out.push(ToolRequest {
            tool,
            args: vec!["--version".to_owned()],
            stdin: None,
        });
        out.push(ToolRequest {
            tool,
            args: vec![
                "check".to_owned(),
                "--message-format=json".to_owned(),
                "--quiet".to_owned(),
            ],
            stdin: Some("fn main() {}\n".to_owned()),
        });
        out.push(ToolRequest {
            tool,
            args: vec!["--emit".to_owned(), "stdout".to_owned()],
            stdin: Some(String::new()),
        });
    }
    out
}

/// A fresh, empty directory unique to `name` below the integration-test temp dir.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_t015_tools")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wait_with_timeout(mut cmd: Command) -> std::process::Output {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child did not finish within {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The child half of [`no_tools_never_runs`]: run every request through `NoTools` (by value,
/// through `&mut dyn ToolRunner` and through a `Box`), with `PATH` pointing at fake `cargo` and
/// `rustfmt` that leave a marker file when started.
fn child_runs_no_tools() {
    for req in requests() {
        let mut direct = NoTools;
        assert_eq!(direct.run(&req), Err(ToolError::Unavailable), "{req:?}");
        let as_dyn: &mut dyn ToolRunner = &mut direct;
        assert_eq!(as_dyn.run(&req), Err(ToolError::Unavailable), "{req:?}");
        let mut boxed: Box<dyn ToolRunner> = Box::new(NoTools);
        assert_eq!(boxed.run(&req), Err(ToolError::Unavailable), "{req:?}");
        // Repeated calls on the same runner keep refusing.
        for _ in 0..3 {
            assert_eq!(boxed.run(&req), Err(ToolError::Unavailable), "{req:?}");
        }
    }
}

const CHILD_ENV: &str = "KILN_T015_NO_TOOLS_CHILD";
const MARKER_ENV: &str = "KILN_T015_MARKER";

/// AC-2: `NoTools.run(&req)` is `Err(ToolError::Unavailable)` for requests of both tools, and
/// it starts no process: with fake `cargo`/`rustfmt` first on `PATH` (shown to work first),
/// running every request through `NoTools` leaves no marker.
#[test]
fn no_tools_never_runs() {
    // Exact values; in the child copy (see below) this is the whole test.
    child_runs_no_tools();
    if std::env::var_os(CHILD_ENV).is_none() {
        parent_checks();
    }
}

/// The parent half of [`no_tools_never_runs`]: value semantics of `NoTools`, then the fake
/// tools on `PATH` and the child copy of the test that runs `NoTools` with them.
fn parent_checks() {
    fn made_by_default<T: Default>() -> T {
        T::default()
    }
    assert_eq!(made_by_default::<NoTools>(), NoTools);
    assert_eq!(format!("{NoTools:?}"), "NoTools");
    let copy = NoTools;
    let _also = copy; // Copy
    assert_eq!(copy.clone(), NoTools);

    #[cfg(not(unix))]
    panic!("precondition: the fake `cargo`/`rustfmt` of this test are shell scripts and need Unix");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = fresh_dir("fake-path");
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let marker = dir.join("marker");
        for tool in BOTH {
            let fake = bin.join(tool.as_str());
            std::fs::write(
                &fake,
                "#!/bin/sh\necho \"$0 $*\" >> \"$KILN_T015_MARKER\"\n",
            )
            .unwrap();
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = match std::env::var_os("PATH") {
            Some(old) => {
                let mut p = std::ffi::OsString::from(&bin);
                p.push(":");
                p.push(old);
                p
            }
            None => bin.clone().into_os_string(),
        };
        // The fakes are found on that PATH and leave the marker when started.
        for tool in BOTH {
            let mut cmd = Command::new(tool.as_str());
            cmd.env("PATH", &path).env(MARKER_ENV, &marker);
            let out = wait_with_timeout(cmd);
            assert!(out.status.success(), "fake {tool:?}: {out:?}");
        }
        let seen = std::fs::read_to_string(&marker).unwrap();
        assert_eq!(seen.lines().count(), 2, "{seen}");
        std::fs::remove_file(&marker).unwrap();

        // The same environment for a child copy of this test, which runs NoTools.
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args(["--exact", "no_tools_never_runs", "--test-threads=1"])
            .env("PATH", &path)
            .env(MARKER_ENV, &marker)
            .env(CHILD_ENV, "1");
        let out = wait_with_timeout(cmd);
        assert!(out.status.success(), "child: {out:?}");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("1 passed"),
            "the child ran the test: {out:?}"
        );
        assert!(
            !marker.exists(),
            "NoTools started a process: {:?}",
            std::fs::read_to_string(&marker)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// AC-3: every `Display` is the pinned text, `Tool::as_str` is exact, and `ToolError` is a
/// `std::error::Error` (without a source).
#[test]
fn tool_error_texts_are_exact() {
    assert_eq!(Tool::Cargo.as_str(), "cargo");
    assert_eq!(Tool::Rustfmt.as_str(), "rustfmt");

    let cases: Vec<(ToolError, &str)> = vec![
        (
            ToolError::Unavailable,
            "external tools are off for this session (start with --tools real to run them)",
        ),
        (
            ToolError::NotFound { tool: Tool::Cargo },
            "`cargo` was not found",
        ),
        (
            ToolError::NotFound {
                tool: Tool::Rustfmt,
            },
            "`rustfmt` was not found",
        ),
        (
            ToolError::Timeout {
                tool: Tool::Cargo,
                seconds: 120,
            },
            "`cargo` timed out after 120 s",
        ),
        (
            ToolError::Timeout {
                tool: Tool::Rustfmt,
                seconds: 0,
            },
            "`rustfmt` timed out after 0 s",
        ),
        (
            ToolError::Timeout {
                tool: Tool::Rustfmt,
                seconds: u64::MAX,
            },
            "`rustfmt` timed out after 18446744073709551615 s",
        ),
        (
            ToolError::Failed {
                tool: Tool::Cargo,
                message: "exit status 101".to_owned(),
            },
            "`cargo` failed: exit status 101",
        ),
        (
            ToolError::Failed {
                tool: Tool::Rustfmt,
                message: String::new(),
            },
            "`rustfmt` failed: ",
        ),
        (
            ToolError::Failed {
                tool: Tool::Rustfmt,
                message: "a: b `c` é😀".to_owned(),
            },
            "`rustfmt` failed: a: b `c` é😀",
        ),
        (
            ToolError::NoRecording { tool: Tool::Cargo },
            "no recorded output for this `cargo` request",
        ),
        (
            ToolError::NoRecording {
                tool: Tool::Rustfmt,
            },
            "no recorded output for this `rustfmt` request",
        ),
    ];
    for (error, text) in &cases {
        assert_eq!(error.to_string(), *text, "{error:?}");
        let boxed: Box<dyn std::error::Error> = Box::new(error.clone());
        assert_eq!(boxed.to_string(), *text);
        assert!(boxed.source().is_none(), "{error:?}");
        // No message names a host path (the working directory or the temp dir).
        let cwd = std::env::current_dir().unwrap();
        assert!(!text.contains(cwd.to_str().unwrap()));
    }
    // Errors are values: equal iff the same variant and fields.
    assert_ne!(
        ToolError::NotFound { tool: Tool::Cargo },
        ToolError::NotFound {
            tool: Tool::Rustfmt
        }
    );
    assert_ne!(
        ToolError::Timeout {
            tool: Tool::Cargo,
            seconds: 1
        },
        ToolError::Timeout {
            tool: Tool::Cargo,
            seconds: 2
        }
    );

    // The variant set is exactly the specified one (an exhaustive match without a wildcard).
    for (error, _) in &cases {
        let tool: Option<Tool> = match error {
            ToolError::Unavailable => None,
            ToolError::NotFound { tool, .. }
            | ToolError::Timeout { tool, .. }
            | ToolError::Failed { tool, .. }
            | ToolError::NoRecording { tool, .. } => Some(*tool),
        };
        if let Some(tool) = tool {
            assert!(error.to_string().contains(&format!("`{}`", tool.as_str())));
        }
    }
    for tool in BOTH {
        match tool {
            Tool::Cargo => assert_eq!(tool.as_str(), "cargo"),
            Tool::Rustfmt => assert_eq!(tool.as_str(), "rustfmt"),
        }
    }

    // Tool: Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash.
    assert!(Tool::Cargo < Tool::Rustfmt);
    let ordered: BTreeSet<Tool> = [Tool::Rustfmt, Tool::Cargo, Tool::Rustfmt].into();
    assert_eq!(ordered.into_iter().collect::<Vec<_>>(), BOTH);
    let hashed: HashSet<Tool> = BOTH.into();
    assert_eq!(hashed.len(), 2);
    assert_eq!(format!("{:?}", Tool::Cargo), "Cargo");
    assert_eq!(format!("{:?}", Tool::Rustfmt), "Rustfmt");

    // ToolRequest: Clone, Debug, PartialEq, Eq, Hash; ToolOutput: Clone, Debug, PartialEq, Eq.
    let req = ToolRequest {
        tool: Tool::Rustfmt,
        args: vec!["--emit".to_owned(), "stdout".to_owned()],
        stdin: Some("fn f(){}".to_owned()),
    };
    let mut reqs = HashSet::new();
    reqs.insert(req.clone());
    reqs.insert(req.clone());
    assert_eq!(reqs.len(), 1);
    assert_ne!(
        req,
        ToolRequest {
            stdin: None,
            ..req.clone()
        }
    );
    let out = ToolOutput {
        status: None,
        stdout: "o".to_owned(),
        stderr: "e".to_owned(),
    };
    assert_eq!(out.clone(), out);
    assert_ne!(
        out,
        ToolOutput {
            status: Some(0),
            ..out.clone()
        }
    );
    assert_eq!(out.status, None::<i32>);
}

/// A runner that records the requests it sees (a `ToolRunner` defined outside the crate).
#[derive(Debug, Default)]
struct Recorder {
    seen: Vec<ToolRequest>,
}

impl ToolRunner for Recorder {
    fn run(&mut self, request: &ToolRequest) -> Result<ToolOutput, ToolError> {
        self.seen.push(request.clone());
        Ok(ToolOutput {
            status: Some(0),
            stdout: request.args.join(" "),
            stderr: String::new(),
        })
    }
}

/// AC-4: the named items resolve at the crate root, `tools::NoTools` is the same type as the
/// root `NoTools`, and the re-exported items still behave as
/// before (exact values).
#[test]
fn workspace_reexports_resolve() {
    let via_module: kiln_workspace::tools::NoTools = NoTools;
    let at_root: kiln_workspace::NoTools = via_module;
    assert_eq!(at_root, kiln_workspace::tools::NoTools);
    let _: kiln_workspace::tools::Tool = Tool::Cargo;
    let _: kiln_workspace::tools::ToolError = ToolError::Unavailable;
    let _: kiln_workspace::tools::ToolOutput = ToolOutput {
        status: Some(1),
        stdout: String::new(),
        stderr: String::new(),
    };
    let _: kiln_workspace::tools::ToolRequest = ToolRequest {
        tool: Tool::Cargo,
        args: Vec::new(),
        stdin: None,
    };
    fn takes_runner(
        r: &mut dyn kiln_workspace::tools::ToolRunner,
    ) -> Result<ToolOutput, ToolError> {
        r.run(&ToolRequest {
            tool: Tool::Cargo,
            args: vec!["a".to_owned(), "b".to_owned()],
            stdin: None,
        })
    }
    let mut rec = Recorder::default();
    assert_eq!(
        takes_runner(&mut rec),
        Ok(ToolOutput {
            status: Some(0),
            stdout: "a b".to_owned(),
            stderr: String::new()
        })
    );
    assert_eq!(rec.seen.len(), 1);
    assert_eq!(takes_runner(&mut NoTools), Err(ToolError::Unavailable));

    // The glob re-exports of `vfs` and `document` still carry today's items.
    let path = RelPath::new("dir/a.txt").unwrap();
    let mut fs = MemoryFs::new();
    fs.insert(&path, b"\xef\xbb\xbfhi\n");
    let vfs: &mut dyn Vfs = &mut fs;
    let doc = Document::open(vfs, &path).unwrap();
    assert_eq!(doc.encoding(), Encoding::Utf8Bom);
    assert_eq!(doc.buffer().text(), "hi\n");
    let missing = RelPath::new("nope.txt").unwrap();
    match Document::open(vfs, &missing) {
        Err(OpenError::Vfs(VfsError::NotFound { path, .. })) => assert_eq!(path, missing),
        other => panic!("open of a missing file: {other:?}"),
    }
    assert_eq!(
        decode(b"\xff\xfeh\x00").unwrap(),
        ("h".to_owned(), Encoding::Utf16Le)
    );
    assert_eq!(encode("h", Encoding::Utf16Be), b"\xfe\xff\x00h");
    let root = fresh_dir("disk");
    let disk = DiskFs::new(&root).unwrap();
    let _: &dyn Vfs = &disk;
    let _ = std::fs::remove_dir_all(&root);
}
