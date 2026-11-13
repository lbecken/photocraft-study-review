//! T-039 AC-1: the synopsis block of `docs/control-protocol.md` "## `kiln-cli`" names, for
//! each of `run`, `exec`, `control` and `commands`, the same `--` options as that subcommand's
//! usage entry in `kiln-cli --help`. The oracle is the binary's own usage text; only option
//! sets are compared, never wording (ruling z1: the sentences of Interface (a), (b), (c) are
//! checked by the director on the diff, not by a test).
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);
const SUBCOMMANDS: [&str; 4] = ["run", "exec", "control", "commands"];

/// Stdout of `kiln-cli --help`; the process is killed after [`TIMEOUT`].
fn help_text() -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn kiln-cli");
    let mut out = child.stdout.take().expect("stdout");
    let reader = thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli --help did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    assert!(status.success(), "kiln-cli --help exited with {status}");
    reader.join().expect("stdout reader")
}

/// The tokens of `text` that begin with `--`, without the brackets and separators around them.
fn options(text: &str) -> BTreeSet<String> {
    text.split(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | '|'))
        .filter(|t| t.starts_with("--"))
        .map(str::to_owned)
        .collect()
}

/// The text of the usage entry of `sub` in the help: the line beginning `kiln-cli <sub>` and
/// its continuation lines (indented lines up to the next `kiln-cli` line or a blank line).
fn help_entry(help: &str, sub: &str) -> String {
    let head = format!("kiln-cli {sub} ");
    let mut lines = help.lines().map(str::trim);
    let first = lines
        .by_ref()
        .find(|l| l.starts_with(&head) || *l == head.trim_end())
        .unwrap_or_else(|| panic!("no usage entry for `{sub}` in kiln-cli --help:\n{help}"));
    let mut entry = first.to_owned();
    for line in lines {
        if line.is_empty() || line.starts_with("kiln-cli") {
            break;
        }
        entry.push(' ');
        entry.push_str(line);
    }
    entry
}

/// The lines of the first ```` ```text ```` block after the heading "## `kiln-cli`".
fn synopsis_block(doc: &str) -> Vec<String> {
    let mut lines = doc.lines();
    lines
        .by_ref()
        .find(|l| l.trim_end() == "## `kiln-cli`")
        .expect("heading ## `kiln-cli` in docs/control-protocol.md");
    lines
        .by_ref()
        .find(|l| l.trim() == "```text")
        .expect("a ```text block under ## `kiln-cli`");
    let block: Vec<String> = lines
        .take_while(|l| l.trim() != "```")
        .map(str::to_owned)
        .collect();
    assert!(!block.is_empty(), "empty synopsis block under ## `kiln-cli`");
    block
}

#[test]
fn protocol_synopsis_matches_help() {
    let doc_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/control-protocol.md");
    let doc = std::fs::read_to_string(&doc_path).expect("read docs/control-protocol.md");
    let block = synopsis_block(&doc);
    let help = help_text();
    for sub in SUBCOMMANDS {
        let head = format!("kiln-cli {sub} ");
        let doc_lines: Vec<&String> = block
            .iter()
            .filter(|l| l.trim().starts_with(&head))
            .collect();
        assert_eq!(
            doc_lines.len(),
            1,
            "exactly one synopsis line for `{sub}` under ## `kiln-cli`: {block:#?}"
        );
        let in_doc = options(doc_lines[0]);
        let entry = help_entry(&help, sub);
        let in_help = options(&entry);
        assert!(
            !in_help.is_empty(),
            "the usage entry of `{sub}` names no option: {entry}"
        );
        assert_eq!(
            in_doc, in_help,
            "options of `{sub}`: docs/control-protocol.md synopsis `{}` vs kiln-cli --help `{entry}`",
            doc_lines[0]
        );
    }
}
