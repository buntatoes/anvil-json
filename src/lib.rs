//! JSON parser and serializer (RFC 8259), with a depth cap and a byte cap.
//!
//! The input is one JSON text. `parse_json` uses the default caps and returns
//! a `JsonValue`, or a `JsonError` with a static message and a byte offset.
//! `parse_json_bounded` takes the caps as arguments. Nesting of arrays and
//! objects stops at `max_depth`. The default, `MAX_JSON_DEPTH`, is 64. The
//! UTF-8 byte length stops at `max_bytes`. The default, `MAX_JSON_BYTES`, is
//! 16 MiB. Whitespace after the value is skipped. Any other trailing byte is
//! an error. `to_json_string` writes one compact line. `to_json_pretty` writes
//! the same tree with a two-space indent. Numbers are stored as `f64`.
//! Objects are a list of pairs, so key order is kept and duplicate keys are
//! kept. `get` returns the first match. Typed getters return `None` when the
//! type does not match.
//!
//! flux-script parses and writes script values with `parse_json` and
//! `to_json_string`. flux-net stores cookie and policy state with
//! `parse_json_bounded` and `to_json_string`, using its own caps (depth 16,
//! 1 MiB) rather than the defaults. flux-sandbox IPC messages use
//! `parse_json_bounded` and `to_json_string` with the sandbox caps (depth 32,
//! 8 MiB). flux-browser profiles use `parse_json` and `to_json_pretty`.
//! Download records are `JsonValue` trees inside that profile object.
//!
//! This crate does not accept comments, trailing commas, or a leading byte
//! order mark. It is not a streaming parser and it has no schema check.
//! There is no decimal number type. Integers past the exact range of `f64`
//! (2^53) are not preserved. Non-finite numbers serialize as `null`. The
//! serializer has no depth cap of its own.

use std::fmt;

/// Default maximum nesting of arrays and objects (64).
///
/// `parse_json` passes this to `parse_json_bounded`. Scalars do not use a
/// level. `max_depth` containers on one path are accepted. The next opening
/// `[` or `{` is a `JsonError`. Callers may pass a smaller cap.
pub const MAX_JSON_DEPTH: usize = 64;

/// Default maximum input size, 16 MiB (`16 * 1024 * 1024`).
///
/// `parse_json` passes this to `parse_json_bounded`. The check is the UTF-8
/// byte length of the source, before any token is read. A longer source fails
/// with the offset set to this limit. Callers may pass a smaller cap.
pub const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;

/// One JSON value.
///
/// Objects are a `Vec` of pairs, not a map. Order is parse order, or the
/// order of `insert` and `push`. Duplicate keys stay in the list. `get` and
/// `insert` use the first match. Numbers are `f64`. `PartialEq` follows
/// `f64`, so two NaN numbers do not compare equal.
#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    /// JSON `null`.
    Null,
    /// JSON `true` or `false`.
    Bool(bool),
    /// JSON number as `f64`. A non-finite value can be built in Rust, or can
    /// come from an exponent the float parser overflows. It is not written
    /// back as a JSON number.
    Number(f64),
    /// Decoded JSON string. Escapes are already applied. The data is Unicode scalars.
    String(String),
    /// JSON array. Elements stay in order.
    Array(Vec<JsonValue>),
    /// JSON object. The first pair with a given key is the one `get` returns.
    Object(Vec<(String, JsonValue)>),
}

/// A parse failure. The serializers do not return this.
///
/// `message` is a static reason. `offset` is a byte index into the source.
/// When the source is longer than `max_bytes`, no byte is parsed and the
/// offset is `max_bytes` itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    /// Static description of the failure.
    pub message: &'static str,
    /// Byte offset into the source, or the size cap when the input is too long.
    pub offset: usize,
}

impl fmt::Display for JsonError {
    /// Writes `JSON error at byte {offset}: {message}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for JsonError {}

impl JsonValue {
    /// Returns an object with no pairs.
    ///
    /// `insert` adds pairs. `to_json_string` of this value is `{}`.
    pub fn empty_object() -> Self {
        Self::Object(Vec::new())
    }

    /// Returns an array with no elements.
    ///
    /// `push` adds elements. `to_json_string` of this value is `[]`.
    pub fn empty_array() -> Self {
        Self::Array(Vec::new())
    }

    /// True only for `JsonValue::Null`.
    ///
    /// A missing key is not null. Use `get` for that. `get` returns `None`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the boolean when this value is `Bool`.
    ///
    /// Every other variant, including `Null`, returns `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the stored `f64` when this value is `Number`.
    ///
    /// Other variants return `None`. The parser does not keep the original
    /// decimal text. Integers above 2^53 may already be rounded. A non-finite
    /// value is returned as `Some`, not as `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Truncates a number toward zero and casts it to `i64`.
    ///
    /// Returns `None` only when the value is not a number. The fractional
    /// part is discarded. A value outside the `i64` range saturates. NaN
    /// becomes 0, and infinities saturate. This is not a checked conversion.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|n| n as i64)
    }

    /// Truncates a finite, non-negative number toward zero into `u64`.
    ///
    /// Returns `None` for non-numbers, negatives, NaN, and infinities. A
    /// value above `u64::MAX` saturates. The fractional part is discarded.
    /// Negative zero is treated as zero and returns `Some(0)`.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_f64().and_then(|n| {
            if n >= 0.0 && n.is_finite() {
                Some(n as u64)
            } else {
                None
            }
        })
    }

    /// Borrows the string when this value is `String`.
    ///
    /// Other variants return `None`. The text is already unescaped.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Borrows the elements when this value is `Array`.
    ///
    /// Other variants return `None`. Order is the stored order.
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Borrows the pairs when this value is `Object`.
    ///
    /// Other variants return `None`. Duplicate keys are all present. Order is
    /// the stored order, not a sorted order.
    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    /// Returns the first pair whose key equals `key`.
    ///
    /// Later duplicates are ignored. A missing key, or a value that is not
    /// an object, returns `None`. Comparison is byte-for-byte on the key string.
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        let entries = self.as_object()?;
        entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// `get`, then `as_str`.
    ///
    /// Returns `None` when the key is missing or the first match is not a string.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Self::as_str)
    }

    /// `get`, then `as_bool`.
    ///
    /// Returns `None` when the key is missing or the first match is not a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Self::as_bool)
    }

    /// `get`, then `as_i64`.
    ///
    /// The cast truncates toward zero and saturates. See `as_i64`. A missing
    /// key or a non-number returns `None`.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Self::as_i64)
    }

    /// `get`, then `as_u64`.
    ///
    /// Negatives and non-finite numbers return `None`. See `as_u64`.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).and_then(Self::as_u64)
    }

    /// `get`, then `as_array`.
    ///
    /// Returns `None` when the key is missing or the first match is not an array.
    pub fn get_array(&self, key: &str) -> Option<&[JsonValue]> {
        self.get(key).and_then(Self::as_array)
    }

    /// `get`, then `as_object`.
    ///
    /// Returns `None` when the key is missing or the first match is not an object.
    pub fn get_object(&self, key: &str) -> Option<&[(String, JsonValue)]> {
        self.get(key).and_then(Self::as_object)
    }

    /// Returns the array element at `index`.
    ///
    /// An out-of-range index, or a value that is not an array, returns `None`.
    /// Object keys are not indexes. Use `get` for those.
    pub fn get_index(&self, index: usize) -> Option<&JsonValue> {
        self.as_array()?.get(index)
    }

    /// Sets `key` on an object.
    ///
    /// If the key is already present, the first matching pair is replaced and
    /// later duplicates are left alone. A new key is appended. If `self` is
    /// not an object, this does nothing and does not report an error.
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

    /// Appends `value` when `self` is an array.
    ///
    /// Any other variant is left unchanged. There is no error return.
    pub fn push(&mut self, value: JsonValue) {
        if let Self::Array(items) = self {
            items.push(value);
        }
    }
}

/// Parses one JSON value using the default depth and size caps.
///
/// The caps are `MAX_JSON_DEPTH` (64) and `MAX_JSON_BYTES` (16 MiB). The
/// result is the value, or a `JsonError`. Trailing non-whitespace is an
/// error. flux-script and flux-browser profiles call this. Callers that need
/// a tighter cap use `parse_json_bounded`.
pub fn parse_json(source: &str) -> Result<JsonValue, JsonError> {
    parse_json_bounded(source, MAX_JSON_DEPTH, MAX_JSON_BYTES)
}

/// Parses one JSON value, stopping at `max_depth` and `max_bytes`.
///
/// `max_bytes` is a UTF-8 byte length, checked before parsing. Over the cap,
/// the error offset is `max_bytes`. `max_depth` counts nested arrays and
/// objects, not scalars. After the value, only space, tab, CR, and LF may
/// remain. flux-net calls this with depth 16 and 1 MiB. flux-sandbox calls
/// it with depth 32 and 8 MiB. Both are tighter than the defaults.
pub fn parse_json_bounded(
    source: &str,
    max_depth: usize,
    max_bytes: usize,
) -> Result<JsonValue, JsonError> {
    // Byte length of the source, before any token is read.
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

/// Writes `value` as one compact JSON line.
///
/// No extra space is inserted after `:` or `,`. Non-finite numbers become
/// `null`. Empty containers stay `[]` and `{}`. flux-script, flux-net, and
/// flux-sandbox call this. There is no depth cap on the way out.
pub fn to_json_string(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value(value, &mut out, None, 0);
    out
}

/// Writes `value` with a two-space indent and a space after each colon.
///
/// Empty containers stay on one line. Nested containers break across lines.
/// The number and string rules match `to_json_string`. flux-browser profiles
/// call this. The indent width is fixed at two spaces.
pub fn to_json_pretty(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value(value, &mut out, Some(2), 0);
    out
}

/// Appends one value to `out`.
///
/// `indent` is `None` for a compact line and `Some(step)` for pretty output.
/// The public pretty helper passes 2. `depth` is how many containers are
/// already open, and it only controls indent. It is not the parser cap, and
/// this function does not refuse a deep tree. Non-finite numbers are written
/// as `null`. A finite number whose fraction is 0 and whose magnitude is
/// below 1e15 is written as an integer. Other numbers use `Display` for `f64`.
/// Object pairs are written in vector order, including duplicate keys.
fn write_value(value: &JsonValue, out: &mut String, indent: Option<usize>, depth: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(n) => {
            // Non-finite values are not JSON numbers. Whole values under 1e15
            // stay integers so a parsed 3 is not written back as 3.0.
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

/// Appends `s` as a JSON string, with quotes and required escapes.
///
/// Quote and backslash are escaped. Backspace, form feed, newline, carriage
/// return, and tab use the short forms. Any other scalar below U+0020 is a
/// four-digit `\u00XX` escape. Every other character is written as UTF-8,
/// not as a `\u` escape. Solidus is not escaped on the way out. The parser
/// still accepts `\/`.
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
            // Remaining controls. The named escapes were handled above.
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Byte cursor over one JSON source.
///
/// `bytes` and `source` are the same text. `bytes` is used for ASCII tests.
/// `source` is used when a string or number must be sliced as UTF-8. `pos`
/// is the next byte index. `depth` is how many arrays and objects are open.
/// `max_depth` is the caller's cap. The parser is recursive, so this counter
/// is also the bound on call depth.
struct Parser<'a> {
    bytes: &'a [u8],
    source: &'a str,
    pos: usize,
    depth: usize,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    /// Builds an error at the current byte, without moving `pos`.
    ///
    /// `message` is stored as given. The offset is `self.pos`, which may be
    /// `bytes.len()` when the input ended. Callers return this immediately.
    fn err(&self, message: &'static str) -> JsonError {
        JsonError {
            message,
            offset: self.pos,
        }
    }

    /// Advances past space, tab, CR, and LF.
    ///
    /// Those four bytes are the RFC 8259 whitespace set. Any other byte,
    /// including other Unicode spaces, stops the skip and stays for the next
    /// token. This does not fail on its own. The end of input just returns.
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Returns the next byte, or `None` at the end of the source.
    ///
    /// `pos` does not move. Callers that accept the byte advance `pos` themselves.
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Counts one more open array or object, or fails at the cap.
    ///
    /// The check runs before the increment. `max_depth` open containers are
    /// allowed, and one more fails. The error offset is the current byte,
    /// which is still the `[` or `{` because the caller has not consumed it.
    /// Scalars do not call this.
    fn enter_container(&mut self) -> Result<(), JsonError> {
        // Fail before incrementing, so the cap is the maximum open containers.
        if self.depth >= self.max_depth {
            return Err(self.err("JSON nesting depth exceeds maximum allowed limit"));
        }
        self.depth += 1;
        Ok(())
    }

    /// Closes one container level.
    ///
    /// `saturating_sub` keeps a mismatched call from wrapping the counter.
    /// `parse_array` and `parse_object` call this only on the success path,
    /// after the matching `]` or `}` has been consumed.
    fn leave_container(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Parses the next value after skipping whitespace.
    ///
    /// `n`, `t`, and `f` start the literals `null`, `true`, and `false`.
    /// `"` starts a string, `[` an array, and `{` an object. `-` or a digit
    /// starts a number. Any other byte is an error. End of input is an error.
    /// This is the recursive entry. Container parsers call it for each element.
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

    /// Matches `literal` exactly at `pos`, or fails without consuming input.
    ///
    /// On success, `pos` moves by `literal.len()` and `value` is returned.
    /// A partial match, including a wrong case, is `invalid literal token`
    /// and `pos` stays where it was. Used for `null`, `true`, and `false`.
    fn parse_literal(&mut self, literal: &[u8], value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.bytes.get(self.pos..self.pos + literal.len()) == Some(literal) {
            self.pos += literal.len();
            Ok(value)
        } else {
            Err(self.err("invalid literal token"))
        }
    }

    /// Parses one JSON string and returns the decoded text.
    ///
    /// The current byte must be `"`. Escapes are `\"`, `\\`, `\/`, `\b`,
    /// `\f`, `\n`, `\r`, `\t`, and `\uXXXX`. A `\u` high surrogate
    /// (U+D800 through U+DBFF) must be followed immediately by `\u` and a
    /// low surrogate (U+DC00 through U+DFFF). The pair becomes one scalar.
    /// A raw byte below 0x20 is rejected. Other bytes are copied as one UTF-8
    /// scalar from `source`. A missing close quote is an error. The closing
    /// quote is consumed.
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
                        // High surrogate. The next two bytes must be `\u` and a low surrogate.
                        if (0xD800..=0xDBFF).contains(&code) {
                            if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                return Err(self.err("missing low surrogate in UTF-16 escape"));
                            }
                            self.pos += 2;
                            let low = self.read_hex_u16()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(self.err("invalid low surrogate in UTF-16 escape"));
                            }
                            // One scalar: 0x10000 + ((high - 0xD800) << 10 | (low - 0xDC00)).
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

    /// Reads four hex digits as a `u16` and advances past them.
    ///
    /// The caller has already consumed the `\u`. Fewer than four bytes left
    /// is a short-escape error. A non-hex digit is an error, and `pos` stays
    /// on those digits because the parse failure returns before the add.
    /// The result is a UTF-16 code unit, not yet checked as a scalar.
    fn read_hex_u16(&mut self) -> Result<u16, JsonError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.err("short \\uXXXX hex escape"));
        }
        let hex = &self.source[self.pos..self.pos + 4];
        let val = u16::from_str_radix(hex, 16).map_err(|_| self.err("invalid hex digit"))?;
        self.pos += 4;
        Ok(val)
    }

    /// Parses one JSON number into `JsonValue::Number`.
    ///
    /// An optional leading `-` is accepted. A leading `+` is not. The integer
    /// part is `0` or a non-zero digit with more digits. A fraction requires
    /// at least one digit. An exponent (`e` or `E`) allows an optional sign
    /// and requires at least one digit. The matched slice is parsed as `f64`,
    /// so the original decimal text is dropped and a huge exponent can become
    /// non-finite. `pos` ends on the first byte that is not part of the number.
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
        // f64, not a decimal type. Integers above 2^53 are not exact.
        let num: f64 = slice
            .parse()
            .map_err(|_| self.err("failed to parse number"))?;
        Ok(JsonValue::Number(num))
    }

    /// Parses one array, including the surrounding brackets.
    ///
    /// The depth counter is incremented first. Elements are `parse_value`,
    /// separated by commas. A trailing comma fails because the comma always
    /// requires another value. `]` closes the array and decrements the depth.
    /// An empty `[]` still enters and leaves the depth counter.
    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.enter_container()?;
        self.pos += 1; // '[' is consumed only after the depth check succeeds.
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

    /// Parses one object, including the surrounding braces.
    ///
    /// The depth counter is incremented first. Each key is a JSON string.
    /// A colon is required. The value is `parse_value`. Pairs are appended,
    /// so a duplicate key is kept after the earlier one. A trailing comma
    /// fails. `}` closes the object and decrements the depth. An empty `{}`
    /// still enters and leaves the depth counter.
    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.enter_container()?;
        self.pos += 1; // '{' is consumed only after the depth check succeeds.
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
    // Locks a nested round trip of objects, arrays, string escapes, and
    // `\u` hex through both compact and pretty text, plus rejection of one
    // extra nesting level and of a payload over a caller-supplied byte cap.
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
