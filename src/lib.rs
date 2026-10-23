//! Zero-dependency bounded JSON parser, value tree, and compact/pretty serializer.

use std::fmt;

/// Default maximum nesting depth for JSON arrays and objects (`64`).
pub const MAX_JSON_DEPTH: usize = 64;

/// Default maximum input payload size in bytes (`16 MiB`).
pub const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub message: &'static str,
    pub offset: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for JsonError {}

impl JsonValue {
    pub fn empty_object() -> Self {
        Self::Object(Vec::new())
    }

    pub fn empty_array() -> Self {
        Self::Array(Vec::new())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|n| n as i64)
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().and_then(|n| {
            if n >= 0.0 && n.is_finite() {
                Some(n as u64)
            } else {
                None
            }
        })
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        let entries = self.as_object()?;
        entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Self::as_str)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Self::as_bool)
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Self::as_i64)
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(Self::as_u64)
    }

    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        self.get(key).and_then(Self::as_array)
    }

    pub fn get_object(&self, key: &str) -> Option<&[(String, JsonValue)]> {
        self.get(key).and_then(Self::as_object)
    }

    pub fn get_index(&self, index: usize) -> Option<&JsonValue> {
        self.as_array()?.get(index)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: JsonValue) {
        let key = key.into();
        if let Self::Object(entries) = self {
            if let Some((_, slot)) = entries.iter_mut().find(|(k, _)| *k == key) {
                *slot = value;
            } else {
                entries.push((key, value));
            }
        }
    }

    pub fn push(&mut self, value: JsonValue) {
        if let Self::Array(items) = self {
            items.push(value);
        }
    }
}

/// Parse a JSON document string into a [`JsonValue`] using default security bounds
/// ([`MAX_JSON_DEPTH`] and [`MAX_JSON_BYTES`]).
pub fn parse_json(source: &str) -> Result<JsonValue, JsonError> {
    parse_json_bounded(source, MAX_JSON_DEPTH, MAX_JSON_BYTES)
}

/// Parse a JSON document string with explicit nesting depth and byte-size bounds.
pub fn parse_json_bounded(
    source: &str,
    max_depth: usize,
    max_bytes: usize,
) -> Result<JsonValue, JsonError> {
    if source.len() > max_bytes {
        return Err(JsonError {
            message: "JSON payload exceeds maximum allowed byte length",
            offset: max_bytes,
        });
    }
    let mut parser = Parser {
        bytes: source.as_bytes(),
        source,
        pos: 0,
        depth: 0,
        max_depth,
    };
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(parser.err("trailing characters after JSON value"));
    }
    Ok(value)
}

/// Serialize a [`JsonValue`] into a compact single-line JSON string.
pub fn to_json_string(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value(value, &mut out, None, 0);
    out
}

/// Serialize a [`JsonValue`] into a 2-space indented multi-line JSON string.
pub fn to_json_pretty(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value(value, &mut out, Some(2), 0);
    out
}

fn write_value(value: &JsonValue, out: &mut String, indent: Option<usize>, depth: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(n) => {
            if !n.is_finite() {
                out.push_str("null");
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{n}"));
            }
        }
        JsonValue::String(s) => write_escaped_string(s, out),
        JsonValue::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                if let Some(step) = indent {
                    out.push('\n');
                    for _ in 0..(depth + 1) * step {
                        out.push(' ');
                    }
                }
                write_value(item, out, indent, depth + 1);
            }
            if let Some(step) = indent {
                out.push('\n');
                for _ in 0..depth * step {
                    out.push(' ');
                }
            }
            out.push(']');
        }
        JsonValue::Object(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (idx, (k, v)) in entries.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                if let Some(step) = indent {
                    out.push('\n');
                    for _ in 0..(depth + 1) * step {
                        out.push(' ');
                    }
                }
                write_escaped_string(k, out);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write_value(v, out, indent, depth + 1);
            }
            if let Some(step) = indent {
                out.push('\n');
                for _ in 0..depth * step {
                    out.push(' ');
                }
            }
            out.push('}');
        }
    }
}

fn write_escaped_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    source: &'a str,
    pos: usize,
    depth: usize,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, message: &'static str) -> JsonError {
        JsonError {
            message,
            offset: self.pos,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn enter_container(&mut self) -> Result<(), JsonError> {
        if self.depth >= self.max_depth {
            return Err(self.err("JSON nesting depth exceeds maximum allowed limit"));
        }
        self.depth += 1;
        Ok(())
    }

    fn leave_container(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        let b = self
            .peek()
            .ok_or_else(|| self.err("unexpected end of JSON input"))?;
        match b {
            b'n' => self.parse_literal(b"null", JsonValue::Null),
            b't' => self.parse_literal(b"true", JsonValue::Bool(true)),
            b'f' => self.parse_literal(b"false", JsonValue::Bool(false)),
            b'"' => self.parse_string().map(JsonValue::String),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(self.err("unexpected character in JSON value")),
        }
    }

    fn parse_literal(&mut self, literal: &[u8], value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.bytes.get(self.pos..self.pos + literal.len()) == Some(literal) {
            self.pos += literal.len();
            Ok(value)
        } else {
            Err(self.err("invalid literal token"))
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        if self.peek() != Some(b'"') {
            return Err(self.err("expected '\"' at start of string"));
        }
        self.pos += 1;
        let mut out = String::new();

        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Ok(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = *self
                    .bytes
                    .get(self.pos)
                    .ok_or_else(|| self.err("unterminated escape sequence"))?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\x08'),
                    b'f' => out.push('\x0C'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let code = self.read_hex_u16()?;
                        if (0xD800..=0xDBFF).contains(&code) {
                            if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                return Err(self.err("missing low surrogate in UTF-16 escape"));
                            }
                            self.pos += 2;
                            let low = self.read_hex_u16()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(self.err("invalid low surrogate in UTF-16 escape"));
                            }
                            let combined = 0x10000
                                + (((u32::from(code) - 0xD800) << 10) | (u32::from(low) - 0xDC00));
                            let ch = char::from_u32(combined)
                                .ok_or_else(|| self.err("invalid Unicode scalar"))?;
                            out.push(ch);
                        } else {
                            let ch = char::from_u32(u32::from(code))
                                .ok_or_else(|| self.err("invalid Unicode escape"))?;
                            out.push(ch);
                        }
                    }
                    _ => return Err(self.err("unknown string escape character")),
                }
            } else if b < 0x20 {
                return Err(self.err("unescaped control character in string"));
            } else {
                let tail = &self.source[self.pos..];
                let ch = tail
                    .chars()
                    .next()
                    .ok_or_else(|| self.err("invalid UTF-8 in string"))?;
                self.pos += ch.len_utf8();
                out.push(ch);
            }
        }
        Err(self.err("unterminated string literal"))
    }

    fn read_hex_u16(&mut self) -> Result<u16, JsonError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.err("short \\uXXXX hex escape"));
        }
        let hex = &self.source[self.pos..self.pos + 4];
        let val = u16::from_str_radix(hex, 16).map_err(|_| self.err("invalid hex digit"))?;
        self.pos += 4;
        Ok(val)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.err("invalid number literal")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit after decimal point"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit in exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let slice = &self.source[start..self.pos];
        let num: f64 = slice
            .parse()
            .map_err(|_| self.err("failed to parse number"))?;
        Ok(JsonValue::Number(num))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.enter_container()?;
        self.pos += 1; // '['
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            self.leave_container();
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    self.leave_container();
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(self.err("expected ',' or ']' in array")),
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.enter_container()?;
        self.pos += 1; // '{'
        self.skip_whitespace();
        let mut entries = Vec::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            self.leave_container();
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(self.err("expected ':' after object key"));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    self.leave_container();
                    return Ok(JsonValue::Object(entries));
                }
                _ => return Err(self.err("expected ',' or '}' in object")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_round_trips_nested_objects_arrays_and_escapes() {
        let src = r#"{
            "browser": "Flux",
            "version": 3,
            "active": true,
            "nothing": null,
            "bookmarks": [
                { "title": "Home\nPage", "url": "https://example.com" },
                { "title": "Unicode \u0046\u006c\u0075\u0078", "url": "file:///index.html" }
            ]
        }"#;
        let doc = parse_json(src).unwrap();
        assert_eq!(doc.get_str("browser"), Some("Flux"));
        assert_eq!(doc.get_i64("version"), Some(3));
        assert_eq!(doc.get_u64("version"), Some(3));
        assert_eq!(doc.get_bool("active"), Some(true));
        assert!(doc.get("nothing").unwrap().is_null());

        let bookmarks = doc.get("bookmarks").unwrap();
        assert_eq!(
            bookmarks.get_index(0).and_then(|b| b.get_str("title")),
            Some("Home\nPage")
        );
        assert_eq!(
            bookmarks.get_index(1).and_then(|b| b.get_str("title")),
            Some("Unicode Flux")
        );

        let compact = to_json_string(&doc);
        let reparsed = parse_json(&compact).unwrap();
        assert_eq!(reparsed, doc);

        let pretty = to_json_pretty(&doc);
        assert_eq!(parse_json(&pretty).unwrap(), doc);
    }

    #[test]
    fn rejects_excessive_nesting_depth_and_oversized_payloads() {
        let deep = "[".repeat(MAX_JSON_DEPTH + 2) + &"]".repeat(MAX_JSON_DEPTH + 2);
        let err = parse_json(&deep).unwrap_err();
        assert!(err.message.contains("depth"));

        let oversized = format!("\"{}\"", "a".repeat(128));
        let size_err = parse_json_bounded(&oversized, 16, 64).unwrap_err();
        assert!(size_err.message.contains("byte length"));
    }
}
