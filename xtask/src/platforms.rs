//! The platform-test rule (METHODOLOGY §8.1, ADR-0009 Decision 4): the registry
//! `xtask/platform-tests.toml` of tests that exist only on some platforms, its reader, and the
//! rules `no-skip` applies to it; shared with the `scorecard` step, which uses the registry to
//! tell a test that is absent on purpose on this platform from a missing one.
//!
//! Platforms: Linux, macOS, Windows. The cfg forms that name a platform set:
//! `unix` / `not(windows)` = Linux and macOS; `windows` / `not(unix)` = Windows;
//! `target_os = "linux"`; `target_os = "macos"`. Any other `cfg` (except `cfg(test)`) on a test
//! stays a `cfg-disabled` skip of `no-skip`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;
use syn::visit::{self, Visit};
use syn::{Attribute, Block, Expr, Meta, Stmt};

use crate::util::rel;

/// One supported platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Platform {
    /// Linux.
    Linux,
    /// macOS.
    Macos,
    /// Windows.
    Windows,
}

impl Platform {
    /// The platform this binary was built for (the host the step runs on).
    pub fn host() -> Platform {
        if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        }
    }

    /// Lower-case name (`linux`, `macos`, `windows`).
    pub fn name(self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::Macos => "macos",
            Platform::Windows => "windows",
        }
    }

    fn bit(self) -> u8 {
        match self {
            Platform::Linux => 1,
            Platform::Macos => 2,
            Platform::Windows => 4,
        }
    }
}

/// A set of platforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Set(u8);

impl Set {
    /// No platform.
    pub const NONE: Set = Set(0);
    /// Linux.
    pub const LINUX: Set = Set(1);
    /// macOS.
    pub const MACOS: Set = Set(2);
    /// Linux and macOS (`cfg(unix)`).
    pub const UNIX: Set = Set(3);
    /// Windows.
    pub const WINDOWS: Set = Set(4);
    /// All three supported platforms.
    pub const ALL: Set = Set(7);

    /// Whether `p` is in the set.
    pub fn contains(self, p: Platform) -> bool {
        self.0 & p.bit() != 0
    }
    /// Union.
    pub fn union(self, o: Set) -> Set {
        Set(self.0 | o.0)
    }
    /// Intersection.
    pub fn inter(self, o: Set) -> Set {
        Set(self.0 & o.0)
    }
    /// The platforms of `ALL` not in this set.
    pub fn complement(self) -> Set {
        Set(Set::ALL.0 & !self.0)
    }
    /// Whether every platform of `o` is in this set.
    pub fn covers(self, o: Set) -> bool {
        self.0 & o.0 == o.0
    }
    /// The registry name of the set, if it has one.
    pub fn name(self) -> Option<&'static str> {
        match self {
            Set::UNIX => Some("unix"),
            Set::WINDOWS => Some("windows"),
            Set::LINUX => Some("linux"),
            Set::MACOS => Some("macos"),
            _ => None,
        }
    }
    /// The set a registry name stands for.
    pub fn from_name(s: &str) -> Option<Set> {
        match s {
            "unix" => Some(Set::UNIX),
            "windows" => Some(Set::WINDOWS),
            "linux" => Some(Set::LINUX),
            "macos" => Some(Set::MACOS),
            _ => None,
        }
    }
    /// `linux, macos`-style listing.
    pub fn list(self) -> String {
        let v: Vec<&str> = [Platform::Linux, Platform::Macos, Platform::Windows]
            .into_iter()
            .filter(|p| self.contains(*p))
            .map(Platform::name)
            .collect();
        if v.is_empty() {
            "no platform".into()
        } else {
            v.join(", ")
        }
    }
}

/// What one `#[cfg(…)]` attribute means for the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cfg {
    /// Exactly `cfg(test)`.
    Test,
    /// One of the platform forms.
    Platform(Set),
    /// Anything else.
    Other,
}

/// Classify `a`; `None` if it is not a `cfg` attribute.
pub fn classify(a: &Attribute) -> Option<Cfg> {
    if !a.path().is_ident("cfg") {
        return None;
    }
    let Meta::List(l) = &a.meta else {
        return Some(Cfg::Other);
    };
    if l.tokens.to_string().trim() == "test" {
        return Some(Cfg::Test);
    }
    Some(match a.parse_args::<Meta>() {
        Ok(m) => platform_meta(&m).map_or(Cfg::Other, Cfg::Platform),
        Err(_) => Cfg::Other,
    })
}

fn platform_meta(m: &Meta) -> Option<Set> {
    match m {
        Meta::Path(p) if p.is_ident("unix") => Some(Set::UNIX),
        Meta::Path(p) if p.is_ident("windows") => Some(Set::WINDOWS),
        Meta::NameValue(nv) if nv.path.is_ident("target_os") => match &nv.value {
            Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => match s.value().as_str() {
                "linux" => Some(Set::LINUX),
                "macos" => Some(Set::MACOS),
                _ => None,
            },
            _ => None,
        },
        Meta::List(l) if l.path.is_ident("not") => match syn::parse2::<Meta>(l.tokens.clone()) {
            Ok(Meta::Path(p)) if p.is_ident("windows") => Some(Set::UNIX),
            Ok(Meta::Path(p)) if p.is_ident("unix") => Some(Set::WINDOWS),
            _ => None,
        },
        _ => None,
    }
}

/// The cfg of an item, summed over its attributes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemCfg {
    /// No cfg, or only `cfg(test)`.
    Unconditional,
    /// Behind platform cfgs only: their intersection, and the line of the first.
    Platform(Set, usize),
    /// Behind a cfg that is not a platform form (or platform forms that exclude every
    /// platform); the line of that attribute.
    Other(usize),
}

impl ItemCfg {
    /// The platforms the item exists on (`Other`: none guaranteed).
    pub fn set(self) -> Set {
        match self {
            ItemCfg::Unconditional => Set::ALL,
            ItemCfg::Platform(s, _) => s,
            ItemCfg::Other(_) => Set::NONE,
        }
    }
}

fn line_of(span: proc_macro2::Span) -> usize {
    span.start().line
}

/// Sum the `cfg` attributes of an item.
pub fn item_cfg(attrs: &[Attribute]) -> ItemCfg {
    let mut set = Set::ALL;
    let mut first = None;
    for a in attrs {
        match classify(a) {
            None | Some(Cfg::Test) => {}
            Some(Cfg::Other) => return ItemCfg::Other(line_of(a.pound_token.span)),
            Some(Cfg::Platform(s)) => {
                set = set.inter(s);
                if set == Set::NONE {
                    return ItemCfg::Other(line_of(a.pound_token.span));
                }
                first.get_or_insert(line_of(a.pound_token.span));
            }
        }
    }
    match first {
        Some(line) => ItemCfg::Platform(set, line),
        None => ItemCfg::Unconditional,
    }
}

/// The platform set and line of the first platform cfg in `attrs`, ignoring other cfgs.
fn platform_attr(attrs: &[Attribute]) -> Option<(Set, usize)> {
    attrs.iter().find_map(|a| match classify(a) {
        Some(Cfg::Platform(s)) => Some((s, line_of(a.pound_token.span))),
        _ => None,
    })
}

// ---- inert helpers -------------------------------------------------------------------------

/// A body that does nothing: no statement, or one expression that is a literal, `None`, `()`,
/// `Ok(())`, `Default::default()`, or a tuple or struct of these (also as `return …`).
pub fn is_trivial_body(b: &Block) -> bool {
    match b.stmts.as_slice() {
        [] => true,
        [Stmt::Expr(e, _)] => trivial_expr(e),
        _ => false,
    }
}

fn trivial_expr(e: &Expr) -> bool {
    match e {
        Expr::Lit(_) => true,
        Expr::Path(p) => p.qself.is_none() && p.path.is_ident("None"),
        Expr::Tuple(t) => t.elems.iter().all(trivial_expr),
        Expr::Paren(p) => trivial_expr(&p.expr),
        Expr::Group(g) => trivial_expr(&g.expr),
        Expr::Block(b) => b.label.is_none() && is_trivial_body(&b.block),
        Expr::Return(r) => r.expr.as_deref().is_none_or(trivial_expr),
        Expr::Struct(s) => s.rest.is_none() && s.fields.iter().all(|f| trivial_expr(&f.expr)),
        Expr::Call(c) => {
            let Expr::Path(p) = &*c.func else {
                return false;
            };
            let segs: Vec<String> = p
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            match segs
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice()
            {
                ["Ok"] => {
                    c.args.len() == 1
                        && matches!(c.args.first(), Some(Expr::Tuple(t)) if t.elems.is_empty())
                }
                ["Default", "default"] => c.args.is_empty(),
                _ => false,
            }
        }
        _ => false,
    }
}

/// One definition of a non-test function behind a platform cfg.
#[derive(Clone, Debug)]
pub struct HelperDef {
    /// Item path (`support::set_mode`).
    pub item: String,
    /// Line of its platform cfg.
    pub line: usize,
    /// Its body does nothing.
    pub trivial: bool,
}

/// Inert helpers among `defs`: `(item, line)` of each trivial definition of a function that is
/// defined more than once under platform cfgs.
pub fn inert_helpers(defs: &[HelperDef]) -> Vec<(String, usize)> {
    let mut by: BTreeMap<&str, Vec<&HelperDef>> = BTreeMap::new();
    for d in defs {
        by.entry(&d.item).or_default().push(d);
    }
    let mut out: Vec<(String, usize)> = by
        .values()
        .filter(|v| v.len() > 1)
        .flat_map(|v| v.iter().filter(|d| d.trivial))
        .map(|d| (d.item.clone(), d.line))
        .collect();
    out.sort_by_key(|(_, l)| *l);
    out
}

/// The `#[cfg(…)]` attributes of a statement (`use` items: none).
fn stmt_attrs(s: &Stmt) -> &[Attribute] {
    match s {
        Stmt::Local(l) => &l.attrs,
        Stmt::Macro(m) => &m.attrs,
        Stmt::Expr(e, _) => expr_attrs(e),
        Stmt::Item(syn::Item::Use(_)) => &[],
        Stmt::Item(i) => item_attrs(i),
    }
}

fn item_attrs(i: &syn::Item) -> &[Attribute] {
    use syn::Item as I;
    match i {
        I::Const(x) => &x.attrs,
        I::Enum(x) => &x.attrs,
        I::ExternCrate(x) => &x.attrs,
        I::Fn(x) => &x.attrs,
        I::ForeignMod(x) => &x.attrs,
        I::Impl(x) => &x.attrs,
        I::Macro(x) => &x.attrs,
        I::Mod(x) => &x.attrs,
        I::Static(x) => &x.attrs,
        I::Struct(x) => &x.attrs,
        I::Trait(x) => &x.attrs,
        I::TraitAlias(x) => &x.attrs,
        I::Type(x) => &x.attrs,
        I::Union(x) => &x.attrs,
        _ => &[],
    }
}

fn expr_attrs(e: &Expr) -> &[Attribute] {
    use syn::Expr as E;
    match e {
        E::Array(x) => &x.attrs,
        E::Assign(x) => &x.attrs,
        E::Async(x) => &x.attrs,
        E::Await(x) => &x.attrs,
        E::Binary(x) => &x.attrs,
        E::Block(x) => &x.attrs,
        E::Break(x) => &x.attrs,
        E::Call(x) => &x.attrs,
        E::Cast(x) => &x.attrs,
        E::Closure(x) => &x.attrs,
        E::Const(x) => &x.attrs,
        E::Continue(x) => &x.attrs,
        E::Field(x) => &x.attrs,
        E::ForLoop(x) => &x.attrs,
        E::Group(x) => &x.attrs,
        E::If(x) => &x.attrs,
        E::Index(x) => &x.attrs,
        E::Infer(x) => &x.attrs,
        E::Let(x) => &x.attrs,
        E::Lit(x) => &x.attrs,
        E::Loop(x) => &x.attrs,
        E::Macro(x) => &x.attrs,
        E::Match(x) => &x.attrs,
        E::MethodCall(x) => &x.attrs,
        E::Paren(x) => &x.attrs,
        E::Path(x) => &x.attrs,
        E::Range(x) => &x.attrs,
        E::RawAddr(x) => &x.attrs,
        E::Reference(x) => &x.attrs,
        E::Repeat(x) => &x.attrs,
        E::Return(x) => &x.attrs,
        E::Struct(x) => &x.attrs,
        E::Try(x) => &x.attrs,
        E::TryBlock(x) => &x.attrs,
        E::Tuple(x) => &x.attrs,
        E::Unary(x) => &x.attrs,
        E::Unsafe(x) => &x.attrs,
        E::While(x) => &x.attrs,
        E::Yield(x) => &x.attrs,
        _ => &[],
    }
}

/// Lines of statements (and match arms) in a test body that carry a platform cfg while no
/// statement (arm) of the same block (match) carries the complementary platforms. Closures are
/// inspected; nested items are not.
pub fn unpaired_statements(body: &Block) -> Vec<usize> {
    struct V(Vec<usize>);
    fn group(out: &mut Vec<usize>, marked: &[(Set, usize)]) {
        for (i, (s, line)) in marked.iter().enumerate() {
            let others = marked
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .fold(Set::NONE, |acc, (_, (o, _))| acc.union(*o));
            if s.union(others) != Set::ALL {
                out.push(*line);
            }
        }
    }
    impl<'a> Visit<'a> for V {
        fn visit_item(&mut self, _: &'a syn::Item) {}
        fn visit_block(&mut self, b: &'a Block) {
            let marked: Vec<(Set, usize)> = b
                .stmts
                .iter()
                .filter_map(|s| platform_attr(stmt_attrs(s)))
                .collect();
            group(&mut self.0, &marked);
            visit::visit_block(self, b);
        }
        fn visit_expr_match(&mut self, m: &'a syn::ExprMatch) {
            let marked: Vec<(Set, usize)> = m
                .arms
                .iter()
                .filter_map(|a| platform_attr(&a.attrs))
                .collect();
            group(&mut self.0, &marked);
            visit::visit_expr_match(self, m);
        }
    }
    let mut v = V(Vec::new());
    v.visit_block(body);
    v.0.sort();
    v.0
}

// ---- the registry --------------------------------------------------------------------------

/// The registry's path, relative to the repository root.
pub const REGISTRY: &str = "xtask/platform-tests.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    file: String,
    item: String,
    platforms: String,
    doc: String,
    quote: String,
    counterpart: String,
    reason: Option<String>,
    task: Option<String>,
    expires: Option<String>,
}

/// What an entry names as the test on the other platforms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Counterpart {
    /// `<file>::<item>`.
    Named {
        /// File, relative to the root.
        file: String,
        /// Item path in the file (`<file>` for the whole file).
        item: String,
    },
    /// `none`: there is none (the reason says why).
    None,
    /// `pending`: a later task writes it.
    Pending {
        /// The task that writes it.
        task: String,
        /// `YYYY-MM-DD`.
        expires: String,
    },
}

/// One registry entry.
#[derive(Clone, Debug)]
pub struct Entry {
    /// 1-based position in the file.
    pub index: usize,
    /// Line of its `[[test]]` header in the file.
    pub line: usize,
    /// File of the test, relative to the root, `/`-separated.
    pub file: String,
    /// Item path in the file, or `<file>`.
    pub item: String,
    /// The platforms the test exists on.
    pub platforms: Set,
    /// The document holding the sentence.
    pub doc: String,
    /// A substring of `doc`.
    pub quote: String,
    /// The counterpart.
    pub counterpart: Counterpart,
}

/// The parsed registry.
#[derive(Clone, Debug, Default)]
pub struct Registry {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

impl Registry {
    /// Number of `pending` entries.
    pub fn pending(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.counterpart, Counterpart::Pending { .. }))
            .count()
    }
}

fn valid_date(d: &str) -> bool {
    d.len() == 10 && crate::timefmt::parse_utc(d).is_ok()
}

fn one_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lines of the `[[test]]` headers of `text`, in order.
fn header_lines(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim_start();
            t.strip_prefix("[[")
                .and_then(|r| r.split_once("]]"))
                .is_some_and(|(name, _)| name.trim() == "test")
        })
        .map(|(i, _)| i + 1)
        .collect()
}

/// Parse a registry text strictly. Errors are one line each, naming the entry; `name` is the
/// path used in them.
pub fn parse(name: &str, text: &str) -> Result<Registry, Vec<String>> {
    let table: toml::Table = match toml::from_str(text) {
        Ok(t) => t,
        Err(e) => {
            return Err(vec![format!("{name}: {}", one_line(&e.to_string()))]);
        }
    };
    let mut errors = Vec::new();
    for key in table.keys() {
        if key != "test" {
            errors.push(format!(
                "{name}: unknown key `{key}` (only [[test]] entries)"
            ));
        }
    }
    let raw: Vec<toml::Value> = match table.get("test") {
        None => Vec::new(),
        Some(toml::Value::Array(a)) => a.clone(),
        Some(_) => {
            errors.push(format!(
                "{name}: `test` must be an array of [[test]] tables"
            ));
            Vec::new()
        }
    };
    let lines = header_lines(text);
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, v) in raw.into_iter().enumerate() {
        let index = i + 1;
        let line = lines.get(i).copied().unwrap_or(0);
        let label =
            |file: &str, item: &str| format!("{name}:{line}: entry {index} ({file}::{item})");
        let e: RawEntry = match v.clone().try_into() {
            Ok(e) => e,
            Err(err) => {
                let get = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("?").to_owned();
                errors.push(format!(
                    "{}: {}",
                    label(&get("file"), &get("item")),
                    one_line(&err.to_string())
                ));
                continue;
            }
        };
        let at = label(&e.file, &e.item);
        let before = errors.len();
        let empty = |s: &str| s.trim().is_empty();
        for (field, value) in [
            ("file", &e.file),
            ("item", &e.item),
            ("doc", &e.doc),
            ("quote", &e.quote),
        ] {
            if empty(value) {
                errors.push(format!("{at}: empty `{field}`"));
            }
        }
        if e.file.contains('\\') {
            errors.push(format!("{at}: `file` must be `/`-separated"));
        }
        let platforms = Set::from_name(&e.platforms);
        if platforms.is_none() {
            errors.push(format!(
                "{at}: platforms `{}` is not unix | windows | linux | macos",
                e.platforms
            ));
        }
        if e.reason.as_deref().is_some_and(empty) {
            errors.push(format!("{at}: empty `reason`"));
        }
        let counterpart = match e.counterpart.as_str() {
            "none" | "pending" if e.reason.is_none() => {
                errors.push(format!(
                    "{at}: counterpart `{}` needs a `reason`",
                    e.counterpart
                ));
                None
            }
            "none" => Some(Counterpart::None),
            "pending" => {
                let task = e.task.clone().unwrap_or_default();
                let expires = e.expires.clone().unwrap_or_default();
                if !crate::util::valid_task_id(&task) {
                    errors.push(format!(
                        "{at}: a pending counterpart needs `task = \"T-NNN\"` (got `{task}`)"
                    ));
                }
                if !valid_date(&expires) {
                    errors.push(format!(
                        "{at}: a pending counterpart needs `expires = \"YYYY-MM-DD\"` (got `{expires}`)"
                    ));
                }
                Some(Counterpart::Pending { task, expires })
            }
            named => match named.split_once("::") {
                Some((f, it)) if !empty(f) && !empty(it) => Some(Counterpart::Named {
                    file: f.to_owned(),
                    item: it.to_owned(),
                }),
                _ => {
                    errors.push(format!(
                        "{at}: counterpart `{named}` is not `<file>::<item>`, `none` or `pending`"
                    ));
                    None
                }
            },
        };
        if e.counterpart != "pending" {
            for (field, v) in [("task", &e.task), ("expires", &e.expires)] {
                if v.is_some() {
                    errors.push(format!("{at}: `{field}` is only for a pending counterpart"));
                }
            }
        }
        if !seen.insert((e.file.clone(), e.item.clone())) {
            errors.push(format!("{at}: the same test is listed twice"));
        }
        if errors.len() == before
            && let (Some(platforms), Some(counterpart)) = (platforms, counterpart)
        {
            entries.push(Entry {
                index,
                line,
                file: e.file,
                item: e.item,
                platforms,
                doc: e.doc,
                quote: e.quote,
                counterpart,
            });
        }
    }
    if errors.is_empty() {
        Ok(Registry { entries })
    } else {
        Err(errors)
    }
}

/// Read `<root>/xtask/platform-tests.toml`; a missing file is an empty registry.
pub fn load(root: &Path) -> Result<Registry, Vec<String>> {
    match std::fs::read_to_string(root.join(REGISTRY)) {
        Ok(text) => parse(REGISTRY, &text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
        Err(e) => Err(vec![format!("{REGISTRY}: {e}")]),
    }
}

// ---- the rules -----------------------------------------------------------------------------

/// The new kinds of `no-skip` finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A platform-conditional test without an entry.
    Unlisted,
    /// An entry whose platforms differ from the item's cfg.
    Mismatch,
    /// An entry that matches nothing.
    Unused,
    /// The entry's document or quote is missing.
    Sentence,
    /// The named counterpart does not exist on the other platforms.
    Counterpart,
    /// A pending counterpart past its date or without a task file.
    PendingExpired,
    /// A twin helper whose body does nothing on one platform.
    InertHelper,
    /// A platform statement in a test with nothing in its place on another platform.
    PlatformStatement,
}

impl Kind {
    /// The name printed in the FAIL line.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Unlisted => "platform-unlisted",
            Kind::Mismatch => "platform-mismatch",
            Kind::Unused => "platform-unused",
            Kind::Sentence => "platform-sentence",
            Kind::Counterpart => "platform-counterpart",
            Kind::PendingExpired => "platform-pending-expired",
            Kind::InertHelper => "inert-helper",
            Kind::PlatformStatement => "platform-statement",
        }
    }
}

/// A test, a module with tests or a file (`<file>`) behind a platform cfg.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformItem {
    /// File relative to the root, `/`-separated.
    pub file: String,
    /// Item path.
    pub item: String,
    /// Line of its platform cfg.
    pub line: usize,
    /// The platforms of its cfg.
    pub platforms: Set,
}

/// A finding located in a scanned file (inert helpers, platform statements).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    /// File relative to the root.
    pub file: String,
    /// Item path.
    pub item: String,
    /// Line.
    pub line: usize,
    /// Kind.
    pub kind: Kind,
}

/// Every test fn, module containing tests and file with tests (`<file>`) of the scan, with the
/// platforms it exists on: `(file, item) -> platforms`.
pub type TestIndex = BTreeMap<(String, String), Set>;

/// One problem line, with its sort key (scan order).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Problem {
    /// File the line names.
    pub file: String,
    /// Line it names.
    pub line: usize,
    /// Kind.
    pub kind: Kind,
    /// The whole text.
    pub text: String,
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether `tasks/<task>-*.md` exists under `root`.
pub fn task_file_exists(root: &Path, task: &str) -> bool {
    let prefix = format!("{task}-");
    std::fs::read_dir(root.join("tasks")).is_ok_and(|rd| {
        rd.flatten().any(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with(&prefix) && n.ends_with(".md") && e.path().is_file()
        })
    })
}

/// Apply the registry rules to the scan: items behind platform cfgs, every test of the scan,
/// and the located findings of the scan. `root` is where `doc` and `tasks/` are read; `today`
/// is `YYYY-MM-DD`. Returns problems in scan order, then the unused entries.
pub fn apply(
    reg: &Registry,
    items: &[PlatformItem],
    tests: &TestIndex,
    located: &[Located],
    root: &Path,
    registry_name: &str,
    today: &str,
) -> Vec<String> {
    let mut out: Vec<Problem> = Vec::new();
    let mut used = BTreeSet::new();
    let mut push = |file: &str, line: usize, item: &str, kind: Kind, what: String| {
        out.push(Problem {
            file: file.to_owned(),
            line,
            kind,
            text: format!("{file}:{line}: `{item}`: {}: {what}", kind.name()),
        });
    };
    for it in items {
        let name = it.platforms.name().unwrap_or("?");
        let Some(e) = reg
            .entries
            .iter()
            .find(|e| e.file == it.file && e.item == it.item)
        else {
            push(
                &it.file,
                it.line,
                &it.item,
                Kind::Unlisted,
                format!(
                    "a test behind cfg `{name}` that is not in {registry_name}; list it with its sentence and counterpart, or make it run on every platform"
                ),
            );
            continue;
        };
        used.insert(e.index);
        let n = e.index;
        if e.platforms != it.platforms {
            push(
                &it.file,
                it.line,
                &it.item,
                Kind::Mismatch,
                format!(
                    "entry {n} says platforms = \"{}\", the item is behind cfg `{name}`",
                    e.platforms.name().unwrap_or("?")
                ),
            );
        }
        match std::fs::read_to_string(root.join(&e.doc)) {
            Err(_) => push(
                &it.file,
                it.line,
                &it.item,
                Kind::Sentence,
                format!("entry {n}: doc `{}` does not exist", e.doc),
            ),
            Ok(text) if !squash(&text).contains(&squash(&e.quote)) => push(
                &it.file,
                it.line,
                &it.item,
                Kind::Sentence,
                format!(
                    "entry {n}: `{}` does not contain the quote \"{}\"",
                    e.doc, e.quote
                ),
            ),
            Ok(_) => {}
        }
        match &e.counterpart {
            Counterpart::None => {}
            Counterpart::Named { file, item } => {
                let needed = e.platforms.complement();
                match tests.get(&(file.clone(), item.clone())) {
                    None => push(
                        &it.file,
                        it.line,
                        &it.item,
                        Kind::Counterpart,
                        format!(
                            "entry {n}: counterpart `{file}::{item}` is not a test or a module with tests"
                        ),
                    ),
                    Some(s) if !s.covers(needed) => push(
                        &it.file,
                        it.line,
                        &it.item,
                        Kind::Counterpart,
                        format!(
                            "entry {n}: counterpart `{file}::{item}` exists on {} only, not on every one of {}",
                            s.list(),
                            needed.list()
                        ),
                    ),
                    Some(_) => {}
                }
            }
            Counterpart::Pending { task, expires } => {
                if expires.as_str() < today {
                    push(
                        &it.file,
                        it.line,
                        &it.item,
                        Kind::PendingExpired,
                        format!(
                            "entry {n}: pending counterpart expired on {expires} (today is {today})"
                        ),
                    );
                }
                if !task_file_exists(root, task) {
                    push(
                        &it.file,
                        it.line,
                        &it.item,
                        Kind::PendingExpired,
                        format!("entry {n}: pending on {task}, but there is no tasks/{task}-*.md"),
                    );
                }
            }
        }
    }
    for l in located {
        let what = match l.kind {
            Kind::InertHelper => "a helper defined per platform whose body here does nothing; give it a real body on every platform or make the caller fail here, naming why".to_owned(),
            _ => "a statement behind a platform cfg with no statement for the other platforms in the same block; add one that checks there (or fails, naming why)".to_owned(),
        };
        push(&l.file, l.line, &l.item, l.kind, what);
    }
    out.sort();
    let mut lines: Vec<String> = out.into_iter().map(|p| p.text).collect();
    for e in &reg.entries {
        if !used.contains(&e.index) {
            lines.push(format!(
                "{registry_name}:{}: `{}::{}`: {}: entry {} matches no platform-conditional test; remove it",
                e.line,
                e.file,
                e.item,
                Kind::Unused.name(),
                e.index
            ));
        }
    }
    lines
}

// ---- scorecard names -----------------------------------------------------------------------

/// The scorecard name prefix of an entry's `file`/`item` (`<crate_ident>[::<target>]::<path>`),
/// as `testbins` qualifies names; `None` if the file is not in a package.
pub fn qualified_prefix(root: &Path, file: &str, item: &str) -> Option<String> {
    let path = root.join(file);
    let mut dir = path.parent()?;
    let (pkg_dir, pkg) = loop {
        if let Ok(t) = std::fs::read_to_string(dir.join("Cargo.toml"))
            && let Ok(v) = toml::from_str::<toml::Value>(&t)
            && let Some(name) = v
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
        {
            break (dir.to_path_buf(), name.replace('-', "_"));
        }
        if dir == root {
            return None;
        }
        dir = dir.parent()?;
    };
    let inner = rel(&pkg_dir, &path);
    let parts: Vec<&str> = inner.trim_end_matches(".rs").split('/').collect();
    let mut q = vec![pkg];
    match parts.as_slice() {
        ["tests", name] | ["tests", name, "main"] | ["src", "bin", name] => {
            q.push((*name).replace('-', "_"))
        }
        ["src", "lib"] | ["src", "main"] => {}
        ["src", rest @ ..] => {
            let rest: Vec<&str> = match rest {
                [m @ .., "mod"] => m.to_vec(),
                r => r.to_vec(),
            };
            q.extend(rest.into_iter().map(str::to_owned));
        }
        _ => return None,
    }
    if item != "<file>" {
        q.push(item.to_owned());
    }
    Some(q.join("::"))
}

/// Scorecard names of tests the registry says do not exist on `host`: `(prefix, platforms)`.
pub fn absent_on(root: &Path, reg: &Registry, host: Platform) -> Vec<(String, Set)> {
    reg.entries
        .iter()
        .filter(|e| !e.platforms.contains(host))
        .filter_map(|e| qualified_prefix(root, &e.file, &e.item).map(|q| (q, e.platforms)))
        .collect()
}

/// Whether the scorecard name `test` is covered by one of the `absent` prefixes.
pub fn is_registered_absent(absent: &[(String, Set)], test: &str) -> bool {
    absent.iter().any(|(p, _)| {
        test == p
            || test
                .strip_prefix(p.as_str())
                .is_some_and(|r| r.starts_with("::"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(src: &str) -> Attribute {
        let f: syn::ItemFn = syn::parse_str(&format!("{src} fn f() {{}}")).unwrap();
        f.attrs[0].clone()
    }

    #[test]
    fn cfg_forms() {
        assert_eq!(classify(&attr("#[cfg(test)]")), Some(Cfg::Test));
        assert_eq!(
            classify(&attr("#[cfg(unix)]")),
            Some(Cfg::Platform(Set::UNIX))
        );
        assert_eq!(
            classify(&attr("#[cfg(not(windows))]")),
            Some(Cfg::Platform(Set::UNIX))
        );
        assert_eq!(
            classify(&attr("#[cfg(windows)]")),
            Some(Cfg::Platform(Set::WINDOWS))
        );
        assert_eq!(
            classify(&attr("#[cfg(not(unix))]")),
            Some(Cfg::Platform(Set::WINDOWS))
        );
        assert_eq!(
            classify(&attr("#[cfg(target_os = \"linux\")]")),
            Some(Cfg::Platform(Set::LINUX))
        );
        assert_eq!(
            classify(&attr("#[cfg(target_os = \"macos\")]")),
            Some(Cfg::Platform(Set::MACOS))
        );
        for other in [
            "#[cfg(feature = \"x\")]",
            "#[cfg(all(test, unix))]",
            "#[cfg(any(unix, windows))]",
            "#[cfg(target_os = \"freebsd\")]",
            "#[cfg(not(target_os = \"linux\"))]",
        ] {
            assert_eq!(classify(&attr(other)), Some(Cfg::Other), "{other}");
        }
        assert_eq!(classify(&attr("#[test]")), None);
    }

    #[test]
    fn set_names_and_cover() {
        assert_eq!(Set::UNIX.complement(), Set::WINDOWS);
        assert_eq!(Set::LINUX.complement(), Set::MACOS.union(Set::WINDOWS));
        assert!(Set::ALL.covers(Set::WINDOWS));
        assert!(!Set::UNIX.covers(Set::WINDOWS));
        for n in ["unix", "windows", "linux", "macos"] {
            assert_eq!(Set::from_name(n).and_then(Set::name), Some(n));
        }
        assert_eq!(Set::from_name("darwin"), None);
    }

    fn body(src: &str) -> Block {
        syn::parse_str(src).unwrap()
    }

    #[test]
    fn trivial_bodies() {
        for t in [
            "{}",
            "{ None }",
            "{ () }",
            "{ Ok(()) }",
            "{ 0 }",
            "{ \"x\" }",
            "{ Default::default() }",
            "{ (None, 0) }",
            "{ S { a: None, b: false } }",
            "{ return None; }",
        ] {
            assert!(is_trivial_body(&body(t)), "{t}");
        }
        for t in [
            "{ x() }",
            "{ Some(1) }",
            "{ Ok(x) }",
            "{ let a = 1; a }",
            "{ panic!(\"no\") }",
            "{ S { a: f() } }",
        ] {
            assert!(!is_trivial_body(&body(t)), "{t}");
        }
    }

    #[test]
    fn statements_need_their_complement() {
        assert!(
            unpaired_statements(&body("{ #[cfg(unix)] { a(); } #[cfg(windows)] b(); }")).is_empty()
        );
        assert!(
            unpaired_statements(&body(
                "{ #[cfg(not(unix))] panic!(\"why\"); #[cfg(unix)] { a(); } }"
            ))
            .is_empty()
        );
        assert!(
            unpaired_statements(&body(
                "{ #[cfg(unix)] use std::os::unix::fs::PermissionsExt; a(); }"
            ))
            .is_empty()
        );
        assert_eq!(
            unpaired_statements(&body("{\n#[cfg(unix)]\nset_mode(p);\n}")),
            [2]
        );
        assert_eq!(
            unpaired_statements(&body("{ a(); if x { #[cfg(windows)] let y = 1; } }")),
            [1]
        );
        // linux + windows leaves macOS out: both are reported.
        assert_eq!(
            unpaired_statements(&body(
                "{\n#[cfg(target_os = \"linux\")]\na();\n#[cfg(windows)]\nb();\n}"
            )),
            [2, 4]
        );
        assert!(
            unpaired_statements(&body(
                "{ #[cfg(target_os = \"linux\")] a(); #[cfg(target_os = \"macos\")] b(); #[cfg(windows)] c(); }"
            ))
            .is_empty()
        );
        // Match arms are a group of their own; closures are inspected, nested fns not.
        assert!(
            unpaired_statements(&body(
                "{ match x { #[cfg(unix)] 1 => a(), #[cfg(windows)] _ => b() } }"
            ))
            .is_empty()
        );
        assert_eq!(
            unpaired_statements(&body("{ let f = || { #[cfg(unix)] a(); }; }")).len(),
            1
        );
        assert!(unpaired_statements(&body("{ fn h() { #[cfg(unix)] a(); } }")).is_empty());
    }

    #[test]
    fn inert_twins_only() {
        let d = |item: &str, line, trivial| HelperDef {
            item: item.into(),
            line,
            trivial,
        };
        let defs = [
            d("support::mode", 10, false),
            d("support::mode", 16, true),
            d("support::link", 20, false),
            d("support::link", 24, false),
            d("support::only_unix", 30, true),
        ];
        assert_eq!(inert_helpers(&defs), [("support::mode".to_string(), 16)]);
    }

    const GOOD: &str = r#"
[[test]]
file = "crates/a/tests/t.rs"
item = "fifo"
platforms = "unix"
doc = "docs/p.md"
quote = "FIFOs are refused"
counterpart = "crates/a/tests/t.rs::fifo_windows"

[[test]]
file = "crates/a/tests/t.rs"
item = "later"
platforms = "unix"
doc = "docs/p.md"
quote = "FIFOs are refused"
counterpart = "pending"
reason = "written by T-075"
task = "T-075"
expires = "2026-12-31"

[[test]]
file = "crates/a/tests/t.rs"
item = "never"
platforms = "linux"
doc = "docs/p.md"
quote = "FIFOs are refused"
counterpart = "none"
reason = "nothing to compare with"
"#;

    #[test]
    fn registry_parses_strictly() {
        let r = parse("reg", GOOD).unwrap();
        assert_eq!(r.entries.len(), 3);
        assert_eq!(r.pending(), 1);
        assert_eq!(r.entries[1].line, 10);
        assert_eq!(
            r.entries[0].counterpart,
            Counterpart::Named {
                file: "crates/a/tests/t.rs".into(),
                item: "fifo_windows".into()
            }
        );
        let bad = |from: &str, to: &str, want: &str| {
            let e = parse("reg", &GOOD.replacen(from, to, 1)).unwrap_err();
            assert_eq!(e.len(), 1, "{from} -> {to}: {e:?}");
            assert!(e[0].contains(want), "{from} -> {to}: {e:?}");
            assert!(e[0].starts_with("reg"), "{e:?}");
        };
        bad(
            "platforms = \"unix\"",
            "platforms = \"bsd\"",
            "platforms `bsd`",
        );
        bad(
            "quote = \"FIFOs are refused\"",
            "quote = \" \"",
            "empty `quote`",
        );
        bad("doc = \"docs/p.md\"", "doc = \"\"", "empty `doc`");
        bad(
            "reason = \"written by T-075\"",
            "reason = \"\"",
            "empty `reason`",
        );
        bad(
            "expires = \"2026-12-31\"",
            "expires = \"2026-13-01\"",
            "expires",
        );
        bad("task = \"T-075\"", "task = \"soon\"", "task");
        bad(
            "reason = \"nothing to compare with\"\n",
            "",
            "needs a `reason`",
        );
        bad(
            "counterpart = \"crates/a/tests/t.rs::fifo_windows\"",
            "counterpart = \"elsewhere\"",
            "is not `<file>::<item>`",
        );
        bad(
            "quote = \"FIFOs are refused\"\ncounterpart = \"none\"",
            "quote = \"FIFOs are refused\"\ncounterpart = \"none\"\nowner = \"x\"",
            "unknown field",
        );
        bad("item = \"never\"", "item = \"fifo\"", "listed twice");
        bad(
            "counterpart = \"none\"",
            "counterpart = \"none\"\ntask = \"T-075\"",
            "only for a pending",
        );
        let e = parse("reg", "[[test]\n").unwrap_err();
        assert_eq!(e.len(), 1);
        assert!(parse("reg", "").unwrap().entries.is_empty());
    }

    #[test]
    fn rules() {
        let d = crate::util::testutil::TempDir::new("platform-rules");
        let root = d.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(
            root.join("docs/p.md"),
            "Directories and\n  FIFOs are refused.\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("tasks")).unwrap();
        std::fs::write(root.join("tasks/T-075-later.md"), "x").unwrap();
        let reg = parse("reg", GOOD).unwrap();
        let f = "crates/a/tests/t.rs";
        let item = |name: &str, line, platforms| PlatformItem {
            file: f.into(),
            item: name.into(),
            line,
            platforms,
        };
        let items = vec![
            item("fifo", 3, Set::UNIX),
            item("later", 9, Set::UNIX),
            item("never", 15, Set::LINUX),
        ];
        let mut tests = TestIndex::new();
        tests.insert((f.into(), "fifo_windows".into()), Set::ALL);
        let ok = apply(&reg, &items, &tests, &[], root, "reg", "2026-11-17");
        assert!(ok.is_empty(), "{ok:?}");

        // Each fault alone gives one line of its kind, naming file, line and item.
        let one = |items: &[PlatformItem], tests: &TestIndex, today: &str, kind: Kind| {
            let p = apply(&reg, items, tests, &[], root, "reg", today);
            assert_eq!(p.len(), 1, "{kind:?}: {p:?}");
            assert!(p[0].contains(kind.name()), "{p:?}");
            p[0].clone()
        };
        let mut extra = items.clone();
        extra.push(item("new_one", 30, Set::WINDOWS));
        assert!(
            one(&extra, &tests, "2026-11-17", Kind::Unlisted)
                .starts_with(&format!("{f}:30: `new_one`: platform-unlisted"))
        );
        let mut mism = items.clone();
        mism[2].platforms = Set::UNIX;
        assert!(
            one(&mism, &tests, "2026-11-17", Kind::Mismatch)
                .starts_with(&format!("{f}:15: `never`"))
        );
        let fewer = &items[..2];
        assert!(
            one(fewer, &tests, "2026-11-17", Kind::Unused)
                .starts_with("reg:21: `crates/a/tests/t.rs::never`")
        );
        let mut unix_only = TestIndex::new();
        unix_only.insert((f.into(), "fifo_windows".into()), Set::UNIX);
        assert!(
            one(&items, &unix_only, "2026-11-17", Kind::Counterpart)
                .contains("not on every one of windows")
        );
        assert!(
            one(&items, &TestIndex::new(), "2026-11-17", Kind::Counterpart)
                .contains("is not a test")
        );
        assert!(
            one(&items, &tests, "2027-01-01", Kind::PendingExpired)
                .contains("expired on 2026-12-31")
        );
        std::fs::remove_file(root.join("tasks/T-075-later.md")).unwrap();
        assert!(
            one(&items, &tests, "2026-11-17", Kind::PendingExpired).contains("no tasks/T-075-*.md")
        );
        std::fs::write(root.join("tasks/T-075-later.md"), "x").unwrap();
        std::fs::write(root.join("docs/p.md"), "Directories are refused.\n").unwrap();
        assert_eq!(
            apply(&reg, &items, &tests, &[], root, "reg", "2026-11-17").len(),
            3
        );
        std::fs::remove_file(root.join("docs/p.md")).unwrap();
        let p = apply(&reg, &items, &tests, &[], root, "reg", "2026-11-17");
        assert!(
            p.iter()
                .all(|l| l.contains("platform-sentence") && l.contains("does not exist")),
            "{p:?}"
        );
    }

    #[test]
    fn scorecard_prefixes() {
        let d = crate::util::testutil::TempDir::new("platform-names");
        let root = d.path();
        std::fs::create_dir_all(root.join("crates/kiln-a/src/m")).unwrap();
        std::fs::create_dir_all(root.join("crates/kiln-a/tests")).unwrap();
        std::fs::write(
            root.join("crates/kiln-a/Cargo.toml"),
            "[package]\nname = \"kiln-a\"\n",
        )
        .unwrap();
        let q = |f: &str, i: &str| qualified_prefix(root, f, i);
        assert_eq!(
            q("crates/kiln-a/tests/acc_t1.rs", "fifo").as_deref(),
            Some("kiln_a::acc_t1::fifo")
        );
        assert_eq!(
            q("crates/kiln-a/src/lib.rs", "tests::x").as_deref(),
            Some("kiln_a::tests::x")
        );
        assert_eq!(
            q("crates/kiln-a/src/m/mod.rs", "tests").as_deref(),
            Some("kiln_a::m::tests")
        );
        assert_eq!(
            q("crates/kiln-a/src/m/deep.rs", "tests::y").as_deref(),
            Some("kiln_a::m::deep::tests::y")
        );
        assert_eq!(
            q("crates/kiln-a/tests/acc_t1.rs", "<file>").as_deref(),
            Some("kiln_a::acc_t1")
        );
        assert_eq!(q("elsewhere/x.rs", "t"), None);
        let absent = vec![("kiln_a::m::tests".to_string(), Set::WINDOWS)];
        assert!(is_registered_absent(&absent, "kiln_a::m::tests::y"));
        assert!(is_registered_absent(&absent, "kiln_a::m::tests"));
        assert!(!is_registered_absent(&absent, "kiln_a::m::tests_more::y"));
    }
}
