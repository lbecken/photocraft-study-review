//! T-079 acceptance tests (test author), adapted in T-073 by ruling z1 (D-249): AC-1, the
//! 20 entries of `xtask/platform-tests.toml` from Interface (2a) (the 8 restored from T-071) and
//! (2) (the 12 product tests) are there, each exactly once and exactly as T-079 wrote it.
//! Entries for other tests (a later task's) are not these tests' matter: `no-skip` checks every
//! entry against a cfg'd item, and the base-names guard keeps every Linux test.
//!
//! The registry is parsed with `toml`; every `[[test]]` entry (any TOML spelling of the same
//! array of tables passes: `[[test]]` headers, an inline `test = [{ … }]`, any key order or
//! quoting) is reduced to `(file, item, platforms, counterpart, task, expires)`. An entry
//! *belongs to* one of the 20 when its `(file, item)` is that test's pair. For each of the 20,
//! exactly one entry belongs to it, and that entry has exactly T-079's `platforms`,
//! `counterpart` and `task` (an absent `task` must be absent): none belonging is "missing", two
//! or more is "entered more than once", another field value is "changed". The second test
//! checks that the `pending` entries among those belonging to the 20 are exactly T-079's 8,
//! with their task and `expires = "2026-12-31"` (ruling Q-6 A, D-233). Values are compared as
//! TOML strings, the registry's format (T-071); another type is a mismatch.
//!
//! Not pinned (AC-1's tuple leaves them to `no-skip` and the reviewer): `doc`, `quote`, `reason`,
//! the order of entries, comments, `expires` of a `none` entry (`no-skip` refuses it).
//!
//! Ruling w1 (D-241, on question Q-T1): for each of the 4 non-UTF-8 tests the registered item
//! is exactly one of two names: the test's name as Interface (1) gives it (not split), or that
//! name followed by `_non_utf8` (the Linux-only part split out). An entry under either name
//! belongs to that test, so an entry under each of the two names is "entered more than once".
//! The other 16 have one item each.
//!
//! Only `std::fs` and `toml` are used (Linux, macOS, Windows alike); `file` values are compared
//! as the `/`-separated strings the registry holds. No process is spawned.
#![cfg(test)]

use std::path::{Path, PathBuf};

const REGISTRY: &str = "xtask/platform-tests.toml";
/// Ruling Q-6 A (D-233): every `pending` entry expires on this date.
const EXPIRES: &str = "2026-12-31";

/// One expected entry. `split` (ruling w1): the item may also be `item` + [`SPLIT_SUFFIX`].
#[derive(Clone, Copy, Debug)]
struct Expected {
    file: &'static str,
    item: &'static str,
    split: bool,
    platforms: &'static str,
    counterpart: &'static str,
    task: Option<&'static str>,
}

const fn none(file: &'static str, item: &'static str, platforms: &'static str) -> Expected {
    Expected {
        file,
        item,
        split: false,
        platforms,
        counterpart: "none",
        task: None,
    }
}

const fn pending(file: &'static str, item: &'static str, task: &'static str) -> Expected {
    Expected {
        file,
        item,
        split: false,
        platforms: "linux",
        counterpart: "pending",
        task: Some(task),
    }
}

/// Ruling w1 (D-241): the name of the Linux-only part split out of a non-UTF-8 test.
const SPLIT_SUFFIX: &str = "_non_utf8";

/// A non-UTF-8-name entry of Interface (2): `item` or `item` + `_non_utf8` (ruling w1).
const fn non_utf8(file: &'static str, item: &'static str) -> Expected {
    Expected {
        file,
        item,
        split: true,
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
    // (2) non-UTF-8 names (4): counterpart "none"; split or not (ruling w1).
    non_utf8(T019_WS, "tree_counts_skipped_names_on_disk"),
    non_utf8(
        T019_WS,
        "disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names",
    ),
    non_utf8(PROJECT, "documented_differences_from_git"),
    non_utf8(
        PROJECT,
        "disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names",
    ),
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
    /// Whether this entry's `(file, item)` is the pair of `e` (either name of ruling w1).
    fn belongs_to(&self, e: &Expected) -> bool {
        self.file.as_deref() == Some(e.file)
            && self.item.as_deref().is_some_and(|i| {
                i == e.item || (e.split && i.strip_suffix(SPLIT_SUFFIX) == Some(e.item))
            })
    }

    /// Whether this entry has exactly `e`'s `platforms`, `counterpart` and `task`.
    fn fields_match(&self, e: &Expected) -> bool {
        self.platforms.as_deref() == Some(e.platforms)
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
fn ac1_each_of_the_twenty_has_exactly_one_entry_with_its_fields() {
    let all = entries();
    let mut problems = Vec::new();
    for want in &EXPECTED {
        let mine: Vec<&Entry> = all.iter().filter(|e| e.belongs_to(want)).collect();
        match mine.as_slice() {
            [] => problems.push(format!("missing: {want:?}")),
            [one] if one.fields_match(want) => {}
            [one] => problems.push(format!("changed: {}; expected {want:?}", one.show())),
            more => problems.push(format!(
                "entered more than once ({}): {}",
                more.len(),
                more.iter()
                    .map(|e| e.show())
                    .collect::<Vec<_>>()
                    .join(" | ")
            )),
        }
    }
    assert!(
        problems.is_empty(),
        "{REGISTRY}: the 20 entries of T-079 Interface (2a) and (2) are not each there exactly \
         once with their fields (AC-1, ruling z1):\n{}",
        problems.join("\n")
    );
}

#[test]
fn ac1_pending_entries_name_their_task_and_expire_on_2026_12_31() {
    let got: std::collections::BTreeSet<(String, String, String, String)> = entries()
        .into_iter()
        .filter(|e| EXPECTED.iter().any(|w| e.belongs_to(w)))
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
                e.item.to_owned(),
                e.task.expect("pending entries name a task").to_owned(),
                EXPIRES.to_owned(),
            )
        })
        .collect();
    assert_eq!(want.len(), 8, "Interface (2) has 8 pending entries");
    assert_eq!(
        got, want,
        "the `pending` entries of {REGISTRY} among T-079's 20 tests (file, item, task, expires) \
         are not the 8 of Interface (2) with expires = {EXPIRES} (AC-1, ruling Q-6 A, ruling z1)"
    );
}
