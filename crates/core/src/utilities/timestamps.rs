//! The Timestamps Utility's GPUI-independent request/result/snapshot contract.
//!
//! Inference follows the established product policy: signed integers of at
//! most ten digits are Unix seconds, exactly thirteen digits are Unix
//! milliseconds, and eleven or twelve digits are ambiguous and require a
//! manual mode. Unix Seconds accepts a decimal fraction; Auto does not, so
//! fractional seconds require the explicit mode. ISO 8601 input must identify
//! one instant with `Z` or an explicit offset. Local time requires a named
//! IANA zone and rejects repeated (fall-back) and nonexistent (spring-forward)
//! daylight-saving wall times rather than guessing an instant.
//!
//! `Now` is exposed through the injectable [`TimestampSource`] seam. The
//! production [`Utility::evaluate`] never reads a clock, so evaluation is
//! deterministic and restoring a snapshot never consults the clock again.

use chrono::{
    DateTime, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Timestamps Utility.
pub const TIMESTAMPS_UTILITY_ID: &str = "timestamps";

/// Schema version of [`TimestampsSnapshot`].
pub const TIMESTAMPS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// How the input is interpreted. `Auto` infers from the exact input shape; the
/// other variants require the matching explicit format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampMode {
    Auto,
    UnixSeconds,
    UnixMilliseconds,
    Iso8601,
    Local,
}

impl TimestampMode {
    /// Declaration order, used for stable control order and focus indexing.
    pub const ALL: [TimestampMode; 5] = [
        TimestampMode::Auto,
        TimestampMode::UnixSeconds,
        TimestampMode::UnixMilliseconds,
        TimestampMode::Iso8601,
        TimestampMode::Local,
    ];

    /// Human-readable label, matching the product baseline.
    pub const fn label(self) -> &'static str {
        match self {
            TimestampMode::Auto => "Auto",
            TimestampMode::UnixSeconds => "Unix Seconds",
            TimestampMode::UnixMilliseconds => "Unix Milliseconds",
            TimestampMode::Iso8601 => "ISO 8601",
            TimestampMode::Local => "Local Time",
        }
    }

    /// Stable index in [`TimestampMode::ALL`].
    pub const fn index(self) -> usize {
        match self {
            TimestampMode::Auto => 0,
            TimestampMode::UnixSeconds => 1,
            TimestampMode::UnixMilliseconds => 2,
            TimestampMode::Iso8601 => 3,
            TimestampMode::Local => 4,
        }
    }
}

/// The label describing how a valid input was interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampInference {
    UnixSeconds,
    UnixMilliseconds,
    Iso8601,
    Local,
    Now,
}

impl TimestampInference {
    /// Human-readable label, matching the product baseline.
    pub const fn label(self) -> &'static str {
        match self {
            TimestampInference::UnixSeconds => "Unix Seconds",
            TimestampInference::UnixMilliseconds => "Unix Milliseconds",
            TimestampInference::Iso8601 => "ISO 8601 with explicit offset",
            TimestampInference::Local => "Local Time with named timezone",
            TimestampInference::Now => "Now action",
        }
    }
}

/// Whether a request came from typing/pasting or from the explicit Now action.
/// The distinction only affects the inference label; it never affects parsing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampOrigin {
    #[default]
    Typed,
    Now,
}

/// The complete, strongly typed input for one evaluation.
///
/// `generation` is a monotonic nonce bumped by every explicit Now action. It is
/// part of request identity, so two deliberate Now actions in the same second
/// remain distinct revisions and record separately instead of being deduped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampsRequest {
    pub input: String,
    pub mode: TimestampMode,
    /// A named IANA zone such as `UTC` or `America/New_York`.
    pub zone: String,
    #[serde(default)]
    pub origin: TimestampOrigin,
    #[serde(default)]
    pub generation: u64,
}

impl TimestampsRequest {
    /// A typed/pasted request. `origin` defaults to [`TimestampOrigin::Typed`].
    pub fn typed(input: impl Into<String>, mode: TimestampMode, zone: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            mode,
            zone: zone.into(),
            origin: TimestampOrigin::Typed,
            generation: 0,
        }
    }

    /// The explicit Now action: reads `source`, renders its instant as ISO 8601
    /// with `Z`, and marks the request so evaluation labels it as Now.
    pub fn now(source: &dyn TimestampSource, zone: impl Into<String>, generation: u64) -> Self {
        let instant = TimestampInstant::from_datetime(source.now());
        Self {
            input: format_iso8601(instant),
            mode: TimestampMode::Iso8601,
            zone: zone.into(),
            origin: TimestampOrigin::Now,
            generation,
        }
    }
}

/// An exact instant as whole seconds plus non-negative nanoseconds within the
/// second. Storing the instant this way keeps snapshots self-contained,
/// serializable and independent of any clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampInstant {
    pub seconds: i64,
    pub nanoseconds: u32,
}

impl TimestampInstant {
    pub fn from_datetime(instant: DateTime<Utc>) -> Self {
        Self {
            seconds: instant.timestamp(),
            nanoseconds: instant.timestamp_subsec_nanos(),
        }
    }

    /// The instant as a UTC date-time, or `None` when it is outside chrono's
    /// representable range.
    pub fn to_datetime(self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.seconds, self.nanoseconds)
    }

    pub fn unix_seconds(self) -> i64 {
        self.seconds
    }

    pub fn subsec_nanoseconds(self) -> u32 {
        self.nanoseconds
    }

    /// Converts a possibly-fractional Unix-seconds value without losing the
    /// fraction beyond nanosecond precision. Returns `None` for non-finite or
    /// out-of-range values.
    pub fn from_unix_seconds(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        let truncated = value.trunc();
        if truncated < i64::MIN as f64 || truncated > i64::MAX as f64 {
            return None;
        }
        let seconds = truncated as i64;
        let fraction = value - truncated;
        let nanosecond_draw = (fraction * 1_000_000_000.0).round();
        let (seconds, nanoseconds) = if nanosecond_draw >= 1_000_000_000.0 {
            (seconds.checked_add(1)?, 0)
        } else if nanosecond_draw < 0.0 {
            (
                seconds.checked_sub(1)?,
                (nanosecond_draw + 1_000_000_000.0).clamp(0.0, 999_999_999.0) as u32,
            )
        } else {
            (seconds, nanosecond_draw as u32)
        };
        DateTime::from_timestamp(seconds, nanoseconds).map(Self::from_datetime)
    }
}

/// Injectable source of the current instant.
///
/// Production uses [`SystemTimestampSource`]; tests implement this trait with a
/// fixed instant so `Now` is deterministic.
pub trait TimestampSource {
    /// The current instant.
    fn now(&self) -> DateTime<Utc>;
}

/// The production source: the system clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemTimestampSource;

impl TimestampSource for SystemTimestampSource {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// The synchronized, locale-stable renderings of one instant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampRepresentations {
    /// How the input was interpreted, labelled for the user.
    pub inference: TimestampInference,
    /// RFC 3339 in UTC, whole seconds or milliseconds when a fraction exists.
    pub iso8601: String,
    /// `YYYY-MM-DD HH:mm:ss UTC`.
    pub utc: String,
    /// `YYYY-MM-DD HH:mm:ss` in the selected zone.
    pub zone_wall_time: String,
    /// Canonical name of the selected zone.
    pub zone_identifier: String,
}

impl TimestampRepresentations {
    /// The multi-line, locale-stable text shown and copied by the workspace.
    pub fn display_text(&self) -> String {
        format!(
            "Inferred input: {}\nISO 8601: {}\nUTC: {}\n{}: {}",
            self.inference.label(),
            self.iso8601,
            self.utc,
            self.zone_identifier,
            self.zone_wall_time,
        )
    }
}

/// The typed outcome shown to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimestampsEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        instant: TimestampInstant,
        representations: TimestampRepresentations,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl TimestampsEvaluation {
    pub fn instant(&self) -> Option<TimestampInstant> {
        match self {
            TimestampsEvaluation::Valid { instant, .. } => Some(*instant),
            _ => None,
        }
    }

    pub fn representations(&self) -> Option<&TimestampRepresentations> {
        match self {
            TimestampsEvaluation::Valid {
                representations, ..
            } => Some(representations),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            TimestampsEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, TimestampsEvaluation::Valid { .. })
    }

    /// The display text for a valid evaluation, or `None` otherwise.
    pub fn display_text(&self) -> Option<String> {
        self.representations()
            .map(TimestampRepresentations::display_text)
    }
}

/// The exact captured instant and its renderings, so preview and restore never
/// rerun the conversion and never read the clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampsSnapshot {
    pub request: TimestampsRequest,
    pub instant: TimestampInstant,
    pub representations: TimestampRepresentations,
}

impl TimestampsSnapshot {
    pub fn restore(
        &self,
    ) -> (
        &TimestampsRequest,
        TimestampInstant,
        &TimestampRepresentations,
    ) {
        (&self.request, self.instant, &self.representations)
    }
}

/// Interprets `request.input` under `request.mode` and renders the instant in
/// `request.zone`. Pure and deterministic: it never reads a clock.
pub fn evaluate(request: &TimestampsRequest) -> TimestampsEvaluation {
    let input = request.input.trim();
    if input.is_empty() {
        return TimestampsEvaluation::Empty;
    }
    let zone = match parse_zone(request.zone.trim()) {
        Some(zone) => zone,
        None => {
            return invalid(format!(
                "Unknown timezone \"{}\". Choose a named IANA zone such as UTC or America/New_York.",
                request.zone.trim()
            ));
        }
    };
    match resolve_instant(input, request.mode, zone) {
        Ok((instant, inferred)) => {
            let inference = if request.origin == TimestampOrigin::Now {
                TimestampInference::Now
            } else {
                inferred
            };
            TimestampsEvaluation::Valid {
                instant,
                representations: render(instant, inference, zone),
            }
        }
        Err(message) => invalid(message),
    }
}

fn invalid(message: impl Into<String>) -> TimestampsEvaluation {
    TimestampsEvaluation::Invalid {
        diagnostics: vec![Diagnostic::error(message)],
    }
}

/// Resolves a named IANA zone, accepting the exact spelling first and then any
/// case-insensitive match so `utc` and `UTC` agree. Callers render the
/// canonical [`Tz::name`].
fn parse_zone(value: &str) -> Option<Tz> {
    if let Ok(zone) = value.parse::<Tz>() {
        return Some(zone);
    }
    chrono_tz::TZ_VARIANTS
        .iter()
        .copied()
        .find(|zone| zone.name().eq_ignore_ascii_case(value))
}

fn resolve_instant(
    input: &str,
    mode: TimestampMode,
    zone: Tz,
) -> Result<(TimestampInstant, TimestampInference), String> {
    match mode {
        TimestampMode::Auto => resolve_auto(input),
        TimestampMode::UnixSeconds => {
            Ok((parse_unix_seconds(input)?, TimestampInference::UnixSeconds))
        }
        TimestampMode::UnixMilliseconds => Ok((
            parse_unix_milliseconds(input)?,
            TimestampInference::UnixMilliseconds,
        )),
        TimestampMode::Iso8601 => Ok((parse_iso8601(input)?, TimestampInference::Iso8601)),
        TimestampMode::Local => Ok((parse_local(input, zone)?, TimestampInference::Local)),
    }
}

/// Exact digit-count inference from the parent specification.
fn resolve_auto(input: &str) -> Result<(TimestampInstant, TimestampInference), String> {
    if let Some(digits) = signed_integer_digits(input) {
        return match digits {
            0..=10 => Ok((parse_unix_seconds(input)?, TimestampInference::UnixSeconds)),
            11..=12 => Err(
                "An 11- or 12-digit integer is ambiguous. Choose Unix Seconds or Unix Milliseconds."
                    .to_owned(),
            ),
            13 => Ok((
                parse_unix_milliseconds(input)?,
                TimestampInference::UnixMilliseconds,
            )),
            _ => Err(
                "Auto recognizes at most 10 digits as Unix seconds and exactly 13 digits as Unix milliseconds."
                    .to_owned(),
            ),
        };
    }
    if !contains_explicit_iso_offset(input) {
        return Err(
            "Auto requires ISO 8601 input to include Z or an explicit offset. Choose Local Time for a named timezone."
                .to_owned(),
        );
    }
    Ok((parse_iso8601(input)?, TimestampInference::Iso8601))
}

fn parse_unix_seconds(input: &str) -> Result<TimestampInstant, String> {
    if signed_integer_digits(input).is_some() {
        let seconds: i64 = input
            .parse()
            .map_err(|_| "Unix Seconds is outside the representable range.".to_owned())?;
        return DateTime::from_timestamp(seconds, 0)
            .map(TimestampInstant::from_datetime)
            .ok_or_else(|| "Unix Seconds is outside the representable range.".to_owned());
    }
    if !is_decimal_seconds(input) {
        return Err("Unix Seconds requires a finite integer or decimal number.".to_owned());
    }
    let value: f64 = input
        .parse()
        .map_err(|_| "Unix Seconds requires a finite integer or decimal number.".to_owned())?;
    TimestampInstant::from_unix_seconds(value)
        .ok_or_else(|| "Unix Seconds is outside the representable range.".to_owned())
}

fn parse_unix_milliseconds(input: &str) -> Result<TimestampInstant, String> {
    if signed_integer_digits(input).is_none() {
        return Err("Unix Milliseconds requires a finite integer number.".to_owned());
    }
    let milliseconds: i64 = input
        .parse()
        .map_err(|_| "Unix Milliseconds is outside the representable range.".to_owned())?;
    let seconds = milliseconds.div_euclid(1_000);
    let nanoseconds = (milliseconds.rem_euclid(1_000) * 1_000_000) as u32;
    DateTime::from_timestamp(seconds, nanoseconds)
        .map(TimestampInstant::from_datetime)
        .ok_or_else(|| "Unix Milliseconds is outside the representable range.".to_owned())
}

fn parse_iso8601(input: &str) -> Result<TimestampInstant, String> {
    if !contains_explicit_iso_offset(input) {
        return Err(
            "ISO 8601 input must include Z or an explicit offset so it identifies one instant."
                .to_owned(),
        );
    }
    DateTime::parse_from_rfc3339(input)
        .map(|date| TimestampInstant::from_datetime(date.with_timezone(&Utc)))
        .map_err(|_| "Input is not a valid ISO 8601 timestamp with an explicit offset.".to_owned())
}

fn parse_local(input: &str, zone: Tz) -> Result<TimestampInstant, String> {
    let naive = parse_wall_time(input).ok_or_else(|| {
        "Local Time requires YYYY-MM-DD HH:mm:ss and a named timezone.".to_owned()
    })?;
    match zone.from_local_datetime(&naive) {
        LocalResult::Single(date) => Ok(TimestampInstant::from_datetime(date.with_timezone(&Utc))),
        LocalResult::Ambiguous(_, _) => Err(format!(
            "That local wall time repeats in {} because of a daylight-saving transition. Use an explicit offset.",
            zone.name()
        )),
        LocalResult::None => Err(format!(
            "That local wall time does not exist in {} because of a daylight-saving transition. Use an explicit offset.",
            zone.name()
        )),
    }
}

/// The number of ASCII digits after an optional sign, or `None` when the input
/// is not a signed integer. Leading zeros count as digits, matching the baseline.
fn signed_integer_digits(value: &str) -> Option<usize> {
    let digits = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(digits.len())
}

/// True for a finite decimal number with at most one `.` and at least one digit.
fn is_decimal_seconds(value: &str) -> bool {
    let unsigned = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    let mut components = unsigned.split('.');
    let whole = components.next().unwrap_or_default();
    let is_digits = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    match components.next() {
        Some(fraction) => {
            if components.next().is_some() {
                return false;
            }
            is_digits(whole) && is_digits(fraction) && !fraction.is_empty()
        }
        None => !whole.is_empty() && is_digits(whole),
    }
}

/// True when the value ends in `Z`/`z` or carries a trailing `±HH:MM` offset.
fn contains_explicit_iso_offset(value: &str) -> bool {
    let bytes = value.as_bytes();
    if matches!(bytes.last(), Some(b'Z') | Some(b'z')) {
        return true;
    }
    if bytes.len() < 6 {
        return false;
    }
    let suffix = &bytes[bytes.len() - 6..];
    (suffix[0] == b'+' || suffix[0] == b'-')
        && suffix[3] == b':'
        && suffix[1].is_ascii_digit()
        && suffix[2].is_ascii_digit()
        && suffix[4].is_ascii_digit()
        && suffix[5].is_ascii_digit()
}

/// Strict `YYYY-MM-DD HH:mm:ss` (or `T`-separated) wall time with two-digit
/// fields, matching the baseline's exact-shape requirement.
fn parse_wall_time(input: &str) -> Option<NaiveDateTime> {
    if input.chars().count() != 19 {
        return None;
    }
    let normalized: String = input
        .chars()
        .map(|character| if character == 'T' { ' ' } else { character })
        .collect();
    let mut fields = normalized.split(' ');
    let date = fields.next()?;
    let time = fields.next()?;
    if fields.next().is_some() {
        return None;
    }

    let mut date_parts = date.split('-');
    let year = date_parts.next()?;
    let month = date_parts.next()?;
    let day = date_parts.next()?;
    if date_parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }

    let mut time_parts = time.split(':');
    let hour = time_parts.next()?;
    let minute = time_parts.next()?;
    let second = time_parts.next()?;
    if time_parts.next().is_some() || hour.len() != 2 || minute.len() != 2 || second.len() != 2 {
        return None;
    }

    let year: i32 = year.parse().ok()?;
    let month: u32 = month.parse().ok()?;
    let day: u32 = day.parse().ok()?;
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    let second: u32 = second.parse().ok()?;

    Some(NaiveDateTime::new(
        NaiveDate::from_ymd_opt(year, month, day)?,
        NaiveTime::from_hms_opt(hour, minute, second)?,
    ))
}

fn render(
    instant: TimestampInstant,
    inference: TimestampInference,
    zone: Tz,
) -> TimestampRepresentations {
    let date = instant
        .to_datetime()
        .expect("an evaluated instant is representable");
    TimestampRepresentations {
        inference,
        iso8601: iso8601_string(date),
        utc: format!("{} UTC", date.format("%Y-%m-%d %H:%M:%S")),
        zone_wall_time: date
            .with_timezone(&zone)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        zone_identifier: zone.name().to_owned(),
    }
}

/// RFC 3339 in UTC. Whole seconds stay compact; a fractional instant renders at
/// millisecond precision, matching the baseline's display.
fn iso8601_string(instant: DateTime<Utc>) -> String {
    if instant.timestamp_subsec_nanos() == 0 {
        instant.to_rfc3339_opts(SecondsFormat::Secs, true)
    } else {
        instant.to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

fn format_iso8601(instant: TimestampInstant) -> String {
    iso8601_string(
        instant
            .to_datetime()
            .expect("a source instant is representable"),
    )
}

/// The Timestamps Utility's identity for the shared [`Utility`] trait.
pub struct Timestamps;

impl Utility for Timestamps {
    type Request = TimestampsRequest;
    type Evaluation = TimestampsEvaluation;
    type Snapshot = TimestampsSnapshot;

    const ID: &'static str = TIMESTAMPS_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = TIMESTAMPS_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> TimestampsEvaluation {
        TimestampsEvaluation::Empty
    }

    fn evaluate(request: &TimestampsRequest) -> TimestampsEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &TimestampsEvaluation) -> bool {
        matches!(evaluation, TimestampsEvaluation::Empty)
    }

    fn snapshot(
        request: &TimestampsRequest,
        evaluation: &TimestampsEvaluation,
    ) -> Option<TimestampsSnapshot> {
        let TimestampsEvaluation::Valid {
            instant,
            representations,
        } = evaluation
        else {
            return None;
        };
        Some(TimestampsSnapshot {
            request: request.clone(),
            instant: *instant,
            representations: representations.clone(),
        })
    }

    fn restore(snapshot: &TimestampsSnapshot) -> (TimestampsRequest, TimestampsEvaluation) {
        (
            snapshot.request.clone(),
            TimestampsEvaluation::Valid {
                instant: snapshot.instant,
                representations: snapshot.representations.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds_request(input: &str, zone: &str) -> TimestampsRequest {
        TimestampsRequest::typed(input, TimestampMode::UnixSeconds, zone)
    }

    fn auto_request(input: &str, zone: &str) -> TimestampsRequest {
        TimestampsRequest::typed(input, TimestampMode::Auto, zone)
    }

    fn valid(input: &str, mode: TimestampMode, zone: &str) -> TimestampsEvaluation {
        let evaluation = evaluate(&TimestampsRequest::typed(input, mode, zone));
        assert!(
            evaluation.is_valid_operation(),
            "{input:?} ({mode:?}) should be valid, got {evaluation:?}"
        );
        evaluation
    }

    fn first_diagnostic(evaluation: &TimestampsEvaluation) -> &str {
        evaluation
            .diagnostics()
            .first()
            .expect("an invalid evaluation has a diagnostic")
            .message
            .as_str()
    }

    struct FixedSource(DateTime<Utc>);

    impl TimestampSource for FixedSource {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    #[test]
    fn empty_and_whitespace_input_is_neutral() {
        for input in ["", "   ", "\n\t"] {
            assert_eq!(
                evaluate(&auto_request(input, "UTC")),
                TimestampsEvaluation::Empty
            );
        }
    }

    #[test]
    fn digit_count_inference_selects_seconds_milliseconds_and_rejects_ambiguous() {
        let ten_digits = valid("1700000000", TimestampMode::Auto, "UTC");
        assert_eq!(
            ten_digits.representations().unwrap().inference,
            TimestampInference::UnixSeconds
        );
        assert_eq!(ten_digits.instant().unwrap().unix_seconds(), 1_700_000_000);

        let thirteen_digits = valid("1700000000000", TimestampMode::Auto, "UTC");
        assert_eq!(
            thirteen_digits.representations().unwrap().inference,
            TimestampInference::UnixMilliseconds
        );
        assert_eq!(
            thirteen_digits.instant().unwrap().unix_seconds(),
            1_700_000_000
        );

        for ambiguous in ["17000000000", "170000000000"] {
            let evaluation = evaluate(&auto_request(ambiguous, "UTC"));
            assert!(first_diagnostic(&evaluation).contains("ambiguous"));
        }

        for too_long in ["17000000000000", "170000000000000"] {
            let evaluation = evaluate(&auto_request(too_long, "UTC"));
            assert!(first_diagnostic(&evaluation).contains("Auto recognizes"));
        }
    }

    #[test]
    fn leading_zeros_still_count_as_digits() {
        let evaluation = valid("0000000001", TimestampMode::Auto, "UTC");
        assert_eq!(
            evaluation.representations().unwrap().inference,
            TimestampInference::UnixSeconds
        );
        assert_eq!(evaluation.instant().unwrap().unix_seconds(), 1);
    }

    #[test]
    fn negative_and_boundary_seconds_are_exact() {
        let negative = valid("-1", TimestampMode::UnixSeconds, "UTC");
        assert_eq!(negative.instant().unwrap().unix_seconds(), -1);
        assert_eq!(
            negative.representations().unwrap().iso8601,
            "1969-12-31T23:59:59Z"
        );

        let plus = valid("+0", TimestampMode::UnixSeconds, "UTC");
        assert_eq!(plus.instant().unwrap().unix_seconds(), 0);

        assert!(evaluate(&seconds_request("9223372036854775807", "UTC"))
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("range")));
        assert!(evaluate(&seconds_request("-9223372036854775808", "UTC"))
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("range")));
        // One past i64::MAX does not even fit the integer parser.
        assert!(evaluate(&seconds_request("9223372036854775808", "UTC"))
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("range")));
    }

    #[test]
    fn fractional_seconds_require_the_explicit_seconds_mode() {
        let auto = evaluate(&auto_request("1.5", "UTC"));
        assert!(!auto.is_valid_operation());
        assert!(first_diagnostic(&auto).contains("ISO 8601"));

        let explicit = valid("1.5", TimestampMode::UnixSeconds, "UTC");
        assert_eq!(explicit.instant().unwrap().seconds, 1);
        assert_eq!(explicit.instant().unwrap().nanoseconds, 500_000_000);
        assert_eq!(
            explicit.representations().unwrap().iso8601,
            "1970-01-01T00:00:01.500Z"
        );

        let leading = valid(".25", TimestampMode::UnixSeconds, "UTC");
        assert_eq!(leading.instant().unwrap().nanoseconds, 250_000_000);

        for malformed in ["5.", ".", "1.2.3", "1e3", "abc"] {
            let evaluation = evaluate(&seconds_request(malformed, "UTC"));
            assert!(
                !evaluation.is_valid_operation(),
                "{malformed:?} must not be a valid Unix Seconds value"
            );
        }
    }

    #[test]
    fn negative_fractional_seconds_normalize_to_a_positive_nanosecond() {
        let evaluation = valid("-1.5", TimestampMode::UnixSeconds, "UTC");
        let instant = evaluation.instant().unwrap();
        assert_eq!(instant.seconds, -2);
        assert_eq!(instant.nanoseconds, 500_000_000);
        assert_eq!(
            evaluation.representations().unwrap().iso8601,
            "1969-12-31T23:59:58.500Z"
        );
    }

    #[test]
    fn milliseconds_floor_negative_values() {
        let evaluation = valid("-1", TimestampMode::UnixMilliseconds, "UTC");
        let instant = evaluation.instant().unwrap();
        assert_eq!(instant.seconds, -1);
        assert_eq!(instant.nanoseconds, 999_000_000);
        assert_eq!(
            evaluation.representations().unwrap().iso8601,
            "1969-12-31T23:59:59.999Z"
        );

        // Milliseconds mode rejects fractions and other shapes.
        for malformed in ["1.5", "1e3", "-", "+"] {
            assert!(!evaluate(&TimestampsRequest::typed(
                malformed,
                TimestampMode::UnixMilliseconds,
                "UTC"
            ))
            .is_valid_operation());
        }
    }

    #[test]
    fn iso_requires_an_explicit_offset_or_z() {
        let naive = evaluate(&TimestampsRequest::typed(
            "2023-11-14T22:13:20",
            TimestampMode::Iso8601,
            "UTC",
        ));
        assert!(!naive.is_valid_operation());
        assert!(first_diagnostic(&naive).contains("explicit offset"));

        let auto_naive = evaluate(&auto_request("2023-11-14T22:13:20", "UTC"));
        assert!(first_diagnostic(&auto_naive).contains("Z or an explicit offset"));

        let with_z = valid("2023-11-14T22:13:20Z", TimestampMode::Iso8601, "UTC");
        assert_eq!(with_z.instant().unwrap().unix_seconds(), 1_700_000_000);

        let with_offset = valid("2023-11-15T00:13:20+02:00", TimestampMode::Iso8601, "UTC");
        assert_eq!(with_offset.instant().unwrap().unix_seconds(), 1_700_000_000);
        assert_eq!(
            with_offset.representations().unwrap().iso8601,
            "2023-11-14T22:13:20Z"
        );

        let auto_offset = evaluate(&auto_request("2023-11-14T22:13:20+00:00", "UTC"));
        assert_eq!(
            auto_offset.representations().unwrap().inference,
            TimestampInference::Iso8601
        );
    }

    #[test]
    fn local_time_requires_a_named_zone_and_known_fixtures_resolve() {
        let new_york = valid(
            "2023-11-14 17:13:20",
            TimestampMode::Local,
            "America/New_York",
        );
        let representations = new_york.representations().unwrap();
        assert_eq!(representations.inference, TimestampInference::Local);
        assert_eq!(representations.iso8601, "2023-11-14T22:13:20Z");
        assert_eq!(representations.utc, "2023-11-14 22:13:20 UTC");
        assert_eq!(representations.zone_wall_time, "2023-11-14 17:13:20");
        assert_eq!(representations.zone_identifier, "America/New_York");

        // A half-hour offset fixture.
        let kolkata = valid("2023-11-14 17:13:20", TimestampMode::Local, "Asia/Kolkata");
        assert_eq!(
            kolkata.representations().unwrap().iso8601,
            "2023-11-14T11:43:20Z"
        );

        let malformed = evaluate(&TimestampsRequest::typed(
            "2023-11-14 17:13",
            TimestampMode::Local,
            "UTC",
        ));
        assert!(first_diagnostic(&malformed).contains("YYYY-MM-DD"));
    }

    #[test]
    fn local_time_rejects_dst_gaps_and_folds() {
        let gap = evaluate(&TimestampsRequest::typed(
            "2023-03-12 02:30:00",
            TimestampMode::Local,
            "America/New_York",
        ));
        assert!(!gap.is_valid_operation());
        assert!(first_diagnostic(&gap).contains("does not exist"));

        let fold = evaluate(&TimestampsRequest::typed(
            "2023-11-05 01:30:00",
            TimestampMode::Local,
            "America/New_York",
        ));
        assert!(!fold.is_valid_operation());
        assert!(first_diagnostic(&fold).contains("repeats"));
    }

    #[test]
    fn unknown_zones_are_rejected_for_every_mode() {
        let evaluation = evaluate(&auto_request("1700000000", "Mars/Phobos"));
        assert!(first_diagnostic(&evaluation).contains("timezone"));
        assert!(!evaluation.is_valid_operation());
    }

    #[test]
    fn zone_names_are_canonicalized_and_output_is_locale_stable() {
        let evaluation = valid("1700000000", TimestampMode::UnixSeconds, "utc");
        let representations = evaluation.representations().unwrap();
        assert_eq!(representations.zone_identifier, "UTC");
        assert_eq!(representations.utc, "2023-11-14 22:13:20 UTC");
        assert_eq!(
            representations.display_text(),
            "Inferred input: Unix Seconds\nISO 8601: 2023-11-14T22:13:20Z\nUTC: 2023-11-14 22:13:20 UTC\nUTC: 2023-11-14 22:13:20"
        );
    }

    #[test]
    fn now_reads_the_injected_clock_and_labels_the_inference() {
        let source =
            FixedSource(DateTime::from_timestamp(1_700_000_000, 0).expect("representable fixture"));
        let request = TimestampsRequest::now(&source, "America/New_York", 7);

        assert_eq!(request.input, "2023-11-14T22:13:20Z");
        assert_eq!(request.mode, TimestampMode::Iso8601);
        assert_eq!(request.origin, TimestampOrigin::Now);
        assert_eq!(request.generation, 7);

        let evaluation = evaluate(&request);
        let representations = evaluation.representations().unwrap();
        assert_eq!(representations.inference, TimestampInference::Now);
        assert_eq!(representations.zone_wall_time, "2023-11-14 17:13:20");
        assert_eq!(
            representations.display_text(),
            "Inferred input: Now action\nISO 8601: 2023-11-14T22:13:20Z\nUTC: 2023-11-14 22:13:20 UTC\nAmerica/New_York: 2023-11-14 17:13:20"
        );
    }

    #[test]
    fn snapshot_serializes_exactly_and_restores_without_reading_the_clock() {
        let request = auto_request("1700000000", "UTC");
        let evaluation = evaluate(&request);
        let snapshot = <Timestamps as Utility>::snapshot(&request, &evaluation).expect("snapshot");

        let json = serde_json::to_value(&snapshot).expect("snapshot serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "request": {
                    "input": "1700000000",
                    "mode": "auto",
                    "zone": "UTC",
                    "origin": "typed",
                    "generation": 0
                },
                "instant": { "seconds": 1700000000i64, "nanoseconds": 0u32 },
                "representations": {
                    "inference": "unix_seconds",
                    "iso8601": "2023-11-14T22:13:20Z",
                    "utc": "2023-11-14 22:13:20 UTC",
                    "zone_wall_time": "2023-11-14 22:13:20",
                    "zone_identifier": "UTC"
                }
            })
        );

        let round_tripped: TimestampsSnapshot =
            serde_json::from_value(json).expect("snapshot deserializes");
        assert_eq!(round_tripped, snapshot);

        let (restored_request, restored) = <Timestamps as Utility>::restore(&snapshot);
        assert_eq!(restored_request, request);
        assert_eq!(restored, evaluation);
        assert_eq!(Timestamps::SNAPSHOT_VERSION, 1);
        assert_eq!(Timestamps::ID, "timestamps");
    }

    #[test]
    fn a_now_snapshot_restores_the_captured_label_and_instant() {
        let source = FixedSource(
            DateTime::from_timestamp(1_700_000_000, 123_000_000).expect("representable fixture"),
        );
        let request = TimestampsRequest::now(&source, "Asia/Kolkata", 1);
        let evaluation = evaluate(&request);
        let snapshot = <Timestamps as Utility>::snapshot(&request, &evaluation).expect("snapshot");

        let (_, restored) = <Timestamps as Utility>::restore(&snapshot);
        let representations = restored.representations().unwrap();
        assert_eq!(representations.inference, TimestampInference::Now);
        assert_eq!(representations.zone_identifier, "Asia/Kolkata");
        assert_eq!(snapshot.instant.nanoseconds, 123_000_000);
    }

    #[test]
    fn invalid_and_neutral_evaluations_never_snapshot() {
        let invalid = evaluate(&auto_request("17000000000", "UTC"));
        assert!(
            <Timestamps as Utility>::snapshot(&auto_request("17000000000", "UTC"), &invalid)
                .is_none()
        );
        assert!(<Timestamps as Utility>::snapshot(
            &auto_request("", "UTC"),
            &<Timestamps as Utility>::neutral()
        )
        .is_none());
    }
}
