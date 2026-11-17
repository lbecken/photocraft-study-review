//! `cargo xtask no-skip`: no test may be ignored, skipped or compiled out, unless allowlisted in
//! `xtask/skips.toml` with a reason and an unexpired date.
//!
//! Detected (by parsing every `.rs` file with `syn`):
//! * `#[ignore]` on any item, and `#[cfg_attr(…, ignore)]`;
//! * in a test function: a call or macro whose name starts with `skip`, and a `return` inside a
//!   conditional, loop or `let … else` (an early exit that makes the test pass without running);
//!   closures and nested functions are not inspected;
//! * a `#[cfg(…)]` other than exactly `cfg(test)` or a platform form (`crate::platforms`) on a
//!   test function, on a module that contains tests (inline, or the file it loads), or as an
//!   inner `#![cfg(…)]` of a file with tests;
//! * a doc-test fence marked `ignore`.
//!
//! Then the platform-test rule (METHODOLOGY §8.1): a test, test module or file behind a platform
//! cfg must be listed in `xtask/platform-tests.toml` (`crate::platforms`), a helper defined per
//! platform must not do nothing on one of them, and a platform statement in a test needs one for
//! the other platforms beside it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ItemFn, ItemMod, Meta};

use crate::platforms::{self, HelperDef, ItemCfg, Located, PlatformItem, Set, TestIndex};
use crate::util::{StepResult, rel, rust_files};

/// What kind of skip was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// `#[ignore]`.
    Ignore,
    /// `#[cfg_attr(…, ignore)]`.
    CfgAttrIgnore,
    /// A `skip*` call or macro in a test.
    SkipCall,
    /// A conditional `return` in a test.
    EarlyReturn,
    /// A test or test module behind a `cfg` other than `test` or a platform form.
    CfgDisabled,
    /// A ```` ```ignore ```` doc test.
    DocIgnore,
}

impl Kind {
    /// The name used in `skips.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Ignore => "ignore",
            Kind::CfgAttrIgnore => "cfg-attr-ignore",
            Kind::SkipCall => "skip-call",
            Kind::EarlyReturn => "early-return",
            Kind::CfgDisabled => "cfg-disabled",
            Kind::DocIgnore => "doc-ignore",
        }
    }
}

/// One finding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// Item path within the file (`tests::name`), or `<file>` for file-level findings.
    pub item: String,
    /// 1-based line.
    pub line: usize,
    /// Kind.
    pub kind: Kind,
}

fn path_last(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

fn is_test_attr(a: &Attribute) -> bool {
    path_last(a.path()) == "test" && !a.path().is_ident("cfg")
}

fn is_test_fn(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

fn line_of(span: proc_macro2::Span) -> usize {
    span.start().line
}

fn contains_tests(items: &[syn::Item]) -> bool {
    struct Has(bool);
    impl<'a> Visit<'a> for Has {
        fn visit_item_fn(&mut self, f: &'a ItemFn) {
            if is_test_fn(&f.attrs) {
                self.0 = true;
            }
            visit::visit_item_fn(self, f);
        }
    }
    let mut h = Has(false);
    for i in items {
        h.visit_item(i);
    }
    h.0
}

/// Inspects a test function body.
struct Body<'f> {
    item: String,
    cond_depth: usize,
    out: &'f mut Vec<Finding>,
}

impl<'a> Visit<'a> for Body<'_> {
    fn visit_expr_closure(&mut self, _: &'a syn::ExprClosure) {}
    fn visit_item(&mut self, _: &'a syn::Item) {}
    fn visit_expr_if(&mut self, e: &'a syn::ExprIf) {
        self.cond_depth += 1;
        visit::visit_expr_if(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_match(&mut self, e: &'a syn::ExprMatch) {
        self.cond_depth += 1;
        visit::visit_expr_match(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_while(&mut self, e: &'a syn::ExprWhile) {
        self.cond_depth += 1;
        visit::visit_expr_while(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_for_loop(&mut self, e: &'a syn::ExprForLoop) {
        self.cond_depth += 1;
        visit::visit_expr_for_loop(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_loop(&mut self, e: &'a syn::ExprLoop) {
        self.cond_depth += 1;
        visit::visit_expr_loop(self, e);
        self.cond_depth -= 1;
    }
    fn visit_local(&mut self, l: &'a syn::Local) {
        if let Some(init) = &l.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.cond_depth += 1;
                self.visit_expr(diverge);
                self.cond_depth -= 1;
            }
        }
    }
    fn visit_expr_return(&mut self, r: &'a syn::ExprReturn) {
        if self.cond_depth > 0 {
            self.out.push(Finding {
                item: self.item.clone(),
                line: line_of(r.return_token.span),
                kind: Kind::EarlyReturn,
            });
        }
        visit::visit_expr_return(self, r);
    }
    fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
        if let Expr::Path(p) = &*c.func
            && path_last(&p.path).starts_with("skip")
        {
            self.out.push(Finding {
                item: self.item.clone(),
                line: line_of(p.path.segments[0].ident.span()),
                kind: Kind::SkipCall,
            });
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_macro(&mut self, m: &'a syn::Macro) {
        if path_last(&m.path).starts_with("skip") {
            self.out.push(Finding {
                item: self.item.clone(),
                line: line_of(m.path.segments[0].ident.span()),
                kind: Kind::SkipCall,
            });
        }
    }
}

/// Walks one file's items.
struct FileScan<'f> {
    file: PathBuf,
    module: Vec<String>,
    /// Platforms the enclosing file and modules exist on.
    env: Set,
    out: &'f mut Vec<Finding>,
    side: &'f mut Side,
}

/// What the scan collects for the platform-test rule.
#[derive(Debug, Default)]
struct Side {
    platform: Vec<(String, usize, Set)>,
    tests: Vec<(String, Set)>,
    helpers: Vec<HelperDef>,
    located: Vec<(String, usize, platforms::Kind)>,
    modules: Vec<(PathBuf, Set)>,
}

impl FileScan<'_> {
    fn name(&self, leaf: &str) -> String {
        let mut p = self.module.clone();
        p.push(leaf.to_owned());
        p.join("::")
    }

    fn check_attrs(&mut self, item: &str, attrs: &[Attribute]) {
        for a in attrs {
            if a.path().is_ident("ignore") {
                self.out.push(Finding {
                    item: item.into(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::Ignore,
                });
            }
            if a.path().is_ident("cfg_attr")
                && let Meta::List(l) = &a.meta
                && l.tokens
                    .clone()
                    .into_iter()
                    .any(|t| matches!(t, proc_macro2::TokenTree::Ident(i) if i == "ignore"))
            {
                self.out.push(Finding {
                    item: item.into(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::CfgAttrIgnore,
                });
            }
            if a.path().is_ident("doc")
                && let Meta::NameValue(nv) = &a.meta
                && let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                && s.value().lines().any(|l| {
                    let t = l.trim_start();
                    t.strip_prefix("```")
                        .is_some_and(|info| info.split([',', ' ']).any(|w| w.trim() == "ignore"))
                })
            {
                self.out.push(Finding {
                    item: item.into(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::DocIgnore,
                });
            }
        }
    }

    fn module_file(&self, m: &ItemMod) -> Option<PathBuf> {
        let dir = self.file.parent()?;
        let stem = self.file.file_stem()?.to_str()?;
        // `lib.rs`/`main.rs`/`mod.rs` load children from their own directory; `foo.rs` from `foo/`.
        let base = if matches!(stem, "lib" | "main" | "mod") {
            dir.to_path_buf()
        } else {
            dir.join(stem)
        };
        let mut nested = base.clone();
        for m in &self.module {
            nested.push(m);
        }
        let name = m.ident.to_string();
        [
            nested.join(format!("{name}.rs")),
            nested.join(&name).join("mod.rs"),
        ]
        .into_iter()
        .find(|p| p.is_file())
    }
}

impl FileScan<'_> {
    /// The item's own cfg: a non-platform cfg is a `cfg-disabled` skip; a platform cfg makes it
    /// a platform item. Returns the platforms it exists on (with the enclosing ones).
    fn classify_test_item(&mut self, item: &str, attrs: &[Attribute]) -> Set {
        let cfg = platforms::item_cfg(attrs);
        match cfg {
            ItemCfg::Other(line) => self.out.push(Finding {
                item: item.to_owned(),
                line,
                kind: Kind::CfgDisabled,
            }),
            ItemCfg::Platform(set, line) => self.side.platform.push((item.to_owned(), line, set)),
            ItemCfg::Unconditional => {}
        }
        self.env.inter(cfg.set())
    }

    fn helper(&mut self, item: String, attrs: &[Attribute], block: &syn::Block) {
        if let ItemCfg::Platform(_, line) = platforms::item_cfg(attrs) {
            self.side.helpers.push(HelperDef {
                item,
                line,
                trivial: platforms::is_trivial_body(block),
            });
        }
    }
}

impl<'a> Visit<'a> for FileScan<'_> {
    fn visit_item_fn(&mut self, f: &'a ItemFn) {
        let item = self.name(&f.sig.ident.to_string());
        self.check_attrs(&item, &f.attrs);
        if is_test_fn(&f.attrs) {
            let set = self.classify_test_item(&item, &f.attrs);
            self.side.tests.push((item.clone(), set));
            for line in platforms::unpaired_statements(&f.block) {
                self.side
                    .located
                    .push((item.clone(), line, platforms::Kind::PlatformStatement));
            }
            let mut body = Body {
                item,
                cond_depth: 0,
                out: self.out,
            };
            for stmt in &f.block.stmts {
                body.visit_stmt(stmt);
            }
        } else {
            self.helper(item, &f.attrs, &f.block);
        }
        // Nested fns inside a fn are rare in tests; not descended.
    }

    fn visit_item_mod(&mut self, m: &'a ItemMod) {
        let name = m.ident.to_string();
        let item = self.name(&name);
        self.check_attrs(&item, &m.attrs);
        let file = match &m.content {
            Some(_) => None,
            None => self.module_file(m),
        };
        let has_tests = match &m.content {
            Some((_, items)) => contains_tests(items),
            None => file.as_ref().is_some_and(|p| {
                std::fs::read_to_string(p)
                    .ok()
                    .and_then(|s| syn::parse_file(&s).ok())
                    .is_some_and(|f| contains_tests(&f.items))
            }),
        };
        let set = if has_tests {
            let set = self.classify_test_item(&item, &m.attrs);
            self.side.tests.push((item.clone(), set));
            set
        } else {
            self.env.inter(platforms::item_cfg(&m.attrs).set())
        };
        if let Some(p) = file {
            self.side.modules.push((p, set));
        }
        if m.content.is_some() {
            let env = self.env;
            self.env = set;
            self.module.push(name);
            visit::visit_item_mod(self, m);
            self.module.pop();
            self.env = env;
        }
    }

    fn visit_item(&mut self, i: &'a syn::Item) {
        match i {
            syn::Item::Fn(f) => self.visit_item_fn(f),
            syn::Item::Mod(m) => self.visit_item_mod(m),
            syn::Item::Impl(imp) => {
                // Twins of a method live in one impl block: the block's line names it.
                let block = line_of(imp.impl_token.span);
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(f) = ii {
                        let name = self.name(&f.sig.ident.to_string());
                        self.check_attrs(&name, &f.attrs);
                        let key = self.name(&format!("<impl@{block}>::{}", f.sig.ident));
                        self.helper(key, &f.attrs, &f.block);
                    }
                }
                self.check_attrs("<impl>", &imp.attrs);
            }
            other => {
                let attrs: &[Attribute] = match other {
                    syn::Item::Struct(s) => &s.attrs,
                    syn::Item::Enum(e) => &e.attrs,
                    syn::Item::Trait(t) => &t.attrs,
                    syn::Item::Const(c) => &c.attrs,
                    syn::Item::Static(s) => &s.attrs,
                    syn::Item::Type(t) => &t.attrs,
                    syn::Item::Macro(m) => &m.attrs,
                    syn::Item::Use(u) => &u.attrs,
                    _ => &[],
                };
                self.check_attrs(&self.name("<item>"), attrs);
            }
        }
    }
}

/// Everything the scan of one file yields.
#[derive(Debug, Default)]
pub struct Scan {
    /// The skip findings (the six kinds of `Kind`).
    pub findings: Vec<Finding>,
    /// Tests, test modules and `<file>` behind a platform cfg: (item, line, platforms).
    pub platform: Vec<(String, usize, Set)>,
    /// Every test fn, module with tests and `<file>` (if it has tests), with the platforms it
    /// exists on as far as this file says.
    pub tests: Vec<(String, Set)>,
    /// `inert-helper` and `platform-statement` findings: (item, line, kind).
    pub located: Vec<(String, usize, platforms::Kind)>,
    /// Out-of-line modules declared here: (the file they load, the platforms they exist on).
    pub modules: Vec<(PathBuf, Set)>,
}

/// Scan one source file.
pub fn scan(file: &Path, source: &str) -> Result<Scan> {
    let parsed = syn::parse_file(source).with_context(|| format!("parse {}", file.display()))?;
    let mut out = Vec::new();
    let mut side = Side::default();
    {
        let mut scan = FileScan {
            file: file.to_path_buf(),
            module: Vec::new(),
            env: Set::ALL,
            out: &mut out,
            side: &mut side,
        };
        scan.check_attrs("<file>", &parsed.attrs);
        if contains_tests(&parsed.items) {
            let set = scan.classify_test_item("<file>", &parsed.attrs);
            scan.side.tests.push(("<file>".into(), set));
            scan.env = set;
        } else {
            scan.env = platforms::item_cfg(&parsed.attrs).set();
        }
        for item in &parsed.items {
            scan.visit_item(item);
        }
    }
    out.sort();
    out.dedup();
    for (item, line) in platforms::inert_helpers(&side.helpers) {
        side.located
            .push((item, line, platforms::Kind::InertHelper));
    }
    side.located.sort_by_key(|(_, l, _)| *l);
    Ok(Scan {
        findings: out,
        platform: side.platform,
        tests: side.tests,
        located: side.located,
        modules: side.modules,
    })
}

/// Skip findings in one source file.
#[cfg(test)]
pub fn scan_file(file: &Path, source: &str) -> Result<Vec<Finding>> {
    Ok(scan(file, source)?.findings)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkipFile {
    #[serde(default)]
    skip: Vec<SkipEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkipEntry {
    file: String,
    item: String,
    kind: Option<String>,
    reason: String,
    expires: String,
}

fn valid_date(d: &str) -> bool {
    d.len() == 10 && crate::timefmt::parse_utc(d).is_ok()
}

/// Apply the allowlist: returns problems (unlisted findings, expired or unused or malformed
/// entries). `findings` are (file relative to root, finding).
pub fn apply_allowlist(
    findings: &[(String, Finding)],
    entries_toml: &str,
    today: &str,
) -> Result<Vec<String>> {
    let file: SkipFile = toml::from_str(entries_toml).context("parse xtask/skips.toml")?;
    let mut problems = Vec::new();
    let mut used = BTreeSet::new();
    for (i, e) in file.skip.iter().enumerate() {
        if e.reason.trim().is_empty() {
            problems.push(format!(
                "skips.toml entry {} ({}): empty reason",
                i + 1,
                e.item
            ));
        }
        if !valid_date(&e.expires) {
            problems.push(format!(
                "skips.toml entry {} ({}): expires `{}` is not YYYY-MM-DD",
                i + 1,
                e.item,
                e.expires
            ));
        } else if e.expires.as_str() < today {
            problems.push(format!(
                "skips.toml entry {} ({} in {}): expired on {} (today is {today})",
                i + 1,
                e.item,
                e.file,
                e.expires
            ));
        }
    }
    for (path, f) in findings {
        let hit = file.skip.iter().position(|e| {
            e.file == *path
                && e.item == f.item
                && e.kind.as_deref().is_none_or(|k| k == f.kind.name())
        });
        match hit {
            Some(i) => {
                used.insert(i);
            }
            None => problems.push(format!(
                "{path}:{}: `{}` is a skipped test ({}); remove the skip or list it in xtask/skips.toml with a reason and expiry",
                f.line,
                f.item,
                f.kind.name()
            )),
        }
    }
    for (i, e) in file.skip.iter().enumerate() {
        if !used.contains(&i) {
            problems.push(format!(
                "skips.toml entry {} ({} in {}) matches nothing; remove it",
                i + 1,
                e.item,
                e.file
            ));
        }
    }
    Ok(problems)
}

/// Directories scanned, relative to the root.
const SCANNED: &[&str] = &["crates", "apps", "xtask"];
/// Excluded: the self-test fixtures.
const EXCLUDED: &[&str] = &["xtask/fixtures"];

/// Known findings of `xtask/fixtures/no-skip/violations.rs` with the fixture registry
/// `platform-tests.toml` beside it: 9 skips and exactly one finding of each of the 8 platform
/// kinds.
const FIXTURE_FINDINGS: usize = 17;

/// The scan of a set of files, gathered for the platform rules.
#[derive(Default)]
struct Gathered {
    skips: Vec<(String, Finding)>,
    items: Vec<PlatformItem>,
    tests: TestIndex,
    located: Vec<Located>,
}

impl Gathered {
    fn add(&mut self, root: &Path, file: &Path, s: Scan, module_sets: &mut Vec<(String, Set)>) {
        let r = rel(root, file);
        for f in s.findings {
            self.skips.push((r.clone(), f));
        }
        for (item, line, platforms) in s.platform {
            self.items.push(PlatformItem {
                file: r.clone(),
                item,
                line,
                platforms,
            });
        }
        for (item, set) in s.tests {
            self.tests.insert((r.clone(), item), set);
        }
        for (item, line, kind) in s.located {
            self.located.push(Located {
                file: r.clone(),
                item,
                line,
                kind,
            });
        }
        for (p, set) in s.modules {
            module_sets.push((rel(root, &p), set));
        }
    }

    /// A file loaded by `#[cfg(…)] mod x;` exists only where the declaration does.
    fn narrow(&mut self, module_sets: &[(String, Set)]) {
        for (file, set) in module_sets {
            for ((f, _), s) in self.tests.iter_mut() {
                if f == file {
                    *s = s.inter(*set);
                }
            }
        }
    }

    fn platform_problems(
        &self,
        reg: &platforms::Registry,
        root: &Path,
        registry_name: &str,
        today: &str,
    ) -> Vec<String> {
        platforms::apply(
            reg,
            &self.items,
            &self.tests,
            &self.located,
            root,
            registry_name,
            today,
        )
    }
}

fn gather(root: &Path, files: &[PathBuf]) -> Result<Gathered> {
    let mut g = Gathered::default();
    let mut module_sets = Vec::new();
    for f in files {
        let src = std::fs::read_to_string(f).with_context(|| format!("read {}", f.display()))?;
        g.add(root, f, scan(f, &src)?, &mut module_sets);
    }
    g.narrow(&module_sets);
    Ok(g)
}

/// The self-test: the violations fixture yields exactly its known findings (skips and, with
/// the fixture registry, platform findings), the clean fixture none. Returns their number.
fn self_test(root: &Path, today: &str) -> Result<usize> {
    let fx = root.join("xtask/fixtures/no-skip");
    let bad_path = fx.join("violations.rs");
    let bad = gather(&fx, std::slice::from_ref(&bad_path))?;
    let kinds: BTreeSet<Kind> = bad.skips.iter().map(|(_, f)| f.kind).collect();
    let all = BTreeSet::from([
        Kind::Ignore,
        Kind::CfgAttrIgnore,
        Kind::SkipCall,
        Kind::EarlyReturn,
        Kind::CfgDisabled,
        Kind::DocIgnore,
    ]);
    if kinds != all || bad.skips.len() != 9 {
        bail!("self-test failed: violations fixture gave {:?}", bad.skips);
    }
    let reg_name = "xtask/fixtures/no-skip/platform-tests.toml";
    let reg_text =
        std::fs::read_to_string(root.join(reg_name)).with_context(|| format!("read {reg_name}"))?;
    let reg = platforms::parse(reg_name, &reg_text)
        .map_err(|e| anyhow::anyhow!("self-test failed: {}", e.join("; ")))?;
    let found = bad.platform_problems(&reg, &fx, reg_name, today);
    let want = [
        platforms::Kind::Unlisted,
        platforms::Kind::Mismatch,
        platforms::Kind::Unused,
        platforms::Kind::Sentence,
        platforms::Kind::Counterpart,
        platforms::Kind::PendingExpired,
        platforms::Kind::InertHelper,
        platforms::Kind::PlatformStatement,
    ];
    let missing: Vec<&str> = want
        .iter()
        .filter(|k| {
            found
                .iter()
                .filter(|l| l.contains(&format!(": {}: ", k.name())))
                .count()
                != 1
        })
        .map(|k| k.name())
        .collect();
    if !missing.is_empty() || bad.skips.len() + found.len() != FIXTURE_FINDINGS {
        bail!(
            "self-test failed: violations fixture gave {} skip(s) and platform findings {found:?} (kinds not found exactly once: {missing:?})",
            bad.skips.len()
        );
    }
    let clean_path = fx.join("clean.rs");
    let clean = gather(&fx, std::slice::from_ref(&clean_path))?;
    let clean_platform =
        clean.platform_problems(&platforms::Registry::default(), &fx, reg_name, today);
    if !clean.skips.is_empty() || !clean_platform.is_empty() {
        bail!(
            "self-test failed: clean fixture gave {:?} {clean_platform:?}",
            clean.skips
        );
    }
    Ok(FIXTURE_FINDINGS)
}

/// Run the self-test, scan the workspace, apply the allowlist, then the platform-test rules.
pub fn check(root: &Path, today: &str) -> Result<StepResult> {
    if !valid_date(today) {
        bail!("today `{today}` is not YYYY-MM-DD");
    }
    let known = self_test(root, today)?;

    let mut files = Vec::new();
    for dir in SCANNED {
        files.extend(rust_files(root, &root.join(dir), EXCLUDED)?);
    }
    if files.is_empty() {
        bail!("no Rust files found to scan");
    }
    let g = gather(root, &files)?;
    let skips =
        std::fs::read_to_string(root.join("xtask/skips.toml")).context("read xtask/skips.toml")?;
    let mut problems = apply_allowlist(&g.skips, &skips, today)?;
    let listed = g.skips.len();
    let (registered, pending) = match platforms::load(root) {
        Ok(reg) => {
            problems.extend(g.platform_problems(&reg, root, platforms::REGISTRY, today));
            (reg.entries.len(), reg.pending())
        }
        Err(errors) => {
            // A malformed registry stops the step: its errors, one line per entry.
            problems.extend(errors);
            return Ok(StepResult::from_problems("", "skip problems", &problems));
        }
    };
    Ok(StepResult::from_problems(
        format!(
            "self-test ok ({known} known findings); {} files scanned, {listed} allowlisted skip(s), {registered} platform test(s) registered ({pending} pending), today = {today}",
            files.len()
        ),
        "skip problems",
        &problems,
    ))
}

/// "Today" for expiry purposes: the commit date of HEAD, so the result is a function of the commit.
pub fn today(root: &Path) -> Result<String> {
    crate::git::head_date(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(src: &str) -> Vec<(String, Kind)> {
        scan_file(Path::new("x.rs"), src)
            .unwrap()
            .into_iter()
            .map(|f| (f.item, f.kind))
            .collect()
    }

    #[test]
    fn fixture_findings_are_exact() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let p = root.join("fixtures/no-skip/violations.rs");
        let got: Vec<(String, Kind)> = scan_file(&p, &std::fs::read_to_string(&p).unwrap())
            .unwrap()
            .into_iter()
            .map(|f| (f.item, f.kind))
            .collect();
        use Kind::*;
        let mut want = vec![
            ("documented".to_string(), DocIgnore),
            ("tests::ignored".into(), Ignore),
            ("tests::ignored_on_linux".into(), CfgAttrIgnore),
            ("tests::skips_via_helper".into(), SkipCall),
            ("tests::skips_via_helper".into(), SkipCall),
            ("tests::returns_early".into(), EarlyReturn),
            ("tests::let_else_return".into(), EarlyReturn),
            ("tests::cfg_gated_test".into(), CfgDisabled),
            ("gated".into(), CfgDisabled),
        ];
        want.sort();
        let mut got = got;
        got.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn unconditional_return_and_cfg_test_are_fine() {
        assert!(
            scan("#[cfg(test)] mod t { #[test] fn a() { assert!(true); return; } }").is_empty()
        );
    }

    #[test]
    fn file_level_cfg_on_test_file_is_reported() {
        assert_eq!(
            scan("#![cfg(feature = \"x\")]\n#[test] fn a() {}"),
            [("<file>".to_string(), Kind::CfgDisabled)]
        );
    }

    #[test]
    fn allowlist_admits_listed_unexpired_entries_only() {
        let f = vec![(
            "crates/a/src/lib.rs".to_string(),
            Finding {
                item: "tests::slow".into(),
                line: 3,
                kind: Kind::Ignore,
            },
        )];
        let ok = "[[skip]]\nfile = \"crates/a/src/lib.rs\"\nitem = \"tests::slow\"\nreason = \"T-009 makes it fast\"\nexpires = \"2026-12-01\"\n";
        assert!(apply_allowlist(&f, ok, "2026-11-12").unwrap().is_empty());
        let expired = apply_allowlist(&f, ok, "2026-12-02").unwrap();
        assert!(expired[0].contains("expired"), "{expired:?}");
        let unlisted = apply_allowlist(&f, "", "2026-11-12").unwrap();
        assert!(unlisted[0].contains("skipped test"), "{unlisted:?}");
        let unused = apply_allowlist(&[], ok, "2026-11-12").unwrap();
        assert!(unused[0].contains("matches nothing"), "{unused:?}");
        let wrong_kind = ok.replace("reason", "kind = \"early-return\"\nreason");
        assert_eq!(
            apply_allowlist(&f, &wrong_kind, "2026-11-12")
                .unwrap()
                .len(),
            2
        );
        let no_reason = ok.replace("T-009 makes it fast", " ");
        assert!(apply_allowlist(&f, &no_reason, "2026-11-12").unwrap()[0].contains("empty reason"));
    }

    #[test]
    fn out_of_line_module_with_tests_behind_cfg_is_reported() {
        let d = crate::util::testutil::TempDir::new("noskip-mod");
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/slow.rs"), "#[test] fn t() {}").unwrap();
        let lib = d.path().join("src/lib.rs");
        let got = scan_file(&lib, "#[cfg(feature = \"slow\")] mod slow;").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].kind, Kind::CfgDisabled);
    }

    #[test]
    fn self_test_counts_the_fixture() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert_eq!(self_test(root, "2026-11-17").unwrap(), FIXTURE_FINDINGS);
        let fx = root.join("xtask/fixtures/no-skip");
        let g = gather(&fx, &[fx.join("violations.rs")]).unwrap();
        let reg = platforms::parse(
            "reg",
            &std::fs::read_to_string(fx.join("platform-tests.toml")).unwrap(),
        )
        .unwrap();
        let got = g.platform_problems(&reg, &fx, "reg", "2026-11-17");
        let kinds: Vec<&str> = got
            .iter()
            .map(|l| l.split(": ").nth(2).unwrap_or_default())
            .collect();
        assert_eq!(
            kinds,
            [
                "platform-unlisted",
                "inert-helper",
                "platform-statement",
                "platform-mismatch",
                "platform-unused",
                "platform-sentence",
                "platform-counterpart",
                "platform-pending-expired",
            ],
            "{got:#?}"
        );
        assert!(got[0].starts_with("violations.rs:"), "{got:?}");
        assert!(
            got[3].starts_with(
                "reg:6: `violations.rs::platform::listed_as_linux`: platform-mismatch"
            ),
            "{got:?}"
        );
    }

    #[test]
    fn platform_cfgs_are_not_skips_but_items() {
        let src = "#[cfg(test)]\nmod t {\n    #[test]\n    #[cfg(unix)]\n    fn a() {}\n    #[test]\n    #[cfg(all(test, unix))]\n    fn b() {}\n}\n";
        let s = super::scan(Path::new("x.rs"), src).unwrap();
        assert_eq!(
            s.findings
                .iter()
                .map(|f| (f.item.as_str(), f.kind))
                .collect::<Vec<_>>(),
            [("t::b", Kind::CfgDisabled)]
        );
        assert_eq!(s.platform, [("t::a".to_string(), 4, Set::UNIX)]);
        assert!(s.tests.contains(&("t::a".to_string(), Set::UNIX)));
        assert!(s.tests.contains(&("t".to_string(), Set::ALL)));
        // A test in a module behind cfg(windows) exists on Windows only.
        let s = super::scan(
            Path::new("x.rs"),
            "#[cfg(windows)]\nmod w {\n    #[test]\n    fn c() {}\n}\n",
        )
        .unwrap();
        assert_eq!(s.platform, [("w".to_string(), 1, Set::WINDOWS)]);
        assert!(s.tests.contains(&("w::c".to_string(), Set::WINDOWS)));
    }

    #[test]
    fn inert_twins_are_found_with_the_trivial_line() {
        let src = "mod s {\n    #[cfg(unix)]\n    pub fn m() -> Option<u32> {\n        Some(1)\n    }\n    #[cfg(not(unix))]\n    pub fn m() -> Option<u32> {\n        None\n    }\n    #[cfg(unix)]\n    pub fn only() {}\n}\n";
        let s = super::scan(Path::new("x.rs"), src).unwrap();
        assert_eq!(
            s.located,
            [("s::m".to_string(), 6, platforms::Kind::InertHelper)]
        );
    }

    #[test]
    fn a_file_loaded_behind_a_platform_cfg_exists_only_there() {
        let d = crate::util::testutil::TempDir::new("noskip-plat-mod");
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/w.rs"), "#[test] fn t() {}").unwrap();
        std::fs::write(d.path().join("src/lib.rs"), "#[cfg(windows)] mod w;").unwrap();
        let g = gather(
            d.path(),
            &[d.path().join("src/lib.rs"), d.path().join("src/w.rs")],
        )
        .unwrap();
        assert_eq!(
            g.tests.get(&("src/w.rs".to_string(), "t".to_string())),
            Some(&Set::WINDOWS)
        );
        assert_eq!(g.items.len(), 1);
        assert_eq!(g.items[0].item, "w");
    }
}
