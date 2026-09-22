//! The Case Conversion Utility's GPUI-independent request/result/snapshot
//! contract.
//!
//! One deterministic, locale-independent segmentation policy is shared by all
//! nine styles. Segmentation walks complete Unicode extended grapheme clusters,
//! so combining marks (including spacing marks and variation selectors) and
//! complex emoji sequences are never split. Word boundaries follow the Swift
//! baseline: acronym runs keep their internal capitals and break before the
//! final capital of a lower-case continuation (`HTTPServer` → `HTTP`,`Server`),
//! letter/digit transitions break in both directions (`version2Value` →
//! `version`,`2`,`Value`), and punctuation, mixed separators and non-emoji
//! symbols separate words while emoji are retained as their own word.
//!
//! Casing uses the Unicode default (locale-invariant) mappings, matching the
//! baseline's `en_US_POSIX` behavior rather than the host locale.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Case Conversion Utility.
pub const CASE_CONVERSION_UTILITY_ID: &str = "case-conversion";

/// Schema version of [`CaseConversionSnapshot`].
pub const CASE_CONVERSION_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Maximum accepted input length in UTF-16 code units, matching the baseline.
pub const MAXIMUM_INPUT_UTF16_LENGTH: usize = 1_048_576;

const ZWJ: char = '\u{200D}';

/// The nine conversion styles. Serialization uses stable Rust names and is
/// independent of any Swift representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseConversionStyle {
    Camel,
    Pascal,
    Snake,
    ScreamingSnake,
    Kebab,
    Title,
    Sentence,
    Lower,
    Upper,
}

impl CaseConversionStyle {
    /// Declaration order, used for stable control order and focus indexing.
    pub const ALL: [CaseConversionStyle; 9] = [
        CaseConversionStyle::Camel,
        CaseConversionStyle::Pascal,
        CaseConversionStyle::Snake,
        CaseConversionStyle::ScreamingSnake,
        CaseConversionStyle::Kebab,
        CaseConversionStyle::Title,
        CaseConversionStyle::Sentence,
        CaseConversionStyle::Lower,
        CaseConversionStyle::Upper,
    ];

    /// Stable index in [`CaseConversionStyle::ALL`].
    pub const fn index(self) -> usize {
        match self {
            CaseConversionStyle::Camel => 0,
            CaseConversionStyle::Pascal => 1,
            CaseConversionStyle::Snake => 2,
            CaseConversionStyle::ScreamingSnake => 3,
            CaseConversionStyle::Kebab => 4,
            CaseConversionStyle::Title => 5,
            CaseConversionStyle::Sentence => 6,
            CaseConversionStyle::Lower => 7,
            CaseConversionStyle::Upper => 8,
        }
    }

    /// Human-readable label, matching the baseline's display names.
    pub const fn label(self) -> &'static str {
        match self {
            CaseConversionStyle::Camel => "camelCase",
            CaseConversionStyle::Pascal => "PascalCase",
            CaseConversionStyle::Snake => "snake_case",
            CaseConversionStyle::ScreamingSnake => "SCREAMING_SNAKE_CASE",
            CaseConversionStyle::Kebab => "kebab-case",
            CaseConversionStyle::Title => "Title Case",
            CaseConversionStyle::Sentence => "sentence case",
            CaseConversionStyle::Lower => "lowercase",
            CaseConversionStyle::Upper => "UPPERCASE",
        }
    }
}

/// The complete, strongly typed input for one evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseConversionRequest {
    pub input: String,
    pub style: CaseConversionStyle,
}

impl CaseConversionRequest {
    pub fn new(input: impl Into<String>, style: CaseConversionStyle) -> Self {
        Self {
            input: input.into(),
            style,
        }
    }
}

/// The typed outcome shown to the user. `Valid` carries the detected words so
/// the workspace can show them without re-segmenting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseConversionEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
        words: Vec<String>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl CaseConversionEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            CaseConversionEvaluation::Valid { output, .. } => Some(output),
            _ => None,
        }
    }

    /// The detected words for a settled valid operation, otherwise empty.
    pub fn words(&self) -> &[String] {
        match self {
            CaseConversionEvaluation::Valid { words, .. } => words,
            _ => &[],
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            CaseConversionEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, CaseConversionEvaluation::Valid { .. })
    }
}

/// The Utility-owned payload persisted in History.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseConversionSnapshot {
    pub request: CaseConversionRequest,
    pub output: String,
    pub words: Vec<String>,
}

impl CaseConversionSnapshot {
    pub fn restore(&self) -> (&CaseConversionRequest, &str) {
        (&self.request, &self.output)
    }
}

/// Segments and renders `request`, or reports why it is not a valid operation.
pub fn evaluate(request: &CaseConversionRequest) -> CaseConversionEvaluation {
    if request.input.is_empty() {
        return CaseConversionEvaluation::Empty;
    }
    if request.input.encode_utf16().count() > MAXIMUM_INPUT_UTF16_LENGTH {
        return CaseConversionEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(
                "Input exceeds the 1,048,576 UTF-16-unit conversion limit.",
            )],
        };
    }
    let words = segment(&request.input);
    let output = render(&words, request.style);
    CaseConversionEvaluation::Valid { output, words }
}

/// The one segmentation policy shared by every style.
pub fn segment(input: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut kinds: Vec<CharacterKind> = Vec::new();

    for grapheme in graphemes(input) {
        match classify(grapheme) {
            CharacterKind::Separator => {
                flush(&mut words, &mut current, &mut kinds, false);
            }
            CharacterKind::Symbol => {
                flush(&mut words, &mut current, &mut kinds, false);
                words.push(grapheme.to_owned());
            }
            kind => {
                if let Some(&previous) = kinds.last() {
                    let before_previous = kinds
                        .len()
                        .checked_sub(2)
                        .and_then(|index| kinds.get(index))
                        .copied();
                    if (matches!(
                        previous,
                        CharacterKind::Lowercase | CharacterKind::UncasedLetter
                    ) && kind == CharacterKind::Uppercase)
                        || (previous == CharacterKind::Digit && kind != CharacterKind::Digit)
                        || (previous != CharacterKind::Digit && kind == CharacterKind::Digit)
                    {
                        flush(&mut words, &mut current, &mut kinds, false);
                    } else if previous == CharacterKind::Uppercase
                        && kind == CharacterKind::Lowercase
                        && before_previous == Some(CharacterKind::Uppercase)
                    {
                        flush(&mut words, &mut current, &mut kinds, true);
                    }
                }
                current.push(grapheme);
                kinds.push(kind);
            }
        }
    }
    flush(&mut words, &mut current, &mut kinds, false);
    words
}

/// Emits the pending word, optionally retaining the last cluster so an acronym
/// can continue into a following capital (`HTTPServer` → `HTTP`,`Server`).
fn flush(
    words: &mut Vec<String>,
    current: &mut Vec<&str>,
    kinds: &mut Vec<CharacterKind>,
    keeping_last: bool,
) {
    let emitted = if keeping_last {
        current.len().saturating_sub(1)
    } else {
        current.len()
    };
    if emitted > 0 {
        words.push(current[..emitted].concat());
    }
    if keeping_last {
        let kept = current.last().copied();
        let kept_kind = kinds.last().copied();
        current.clear();
        kinds.clear();
        if let Some(cluster) = kept {
            current.push(cluster);
            kinds.push(kept_kind.expect("a retained cluster always has a kind"));
        }
    } else {
        current.clear();
        kinds.clear();
    }
}

fn render(words: &[String], style: CaseConversionStyle) -> String {
    let lower: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
    let capitalized: Vec<String> = lower.iter().map(|word| capitalize(word)).collect();
    match style {
        CaseConversionStyle::Camel => {
            let mut output = String::new();
            if let Some(first) = lower.first() {
                output.push_str(first);
            }
            for word in capitalized.iter().skip(1) {
                output.push_str(word);
            }
            output
        }
        CaseConversionStyle::Pascal => capitalized.concat(),
        CaseConversionStyle::Snake => lower.join("_"),
        CaseConversionStyle::ScreamingSnake => uppercase_words(words).join("_"),
        CaseConversionStyle::Kebab => lower.join("-"),
        CaseConversionStyle::Title => capitalized.join(" "),
        CaseConversionStyle::Sentence => match lower.split_first() {
            None => String::new(),
            Some((first, rest)) => {
                let mut output = capitalize(first);
                for word in rest {
                    output.push(' ');
                    output.push_str(word);
                }
                output
            }
        },
        CaseConversionStyle::Lower => lower.join(" "),
        CaseConversionStyle::Upper => uppercase_words(words).join(" "),
    }
}

fn uppercase_words(words: &[String]) -> Vec<String> {
    words.iter().map(|word| word.to_uppercase()).collect()
}

/// Upper-cases the first grapheme cluster only, so a leading combining mark or
/// emoji cluster is not split.
fn capitalize(word: &str) -> String {
    let clusters = graphemes(word);
    let Some(first) = clusters.first() else {
        return String::new();
    };
    let mut output = first.to_uppercase();
    output.push_str(&word[first.len()..]);
    output
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharacterKind {
    Uppercase,
    Lowercase,
    UncasedLetter,
    Digit,
    Separator,
    Symbol,
}

/// Classifies one extended grapheme cluster with the same priority as the
/// baseline: decimal digit, then upper, then lower, then any letter; otherwise
/// an emoji-presentation cluster is a symbol and everything else separates.
fn classify(grapheme: &str) -> CharacterKind {
    let mut has_digit = false;
    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_letter = false;
    let mut has_emoji = false;
    for character in grapheme.chars() {
        has_digit |= in_ranges(DECIMAL_DIGIT, character);
        has_upper |= character.is_uppercase();
        has_lower |= character.is_lowercase();
        has_letter |= character.is_alphabetic();
        has_emoji |= in_ranges(EMOJI_PRESENTATION, character);
    }
    if has_digit {
        CharacterKind::Digit
    } else if has_upper {
        CharacterKind::Uppercase
    } else if has_lower {
        CharacterKind::Lowercase
    } else if has_letter {
        CharacterKind::UncasedLetter
    } else if has_emoji {
        CharacterKind::Symbol
    } else {
        CharacterKind::Separator
    }
}

fn is_control(character: char) -> bool {
    character == '\r' || character == '\n' || in_ranges(GRAPHEME_CONTROL, character)
}

fn is_extend(character: char) -> bool {
    in_ranges(GRAPHEME_EXTEND, character)
}

fn is_spacing_mark(character: char) -> bool {
    in_ranges(GRAPHEME_SPACING_MARK, character)
}

fn is_prepend(character: char) -> bool {
    in_ranges(GRAPHEME_PREPEND, character)
}

fn is_regional_indicator(character: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&character)
}

fn in_ranges(ranges: &[(u32, u32)], character: char) -> bool {
    let code = character as u32;
    ranges
        .binary_search_by(|&(start, end)| {
            if code < start {
                Ordering::Greater
            } else if code > end {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// Splits `text` at Unicode extended-grapheme-cluster boundaries.
///
/// This is a self-contained implementation of the relevant UAX #29 rules
/// (CR/LF, control, extend, ZWJ, spacing mark, prepend and regional-indicator
/// pairing). The ZWJ rule joins the cluster after any joiner rather than only
/// after an extended pictograph; that over-join never splits a real emoji
/// sequence and does not change word boundaries for normal text.
fn graphemes(text: &str) -> Vec<&str> {
    let mut clusters = Vec::new();
    let mut start = 0usize;
    let mut previous: Option<char> = None;
    let mut regional_run = 0usize;

    for (index, character) in text.char_indices() {
        if let Some(previous_character) = previous {
            let break_here = if previous_character == '\r' && character == '\n' {
                false
            } else if is_control(previous_character)
                || previous_character == '\r'
                || previous_character == '\n'
                || is_control(character)
                || character == '\r'
                || character == '\n'
            {
                true
            } else if is_extend(character)
                || character == ZWJ
                || is_spacing_mark(character)
                || is_prepend(previous_character)
                || previous_character == ZWJ
            {
                false
            } else {
                !(is_regional_indicator(previous_character)
                    && is_regional_indicator(character)
                    && regional_run % 2 == 1)
            };
            if break_here {
                clusters.push(&text[start..index]);
                start = index;
                regional_run = 0;
            }
        }
        if is_regional_indicator(character) {
            regional_run += 1;
        } else {
            regional_run = 0;
        }
        previous = Some(character);
    }
    if start < text.len() {
        clusters.push(&text[start..]);
    }
    clusters
}

/// The Case Conversion Utility's identity for the shared [`Utility`] trait.
pub struct CaseConversion;

impl Utility for CaseConversion {
    type Request = CaseConversionRequest;
    type Evaluation = CaseConversionEvaluation;
    type Snapshot = CaseConversionSnapshot;

    const ID: &'static str = CASE_CONVERSION_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = CASE_CONVERSION_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> CaseConversionEvaluation {
        CaseConversionEvaluation::Empty
    }

    fn evaluate(request: &CaseConversionRequest) -> CaseConversionEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &CaseConversionEvaluation) -> bool {
        matches!(evaluation, CaseConversionEvaluation::Empty)
    }

    fn snapshot(
        request: &CaseConversionRequest,
        evaluation: &CaseConversionEvaluation,
    ) -> Option<CaseConversionSnapshot> {
        match evaluation {
            CaseConversionEvaluation::Valid { output, words } => Some(CaseConversionSnapshot {
                request: request.clone(),
                output: output.clone(),
                words: words.clone(),
            }),
            _ => None,
        }
    }

    fn restore(
        snapshot: &CaseConversionSnapshot,
    ) -> (CaseConversionRequest, CaseConversionEvaluation) {
        (
            snapshot.request.clone(),
            CaseConversionEvaluation::Valid {
                output: snapshot.output.clone(),
                words: snapshot.words.clone(),
            },
        )
    }
}

// Generated from Unicode 16.0.0 UCD (GraphemeBreakProperty.txt, emoji-data.txt
// and the UnicodeData decimal-digit categories). Regenerate if the Unicode
// version changes; do not edit by hand.

/// Extended grapheme clusters (UAX #29 `Extend`, including Mn, Me, emoji
/// modifiers, variation selectors and tag characters).
const GRAPHEME_EXTEND: &[(u32, u32)] = &[
    (0x0300, 0x036F),
    (0x0483, 0x0489),
    (0x0591, 0x05BD),
    (0x05BF, 0x05BF),
    (0x05C1, 0x05C2),
    (0x05C4, 0x05C5),
    (0x05C7, 0x05C7),
    (0x0610, 0x061A),
    (0x064B, 0x065F),
    (0x0670, 0x0670),
    (0x06D6, 0x06DC),
    (0x06DF, 0x06E4),
    (0x06E7, 0x06E8),
    (0x06EA, 0x06ED),
    (0x0711, 0x0711),
    (0x0730, 0x074A),
    (0x07A6, 0x07B0),
    (0x07EB, 0x07F3),
    (0x07FD, 0x07FD),
    (0x0816, 0x0819),
    (0x081B, 0x0823),
    (0x0825, 0x0827),
    (0x0829, 0x082D),
    (0x0859, 0x085B),
    (0x0897, 0x089F),
    (0x08CA, 0x08E1),
    (0x08E3, 0x0902),
    (0x093A, 0x093A),
    (0x093C, 0x093C),
    (0x0941, 0x0948),
    (0x094D, 0x094D),
    (0x0951, 0x0957),
    (0x0962, 0x0963),
    (0x0981, 0x0981),
    (0x09BC, 0x09BC),
    (0x09BE, 0x09BE),
    (0x09C1, 0x09C4),
    (0x09CD, 0x09CD),
    (0x09D7, 0x09D7),
    (0x09E2, 0x09E3),
    (0x09FE, 0x09FE),
    (0x0A01, 0x0A02),
    (0x0A3C, 0x0A3C),
    (0x0A41, 0x0A42),
    (0x0A47, 0x0A48),
    (0x0A4B, 0x0A4D),
    (0x0A51, 0x0A51),
    (0x0A70, 0x0A71),
    (0x0A75, 0x0A75),
    (0x0A81, 0x0A82),
    (0x0ABC, 0x0ABC),
    (0x0AC1, 0x0AC5),
    (0x0AC7, 0x0AC8),
    (0x0ACD, 0x0ACD),
    (0x0AE2, 0x0AE3),
    (0x0AFA, 0x0AFF),
    (0x0B01, 0x0B01),
    (0x0B3C, 0x0B3C),
    (0x0B3E, 0x0B3F),
    (0x0B41, 0x0B44),
    (0x0B4D, 0x0B4D),
    (0x0B55, 0x0B57),
    (0x0B62, 0x0B63),
    (0x0B82, 0x0B82),
    (0x0BBE, 0x0BBE),
    (0x0BC0, 0x0BC0),
    (0x0BCD, 0x0BCD),
    (0x0BD7, 0x0BD7),
    (0x0C00, 0x0C00),
    (0x0C04, 0x0C04),
    (0x0C3C, 0x0C3C),
    (0x0C3E, 0x0C40),
    (0x0C46, 0x0C48),
    (0x0C4A, 0x0C4D),
    (0x0C55, 0x0C56),
    (0x0C62, 0x0C63),
    (0x0C81, 0x0C81),
    (0x0CBC, 0x0CBC),
    (0x0CBF, 0x0CC0),
    (0x0CC2, 0x0CC2),
    (0x0CC6, 0x0CC8),
    (0x0CCA, 0x0CCD),
    (0x0CD5, 0x0CD6),
    (0x0CE2, 0x0CE3),
    (0x0D00, 0x0D01),
    (0x0D3B, 0x0D3C),
    (0x0D3E, 0x0D3E),
    (0x0D41, 0x0D44),
    (0x0D4D, 0x0D4D),
    (0x0D57, 0x0D57),
    (0x0D62, 0x0D63),
    (0x0D81, 0x0D81),
    (0x0DCA, 0x0DCA),
    (0x0DCF, 0x0DCF),
    (0x0DD2, 0x0DD4),
    (0x0DD6, 0x0DD6),
    (0x0DDF, 0x0DDF),
    (0x0E31, 0x0E31),
    (0x0E34, 0x0E3A),
    (0x0E47, 0x0E4E),
    (0x0EB1, 0x0EB1),
    (0x0EB4, 0x0EBC),
    (0x0EC8, 0x0ECE),
    (0x0F18, 0x0F19),
    (0x0F35, 0x0F35),
    (0x0F37, 0x0F37),
    (0x0F39, 0x0F39),
    (0x0F71, 0x0F7E),
    (0x0F80, 0x0F84),
    (0x0F86, 0x0F87),
    (0x0F8D, 0x0F97),
    (0x0F99, 0x0FBC),
    (0x0FC6, 0x0FC6),
    (0x102D, 0x1030),
    (0x1032, 0x1037),
    (0x1039, 0x103A),
    (0x103D, 0x103E),
    (0x1058, 0x1059),
    (0x105E, 0x1060),
    (0x1071, 0x1074),
    (0x1082, 0x1082),
    (0x1085, 0x1086),
    (0x108D, 0x108D),
    (0x109D, 0x109D),
    (0x135D, 0x135F),
    (0x1712, 0x1715),
    (0x1732, 0x1734),
    (0x1752, 0x1753),
    (0x1772, 0x1773),
    (0x17B4, 0x17B5),
    (0x17B7, 0x17BD),
    (0x17C6, 0x17C6),
    (0x17C9, 0x17D3),
    (0x17DD, 0x17DD),
    (0x180B, 0x180D),
    (0x180F, 0x180F),
    (0x1885, 0x1886),
    (0x18A9, 0x18A9),
    (0x1920, 0x1922),
    (0x1927, 0x1928),
    (0x1932, 0x1932),
    (0x1939, 0x193B),
    (0x1A17, 0x1A18),
    (0x1A1B, 0x1A1B),
    (0x1A56, 0x1A56),
    (0x1A58, 0x1A5E),
    (0x1A60, 0x1A60),
    (0x1A62, 0x1A62),
    (0x1A65, 0x1A6C),
    (0x1A73, 0x1A7C),
    (0x1A7F, 0x1A7F),
    (0x1AB0, 0x1ACE),
    (0x1B00, 0x1B03),
    (0x1B34, 0x1B3D),
    (0x1B42, 0x1B44),
    (0x1B6B, 0x1B73),
    (0x1B80, 0x1B81),
    (0x1BA2, 0x1BA5),
    (0x1BA8, 0x1BAD),
    (0x1BE6, 0x1BE6),
    (0x1BE8, 0x1BE9),
    (0x1BED, 0x1BED),
    (0x1BEF, 0x1BF3),
    (0x1C2C, 0x1C33),
    (0x1C36, 0x1C37),
    (0x1CD0, 0x1CD2),
    (0x1CD4, 0x1CE0),
    (0x1CE2, 0x1CE8),
    (0x1CED, 0x1CED),
    (0x1CF4, 0x1CF4),
    (0x1CF8, 0x1CF9),
    (0x1DC0, 0x1DFF),
    (0x200C, 0x200C),
    (0x20D0, 0x20F0),
    (0x2CEF, 0x2CF1),
    (0x2D7F, 0x2D7F),
    (0x2DE0, 0x2DFF),
    (0x302A, 0x302F),
    (0x3099, 0x309A),
    (0xA66F, 0xA672),
    (0xA674, 0xA67D),
    (0xA69E, 0xA69F),
    (0xA6F0, 0xA6F1),
    (0xA802, 0xA802),
    (0xA806, 0xA806),
    (0xA80B, 0xA80B),
    (0xA825, 0xA826),
    (0xA82C, 0xA82C),
    (0xA8C4, 0xA8C5),
    (0xA8E0, 0xA8F1),
    (0xA8FF, 0xA8FF),
    (0xA926, 0xA92D),
    (0xA947, 0xA951),
    (0xA953, 0xA953),
    (0xA980, 0xA982),
    (0xA9B3, 0xA9B3),
    (0xA9B6, 0xA9B9),
    (0xA9BC, 0xA9BD),
    (0xA9C0, 0xA9C0),
    (0xA9E5, 0xA9E5),
    (0xAA29, 0xAA2E),
    (0xAA31, 0xAA32),
    (0xAA35, 0xAA36),
    (0xAA43, 0xAA43),
    (0xAA4C, 0xAA4C),
    (0xAA7C, 0xAA7C),
    (0xAAB0, 0xAAB0),
    (0xAAB2, 0xAAB4),
    (0xAAB7, 0xAAB8),
    (0xAABE, 0xAABF),
    (0xAAC1, 0xAAC1),
    (0xAAEC, 0xAAED),
    (0xAAF6, 0xAAF6),
    (0xABE5, 0xABE5),
    (0xABE8, 0xABE8),
    (0xABED, 0xABED),
    (0xFB1E, 0xFB1E),
    (0xFE00, 0xFE0F),
    (0xFE20, 0xFE2F),
    (0xFF9E, 0xFF9F),
    (0x101FD, 0x101FD),
    (0x102E0, 0x102E0),
    (0x10376, 0x1037A),
    (0x10A01, 0x10A03),
    (0x10A05, 0x10A06),
    (0x10A0C, 0x10A0F),
    (0x10A38, 0x10A3A),
    (0x10A3F, 0x10A3F),
    (0x10AE5, 0x10AE6),
    (0x10D24, 0x10D27),
    (0x10D69, 0x10D6D),
    (0x10EAB, 0x10EAC),
    (0x10EFC, 0x10EFF),
    (0x10F46, 0x10F50),
    (0x10F82, 0x10F85),
    (0x11001, 0x11001),
    (0x11038, 0x11046),
    (0x11070, 0x11070),
    (0x11073, 0x11074),
    (0x1107F, 0x11081),
    (0x110B3, 0x110B6),
    (0x110B9, 0x110BA),
    (0x110C2, 0x110C2),
    (0x11100, 0x11102),
    (0x11127, 0x1112B),
    (0x1112D, 0x11134),
    (0x11173, 0x11173),
    (0x11180, 0x11181),
    (0x111B6, 0x111BE),
    (0x111C0, 0x111C0),
    (0x111C9, 0x111CC),
    (0x111CF, 0x111CF),
    (0x1122F, 0x11231),
    (0x11234, 0x11237),
    (0x1123E, 0x1123E),
    (0x11241, 0x11241),
    (0x112DF, 0x112DF),
    (0x112E3, 0x112EA),
    (0x11300, 0x11301),
    (0x1133B, 0x1133C),
    (0x1133E, 0x1133E),
    (0x11340, 0x11340),
    (0x1134D, 0x1134D),
    (0x11357, 0x11357),
    (0x11366, 0x1136C),
    (0x11370, 0x11374),
    (0x113B8, 0x113B8),
    (0x113BB, 0x113C0),
    (0x113C2, 0x113C2),
    (0x113C5, 0x113C5),
    (0x113C7, 0x113C9),
    (0x113CE, 0x113D0),
    (0x113D2, 0x113D2),
    (0x113E1, 0x113E2),
    (0x11438, 0x1143F),
    (0x11442, 0x11444),
    (0x11446, 0x11446),
    (0x1145E, 0x1145E),
    (0x114B0, 0x114B0),
    (0x114B3, 0x114B8),
    (0x114BA, 0x114BA),
    (0x114BD, 0x114BD),
    (0x114BF, 0x114C0),
    (0x114C2, 0x114C3),
    (0x115AF, 0x115AF),
    (0x115B2, 0x115B5),
    (0x115BC, 0x115BD),
    (0x115BF, 0x115C0),
    (0x115DC, 0x115DD),
    (0x11633, 0x1163A),
    (0x1163D, 0x1163D),
    (0x1163F, 0x11640),
    (0x116AB, 0x116AB),
    (0x116AD, 0x116AD),
    (0x116B0, 0x116B7),
    (0x1171D, 0x1171D),
    (0x1171F, 0x1171F),
    (0x11722, 0x11725),
    (0x11727, 0x1172B),
    (0x1182F, 0x11837),
    (0x11839, 0x1183A),
    (0x11930, 0x11930),
    (0x1193B, 0x1193E),
    (0x11943, 0x11943),
    (0x119D4, 0x119D7),
    (0x119DA, 0x119DB),
    (0x119E0, 0x119E0),
    (0x11A01, 0x11A0A),
    (0x11A33, 0x11A38),
    (0x11A3B, 0x11A3E),
    (0x11A47, 0x11A47),
    (0x11A51, 0x11A56),
    (0x11A59, 0x11A5B),
    (0x11A8A, 0x11A96),
    (0x11A98, 0x11A99),
    (0x11C30, 0x11C36),
    (0x11C38, 0x11C3D),
    (0x11C3F, 0x11C3F),
    (0x11C92, 0x11CA7),
    (0x11CAA, 0x11CB0),
    (0x11CB2, 0x11CB3),
    (0x11CB5, 0x11CB6),
    (0x11D31, 0x11D36),
    (0x11D3A, 0x11D3A),
    (0x11D3C, 0x11D3D),
    (0x11D3F, 0x11D45),
    (0x11D47, 0x11D47),
    (0x11D90, 0x11D91),
    (0x11D95, 0x11D95),
    (0x11D97, 0x11D97),
    (0x11EF3, 0x11EF4),
    (0x11F00, 0x11F01),
    (0x11F36, 0x11F3A),
    (0x11F40, 0x11F42),
    (0x11F5A, 0x11F5A),
    (0x13440, 0x13440),
    (0x13447, 0x13455),
    (0x1611E, 0x16129),
    (0x1612D, 0x1612F),
    (0x16AF0, 0x16AF4),
    (0x16B30, 0x16B36),
    (0x16F4F, 0x16F4F),
    (0x16F8F, 0x16F92),
    (0x16FE4, 0x16FE4),
    (0x16FF0, 0x16FF1),
    (0x1BC9D, 0x1BC9E),
    (0x1CF00, 0x1CF2D),
    (0x1CF30, 0x1CF46),
    (0x1D165, 0x1D169),
    (0x1D16D, 0x1D172),
    (0x1D17B, 0x1D182),
    (0x1D185, 0x1D18B),
    (0x1D1AA, 0x1D1AD),
    (0x1D242, 0x1D244),
    (0x1DA00, 0x1DA36),
    (0x1DA3B, 0x1DA6C),
    (0x1DA75, 0x1DA75),
    (0x1DA84, 0x1DA84),
    (0x1DA9B, 0x1DA9F),
    (0x1DAA1, 0x1DAAF),
    (0x1E000, 0x1E006),
    (0x1E008, 0x1E018),
    (0x1E01B, 0x1E021),
    (0x1E023, 0x1E024),
    (0x1E026, 0x1E02A),
    (0x1E08F, 0x1E08F),
    (0x1E130, 0x1E136),
    (0x1E2AE, 0x1E2AE),
    (0x1E2EC, 0x1E2EF),
    (0x1E4EC, 0x1E4EF),
    (0x1E5EE, 0x1E5EF),
    (0x1E8D0, 0x1E8D6),
    (0x1E944, 0x1E94A),
    (0x1F3FB, 0x1F3FF),
    (0xE0020, 0xE007F),
    (0xE0100, 0xE01EF),
];

/// Spacing marks (`SpacingMark`) that continue the preceding cluster.
const GRAPHEME_SPACING_MARK: &[(u32, u32)] = &[
    (0x0903, 0x0903),
    (0x093B, 0x093B),
    (0x093E, 0x0940),
    (0x0949, 0x094C),
    (0x094E, 0x094F),
    (0x0982, 0x0983),
    (0x09BF, 0x09C0),
    (0x09C7, 0x09C8),
    (0x09CB, 0x09CC),
    (0x0A03, 0x0A03),
    (0x0A3E, 0x0A40),
    (0x0A83, 0x0A83),
    (0x0ABE, 0x0AC0),
    (0x0AC9, 0x0AC9),
    (0x0ACB, 0x0ACC),
    (0x0B02, 0x0B03),
    (0x0B40, 0x0B40),
    (0x0B47, 0x0B48),
    (0x0B4B, 0x0B4C),
    (0x0BBF, 0x0BBF),
    (0x0BC1, 0x0BC2),
    (0x0BC6, 0x0BC8),
    (0x0BCA, 0x0BCC),
    (0x0C01, 0x0C03),
    (0x0C41, 0x0C44),
    (0x0C82, 0x0C83),
    (0x0CBE, 0x0CBE),
    (0x0CC1, 0x0CC1),
    (0x0CC3, 0x0CC4),
    (0x0CF3, 0x0CF3),
    (0x0D02, 0x0D03),
    (0x0D3F, 0x0D40),
    (0x0D46, 0x0D48),
    (0x0D4A, 0x0D4C),
    (0x0D82, 0x0D83),
    (0x0DD0, 0x0DD1),
    (0x0DD8, 0x0DDE),
    (0x0DF2, 0x0DF3),
    (0x0E33, 0x0E33),
    (0x0EB3, 0x0EB3),
    (0x0F3E, 0x0F3F),
    (0x0F7F, 0x0F7F),
    (0x1031, 0x1031),
    (0x103B, 0x103C),
    (0x1056, 0x1057),
    (0x1084, 0x1084),
    (0x17B6, 0x17B6),
    (0x17BE, 0x17C5),
    (0x17C7, 0x17C8),
    (0x1923, 0x1926),
    (0x1929, 0x192B),
    (0x1930, 0x1931),
    (0x1933, 0x1938),
    (0x1A19, 0x1A1A),
    (0x1A55, 0x1A55),
    (0x1A57, 0x1A57),
    (0x1A6D, 0x1A72),
    (0x1B04, 0x1B04),
    (0x1B3E, 0x1B41),
    (0x1B82, 0x1B82),
    (0x1BA1, 0x1BA1),
    (0x1BA6, 0x1BA7),
    (0x1BE7, 0x1BE7),
    (0x1BEA, 0x1BEC),
    (0x1BEE, 0x1BEE),
    (0x1C24, 0x1C2B),
    (0x1C34, 0x1C35),
    (0x1CE1, 0x1CE1),
    (0x1CF7, 0x1CF7),
    (0xA823, 0xA824),
    (0xA827, 0xA827),
    (0xA880, 0xA881),
    (0xA8B4, 0xA8C3),
    (0xA952, 0xA952),
    (0xA983, 0xA983),
    (0xA9B4, 0xA9B5),
    (0xA9BA, 0xA9BB),
    (0xA9BE, 0xA9BF),
    (0xAA2F, 0xAA30),
    (0xAA33, 0xAA34),
    (0xAA4D, 0xAA4D),
    (0xAAEB, 0xAAEB),
    (0xAAEE, 0xAAEF),
    (0xAAF5, 0xAAF5),
    (0xABE3, 0xABE4),
    (0xABE6, 0xABE7),
    (0xABE9, 0xABEA),
    (0xABEC, 0xABEC),
    (0x11000, 0x11000),
    (0x11002, 0x11002),
    (0x11082, 0x11082),
    (0x110B0, 0x110B2),
    (0x110B7, 0x110B8),
    (0x1112C, 0x1112C),
    (0x11145, 0x11146),
    (0x11182, 0x11182),
    (0x111B3, 0x111B5),
    (0x111BF, 0x111BF),
    (0x111CE, 0x111CE),
    (0x1122C, 0x1122E),
    (0x11232, 0x11233),
    (0x112E0, 0x112E2),
    (0x11302, 0x11303),
    (0x1133F, 0x1133F),
    (0x11341, 0x11344),
    (0x11347, 0x11348),
    (0x1134B, 0x1134C),
    (0x11362, 0x11363),
    (0x113B9, 0x113BA),
    (0x113CA, 0x113CA),
    (0x113CC, 0x113CD),
    (0x11435, 0x11437),
    (0x11440, 0x11441),
    (0x11445, 0x11445),
    (0x114B1, 0x114B2),
    (0x114B9, 0x114B9),
    (0x114BB, 0x114BC),
    (0x114BE, 0x114BE),
    (0x114C1, 0x114C1),
    (0x115B0, 0x115B1),
    (0x115B8, 0x115BB),
    (0x115BE, 0x115BE),
    (0x11630, 0x11632),
    (0x1163B, 0x1163C),
    (0x1163E, 0x1163E),
    (0x116AC, 0x116AC),
    (0x116AE, 0x116AF),
    (0x1171E, 0x1171E),
    (0x11726, 0x11726),
    (0x1182C, 0x1182E),
    (0x11838, 0x11838),
    (0x11931, 0x11935),
    (0x11937, 0x11938),
    (0x11940, 0x11940),
    (0x11942, 0x11942),
    (0x119D1, 0x119D3),
    (0x119DC, 0x119DF),
    (0x119E4, 0x119E4),
    (0x11A39, 0x11A39),
    (0x11A57, 0x11A58),
    (0x11A97, 0x11A97),
    (0x11C2F, 0x11C2F),
    (0x11C3E, 0x11C3E),
    (0x11CA9, 0x11CA9),
    (0x11CB1, 0x11CB1),
    (0x11CB4, 0x11CB4),
    (0x11D8A, 0x11D8E),
    (0x11D93, 0x11D94),
    (0x11D96, 0x11D96),
    (0x11EF5, 0x11EF6),
    (0x11F03, 0x11F03),
    (0x11F34, 0x11F35),
    (0x11F3E, 0x11F3F),
    (0x1612A, 0x1612C),
    (0x16F51, 0x16F87),
];

/// Prepend characters (`Prepend`) that begin an extended grapheme cluster.
const GRAPHEME_PREPEND: &[(u32, u32)] = &[
    (0x0600, 0x0605),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x0D4E, 0x0D4E),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x111C2, 0x111C3),
    (0x113D1, 0x113D1),
    (0x1193F, 0x1193F),
    (0x11941, 0x11941),
    (0x11A3A, 0x11A3A),
    (0x11A84, 0x11A89),
    (0x11D46, 0x11D46),
    (0x11F02, 0x11F02),
];

/// Control characters (`Control`), excluding CR and LF which are checked first.
const GRAPHEME_CONTROL: &[(u32, u32)] = &[
    (0x0000, 0x0009),
    (0x000B, 0x000C),
    (0x000E, 0x001F),
    (0x007F, 0x009F),
    (0x00AD, 0x00AD),
    (0x061C, 0x061C),
    (0x180E, 0x180E),
    (0x200B, 0x200B),
    (0x200E, 0x200F),
    (0x2028, 0x202E),
    (0x2060, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF0, 0xFFFB),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0000, 0xE001F),
    (0xE0080, 0xE00FF),
    (0xE01F0, 0xE0FFF),
];

/// Clusters with default emoji presentation, retained as their own word.
const EMOJI_PRESENTATION: &[(u32, u32)] = &[
    (0x231A, 0x231B),
    (0x23E9, 0x23EC),
    (0x23F0, 0x23F0),
    (0x23F3, 0x23F3),
    (0x25FD, 0x25FE),
    (0x2614, 0x2615),
    (0x2648, 0x2653),
    (0x267F, 0x267F),
    (0x2693, 0x2693),
    (0x26A1, 0x26A1),
    (0x26AA, 0x26AB),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26CE, 0x26CE),
    (0x26D4, 0x26D4),
    (0x26EA, 0x26EA),
    (0x26F2, 0x26F3),
    (0x26F5, 0x26F5),
    (0x26FA, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2705, 0x2705),
    (0x270A, 0x270B),
    (0x2728, 0x2728),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x1F004, 0x1F004),
    (0x1F0CF, 0x1F0CF),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F1E6, 0x1F1FF),
    (0x1F201, 0x1F201),
    (0x1F21A, 0x1F21A),
    (0x1F22F, 0x1F22F),
    (0x1F232, 0x1F236),
    (0x1F238, 0x1F23A),
    (0x1F250, 0x1F251),
    (0x1F300, 0x1F320),
    (0x1F32D, 0x1F335),
    (0x1F337, 0x1F37C),
    (0x1F37E, 0x1F393),
    (0x1F3A0, 0x1F3CA),
    (0x1F3CF, 0x1F3D3),
    (0x1F3E0, 0x1F3F0),
    (0x1F3F4, 0x1F3F4),
    (0x1F3F8, 0x1F43E),
    (0x1F440, 0x1F440),
    (0x1F442, 0x1F4FC),
    (0x1F4FF, 0x1F53D),
    (0x1F54B, 0x1F54E),
    (0x1F550, 0x1F567),
    (0x1F57A, 0x1F57A),
    (0x1F595, 0x1F596),
    (0x1F5A4, 0x1F5A4),
    (0x1F5FB, 0x1F64F),
    (0x1F680, 0x1F6C5),
    (0x1F6CC, 0x1F6CC),
    (0x1F6D0, 0x1F6D2),
    (0x1F6D5, 0x1F6D7),
    (0x1F6DC, 0x1F6DF),
    (0x1F6EB, 0x1F6EC),
    (0x1F6F4, 0x1F6FC),
    (0x1F7E0, 0x1F7EB),
    (0x1F7F0, 0x1F7F0),
    (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945),
    (0x1F947, 0x1F9FF),
    (0x1FA70, 0x1FA7C),
    (0x1FA80, 0x1FA89),
    (0x1FA8F, 0x1FAC6),
    (0x1FACE, 0x1FADC),
    (0x1FADF, 0x1FAE9),
    (0x1FAF0, 0x1FAF8),
];

/// Unicode decimal digits (`Nd`), matched by the baseline's `decimalDigits`.
const DECIMAL_DIGIT: &[(u32, u32)] = &[
    (0x0030, 0x0039),
    (0x0660, 0x0669),
    (0x06F0, 0x06F9),
    (0x07C0, 0x07C9),
    (0x0966, 0x096F),
    (0x09E6, 0x09EF),
    (0x0A66, 0x0A6F),
    (0x0AE6, 0x0AEF),
    (0x0B66, 0x0B6F),
    (0x0BE6, 0x0BEF),
    (0x0C66, 0x0C6F),
    (0x0CE6, 0x0CEF),
    (0x0D66, 0x0D6F),
    (0x0DE6, 0x0DEF),
    (0x0E50, 0x0E59),
    (0x0ED0, 0x0ED9),
    (0x0F20, 0x0F29),
    (0x1040, 0x1049),
    (0x1090, 0x1099),
    (0x17E0, 0x17E9),
    (0x1810, 0x1819),
    (0x1946, 0x194F),
    (0x19D0, 0x19D9),
    (0x1A80, 0x1A89),
    (0x1A90, 0x1A99),
    (0x1B50, 0x1B59),
    (0x1BB0, 0x1BB9),
    (0x1C40, 0x1C49),
    (0x1C50, 0x1C59),
    (0xA620, 0xA629),
    (0xA8D0, 0xA8D9),
    (0xA900, 0xA909),
    (0xA9D0, 0xA9D9),
    (0xA9F0, 0xA9F9),
    (0xAA50, 0xAA59),
    (0xABF0, 0xABF9),
    (0xFF10, 0xFF19),
    (0x104A0, 0x104A9),
    (0x10D30, 0x10D39),
    (0x10D40, 0x10D49),
    (0x11066, 0x1106F),
    (0x110F0, 0x110F9),
    (0x11136, 0x1113F),
    (0x111D0, 0x111D9),
    (0x112F0, 0x112F9),
    (0x11450, 0x11459),
    (0x114D0, 0x114D9),
    (0x11650, 0x11659),
    (0x116C0, 0x116C9),
    (0x116D0, 0x116E3),
    (0x11730, 0x11739),
    (0x118E0, 0x118E9),
    (0x11950, 0x11959),
    (0x11BF0, 0x11BF9),
    (0x11C50, 0x11C59),
    (0x11D50, 0x11D59),
    (0x11DA0, 0x11DA9),
    (0x11F50, 0x11F59),
    (0x16130, 0x16139),
    (0x16A60, 0x16A69),
    (0x16AC0, 0x16AC9),
    (0x16B50, 0x16B59),
    (0x16D70, 0x16D79),
    (0x1CCF0, 0x1CCF9),
    (0x1D7CE, 0x1D7FF),
    (0x1E140, 0x1E149),
    (0x1E2F0, 0x1E2F9),
    (0x1E4F0, 0x1E4F9),
    (0x1E5F1, 0x1E5FA),
    (0x1E950, 0x1E959),
    (0x1FBF0, 0x1FBF9),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn style_outputs(input: &str) -> Vec<(&'static str, String)> {
        CaseConversionStyle::ALL
            .into_iter()
            .map(|style| {
                let evaluation = evaluate(&CaseConversionRequest::new(input, style));
                (
                    style.label(),
                    evaluation.output().expect("valid output").to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn segmentation_keeps_acronym_runs_and_splits_the_trailing_capital() {
        assert_eq!(segment("HTTPServer"), vec!["HTTP", "Server"]);
        assert_eq!(segment("XMLHttpRequest"), vec!["XML", "Http", "Request"]);
        assert_eq!(
            segment("parseHTMLDocument"),
            vec!["parse", "HTML", "Document"]
        );
    }

    #[test]
    fn segmentation_pins_digit_transitions_in_both_directions() {
        assert_eq!(segment("version2Value"), vec!["version", "2", "Value"]);
        assert_eq!(segment("2ndPlace"), vec!["2", "nd", "Place"]);
        assert_eq!(segment("item99Count"), vec!["item", "99", "Count"]);
    }

    #[test]
    fn segmentation_treats_mixed_separators_punctuation_and_symbols_as_breaks() {
        assert_eq!(
            segment("foo_bar-baz.qux quux"),
            ["foo", "bar", "baz", "qux", "quux"]
        );
        assert_eq!(segment("foo+bar=BAZ"), ["foo", "bar", "BAZ"]);
        assert_eq!(segment("__camelCase__"), ["camel", "Case"]);
    }

    #[test]
    fn emoji_are_retained_as_their_own_word_and_not_split() {
        assert_eq!(segment("foo💡bar"), ["foo", "💡", "bar"]);
        assert_eq!(segment("a👨‍👩‍👧b"), ["a", "👨‍👩‍👧", "b"]);
    }

    #[test]
    fn combining_marks_stay_attached_to_their_base_grapheme() {
        let decomposed = "Cafe\u{301}Bar";
        assert_eq!(segment(decomposed), ["Cafe\u{301}", "Bar"]);
        let evaluation = evaluate(&CaseConversionRequest::new(
            decomposed,
            CaseConversionStyle::Snake,
        ));
        assert_eq!(evaluation.words(), ["Cafe\u{301}", "Bar"]);
        assert_eq!(evaluation.output(), Some("cafe\u{301}_bar"));
    }

    #[test]
    fn all_nine_styles_render_the_shared_words() {
        assert_eq!(
            style_outputs("HTTPServer"),
            vec![
                ("camelCase", "httpServer".to_owned()),
                ("PascalCase", "HttpServer".to_owned()),
                ("snake_case", "http_server".to_owned()),
                ("SCREAMING_SNAKE_CASE", "HTTP_SERVER".to_owned()),
                ("kebab-case", "http-server".to_owned()),
                ("Title Case", "Http Server".to_owned()),
                ("sentence case", "Http server".to_owned()),
                ("lowercase", "http server".to_owned()),
                ("UPPERCASE", "HTTP SERVER".to_owned()),
            ]
        );
        assert_eq!(
            style_outputs("version2Value"),
            vec![
                ("camelCase", "version2Value".to_owned()),
                ("PascalCase", "Version2Value".to_owned()),
                ("snake_case", "version_2_value".to_owned()),
                ("SCREAMING_SNAKE_CASE", "VERSION_2_VALUE".to_owned()),
                ("kebab-case", "version-2-value".to_owned()),
                ("Title Case", "Version 2 Value".to_owned()),
                ("sentence case", "Version 2 value".to_owned()),
                ("lowercase", "version 2 value".to_owned()),
                ("UPPERCASE", "VERSION 2 VALUE".to_owned()),
            ]
        );
    }

    #[test]
    fn sentence_style_capitalizes_only_the_first_word() {
        let evaluation = evaluate(&CaseConversionRequest::new(
            "hello WORLD",
            CaseConversionStyle::Sentence,
        ));
        assert_eq!(evaluation.output(), Some("Hello world"));
    }

    #[test]
    fn empty_input_is_neutral() {
        let evaluation = evaluate(&CaseConversionRequest::new("", CaseConversionStyle::Camel));
        assert_eq!(evaluation, CaseConversionEvaluation::Empty);
        assert!(CaseConversion::is_neutral(&evaluation));
        assert!(!evaluation.is_valid_operation());
    }

    #[test]
    fn input_beyond_the_utf16_limit_is_invalid() {
        let oversized = "a".repeat(MAXIMUM_INPUT_UTF16_LENGTH + 1);
        let evaluation = evaluate(&CaseConversionRequest::new(
            oversized,
            CaseConversionStyle::Upper,
        ));
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.output().is_none());
        assert_eq!(evaluation.diagnostics().len(), 1);
        assert!(evaluation.diagnostics()[0].message.contains("1,048,576"));
    }

    #[test]
    fn astral_input_counts_utf16_units() {
        // Each astral scalar is two UTF-16 code units: 524,289 astral scalars
        // cross the one-unit-over limit that a scalar count would miss.
        let oversized = "😀".repeat(MAXIMUM_INPUT_UTF16_LENGTH / 2 + 1);
        let evaluation = evaluate(&CaseConversionRequest::new(
            oversized,
            CaseConversionStyle::Upper,
        ));
        assert!(!evaluation.is_valid_operation());
    }

    #[test]
    fn snapshot_round_trips_exactly_without_reevaluation() {
        let request = CaseConversionRequest::new("HTTPServer", CaseConversionStyle::Kebab);
        let evaluation = evaluate(&request);
        let snapshot =
            <CaseConversion as Utility>::snapshot(&request, &evaluation).expect("snapshot");

        let encoded = serde_json::to_value(&snapshot).expect("serialize");
        let decoded: CaseConversionSnapshot = serde_json::from_value(encoded).expect("deserialize");
        assert_eq!(decoded, snapshot);

        let (restored_request, restored_evaluation) =
            <CaseConversion as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.output(), Some("http-server"));
        assert_eq!(restored_evaluation.words(), ["HTTP", "Server"]);
    }

    #[test]
    fn neutral_and_invalid_evaluations_never_snapshot() {
        let request = CaseConversionRequest::new("", CaseConversionStyle::Snake);
        let empty = evaluate(&request);
        assert!(<CaseConversion as Utility>::snapshot(&request, &empty).is_none());

        let oversized = CaseConversionRequest::new(
            "a".repeat(MAXIMUM_INPUT_UTF16_LENGTH + 1),
            CaseConversionStyle::Snake,
        );
        let invalid = evaluate(&oversized);
        assert!(<CaseConversion as Utility>::snapshot(&oversized, &invalid).is_none());
    }

    #[test]
    fn request_snake_case_serialization_is_stable() {
        let request = CaseConversionRequest::new("a", CaseConversionStyle::ScreamingSnake);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"input":"a","style":"screaming_snake"}"#
        );
    }
}
