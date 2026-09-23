//! The Sample Data Utility's GPUI-independent request/result/snapshot contract.
//!
//! A request is an ordered list of strongly typed [`FieldDefinition`]s, a row
//! count and an output format. Generation is deliberately explicit (nothing is
//! produced until the user asks for it) and fully deterministic for a given
//! injected [`IdentifierSource`]: the same source always yields the same rows,
//! so tests use fixed bytes without weakening the production path.
//!
//! Randomness and the clock come from the Identifier Generator's
//! [`IdentifierSource`] seam, which is reused, not duplicated. UUID fields call
//! the Identifier Generator's own UUID v4 engine, so a sample UUID is a real
//! RFC 9562 value. Nothing here simulates locales, relationships or real
//! people: names and e-mail addresses are always explicitly fictional.
//!
//! ## Output
//!
//! JSON is a pretty-printed array of objects in field order. CSV follows RFC
//! 4180 quoting: the header and every value are always quoted, and embedded
//! `"` is doubled. Because every field is quoted, commas, quotes, newlines,
//! Unicode and empty values are all preserved exactly. There are no external
//! datasets, relations or schema imports.

use std::collections::HashSet;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utilities::identifiers::{
    evaluate_with_source, IdentifierAction, IdentifierSource, IdentifiersEvaluation,
    IdentifiersRequest, SystemIdentifierSource,
};
use crate::utility::Utility;

/// Immutable catalog identity of the Sample Data Utility.
pub const SAMPLE_DATA_UTILITY_ID: &str = "sample-data";

/// Schema version of [`SampleDataSnapshot`].
pub const SAMPLE_DATA_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Inclusive lower bound for the generated row count.
pub const MINIMUM_ROW_COUNT: u32 = 1;

/// Inclusive upper bound for the generated row count.
pub const MAXIMUM_ROW_COUNT: u32 = 1_000;

/// Inclusive lower bound for the number of fields.
pub const MINIMUM_FIELD_COUNT: usize = 1;

/// Inclusive upper bound for the number of fields.
pub const MAXIMUM_FIELD_COUNT: usize = 50;

/// Largest whole-number integer range the engine will draw from.
const MAXIMUM_INTEGER_RANGE: f64 = 9_000_000_000_000_000.0;

/// Bounds of the whole 32-bit signed integer domain, expressed as floats.
const MINIMUM_INTEGER_BOUND: f64 = -9_000_000_000_000_000.0;
const MAXIMUM_INTEGER_BOUND: f64 = 9_000_000_000_000_000.0;

/// Default upper date bound: 2030-01-01T00:00:00Z (matching the baseline).
const DEFAULT_DATE_MAXIMUM_SECONDS: f64 = 1_893_456_000.0;

/// Fictional given names. These are invented placeholders, not real people.
const FICTIONAL_FIRST_NAMES: [&str; 4] = ["Ada", "Lin", "Sam", "Noor"];

/// Fictional family names. These are invented placeholders, not real people.
const FICTIONAL_LAST_NAMES: [&str; 4] = ["Example", "Sample", "Placeholder", "Imaginary"];

/// The output format for a generated batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SampleDataFormat {
    Json,
    Csv,
}

impl SampleDataFormat {
    /// Declaration order, used for stable control order.
    pub const ALL: [SampleDataFormat; 2] = [SampleDataFormat::Json, SampleDataFormat::Csv];

    /// Human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            SampleDataFormat::Json => "JSON",
            SampleDataFormat::Csv => "CSV",
        }
    }

    /// Stable index in [`SampleDataFormat::ALL`].
    pub const fn index(self) -> usize {
        match self {
            SampleDataFormat::Json => 0,
            SampleDataFormat::Csv => 1,
        }
    }
}

/// The bounded value families a field can produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleFieldType {
    /// An explicitly fictional full name.
    FictionalName,
    /// An explicitly fictional address at `example.invalid`.
    FictionalEmail,
    /// A bounded integer or decimal number.
    Number,
    /// `true` or `false`.
    Boolean,
    /// An ISO 8601 instant inside a bounded date range.
    Date,
    /// An RFC 9562 UUID version 4.
    Uuid,
    /// One of the caller-supplied choices.
    Enumeration,
}

impl SampleFieldType {
    /// Declaration order, used for stable control order and cycling.
    pub const ALL: [SampleFieldType; 7] = [
        SampleFieldType::FictionalName,
        SampleFieldType::FictionalEmail,
        SampleFieldType::Number,
        SampleFieldType::Boolean,
        SampleFieldType::Date,
        SampleFieldType::Uuid,
        SampleFieldType::Enumeration,
    ];

    /// Human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            SampleFieldType::FictionalName => "Fictional name",
            SampleFieldType::FictionalEmail => "Fictional email",
            SampleFieldType::Number => "Number",
            SampleFieldType::Boolean => "Boolean",
            SampleFieldType::Date => "Date",
            SampleFieldType::Uuid => "UUID",
            SampleFieldType::Enumeration => "Enum",
        }
    }

    /// Stable index in [`SampleFieldType::ALL`].
    pub const fn index(self) -> usize {
        match self {
            SampleFieldType::FictionalName => 0,
            SampleFieldType::FictionalEmail => 1,
            SampleFieldType::Number => 2,
            SampleFieldType::Boolean => 3,
            SampleFieldType::Date => 4,
            SampleFieldType::Uuid => 5,
            SampleFieldType::Enumeration => 6,
        }
    }

    /// The next family in declaration order, wrapping at the end.
    pub const fn successor(self) -> Self {
        SampleFieldType::ALL[(self.index() + 1) % SampleFieldType::ALL.len()]
    }
}

/// One strongly typed field in the ordered schema.
///
/// Every option is always present so a field's shape survives a type switch and
/// a snapshot round-trip; only the options relevant to `field_type` are read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: SampleFieldType,
    pub number_minimum: f64,
    pub number_maximum: f64,
    pub number_is_integer: bool,
    pub date_minimum_seconds: f64,
    pub date_maximum_seconds: f64,
    pub enum_choices: Vec<String>,
}

impl FieldDefinition {
    /// A field with the documented bounded defaults for its family.
    pub fn new(name: impl Into<String>, field_type: SampleFieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            number_minimum: 0.0,
            number_maximum: 100.0,
            number_is_integer: true,
            date_minimum_seconds: 0.0,
            date_maximum_seconds: DEFAULT_DATE_MAXIMUM_SECONDS,
            enum_choices: vec!["alpha".to_owned(), "beta".to_owned()],
        }
    }
}

/// The complete, strongly typed input for one generation.
///
/// `generation` is a monotonic nonce bumped by every explicit Generate action.
/// It does not affect the schema or the output; it makes two deliberate
/// identical generations distinct to the shared revision session so each is
/// recorded separately instead of being deduplicated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleDataRequest {
    pub fields: Vec<FieldDefinition>,
    pub row_count: u32,
    pub output: SampleDataFormat,
    #[serde(default)]
    pub generation: u64,
}

impl Default for SampleDataRequest {
    /// The baseline default: ten JSON rows over `name`, `email` and `id`.
    fn default() -> Self {
        Self {
            fields: vec![
                FieldDefinition::new("name", SampleFieldType::FictionalName),
                FieldDefinition::new("email", SampleFieldType::FictionalEmail),
                FieldDefinition::new("id", SampleFieldType::Uuid),
            ],
            row_count: 10,
            output: SampleDataFormat::Json,
            generation: 0,
        }
    }
}

impl SampleDataRequest {
    pub fn new() -> Self {
        Self::default()
    }

    /// The request with the generation nonce cleared, for display and
    /// comparison without treating a repeat as different content.
    pub fn configuration(&self) -> Self {
        Self {
            generation: 0,
            ..self.clone()
        }
    }
}

/// One generated cell. The variant preserves the field family so JSON and CSV
/// render numbers, booleans and text correctly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleValue {
    Text(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
}

impl SampleValue {
    /// The CSV/inline display form: JSON-style booleans and trimmed decimals.
    pub fn display(&self) -> String {
        match self {
            SampleValue::Text(value) => value.clone(),
            SampleValue::Integer(value) => value.to_string(),
            SampleValue::Decimal(value) => format_decimal(*value),
            SampleValue::Boolean(value) => if *value { "true" } else { "false" }.to_owned(),
        }
    }

    /// The JSON literal for this cell (bare numbers and booleans).
    fn json(&self) -> String {
        match self {
            SampleValue::Text(value) => json_string(value),
            SampleValue::Integer(value) => value.to_string(),
            SampleValue::Decimal(value) => format_decimal(*value),
            SampleValue::Boolean(value) => if *value { "true" } else { "false" }.to_owned(),
        }
    }
}

/// The typed outcome shown to the user.
#[derive(Clone, Debug, PartialEq)]
pub enum SampleDataEvaluation {
    /// Nothing has been generated yet: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
        rows: Vec<Vec<SampleValue>>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl SampleDataEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            SampleDataEvaluation::Valid { output, .. } => Some(output),
            _ => None,
        }
    }

    pub fn rows(&self) -> &[Vec<SampleValue>] {
        match self {
            SampleDataEvaluation::Valid { rows, .. } => rows,
            _ => &[],
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            SampleDataEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, SampleDataEvaluation::Valid { .. })
    }
}

/// The exact schema, rows and output, captured so restore never regenerates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleDataSnapshot {
    pub request: SampleDataRequest,
    pub rows: Vec<Vec<SampleValue>>,
    pub output: String,
}

impl SampleDataSnapshot {
    pub fn restore(&self) -> (&SampleDataRequest, &str) {
        (&self.request, &self.output)
    }
}

/// Validates a request, returning every problem found.
///
/// An empty result means the request is generatable. Validation is a pure
/// function of the request, so the UI can show diagnostics without generating.
pub fn validate(request: &SampleDataRequest) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !(MINIMUM_ROW_COUNT..=MAXIMUM_ROW_COUNT).contains(&request.row_count) {
        diagnostics.push(Diagnostic::error(format!(
            "Row count must be {MINIMUM_ROW_COUNT} through {MAXIMUM_ROW_COUNT}."
        )));
    }
    if !(MINIMUM_FIELD_COUNT..=MAXIMUM_FIELD_COUNT).contains(&request.fields.len()) {
        diagnostics.push(Diagnostic::error(format!(
            "Define {MINIMUM_FIELD_COUNT} through {MAXIMUM_FIELD_COUNT} fields."
        )));
    }

    let mut seen: HashSet<&str> = HashSet::new();
    for (index, field) in request.fields.iter().enumerate() {
        let position = index + 1;
        if !is_valid_name(&field.name) {
            diagnostics.push(Diagnostic::error(format!(
                "Field {position} name must begin with a letter or underscore and contain only ASCII letters, digits, and underscores."
            )));
        } else if !seen.insert(field.name.as_str()) {
            diagnostics.push(Diagnostic::error(format!(
                "Field name \"{}\" is used more than once.",
                field.name
            )));
        }

        match field.field_type {
            SampleFieldType::Number => validate_number(position, field, &mut diagnostics),
            SampleFieldType::Date => validate_date(position, field, &mut diagnostics),
            SampleFieldType::Enumeration => {
                if field.enum_choices.is_empty()
                    || field.enum_choices.iter().any(|choice| choice.is_empty())
                {
                    diagnostics.push(Diagnostic::error(format!(
                        "Field {position} enum choices must be explicit and non-empty."
                    )));
                }
            }
            SampleFieldType::FictionalName
            | SampleFieldType::FictionalEmail
            | SampleFieldType::Boolean
            | SampleFieldType::Uuid => {}
        }
    }
    diagnostics
}

fn validate_number(position: usize, field: &FieldDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let minimum = field.number_minimum;
    let maximum = field.number_maximum;
    if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
        diagnostics.push(Diagnostic::error(format!(
            "Field {position} number ranges require finite minimum and maximum values in ascending order."
        )));
        return;
    }
    if field.number_is_integer {
        if minimum.fract() != 0.0
            || maximum.fract() != 0.0
            || minimum < MINIMUM_INTEGER_BOUND
            || maximum > MAXIMUM_INTEGER_BOUND
            || (maximum - minimum) > MAXIMUM_INTEGER_RANGE
        {
            diagnostics.push(Diagnostic::error(format!(
                "Field {position} integer ranges require whole-number bounds with at most 9 quadrillion values."
            )));
        }
    } else if !(maximum - minimum).is_finite() {
        diagnostics.push(Diagnostic::error(format!(
            "Field {position} decimal range width must be finite."
        )));
    }
}

fn validate_date(position: usize, field: &FieldDefinition, diagnostics: &mut Vec<Diagnostic>) {
    let minimum = field.date_minimum_seconds;
    let maximum = field.date_maximum_seconds;
    if !minimum.is_finite()
        || !maximum.is_finite()
        || minimum > maximum
        || !(maximum - minimum).is_finite()
    {
        diagnostics.push(Diagnostic::error(format!(
            "Field {position} date ranges require the earliest instant first."
        )));
        return;
    }
    if !is_representable_instant(minimum) || !is_representable_instant(maximum) {
        diagnostics.push(Diagnostic::error(format!(
            "Field {position} date bounds must be representable ISO 8601 instants."
        )));
    }
}

fn is_valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_representable_instant(seconds: f64) -> bool {
    seconds.is_finite() && DateTime::<Utc>::from_timestamp(seconds.floor() as i64, 0).is_some()
}

/// Generates using the production clock and operating-system randomness.
pub fn evaluate(request: &SampleDataRequest) -> SampleDataEvaluation {
    generate_with_source(request, &SystemIdentifierSource)
}

/// Generates using an injected randomness/clock source.
///
/// The same source always produces the same rows, so tests are deterministic.
pub fn generate_with_source(
    request: &SampleDataRequest,
    source: &dyn IdentifierSource,
) -> SampleDataEvaluation {
    let diagnostics = validate(request);
    if !diagnostics.is_empty() {
        return SampleDataEvaluation::Invalid { diagnostics };
    }

    let mut rows = Vec::with_capacity(request.row_count as usize);
    for _ in 0..request.row_count {
        let mut row = Vec::with_capacity(request.fields.len());
        for field in &request.fields {
            row.push(generate_value(field, source));
        }
        rows.push(row);
    }

    let output = match request.output {
        SampleDataFormat::Json => render_json(&request.fields, &rows),
        SampleDataFormat::Csv => render_csv(&request.fields, &rows),
    };
    SampleDataEvaluation::Valid { output, rows }
}

fn generate_value(field: &FieldDefinition, source: &dyn IdentifierSource) -> SampleValue {
    match field.field_type {
        SampleFieldType::FictionalName => SampleValue::Text(fictional_name(source)),
        SampleFieldType::FictionalEmail => SampleValue::Text(fictional_email(source)),
        SampleFieldType::Number => generate_number(field, source),
        SampleFieldType::Boolean => SampleValue::Boolean(draw_index(2, source) == 1),
        SampleFieldType::Date => SampleValue::Text(format_date(date_seconds(field, source))),
        SampleFieldType::Uuid => SampleValue::Text(generate_uuid(source)),
        SampleFieldType::Enumeration => {
            let choice = draw_index(field.enum_choices.len() as u64, source) as usize;
            SampleValue::Text(field.enum_choices[choice].clone())
        }
    }
}

fn fictional_name(source: &dyn IdentifierSource) -> String {
    let first = FICTIONAL_FIRST_NAMES[draw_index(4, source) as usize];
    let last = FICTIONAL_LAST_NAMES[draw_index(4, source) as usize];
    format!("Fictional {first} {last}")
}

fn fictional_email(source: &dyn IdentifierSource) -> String {
    let first = FICTIONAL_FIRST_NAMES[draw_index(4, source) as usize].to_ascii_lowercase();
    let last = FICTIONAL_LAST_NAMES[draw_index(4, source) as usize].to_ascii_lowercase();
    let suffix = draw_index(10_000, source);
    format!("fictional.{first}.{last}{suffix}@example.invalid")
}

fn generate_number(field: &FieldDefinition, source: &dyn IdentifierSource) -> SampleValue {
    if field.number_is_integer {
        let minimum = field.number_minimum as i64;
        let maximum = field.number_maximum as i64;
        let span = (maximum - minimum) as u64 + 1;
        SampleValue::Integer(minimum + draw_index(span, source) as i64)
    } else {
        let unit = unit_interval(source);
        SampleValue::Decimal(
            field.number_minimum + (field.number_maximum - field.number_minimum) * unit,
        )
    }
}

fn date_seconds(field: &FieldDefinition, source: &dyn IdentifierSource) -> f64 {
    let span = field.date_maximum_seconds - field.date_minimum_seconds;
    field.date_minimum_seconds + span * unit_interval(source)
}

/// A UUID version 4 produced by the Identifier Generator's own engine.
fn generate_uuid(source: &dyn IdentifierSource) -> String {
    let request = IdentifiersRequest {
        action: IdentifierAction::Generate,
        count: 1,
        ..IdentifiersRequest::default()
    };
    match evaluate_with_source(&request, source) {
        IdentifiersEvaluation::Valid { values } => values.first().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

/// A uniform fraction in `[0, 1)` from one 64-bit draw.
fn unit_interval(source: &dyn IdentifierSource) -> f64 {
    draw_u64(source) as f64 / u64::MAX as f64
}

/// One unbiased-enough bounded index (modulo reduction, as in the baseline).
fn draw_index(bound: u64, source: &dyn IdentifierSource) -> u64 {
    debug_assert!(bound > 0);
    draw_u64(source) % bound
}

fn draw_u64(source: &dyn IdentifierSource) -> u64 {
    let mut bytes = [0u8; 8];
    source.fill_random(&mut bytes);
    u64::from_be_bytes(bytes)
}

/// Renders 3 fractional digits in UTC, e.g. `1970-01-01T00:00:00.000Z`.
fn format_date(seconds: f64) -> String {
    let whole = seconds.floor();
    let mut ticks = whole as i64;
    let mut millis = ((seconds - whole) * 1000.0).round() as i64;
    if millis >= 1000 {
        ticks = ticks.saturating_add(millis / 1000);
        millis %= 1000;
    }
    let nanos = (millis.max(0) as u32) * 1_000_000;
    match DateTime::<Utc>::from_timestamp(ticks, nanos) {
        Some(instant) => instant.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => "1970-01-01T00:00:00.000Z".to_owned(),
    }
}

/// Trims a finite decimal to at most 8 fractional digits, like the baseline.
fn format_decimal(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    let mut text = format!("{value:.8}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".to_owned();
    }
    text
}

fn render_json(fields: &[FieldDefinition], rows: &[Vec<SampleValue>]) -> String {
    let mut output = String::from("[\n");
    for (row_index, row) in rows.iter().enumerate() {
        if row_index > 0 {
            output.push_str(",\n");
        }
        output.push_str("  {\n");
        for (field_index, (field, value)) in fields.iter().zip(row).enumerate() {
            if field_index > 0 {
                output.push_str(",\n");
            }
            output.push_str("    ");
            output.push_str(&json_string(&field.name));
            output.push_str(": ");
            output.push_str(&value.json());
        }
        output.push_str("\n  }");
    }
    output.push_str("\n]");
    output
}

fn render_csv(fields: &[FieldDefinition], rows: &[Vec<SampleValue>]) -> String {
    let mut output = String::new();
    let header: Vec<String> = fields.iter().map(|field| quote(&field.name)).collect();
    output.push_str(&header.join(","));
    output.push('\n');
    for row in rows {
        let line: Vec<String> = row.iter().map(|value| quote(&value.display())).collect();
        output.push_str(&line.join(","));
        output.push('\n');
    }
    output
}

/// RFC 4180 quoting: always wrap in quotes and double embedded quotes.
fn quote(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');
    for character in value.chars() {
        if character == '"' {
            result.push('"');
        }
        result.push(character);
    }
    result.push('"');
    result
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("a string always serializes as JSON")
}

/// The Sample Data Utility's identity for the shared [`Utility`] trait.
pub struct SampleData;

impl Utility for SampleData {
    type Request = SampleDataRequest;
    type Evaluation = SampleDataEvaluation;
    type Snapshot = SampleDataSnapshot;

    const ID: &'static str = SAMPLE_DATA_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = SAMPLE_DATA_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> SampleDataEvaluation {
        SampleDataEvaluation::Empty
    }

    fn evaluate(request: &SampleDataRequest) -> SampleDataEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &SampleDataEvaluation) -> bool {
        matches!(evaluation, SampleDataEvaluation::Empty)
    }

    fn snapshot(
        request: &SampleDataRequest,
        evaluation: &SampleDataEvaluation,
    ) -> Option<SampleDataSnapshot> {
        let SampleDataEvaluation::Valid { output, rows } = evaluation else {
            return None;
        };
        Some(SampleDataSnapshot {
            request: request.clone(),
            rows: rows.clone(),
            output: output.clone(),
        })
    }

    fn restore(snapshot: &SampleDataSnapshot) -> (SampleDataRequest, SampleDataEvaluation) {
        (
            snapshot.request.clone(),
            SampleDataEvaluation::Valid {
                output: snapshot.output.clone(),
                rows: snapshot.rows.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic source: a repeating byte pattern and a fixed instant.
    struct FixedSource {
        bytes: Vec<u8>,
        millis: u64,
    }

    impl FixedSource {
        fn new(seed: u8) -> Self {
            Self {
                bytes: (0..64)
                    .map(|index| seed.wrapping_add(index as u8))
                    .collect(),
                millis: 1_700_000_000_000,
            }
        }
    }

    impl IdentifierSource for FixedSource {
        fn fill_random(&self, buffer: &mut [u8]) {
            for (index, slot) in buffer.iter_mut().enumerate() {
                *slot = self.bytes[index % self.bytes.len()];
            }
        }

        fn unix_millis(&self) -> u64 {
            self.millis
        }
    }

    fn generated(request: &SampleDataRequest) -> SampleDataEvaluation {
        generate_with_source(request, &FixedSource::new(0x11))
    }

    #[test]
    fn default_request_is_ten_json_rows_over_three_fields() {
        let request = SampleDataRequest::default();
        assert_eq!(request.row_count, 10);
        assert_eq!(request.output, SampleDataFormat::Json);
        assert_eq!(
            request
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["name", "email", "id"]
        );

        let evaluation = generated(&request);
        assert!(evaluation.is_valid_operation());
        assert_eq!(evaluation.rows().len(), 10);
        assert!(evaluation.rows().iter().all(|row| row.len() == 3));

        let output = evaluation.output().expect("valid output");
        let parsed: serde_json::Value = serde_json::from_str(output).expect("valid JSON");
        assert_eq!(parsed.as_array().expect("an array").len(), 10);
        assert!(output.contains("Fictional"));
        // Field order is stable: the name key precedes the email key.
        assert!(output.find("\"name\"").unwrap() < output.find("\"email\"").unwrap());
        assert!(output.find("\"email\"").unwrap() < output.find("\"id\"").unwrap());
    }

    #[test]
    fn every_field_type_generates_in_order_and_is_deterministic() {
        let request = SampleDataRequest {
            row_count: 2,
            output: SampleDataFormat::Json,
            fields: vec![
                FieldDefinition::new("name", SampleFieldType::FictionalName),
                FieldDefinition::new("email", SampleFieldType::FictionalEmail),
                FieldDefinition {
                    number_minimum: 10.0,
                    number_maximum: 20.0,
                    ..FieldDefinition::new("score", SampleFieldType::Number)
                },
                FieldDefinition {
                    number_minimum: -1.0,
                    number_maximum: 1.0,
                    number_is_integer: false,
                    ..FieldDefinition::new("ratio", SampleFieldType::Number)
                },
                FieldDefinition::new("active", SampleFieldType::Boolean),
                FieldDefinition {
                    date_minimum_seconds: 0.0,
                    date_maximum_seconds: 86_400.0,
                    ..FieldDefinition::new("created", SampleFieldType::Date)
                },
                FieldDefinition::new("id", SampleFieldType::Uuid),
                FieldDefinition {
                    enum_choices: vec!["calm".to_owned(), "👩🏽‍💻".to_owned()],
                    ..FieldDefinition::new("mood", SampleFieldType::Enumeration)
                },
            ],
            generation: 0,
        };
        let first = generated(&request);
        let second = generated(&request);
        assert_eq!(first, second, "a fixed source is deterministic");

        let rows = first.rows();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.len() == 8));

        let uuid = match &rows[0][6] {
            SampleValue::Text(value) => value,
            other => panic!("expected a UUID text cell, got {other:?}"),
        };
        let parsed = uuid::Uuid::parse_str(uuid).expect("a real UUID");
        assert_eq!(parsed.get_version_num(), 4);

        let created = match &rows[0][5] {
            SampleValue::Text(value) => value,
            other => panic!("expected a date text cell, got {other:?}"),
        };
        assert!(created.ends_with('Z') && created.contains('T'));

        let mood = match &rows[0][7] {
            SampleValue::Text(value) => value.as_str(),
            other => panic!("expected an enum text cell, got {other:?}"),
        };
        assert!(["calm", "👩🏽‍💻"].contains(&mood));

        let score = match &rows[0][2] {
            SampleValue::Integer(value) => *value,
            other => panic!("expected an integer cell, got {other:?}"),
        };
        assert!((10..=20).contains(&score));

        let ratio = match &rows[0][3] {
            SampleValue::Decimal(value) => *value,
            other => panic!("expected a decimal cell, got {other:?}"),
        };
        assert!((-1.0..=1.0).contains(&ratio) && ratio.is_finite());

        let output = first.output().expect("valid output");
        let parsed: serde_json::Value = serde_json::from_str(output).expect("valid JSON");
        assert_eq!(parsed.as_array().expect("an array").len(), 2);
    }

    #[test]
    fn csv_quotes_delimiters_quotes_newlines_unicode_and_empty_values() {
        let request = SampleDataRequest {
            row_count: 1,
            output: SampleDataFormat::Csv,
            fields: vec![
                FieldDefinition {
                    enum_choices: vec!["comma, quote \" and\nnewline".to_owned()],
                    ..FieldDefinition::new("value", SampleFieldType::Enumeration)
                },
                FieldDefinition {
                    enum_choices: vec!["👩🏽‍💻".to_owned()],
                    ..FieldDefinition::new("emoji", SampleFieldType::Enumeration)
                },
            ],
            generation: 0,
        };
        let output = generated(&request)
            .output()
            .expect("valid output")
            .to_owned();
        assert_eq!(
            output,
            "\"value\",\"emoji\"\n\"comma, quote \"\" and\nnewline\",\"👩🏽‍💻\"\n"
        );

        // Always-quoted output means an empty value is representable exactly.
        assert_eq!(quote(""), "\"\"");
        assert_eq!(quote("a\"b"), "\"a\"\"b\"");
        assert_eq!(quote("unicode 👩🏽‍💻"), "\"unicode 👩🏽‍💻\"");
    }

    #[test]
    fn row_and_field_bounds_are_enforced_at_both_ends() {
        let zero_rows = SampleDataRequest {
            row_count: 0,
            ..SampleDataRequest::default()
        };
        assert!(diagnostics(&zero_rows)
            .iter()
            .any(|d| d.message.contains("Row count")));

        let too_many_rows = SampleDataRequest {
            row_count: 1_001,
            ..SampleDataRequest::default()
        };
        assert!(diagnostics(&too_many_rows)
            .iter()
            .any(|d| d.message.contains("Row count")));

        let empty = SampleDataRequest {
            fields: Vec::new(),
            ..SampleDataRequest::default()
        };
        assert!(diagnostics(&empty)
            .iter()
            .any(|d| d.message.contains("fields")));

        let too_many = SampleDataRequest {
            fields: (0..=MAXIMUM_FIELD_COUNT)
                .map(|index| {
                    FieldDefinition::new(format!("field{index}"), SampleFieldType::Boolean)
                })
                .collect(),
            ..SampleDataRequest::default()
        };
        assert!(diagnostics(&too_many)
            .iter()
            .any(|d| d.message.contains("fields")));

        // The documented upper bounds are accepted and generatable.
        let maximum = SampleDataRequest {
            row_count: MAXIMUM_ROW_COUNT,
            fields: (0..MAXIMUM_FIELD_COUNT)
                .map(|index| {
                    FieldDefinition::new(format!("field{index}"), SampleFieldType::Boolean)
                })
                .collect(),
            ..SampleDataRequest::default()
        };
        assert!(validate(&maximum).is_empty());
        let evaluation = generated(&maximum);
        assert_eq!(evaluation.rows().len(), MAXIMUM_ROW_COUNT as usize);
        assert!(evaluation
            .rows()
            .iter()
            .all(|row| row.len() == MAXIMUM_FIELD_COUNT));
    }

    #[test]
    fn invalid_names_and_duplicates_are_diagnosed() {
        for name in ["", "1bad", "bad name", "bad-name", "naïve"] {
            let request = SampleDataRequest {
                fields: vec![FieldDefinition::new(name, SampleFieldType::Boolean)],
                ..SampleDataRequest::default()
            };
            let found = diagnostics(&request);
            assert!(
                found.iter().any(|d| d.message.contains("must begin")),
                "{name:?} must be rejected"
            );
        }

        let duplicate = SampleDataRequest {
            fields: vec![
                FieldDefinition::new("same", SampleFieldType::Boolean),
                FieldDefinition::new("same", SampleFieldType::Uuid),
            ],
            ..SampleDataRequest::default()
        };
        assert!(diagnostics(&duplicate)
            .iter()
            .any(|d| d.message.contains("more than once")));
    }

    #[test]
    fn invalid_options_are_diagnosed_before_generation() {
        let cases = [
            FieldDefinition {
                number_minimum: 2.0,
                number_maximum: 1.0,
                ..FieldDefinition::new("n", SampleFieldType::Number)
            },
            FieldDefinition {
                number_minimum: 0.5,
                number_maximum: 2.0,
                number_is_integer: true,
                ..FieldDefinition::new("n", SampleFieldType::Number)
            },
            FieldDefinition {
                date_minimum_seconds: 10.0,
                date_maximum_seconds: 1.0,
                ..FieldDefinition::new("d", SampleFieldType::Date)
            },
            FieldDefinition {
                enum_choices: Vec::new(),
                ..FieldDefinition::new("e", SampleFieldType::Enumeration)
            },
            FieldDefinition {
                enum_choices: vec!["ok".to_owned(), String::new()],
                ..FieldDefinition::new("e", SampleFieldType::Enumeration)
            },
        ];
        for field in cases {
            let request = SampleDataRequest {
                fields: vec![field.clone()],
                ..SampleDataRequest::default()
            };
            let evaluation = generated(&request);
            assert!(!evaluation.is_valid_operation(), "{field:?}");
            assert!(evaluation.output().is_none());
            assert!(!evaluation.diagnostics().is_empty());
        }
    }

    #[test]
    fn snapshot_round_trips_exactly_and_restore_does_not_regenerate() {
        let request = SampleDataRequest {
            row_count: 3,
            ..SampleDataRequest::default()
        };
        let evaluation = generated(&request);
        let snapshot =
            <SampleData as Utility>::snapshot(&request, &evaluation).expect("settled snapshot");
        assert_eq!(snapshot.restore().1, evaluation.output().unwrap());

        let encoded = serde_json::to_value(&snapshot).expect("snapshot serializes");
        let decoded: SampleDataSnapshot =
            serde_json::from_value(encoded).expect("snapshot deserializes");
        assert_eq!(decoded, snapshot);

        let (restored_request, restored_evaluation) = <SampleData as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation, evaluation);
        // Restoring reads only the captured snapshot: a different live draw
        // could never change the restored output.
        let other = generate_with_source(&request, &FixedSource::new(0xEE));
        assert_ne!(other.output(), Some(restored_evaluation.output().unwrap()));
    }

    #[test]
    fn neutral_and_invalid_operations_never_snapshot() {
        let request = SampleDataRequest::default();
        assert!(
            <SampleData as Utility>::snapshot(&request, &SampleDataEvaluation::Empty).is_none()
        );

        let invalid = {
            let mut request = request.clone();
            request.row_count = 0;
            generated(&request)
        };
        assert!(<SampleData as Utility>::snapshot(&request, &invalid).is_none());
    }

    #[test]
    fn repeated_generation_with_a_new_nonce_records_separately() {
        use crate::session::{Session, SubmitOutcome};

        let mut session = Session::<SampleData>::new();
        let first = SampleDataRequest {
            generation: 1,
            ..SampleDataRequest::default()
        };
        let SubmitOutcome::Scheduled(first_revision) = session.submit(first) else {
            panic!("expected a scheduled revision");
        };
        session.resolve(first_revision);
        assert!(session.take_snapshot().is_some());

        let second = SampleDataRequest {
            generation: 2,
            ..SampleDataRequest::default()
        };
        let SubmitOutcome::Scheduled(second_revision) = session.submit(second) else {
            panic!("a repeated Generate is a new revision");
        };
        assert!(second_revision > first_revision);
        session.resolve(second_revision);
        assert!(
            session.take_snapshot().is_some(),
            "a deliberate repeated generation records separately"
        );
    }

    #[test]
    fn csv_and_json_render_the_same_rows() {
        let request = SampleDataRequest {
            row_count: 2,
            output: SampleDataFormat::Csv,
            fields: vec![
                FieldDefinition::new("active", SampleFieldType::Boolean),
                FieldDefinition::new("id", SampleFieldType::Uuid),
            ],
            generation: 0,
        };
        let csv = generated(&request)
            .output()
            .expect("valid output")
            .to_owned();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "header plus two rows");
        assert_eq!(lines[0], "\"active\",\"id\"");
        assert!(lines[1].starts_with("\"true\",\"") || lines[1].starts_with("\"false\",\""));
    }

    fn diagnostics(request: &SampleDataRequest) -> Vec<Diagnostic> {
        match generated(request) {
            SampleDataEvaluation::Invalid { diagnostics } => diagnostics,
            other => panic!("expected an invalid evaluation, got {other:?}"),
        }
    }
}
