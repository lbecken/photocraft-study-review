//! T-092 acceptance tests (test author), written from `tasks/T-092-mark-product-tests-for-windows.md`
//! (Interface (1), Interface (2), AC-1) alone. Every test here fails on base.
//!
//! AC-1, the registry (`xtask/platform-tests.toml`, parsed with `toml`: any TOML spelling of the
//! same array of tables passes; values are compared as TOML strings, another type is a mismatch):
//! 1. The entries whose `file` is one of the 9 test files of `touches` are, as the multiset of
//!    `(file, item, platforms, counterpart)`, exactly Interface (2)'s 12 (an equality over an
//!    exact list: an extra entry, a missing one, one entered twice or one field changed fails).
//!    Entries of other files are not this test's matter, so a later task's entries elsewhere do
//!    not break it (D-249).
//! 2. The two `disk.rs` entries that were pending on T-092 each exist exactly once, keep
//!    `platforms = "unix"`, name exactly W40's / W41's test and have no `reason`, `task`,
//!    `expires` key.
//! 3. No entry has `task = "T-092"` (the list of such entries equals the empty list).
//!
//! What the registry names must be in the sources (Interface (1); read with `syn`, as
//! `guard_t079.rs` reads test names; `cfg!(…)` expressions are not attributes and are not read):
//! 4. In the 9 files the tests behind a platform cfg attribute (on the fn, an inline `mod`
//!    around it, the file's `#![cfg]` or a `mod` declaration of the file) are exactly the 13 of
//!    the U rows and the three `_unix` halves of the splits, each behind a cfg meaning Linux and
//!    macOS (`unix`, `not(windows)`, …) at exactly the place its registry item names: the fn for
//!    a test entry, the file for an `item = "<file>"` entry.
//! 5. W40 and W41 (the counterparts of entry 2) exist each as exactly one `#[test]` fn of
//!    `disk.rs`'s `mod tests` with no platform cfg attribute.
//!
//! Not pinned (left to `no-skip` and the reviewer): `doc`, `quote`, `reason` of the 12, the order
//! of entries, comments, entries of other files. Only `std::fs`, `toml` and `syn` are used (the
//! same on Linux, macOS and Windows); paths are `/`-separated repository-relative strings. No
//! process is spawned.
#![cfg(test)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const REGISTRY: &str = "xtask/platform-tests.toml";
const DISK: &str = "crates/kiln-workspace/src/disk.rs";
const WS: &str = "crates/kiln-workspace";

const T002: &str = "crates/kiln-workspace/tests/acceptance_t002.rs";
const T002_R2: &str = "crates/kiln-workspace/tests/acceptance_t002_r2.rs";
const T008: &str = "crates/kiln-workspace/tests/acceptance_t008.rs";
const T015_TOOLS: &str = "crates/kiln-workspace/tests/acceptance_t015_tools.rs";
const T036_WS: &str = "crates/kiln-workspace/tests/acceptance_t036_workspace.rs";
const CONFINEMENT: &str = "crates/kiln-workspace/tests/confinement.rs";
const G008: &str = "crates/kiln-workspace/tests/guard_t008.rs";
const G036_REMOVAL: &str = "crates/kiln-workspace/tests/guard_t036_removal_workspace.rs";
const G036_WS: &str = "crates/kiln-workspace/tests/guard_t036_workspace.rs";

/// The 9 test files of `touches` (after the cut, D-273).
const NINE_FILES: [&str; 9] = [
    T002,
    T002_R2,
    T008,
    T015_TOOLS,
    T036_WS,
    CONFINEMENT,
    G008,
    G036_REMOVAL,
    G036_WS,
];

const ACW2_READ_ONLY: &str = "crates/kiln-workspace/tests/guard_t073.rs::acw2_save_of_read_only_target_is_refused_and_changes_nothing";
const ACW2_PARENT: &str =
    "crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root";

/// Interface (2): the 12 entries of the 9 files, `(file, item, platforms, counterpart)`.
const TWELVE: [(&str, &str, &str, &str); 12] = [
    // W10
    (
        T002_R2,
        "saved_file_is_never_more_readable_than_the_target",
        "unix",
        "none",
    ),
    // W11
    (
        T002_R2,
        "save_to_unwritable_target_is_permission_denied_and_untouched",
        "unix",
        ACW2_READ_ONLY,
    ),
    // W15
    (
        T008,
        "non_regular_files_are_refused_without_blocking",
        "unix",
        "none",
    ),
    // W16
    (
        T008,
        "save_onto_non_regular_files_leaves_them_alone",
        "unix",
        "none",
    ),
    // W17 `_unix`
    (
        T008,
        "outside_root_wins_over_not_found_unix",
        "unix",
        "crates/kiln-workspace/tests/acceptance_t008.rs::outside_root_wins_over_not_found",
    ),
    // W19
    (
        T008,
        "parent_swapped_between_requests_is_refused",
        "unix",
        ACW2_PARENT,
    ),
    // W24 (`<file>`, spec check M1)
    (T036_WS, "<file>", "unix", "none"),
    // W27
    (
        CONFINEMENT,
        "non_regular_files_are_refused_without_blocking",
        "unix",
        "none",
    ),
    // W28 `_unix`
    (
        CONFINEMENT,
        "swaps_between_open_and_save_are_checked_again_unix",
        "unix",
        "crates/kiln-workspace/tests/confinement.rs::swaps_between_open_and_save_are_checked_again",
    ),
    // W30
    (
        CONFINEMENT,
        "root_that_cannot_be_opened_for_reading_is_refused_by_new",
        "unix",
        "none",
    ),
    // W32 + W33 (`<file>`)
    (
        G036_REMOVAL,
        "<file>",
        "unix",
        "crates/kiln-workspace/src/disk.rs::tests::root_removed_on_every_platform",
    ),
    // W35 `_unix`
    (
        G036_WS,
        "vanished_root_is_refused_unix",
        "unix",
        ACW2_PARENT,
    ),
];

/// W40 and W41: the new portable tests of `disk.rs`'s test module.
const W40: &str = "tests::leftover_at_temp_name_is_never_touched_on_every_platform";
const W41: &str = "tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged_on_every_platform";

/// Interface (2): the two `disk.rs` entries pending on T-092 at base, and the counterpart each
/// must name after this task.
const TWO: [(&str, &str); 2] = [
    ("tests::leftover_at_temp_name_is_never_touched", W40),
    (
        "tests::failed_save_after_writability_check_leaves_target_and_listing_unchanged",
        W41,
    ),
];

/// The keys an entry with a named counterpart must not have (Interface (2)).
const FORBIDDEN: [&str; 3] = ["reason", "task", "expires"];

// ---- the registry -------------------------------------------------------------------------

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

/// Every `[[test]]` entry of the registry, as key → value (a string as itself, any other TOML
/// value as `<not a string: …>`, which matches no expected value).
fn entries() -> Vec<BTreeMap<String, String>> {
    let p = root().join(REGISTRY);
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", p.display()));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("{REGISTRY} is not TOML: {e}"));
    let Some(tests) = table.get("test") else {
        return Vec::new();
    };
    tests
        .as_array()
        .unwrap_or_else(|| panic!("`test` in {REGISTRY} is not an array of tables"))
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

fn field(e: &BTreeMap<String, String>, k: &str) -> String {
    e.get(k).cloned().unwrap_or_else(|| "<absent>".to_owned())
}

fn show(e: &BTreeMap<String, String>) -> String {
    e.iter()
        .filter(|(k, _)| !matches!(k.as_str(), "doc" | "quote" | "reason"))
        .map(|(k, v)| format!("{k} = {v:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// AC-1, first part: the entries of the 9 files equal Interface (2)'s 12, as a multiset of
/// `(file, item, platforms, counterpart)`. Rejects (besides base, where none of the 12 exists):
/// `confinement.rs::walk_corner_cases` registered `unix` (a P row registered); W19 with
/// `counterpart = "none"`; W24 as the test-level item `unreadable_root_is_refused_by_new`
/// instead of `<file>`; W17's `_unix` naming itself; an entry entered twice; one left out.
#[test]
fn ac1_entries_of_the_nine_files_are_exactly_the_twelve() {
    let mut actual: Vec<(String, String, String, String)> = entries()
        .iter()
        .filter(|e| {
            e.get("file")
                .is_some_and(|f| NINE_FILES.contains(&f.as_str()))
        })
        .map(|e| {
            (
                field(e, "file"),
                field(e, "item"),
                field(e, "platforms"),
                field(e, "counterpart"),
            )
        })
        .collect();
    actual.sort();
    let mut expected: Vec<(String, String, String, String)> = TWELVE
        .iter()
        .map(|(f, i, p, c)| {
            (
                (*f).to_owned(),
                (*i).to_owned(),
                (*p).to_owned(),
                (*c).to_owned(),
            )
        })
        .collect();
    expected.sort();
    let mut missing = expected.clone();
    let mut extra = Vec::new();
    for a in &actual {
        match missing.iter().position(|x| x == a) {
            Some(i) => {
                missing.remove(i);
            }
            None => extra.push(a.clone()),
        }
    }
    let fmt = |v: &[(String, String, String, String)]| {
        v.iter()
            .map(|(f, i, p, c)| {
                format!("  file = {f:?}, item = {i:?}, platforms = {p:?}, counterpart = {c:?}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(
        missing.is_empty() && extra.is_empty(),
        "{REGISTRY}: the entries of the 9 test files of T-092 are not exactly Interface (2)'s 12 \
         (AC-1); {} of the 12 missing or changed:\n{}\n{} not among the 12 (unexpected, changed or \
         entered twice):\n{}",
        missing.len(),
        fmt(&missing),
        extra.len(),
        fmt(&extra)
    );
}

/// AC-1, second part: the two `disk.rs` entries name exactly W40 and W41. Rejects (besides
/// base's `pending`/`T-092`): W41's entry naming W40's test; a counterpart without the
/// `crates/kiln-workspace/src/disk.rs::` prefix; `reason` or `expires` kept; `platforms`
/// changed; an entry dropped or entered twice.
#[test]
fn ac1_the_two_disk_entries_name_w40_and_w41() {
    let all = entries();
    let mut problems = Vec::new();
    for (item, test) in TWO {
        let counterpart = format!("{DISK}::{test}");
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
        if one.get("counterpart") != Some(&counterpart) {
            wrong.push(format!(
                "counterpart = {:?}, expected {counterpart:?}",
                one.get("counterpart")
            ));
        }
        for k in FORBIDDEN {
            if one.contains_key(k) {
                wrong.push(format!("has `{k}`, expected no `{k}`"));
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
        "{REGISTRY}: the two disk.rs entries pending on T-092 do not name exactly W40 and W41 \
         (AC-1):\n{}",
        problems.join("\n")
    );
}

/// AC-1, third part: no entry has `task = "T-092"`. Rejects: base (two such entries); any entry,
/// of the 14 or another, still or newly pending on T-092.
#[test]
fn ac1_no_entry_waits_on_t092() {
    let waiting: Vec<String> = entries()
        .iter()
        .filter(|e| e.get("task").map(String::as_str) == Some("T-092"))
        .map(show)
        .collect();
    assert_eq!(
        waiting,
        Vec::<String>::new(),
        "{REGISTRY}: entries with task = \"T-092\" remain (AC-1: none may)"
    );
}

// ---- the sources --------------------------------------------------------------------------

/// The Linux/macOS/Windows set a cfg predicate holds on, as bits (1 Linux, 2 macOS, 4 Windows);
/// `Err` for a predicate this reader does not know (it is then reported, never guessed).
fn platforms_of(m: &syn::Meta) -> Result<u8, String> {
    const ALL: u8 = 7;
    let text = |m: &syn::Meta| match m {
        syn::Meta::Path(p) => p.get_ident().map(ToString::to_string).unwrap_or_default(),
        syn::Meta::NameValue(nv) => format!(
            "{} = …",
            nv.path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default()
        ),
        syn::Meta::List(l) => format!(
            "{}(…)",
            l.path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default()
        ),
    };
    match m {
        syn::Meta::Path(p) if p.is_ident("test") => Ok(ALL),
        syn::Meta::Path(p) if p.is_ident("unix") => Ok(1 | 2),
        syn::Meta::Path(p) if p.is_ident("windows") => Ok(4),
        syn::Meta::NameValue(nv) => {
            let v = match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => s.value(),
                _ => return Err(text(m)),
            };
            let key = nv
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();
            match (key.as_str(), v.as_str()) {
                ("target_os", "linux") => Ok(1),
                ("target_os", "macos") => Ok(2),
                ("target_os", "windows") => Ok(4),
                ("target_family", "unix") => Ok(1 | 2),
                ("target_family", "windows") => Ok(4),
                _ => Err(format!("{key} = {v:?}")),
            }
        }
        syn::Meta::List(l) => {
            let args: Vec<syn::Meta> = nested(l);
            if l.path.is_ident("not") && args.len() == 1 {
                Ok(ALL & !platforms_of(&args[0])?)
            } else if l.path.is_ident("all") {
                args.iter().try_fold(ALL, |a, m| Ok(a & platforms_of(m)?))
            } else if l.path.is_ident("any") {
                args.iter().try_fold(0, |a, m| Ok(a | platforms_of(m)?))
            } else {
                Err(text(m))
            }
        }
        _ => Err(text(m)),
    }
}

fn nested(l: &syn::MetaList) -> Vec<syn::Meta> {
    l.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .unwrap_or_else(|e| panic!("precondition: unreadable attribute arguments: {e}"))
        .into_iter()
        .collect()
}

/// Whether a cfg predicate names a platform (`unix`, `windows`, a `target_*` key).
fn is_platform(m: &syn::Meta) -> bool {
    let named = |p: &syn::Path| {
        p.is_ident("unix")
            || p.is_ident("windows")
            || p.get_ident()
                .is_some_and(|i| i.to_string().starts_with("target_"))
    };
    match m {
        syn::Meta::Path(p) => named(p),
        syn::Meta::NameValue(nv) => named(&nv.path),
        syn::Meta::List(l) => named(&l.path) || nested(l).iter().any(is_platform),
    }
}

fn set_name(bits: Result<u8, String>) -> String {
    match bits {
        Ok(3) => "unix".to_owned(),
        Ok(4) => "windows".to_owned(),
        Ok(1) => "linux".to_owned(),
        Ok(2) => "macos".to_owned(),
        Ok(7) => "every platform".to_owned(),
        Ok(b) => format!("platform set {b:#05b}"),
        Err(t) => format!("unknown predicate {t}"),
    }
}

/// The platform cfg attributes among `attrs`, each as `<at>=<set>` (`#[cfg(p)]` by the set `p`
/// holds on; any `cfg_attr` naming a platform in its predicate or what it applies as
/// `<at>=cfg_attr`, a form no row uses).
fn platform_cfgs(attrs: &[syn::Attribute], at: &str, out: &mut Vec<String>) {
    for a in attrs {
        let syn::Meta::List(l) = &a.meta else {
            continue;
        };
        if a.path().is_ident("cfg") {
            if let Some(p) = nested(l).into_iter().next()
                && is_platform(&p)
            {
                out.push(format!("{at}={}", set_name(platforms_of(&p))));
            }
        } else if a.path().is_ident("cfg_attr") && nested(l).iter().any(is_platform) {
            out.push(format!("{at}=cfg_attr({})", l.tokens));
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|_| panic!("{} is not under {}", path.display(), root.display()))
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every `.rs` file under `crates/kiln-workspace` (the crate of every file read here).
fn crate_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.join(WS)];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", d.display()));
        for entry in entries {
            let p = entry.expect("a directory entry").path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "target") {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(rel(root, &p));
            }
        }
    }
    out.sort();
    out
}

fn parse(root: &Path, file: &str) -> syn::File {
    let p = root.join(file);
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", p.display()));
    syn::parse_file(&text).unwrap_or_else(|e| panic!("precondition: {file} does not parse: {e}"))
}

/// The attributes of every `mod name;` declaration in the crate that may load `file` (both
/// directory conventions and `#[path]`; an over-approximation only adds findings).
fn declaration_attrs(root: &Path, file: &str) -> Vec<syn::Attribute> {
    let mut out = Vec::new();
    for f in crate_files(root) {
        let dir = f.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
        let stem = f
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".rs")
            .to_owned();
        for item in parse(root, &f).items {
            let syn::Item::Mod(m) = item else { continue };
            if m.content.is_some() {
                continue;
            }
            let name = m.ident.to_string();
            let mut candidates = vec![
                format!("{dir}/{name}.rs"),
                format!("{dir}/{name}/mod.rs"),
                format!("{dir}/{stem}/{name}.rs"),
                format!("{dir}/{stem}/{name}/mod.rs"),
            ];
            for a in &m.attrs {
                if a.path().is_ident("path")
                    && let syn::Meta::NameValue(nv) = &a.meta
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                {
                    candidates = vec![format!("{dir}/{}", s.value().replace('\\', "/"))];
                }
            }
            if candidates.iter().any(|c| c == file) {
                out.extend(m.attrs.iter().cloned());
            }
        }
    }
    out
}

fn is_test_attr(a: &syn::Attribute) -> bool {
    a.path().segments.last().is_some_and(|s| s.ident == "test")
}

/// Every `#[test]` fn of `file` (any inline module depth) as `<mod path>::<fn>` → the platform
/// cfgs bearing on it (`file=…` for the file's `#![cfg]` or its `mod` declaration, `mod <m>=…`,
/// `fn=…`). A name defined twice is listed with `#2` appended to the second.
fn tests_of(root: &Path, file: &str) -> BTreeMap<String, Vec<String>> {
    fn walk(
        items: &[syn::Item],
        prefix: &str,
        outer: &[String],
        out: &mut BTreeMap<String, Vec<String>>,
    ) {
        for item in items {
            match item {
                syn::Item::Fn(f) if f.attrs.iter().any(is_test_attr) => {
                    let mut cfgs = outer.to_vec();
                    platform_cfgs(&f.attrs, "fn", &mut cfgs);
                    let mut name = format!("{prefix}{}", f.sig.ident);
                    if out.contains_key(&name) {
                        name.push_str("#2");
                    }
                    out.insert(name, cfgs);
                }
                syn::Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        let mut cfgs = outer.to_vec();
                        platform_cfgs(&m.attrs, &format!("mod {}", m.ident), &mut cfgs);
                        walk(inner, &format!("{prefix}{}::", m.ident), &cfgs, out);
                    }
                }
                _ => {}
            }
        }
    }
    let parsed = parse(root, file);
    let mut outer = Vec::new();
    platform_cfgs(&declaration_attrs(root, file), "file", &mut outer);
    platform_cfgs(&parsed.attrs, "file", &mut outer);
    let mut out = BTreeMap::new();
    walk(&parsed.items, "", &outer, &mut out);
    out
}

/// Interface (1): the Unix tests of the 9 files after this task, `(file, test, place)`: `fn` for
/// a test entry (U rows and `_unix` halves), `file` for the `<file>` entries (W24, W32, W33).
const UNIX_TESTS: [(&str, &str, &str); 13] = [
    (
        T002_R2,
        "saved_file_is_never_more_readable_than_the_target",
        "fn",
    ),
    (
        T002_R2,
        "save_to_unwritable_target_is_permission_denied_and_untouched",
        "fn",
    ),
    (T008, "non_regular_files_are_refused_without_blocking", "fn"),
    (T008, "save_onto_non_regular_files_leaves_them_alone", "fn"),
    (T008, "outside_root_wins_over_not_found_unix", "fn"),
    (T008, "parent_swapped_between_requests_is_refused", "fn"),
    (T036_WS, "unreadable_root_is_refused_by_new", "file"),
    (
        CONFINEMENT,
        "non_regular_files_are_refused_without_blocking",
        "fn",
    ),
    (
        CONFINEMENT,
        "swaps_between_open_and_save_are_checked_again_unix",
        "fn",
    ),
    (
        CONFINEMENT,
        "root_that_cannot_be_opened_for_reading_is_refused_by_new",
        "fn",
    ),
    (
        G036_REMOVAL,
        "root_removed_and_replaced_by_link_is_refused",
        "file",
    ),
    (
        G036_REMOVAL,
        "root_removed_and_replaced_by_new_directory_is_refused",
        "file",
    ),
    (G036_WS, "vanished_root_is_refused_unix", "fn"),
];

/// Interface (1) and (2) in the sources: in the 9 files, the tests behind a platform cfg are
/// exactly [`UNIX_TESTS`], each behind exactly one cfg meaning Linux and macOS, at its place.
/// Rejects (besides base, where none is): `#[cfg(unix)]` on a P test such as
/// `guard_t008.rs::symlinks_inside_root_still_work` or on `acceptance_t002.rs`'s
/// `relpath_accepts_and_rejects_exact_cases` (not a row: unchanged, portable); W24's test given
/// a test-level `#[cfg(unix)]` instead of the file's; a U test behind `target_os = "linux"`
/// (the registry says `unix`); a U test left portable; a `_unix` half missing or renamed; a
/// Windows-only test added to these files.
#[test]
fn ac1_unix_only_tests_of_the_nine_files_are_exactly_the_listed_ones() {
    let root = root();
    let mut actual = BTreeMap::new();
    for file in NINE_FILES {
        for (name, cfgs) in tests_of(&root, file) {
            if !cfgs.is_empty() {
                actual.insert(format!("{file}::{name}"), cfgs.join(", "));
            }
        }
    }
    let expected: BTreeMap<String, String> = UNIX_TESTS
        .iter()
        .map(|(f, t, at)| (format!("{f}::{t}"), format!("{at}=unix")))
        .collect();
    let show = |m: &BTreeMap<String, String>| {
        m.iter()
            .map(|(k, v)| format!("  {k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(
        actual == expected,
        "the tests behind a platform cfg in T-092's 9 files are not exactly Interface (1)'s Unix \
         tests at their places (AC-1's entries name them);\nfound ({}):\n{}\nexpected ({}):\n{}",
        actual.len(),
        show(&actual),
        expected.len(),
        show(&expected)
    );
}

/// The counterparts named by the two `disk.rs` entries exist: W40 and W41 each exactly one
/// `#[test]` fn of `disk.rs`'s `mod tests`, with no platform cfg attribute on it, `mod tests`
/// or the file. Rejects (besides base, where neither exists): W41 written under another name;
/// either behind `#[cfg(windows)]` or `#[cfg(unix)]`; either defined twice.
#[test]
fn ac1_w40_and_w41_exist_once_in_disk_tests_without_platform_cfg() {
    let root = root();
    let tests = tests_of(&root, DISK);
    let mut problems = Vec::new();
    for name in [W40, W41] {
        match tests.get(name) {
            None => problems.push(format!("{DISK}::{name}: no such #[test] fn")),
            Some(cfgs) if !cfgs.is_empty() => problems.push(format!(
                "{DISK}::{name}: behind a platform cfg: {}",
                cfgs.join(", ")
            )),
            Some(_) => {}
        }
        if tests.contains_key(&format!("{name}#2")) {
            problems.push(format!("{DISK}::{name}: defined more than once"));
        }
    }
    assert!(
        problems.is_empty(),
        "W40/W41 (Interface (1)), the counterparts of the two disk.rs entries, are not each one \
         portable #[test] fn:\n{}",
        problems.join("\n")
    );
}
