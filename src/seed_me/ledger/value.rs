//! Ordered ledger JSON values; unrelated serde_json maps and output stay unchanged.
//! Integers are exact up to 128 bits, and equality ignores object order but not array order.

use std::ops::{Index, IndexMut};

/// A ledger's JSON value, including the non-finite numbers that old helpers accepted on input.
/// Output refuses those numbers.
#[derive(Clone, Debug)]
pub enum Value {
    /// JSON null.
    Null,
    /// A boolean, never equal to a number.
    Bool(bool),
    /// An integer; parsing refuses one outside 128 bits.
    Int(i128),
    /// An IEEE double, distinct from an integer when serialized.
    Float(f64),
    /// Literal Unicode text.
    String(String),
    /// Ordered entries.
    Array(Vec<Value>),
    /// Unique keys in insertion order; replacement keeps the first position.
    Object(Vec<(String, Value)>),
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(v) => Self::Bool(v),
            serde_json::Value::String(v) => Self::String(v),
            serde_json::Value::Array(v) => Self::Array(v.into_iter().map(Self::from).collect()),
            serde_json::Value::Object(v) => {
                Self::Object(v.into_iter().map(|(k, v)| (k, Self::from(v))).collect())
            }
            serde_json::Value::Number(v) => match (v.as_i64(), v.as_u64()) {
                (Some(v), _) => Self::Int(v.into()),
                (_, Some(v)) => Self::Int(v.into()),
                _ => Self::Float(v.as_f64().expect("JSON number")),
            },
        }
    }
}

impl Value {
    /// Convert to the application's JSON type without serializing through text. Object keys
    /// come out sorted. Refuses non-finite numbers.
    pub fn to_json(&self) -> Result<serde_json::Value, String> {
        Ok(match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(v) => (*v).into(),
            Self::String(v) => v.clone().into(),
            // Beyond 64 bits, the application's JSON holds the nearest float.
            Self::Int(v) => match (i64::try_from(*v), u64::try_from(*v)) {
                (Ok(v), _) => v.into(),
                (_, Ok(v)) => v.into(),
                _ => serde_json::Value::Number(
                    serde_json::Number::from_f64(*v as f64).expect("finite integer"),
                ),
            },
            Self::Float(v) => serde_json::Value::Number(
                serde_json::Number::from_f64(*v).ok_or("Ledger number must be finite.")?,
            ),
            Self::Array(v) => {
                serde_json::Value::Array(v.iter().map(Self::to_json).collect::<Result<_, _>>()?)
            }
            Self::Object(v) => serde_json::Value::Object(
                v.iter()
                    .map(|(k, v)| Ok((k.clone(), v.to_json()?)))
                    .collect::<Result<_, String>>()?,
            ),
        })
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Null, Null) => true,
            (Bool(a), Bool(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => a == b,
            (String(a), String(b)) => a == b,
            (Array(a), Array(b)) => a == b,
            (Object(a), Object(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| other.get(k) == Some(v))
            }
            _ => false,
        }
    }
}

impl Value {
    /// Parse ledger JSON, retaining key positions; NaN and Infinity read as floats.
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

    /// Two-space JSON plus newline, with literal Unicode; non-finite numbers refuse.
    pub fn pretty(&self) -> Result<String, String> {
        let mut out = String::new();
        self.write(&mut out, 0)?;
        out.push('\n');
        Ok(out)
    }

    /// Compact JSON with sorted keys, as digests hash it; `spaced` separates with ", " and ": ".
    pub(super) fn canonical(&self, spaced: bool) -> Result<String, String> {
        let json = self.to_json()?;
        if !spaced {
            return Ok(json.to_string());
        }
        struct Spaced;
        impl serde_json::ser::Formatter for Spaced {
            fn begin_array_value<W: ?Sized + std::io::Write>(
                &mut self,
                w: &mut W,
                first: bool,
            ) -> std::io::Result<()> {
                if first { Ok(()) } else { w.write_all(b", ") }
            }
            fn begin_object_key<W: ?Sized + std::io::Write>(
                &mut self,
                w: &mut W,
                first: bool,
            ) -> std::io::Result<()> {
                if first { Ok(()) } else { w.write_all(b", ") }
            }
            fn begin_object_value<W: ?Sized + std::io::Write>(
                &mut self,
                w: &mut W,
            ) -> std::io::Result<()> {
                w.write_all(b": ")
            }
        }
        let mut out = vec![];
        serde::Serialize::serialize(
            &json,
            &mut serde_json::Serializer::with_formatter(&mut out, Spaced),
        )
        .map_err(|e| e.to_string())?;
        String::from_utf8(out).map_err(|e| e.to_string())
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

    /// A nonnegative integer that fits a u64, such as a version a caller displays.
    pub fn counter(&self) -> Option<u64> {
        match self {
            Self::Int(v) => u64::try_from(*v).ok(),
            _ => None,
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
            Self::Int(v) => *v != 0,
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
        matches!(self, Self::Int(v) if *v >= 0)
    }
    /// Integer order; any other value sorts before every integer.
    pub(super) fn cmp_int(&self, other: &Self) -> std::cmp::Ordering {
        let number = |v: &Self| match v {
            Self::Int(v) => Some(*v),
            _ => None,
        };
        number(self).cmp(&number(other))
    }
    /// A validated counter plus `amount`, saturating at the largest stored integer.
    pub(super) fn add(&self, amount: u8) -> Self {
        let Self::Int(v) = self else {
            unreachable!("validated integer")
        };
        Self::Int(v.saturating_add(amount.into()))
    }
    /// A validated counter doubled, saturating at the largest stored integer.
    pub(super) fn twice(&self) -> Self {
        let Self::Int(v) = self else {
            unreachable!("validated integer")
        };
        Self::Int(v.saturating_mul(2))
    }
    fn write(&self, out: &mut String, depth: usize) -> Result<(), String> {
        let indent = |n| "  ".repeat(n);
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            Self::Int(v) => out.push_str(&v.to_string()),
            Self::Float(v) => out.push_str(
                &serde_json::Number::from_f64(*v)
                    .ok_or("Ledger number must be finite.")?
                    .to_string(),
            ),
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
    Value::Int(v.into())
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
                let start = self.pos;
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
                            token
                                .parse()
                                .map(Value::Int)
                                .map_err(|_| self.error("Ledger number is out of range", start))
                        }
                    }
                }
            }
        }
    }
}
