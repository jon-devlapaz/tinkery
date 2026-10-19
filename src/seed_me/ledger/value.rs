//! Ordered Python JSON values; unrelated serde_json maps and output stay unchanged.
//! Numbers retain integer precision, and equality ignores object order but not array order.

use std::ops::{Index, IndexMut};

/// A ledger's lossless JSON value, including Python's non-finite input numbers.
#[derive(Clone, Debug)]
pub enum Value {
    /// JSON null.
    Null,
    /// A boolean, numerically equal to zero or one as in Python.
    Bool(bool),
    /// An arbitrary precision decimal integer.
    Int(String),
    /// An IEEE double, distinct from an integer when serialized.
    Float(f64),
    /// Literal Unicode text.
    String(String),
    /// Ordered entries.
    Array(Vec<Value>),
    /// Unique keys in insertion order; replacement keeps the first position.
    Object(Vec<(String, Value)>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Null, Null) => true,
            (String(a), String(b)) => a == b,
            (Array(a), Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| container_eq(a, b))
            }
            (Object(a), Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| other.get(k).is_some_and(|b| container_eq(v, b)))
            }
            (Float(a), Float(b)) => a == b,
            (a, b) => match (a.integer(), b.integer()) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            },
        }
    }
}

// json.loads uses one shared NaN constant. Python containers compare identical
// elements before invoking ==; scalar NaN equality itself remains false.
fn container_eq(a: &Value, b: &Value) -> bool {
    matches!((a,b), (Value::Float(a), Value::Float(b)) if a.is_nan() && b.is_nan()) || a == b
}

impl Value {
    fn integer(&self) -> Option<String> {
        match self {
            Self::Bool(v) => Some(if *v { "1" } else { "0" }.into()),
            Self::Int(v) => Some(v.clone()),
            Self::Float(v) if v.is_finite() && v.fract() == 0.0 => Some(if *v == 0.0 {
                "0".into()
            } else {
                format!("{v:.0}")
            }),
            _ => None,
        }
    }

    /// Parse Python JSON, retaining key positions, large integers and NaN/Infinity.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut parser = Parser { input, pos: 0 };
        if input.starts_with('\u{feff}') {
            return Err(parser.error("Unexpected UTF-8 BOM (decode using utf-8-sig)", 0));
        }
        let result = parser.value()?;
        parser.space();
        if parser.pos != input.len() {
            return Err(parser.error("Extra data", parser.pos));
        }
        Ok(result)
    }

    /// Python atomic_write's two-space JSON plus newline; non-finite or over-limit numbers refuse.
    pub fn pretty(&self) -> Result<String, String> {
        let mut out = String::new();
        self.write(&mut out, 0)?;
        out.push('\n');
        Ok(out)
    }

    /// The value of a named field, without inventing a missing value.
    pub fn get(&self, key: &str) -> Option<&Self> {
        self.object()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// An object's fields in insertion order, or None for another kind.
    pub fn object(&self) -> Option<&[(String, Self)]> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// An array's elements, or None for another kind.
    pub fn array(&self) -> Option<&[Self]> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Literal text, or None for another kind.
    pub fn string(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub(super) fn fields(&self) -> &[(String, Self)] {
        self.object().unwrap_or(&[])
    }
    pub(super) fn items(&self) -> &[Self] {
        self.array().unwrap_or(&[])
    }
    pub(super) fn is(&self, s: &str) -> bool {
        self.string() == Some(s)
    }
    pub(super) fn has(&self, k: &str) -> bool {
        self.get(k).is_some()
    }
    pub(super) fn truth(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(v) => *v,
            Self::Int(v) => v != "0",
            Self::Float(v) => *v != 0.0,
            Self::String(v) => !v.is_empty(),
            Self::Array(v) => !v.is_empty(),
            Self::Object(v) => !v.is_empty(),
        }
    }
    pub(super) fn set(&mut self, key: &str, value: Self) {
        let Self::Object(fields) = self else { return };
        if let Some((_, v)) = fields.iter_mut().find(|(k, _)| k == key) {
            *v = value;
        } else {
            fields.push((key.into(), value));
        }
    }
    pub(super) fn default(&mut self, key: &str, value: Self) {
        if !self.has(key) {
            self.set(key, value);
        }
    }
    pub(super) fn remove(&mut self, key: &str) {
        if let Self::Object(fields) = self {
            fields.retain(|(k, _)| k != key);
        }
    }
    pub(super) fn list_mut(&mut self) -> &mut Vec<Self> {
        match self {
            Self::Array(v) => v,
            _ => unreachable!("validated array"),
        }
    }
    pub(super) fn natural(&self) -> bool {
        matches!(self, Self::Int(v) if !v.starts_with('-'))
    }
    pub(super) fn cmp_int(&self, other: &Self) -> std::cmp::Ordering {
        let a = self.integer().unwrap_or_default();
        let b = other.integer().unwrap_or_default();
        a.len().cmp(&b.len()).then(a.cmp(&b))
    }
    pub(super) fn add(&self, amount: u8) -> Self {
        let Self::Int(v) = self else {
            unreachable!("validated integer")
        };
        let mut digits = v.as_bytes().to_vec();
        let mut carry = amount;
        for digit in digits.iter_mut().rev() {
            let sum = *digit - b'0' + carry;
            *digit = b'0' + sum % 10;
            carry = sum / 10;
        }
        while carry != 0 {
            digits.insert(0, b'0' + carry % 10);
            carry /= 10;
        }
        Self::Int(String::from_utf8(digits).unwrap())
    }
    pub(super) fn twice(&self) -> Self {
        let Self::Int(v) = self else {
            unreachable!("validated integer")
        };
        let mut digits = v.as_bytes().to_vec();
        let mut carry = 0;
        for digit in digits.iter_mut().rev() {
            let sum = (*digit - b'0') * 2 + carry;
            *digit = b'0' + sum % 10;
            carry = sum / 10;
        }
        if carry != 0 {
            digits.insert(0, b'0' + carry);
        }
        Self::Int(String::from_utf8(digits).unwrap())
    }
    fn write(&self, out: &mut String, depth: usize) -> Result<(), String> {
        let indent = |n| "  ".repeat(n);
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            Self::Int(v) => {
                if v.trim_start_matches('-').len() > 4300 {
                    return Err(concat!(
                        "Exceeds the limit (4300 digits) for integer string conversion; ",
                        "use sys.set_int_max_str_digits() to increase the limit"
                    )
                    .into());
                }
                out.push_str(v);
            }
            Self::Float(v) => {
                if !v.is_finite() {
                    let repr = if v.is_nan() {
                        "nan"
                    } else if v.is_sign_negative() {
                        "-inf"
                    } else {
                        "inf"
                    };
                    return Err(format!(
                        "Out of range float values are not JSON compliant: {repr}"
                    ));
                }
                out.push_str(&float_repr(*v));
            }
            Self::String(v) => out.push_str(&serde_json::to_string(v).map_err(|e| e.to_string())?),
            Self::Array(values) => {
                out.push('[');
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    out.push_str(&indent(depth + 1));
                    v.write(out, depth + 1)?;
                }
                if !values.is_empty() {
                    out.push('\n');
                    out.push_str(&indent(depth));
                }
                out.push(']');
            }
            Self::Object(values) => {
                out.push('{');
                for (i, (k, v)) in values.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('\n');
                    out.push_str(&indent(depth + 1));
                    out.push_str(&serde_json::to_string(k).map_err(|e| e.to_string())?);
                    out.push_str(": ");
                    v.write(out, depth + 1)?;
                }
                if !values.is_empty() {
                    out.push('\n');
                    out.push_str(&indent(depth));
                }
                out.push('}');
            }
        }
        Ok(())
    }
}

impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Self {
        self.get(key).unwrap_or(&Value::Null)
    }
}
impl IndexMut<&str> for Value {
    fn index_mut(&mut self, key: &str) -> &mut Self {
        let Self::Object(fields) = self else {
            unreachable!("validated object")
        };
        &mut fields
            .iter_mut()
            .find(|(k, _)| k == key)
            .expect("existing field")
            .1
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::String(v.into())
    }
}

pub(super) fn object(fields: &[(&str, Value)]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|(k, v)| ((*k).into(), v.clone()))
            .collect(),
    )
}
pub(super) fn array() -> Value {
    Value::Array(vec![])
}
pub(super) fn int(v: u64) -> Value {
    Value::Int(v.to_string())
}

fn float_repr(value: f64) -> String {
    // Formatting at a fixed precision rounds ties to even, as Python repr does.
    // Rust Debug's shortest representation instead chooses the upper tie.
    let repr = (0..=16)
        .find_map(|precision| {
            let candidate = format!("{value:.precision$e}");
            (candidate.parse::<f64>().ok().map(f64::to_bits) == Some(value.to_bits()))
                .then_some(candidate)
        })
        .expect("17 significant digits round-trip an IEEE double");
    let (negative, repr) = repr
        .strip_prefix('-')
        .map_or((false, repr.as_str()), |s| (true, s));
    let (mantissa, exponent) = repr
        .split_once('e')
        .map_or((repr, 0), |(m, e)| (m, e.parse::<i32>().unwrap()));
    let dot = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let raw: String = mantissa.chars().filter(|c| *c != '.').collect();
    let leading = raw.len() - raw.trim_start_matches('0').len();
    let digits = raw.trim_start_matches('0').trim_end_matches('0');
    if digits.is_empty() {
        return if negative { "-0.0" } else { "0.0" }.into();
    }
    let exp = exponent + dot - leading as i32 - 1;
    let mut out = if negative { "-".into() } else { String::new() };
    if !(-4..16).contains(&exp) {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push_str(&format!("e{exp:+03}"));
    } else {
        let pos = exp + 1;
        if pos <= 0 {
            out.push_str("0.");
            out.push_str(&"0".repeat((-pos) as usize));
            out.push_str(digits);
        } else if pos as usize >= digits.len() {
            out.push_str(digits);
            out.push_str(&"0".repeat(pos as usize - digits.len()));
            out.push_str(".0");
        } else {
            out.push_str(&digits[..pos as usize]);
            out.push('.');
            out.push_str(&digits[pos as usize..]);
        }
    }
    out
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}
impl Parser<'_> {
    fn error(&self, message: &str, pos: usize) -> String {
        let prefix = &self.input[..pos];
        let line = prefix.bytes().filter(|c| *c == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        format!(
            "{message}: line {line} column {column} (char {})",
            prefix.chars().count()
        )
    }
    fn space(&mut self) {
        while self
            .input
            .as_bytes()
            .get(self.pos)
            .is_some_and(|c| b" \r\n\t".contains(c))
        {
            self.pos += 1;
        }
    }
    fn string(&mut self) -> Result<String, String> {
        let start = self.pos;
        self.pos += 1;
        while let Some(c) = self.input.as_bytes().get(self.pos) {
            self.pos += 1;
            if *c == b'\\' {
                self.pos += 1;
            } else if *c == b'"' {
                return serde_json::from_str(&self.input[start..self.pos])
                    .map_err(|e| e.to_string());
            }
        }
        Err(self.error("Unterminated string starting at", start))
    }
    fn value(&mut self) -> Result<Value, String> {
        self.space();
        let Some(c) = self.input.as_bytes().get(self.pos).copied() else {
            return Err(self.error("Expecting value", self.pos));
        };
        match c {
            b'"' => Ok(Value::String(self.string()?)),
            b'{' | b'[' => {
                self.pos += 1;
                let mut result = if c == b'{' {
                    Value::Object(vec![])
                } else {
                    array()
                };
                let close = if c == b'{' { b'}' } else { b']' };
                self.space();
                if self.input.as_bytes().get(self.pos) == Some(&close) {
                    self.pos += 1;
                    return Ok(result);
                }
                loop {
                    self.space();
                    if c == b'{' {
                        if self.input.as_bytes().get(self.pos) != Some(&b'"') {
                            return Err(self.error(
                                "Expecting property name enclosed in double quotes",
                                self.pos,
                            ));
                        }
                        let key = self.string()?;
                        self.space();
                        if self.input.as_bytes().get(self.pos) != Some(&b':') {
                            return Err(self.error("Expecting ':' delimiter", self.pos));
                        }
                        self.pos += 1;
                        result.set(&key, self.value()?);
                    } else {
                        result.list_mut().push(self.value()?);
                    }
                    self.space();
                    match self.input.as_bytes().get(self.pos) {
                        Some(v) if *v == close => {
                            self.pos += 1;
                            return Ok(result);
                        }
                        Some(b',') => {
                            let comma = self.pos;
                            self.pos += 1;
                            self.space();
                            if self.input.as_bytes().get(self.pos) == Some(&close) {
                                let kind = if c == b'{' { "object" } else { "array" };
                                return Err(self.error(
                                    &format!("Illegal trailing comma before end of {kind}"),
                                    comma,
                                ));
                            }
                        }
                        _ => return Err(self.error("Expecting ',' delimiter", self.pos)),
                    }
                }
            }
            _ => {
                let remaining = &self.input[self.pos..];
                static NUMBER: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
                let pattern = NUMBER.get_or_init(|| {
                    regex::Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?").unwrap()
                });
                let token = if let Some(keyword) =
                    ["null", "true", "false", "NaN", "Infinity", "-Infinity"]
                        .iter()
                        .find(|word| remaining.starts_with(**word))
                {
                    *keyword
                } else if let Some(number) = pattern.find(remaining) {
                    number.as_str()
                } else {
                    return Err(self.error("Expecting value", self.pos));
                };
                self.pos += token.len();
                match token {
                    "null" => Ok(Value::Null),
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    "NaN" => Ok(Value::Float(f64::NAN)),
                    "Infinity" => Ok(Value::Float(f64::INFINITY)),
                    "-Infinity" => Ok(Value::Float(f64::NEG_INFINITY)),
                    _ => {
                        if token.contains(['.', 'e', 'E']) {
                            token
                                .parse()
                                .map(Value::Float)
                                .map_err(|_| "invalid JSON float".into())
                        } else {
                            let digits = token.trim_start_matches('-').len();
                            if digits > 4300 {
                                return Err(format!(
                                    concat!(
                                        "Exceeds the limit (4300 digits) for integer string conversion: ",
                                        "value has {} digits; use sys.set_int_max_str_digits() to increase the limit"
                                    ),
                                    digits
                                ));
                            }
                            Ok(Value::Int(if token == "-0" { "0" } else { token }.into()))
                        }
                    }
                }
            }
        }
    }
}
