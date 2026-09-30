//! The JSON Utility's GPUI-independent request/result/snapshot contract and engine.
//!
//! The engine never repairs malformed JSON silently: invalid input yields diagnostics
//! with a source location, and the caller receives no output.
//!
//! Numbers are parsed and re-emitted from their exact source literal. This keeps
//! arbitrary-precision integers and decimal spellings faithful and, unlike
//! `serde_json`'s `arbitrary_precision` mode, cannot mistake a user object whose
//! only key is `$serde_json::private::Number` for a number.

use serde::{Deserialize, Serialize};

mod completion;
pub use completion::{JsonCompletion, JsonCompletionKind, JsonQueryIndex};

/// Immutable catalog identity of the JSON Utility. History and snapshots carry it.
pub const JSON_UTILITY_ID: &str = "json";

/// Schema version of [`JsonSnapshot`]. The Utility owns decoding and migration.
pub const JSON_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Maximum container nesting the parser accepts.
///
/// The limit is checked before descending, so a hostile deeply nested document is
/// refused with a diagnostic instead of overflowing the stack during parsing,
/// rendering or drop. Deeper input is never partially processed.
pub const MAX_NESTING_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonMode {
    Format,
    Minify,
    Query,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Indentation {
    TwoSpaces,
    FourSpaces,
}

impl Indentation {
    fn as_str(self) -> &'static str {
        match self {
            Indentation::TwoSpaces => "  ",
            Indentation::FourSpaces => "    ",
        }
    }
}

pub use crate::diagnostic::{Diagnostic, Severity, SourceLocation};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRequest {
    pub input: String,
    pub mode: JsonMode,
    pub indentation: Indentation,
    pub sort_keys: bool,
    pub query: String,
}

impl JsonRequest {
    pub fn new(input: impl Into<String>, mode: JsonMode) -> Self {
        Self {
            input: input.into(),
            mode,
            indentation: Indentation::TwoSpaces,
            sort_keys: false,
            query: String::new(),
        }
    }
}

/// The typed outcome of one JSON evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonEvaluation {
    /// Empty or whitespace-only input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl JsonEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            JsonEvaluation::Valid { output } => Some(output),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            JsonEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    /// A Utility Operation is complete only for a settled valid result.
    pub fn is_valid_operation(&self) -> bool {
        matches!(self, JsonEvaluation::Valid { .. })
    }
}

/// Utility-owned snapshot for a completed JSON operation. History is introduced later.
///
/// The input is carried once, inside [`JsonRequest`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSnapshot {
    pub output: String,
    pub request: JsonRequest,
}

impl JsonSnapshot {
    /// Captures the already-settled result; restore never reevaluates input.
    pub fn from_settled(request: JsonRequest, evaluation: &JsonEvaluation) -> Option<Self> {
        evaluation.output().map(|output| Self {
            output: output.to_owned(),
            request,
        })
    }

    pub fn restore(&self) -> (&JsonRequest, &str) {
        (&self.request, &self.output)
    }
}

/// The JSON Utility's identity for the shared [`crate::utility::Utility`] trait.
pub struct Json;

impl crate::utility::Utility for Json {
    type Request = JsonRequest;
    type Evaluation = JsonEvaluation;
    type Snapshot = JsonSnapshot;

    const ID: &'static str = JSON_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = JSON_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> JsonEvaluation {
        JsonEvaluation::Empty
    }

    fn evaluate(request: &JsonRequest) -> JsonEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &JsonEvaluation) -> bool {
        matches!(evaluation, JsonEvaluation::Empty)
    }

    fn snapshot(request: &JsonRequest, evaluation: &JsonEvaluation) -> Option<JsonSnapshot> {
        JsonSnapshot::from_settled(request.clone(), evaluation)
    }

    fn restore(snapshot: &JsonSnapshot) -> (JsonRequest, JsonEvaluation) {
        (
            snapshot.request.clone(),
            JsonEvaluation::Valid {
                output: snapshot.output.clone(),
            },
        )
    }
}

/// The JSON Utility's revision-gated session.
pub type JsonSession = crate::session::Session<Json>;

pub fn evaluate(request: &JsonRequest) -> JsonEvaluation {
    if request.input.trim().is_empty() {
        return JsonEvaluation::Empty;
    }

    let value = match parse(&request.input) {
        Ok(value) => value,
        Err(error) => {
            return JsonEvaluation::Invalid {
                diagnostics: vec![error.into_diagnostic()],
            }
        }
    };

    match request.mode {
        JsonMode::Format | JsonMode::Minify => JsonEvaluation::Valid {
            output: render_document(&value, request),
        },
        JsonMode::Query => match query(&value, &request.query) {
            Ok(found) => JsonEvaluation::Valid {
                output: render_query_value(found, request),
            },
            Err(diagnostic) => JsonEvaluation::Invalid {
                diagnostics: vec![diagnostic],
            },
        },
    }
}

fn query_diagnostic(part: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        message: format!("Query path not found at ‘{part}’."),
        location: None,
    }
}

fn render_document(value: &JsonValue, request: &JsonRequest) -> String {
    let mut output = String::new();
    let indent = request.indentation.as_str();
    match request.mode {
        JsonMode::Minify => write_value(
            &mut output,
            value,
            Style::Minified,
            request.sort_keys,
            indent,
            0,
        ),
        _ => write_value(
            &mut output,
            value,
            Style::Pretty,
            request.sort_keys,
            indent,
            0,
        ),
    }
    output
}

fn render_query_value(value: &JsonValue, _request: &JsonRequest) -> String {
    match value {
        JsonValue::Object(_) | JsonValue::Array(_) => {
            // Swift baseline: a queried container is always pretty-printed with
            // recursively sorted keys and two-space indentation, independent of
            // the Format options.
            let mut output = String::new();
            write_value(&mut output, value, Style::Pretty, true, "  ", 0);
            output
        }
        JsonValue::String(text) => text.clone(),
        JsonValue::Null => "null".to_string(),
        // Baseline compatibility: the Swift Utility rendered a queried boolean
        // through `NSNumber`, i.e. `true` -> "1" and `false` -> "0".
        JsonValue::Bool(true) => "1".to_string(),
        JsonValue::Bool(false) => "0".to_string(),
        JsonValue::Number(raw) => raw.clone(),
    }
}

fn query<'a>(root: &'a JsonValue, path: &str) -> Result<&'a JsonValue, Diagnostic> {
    if path.is_empty() {
        return Ok(root);
    }

    let parts: Vec<String> = if let Some(pointer) = path.strip_prefix('/') {
        pointer.split('/').map(unescape_pointer_token).collect()
    } else {
        path.replace('[', ".")
            .replace(']', "")
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect()
    };

    let mut current = root;
    for part in &parts {
        current = match current {
            JsonValue::Object(entries) => entries
                .iter()
                .find(|(key, _)| key == part)
                .map(|(_, value)| value)
                .ok_or_else(|| query_diagnostic(part))?,
            JsonValue::Array(items) => {
                let index: usize = part.parse().map_err(|_| query_diagnostic(part))?;
                items.get(index).ok_or_else(|| query_diagnostic(part))?
            }
            _ => return Err(query_diagnostic(part)),
        };
    }
    Ok(current)
}

fn unescape_pointer_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Style {
    Minified,
    Pretty,
}

fn write_value(
    output: &mut String,
    value: &JsonValue,
    style: Style,
    sort_keys: bool,
    indent: &str,
    depth: usize,
) {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(true) => output.push_str("true"),
        JsonValue::Bool(false) => output.push_str("false"),
        JsonValue::Number(raw) => output.push_str(raw),
        JsonValue::String(text) => write_string(output, text),
        JsonValue::Array(items) => write_array(output, items, style, sort_keys, indent, depth),
        JsonValue::Object(entries) => {
            write_object(output, entries, style, sort_keys, indent, depth)
        }
    }
}

fn write_array(
    output: &mut String,
    items: &[JsonValue],
    style: Style,
    sort_keys: bool,
    indent: &str,
    depth: usize,
) {
    if items.is_empty() {
        output.push_str("[]");
        return;
    }
    output.push('[');
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        match style {
            Style::Pretty => {
                output.push('\n');
                write_indent(output, indent, depth + 1);
                write_value(output, item, style, sort_keys, indent, depth + 1);
            }
            Style::Minified => write_value(output, item, style, sort_keys, indent, depth + 1),
        }
    }
    if matches!(style, Style::Pretty) {
        output.push('\n');
        write_indent(output, indent, depth);
    }
    output.push(']');
}

fn write_object(
    output: &mut String,
    entries: &[(String, JsonValue)],
    style: Style,
    sort_keys: bool,
    indent: &str,
    depth: usize,
) {
    if entries.is_empty() {
        output.push_str("{}");
        return;
    }

    let mut ordered: Vec<&(String, JsonValue)> = entries.iter().collect();
    if sort_keys {
        ordered.sort_by(|a, b| a.0.cmp(&b.0));
    }

    output.push('{');
    for (index, (key, value)) in ordered.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        match style {
            Style::Pretty => {
                output.push('\n');
                write_indent(output, indent, depth + 1);
                write_string(output, key);
                output.push_str(": ");
                write_value(output, value, style, sort_keys, indent, depth + 1);
            }
            Style::Minified => {
                write_string(output, key);
                output.push(':');
                write_value(output, value, style, sort_keys, indent, depth + 1);
            }
        }
    }
    if matches!(style, Style::Pretty) {
        output.push('\n');
        write_indent(output, indent, depth);
    }
    output.push('}');
}

fn write_indent(output: &mut String, indent: &str, depth: usize) {
    for _ in 0..depth {
        output.push_str(indent);
    }
}

fn write_string(output: &mut String, text: &str) {
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            control if control < '\u{20}' => {
                output.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

struct JsonError {
    message: String,
    line: u32,
    column: u32,
}

impl JsonError {
    fn into_diagnostic(self) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            message: format!("Invalid JSON: {}", self.message),
            location: Some(SourceLocation {
                line: self.line,
                column: self.column,
            }),
        }
    }
}

fn parse(input: &str) -> Result<JsonValue, JsonError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(parser.error("trailing characters after the top-level value"));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: u32,
    column: u32,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
            depth: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        if byte == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(byte)
    }

    fn error(&self, message: impl Into<String>) -> JsonError {
        JsonError {
            message: message.into(),
            line: self.line,
            column: self.column,
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.bump();
        }
    }

    /// Enters a container, enforcing the nesting budget *before* descending.
    /// Only containers count, so `[[...]]` with a scalar leaf and `[[]]` with an
    /// empty leaf are limited identically.
    fn enter_container(&mut self) -> Result<(), JsonError> {
        if self.depth + 1 > MAX_NESTING_DEPTH {
            return Err(self.error(format!(
                "maximum nesting depth of {MAX_NESTING_DEPTH} exceeded"
            )));
        }
        self.depth += 1;
        Ok(())
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(JsonValue::String(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(_) => Err(self.error("expected value")),
            None => Err(self.error("unexpected end of input, expected value")),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        for expected in literal.bytes() {
            match self.bump() {
                Some(actual) if actual == expected => {}
                _ => return Err(self.error(format!("expected `{literal}`"))),
            }
        }
        Ok(value)
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.enter_container()?;
        let result = self.parse_object_inner();
        self.depth -= 1;
        result
    }

    fn parse_object_inner(&mut self) -> Result<JsonValue, JsonError> {
        self.bump();
        let mut entries: Vec<(String, JsonValue)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a string object key"));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(self.error("expected `:` after object key"));
            }
            self.bump();
            let value = self.parse_value()?;
            match entries.iter_mut().find(|(existing, _)| existing == &key) {
                Some(slot) => slot.1 = value,
                None => entries.push((key, value)),
            }
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                }
                Some(b'}') => {
                    self.bump();
                    return Ok(JsonValue::Object(entries));
                }
                _ => return Err(self.error("expected `,` or `}` in object")),
            }
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.enter_container()?;
        let result = self.parse_array_inner();
        self.depth -= 1;
        result
    }

    fn parse_array_inner(&mut self) -> Result<JsonValue, JsonError> {
        self.bump();
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                }
                Some(b']') => {
                    self.bump();
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(self.error("expected `,` or `]` in array")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.bump();
        let mut text = String::new();
        loop {
            match self.bump() {
                Some(b'"') => return Ok(text),
                Some(b'\\') => self.parse_escape(&mut text)?,
                Some(byte) if byte < 0x20 => return Err(self.error("control character in string")),
                Some(byte) if byte < 0x80 => text.push(byte as char),
                Some(_) => {
                    self.pos -= 1;
                    self.column -= 1;
                    let character = self
                        .input
                        .get(self.pos..)
                        .and_then(|rest| rest.chars().next())
                        .ok_or_else(|| self.error("invalid UTF-8 in string"))?;
                    self.pos += character.len_utf8();
                    self.column += 1;
                    text.push(character);
                }
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn parse_escape(&mut self, text: &mut String) -> Result<(), JsonError> {
        match self.bump() {
            Some(b'"') => text.push('"'),
            Some(b'\\') => text.push('\\'),
            Some(b'/') => text.push('/'),
            Some(b'b') => text.push('\u{08}'),
            Some(b'f') => text.push('\u{0c}'),
            Some(b'n') => text.push('\n'),
            Some(b'r') => text.push('\r'),
            Some(b't') => text.push('\t'),
            Some(b'u') => {
                let first = self.parse_hex_quad()?;
                if (0xD800..=0xDBFF).contains(&first) {
                    if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                        return Err(self.error("expected a low surrogate escape"));
                    }
                    let second = self.parse_hex_quad()?;
                    if !(0xDC00..=0xDFFF).contains(&second) {
                        return Err(self.error("invalid low surrogate"));
                    }
                    let combined = 0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                    let character = char::from_u32(combined)
                        .ok_or_else(|| self.error("invalid unicode escape"))?;
                    text.push(character);
                } else if (0xDC00..=0xDFFF).contains(&first) {
                    return Err(self.error("unexpected low surrogate"));
                } else {
                    let character = char::from_u32(first)
                        .ok_or_else(|| self.error("invalid unicode escape"))?;
                    text.push(character);
                }
            }
            _ => return Err(self.error("invalid escape sequence")),
        }
        Ok(())
    }

    fn parse_hex_quad(&mut self) -> Result<u32, JsonError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = match self.bump() {
                Some(byte @ b'0'..=b'9') => (byte - b'0') as u32,
                Some(byte @ b'a'..=b'f') => (byte - b'a' + 10) as u32,
                Some(byte @ b'A'..=b'F') => (byte - b'A' + 10) as u32,
                _ => return Err(self.error("invalid unicode escape")),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        match self.peek() {
            Some(b'0') => {
                self.bump();
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.bump();
                }
            }
            _ => return Err(self.error("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.bump();
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("invalid number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        Ok(JsonValue::Number(self.input[start..self.pos].to_string()))
    }
}
