//! T-055 guards: behaviour main already has that the task must keep (they pass on base).
//!
//! * `kiln` with no subcommand keeps its current behaviour in this task (Interface, "Exit
//!   codes": T-056 replaces it with the window): it exits 1, prints nothing on stdout and says
//!   why on stderr, without a panic, also with no display variable set. The message text is not
//!   pinned (golden rule 9).
//! * The oracle of AC-1, `kiln_control::Host`, still answers the control parts of Pinned (g) as
//!   the spec records them for this main: which responses succeed, the refusal's code and kind,
//!   and a log of the three successful requests renumbered 1 to 3.
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use kiln_control::Host;

const TIMEOUT: Duration = Duration::from_secs(60);

/// (exit code, stdout, stderr) of `kiln` with no arguments, display variables removed.
fn kiln_no_args() -> (i32, Vec<u8>, Vec<u8>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln"))
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln");
    let mut out = child.stdout.take().expect("stdout");
    let mut err = child.stderr.take().expect("stderr");
    let out_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let err_reader = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let code = status
        .code()
        .unwrap_or_else(|| panic!("kiln ended by a signal ({status})"));
    (
        code,
        out_reader.join().expect("stdout reader"),
        err_reader.join().expect("stderr reader"),
    )
}

#[test]
fn kiln_without_subcommand_exits_1_with_a_message() {
    let (code, stdout, stderr) = kiln_no_args();
    assert_eq!(
        code,
        1,
        "exit code of `kiln` (stderr: {})",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stdout.is_empty(), "`kiln` printed on stdout");
    assert!(!stderr.is_empty(), "`kiln` gave no reason on stderr");
    assert!(
        !String::from_utf8_lossy(&stderr).contains("panicked"),
        "`kiln` panicked"
    );
}

fn case_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("t055-guard")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("src")).expect("create case dir");
    d
}

/// `"ok":true` or `"ok":false` (compact JSON with sorted keys, as the protocol states).
fn ok_of(response: &str) -> bool {
    match (
        response.contains(r#""ok":true"#),
        response.contains(r#""ok":false"#),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => panic!("not one ok flag: {response}"),
    }
}

#[test]
fn host_answers_the_pinned_worked_example() {
    let root = case_dir("pinned-g");
    fs::write(root.join("src/a.rs"), "fn main() {\n    let x = 1;\n}\n").expect("write a.rs");
    let mut host = Host::on_disk(&root).expect("Host::on_disk");
    let lines: [&[u8]; 4] = [
        br#"{"id":1,"command":"doc.list"}"#,
        br#"{"id":2,"command":"doc.open","params":{"path":"src/a.rs"}}"#,
        br#"{"id":3,"command":"doc.open","params":{"path":"src/missing.rs"}}"#,
        br#"{"id":4,"command":"doc.list"}"#,
    ];
    let responses: Vec<String> = lines
        .iter()
        .map(|l| host.handle_line(l).expect("a response"))
        .collect();
    let oks: Vec<bool> = responses.iter().map(|r| ok_of(r)).collect();
    assert_eq!(oks, [true, true, false, true], "{responses:#?}");
    assert!(
        responses[2].contains(r#""code":"rejected""#)
            && responses[2].contains(r#""kind":"not_found""#),
        "{}",
        responses[2]
    );
    assert!(
        responses[3].contains(r#""path":"src/a.rs""#),
        "{}",
        responses[3]
    );
    let log = host.log_script();
    let expected = "{\"command\":\"doc.list\",\"id\":1,\"params\":{}}\n\
                    {\"command\":\"doc.open\",\"id\":2,\"params\":{\"path\":\"src/a.rs\"}}\n\
                    {\"command\":\"doc.list\",\"id\":3,\"params\":{}}\n";
    assert_eq!(
        log, expected,
        "the log holds requests 1, 2 and 4, renumbered 1 to 3"
    );
    let _ = fs::remove_dir_all(&root);
}
