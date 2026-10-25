//! File-and-line receipts: recognize complete numeric ranges and flag unavailable lines.
//! Reported paths share this grammar; neither path nor line existence proves a file was read
//! or that its contents support the finding. Receipt words are never rewritten.

use std::path::Path;

struct Citation<'a> {
    path: &'a str,
    ranges: Option<Vec<(u64, u64)>>,
}

// A filename and numeric start identify a candidate. Parse its entire range token below;
// matching only a valid prefix would hide malformed suffixes and later abbreviated ranges.
fn start() -> &'static regex::Regex {
    static START: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?:^|[\s,;(\[`])([\w./-]*\w\.\w+):([+-]?[0-9])")
            .expect("the citation start pattern compiles")
    });
    &START
}

fn parse(checked: &str) -> impl Iterator<Item = Citation<'_>> {
    start().captures_iter(checked).map(|capture| Citation {
        path: capture.get(1).unwrap().as_str(),
        ranges: ranges(&checked[capture.get(2).unwrap().start()..]),
    })
}

fn ranges(mut remaining: &str) -> Option<Vec<(u64, u64)>> {
    let mut ranges = Vec::new();
    loop {
        let mut end = token_end(remaining);
        // Spaces around a dash still join its two bounds. Do not establish only the
        // first line of `3 – 29`; a missing or malformed second bound is a warning.
        loop {
            let tail = &remaining[end..];
            let next = tail.trim_start();
            if tail.len() == next.len()
                || !(next.starts_with(['-', '–', '—'])
                    || remaining[..end].ends_with(['-', '–', '—']))
            {
                break;
            }
            if let Some(dash) = next.chars().next().filter(|c| ['-', '–', '—'].contains(c)) {
                let after = next[dash.len_utf8()..].trim_start();
                if !after.is_empty()
                    && !after.starts_with(|c: char| c.is_ascii_digit() || ",-–—+".contains(c))
                {
                    break; // A spaced dash followed by prose ends the citation.
                }
            }
            let length = token_end(next);
            if length == 0 {
                break;
            }
            end = remaining.len() - next.len() + length;
        }
        // Sentence punctuation may follow a citation. Punctuation inside a numeric token
        // still fails parsing, as do incomplete or repeated dashes and numeric overflow.
        let token = remaining[..end].trim_end_matches(['.', ';', ':', '!', '?']);
        let mut bounds = token.split(['-', '–', '—']).map(str::trim);
        let number = |text: &str| {
            text.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| text.parse::<u64>().ok())
                .flatten()
        };
        let first = number(bounds.next()?)?;
        let last = bounds.next().map_or(Some(first), number)?;
        if first == 0 || last < first || bounds.next().is_some() {
            return None;
        }
        ranges.push((first, last));
        let Some(next) = remaining[end..].trim_start().strip_prefix(',') else {
            return Some(ranges);
        };
        let next = next.trim_start();
        // A numeric continuation belongs to this file. Prose or another filename ends
        // this citation; malformed numeric continuations must not disappear as prose.
        if start()
            .find(&next[..token_end(next)])
            .is_some_and(|found| found.start() == 0)
            || !next.starts_with(|c: char| c.is_ascii_digit() || ",-–—+".contains(c))
        {
            return Some(ranges);
        }
        remaining = next;
    }
}

fn token_end(text: &str) -> usize {
    text.find(|c: char| c.is_whitespace() || ",;)]}`\"".contains(c))
        .unwrap_or(text.len())
}

/// Paths named by well-formed line citations, or no attribution when any candidate is malformed.
/// This uses recorded words only; it does not reread files or verify their origin.
pub(super) fn paths(checked: &str) -> Option<Vec<&str>> {
    let mut paths = Vec::new();
    for cite in parse(checked) {
        cite.ranges?;
        paths.push(cite.path);
    }
    (!paths.is_empty()).then_some(paths)
}

/// The first unavailable citation, checking every range against a regular readable file in
/// `folder`. Plain prose is unjudged; valid citations establish line existence only.
pub(super) fn unfound(checked: &str, folder: &Path) -> Option<String> {
    let root = folder.canonicalize();
    parse(checked).find_map(|cite| {
        let path = cite.path;
        let Some(ranges) = cite.ranges else {
            return Some(format!("{path} has an invalid line range"));
        };
        let Ok(root) = &root else {
            return Some("the folder could not be read".into());
        };
        let Ok(file) = root.join(path).canonicalize() else {
            return Some(format!("{path} isn't in the folder"));
        };
        if !file.starts_with(root) {
            return Some(format!("{path} is outside the folder"));
        }
        let unreadable = || format!("{path} is not a readable file");
        if !file.is_file() {
            return Some(unreadable());
        }
        let Ok(bytes) = std::fs::read(file) else {
            return Some(unreadable());
        };
        let lines = bytes.iter().filter(|b| **b == b'\n').count() as u64
            + u64::from(!bytes.is_empty() && !bytes.ends_with(b"\n"));
        ranges.into_iter().find_map(|(_, last)| {
            (last > lines).then(|| format!("{path} has {lines} lines, not {last}"))
        })
    })
}
