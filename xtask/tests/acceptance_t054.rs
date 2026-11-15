//! T-054 acceptance tests (test author): AC-4, the change to the root `Cargo.toml` is exactly
//! the two profile tables of the Interface.
//!
//! The root manifest is parsed with `toml` (Rust API used by tests) and the tests assert, as AC-4
//! states them: `profile.dev.debug` is `"line-tables-only"`, `profile.dev.incremental` is `false`,
//! `profile.dev.package."*".debug` is `false`; `profile.release` and `profile.bench` are absent;
//! `profile.dev` sets no `opt-level`, `debug-assertions` or `overflow-checks`. Nothing else of
//! the file is read (no whole-file comparison, no key order, no comment text).
//!
//! The values are compared as the TOML values the Interface writes (a string and two booleans):
//! AC-4 pins those values, so another spelling Cargo would also accept (`debug = 0` or `"none"`
//! for `false`) is not the specified table and fails here.
//!
//! On base the root `Cargo.toml` has no `[profile.*]` table, so every test fails by assertion.
//! No Unix-only API is used: one file is read with `std::fs` (runs on Linux, macOS and Windows).
#![cfg(test)]

use std::path::{Path, PathBuf};

/// The repository root (the parent of the `xtask` package).
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// The root `Cargo.toml`, parsed. Unreadable or not TOML is a precondition failure.
fn manifest() -> toml::Table {
    let p = root().join("Cargo.toml");
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", p.display()));
    text.parse::<toml::Table>()
        .unwrap_or_else(|e| panic!("precondition: the root Cargo.toml is not TOML: {e}"))
}

/// `[profile]` of the root manifest; absent is an assertion failure (T-054 adds it).
fn profile(m: &toml::Table) -> &toml::Table {
    m.get("profile")
        .unwrap_or_else(|| {
            panic!("the root Cargo.toml has no [profile.*] table (T-054 adds [profile.dev])")
        })
        .as_table()
        .unwrap_or_else(|| panic!("`profile` in the root Cargo.toml is not a table"))
}

/// `[profile.dev]` of the root manifest; absent is an assertion failure.
fn dev(m: &toml::Table) -> &toml::Table {
    profile(m)
        .get("dev")
        .unwrap_or_else(|| panic!("the root Cargo.toml has no [profile.dev] table"))
        .as_table()
        .unwrap_or_else(|| panic!("`profile.dev` in the root Cargo.toml is not a table"))
}

#[test]
fn ac4_dev_debug_is_line_tables_only() {
    let m = manifest();
    let v = dev(&m).get("debug");
    assert_eq!(
        v,
        Some(&toml::Value::String("line-tables-only".to_owned())),
        "profile.dev.debug in the root Cargo.toml"
    );
}

#[test]
fn ac4_dev_incremental_is_false() {
    let m = manifest();
    let v = dev(&m).get("incremental");
    assert_eq!(
        v,
        Some(&toml::Value::Boolean(false)),
        "profile.dev.incremental in the root Cargo.toml"
    );
}

#[test]
fn ac4_dev_dependencies_debug_is_false() {
    let m = manifest();
    let star = dev(&m)
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|p| p.get("*"))
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("the root Cargo.toml has no [profile.dev.package.\"*\"] table"));
    assert_eq!(
        star.get("debug"),
        Some(&toml::Value::Boolean(false)),
        "profile.dev.package.\"*\".debug in the root Cargo.toml"
    );
}

#[test]
fn ac4_dev_sets_no_opt_level_debug_assertions_or_overflow_checks() {
    let m = manifest();
    let d = dev(&m);
    for key in ["opt-level", "debug-assertions", "overflow-checks"] {
        assert!(
            !d.contains_key(key),
            "profile.dev sets `{key}` (AC-4: left at the dev default): {:?}",
            d.get(key)
        );
    }
}

#[test]
fn ac4_release_and_bench_profiles_are_absent() {
    let m = manifest();
    // The [profile] table T-054 adds must be there (else this would hold on any manifest).
    let p = profile(&m);
    assert!(
        p.contains_key("dev"),
        "the root Cargo.toml has no [profile.dev] table"
    );
    for name in ["release", "bench"] {
        assert!(
            !p.contains_key(name),
            "the root Cargo.toml has a [profile.{name}] table (AC-4: not written)"
        );
    }
}
