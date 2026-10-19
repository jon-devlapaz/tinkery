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
    // CPython v3.13.16, Modules/_datetimemodule.c: the C parser's byte cursor
    // and end pointer are distinct; slicing at the zone loses its leniencies.
    let input = input.trim_matches(whitespace).replace('Z', "+00:00");
    if input.chars().count() < 7 {
        return None;
    }
    let bytes = input.as_bytes();
    let separator = date_separator(bytes)?;
    date(bytes, separator)?;
    let separator_width = input.get(separator..)?.chars().next()?.len_utf8();
    let start = separator + separator_width;
    let zone = (start..bytes.len()).find(|&i| matches!(bytes[i], b'Z' | b'+' | b'-'))?;
    let (time, _) = clock(bytes, start, zone)?;
    if time[0] > 23 || time[1] > 59 || time[2] > 59 {
        return None;
    }
    if bytes[zone] == b'Z' {
        return (zone + 1 == bytes.len()).then_some(());
    }
    let (offset, trailing) = clock(bytes, zone + 1, bytes.len())?;
    // timedelta normalizes offset minutes/seconds; timezone requires < 24 hours.
    // A zero-second offset becomes UTC even when fractional microseconds exist.
    (!trailing && offset[0] * 3600 + offset[1] * 60 + offset[2] < 86400).then_some(())
}
fn byte(input: &[u8], pos: usize) -> u8 {
    input.get(pos).copied().unwrap_or(0)
}
fn date_separator(input: &[u8]) -> Option<usize> {
    let len = input.len();
    if len == 7 {
        return Some(7);
    }
    Some(if input[4] == b'-' {
        if input[5] == b'W' {
            if len < 8 {
                return None;
            }
            if len > 8 && input[8] == b'-' {
                if len == 9 {
                    return None;
                }
                if len > 10 && input[10].is_ascii_digit() {
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
    } else if input[4] == b'W' {
        let mut i = 7;
        while i < len && input[i].is_ascii_digit() {
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
    })
}
fn digits(input: &[u8], pos: &mut usize, count: usize) -> Option<u32> {
    let mut value = 0;
    for _ in 0..count {
        let digit = byte(input, *pos);
        if !digit.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(digit - b'0');
        *pos += 1;
    }
    Some(value)
}
fn date(input: &[u8], end: usize) -> Option<()> {
    let mut pos = 0;
    let year = digits(input, &mut pos, 4)?;
    if !(1..=9999).contains(&year) {
        return None;
    }
    let separated = byte(input, pos) == b'-';
    pos += usize::from(separated);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    if byte(input, pos) == b'W' {
        pos += 1;
        let week = digits(input, &mut pos, 2)?;
        let day = if pos < end {
            if separated {
                if byte(input, pos) != b'-' {
                    return None;
                }
                pos += 1;
            }
            digits(input, &mut pos, 1)?
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
        let month = digits(input, &mut pos, 2)?;
        if separated {
            if byte(input, pos) != b'-' {
                return None;
            }
            pos += 1;
        }
        let day = digits(input, &mut pos, 2)?;
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
    Some(())
}
fn clock(input: &[u8], mut pos: usize, end: usize) -> Option<([u32; 4], bool)> {
    let mut values = [0; 4];
    let mut separated = true;
    for (i, value) in values[..3].iter_mut().enumerate() {
        *value = digits(input, &mut pos, 2)?;
        let c = byte(input, pos);
        pos += 1;
        if i == 0 {
            separated = c == b':';
        }
        // The C parser consumes this byte before testing its end pointer.
        // Since v3.13.16 a fraction separator needs at least one digit after it.
        if pos >= end {
            return (!matches!(c, b'.' | b',')).then_some((values, c != 0));
        }
        if separated && c == b':' {
            continue;
        }
        if matches!(c, b'.' | b',') {
            break;
        }
        if !separated {
            pos -= 1;
        } else {
            return None;
        }
    }
    let count = end.checked_sub(pos)?.min(6);
    values[3] = digits(input, &mut pos, count)? * 10_u32.pow(6 - count as u32);
    while byte(input, pos).is_ascii_digit() {
        pos += 1;
    }
    Some((values, byte(input, pos) != 0))
}
