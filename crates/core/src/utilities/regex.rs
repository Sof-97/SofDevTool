//! The Regex Utility's GPUI-independent request/result/snapshot contract.
//!
//! This Utility intentionally uses the pinned Rust `regex` crate dialect. It is
//! not ICU and no ICU behavior is emulated. It rejects look-around
//! (look-ahead/look-behind) and backreferences at
//! compile time; those failures become explicit diagnostics rather than being
//! translated.
//!
//! # Supported syntax
//!
//! Ordinary Rust `regex` syntax: literals, `.`, character classes, anchors,
//! alternation, repetition, groups and named groups (`(?P<name>…)` or
//! `(?<name>…)`). Unicode mode is always enabled, so classes and offsets are
//! Unicode-aware.
//!
//! # Supported flags
//!
//! | Flag | Meaning |
//! | --- | --- |
//! | `case_insensitive` (`i`) | ASCII/Unicode case-insensitive matching |
//! | `multi_line` (`m`) | `^` and `$` match at line boundaries |
//! | `dot_matches_new_line` (`s`) | `.` also matches `\n` |
//! | `ignore_whitespace` (`x`) | Ignore pattern whitespace and allow `#` comments |
//! | `swap_greed` (`U`) | Swap the meaning of greedy and lazy repetition |
//! | `crlf` (`R`) | Treat `\r\n` as the line terminator in multi-line mode |
//!
//! # Replacement syntax
//!
//! `$1`/`${1}` refer to numbered groups, `$name`/`${name}` to named groups and
//! `$$` to a literal `$`. A reference to a group that did not participate in a
//! match expands to the empty string.
//!
//! # Byte offsets
//!
//! Every [`TextRange`] is a half-open **UTF-8 byte offset** range into the exact
//! input text, so a multi-byte character reports the byte span it occupies.
//!
//! # Named limits
//!
//! Work is bounded before any result is published. Over-limit input is refused
//! with a diagnostic; no partial matches or replacement are returned, the input
//! is left untouched and nothing is recorded to History.
//!
//! | Limit | Default | Applies to |
//! | --- | --- | --- |
//! | [`MAX_PATTERN_BYTES`] | 16 KiB | Pattern source length |
//! | [`PATTERN_PROGRAM_SIZE_LIMIT`] | 1 MiB | Approximate compiled program size |
//! | [`MAX_TEXT_BYTES`] | 1 MiB | Test text length |
//! | [`MAX_REPLACEMENT_BYTES`] | 256 KiB | Replacement template length |
//! | [`MAX_MATCHES`] | 10,000 | Number of matches |
//! | [`MAX_CAPTURE_SLOTS`] | 50,000 | Total capture groups across matches |
//! | [`MAX_REPLACEMENT_OUTPUT_BYTES`] | 2 MiB | Replacement preview length |

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Regex Utility.
pub const REGEX_UTILITY_ID: &str = "rust-regex";

/// Schema version of [`RegexSnapshot`].
pub const REGEX_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// The visible engine label shown next to the Regex workspace.
pub const RUST_REGEX_ENGINE_LABEL: &str = "Rust regex engine (regex crate)";

/// The visible dialect note: what this engine deliberately does not support.
pub const RUST_REGEX_DIALECT_NOTE: &str = "Rust regex crate dialect, not ICU. Look-around \
     (look-ahead/look-behind) and backreferences are rejected with a diagnostic.";

/// The visible flag legend, matching the flags this contract actually sets.
pub const RUST_REGEX_FLAGS_NOTE: &str = "Flags: i case-insensitive, m multi-line, s dot matches \
     newline, x ignore pattern whitespace, U swap greed, R CRLF mode. Unicode mode is always on.";

/// The visible replacement-template guidance.
pub const RUST_REGEX_REPLACEMENT_NOTE: &str = "Replacement: $1 or ${1} for numbered groups, \
     $name or ${name} for named groups, $$ for a literal $. A group that did not participate \
     expands to empty text.";

/// Longest accepted pattern source, in UTF-8 bytes.
pub const MAX_PATTERN_BYTES: usize = 16 * 1_024;
/// Approximate compiled-program size limit handed to `RegexBuilder::size_limit`.
pub const PATTERN_PROGRAM_SIZE_LIMIT: usize = 1 << 20;
/// Longest accepted test text, in UTF-8 bytes.
pub const MAX_TEXT_BYTES: usize = 1 << 20;
/// Longest accepted replacement template, in UTF-8 bytes.
pub const MAX_REPLACEMENT_BYTES: usize = 256 * 1_024;
/// Largest number of matches reported for one operation.
pub const MAX_MATCHES: usize = 10_000;
/// Largest total number of capture groups across all reported matches.
pub const MAX_CAPTURE_SLOTS: usize = 50_000;
/// Largest replacement preview, in UTF-8 bytes.
pub const MAX_REPLACEMENT_OUTPUT_BYTES: usize = 2 << 20;

/// The complete, strongly typed input for one Regex operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegexRequest {
    pub pattern: String,
    pub text: String,
    pub flags: RegexFlags,
    pub replacement: String,
}

impl RegexRequest {
    pub fn new(pattern: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            text: text.into(),
            flags: RegexFlags::default(),
            replacement: String::new(),
        }
    }
}

/// The flags this Utility can set on the Rust regex engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RegexFlags {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub ignore_whitespace: bool,
    pub swap_greed: bool,
    pub crlf: bool,
}

/// A half-open UTF-8 byte-offset range into the test text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

/// One capture group within a match. `value`/`range` are `None` when the group
/// did not participate in the match (an absent capture).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureInfo {
    pub index: usize,
    pub name: Option<String>,
    pub value: Option<String>,
    pub range: Option<TextRange>,
}

/// One match: its ordinal (0-based, in document order), text, byte range and
/// every capture group including group 0 for the whole match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchInfo {
    pub ordinal: usize,
    pub value: String,
    pub range: TextRange,
    pub captures: Vec<CaptureInfo>,
}

/// The typed outcome of one Regex operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegexEvaluation {
    /// Empty pattern and empty text are neutral: no result, no diagnostics.
    Empty,
    Valid {
        matches: Vec<MatchInfo>,
        /// The full-text preview of applying the replacement template.
        replacement: Option<String>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl RegexEvaluation {
    pub fn matches(&self) -> &[MatchInfo] {
        match self {
            RegexEvaluation::Valid { matches, .. } => matches,
            _ => &[],
        }
    }

    pub fn replacement(&self) -> Option<&str> {
        match self {
            RegexEvaluation::Valid { replacement, .. } => replacement.as_deref(),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            RegexEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, RegexEvaluation::Valid { .. })
    }
}

/// The bounded evaluation policy. Every field is a named, recorded boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegexLimits {
    pub max_pattern_bytes: usize,
    pub pattern_program_size: usize,
    pub max_text_bytes: usize,
    pub max_replacement_bytes: usize,
    pub max_matches: usize,
    pub max_captures: usize,
    pub max_replacement_output_bytes: usize,
}

impl Default for RegexLimits {
    fn default() -> Self {
        Self {
            max_pattern_bytes: MAX_PATTERN_BYTES,
            pattern_program_size: PATTERN_PROGRAM_SIZE_LIMIT,
            max_text_bytes: MAX_TEXT_BYTES,
            max_replacement_bytes: MAX_REPLACEMENT_BYTES,
            max_matches: MAX_MATCHES,
            max_captures: MAX_CAPTURE_SLOTS,
            max_replacement_output_bytes: MAX_REPLACEMENT_OUTPUT_BYTES,
        }
    }
}

/// Captured state of one settled valid Regex operation. It stores the exact
/// matches and replacement preview, so restore never reruns the engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegexSnapshot {
    pub request: RegexRequest,
    pub matches: Vec<MatchInfo>,
    pub replacement: String,
}

impl RegexSnapshot {
    pub fn restore(&self) -> (&RegexRequest, &[MatchInfo], &str) {
        (&self.request, &self.matches, &self.replacement)
    }
}

/// Evaluates with the production [`RegexLimits`].
pub fn evaluate(request: &RegexRequest) -> RegexEvaluation {
    evaluate_with_limits(request, &RegexLimits::default())
}

/// Evaluates with an explicit limits policy.
///
/// Refusal is total: over-limit pattern, text, template, match count, capture
/// count or replacement output produces [`RegexEvaluation::Invalid`] with no
/// partial success.
pub fn evaluate_with_limits(request: &RegexRequest, limits: &RegexLimits) -> RegexEvaluation {
    evaluate_with_cancellation(request, limits, || false).expect("non-cancellable evaluation")
}

/// Evaluates with stage-level cancellation. `None` means the result is obsolete
/// and must never be shown or recorded. The callback runs between operations
/// controlled by this Utility; it cannot interrupt one `regex` engine call.
pub fn evaluate_with_cancellation(
    request: &RegexRequest,
    limits: &RegexLimits,
    cancelled: impl Fn() -> bool,
) -> Option<RegexEvaluation> {
    if cancelled() {
        return None;
    }
    if request.pattern.is_empty() {
        if request.text.is_empty() {
            return Some(RegexEvaluation::Empty);
        }
        return Some(RegexEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(
                "Enter a pattern to test against the text.",
            )],
        });
    }
    if request.pattern.len() > limits.max_pattern_bytes {
        return Some(RegexEvaluation::Invalid {
            diagnostics: vec![too_large(format!(
                "The pattern exceeds the {}-byte pattern limit.",
                limits.max_pattern_bytes
            ))],
        });
    }
    if request.text.len() > limits.max_text_bytes {
        return Some(RegexEvaluation::Invalid {
            diagnostics: vec![too_large(format!(
                "The test text exceeds the {}-byte input limit.",
                limits.max_text_bytes
            ))],
        });
    }
    if request.replacement.len() > limits.max_replacement_bytes {
        return Some(RegexEvaluation::Invalid {
            diagnostics: vec![too_large(format!(
                "The replacement template exceeds the {}-byte limit.",
                limits.max_replacement_bytes
            ))],
        });
    }

    let regex = match build_regex(request, limits) {
        Ok(regex) => regex,
        Err(diagnostic) => {
            return Some(RegexEvaluation::Invalid {
                diagnostics: vec![diagnostic],
            })
        }
    };
    if cancelled() {
        return None;
    }

    let names: Vec<Option<String>> = regex
        .capture_names()
        .map(|name| name.map(str::to_owned))
        .collect();
    let captures_per_match = regex.captures_len();

    let mut matches: Vec<MatchInfo> = Vec::new();
    let mut replacement_output = String::new();
    let mut last_end = 0usize;
    let mut capture_slots = 0usize;

    let mut capture_iter = regex.captures_iter(&request.text);
    loop {
        if cancelled() {
            return None;
        }
        let Some(captures) = capture_iter.next() else {
            break;
        };
        if cancelled() {
            return None;
        }
        if matches.len() >= limits.max_matches {
            return Some(RegexEvaluation::Invalid {
                diagnostics: vec![too_large(format!(
                    "The match count exceeds the limit of {} matches.",
                    limits.max_matches
                ))],
            });
        }
        let prospective_slots = capture_slots.saturating_add(captures_per_match);
        if prospective_slots > limits.max_captures {
            return Some(RegexEvaluation::Invalid {
                diagnostics: vec![too_large(format!(
                    "The capture count exceeds the limit of {} capture slots.",
                    limits.max_captures
                ))],
            });
        }
        capture_slots = prospective_slots;

        // `captures_iter` already advances past a zero-length match, so this
        // loop always terminates; group 0 is guaranteed to participate.
        let whole = captures
            .get(0)
            .expect("capture group 0 always participates in a match");
        replacement_output.push_str(&request.text[last_end..whole.start()]);
        if cancelled() {
            return None;
        }
        captures.expand(&request.replacement, &mut replacement_output);
        if cancelled() {
            return None;
        }
        last_end = whole.end();
        if replacement_output.len() > limits.max_replacement_output_bytes {
            return Some(RegexEvaluation::Invalid {
                diagnostics: vec![too_large(format!(
                    "The replacement preview exceeds the {}-byte output limit.",
                    limits.max_replacement_output_bytes
                ))],
            });
        }

        matches.push(match_info(matches.len(), &captures, &names, &cancelled)?);
    }

    if cancelled() {
        return None;
    }
    replacement_output.push_str(&request.text[last_end..]);
    if replacement_output.len() > limits.max_replacement_output_bytes {
        return Some(RegexEvaluation::Invalid {
            diagnostics: vec![too_large(format!(
                "The replacement preview exceeds the {}-byte output limit.",
                limits.max_replacement_output_bytes
            ))],
        });
    }

    if cancelled() {
        return None;
    }
    Some(RegexEvaluation::Valid {
        matches,
        replacement: Some(replacement_output),
    })
}

fn build_regex(request: &RegexRequest, limits: &RegexLimits) -> Result<regex::Regex, Diagnostic> {
    let mut builder = RegexBuilder::new(&request.pattern);
    builder
        .case_insensitive(request.flags.case_insensitive)
        .multi_line(request.flags.multi_line)
        .dot_matches_new_line(request.flags.dot_matches_new_line)
        .ignore_whitespace(request.flags.ignore_whitespace)
        .swap_greed(request.flags.swap_greed)
        .crlf(request.flags.crlf)
        .size_limit(limits.pattern_program_size);
    builder.build().map_err(|error| pattern_diagnostic(&error))
}

/// Turns a compile failure into a precise diagnostic, naming the dialect
/// limitation for look-around and backreferences instead of translating them.
fn pattern_diagnostic(error: &regex::Error) -> Diagnostic {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("look-around")
        || lower.contains("look-ahead")
        || lower.contains("look-behind")
    {
        Diagnostic::error(format!(
            "This dialect does not support look-around (look-ahead or look-behind); there is no \
             ICU emulation. {message}"
        ))
    } else if lower.contains("backreference") {
        Diagnostic::error(format!(
            "This dialect does not support backreferences; there is no ICU emulation. {message}"
        ))
    } else if lower.contains("size limit") {
        Diagnostic::error(format!(
            "The compiled pattern exceeds the configured program-size limit. {message}"
        ))
    } else {
        Diagnostic::error(format!(
            "The pattern is not valid Rust regex syntax. {message}"
        ))
    }
}

fn too_large(message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(format!(
        "{} No partial result was published and the input was kept.",
        message.into()
    ))
}

fn match_info(
    ordinal: usize,
    captures: &regex::Captures<'_>,
    names: &[Option<String>],
    cancelled: &impl Fn() -> bool,
) -> Option<MatchInfo> {
    let whole = captures
        .get(0)
        .expect("capture group 0 always participates in a match");
    let mut groups = Vec::with_capacity(captures.len());
    for index in 0..captures.len() {
        if cancelled() {
            return None;
        }
        let matched = captures.get(index);
        groups.push(CaptureInfo {
            index,
            name: names.get(index).cloned().flatten(),
            value: matched.map(|matched| matched.as_str().to_owned()),
            range: matched.map(|matched| TextRange {
                start: matched.start(),
                end: matched.end(),
            }),
        });
    }
    Some(MatchInfo {
        ordinal,
        value: whole.as_str().to_owned(),
        range: TextRange {
            start: whole.start(),
            end: whole.end(),
        },
        captures: groups,
    })
}

/// The Regex Utility's identity for the shared [`Utility`] trait.
pub struct Regex;

impl Utility for Regex {
    type Request = RegexRequest;
    type Evaluation = RegexEvaluation;
    type Snapshot = RegexSnapshot;

    const ID: &'static str = REGEX_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = REGEX_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> RegexEvaluation {
        RegexEvaluation::Empty
    }

    fn evaluate(request: &RegexRequest) -> RegexEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &RegexEvaluation) -> bool {
        matches!(evaluation, RegexEvaluation::Empty)
    }

    fn snapshot(request: &RegexRequest, evaluation: &RegexEvaluation) -> Option<RegexSnapshot> {
        match evaluation {
            RegexEvaluation::Valid {
                matches,
                replacement,
            } => Some(RegexSnapshot {
                request: request.clone(),
                matches: matches.clone(),
                replacement: replacement.clone().unwrap_or_default(),
            }),
            _ => None,
        }
    }

    fn restore(snapshot: &RegexSnapshot) -> (RegexRequest, RegexEvaluation) {
        (
            snapshot.request.clone(),
            RegexEvaluation::Valid {
                matches: snapshot.matches.clone(),
                replacement: Some(snapshot.replacement.clone()),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(pattern: &str, text: &str) -> RegexRequest {
        RegexRequest::new(pattern, text)
    }

    fn evaluate_pattern(pattern: &str, text: &str) -> RegexEvaluation {
        evaluate(&request(pattern, text))
    }

    fn ranges(evaluation: &RegexEvaluation) -> Vec<(usize, usize)> {
        evaluation
            .matches()
            .iter()
            .map(|matched| (matched.range.start, matched.range.end))
            .collect()
    }

    #[test]
    fn documented_limits_are_the_default_policy() {
        let limits = RegexLimits::default();
        assert_eq!(limits.max_pattern_bytes, MAX_PATTERN_BYTES);
        assert_eq!(limits.pattern_program_size, PATTERN_PROGRAM_SIZE_LIMIT);
        assert_eq!(limits.max_text_bytes, MAX_TEXT_BYTES);
        assert_eq!(limits.max_replacement_bytes, MAX_REPLACEMENT_BYTES);
        assert_eq!(limits.max_matches, MAX_MATCHES);
        assert_eq!(limits.max_captures, MAX_CAPTURE_SLOTS);
        assert_eq!(
            limits.max_replacement_output_bytes,
            MAX_REPLACEMENT_OUTPUT_BYTES
        );
        assert_eq!(MAX_PATTERN_BYTES, 16_384);
        assert_eq!(MAX_TEXT_BYTES, 1_048_576);
        assert_eq!(MAX_REPLACEMENT_BYTES, 262_144);
        assert_eq!(MAX_MATCHES, 10_000);
        assert_eq!(MAX_CAPTURE_SLOTS, 50_000);
        assert_eq!(MAX_REPLACEMENT_OUTPUT_BYTES, 2_097_152);
    }

    #[test]
    fn demanding_real_engine_capture_and_replacement_work_completes() {
        // Exercises the actual regex engine and capture/replacement loop near
        // the match budget without asserting a machine-dependent duration.
        let text = format!("{} ", "a".repeat(30)).repeat(8_192);
        let mut request = request(r"(?P<word>\w+)", &text);
        request.replacement = "[$word]".to_owned();
        let evaluation = evaluate(&request);
        assert!(evaluation.is_valid_operation());
        assert_eq!(evaluation.matches().len(), 8_192);
        assert_eq!(evaluation.matches()[8_191].captures.len(), 2);
        assert_eq!(
            evaluation
                .replacement()
                .expect("complete replacement")
                .len(),
            text.len() + 2 * 8_192
        );
    }

    #[test]
    fn cancellation_discards_partial_engine_work() {
        use std::cell::Cell;

        let checks = Cell::new(0);
        let request = request(r"(\w+)", &"word ".repeat(1_000));
        let result = evaluate_with_cancellation(&request, &RegexLimits::default(), || {
            checks.set(checks.get() + 1);
            checks.get() > 40
        });
        assert!(
            result.is_none(),
            "cancelled work cannot publish partial matches"
        );
        assert!(checks.get() > 40);
    }

    #[test]
    fn supported_syntax_matches_and_captures() {
        let evaluation =
            evaluate_pattern(r"(?P<first>\w+),\s*(?P<last>\w+)", "Doe, John; Smith, Jane");
        assert!(evaluation.is_valid_operation());
        assert_eq!(evaluation.matches().len(), 2);

        let first = &evaluation.matches()[0];
        assert_eq!(first.ordinal, 0);
        assert_eq!(first.value, "Doe, John");
        assert_eq!(first.range, TextRange { start: 0, end: 9 });
        assert_eq!(first.captures.len(), 3);
        assert_eq!(first.captures[0].value.as_deref(), Some("Doe, John"));
        assert_eq!(first.captures[1].name.as_deref(), Some("first"));
        assert_eq!(first.captures[1].value.as_deref(), Some("Doe"));
        assert_eq!(first.captures[2].name.as_deref(), Some("last"));
        assert_eq!(first.captures[2].value.as_deref(), Some("John"));
        assert_eq!(
            first.captures[2].range,
            Some(TextRange { start: 5, end: 9 })
        );
    }

    #[test]
    fn all_supported_flags_change_matching() {
        // `i` case-insensitivity.
        let mut insensitive = request("abc", "ABC");
        insensitive.flags.case_insensitive = true;
        assert_eq!(evaluate(&insensitive).matches().len(), 1);

        // `m` multi-line anchors.
        let mut multiline = request("^b", "a\nb");
        multiline.flags.multi_line = true;
        assert_eq!(evaluate(&multiline).matches().len(), 1);

        // `s` dot matches newline.
        let mut dot = request("a.b", "a\nb");
        dot.flags.dot_matches_new_line = true;
        assert_eq!(evaluate(&dot).matches().len(), 1);

        // `x` ignores pattern whitespace.
        let mut whitespace = request("a b", "ab");
        whitespace.flags.ignore_whitespace = true;
        assert_eq!(evaluate(&whitespace).matches().len(), 1);

        // `U` swaps greedy and lazy meaning.
        let mut swapped = request("a+?", "aaa");
        swapped.flags.swap_greed = true;
        let evaluation = evaluate(&swapped);
        assert_eq!(evaluation.matches()[0].value, "aaa");

        // `R` treats CRLF as the line terminator.
        let mut crlf = request("^b", "a\r\nb");
        crlf.flags.crlf = true;
        crlf.flags.multi_line = true;
        assert_eq!(evaluate(&crlf).matches().len(), 1);
    }

    #[test]
    fn look_ahead_look_behind_and_backreferences_are_diagnosed() {
        for pattern in [r"a(?=b)", r"a(?!b)", r"(?<=a)b", r"(?<!a)b"] {
            let evaluation = evaluate_pattern(pattern, "ab");
            assert!(
                !evaluation.is_valid_operation(),
                "{pattern} must be rejected"
            );
            assert!(evaluation.matches().is_empty());
            let message = evaluation.diagnostics()[0].message.to_ascii_lowercase();
            assert!(
                message.contains("look-around") && message.contains("no icu emulation"),
                "got: {}",
                evaluation.diagnostics()[0].message
            );
        }

        let backreference = evaluate_pattern(r"(a)\1", "aa");
        assert!(!backreference.is_valid_operation());
        let message = backreference.diagnostics()[0].message.to_ascii_lowercase();
        assert!(
            message.contains("backreference") && message.contains("no icu emulation"),
            "got: {}",
            backreference.diagnostics()[0].message
        );
    }

    #[test]
    fn invalid_syntax_is_diagnosed_without_a_result() {
        let evaluation = evaluate_pattern(r"(unclosed", "x");
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.replacement().is_none());
        assert!(evaluation.diagnostics()[0]
            .message
            .contains("not valid Rust regex"));
    }

    #[test]
    fn unicode_offsets_are_utf8_byte_offsets() {
        // "aé😀": a=1 byte, é=2 bytes, 😀=4 bytes.
        let evaluation = evaluate_pattern("[é😀]", "aé😀");
        assert_eq!(ranges(&evaluation), vec![(1, 3), (3, 7)]);
        assert_eq!(evaluation.matches()[0].value, "é");
        assert_eq!(evaluation.matches()[1].value, "😀");

        let grouped = evaluate_pattern(r"(😀)(\w+)", "x😀yz");
        assert_eq!(
            grouped.matches()[0].captures[1].range,
            Some(TextRange { start: 1, end: 5 })
        );
        assert_eq!(
            grouped.matches()[0].captures[2].range,
            Some(TextRange { start: 5, end: 7 })
        );
    }

    #[test]
    fn absent_captures_report_none() {
        let evaluation = evaluate_pattern(r"(a)(b)?", "a");
        let matched = &evaluation.matches()[0];
        assert_eq!(matched.captures.len(), 3);
        assert_eq!(matched.captures[1].value.as_deref(), Some("a"));
        assert_eq!(matched.captures[2].value, None);
        assert_eq!(matched.captures[2].range, None);
    }

    #[test]
    fn zero_length_matches_terminate_and_are_reported() {
        // `a*` on "ba" yields an empty match at 0 and "a" at 1..2. The crate
        // skips the empty match immediately after a non-empty match that ends
        // at the haystack end; crucially the loop still advances rather than
        // spinning forever.
        let evaluation = evaluate_pattern("a*", "ba");
        assert_eq!(ranges(&evaluation), vec![(0, 0), (1, 2)]);

        // `x*` never matches a character, so every reported match is empty.
        let zeros = evaluate_pattern("x*", "ab");
        assert!(zeros.is_valid_operation());
        assert!(zeros
            .matches()
            .iter()
            .all(|matched| matched.value.is_empty()));
        assert_eq!(ranges(&zeros), vec![(0, 0), (1, 1), (2, 2)]);

        let anchored = evaluate_pattern("", "abc");
        // An empty pattern is only neutral with empty text; with text it is
        // invalid rather than an accidental every-position match.
        assert!(!anchored.is_valid_operation());
    }

    #[test]
    fn replacement_preview_expands_numbered_and_named_references() {
        let mut numbered = request(r"(\w+),\s*(\w+)", "Doe, John");
        numbered.replacement = "$2 $1".to_owned();
        assert_eq!(evaluate(&numbered).replacement(), Some("John Doe"));

        let mut named = request(r"(?P<last>\w+), (?P<first>\w+)", "Doe, John");
        named.replacement = "${first} ${last}".to_owned();
        assert_eq!(evaluate(&named).replacement(), Some("John Doe"));

        let mut literal = request("a", "a");
        literal.replacement = "$$1".to_owned();
        assert_eq!(evaluate(&literal).replacement(), Some("$1"));

        // Deleting matches is a valid empty replacement.
        let mut removed = request(r"\s+", "a b c");
        removed.replacement = String::new();
        assert_eq!(evaluate(&removed).replacement(), Some("abc"));

        // An absent group reference expands to empty text.
        let mut absent = request(r"(a)(b)?", "a");
        absent.replacement = "[$1$2]".to_owned();
        assert_eq!(evaluate(&absent).replacement(), Some("[a]"));
    }

    #[test]
    fn over_limit_input_is_refused_without_partial_output() {
        let limits = RegexLimits {
            max_pattern_bytes: 4,
            ..RegexLimits::default()
        };
        let evaluation = evaluate_with_limits(&request("abcde", "abcde"), &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.matches().is_empty());
        assert!(evaluation.diagnostics()[0]
            .message
            .contains("pattern limit"));

        let limits = RegexLimits {
            max_text_bytes: 2,
            ..RegexLimits::default()
        };
        let evaluation = evaluate_with_limits(&request("a", "abc"), &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.diagnostics()[0].message.contains("input limit"));

        let limits = RegexLimits {
            max_replacement_bytes: 1,
            ..RegexLimits::default()
        };
        let mut over_template = request("a", "a");
        over_template.replacement = "xx".to_owned();
        let evaluation = evaluate_with_limits(&over_template, &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.diagnostics()[0]
            .message
            .contains("replacement template"));
    }

    #[test]
    fn match_and_capture_limits_refuse_rather_than_truncate() {
        let limits = RegexLimits {
            max_matches: 2,
            ..RegexLimits::default()
        };
        let evaluation = evaluate_with_limits(&request("a", "aaa"), &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.matches().is_empty());
        assert!(evaluation.diagnostics()[0].message.contains("match count"));

        let limits = RegexLimits {
            max_captures: 2,
            ..RegexLimits::default()
        };
        let evaluation = evaluate_with_limits(&request("(a)(b)", "abab"), &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.diagnostics()[0]
            .message
            .contains("capture count"));

        // Exactly at the limit is still accepted.
        let limits = RegexLimits {
            max_matches: 3,
            max_captures: 3,
            ..RegexLimits::default()
        };
        let evaluation = evaluate_with_limits(&request("a", "aaa"), &limits);
        assert!(evaluation.is_valid_operation());
        assert_eq!(evaluation.matches().len(), 3);
    }

    #[test]
    fn replacement_output_limit_refuses_rather_than_truncate() {
        let limits = RegexLimits {
            max_replacement_output_bytes: 4,
            ..RegexLimits::default()
        };
        let mut request = request("a", "aaaaa");
        request.replacement = "b".to_owned();
        let evaluation = evaluate_with_limits(&request, &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.replacement().is_none());
        assert!(evaluation.diagnostics()[0]
            .message
            .contains("replacement preview"));
    }

    #[test]
    fn compiled_program_size_limit_is_diagnosed() {
        let limits = RegexLimits {
            // `\w` compiles to a large Unicode program.
            pattern_program_size: 1_000,
            ..RegexLimits::default()
        };
        let evaluation = evaluate_with_limits(&request(r"\w", "word"), &limits);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.diagnostics()[0]
            .message
            .contains("program-size limit"));
    }

    #[test]
    fn empty_input_is_neutral_and_never_snapshots() {
        let request = request("", "");
        let evaluation = evaluate(&request);
        assert_eq!(evaluation, RegexEvaluation::Empty);
        assert!(<Regex as Utility>::is_neutral(&evaluation));
        assert!(<Regex as Utility>::snapshot(&request, &evaluation).is_none());
    }

    #[test]
    fn invalid_and_neutral_operations_never_snapshot() {
        for request in [
            request("", "text without a pattern"),
            request(r"a(?=b)", "ab"),
            request(r"(unclosed", "x"),
        ] {
            let evaluation = evaluate(&request);
            assert!(!evaluation.is_valid_operation());
            assert!(<Regex as Utility>::snapshot(&request, &evaluation).is_none());
        }
    }

    #[test]
    fn valid_zero_match_operation_still_snapshots() {
        let request = request("z", "abc");
        let evaluation = evaluate(&request);
        assert!(evaluation.is_valid_operation());
        assert!(evaluation.matches().is_empty());
        let snapshot = <Regex as Utility>::snapshot(&request, &evaluation).expect("snapshot");
        assert_eq!(snapshot.replacement, "abc");
    }

    #[test]
    fn snapshot_round_trips_exactly_without_reevaluation() {
        let mut request = request(r"(?P<word>\w+)", "one two");
        request.flags.case_insensitive = true;
        request.replacement = "<$word>".to_owned();
        let evaluation = evaluate(&request);
        let snapshot =
            <Regex as Utility>::snapshot(&request, &evaluation).expect("valid operation snapshots");

        let encoded = serde_json::to_string(&snapshot).expect("snapshot serializes");
        let decoded: RegexSnapshot = serde_json::from_str(&encoded).expect("snapshot deserializes");
        assert_eq!(decoded, snapshot);

        let (restored_request, restored_evaluation) = <Regex as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation, evaluation);
    }

    #[test]
    fn session_rejects_stale_revisions_and_snapshots_once() {
        use crate::session::{Session, SubmitOutcome};

        let mut session = Session::<Regex>::new();
        let SubmitOutcome::Scheduled(first) = session.submit(request("a", "aaa")) else {
            panic!("expected a scheduled revision");
        };
        let SubmitOutcome::Scheduled(second) = session.submit(request("b", "bbb")) else {
            panic!("expected a scheduled revision");
        };
        assert!(
            session.resolve(first).is_none(),
            "stale revision must not settle"
        );
        let settled = session.resolve(second).expect("current revision settles");
        assert_eq!(settled.matches().len(), 3);

        let snapshot = session.take_snapshot().expect("settled snapshot");
        assert_eq!(snapshot.matches.len(), 3);
        assert!(
            session.take_snapshot().is_none(),
            "an operation records at most once"
        );
    }
}
