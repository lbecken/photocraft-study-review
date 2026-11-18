//! T-074 AC-3 (test author): the nine registry entries of `disk.rs`'s Unix-only tests that
//! waited on T-074 each name their Windows counterpart. Written from
//! `tasks/T-074-windows-confinement-rules.md` (Interface (5), "Registry text", AC-3) alone.
//!
//! For each of the nine `(file, item)` pairs of Interface (5) the registry holds exactly one
//! entry; it has `platforms = "unix"`, `counterpart` equal to exactly the string of Interface
//! (5)'s "Registry text" for that row, and no `reason`, `task` or `expires` key at all. And no
//! entry of the registry has `task = "T-074"`. Fails on base on every platform: there the nine
//! are `counterpart = "pending"`, `task = "T-074"`, `expires = "2026-12-31"`.
//!
//! The registry is read as text and parsed with `toml`, as `acceptance_t079.rs` does: any TOML
//! spelling of the same array of tables passes. Values are compared as TOML strings; another
//! type is a mismatch. Not pinned here (other tasks change them, or `no-skip` and the reviewer
//! check them): `doc`, `quote`, the order of entries, comments, the other entries.
//!
//! Only `std::fs` and `toml` are used (Linux, macOS, Windows alike). No process is spawned.
#![cfg(test)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const REGISTRY: &str = "xtask/platform-tests.toml";
const DISK: &str = "crates/kiln-workspace/src/disk.rs";

/// Interface (5): (entry item in `disk.rs`, the counterpart string exactly).
const NINE: [(&str, &str); 9] = [
    (
        "tests::symlink_at_temp_name_is_not_followed",
        "crates/kiln-workspace/src/disk.rs::tests::link_at_temp_name_is_not_followed_on_every_platform",
    ),
    (
        "tests::replace_swaps_inode_keeps_permissions_and_follows_symlinks",
        "crates/kiln-workspace/src/disk.rs::tests::save_through_a_link_on_every_platform",
    ),
    (
        "tests::root_replaced_by_a_symlink_between_requests_is_outside",
        "crates/kiln-workspace/src/disk.rs::tests::root_replaced_by_a_link_on_every_platform",
    ),
    (
        "tests::parent_replaced_by_a_symlink_between_requests_is_outside",
        "crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root",
    ),
    (
        "tests::root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside",
        "crates/kiln-workspace/src/disk.rs::tests::root_removed_on_every_platform",
    ),
    (
        "tests::root_given_through_a_link_that_is_retargeted",
        "crates/kiln-workspace/src/disk.rs::tests::root_given_through_a_link_on_every_platform",
    ),
    (
        "tests::link_target_with_trailing_slash_requires_a_directory",
        "crates/kiln-workspace/src/disk.rs::tests::save_through_a_link_on_every_platform",
    ),
    (
        "tests::check_location_is_ok_for_every_inside_location",
        "crates/kiln-workspace/src/disk.rs::tests::check_location_inside_on_every_platform",
    ),
    (
        "tests::check_location_refuses_outside_loops_and_a_replaced_root",
        "crates/kiln-workspace/src/disk.rs::tests::check_location_outside_on_every_platform",
    ),
];

/// The keys an entry with a named counterpart must not have.
const FORBIDDEN: [&str; 3] = ["reason", "task", "expires"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// Every `[[test]]` entry of the registry, as key → value (a string as itself, any other TOML
/// value as `<not a string: …>`).
fn entries() -> Vec<BTreeMap<String, String>> {
    let p = root().join(REGISTRY);
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", p.display()));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("{REGISTRY} is not TOML: {e}"));
    let tests = table
        .get("test")
        .unwrap_or_else(|| panic!("{REGISTRY} has no `test` entries"))
        .as_array()
        .unwrap_or_else(|| panic!("`test` in {REGISTRY} is not an array of tables"));
    tests
        .iter()
        .map(|t| {
            t.as_table()
                .unwrap_or_else(|| panic!("an element of `test` in {REGISTRY} is not a table"))
                .iter()
                .map(|(k, v)| {
                    let v = match v.as_str() {
                        Some(s) => s.to_owned(),
                        None => format!("<not a string: {v}>"),
                    };
                    (k.clone(), v)
                })
                .collect()
        })
        .collect()
}

fn show(e: &BTreeMap<String, String>) -> String {
    e.iter()
        .filter(|(k, _)| !matches!(k.as_str(), "doc" | "quote" | "reason"))
        .map(|(k, v)| format!("{k} = {v:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// AC-3, first half: each of the nine has exactly one entry, `platforms = "unix"`, its exact
/// counterpart, and none of `reason`, `task`, `expires`. Rejects (besides base's `pending`):
/// one left `pending` with `task = "T-095"`; #14 naming #15's test; a counterpart without the
/// file prefix or with another file; `expires` or `reason` kept beside a named counterpart; an
/// entry duplicated; an entry dropped; `platforms` changed.
#[test]
fn ac3_each_of_the_nine_entries_names_exactly_its_counterpart() {
    let all = entries();
    let mut problems = Vec::new();
    for (item, counterpart) in NINE {
        let mine: Vec<&BTreeMap<String, String>> = all
            .iter()
            .filter(|e| {
                e.get("file").map(String::as_str) == Some(DISK)
                    && e.get("item").map(String::as_str) == Some(item)
            })
            .collect();
        let [one] = mine.as_slice() else {
            problems.push(format!(
                "{DISK}::{item}: expected exactly one entry, found {}: {}",
                mine.len(),
                mine.iter().map(|e| show(e)).collect::<Vec<_>>().join(" | ")
            ));
            continue;
        };
        let mut wrong = Vec::new();
        if one.get("platforms").map(String::as_str) != Some("unix") {
            wrong.push(format!(
                "platforms = {:?}, expected \"unix\"",
                one.get("platforms")
            ));
        }
        if one.get("counterpart").map(String::as_str) != Some(counterpart) {
            wrong.push(format!(
                "counterpart = {:?}, expected {counterpart:?}",
                one.get("counterpart")
            ));
        }
        for k in FORBIDDEN {
            if one.contains_key(k) {
                wrong.push(format!("has {k}, expected no {k}"));
            }
        }
        if !wrong.is_empty() {
            problems.push(format!(
                "{DISK}::{item}: {} (entry: {})",
                wrong.join("; "),
                show(one)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{REGISTRY}: the nine entries of T-074 Interface (5) do not each name exactly their \
         counterpart (AC-3):\n{}",
        problems.join("\n")
    );
}

/// AC-3, second half: no entry of the registry has `task = "T-074"`. Compared as an equality:
/// the list of entries with that task must be empty. Rejects: base (nine such entries); any
/// entry, of the nine or another, still or newly pending on T-074.
#[test]
fn ac3_no_entry_waits_on_t074() {
    let waiting: Vec<String> = entries()
        .iter()
        .filter(|e| e.get("task").map(String::as_str) == Some("T-074"))
        .map(show)
        .collect();
    assert_eq!(
        waiting,
        Vec::<String>::new(),
        "{REGISTRY}: entries with task = \"T-074\" remain (AC-3: none may)"
    );
}
