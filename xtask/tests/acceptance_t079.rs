//! T-079 acceptance tests (test author): AC-1, the entries of `xtask/platform-tests.toml` are
//! exactly the 20 of Interface (2a) (the 8 restored from T-071) and (2) (the 12 product tests).
//!
//! The registry is parsed with `toml`; every `[[test]]` entry (any TOML spelling of the same
//! array of tables passes: `[[test]]` headers, an inline `test = [{ … }]`, any key order or
//! quoting) is reduced to the tuple AC-1 names, `(file, item, platforms, counterpart, task)`,
//! and the list of tuples must match the 20 expected ones one to one: an expected tuple without
//! an entry, an entry that matches no expected tuple, or two entries for one expected tuple
//! fail. `task` is part of the tuple, so a `pending` entry without `task` (or with another
//! task), and a `none` entry carrying a `task`, match nothing. For the 8 `pending` entries the
//! second test also checks `expires = "2026-12-31"` (ruling Q-6 A, D-233). Values are compared
//! as TOML strings, the registry's format (T-071); another type is a mismatch.
//!
//! The variants AC-1 names are rejected as follows: one of the 12 entered with
//! `platforms = "unix"` is an entry whose tuple matches none of the 20 (and its expected tuple
//! is then missing); a 21st entry is an entry left over after the 20 are matched (any entry
//! beyond 20 is, whatever it says, since each expected tuple takes one entry).
//!
//! Not pinned (AC-1's tuple leaves them to `no-skip` and the reviewer): `doc`, `quote`, `reason`,
//! the order of entries, comments, `expires` of a `none` entry (`no-skip` refuses it).
//!
//! QUESTION Q-T1 (test author, R-328; see the task log): Interface (1) lets the 4 non-UTF-8
//! tests be split ("split out if the test checks other things"; Interface (3): "a split keeps
//! the original name for the part that stays"), and the spec does not name the split-out
//! Linux fn the entry then lists. Until a ruling, those 4 expected tuples fix `file` (2 in
//! `crates/kiln-workspace/tests/acceptance_t019_workspace.rs`, 2 in
//! `crates/kiln-workspace/tests/project.rs`), `platforms = "linux"`, `counterpart = "none"`
//! and no `task`, and accept any `item` (distinct entries: the same `file::item` twice is
//! refused). The other 16 tuples are exact.
//!
//! On the base the registry has no entry, so both tests fail by assertion. Only `std::fs` and
//! `toml` are used (Linux, macOS, Windows alike); `file` values are compared as the
//! `/`-separated strings the registry holds. No process is spawned.
#![cfg(test)]

use std::path::{Path, PathBuf};

const REGISTRY: &str = "xtask/platform-tests.toml";
/// Ruling Q-6 A (D-233): every `pending` entry expires on this date.
const EXPIRES: &str = "2026-12-31";

/// One expected entry: `item` `None` is the open item of Q-T1 (any item in `file`).
#[derive(Clone, Copy, Debug)]
struct Expected {
    file: &'static str,
    item: Option<&'static str>,
    platforms: &'static str,
    counterpart: &'static str,
    task: Option<&'static str>,
}

const fn none(file: &'static str, item: &'static str, platforms: &'static str) -> Expected {
    Expected {
        file,
        item: Some(item),
        platforms,
        counterpart: "none",
        task: None,
    }
}

const fn pending(file: &'static str, item: &'static str, task: &'static str) -> Expected {
    Expected {
        file,
        item: Some(item),
        platforms: "linux",
        counterpart: "pending",
        task: Some(task),
    }
}

/// A non-UTF-8-name entry of Interface (2): item open (Q-T1).
const fn non_utf8(file: &'static str) -> Expected {
    Expected {
        file,
        item: None,
        platforms: "linux",
        counterpart: "none",
        task: None,
    }
}

const T019_WS: &str = "crates/kiln-workspace/tests/acceptance_t019_workspace.rs";
const PROJECT: &str = "crates/kiln-workspace/tests/project.rs";

/// Interface (2a) then (2): the 20 entries.
const EXPECTED: [Expected; 20] = [
    // (2a) the 8 restored from T-071 (counterpart "none").
    none(
        "xtask/src/queue.rs",
        "tests::basic_refusals_write_nothing",
        "linux",
    ),
    none(
        "xtask/src/worktree.rs",
        "tests::create_seed_list_remove",
        "linux",
    ),
    none(
        "xtask/src/publish.rs",
        "tests::a_failed_content_commit_in_step_four_exits_one",
        "unix",
    ),
    none(
        "xtask/tests/acceptance_t013_publish.rs",
        "publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue",
        "linux",
    ),
    none(
        "xtask/tests/guard_t005.rs",
        "worktree_build_does_not_touch_main_target",
        "unix",
    ),
    none(
        "xtask/tests/acceptance_t005_r3.rs",
        "tampered_cached_gate_binary_does_not_decide_the_gate",
        "linux",
    ),
    none(
        "xtask/tests/acceptance_t005_r4.rs",
        "planted_future_dated_xtask_does_not_decide_the_gate",
        "linux",
    ),
    none(
        "xtask/src/queue.rs",
        "tests::main_xtask_is_rebuilt_from_base_on_every_run",
        "unix",
    ),
    // (2) /dev/full (3): pending T-076.
    pending(
        "apps/kiln/tests/acceptance_t055.rs",
        "ac8_stdout_write_failure_exits_1",
        "T-076",
    ),
    pending(
        "apps/kiln-cli/tests/acceptance_t004_r2.rs",
        "run_and_control_exit_1_when_stdout_cannot_be_written",
        "T-076",
    ),
    pending(
        "apps/kiln-cli/tests/run.rs",
        "run_exits_1_when_stdout_cannot_be_written",
        "T-076",
    ),
    // (2) non-UTF-8 names (4): counterpart "none" (item open, Q-T1).
    non_utf8(T019_WS),
    non_utf8(T019_WS),
    non_utf8(PROJECT),
    non_utf8(PROJECT),
    // (2) no display (5): pending T-077.
    pending(
        "apps/kiln/tests/acceptance_t065.rs",
        "ac1_no_display_every_window_form",
        "T-077",
    ),
    pending(
        "apps/kiln/tests/acceptance_t065.rs",
        "ac1_no_display_exits_2_with_one_line",
        "T-077",
    ),
    pending(
        "apps/kiln/tests/acceptance_t065.rs",
        "ac1_wayland_only_or_unreachable_display_exits_2",
        "T-077",
    ),
    pending(
        "apps/kiln/tests/window_t065.rs",
        "without_a_display_the_window_exits_2_with_one_line",
        "T-077",
    ),
    pending(
        "apps/kiln/tests/guard_t055.rs",
        "kiln_without_subcommand_and_display_exits_nonzero_with_a_message",
        "T-077",
    ),
];

/// An entry of the registry, reduced to AC-1's tuple plus `expires`. A key that is absent is
/// `None`; a value that is not a TOML string is `Some("<not a string: …>")` (matches nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    file: Option<String>,
    item: Option<String>,
    platforms: Option<String>,
    counterpart: Option<String>,
    task: Option<String>,
    expires: Option<String>,
}

impl Entry {
    fn matches(&self, e: &Expected) -> bool {
        self.file.as_deref() == Some(e.file)
            && e.item.is_none_or(|i| self.item.as_deref() == Some(i))
            && self.item.is_some()
            && self.platforms.as_deref() == Some(e.platforms)
            && self.counterpart.as_deref() == Some(e.counterpart)
            && self.task.as_deref() == e.task
    }

    fn show(&self) -> String {
        format!(
            "file = {:?}, item = {:?}, platforms = {:?}, counterpart = {:?}, task = {:?}",
            self.file, self.item, self.platforms, self.counterpart, self.task
        )
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// The registry's entries (a missing file is an empty registry, as `no-skip` reads it).
fn entries() -> Vec<Entry> {
    let p = root().join(REGISTRY);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => panic!("precondition: cannot read {}: {e}", p.display()),
    };
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("{REGISTRY} is not TOML: {e}"));
    let Some(tests) = table.get("test") else {
        return Vec::new();
    };
    let tests = tests
        .as_array()
        .unwrap_or_else(|| panic!("`test` in {REGISTRY} is not an array of tables"));
    tests
        .iter()
        .map(|t| {
            let t = t
                .as_table()
                .unwrap_or_else(|| panic!("an element of `test` in {REGISTRY} is not a table"));
            let field = |k: &str| {
                t.get(k).map(|v| match v.as_str() {
                    Some(s) => s.to_owned(),
                    None => format!("<not a string: {v}>"),
                })
            };
            Entry {
                file: field("file"),
                item: field("item"),
                platforms: field("platforms"),
                counterpart: field("counterpart"),
                task: field("task"),
                expires: field("expires"),
            }
        })
        .collect()
}

#[test]
fn ac1_registry_entries_are_exactly_the_twenty_of_interface_2_and_2a() {
    let mut left = entries();
    let mut problems = Vec::new();

    // The same test listed twice is never one of the 20 (each names a distinct test).
    let mut seen = std::collections::BTreeSet::new();
    for e in &left {
        let key = (e.file.clone(), e.item.clone());
        if !seen.insert(key) {
            problems.push(format!("listed twice: {}", e.show()));
        }
    }

    // Exact expected tuples first, then the open-item ones (Q-T1), one entry each.
    let mut order: Vec<&Expected> = EXPECTED.iter().filter(|e| e.item.is_some()).collect();
    order.extend(EXPECTED.iter().filter(|e| e.item.is_none()));
    for want in order {
        match left.iter().position(|e| e.matches(want)) {
            Some(i) => {
                left.remove(i);
            }
            None => problems.push(format!("missing: {want:?}")),
        }
    }
    for e in &left {
        problems.push(format!("not one of the 20: {}", e.show()));
    }
    assert!(
        problems.is_empty(),
        "{REGISTRY} is not exactly the 20 entries of T-079 Interface (2a) and (2) (AC-1):\n{}",
        problems.join("\n")
    );
}

#[test]
fn ac1_pending_entries_name_their_task_and_expire_on_2026_12_31() {
    let got: std::collections::BTreeSet<(String, String, String, String)> = entries()
        .into_iter()
        .filter(|e| e.counterpart.as_deref() == Some("pending"))
        .map(|e| {
            let s = |v: Option<String>| v.unwrap_or_else(|| "<absent>".to_owned());
            (s(e.file), s(e.item), s(e.task), s(e.expires))
        })
        .collect();
    let want: std::collections::BTreeSet<(String, String, String, String)> = EXPECTED
        .iter()
        .filter(|e| e.counterpart == "pending")
        .map(|e| {
            (
                e.file.to_owned(),
                e.item.expect("pending entries are named").to_owned(),
                e.task.expect("pending entries name a task").to_owned(),
                EXPIRES.to_owned(),
            )
        })
        .collect();
    assert_eq!(want.len(), 8, "Interface (2) has 8 pending entries");
    assert_eq!(
        got, want,
        "the `pending` entries of {REGISTRY} (file, item, task, expires) are not the 8 of \
         Interface (2) with expires = {EXPIRES} (AC-1, ruling Q-6 A)"
    );
}
