//! The YAML/JSON Utility's GPUI-independent request/result/snapshot contract.
//!
//! Conversion is deliberately conservative. It accepts one YAML 1.2
//! Core-oriented document (anchors and aliases expand) or one JSON document,
//! and it refuses input it cannot represent faithfully instead of silently
//! changing it. In particular it rejects multiple YAML documents, duplicate
//! mapping keys, non-string mapping keys, unsupported YAML tags and
//! non-finite YAML numbers.
//!
//! Date-looking and YAML 1.1 boolean-looking plain scalars (`yes`, `no`, `on`,
//! `off`, `2026-08-31`) stay strings because they are strings under the YAML
//! 1.2 Core schema. Comments, original formatting, mapping order and alias
//! identity are not preserved; the presentation layer discloses that.
//!
//! Parsing uses the event-level parser from the maintained `saphyr` project so
//! that tags and duplicate keys are visible to the app-owned Core rules. The
//! parser types stay internal to this module.

use std::collections::{HashMap, HashSet};

use saphyr_parser::{Event, Parser, SpannedEventReceiver, TScalarStyle, Tag};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the YAML/JSON Utility.
pub const YAML_JSON_UTILITY_ID: &str = "yaml-json";

/// Schema version of [`YamlJsonSnapshot`].
pub const YAML_JSON_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Longest canonical plain-decimal expansion emitted for one YAML number.
const MAXIMUM_CANONICAL_DIGITS: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YamlJsonDirection {
    YamlToJson,
    JsonToYaml,
}

impl YamlJsonDirection {
    /// The direction's user-facing label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::YamlToJson => "YAML → JSON",
            Self::JsonToYaml => "JSON → YAML",
        }
    }

    /// The opposite direction, used by the Swap action.
    pub const fn swapped(self) -> Self {
        match self {
            Self::YamlToJson => Self::JsonToYaml,
            Self::JsonToYaml => Self::YamlToJson,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlJsonRequest {
    pub input: String,
    pub direction: YamlJsonDirection,
}

impl YamlJsonRequest {
    pub fn new(input: impl Into<String>, direction: YamlJsonDirection) -> Self {
        Self {
            input: input.into(),
            direction,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum YamlJsonEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl YamlJsonEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            YamlJsonEvaluation::Valid { output } => Some(output),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            YamlJsonEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, YamlJsonEvaluation::Valid { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YamlJsonSnapshot {
    pub request: YamlJsonRequest,
    pub output: String,
}

impl YamlJsonSnapshot {
    pub fn restore(&self) -> (&YamlJsonRequest, &str) {
        (&self.request, &self.output)
    }
}

/// Named resource bounds applied before any output is produced.
///
/// Input, nesting depth and alias expansion are bounded independently so a
/// small document can never expand into unbounded work or partial output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YamlJsonResourcePolicy {
    pub maximum_input_bytes: usize,
    pub maximum_nesting_depth: usize,
    pub maximum_alias_expansions: usize,
    pub maximum_expanded_nodes: usize,
}

impl YamlJsonResourcePolicy {
    /// The production policy: 1 MiB input, 128 levels, 10 000 alias
    /// expansions and a one-million-node expansion budget.
    pub const PRODUCTION: Self = Self {
        maximum_input_bytes: 1_048_576,
        maximum_nesting_depth: 128,
        maximum_alias_expansions: 10_000,
        maximum_expanded_nodes: 1_000_000,
    };
}

impl Default for YamlJsonResourcePolicy {
    fn default() -> Self {
        Self::PRODUCTION
    }
}

pub fn evaluate(request: &YamlJsonRequest) -> YamlJsonEvaluation {
    evaluate_with_policy(request, YamlJsonResourcePolicy::PRODUCTION)
}

pub fn evaluate_with_policy(
    request: &YamlJsonRequest,
    policy: YamlJsonResourcePolicy,
) -> YamlJsonEvaluation {
    if request.input.is_empty() {
        return YamlJsonEvaluation::Empty;
    }
    if request.input.len() > policy.maximum_input_bytes {
        return invalid(format!(
            "Input exceeds the {}-byte conversion limit; nothing was converted.",
            policy.maximum_input_bytes
        ));
    }
    if request.input.trim().is_empty() {
        return YamlJsonEvaluation::Empty;
    }

    let converted = match request.direction {
        YamlJsonDirection::YamlToJson => {
            parse_yaml(&request.input, policy).map(|value| write_json(&value))
        }
        YamlJsonDirection::JsonToYaml => {
            parse_json(&request.input, policy).map(|value| write_yaml(&value))
        }
    };

    match converted {
        Ok(output) => YamlJsonEvaluation::Valid { output },
        Err(diagnostic) => YamlJsonEvaluation::Invalid {
            diagnostics: vec![diagnostic],
        },
    }
}

fn invalid(message: impl Into<String>) -> YamlJsonEvaluation {
    YamlJsonEvaluation::Invalid {
        diagnostics: vec![Diagnostic::error(message)],
    }
}

/// The YAML/JSON Utility's identity for the shared [`Utility`] trait.
pub struct YamlJson;

impl Utility for YamlJson {
    type Request = YamlJsonRequest;
    type Evaluation = YamlJsonEvaluation;
    type Snapshot = YamlJsonSnapshot;

    const ID: &'static str = YAML_JSON_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = YAML_JSON_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> YamlJsonEvaluation {
        YamlJsonEvaluation::Empty
    }

    fn evaluate(request: &YamlJsonRequest) -> YamlJsonEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &YamlJsonEvaluation) -> bool {
        matches!(evaluation, YamlJsonEvaluation::Empty)
    }

    fn snapshot(
        request: &YamlJsonRequest,
        evaluation: &YamlJsonEvaluation,
    ) -> Option<YamlJsonSnapshot> {
        evaluation.output().map(|output| YamlJsonSnapshot {
            request: request.clone(),
            output: output.to_owned(),
        })
    }

    fn restore(snapshot: &YamlJsonSnapshot) -> (YamlJsonRequest, YamlJsonEvaluation) {
        (
            snapshot.request.clone(),
            YamlJsonEvaluation::Valid {
                output: snapshot.output.clone(),
            },
        )
    }
}

/// The app-owned data model shared by both directions.
///
/// Numbers are kept as canonical JSON literals, so their meaning is never
/// routed through a lossy binary float.
#[derive(Clone, Debug, PartialEq, Eq)]
enum YamlValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<YamlValue>),
    Object(Vec<(String, YamlValue)>),
}

// ---------------------------------------------------------------------------
// YAML → value
// ---------------------------------------------------------------------------

fn parse_yaml(source: &str, policy: YamlJsonResourcePolicy) -> Result<YamlValue, Diagnostic> {
    let mut parser = Parser::new_from_str(source);
    let mut builder = YamlEventBuilder::new(policy);
    match parser.load(&mut builder, true) {
        Ok(()) => builder.finish(),
        Err(error) => Err(Diagnostic::error(format!("Invalid YAML: {error}."))),
    }
}

#[derive(Clone, Copy)]
enum CollectionKind {
    Sequence,
    Mapping,
}

#[derive(Clone, Copy)]
enum ScalarTag {
    None,
    NonSpecific,
    Str,
    Int,
    Float,
    Bool,
    Null,
}

enum Frame {
    Sequence {
        items: Vec<YamlValue>,
        anchor: usize,
    },
    Mapping {
        pairs: Vec<(String, YamlValue)>,
        pending_key: Option<String>,
        seen: HashSet<String>,
        anchor: usize,
    },
}

struct YamlEventBuilder {
    policy: YamlJsonResourcePolicy,
    stack: Vec<Frame>,
    root: Option<YamlValue>,
    anchors: HashMap<usize, YamlValue>,
    documents: usize,
    alias_expansions: usize,
    expanded_nodes: usize,
    error: Option<Diagnostic>,
}

impl YamlEventBuilder {
    fn new(policy: YamlJsonResourcePolicy) -> Self {
        Self {
            policy,
            stack: Vec::new(),
            root: None,
            anchors: HashMap::new(),
            documents: 0,
            alias_expansions: 0,
            expanded_nodes: 0,
            error: None,
        }
    }

    fn finish(self) -> Result<YamlValue, Diagnostic> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if !self.stack.is_empty() {
            return Err(Diagnostic::error(
                "Invalid YAML: the document ended inside a collection.",
            ));
        }
        Ok(self.root.unwrap_or(YamlValue::Null))
    }

    fn located(message: impl Into<String>, span: saphyr_parser::Span) -> Diagnostic {
        Diagnostic::error(message).at(span.start.line() as u32, span.start.col() as u32)
    }

    fn depth_diagnostic(&self) -> Diagnostic {
        Diagnostic::error(format!(
            "Document nesting exceeds the supported depth of {}; nothing was converted.",
            self.policy.maximum_nesting_depth
        ))
    }

    fn charge_nodes(&mut self, amount: usize) -> bool {
        self.expanded_nodes = self.expanded_nodes.saturating_add(amount);
        if self.expanded_nodes > self.policy.maximum_expanded_nodes {
            self.error = Some(Diagnostic::error(
                "Alias expansion exceeds the supported node budget; nothing was converted.",
            ));
            return false;
        }
        true
    }

    /// Appends a completed node to its parent (or the document root), rejecting
    /// duplicate and non-string mapping keys.
    fn push(&mut self, value: YamlValue, anchor: usize, span: saphyr_parser::Span) {
        if self.error.is_some() {
            return;
        }
        if anchor > 0 {
            self.anchors.insert(anchor, value.clone());
        }
        match self.stack.last_mut() {
            Some(Frame::Sequence { items, .. }) => items.push(value),
            Some(Frame::Mapping {
                pairs,
                pending_key,
                seen,
                ..
            }) => {
                if let Some(key) = pending_key.take() {
                    pairs.push((key, value));
                } else {
                    match value {
                        YamlValue::String(key) => {
                            if !seen.insert(key.clone()) {
                                self.error = Some(Self::located(
                                    format!("Duplicate YAML mapping key ‘{key}’."),
                                    span,
                                ));
                            } else {
                                *pending_key = Some(key);
                            }
                        }
                        _ => {
                            self.error = Some(Self::located(
                                "YAML mapping keys must resolve to unique strings for JSON conversion.",
                                span,
                            ));
                        }
                    }
                }
            }
            None => {
                if self.root.is_some() {
                    self.error = Some(Self::located(
                        "Invalid YAML: a single document must contain exactly one root value.",
                        span,
                    ));
                } else {
                    self.root = Some(value);
                }
            }
        }
    }

    fn resolve_scalar(
        &self,
        value: &str,
        style: TScalarStyle,
        tag: Option<&Tag>,
        span: saphyr_parser::Span,
    ) -> Result<YamlValue, Diagnostic> {
        match classify_scalar_tag(tag, span)? {
            ScalarTag::Str => Ok(YamlValue::String(value.to_owned())),
            ScalarTag::Null => {
                if is_null(value) {
                    Ok(YamlValue::Null)
                } else {
                    Err(Self::located("Invalid explicitly tagged YAML null.", span))
                }
            }
            ScalarTag::Bool => core_boolean(value)
                .map(YamlValue::Bool)
                .ok_or_else(|| Self::located("Invalid explicitly tagged YAML boolean.", span)),
            ScalarTag::Int => parse_integer(value)
                .map(YamlValue::Number)
                .map_err(|message| Self::located(message, span)),
            ScalarTag::Float => parse_float(value)
                .map(YamlValue::Number)
                .map_err(|message| Self::located(message, span)),
            ScalarTag::None | ScalarTag::NonSpecific => {
                if is_quoted(style) {
                    Ok(YamlValue::String(value.to_owned()))
                } else {
                    resolve_core_plain(value, span)
                }
            }
        }
    }
}

impl SpannedEventReceiver for YamlEventBuilder {
    fn on_event(&mut self, event: Event, span: saphyr_parser::Span) {
        if self.error.is_some() {
            return;
        }
        match event {
            Event::Nothing | Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart(_) => {
                self.documents += 1;
                if self.documents > 1 {
                    self.error = Some(Self::located(
                        "The input contains another document; conversion supports one YAML document.",
                        span,
                    ));
                }
                self.anchors.clear();
            }
            Event::DocumentEnd => {
                if !self.stack.is_empty() {
                    self.error = Some(Self::located(
                        "Invalid YAML: the document ended inside a collection.",
                        span,
                    ));
                }
            }
            Event::Scalar(value, style, anchor, tag) => {
                if !self.charge_nodes(1) {
                    return;
                }
                match self.resolve_scalar(&value, style, tag.as_ref(), span) {
                    Ok(node) => self.push(node, anchor, span),
                    Err(diagnostic) => self.error = Some(diagnostic),
                }
            }
            Event::SequenceStart(anchor, tag) => {
                if let Err(diagnostic) =
                    validate_collection_tag(tag.as_ref(), CollectionKind::Sequence, span)
                {
                    self.error = Some(diagnostic);
                    return;
                }
                if self.stack.len() + 1 > self.policy.maximum_nesting_depth {
                    self.error = Some(self.depth_diagnostic());
                    return;
                }
                if !self.charge_nodes(1) {
                    return;
                }
                self.stack.push(Frame::Sequence {
                    items: Vec::new(),
                    anchor,
                });
            }
            Event::MappingStart(anchor, tag) => {
                if let Err(diagnostic) =
                    validate_collection_tag(tag.as_ref(), CollectionKind::Mapping, span)
                {
                    self.error = Some(diagnostic);
                    return;
                }
                if self.stack.len() + 1 > self.policy.maximum_nesting_depth {
                    self.error = Some(self.depth_diagnostic());
                    return;
                }
                if !self.charge_nodes(1) {
                    return;
                }
                self.stack.push(Frame::Mapping {
                    pairs: Vec::new(),
                    pending_key: None,
                    seen: HashSet::new(),
                    anchor,
                });
            }
            Event::SequenceEnd => match self.stack.pop() {
                Some(Frame::Sequence { items, anchor }) => {
                    self.push(YamlValue::Array(items), anchor, span);
                }
                _ => {
                    self.error = Some(Self::located(
                        "Invalid YAML: unexpected sequence end.",
                        span,
                    ));
                }
            },
            Event::MappingEnd => match self.stack.pop() {
                Some(Frame::Mapping {
                    pairs,
                    pending_key,
                    anchor,
                    ..
                }) => {
                    if pending_key.is_some() {
                        self.error = Some(Self::located(
                            "Invalid YAML: a mapping key has no value.",
                            span,
                        ));
                    } else {
                        self.push(YamlValue::Object(pairs), anchor, span);
                    }
                }
                _ => {
                    self.error = Some(Self::located("Invalid YAML: unexpected mapping end.", span));
                }
            },
            Event::Alias(anchor) => {
                let Some(node) = self.anchors.get(&anchor).cloned() else {
                    self.error = Some(Self::located(
                        "Unresolved YAML aliases are unsupported.",
                        span,
                    ));
                    return;
                };
                self.alias_expansions += 1;
                if self.alias_expansions > self.policy.maximum_alias_expansions {
                    self.error = Some(Self::located(
                        "Alias expansion exceeds the supported limit; nothing was converted.",
                        span,
                    ));
                    return;
                }
                if !self.charge_nodes(count_nodes(&node)) {
                    return;
                }
                self.push(node, 0, span);
            }
        }
    }
}

fn classify_scalar_tag(
    tag: Option<&Tag>,
    span: saphyr_parser::Span,
) -> Result<ScalarTag, Diagnostic> {
    let Some(tag) = tag else {
        return Ok(ScalarTag::None);
    };
    if tag.handle == "tag:yaml.org,2002:" {
        return match tag.suffix.as_str() {
            "str" => Ok(ScalarTag::Str),
            "int" => Ok(ScalarTag::Int),
            "float" => Ok(ScalarTag::Float),
            "bool" => Ok(ScalarTag::Bool),
            "null" => Ok(ScalarTag::Null),
            _ => Err(unsupported_tag(tag, span)),
        };
    }
    if is_non_specific_tag(tag) {
        return Ok(ScalarTag::NonSpecific);
    }
    Err(unsupported_tag(tag, span))
}

fn validate_collection_tag(
    tag: Option<&Tag>,
    kind: CollectionKind,
    span: saphyr_parser::Span,
) -> Result<(), Diagnostic> {
    let Some(tag) = tag else {
        return Ok(());
    };
    let expected = match kind {
        CollectionKind::Sequence => "seq",
        CollectionKind::Mapping => "map",
    };
    if tag.handle == "tag:yaml.org,2002:" && tag.suffix == expected {
        return Ok(());
    }
    if is_non_specific_tag(tag) {
        return Ok(());
    }
    Err(unsupported_tag(tag, span))
}

fn is_non_specific_tag(tag: &Tag) -> bool {
    (tag.handle.is_empty() && tag.suffix == "!") || (tag.handle == "!" && tag.suffix.is_empty())
}

fn unsupported_tag(tag: &Tag, span: saphyr_parser::Span) -> Diagnostic {
    YamlEventBuilder::located(
        format!("Unsupported YAML tag ‘{}’.", display_tag(tag)),
        span,
    )
}

fn display_tag(tag: &Tag) -> String {
    if tag.handle.is_empty() {
        if tag.suffix == "!" {
            "!".to_owned()
        } else {
            format!("!<{}>", tag.suffix)
        }
    } else if tag.handle == "tag:yaml.org,2002:" {
        format!("!!{}", tag.suffix)
    } else {
        format!("{}{}", tag.handle, tag.suffix)
    }
}

fn count_nodes(value: &YamlValue) -> usize {
    match value {
        YamlValue::Array(items) => 1 + items.iter().map(count_nodes).sum::<usize>(),
        YamlValue::Object(members) => {
            1 + members
                .iter()
                .map(|(_, value)| count_nodes(value))
                .sum::<usize>()
        }
        _ => 1,
    }
}

fn resolve_core_plain(source: &str, span: saphyr_parser::Span) -> Result<YamlValue, Diagnostic> {
    if is_null(source) {
        return Ok(YamlValue::Null);
    }
    if let Some(boolean) = core_boolean(source) {
        return Ok(YamlValue::Bool(boolean));
    }
    if is_integer(source) {
        return parse_integer(source)
            .map(YamlValue::Number)
            .map_err(|message| YamlEventBuilder::located(message, span));
    }
    if is_float(source) {
        return parse_float(source)
            .map(YamlValue::Number)
            .map_err(|message| YamlEventBuilder::located(message, span));
    }
    if is_nonfinite(source) {
        return Err(YamlEventBuilder::located(
            "Non-finite YAML numbers cannot be represented in JSON.",
            span,
        ));
    }
    Ok(YamlValue::String(source.to_owned()))
}

fn is_quoted(style: TScalarStyle) -> bool {
    matches!(
        style,
        TScalarStyle::SingleQuoted
            | TScalarStyle::DoubleQuoted
            | TScalarStyle::Literal
            | TScalarStyle::Folded
    )
}

fn strip_sign(source: &str) -> &str {
    source
        .strip_prefix('+')
        .or_else(|| source.strip_prefix('-'))
        .unwrap_or(source)
}

fn is_null(source: &str) -> bool {
    source.is_empty() || source == "~" || source.eq_ignore_ascii_case("null")
}

fn core_boolean(source: &str) -> Option<bool> {
    if source.eq_ignore_ascii_case("true") {
        Some(true)
    } else if source.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn is_nonfinite(source: &str) -> bool {
    matches!(
        strip_sign(source),
        ".inf" | ".Inf" | ".INF" | ".nan" | ".NaN" | ".NAN"
    )
}

fn is_integer(source: &str) -> bool {
    let body = strip_sign(source);
    if body.is_empty() {
        return false;
    }
    if let Some(rest) = body.strip_prefix("0o") {
        return !rest.is_empty() && rest.chars().all(|c| matches!(c, '0'..='7' | '_'));
    }
    if let Some(rest) = body.strip_prefix("0x") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit() || c == '_');
    }
    let digits: String = body.chars().filter(|c| *c != '_').collect();
    if digits == "0" {
        return true;
    }
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) && !digits.starts_with('0')
}

fn is_float(source: &str) -> bool {
    let body = strip_sign(source);
    if body.is_empty() {
        return false;
    }
    if let Some(index) = body.find(['e', 'E']) {
        let mantissa = &body[..index];
        let exponent = &body[index + 1..];
        is_float_mantissa(mantissa) && is_signed_digits(exponent)
    } else {
        match body.split_once('.') {
            Some((integer, fraction)) => {
                is_optional_integer_part(integer) && is_digit_underscore(fraction)
            }
            None => false,
        }
    }
}

fn is_float_mantissa(mantissa: &str) -> bool {
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (mantissa, None),
    };
    if integer.is_empty() || !integer.starts_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    if !integer.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return false;
    }
    match fraction {
        Some(fraction) => fraction.chars().all(|c| c.is_ascii_digit() || c == '_'),
        None => true,
    }
}

fn is_optional_integer_part(integer: &str) -> bool {
    integer.is_empty()
        || (integer.starts_with(|c: char| c.is_ascii_digit())
            && integer.chars().all(|c| c.is_ascii_digit() || c == '_'))
}

fn is_digit_underscore(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '_')
}

fn is_signed_digits(value: &str) -> bool {
    let value = strip_sign(value);
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
}

fn parse_integer(source: &str) -> Result<String, &'static str> {
    let cleaned: String = source.chars().filter(|c| *c != '_').collect();
    let (negative, unsigned) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned.strip_prefix('+').unwrap_or(cleaned.as_str())),
    };
    let (radix, digits) = if let Some(rest) = unsigned.strip_prefix("0o") {
        (8, rest)
    } else if let Some(rest) = unsigned.strip_prefix("0x") {
        (16, rest)
    } else {
        (10, unsigned)
    };
    let magnitude = u64::from_str_radix(digits, radix)
        .map_err(|_| "Integer is outside the exact JSON conversion range.")?;
    if magnitude == 0 {
        return Ok("0".to_owned());
    }
    if negative {
        if magnitude == i64::MAX as u64 + 1 {
            Ok(i64::MIN.to_string())
        } else if magnitude <= i64::MAX as u64 {
            Ok(format!("-{magnitude}"))
        } else {
            Err("Integer is outside the exact JSON conversion range.")
        }
    } else {
        Ok(magnitude.to_string())
    }
}

fn parse_float(source: &str) -> Result<String, &'static str> {
    let cleaned: String = source.chars().filter(|c| *c != '_').collect();
    if is_nonfinite(&cleaned) {
        return Err("Non-finite YAML numbers cannot be represented in JSON.");
    }
    canonicalize_decimal(&cleaned)
}

/// Canonicalizes a YAML Core float into a plain JSON number literal without
/// changing its decimal value (`1e2` becomes `100`, `-0` becomes `0`).
fn canonicalize_decimal(source: &str) -> Result<String, &'static str> {
    const UNREPRESENTABLE: &str = "Number cannot be represented faithfully in JSON.";

    let (negative, unsigned) = match source.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, source.strip_prefix('+').unwrap_or(source)),
    };
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => {
            let exponent = unsigned[index + 1..]
                .parse::<i64>()
                .map_err(|_| UNREPRESENTABLE)?;
            (&unsigned[..index], exponent)
        }
        None => (unsigned, 0),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, fraction),
        None => (mantissa, ""),
    };

    let mut digits = String::with_capacity(integer.len() + fraction.len());
    digits.push_str(integer);
    digits.push_str(fraction);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(UNREPRESENTABLE);
    }

    let mut exponent = exponent
        .checked_sub(fraction.len() as i64)
        .ok_or(UNREPRESENTABLE)?;
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return Ok("0".to_owned());
    }
    let without_trailing = trimmed.trim_end_matches('0');
    exponent = exponent
        .checked_add((trimmed.len() - without_trailing.len()) as i64)
        .ok_or(UNREPRESENTABLE)?;
    let significant = without_trailing;

    let magnitude = if exponent >= 0 {
        let length = significant.len() as i64 + exponent;
        if length > MAXIMUM_CANONICAL_DIGITS as i64 {
            return Err(UNREPRESENTABLE);
        }
        format!("{significant}{}", "0".repeat(exponent as usize))
    } else {
        let point = significant.len() as i64 + exponent;
        if -exponent > MAXIMUM_CANONICAL_DIGITS as i64 {
            return Err(UNREPRESENTABLE);
        }
        if point > 0 {
            let (whole, fraction) = significant.split_at(point as usize);
            format!("{whole}.{fraction}")
        } else {
            format!("0.{}{significant}", "0".repeat((-point) as usize))
        }
    };
    if negative {
        Ok(format!("-{magnitude}"))
    } else {
        Ok(magnitude)
    }
}

// ---------------------------------------------------------------------------
// JSON → value
// ---------------------------------------------------------------------------

fn parse_json(source: &str, policy: YamlJsonResourcePolicy) -> Result<YamlValue, Diagnostic> {
    JsonParser::parse(source, policy.maximum_nesting_depth)
}

struct JsonParser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn parse(source: &'a str, maximum_depth: usize) -> Result<YamlValue, Diagnostic> {
        let mut parser = Self {
            source,
            bytes: source.as_bytes(),
            position: 0,
        };
        parser.skip_whitespace();
        let node = parser.parse_value(1, maximum_depth)?;
        parser.skip_whitespace();
        if parser.position != parser.bytes.len() {
            return Err(Diagnostic::error(
                "Invalid JSON: unexpected trailing characters.",
            ));
        }
        Ok(node)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let byte = self.peek();
        if byte.is_some() {
            self.position += 1;
        }
        byte
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn parse_value(&mut self, depth: usize, maximum_depth: usize) -> Result<YamlValue, Diagnostic> {
        match self.peek() {
            Some(b'{') => self.parse_object(depth, maximum_depth),
            Some(b'[') => self.parse_array(depth, maximum_depth),
            Some(b'"') => self.parse_string().map(YamlValue::String),
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(YamlValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(YamlValue::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(YamlValue::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(YamlValue::Number),
            Some(_) => Err(Diagnostic::error("Invalid JSON: unexpected character.")),
            None => Err(Diagnostic::error("Invalid JSON: unexpected end of input.")),
        }
    }

    fn expect_literal(&mut self, literal: &str) -> Result<(), Diagnostic> {
        if self.source[self.position..].starts_with(literal) {
            self.position += literal.len();
            Ok(())
        } else {
            Err(Diagnostic::error("Invalid JSON: unexpected literal."))
        }
    }

    fn parse_array(&mut self, depth: usize, maximum_depth: usize) -> Result<YamlValue, Diagnostic> {
        if depth > maximum_depth {
            return Err(depth_diagnostic(maximum_depth));
        }
        self.position += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(YamlValue::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.parse_value(depth + 1, maximum_depth)?);
            self.skip_whitespace();
            match self.take() {
                Some(b',') => {}
                Some(b']') => break,
                _ => {
                    return Err(Diagnostic::error(
                        "Invalid JSON: expected ‘,’ or ‘]’ in an array.",
                    ))
                }
            }
        }
        Ok(YamlValue::Array(items))
    }

    fn parse_object(
        &mut self,
        depth: usize,
        maximum_depth: usize,
    ) -> Result<YamlValue, Diagnostic> {
        if depth > maximum_depth {
            return Err(depth_diagnostic(maximum_depth));
        }
        self.position += 1;
        let mut members = Vec::new();
        let mut seen = HashSet::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(YamlValue::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(Diagnostic::error(
                    "Invalid JSON: object keys must be strings.",
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.take() != Some(b':') {
                return Err(Diagnostic::error("Invalid JSON: expected ‘:’ after a key."));
            }
            self.skip_whitespace();
            let value = self.parse_value(depth + 1, maximum_depth)?;
            if !seen.insert(key.clone()) {
                return Err(Diagnostic::error(format!(
                    "Duplicate JSON object key ‘{key}’."
                )));
            }
            members.push((key, value));
            self.skip_whitespace();
            match self.take() {
                Some(b',') => {}
                Some(b'}') => break,
                _ => {
                    return Err(Diagnostic::error(
                        "Invalid JSON: expected ‘,’ or ‘}’ in an object.",
                    ))
                }
            }
        }
        Ok(YamlValue::Object(members))
    }

    fn parse_string(&mut self) -> Result<String, Diagnostic> {
        if self.take() != Some(b'"') {
            return Err(Diagnostic::error("Invalid JSON: expected a string."));
        }
        let mut output = String::new();
        loop {
            match self.take() {
                None => return Err(Diagnostic::error("Invalid JSON: unterminated string.")),
                Some(b'"') => break,
                Some(b'\\') => self.parse_escape(&mut output)?,
                Some(byte) if byte < 0x20 => {
                    return Err(Diagnostic::error(
                        "Invalid JSON: control character in a string.",
                    ))
                }
                Some(byte) if byte < 0x80 => output.push(byte as char),
                Some(_) => {
                    let start = self.position - 1;
                    let character = self.source[start..]
                        .chars()
                        .next()
                        .ok_or_else(|| Diagnostic::error("Invalid JSON: malformed UTF-8."))?;
                    self.position = start + character.len_utf8();
                    output.push(character);
                }
            }
        }
        Ok(output)
    }

    fn parse_escape(&mut self, output: &mut String) -> Result<(), Diagnostic> {
        match self.take() {
            Some(b'"') => output.push('"'),
            Some(b'\\') => output.push('\\'),
            Some(b'/') => output.push('/'),
            Some(b'b') => output.push('\u{0008}'),
            Some(b'f') => output.push('\u{000C}'),
            Some(b'n') => output.push('\n'),
            Some(b'r') => output.push('\r'),
            Some(b't') => output.push('\t'),
            Some(b'u') => {
                let code = self.parse_hex4()?;
                if (0xD800..=0xDBFF).contains(&code) {
                    if self.take() != Some(b'\\') || self.take() != Some(b'u') {
                        return Err(Diagnostic::error(
                            "Invalid JSON: lone high surrogate in a string.",
                        ));
                    }
                    let low = self.parse_hex4()?;
                    if !(0xDC00..=0xDFFF).contains(&low) {
                        return Err(Diagnostic::error(
                            "Invalid JSON: invalid surrogate pair in a string.",
                        ));
                    }
                    let combined = 0x1_0000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                    output.push(char::from_u32(combined).ok_or_else(|| {
                        Diagnostic::error("Invalid JSON: invalid unicode escape.")
                    })?);
                } else if (0xDC00..=0xDFFF).contains(&code) {
                    return Err(Diagnostic::error(
                        "Invalid JSON: lone low surrogate in a string.",
                    ));
                } else {
                    output.push(char::from_u32(code).ok_or_else(|| {
                        Diagnostic::error("Invalid JSON: invalid unicode escape.")
                    })?);
                }
            }
            _ => return Err(Diagnostic::error("Invalid JSON: invalid escape.")),
        }
        Ok(())
    }

    fn parse_hex4(&mut self) -> Result<u32, Diagnostic> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self
                .take()
                .ok_or_else(|| Diagnostic::error("Invalid JSON: incomplete unicode escape."))?;
            let digit = (byte as char)
                .to_digit(16)
                .ok_or_else(|| Diagnostic::error("Invalid JSON: invalid unicode escape."))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, Diagnostic> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        match self.peek() {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(Diagnostic::error("Invalid JSON: invalid number.")),
        }
        if self.peek() == Some(b'.') {
            self.position += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(Diagnostic::error("Invalid JSON: invalid number."));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(Diagnostic::error("Invalid JSON: invalid number."));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        Ok(self.source[start..self.position].to_owned())
    }
}

fn depth_diagnostic(maximum_depth: usize) -> Diagnostic {
    Diagnostic::error(format!(
        "Document nesting exceeds the supported depth of {maximum_depth}; nothing was converted."
    ))
}

// ---------------------------------------------------------------------------
// Writers
// ---------------------------------------------------------------------------

fn write_json(value: &YamlValue) -> String {
    let mut output = String::new();
    write_json_value(value, 0, &mut output);
    output
}

fn write_json_value(value: &YamlValue, indent: usize, output: &mut String) {
    match value {
        YamlValue::Null => output.push_str("null"),
        YamlValue::Bool(boolean) => output.push_str(if *boolean { "true" } else { "false" }),
        YamlValue::Number(number) => output.push_str(number),
        YamlValue::String(text) => output.push_str(&json_quote(text)),
        YamlValue::Array(items) => {
            if items.is_empty() {
                output.push_str("[]");
                return;
            }
            let child_indent = indent + 2;
            output.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                push_indent(output, child_indent);
                write_json_value(item, child_indent, output);
                if index + 1 < items.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            push_indent(output, indent);
            output.push(']');
        }
        YamlValue::Object(members) => {
            if members.is_empty() {
                output.push_str("{}");
                return;
            }
            let sorted = sorted_members(members);
            let child_indent = indent + 2;
            output.push_str("{\n");
            for (index, (key, member)) in sorted.iter().enumerate() {
                push_indent(output, child_indent);
                output.push_str(&json_quote(key));
                output.push_str(" : ");
                write_json_value(member, child_indent, output);
                if index + 1 < sorted.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            push_indent(output, indent);
            output.push('}');
        }
    }
}

fn write_yaml(value: &YamlValue) -> String {
    let mut output = String::new();
    write_yaml_block(value, 0, &mut output);
    output
}

/// Writes a block node at the start of a line. The first line carries no
/// leading indentation; later lines are indented by `indent` spaces.
fn write_yaml_block(value: &YamlValue, indent: usize, output: &mut String) {
    match value {
        YamlValue::Array(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    push_indent(output, indent);
                }
                output.push('-');
                write_yaml_inline(item, indent + 2, output);
            }
        }
        YamlValue::Object(members) if !members.is_empty() => {
            let sorted = sorted_members(members);
            for (index, (key, member)) in sorted.iter().enumerate() {
                if index > 0 {
                    push_indent(output, indent);
                }
                output.push_str(&yaml_string(key));
                output.push(':');
                write_yaml_inline(member, indent + 2, output);
            }
        }
        _ => {
            output.push_str(&yaml_scalar(value));
            output.push('\n');
        }
    }
}

/// Writes a value that follows an existing prefix (`-` or `key:`) on the
/// current line. Scalars and empty collections stay on that line; a non-empty
/// collection continues as an indented block on the next line.
fn write_yaml_inline(value: &YamlValue, indent: usize, output: &mut String) {
    match value {
        YamlValue::Array(items) if !items.is_empty() => {
            output.push('\n');
            write_yaml_block(value, indent, output);
        }
        YamlValue::Object(members) if !members.is_empty() => {
            output.push('\n');
            write_yaml_block(value, indent, output);
        }
        _ => {
            output.push(' ');
            output.push_str(&yaml_scalar(value));
            output.push('\n');
        }
    }
}

fn yaml_scalar(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "null".to_owned(),
        YamlValue::Bool(boolean) => if *boolean { "true" } else { "false" }.to_owned(),
        YamlValue::Number(number) => number.clone(),
        YamlValue::String(text) => yaml_string(text),
        YamlValue::Array(_) => "[]".to_owned(),
        YamlValue::Object(_) => "{}".to_owned(),
    }
}

/// Emits a string as a plain scalar when that round-trips as a string under the
/// app-owned Core rules, otherwise as a double-quoted scalar.
fn yaml_string(text: &str) -> String {
    if is_plain_safe(text) {
        text.to_owned()
    } else {
        double_quote(text)
    }
}

fn is_plain_safe(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace) {
        return false;
    }
    if is_null(text)
        || core_boolean(text).is_some()
        || is_integer(text)
        || is_float(text)
        || is_nonfinite(text)
    {
        return false;
    }
    let mut first = true;
    for character in text.chars() {
        if first {
            first = false;
            if matches!(
                character,
                '-' | '?'
                    | ':'
                    | ','
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '#'
                    | '&'
                    | '*'
                    | '!'
                    | '|'
                    | '>'
                    | '\''
                    | '"'
                    | '%'
                    | '@'
                    | '`'
            ) {
                return false;
            }
        }
        if character.is_control()
            || matches!(
                character,
                ':' | '#'
                    | '"'
                    | '\''
                    | '\\'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | ','
                    | '&'
                    | '*'
                    | '!'
                    | '|'
                    | '>'
                    | '%'
                    | '@'
                    | '`'
            )
        {
            return false;
        }
    }
    true
}

fn double_quote(text: &str) -> String {
    let mut output = String::with_capacity(text.len() + 2);
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000C}' => output.push_str("\\f"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn json_quote(text: &str) -> String {
    double_quote(text)
}

fn sorted_members(members: &[(String, YamlValue)]) -> Vec<(&String, &YamlValue)> {
    let mut sorted: Vec<(&String, &YamlValue)> =
        members.iter().map(|(key, value)| (key, value)).collect();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    sorted
}

fn push_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml_to_json(input: &str) -> YamlJsonEvaluation {
        evaluate(&YamlJsonRequest::new(input, YamlJsonDirection::YamlToJson))
    }

    fn json_to_yaml(input: &str) -> YamlJsonEvaluation {
        evaluate(&YamlJsonRequest::new(input, YamlJsonDirection::JsonToYaml))
    }

    fn output(evaluation: &YamlJsonEvaluation) -> String {
        evaluation.output().expect("valid output").to_owned()
    }

    fn message(evaluation: &YamlJsonEvaluation) -> String {
        evaluation
            .diagnostics()
            .first()
            .expect("a diagnostic")
            .message
            .clone()
    }

    #[test]
    fn core_scalars_convert_to_canonical_json() {
        let input = "enabled: true\nlegacy: yes\ncreated: 2026-08-31\nmessage: café 👩🏽‍💻\n";
        let evaluation = yaml_to_json(input);
        assert_eq!(
            output(&evaluation),
            "{\n  \"created\" : \"2026-08-31\",\n  \"enabled\" : true,\n  \"legacy\" : \"yes\",\n  \"message\" : \"café 👩🏽‍💻\"\n}"
        );
    }

    #[test]
    fn core_numeric_forms_are_canonical_and_out_of_range_integers_fail() {
        let evaluation = yaml_to_json(
            "decimal: 42\noctal: 0o17\nhex: 0x10\nfraction: 1.25\nexponent: 1e2\nnegativeZero: -0",
        );
        let output = output(&evaluation);
        assert!(output.contains("\"decimal\" : 42"), "{output}");
        assert!(output.contains("\"octal\" : 15"), "{output}");
        assert!(output.contains("\"hex\" : 16"), "{output}");
        assert!(output.contains("\"fraction\" : 1.25"), "{output}");
        assert!(output.contains("\"exponent\" : 100"), "{output}");
        assert!(output.contains("\"negativeZero\" : 0"), "{output}");

        let too_large = yaml_to_json("value: 18446744073709551616");
        assert!(message(&too_large).contains("outside the exact JSON conversion range"));
    }

    #[test]
    fn anchors_and_aliases_expand_into_json_values() {
        let input = "profile: &profile\n  name: café 👩🏽‍💻\n  active: true\ncopy: *profile\n";
        let output = output(&yaml_to_json(input));
        assert_eq!(output.matches("café 👩🏽‍💻").count(), 2, "{output}");
        assert!(output.contains("\"copy\""), "{output}");
        assert!(output.contains("\"profile\""), "{output}");
    }

    #[test]
    fn malformed_and_unsupported_yaml_is_diagnosed() {
        let cases = [
            ("---\na: 1\n---\nb: 2", "another document"),
            ("true: value", "keys must resolve to unique strings"),
            ("value: .inf", "Non-finite"),
            ("value: !widget hello", "Unsupported YAML tag"),
            ("a: 1\na: 2", "Duplicate YAML mapping key"),
            ("items: [1,", "Invalid YAML"),
            ("a: !!binary aGk=", "Unsupported YAML tag"),
            ("a: !!set\n  ? x\n", "Unsupported YAML tag"),
            ("a: &x [*x]", "Unresolved YAML aliases"),
        ];
        for (input, expected) in cases {
            let evaluation = yaml_to_json(input);
            assert!(!evaluation.is_valid_operation(), "{input}");
            assert!(evaluation.output().is_none(), "{input}");
            let message = message(&evaluation);
            assert!(message.contains(expected), "{input} -> {message}");
        }
    }

    #[test]
    fn yaml_11_boolean_looking_and_date_looking_scalars_stay_strings() {
        let evaluation = yaml_to_json("a: yes\nb: no\nc: on\nd: off\ne: 2026-08-31\n");
        let output = output(&evaluation);
        assert!(output.contains("\"a\" : \"yes\""), "{output}");
        assert!(output.contains("\"b\" : \"no\""), "{output}");
        assert!(output.contains("\"c\" : \"on\""), "{output}");
        assert!(output.contains("\"d\" : \"off\""), "{output}");
        assert!(output.contains("\"e\" : \"2026-08-31\""), "{output}");
    }

    #[test]
    fn json_to_yaml_round_trips_and_quotes_ambiguous_strings() {
        let json = r#"{"z":1,"a":"yes","emoji":"👩🏽‍💻","items":[null,false,1.25],"flag":"true"}"#;
        let yaml = output(&json_to_yaml(json));
        assert!(yaml.contains("a: yes"), "{yaml}");
        assert!(yaml.contains("flag: \"true\""), "{yaml}");
        assert!(yaml.contains("👩🏽‍💻"), "{yaml}");

        let round_trip = output(&yaml_to_json(&yaml));
        assert!(
            round_trip.find("\"a\"").unwrap() < round_trip.find("\"z\"").unwrap(),
            "{round_trip}"
        );
        assert!(round_trip.contains("\"a\" : \"yes\""), "{round_trip}");
        assert!(round_trip.contains("\"flag\" : \"true\""), "{round_trip}");
        assert!(round_trip.contains("1.25"), "{round_trip}");
    }

    #[test]
    fn json_numbers_are_preserved_exactly() {
        let json = r#"{"big":18446744073709551615,"negative":-9223372036854775808,"decimal":0.30000000000000004}"#;
        let yaml = output(&json_to_yaml(json));
        assert!(yaml.contains("big: 18446744073709551615"), "{yaml}");
        assert!(yaml.contains("negative: -9223372036854775808"), "{yaml}");
        assert!(yaml.contains("decimal: 0.30000000000000004"), "{yaml}");
    }

    #[test]
    fn unicode_round_trips_through_yaml() {
        let yaml = output(&json_to_yaml(r#"{"message":"café 👩🏽‍💻"}"#));
        assert!(yaml.contains("café 👩🏽‍💻"), "{yaml}");
        let json = output(&yaml_to_json(&yaml));
        assert!(json.contains("café 👩🏽‍💻"), "{json}");
    }

    #[test]
    fn over_limit_input_is_refused_without_partial_output() {
        let policy = YamlJsonResourcePolicy {
            maximum_input_bytes: 4,
            ..YamlJsonResourcePolicy::PRODUCTION
        };
        let evaluation = evaluate_with_policy(
            &YamlJsonRequest::new("value: 1", YamlJsonDirection::YamlToJson),
            policy,
        );
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.output().is_none());
        assert!(message(&evaluation).contains("conversion limit"));
    }

    #[test]
    fn over_limit_nesting_is_refused_in_both_directions() {
        let policy = YamlJsonResourcePolicy {
            maximum_nesting_depth: 2,
            ..YamlJsonResourcePolicy::PRODUCTION
        };
        let deep_json = evaluate_with_policy(
            &YamlJsonRequest::new("[[[]]]", YamlJsonDirection::JsonToYaml),
            policy,
        );
        assert!(!deep_json.is_valid_operation());
        assert!(message(&deep_json).contains("nesting exceeds"));

        let deep_yaml = evaluate_with_policy(
            &YamlJsonRequest::new(
                "items:\n  - child:\n      value: 1",
                YamlJsonDirection::YamlToJson,
            ),
            policy,
        );
        assert!(!deep_yaml.is_valid_operation());
        assert!(message(&deep_yaml).contains("nesting exceeds"));
    }

    #[test]
    fn alias_expansion_is_bounded() {
        let policy = YamlJsonResourcePolicy {
            maximum_alias_expansions: 1,
            ..YamlJsonResourcePolicy::PRODUCTION
        };
        let input = "a: &x [1]\nb: *x\nc: *x\n";
        let evaluation = evaluate_with_policy(
            &YamlJsonRequest::new(input, YamlJsonDirection::YamlToJson),
            policy,
        );
        assert!(!evaluation.is_valid_operation());
        assert!(message(&evaluation).contains("Alias expansion"));
    }

    #[test]
    fn invalid_json_is_diagnosed_without_repair() {
        for input in ["{", "{\"a\":}", "[1,]", "01", "\"\\ud800\""] {
            let evaluation = json_to_yaml(input);
            assert!(!evaluation.is_valid_operation(), "{input}");
            assert!(evaluation.output().is_none(), "{input}");
            assert!(message(&evaluation).starts_with("Invalid JSON"), "{input}");
        }
    }

    #[test]
    fn duplicate_json_keys_are_rejected() {
        let evaluation = json_to_yaml(r#"{"a":1,"a":2}"#);
        assert!(!evaluation.is_valid_operation());
        assert!(message(&evaluation).contains("Duplicate JSON object key"));
    }

    #[test]
    fn empty_and_whitespace_input_is_neutral() {
        assert_eq!(
            evaluate(&YamlJsonRequest::new("", YamlJsonDirection::YamlToJson)),
            YamlJsonEvaluation::Empty
        );
        assert_eq!(
            evaluate(&YamlJsonRequest::new("  \n", YamlJsonDirection::JsonToYaml)),
            YamlJsonEvaluation::Empty
        );
    }

    #[test]
    fn snapshot_round_trips_exactly_and_without_reevaluation() {
        let request =
            YamlJsonRequest::new(r#"{"message":"café 👩🏽‍💻"}"#, YamlJsonDirection::JsonToYaml);
        let evaluation = evaluate(&request);
        let snapshot = <YamlJson as Utility>::snapshot(&request, &evaluation).expect("snapshot");

        let encoded = serde_json::to_value(&snapshot).expect("serialize");
        let decoded: YamlJsonSnapshot = serde_json::from_value(encoded).expect("deserialize");
        assert_eq!(decoded, snapshot);
        assert_eq!(decoded.restore().1, output(&evaluation));

        let (restored_request, restored_evaluation) = <YamlJson as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.output(), evaluation.output());
    }

    #[test]
    fn neutral_and_invalid_operations_never_snapshot() {
        let neutral = YamlJsonRequest::new("", YamlJsonDirection::YamlToJson);
        assert!(<YamlJson as Utility>::snapshot(&neutral, &evaluate(&neutral)).is_none());
        let invalid = YamlJsonRequest::new("{", YamlJsonDirection::JsonToYaml);
        assert!(<YamlJson as Utility>::snapshot(&invalid, &evaluate(&invalid)).is_none());
    }
}
