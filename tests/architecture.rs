//! Architecture checks: the design limits in `docs/design/architecture.md`, read from the source.
//!
//! Each limit records today's truth and may only get stricter. A check fails when the code gets
//! worse than its limit, and also when it gets better, so the limit is lowered in the same PR and
//! the gain can't quietly slip back. Test code is not checked: anything behind `#[cfg(test)]` is
//! skipped.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use syn::{Attribute, Item, UseTree, spanned::Spanned, visit::Visit};

/// Most fields `BrainDump` may have. Step #37 (a `Conversation` type) lowers it.
const BRAIN_DUMP_FIELDS: usize = 56;
/// Most `pub` fields `BrainDump` may have. The menu reads `quit` and `receipt` directly today.
const BRAIN_DUMP_PUB_FIELDS: usize = 9;
/// Most non-test source lines longer than [`LONG_LINE`] characters, mainly the one-line prompts.
/// Step #38 (prompts in their own files) lowers it.
const LONG_LINES: usize = 33;
const LONG_LINE: usize = 200;

/// Where a phase reaches into another phase today. Each entry names the step that removes it.
/// An entry that no longer matches any code fails the check, so the list only shrinks.
const EXCEPTIONS: &[(&str, &str, &str)] = &[
    (
        "shape",
        "shaping::brain_dump::handoff::continuation_prompt",
        "#36 moves the continuation prompt into one Seed Me module",
    ),
    (
        "shape",
        "shaping::confirmed_goal",
        "#35 moves the goal block into a shared goal module",
    ),
];

/// The area a module path (without `crate`) belongs to. Phases are think, shape and menu (which
/// also holds measure and the unbuilt places today); `shared` is the crate root and voice.
fn area(path: &[String]) -> &'static str {
    match path
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["shaping", ..] => "think",
        ["home", "shape" | "discovery", ..] => "shape",
        ["home", ..] => "menu",
        _ => "shared",
    }
}

/// May code in area `from` name code in area `to`? Only the menu knows every phase; anything
/// may use shared code; shared code names no phase.
fn allowed(from: &str, to: &str) -> bool {
    from == to || from == "menu" || to == "shared"
}

#[test]
fn phases_only_reach_each_other_through_listed_exceptions() {
    let mut violations = vec![];
    let mut used = BTreeSet::new();
    for (file, module) in production_files() {
        let from = area(&module);
        for target in references(&file, &module) {
            let to = area(&target);
            if allowed(from, to) {
                continue;
            }
            let joined = target.join("::");
            match EXCEPTIONS
                .iter()
                .position(|(f, prefix, _)| *f == from && starts_with_segments(&joined, prefix))
            {
                Some(i) => {
                    used.insert(i);
                }
                None => violations.push(format!("{} ({from}) uses {joined} ({to})", rel(&file))),
            }
        }
    }
    let stale: Vec<_> = EXCEPTIONS
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
        .map(|(_, (from, prefix, _))| format!("{from} -> {prefix}"))
        .collect();
    assert!(
        violations.is_empty(),
        "A phase reaches into another phase. Go through shared code or the menu instead:\n{}",
        violations.join("\n")
    );
    assert!(
        stale.is_empty(),
        "These exceptions are no longer used; delete them from EXCEPTIONS:\n{}",
        stale.join("\n")
    );
}

#[test]
fn brain_dump_does_not_grow() {
    let file = root().join("src/shaping/brain_dump.rs");
    let parsed = syn::parse_file(&fs::read_to_string(&file).unwrap()).unwrap();
    let fields = parsed
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(s) if s.ident == "BrainDump" => Some(&s.fields),
            _ => None,
        })
        .expect("BrainDump is defined in src/shaping/brain_dump.rs");
    let public = fields
        .iter()
        .filter(|f| matches!(f.vis, syn::Visibility::Public(_)))
        .count();
    at_limit("BrainDump fields", fields.len(), BRAIN_DUMP_FIELDS);
    at_limit("BrainDump pub fields", public, BRAIN_DUMP_PUB_FIELDS);
}

#[test]
fn long_lines_do_not_grow() {
    let mut files: Vec<_> = production_files().into_iter().map(|(f, _)| f).collect();
    files.push(root().join("src/main.rs"));
    let mut found = vec![];
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        let tests = test_line_ranges(&syn::parse_file(&text).unwrap());
        for (i, line) in text.lines().enumerate() {
            let n = i + 1;
            if line.chars().count() > LONG_LINE && !tests.iter().any(|r| r.contains(&n)) {
                found.push(format!("{}:{n}", rel(&file)));
            }
        }
    }
    at_limit(
        &format!(
            "source lines over {LONG_LINE} characters ({})",
            found.join(", ")
        ),
        found.len(),
        LONG_LINES,
    );
}

fn at_limit(what: &str, actual: usize, limit: usize) {
    assert!(
        actual <= limit,
        "{what}: {actual}, over the limit of {limit}. Limits only get stricter."
    );
    assert!(
        actual == limit,
        "{what}: {actual}, better than the limit of {limit}. Lower the limit to {actual} in tests/architecture.rs."
    );
}

fn starts_with_segments(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}::"))
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rel(file: &Path) -> String {
    file.strip_prefix(root())
        .unwrap_or(file)
        .display()
        .to_string()
}

fn is_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.meta.require_list().is_ok_and(|l| {
                let cfg = l.tokens.to_string();
                cfg == "test"
                    || (cfg.starts_with("all")
                        && cfg
                            .split(|c: char| !c.is_alphanumeric())
                            .any(|w| w == "test"))
            })
    })
}

/// Every library source file outside `#[cfg(test)]`, with its module path, found by following
/// `mod` declarations from `src/lib.rs`.
fn production_files() -> Vec<(PathBuf, Vec<String>)> {
    let mut out = vec![];
    walk(
        &root().join("src/lib.rs"),
        vec![],
        &root().join("src"),
        &mut out,
    );
    out
}

fn walk(file: &Path, module: Vec<String>, children: &Path, out: &mut Vec<(PathBuf, Vec<String>)>) {
    let parsed = syn::parse_file(&fs::read_to_string(file).unwrap()).unwrap();
    out.push((file.to_path_buf(), module.clone()));
    walk_items(&parsed.items, &module, children, out);
}

fn walk_items(
    items: &[Item],
    module: &[String],
    children: &Path,
    out: &mut Vec<(PathBuf, Vec<String>)>,
) {
    for item in items {
        let Item::Mod(m) = item else { continue };
        if is_test(&m.attrs) {
            continue;
        }
        let name = m.ident.to_string();
        let mut path = module.to_vec();
        path.push(name.clone());
        match &m.content {
            Some(_) => panic!(
                "inline module {} isn't checked; give it its own file",
                path.join("::")
            ),
            None => {
                let flat = children.join(format!("{name}.rs"));
                if flat.is_file() {
                    walk(&flat, path, &children.join(&name), out);
                } else {
                    let dir = children.join(&name);
                    walk(&dir.join("mod.rs"), path, &dir, out);
                }
            }
        }
    }
}

/// Crate paths (without `crate`) named by `use` items and `crate::`/`super::` paths in non-test
/// code of one file.
fn references(file: &Path, module: &[String]) -> Vec<Vec<String>> {
    struct Refs<'a> {
        module: &'a [String],
        found: Vec<Vec<String>>,
    }
    impl Refs<'_> {
        fn resolve(&self, segments: &[String]) -> Option<Vec<String>> {
            let mut base = self.module.to_vec();
            let mut rest = segments;
            match rest.first().map(String::as_str) {
                Some("crate") | Some("tinkery") => {
                    base.clear();
                    rest = &rest[1..];
                }
                Some("super") => {
                    while rest.first().is_some_and(|s| s == "super") {
                        base.pop();
                        rest = &rest[1..];
                    }
                }
                Some("self") => rest = &rest[1..],
                _ => return None,
            }
            base.extend(rest.iter().cloned());
            Some(base)
        }
        fn use_tree(&mut self, prefix: &mut Vec<String>, tree: &UseTree) {
            match tree {
                UseTree::Path(p) => {
                    prefix.push(p.ident.to_string());
                    self.use_tree(prefix, &p.tree);
                    prefix.pop();
                }
                UseTree::Name(n) => self.leaf(prefix, &n.ident.to_string()),
                UseTree::Rename(r) => self.leaf(prefix, &r.ident.to_string()),
                UseTree::Glob(_) => self.leaf(prefix, "self"),
                UseTree::Group(g) => {
                    for t in &g.items {
                        self.use_tree(prefix, t);
                    }
                }
            }
        }
        fn leaf(&mut self, prefix: &[String], name: &str) {
            let mut full = prefix.to_vec();
            if name != "self" {
                full.push(name.into());
            }
            if let Some(path) = self.resolve(&full) {
                self.found.push(path);
            }
        }
    }
    impl<'ast> Visit<'ast> for Refs<'_> {
        fn visit_item(&mut self, item: &'ast Item) {
            let attrs = match item {
                Item::Fn(i) => &i.attrs,
                Item::Impl(i) => &i.attrs,
                Item::Mod(i) => &i.attrs,
                Item::Use(i) => &i.attrs,
                Item::Struct(i) => &i.attrs,
                Item::Enum(i) => &i.attrs,
                Item::Const(i) => &i.attrs,
                Item::Static(i) => &i.attrs,
                Item::Trait(i) => &i.attrs,
                _ => return syn::visit::visit_item(self, item),
            };
            if is_test(attrs) {
                return;
            }
            if let Item::Use(u) = item {
                self.use_tree(&mut vec![], &u.tree);
                return;
            }
            syn::visit::visit_item(self, item);
        }
        fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
            if !is_test(&f.attrs) {
                syn::visit::visit_impl_item_fn(self, f);
            }
        }
        // `pub(super)` names a module but depends on nothing.
        fn visit_visibility(&mut self, _: &'ast syn::Visibility) {}
        fn visit_path(&mut self, path: &'ast syn::Path) {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            if let Some(resolved) = self.resolve(&segments) {
                self.found.push(resolved);
            }
            syn::visit::visit_path(self, path);
        }
    }
    let parsed = syn::parse_file(&fs::read_to_string(file).unwrap()).unwrap();
    let mut refs = Refs {
        module,
        found: vec![],
    };
    for item in &parsed.items {
        refs.visit_item(item);
    }
    refs.found
}

/// Line ranges of `#[cfg(test)]` items, at any depth of inline modules and impls.
fn test_line_ranges(file: &syn::File) -> Vec<std::ops::RangeInclusive<usize>> {
    struct Ranges(Vec<std::ops::RangeInclusive<usize>>);
    impl Ranges {
        fn add(&mut self, node: &impl Spanned) {
            let span = node.span();
            self.0.push(span.start().line..=span.end().line);
        }
    }
    impl<'ast> Visit<'ast> for Ranges {
        fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
            if is_test(&m.attrs) {
                self.add(m)
            } else {
                syn::visit::visit_item_mod(self, m)
            }
        }
        fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
            if is_test(&f.attrs) {
                self.add(f)
            } else {
                syn::visit::visit_item_fn(self, f)
            }
        }
        fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
            if is_test(&i.attrs) {
                self.add(i)
            } else {
                syn::visit::visit_item_impl(self, i)
            }
        }
        fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
            if is_test(&f.attrs) {
                self.add(f)
            } else {
                syn::visit::visit_impl_item_fn(self, f)
            }
        }
    }
    let mut ranges = Ranges(vec![]);
    ranges.visit_file(file);
    ranges.0
}
