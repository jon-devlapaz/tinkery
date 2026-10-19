//! Python's text, receipt-time and secret boundaries; receipt commands remain inert data.

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
    c == '_'
        || super::unicode::ALNUM
            .binary_search_by(|&(lo, hi)| {
                if (c as u32) < lo {
                    std::cmp::Ordering::Greater
                } else if (c as u32) > hi {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok()
}

pub(super) fn timestamp(input: &str) -> bool {
    timestamp_parts(input).is_some()
}
fn timestamp_parts(input: &str) -> Option<()> {
    let input = input.trim_matches(whitespace).replace('Z', "+00:00");
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    if len < 8 {
        return None;
    }
    let separator = if chars[4] == '-' {
        if chars[5] == 'W' {
            if len < 8 {
                return None;
            }
            if len > 8 && chars[8] == '-' {
                if len == 9 {
                    return None;
                }
                if len > 10 && chars[10].is_ascii_digit() {
                    8
                } else {
                    10
                }
            } else {
                8
            }
        } else {
            10
        }
    } else if chars[4] == 'W' {
        let mut i = 7;
        while i < len && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i < 9 {
            i
        } else if i % 2 == 0 {
            7
        } else {
            8
        }
    } else {
        8
    };
    if separator >= len {
        return None;
    }
    date(chars.get(..separator)?)?;
    let time: String = chars[separator + 1..].iter().collect();
    let zone = time.find(['-', '+'])?;
    let (h, m, s, _) = clock(&time[..zone])?;
    if h > 23 || m > 59 || s > 59 {
        return None;
    }
    let (h, m, s, fraction) = clock(&time[zone + 1..])?;
    // Offset minute/second overflow is normalized by timedelta, not rejected.
    if h * 3600 + m * 60 + s >= 86400 || (h * 3600 + m * 60 + s == 86400 && fraction) {
        return None;
    }
    Some(())
}
fn digits(s: &[char]) -> Option<u32> {
    s.iter().try_fold(0, |v, c| {
        c.is_ascii_digit().then(|| v * 10 + *c as u32 - '0' as u32)
    })
}
fn date(s: &[char]) -> Option<()> {
    if ![7, 8, 10].contains(&s.len()) {
        return None;
    }
    let year = digits(s.get(..4)?)?;
    if !(1..=9999).contains(&year) {
        return None;
    }
    let sep = s[4] == '-';
    let mut pos = 4 + usize::from(sep);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    if s[pos] == 'W' {
        pos += 1;
        let week = digits(s.get(pos..pos + 2)?)?;
        pos += 2;
        let day = if s.len() > pos {
            if (s[pos] == '-') != sep {
                return None;
            }
            pos += usize::from(sep);
            let d = digits(s.get(pos..pos + 1)?)?;
            pos += 1;
            d
        } else {
            1
        };
        let y = year - 1;
        let jan1 = (365 * y + y / 4 - y / 100 + y / 400) % 7; // Monday = 0.
        if week == 0
            || week > 53
            || (week == 53 && jan1 != 3 && !(jan1 == 2 && leap))
            || !(1..=7).contains(&day)
        {
            return None;
        }
        let monday = 1 - jan1 as i32 + if jan1 > 3 { 7 } else { 0 };
        let ordinal = monday + (week as i32 - 1) * 7 + day as i32 - 1;
        if (year == 1 && ordinal < 1) || (year == 9999 && ordinal > 365) {
            return None;
        }
    } else {
        let month = digits(s.get(pos..pos + 2)?)?;
        pos += 2;
        if (s.get(pos) == Some(&'-')) != sep {
            return None;
        }
        pos += usize::from(sep);
        let day = digits(s.get(pos..pos + 2)?)?;
        pos += 2;
        let max = match month {
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => return None,
        };
        if day < 1 || day > max {
            return None;
        }
    }
    (pos == s.len()).then_some(())
}
fn clock(s: &str) -> Option<(u32, u32, u32, bool)> {
    let s: Vec<char> = s.chars().collect();
    let mut values = [0; 3];
    let mut pos = 0;
    let mut colon = false;
    for (i, value) in values.iter_mut().enumerate() {
        *value = digits(s.get(pos..pos + 2)?)?;
        pos += 2;
        if i == 0 {
            colon = s.get(pos) == Some(&':');
        }
        if pos == s.len() || i >= 2 {
            break;
        }
        // CPython's C parser permits fractional hours and minutes too.
        if matches!(s[pos], '.' | ',') {
            break;
        }
        if colon {
            if s[pos] != ':' {
                return None;
            }
            pos += 1;
        }
    }
    let mut fraction = false;
    if pos < s.len() {
        if !matches!(s[pos], '.' | ',') {
            return None;
        }
        pos += 1;
        if !s[pos..].iter().all(char::is_ascii_digit) {
            return None;
        }
        fraction = s[pos..].iter().take(6).any(|c| *c != '0');
    }
    Some((values[0], values[1], values[2], fraction))
}
