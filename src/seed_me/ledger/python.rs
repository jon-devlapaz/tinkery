//! Nonblank text, strict receipt times and secret screening; receipt commands remain inert data.

use super::value::Value;
use std::sync::OnceLock;

pub(super) fn whitespace(c: char) -> bool {
    matches!(c, '\t'..='\r' | '\u{1c}'..='\u{20}' | '\u{85}' | '\u{a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}
pub(super) fn text(v: &Value) -> bool {
    v.string()
        .is_some_and(|s| s.chars().any(|c| !whitespace(c)))
}

pub(super) fn secret(input: &str) -> bool {
    static PATTERNS: OnceLock<Vec<(regex::Regex, bool, bool)>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        let space = r"[\x09-\x0d\x1c-\x20\x85\x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}]";
        [
            ("-----begin [a-z ]*private key-----", false, false),
            ("akia[0-9a-z]{16}", true, true),
            ("gh[pousr]_[a-z0-9]{20,}", true, false),
            ("sk-[a-z0-9_-]{20,}", true, false),
            ("xox[abprs]-[a-z0-9-]{10,}", true, false),
            (r"://[^/\s:@]+:[^/\s@]+@", false, false),
            (r"(?:password|passwd|secret|token|api[_-]?key)\s*=\s*\S{6,}", true, false),
            (r"bearer\s+[a-z0-9._~+/=-]{16,}", true, false),
        ].into_iter().map(|(p, before, after)| {
            let p = p.replace(r"\S", &format!("[^{0}]", &space[1..space.len()-1]))
                .replace(r"\s", space);
            (regex::Regex::new(&p).unwrap(), before, after)
        }).collect()
    });
    let normalized: String = input
        .chars()
        .map(|c| match c {
            '\u{130}' | '\u{131}' => 'i',
            '\u{17f}' => 's',
            '\u{212a}' => 'k',
            c if c.is_ascii_uppercase() => c.to_ascii_lowercase(),
            c => c,
        })
        .collect();
    patterns.iter().any(|(regex, before, after)| {
        // Try every start: a failed trailing boundary must not hide an overlapping match.
        normalized.char_indices().any(|(start, _)| {
            regex.find_at(&normalized, start).is_some_and(|m| {
                (!before
                    || !normalized[..m.start()]
                        .chars()
                        .next_back()
                        .is_some_and(word))
                    && (!after || !normalized[m.end()..].chars().next().is_some_and(word))
            })
        })
    })
}
fn word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}
pub(super) fn printable(c: char) -> bool {
    static NONPRINTABLE: OnceLock<regex::Regex> = OnceLock::new();
    c == ' '
        || !NONPRINTABLE
            .get_or_init(|| regex::Regex::new(r"[\p{C}\p{Z}]").unwrap())
            .is_match(&c.to_string())
}

pub(super) fn timestamp(input: &str) -> bool {
    time::OffsetDateTime::parse(input, &time::format_description::well_known::Rfc3339).is_ok()
}
