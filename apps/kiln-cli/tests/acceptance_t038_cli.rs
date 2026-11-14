//! T-038 acceptance tests through the built `kiln-cli`: AC-1 (each command's published
//! `refusals`, and `error_kinds.rejected` as their union), AC-4 (every published refusal
//! example, run through the binary, refuses with its kind; the examples cover the E column) and
//! AC-8 (the kinds table of `docs/control-protocol.md` has no `rejected`/`other` row and lists
//! only published `rejected` kinds). Written by the test author from
//! `tasks/T-038-error-kinds-match-commands.md` alone.
//!
//! The oracle is the spec's table "The kinds each command can report", encoded once below as
//! data ([`TABLE`]): a later task that adds a command or a kind changes that table only. The
//! tests look up the table's commands in the description; they never assert the registry's
//! command list, the absence of a kind from the table of `RejectReason`, the wording of a `when`
//! text, a message, or a document sentence (golden rule 9). Order is checked against
//! `RejectReason::ALL`, the order the spec names. Rulings applied (D-130): the comparison of
//! `commands --schema` with `docs/commands.json` is the gate's `schema` step and is not written
//! here; AC-8's check that a phrase is gone from a document is the reviewer's.
//!
//! Every child process has a 10 s timeout that only detects hangs; no time is asserted.
//! Fixtures live in fresh directories removed on drop (success and failure).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use kiln_commands::RejectReason;
use serde_json::{Value, json};

/// Hang detector for every child process; never a duration assertion.
const TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------------------------
// The spec's table ("The kinds each command can report"): commands → E (refusal examples,
// `MemoryFs` produces them) and D (disk only). This is the oracle of AC-1 and AC-4.

const NSD: &str = "no_such_document";
const OOR: &str = "out_of_range";

/// (commands of the row, E column, D column), as in the spec's table.
const TABLE: &[(&[&str], &[&str], &[&str])] = &[
    (&["buffer.insert"], &[OOR], &[]),
    (&["buffer.text", "doc.list"], &[], &[]),
    (
        &[
            "cursor.get",
            "cursor.move",
            "doc.info",
            "doc.redo",
            "doc.text",
            "doc.undo",
            "eol.counts",
            "eol.normalize",
            "nav.goto-line",
        ],
        &[NSD],
        &[],
    ),
    (
        &["cursor.set", "doc.position"],
        &[NSD, OOR, "not_a_boundary"],
        &[],
    ),
    (&["doc.close"], &[NSD, "unsaved_changes"], &[]),
    (
        &["doc.delete", "doc.replace"],
        &[NSD, OOR, "inverted_range"],
        &[],
    ),
    (&["doc.insert", "doc.line"], &[NSD, OOR], &[]),
    (
        &["doc.open"],
        &["invalid_path", "not_found", "not_a_file", "decode_error"],
        &["io_error", "outside_root"],
    ),
    (
        &["doc.save"],
        &[NSD],
        &["not_found", "not_a_file", "io_error", "outside_root"],
    ),
    (
        &["project.tree"],
        &["invalid_path", "not_found", "not_a_file"],
        &["io_error", "outside_root"],
    ),
    (&["syntax.tree", "nav.outline"], &[NSD, "too_large"], &[]),
    (
        &["syntax.highlight"],
        &[NSD, "too_large", OOR, "inverted_range"],
        &[],
    ),
    (
        &["syntax.match-bracket", "nav.goto-symbol"],
        &[NSD, "too_large", OOR],
        &[],
    ),
    (&["search.find"], &[NSD, "invalid_pattern"], &[]),
    (&["search.replace"], &[NSD, "invalid_pattern", OOR], &[]),
    (
        &["nav.workspace-symbols"],
        &["invalid_pattern"],
        &["io_error", "outside_root", "not_found", "not_a_file"],
    ),
];

/// The command whose published `refusals` may hold more than its row (T-027 adds
/// `changed_on_disk`; AC-1).
const SUPERSET_ROW: &str = "doc.save";

/// One row per command: name → (E, D).
fn table() -> BTreeMap<&'static str, (BTreeSet<&'static str>, BTreeSet<&'static str>)> {
    let mut out = BTreeMap::new();
    for (names, e, d) in TABLE {
        for name in *names {
            let row = (e.iter().copied().collect(), d.iter().copied().collect());
            assert!(
                out.insert(*name, row).is_none(),
                "test bug: {name} twice in the table"
            );
        }
    }
    out
}

/// The counts the spec pins for its table ("30 commands; 56 E pairs; 12 D pairs"; 13 kinds),
/// so a typo in [`TABLE`] is caught here and not taken for a product defect.
fn check_table_encoding() {
    let t = table();
    assert_eq!(t.len(), 30, "test bug: the table has 30 commands");
    let e: usize = t.values().map(|(e, _)| e.len()).sum();
    let d: usize = t.values().map(|(_, d)| d.len()).sum();
    assert_eq!(e, 56, "test bug: 56 E pairs");
    assert_eq!(d, 12, "test bug: 12 D pairs");
    for (name, (e, d)) in &t {
        assert!(e.is_disjoint(d), "test bug: {name} has a kind in E and D");
    }
    let union: BTreeSet<&str> = t.values().flat_map(|(e, d)| e.union(d).copied()).collect();
    assert_eq!(union.len(), 13, "test bug: 13 kinds are declared");
    let all = all_kinds();
    for k in &union {
        assert!(all.contains(k), "test bug: {k} is not a RejectReason");
    }
}

/// The kinds of `RejectReason::ALL`, in order (the order the spec names for every list).
fn all_kinds() -> Vec<&'static str> {
    RejectReason::ALL.iter().map(|r| r.as_str()).collect()
}

/// `list` holds distinct items, in the order they have in `order` (a subsequence of it).
fn in_order_of(list: &[&str], order: &[&str]) -> Result<(), String> {
    let mut last = None;
    for k in list {
        let Some(pos) = order.iter().position(|o| o == k) else {
            return Err(format!("{k} is not in {order:?}"));
        };
        if let Some(prev) = last
            && pos <= prev
        {
            return Err(format!("{list:?} is not in the order of {order:?} (or repeats)"));
        }
        last = Some(pos);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Running the binary.

struct Out {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

fn wait_or_kill(child: &mut Child, what: &str) -> ExitStatus {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("hang detector: {what} did not finish within {TIMEOUT:?}; killed");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Run `kiln-cli <args>` in `cwd` (stdin empty), with the hang detector.
fn kiln(args: &[&str], cwd: &Path) -> Out {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let out = thread::spawn(move || {
        let mut s = String::new();
        let _ = so.read_to_string(&mut s);
        s
    });
    let err = thread::spawn(move || {
        let mut s = String::new();
        let _ = se.read_to_string(&mut s);
        s
    });
    let status = wait_or_kill(&mut child, &format!("kiln-cli {}", args.join(" ")));
    Out {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

/// A fresh directory under the system temp dir, removed on drop (failed runs included).
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("k38a-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `kiln-cli commands --schema`, parsed.
fn schema(cwd: &Path) -> Value {
    let out = kiln(&["commands", "--schema"], cwd);
    assert!(
        out.status.success(),
        "commands --schema: {:?} {}",
        out.status,
        out.stderr
    );
    serde_json::from_str(&out.stdout).expect("commands --schema prints JSON")
}

fn commands(schema: &Value) -> &Vec<Value> {
    schema["commands"]
        .as_array()
        .expect("`commands` is an array")
}

fn find<'a>(schema: &'a Value, name: &str) -> &'a Value {
    commands(schema)
        .iter()
        .find(|c| c["name"] == json!(name))
        .unwrap_or_else(|| panic!("the spec's table names {name}; commands --schema lacks it"))
}

/// The published `rejected` kinds, after checking each entry has exactly `kind` and `when`.
fn rejected_kinds(schema: &Value) -> Vec<String> {
    let list = schema["error_kinds"]["rejected"]
        .as_array()
        .expect("error_kinds.rejected is an array");
    list.iter()
        .map(|e| {
            let keys: BTreeSet<&str> = e
                .as_object()
                .unwrap_or_else(|| panic!("an error_kinds entry is an object: {e}"))
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                keys,
                BTreeSet::from(["kind", "when"]),
                "each error_kinds entry has exactly the keys kind, when (Interface 1): {e}"
            );
            assert!(e["when"].is_string(), "`when` is a string: {e}");
            e["kind"]
                .as_str()
                .unwrap_or_else(|| panic!("`kind` is a string: {e}"))
                .to_owned()
        })
        .collect()
}

/// A command entry's `refusals`, as strings (panics naming the command if absent or malformed).
fn refusals(cmd: &Value) -> Vec<String> {
    let name = &cmd["name"];
    let list = cmd["refusals"].as_array().unwrap_or_else(|| {
        panic!("command {name} has no `refusals` list (Interface 1); its keys: {:?}", keys(cmd))
    });
    list.iter()
        .map(|k| {
            k.as_str()
                .unwrap_or_else(|| panic!("{name}: a refusal is a string: {k}"))
                .to_owned()
        })
        .collect()
}

fn keys(v: &Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

// ---------------------------------------------------------------------------------------------
// AC-1

/// AC-1: in `commands --schema` every command entry has `refusals`: distinct strings, each a
/// kind of `error_kinds.rejected`, in that list's order. For each command of the spec's table
/// (but `doc.save`) the set of `refusals` equals its row (E ∪ D); `doc.save`'s contains its row.
/// `error_kinds.rejected` is exactly the union of all `refusals`, in `RejectReason::ALL` order;
/// each of its entries has exactly the keys `kind`, `when`; the top-level keys, the keys of
/// `error_kinds` and `error_kinds.internal` are unchanged.
/// Fails on base by assertion: no command entry has `refusals`.
#[test]
fn schema_publishes_each_commands_refusals() {
    check_table_encoding();
    let dir = Scratch::new("schema");
    let s = schema(&dir.0);
    let all = all_kinds();

    let top: BTreeSet<String> = keys(&s).into_iter().collect();
    assert_eq!(
        top,
        BTreeSet::from(["commands".to_owned(), "error_kinds".to_owned()]),
        "top-level keys are unchanged (Interface 1)"
    );
    let ek: BTreeSet<String> = keys(&s["error_kinds"]).into_iter().collect();
    assert_eq!(
        ek,
        BTreeSet::from(["internal".to_owned(), "rejected".to_owned()]),
        "keys of error_kinds are unchanged (Interface 1)"
    );
    let internal: Vec<&Value> = s["error_kinds"]["internal"]
        .as_array()
        .expect("error_kinds.internal is an array")
        .iter()
        .map(|e| &e["kind"])
        .collect();
    assert_eq!(
        internal,
        [&json!("panic"), &json!("result_schema"), &json!("other")],
        "error_kinds.internal is unchanged (Interface 1)"
    );

    let published = rejected_kinds(&s);
    let published = strs(&published);
    in_order_of(&published, &all)
        .unwrap_or_else(|e| panic!("error_kinds.rejected in RejectReason::ALL order: {e}"));

    // Every command entry, whether or not the table names it.
    let mut union = BTreeSet::new();
    for cmd in commands(&s) {
        let name = cmd["name"].as_str().expect("a command has a name");
        let r = refusals(cmd);
        let r = strs(&r);
        in_order_of(&r, &published).unwrap_or_else(|e| {
            panic!("{name}: `refusals` are distinct published kinds in error_kinds order: {e}")
        });
        union.extend(r.iter().map(|k| (*k).to_owned()));
    }

    // The table's rows (exact sets; doc.save a superset).
    let mut wrong = Vec::new();
    for (name, (e, d)) in table() {
        let cmd = find(&s, name);
        let got: BTreeSet<String> = refusals(cmd).into_iter().collect();
        let want: BTreeSet<String> = e.union(&d).map(|k| (*k).to_owned()).collect();
        let fits = if name == SUPERSET_ROW {
            got.is_superset(&want)
        } else {
            got == want
        };
        if !fits {
            wrong.push(format!("{name}: published {got:?}, the table says {want:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "per-command `refusals` differ from the spec's table:\n{}",
        wrong.join("\n")
    );

    // error_kinds.rejected = the union of the refusals, in ALL order.
    let want: Vec<&str> = all
        .iter()
        .copied()
        .filter(|k| union.contains(*k))
        .collect();
    assert_eq!(
        published, want,
        "error_kinds.rejected is exactly the union of all commands' `refusals`, in \
         RejectReason::ALL order (Interface 1)"
    );
}

// ---------------------------------------------------------------------------------------------
// AC-4

/// The exact keys of a refusal example (Interface 1).
const EXAMPLE_KEYS: [&str; 6] = [
    "description",
    "files",
    "initial_text",
    "kind",
    "params",
    "setup",
];

/// Run one refusal example of `name` through the binary in a fresh directory under `base`;
/// `Err` says what went wrong.
fn run_refusal_example(base: &Path, n: usize, name: &str, ex: &Value) -> Result<(), String> {
    let got: BTreeSet<&str> = ex
        .as_object()
        .ok_or("not an object")?
        .keys()
        .map(String::as_str)
        .collect();
    if got != BTreeSet::from(EXAMPLE_KEYS) {
        return Err(format!("keys {got:?}, want exactly {EXAMPLE_KEYS:?}"));
    }
    let kind = ex["kind"].as_str().ok_or("`kind` is not a string")?;
    let initial = ex["initial_text"]
        .as_str()
        .ok_or("`initial_text` is not a string")?;
    if !ex["description"].is_string() {
        return Err("`description` is not a string".to_owned());
    }
    let files = ex["files"].as_array().ok_or("`files` is not a list")?;
    let setup = ex["setup"].as_array().ok_or("`setup` is not a list")?;

    let dir = base.join(format!("ex{n}"));
    fs::create_dir_all(&dir).unwrap();
    // The manifest: the example's `files` copied as the manifest's `files` (ruling Q-6).
    fs::write(
        dir.join("m.json"),
        json!({ "files": files }).to_string().as_bytes(),
    )
    .unwrap();

    if !initial.is_empty() {
        // "when `initial_text` is not empty (the example has no setup)" (AC-4).
        if !setup.is_empty() {
            return Err("has an initial_text and a setup; AC-4 says such an example has no \
                        setup"
                .to_owned());
        }
        let params = ex["params"].to_string();
        let out = kiln(
            &[
                "exec",
                name,
                &params,
                "--text",
                initial,
                "--memory-seed",
                "m.json",
            ],
            &dir,
        );
        let err: Value = serde_json::from_str(out.stderr.lines().next().unwrap_or(""))
            .map_err(|e| format!("exec: stderr is not one error object ({e}): {}", out.stderr))?;
        if err["code"] != json!("rejected") || err["kind"] != json!(kind) {
            return Err(format!(
                "exec: want rejected/{kind}, got {err} (exit {:?})",
                out.status.code()
            ));
        }
        if out.status.code() != Some(2) {
            return Err(format!("exec: a command error exits 2, got {:?}", out.status));
        }
        return Ok(());
    }

    let mut script = String::new();
    for (i, step) in setup.iter().enumerate() {
        let (Some(cmd), Some(params)) = (step[0].as_str(), step.get(1)) else {
            return Err(format!(
                "setup {i} is not [name, params] (as in `examples`): {step}"
            ));
        };
        if step.as_array().map(Vec::len) != Some(2) {
            return Err(format!("setup {i} is not [name, params]: {step}"));
        }
        script.push_str(&json!({"id": i, "command": cmd, "params": params}).to_string());
        script.push('\n');
    }
    let last = setup.len();
    script.push_str(&json!({"id": last, "command": name, "params": ex["params"]}).to_string());
    script.push('\n');
    fs::write(dir.join("s.jsonl"), script.as_bytes()).unwrap();

    let out = kiln(&["run", "s.jsonl", "--memory-seed", "m.json"], &dir);
    let responses: Vec<Value> = out
        .stdout
        .lines()
        .map(|l| serde_json::from_str(l).map_err(|e| format!("run: bad line {l}: {e}")))
        .collect::<Result<_, _>>()?;
    if responses.len() != last + 1 {
        return Err(format!(
            "run: {} responses for {} requests (exit {:?}, stderr {})",
            responses.len(),
            last + 1,
            out.status.code(),
            out.stderr
        ));
    }
    for (i, r) in responses[..last].iter().enumerate() {
        if r["ok"] != json!(true) {
            return Err(format!("setup {i} failed: {r}"));
        }
    }
    let r = &responses[last];
    if r["ok"] != json!(false)
        || r["error"]["code"] != json!("rejected")
        || r["error"]["kind"] != json!(kind)
    {
        return Err(format!("want rejected/{kind}, got {r}"));
    }
    if out.status.code() != Some(3) {
        return Err(format!("run with one failed request exits 3, got {:?}", out.status));
    }
    Ok(())
}

/// AC-4: every refusal example in `commands --schema`, run as written (manifest from `files`, a
/// script of its `setup` then the request, `kiln-cli run s.jsonl --memory-seed m.json`; with a
/// non-empty `initial_text`, `kiln-cli exec … --text … --memory-seed m.json`), has every setup
/// response `ok: true` and ends `rejected` with the example's `kind`, which is one of the
/// command's `refusals`. For each command of the spec's table the set of kinds of its refusal
/// examples equals its E column (so no D kind has an example). Also (AC-3, through the binary;
/// the evaluator's call of Pinned (h)) `kiln-cli commands --audit` exits 0.
/// Oracle: the binary, independently of `audit`, and the spec's table.
/// Fails on base by assertion: no command entry has `refusal_examples`.
#[test]
fn every_refusal_example_refuses_through_the_binary() {
    check_table_encoding();
    let dir = Scratch::new("examples");
    let audit = kiln(&["commands", "--audit"], &dir.0);
    assert!(
        audit.status.success(),
        "kiln-cli commands --audit exits 0 (AC-3; Interface 3: the standard registry's audit is \
         empty): {:?}\n{}",
        audit.status,
        audit.stderr
    );

    let s = schema(&dir.0);
    let mut failures = Vec::new();
    let mut by_command: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut n = 0;
    for cmd in commands(&s) {
        let name = cmd["name"].as_str().expect("a command has a name");
        let Some(examples) = cmd["refusal_examples"].as_array() else {
            failures.push(format!(
                "{name}: no `refusal_examples` list (Interface 1); keys {:?}",
                keys(cmd)
            ));
            continue;
        };
        let declared: BTreeSet<String> = cmd["refusals"]
            .as_array()
            .map(|l| l.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect())
            .unwrap_or_default();
        let kinds = by_command.entry(name.to_owned()).or_default();
        for (i, ex) in examples.iter().enumerate() {
            n += 1;
            let kind = ex["kind"].as_str().unwrap_or("?").to_owned();
            if !declared.contains(&kind) {
                failures.push(format!(
                    "{name} refusal example {i}: kind {kind} is not in its refusals {declared:?}"
                ));
            }
            if let Err(e) = run_refusal_example(&dir.0, n, name, ex) {
                failures.push(format!("{name} refusal example {i} ({kind}): {e}"));
            }
            kinds.insert(kind);
        }
    }

    for (name, (e, _)) in table() {
        let want: BTreeSet<String> = e.iter().map(|k| (*k).to_owned()).collect();
        match by_command.get(name) {
            None => failures.push(format!("{name}: not in commands --schema")),
            Some(got) if *got != want => failures.push(format!(
                "{name}: refusal examples cover {got:?}; the table's E column is {want:?}"
            )),
            Some(_) => {}
        }
    }
    assert!(
        failures.is_empty(),
        "{} problem(s) over {n} refusal example(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// AC-8

/// The (code, kind) rows of the table under "### Error codes and kinds" in
/// `docs/control-protocol.md` (the backticked words of the first two cells).
fn kinds_table_rows(doc: &str) -> Vec<(String, String)> {
    let start = doc
        .find("### Error codes and kinds")
        .expect("docs/control-protocol.md has the section \"Error codes and kinds\"");
    let mut rows = Vec::new();
    let mut in_table = false;
    for line in doc[start..].lines().skip(1) {
        let line = line.trim();
        if !line.starts_with('|') {
            if in_table {
                break;
            }
            continue;
        }
        in_table = true;
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 {
            continue;
        }
        let tick = |c: &str| {
            c.strip_prefix('`')
                .and_then(|c| c.strip_suffix('`'))
                .map(str::to_owned)
        };
        if let (Some(code), Some(kind)) = (tick(cells[0]), tick(cells[1])) {
            rows.push((code, kind));
        }
    }
    rows
}

/// AC-8: the table of "Error codes and kinds" in `docs/control-protocol.md` has no row with
/// code `rejected` and kind `other`, and every `rejected` row's kind is published in
/// `error_kinds.rejected` of `kiln-cli commands --schema`. (The removal of the phrase in
/// `doc-positions.md` is the reviewer's diff check, ruling D-130.)
/// Fails on base by assertion: the `rejected`/`other` row is there.
#[test]
fn protocol_docs_state_the_published_set() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/control-protocol.md");
    let doc = fs::read_to_string(&path).expect("read docs/control-protocol.md");
    let rows = kinds_table_rows(&doc);
    let rejected: Vec<&str> = rows
        .iter()
        .filter(|(code, _)| code == "rejected")
        .map(|(_, kind)| kind.as_str())
        .collect();
    assert!(
        !rejected.is_empty(),
        "precondition: the kinds table was found and has `rejected` rows: {rows:?}"
    );
    assert!(
        !rejected.contains(&"other"),
        "the row `rejected` / `other` is removed (D-092 Q-2): {rows:?}"
    );
    let dir = Scratch::new("docs");
    let s = schema(&dir.0);
    let published = rejected_kinds(&s);
    let unpublished: Vec<&&str> = rejected
        .iter()
        .filter(|k| !published.iter().any(|p| p == **k))
        .collect();
    assert!(
        unpublished.is_empty(),
        "every `rejected` row's kind is published in error_kinds: not published {unpublished:?}"
    );
}
