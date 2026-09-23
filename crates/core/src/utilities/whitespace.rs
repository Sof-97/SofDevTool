//! The Whitespace Conversion Utility's GPUI-independent contract.
//!
//! Nine explicit, separately named actions transform whitespace with no generic
//! "clean" command. The engine preserves each line's ending unless the action
//! normalizes them, and it applies only the chosen action's exact semantics:
//! edge/per-line trim, horizontal/all collapse, line-ending normalization,
//! tab-stop expansion, leading-indent-only spaces-to-tabs, blank-line removal
//! and dedent. Tab width is bounded to 1-8 (default 4); a rejected width
//! produces a diagnostic instead of a partial result.

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Whitespace Conversion Utility.
pub const WHITESPACE_UTILITY_ID: &str = "whitespace-conversion";

/// Schema version of [`WhitespaceSnapshot`].
pub const WHITESPACE_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Smallest accepted tab width.
pub const MIN_TAB_WIDTH: u8 = 1;
/// Largest accepted tab width.
pub const MAX_TAB_WIDTH: u8 = 8;
/// Default tab width, matching the existing product contract.
pub const DEFAULT_TAB_WIDTH: u8 = 4;

/// The nine independently named whitespace actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhitespaceAction {
    EdgeTrim,
    PerLineTrim,
    CollapseHorizontalWhitespace,
    CollapseAllWhitespace,
    NormalizeLineEndings,
    TabsToSpaces,
    SpacesToTabs,
    RemoveBlankLines,
    Dedent,
}

impl WhitespaceAction {
    /// Every action, in the catalog order shown to the user.
    pub const ALL: [WhitespaceAction; 9] = [
        WhitespaceAction::EdgeTrim,
        WhitespaceAction::PerLineTrim,
        WhitespaceAction::CollapseHorizontalWhitespace,
        WhitespaceAction::CollapseAllWhitespace,
        WhitespaceAction::NormalizeLineEndings,
        WhitespaceAction::TabsToSpaces,
        WhitespaceAction::SpacesToTabs,
        WhitespaceAction::RemoveBlankLines,
        WhitespaceAction::Dedent,
    ];

    /// True when this action reads the tab-width option.
    pub fn uses_tab_width(self) -> bool {
        matches!(self, Self::TabsToSpaces | Self::SpacesToTabs | Self::Dedent)
    }
}

/// The explicit line-ending target used by `NormalizeLineEndings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhitespaceLineEnding {
    Lf,
    Crlf,
    Cr,
}

impl WhitespaceLineEnding {
    /// Every option, in the catalog order shown to the user.
    pub const ALL: [WhitespaceLineEnding; 3] = [
        WhitespaceLineEnding::Lf,
        WhitespaceLineEnding::Crlf,
        WhitespaceLineEnding::Cr,
    ];

    /// The exact text written for this ending.
    pub fn value(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
            Self::Cr => "\r",
        }
    }

    /// The short human label for this ending.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
            Self::Cr => "CR",
        }
    }
}

/// The complete, strongly typed input for one whitespace evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhitespaceRequest {
    pub input: String,
    pub action: WhitespaceAction,
    pub line_ending: WhitespaceLineEnding,
    pub tab_width: u8,
}

impl WhitespaceRequest {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            action: WhitespaceAction::EdgeTrim,
            line_ending: WhitespaceLineEnding::Lf,
            tab_width: DEFAULT_TAB_WIDTH,
        }
    }
}

/// The typed outcome of one whitespace evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhitespaceEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl WhitespaceEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            WhitespaceEvaluation::Valid { output } => Some(output),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            WhitespaceEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, WhitespaceEvaluation::Valid { .. })
    }
}

/// The Utility-owned payload persisted in History.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhitespaceSnapshot {
    pub request: WhitespaceRequest,
    pub output: String,
}

impl WhitespaceSnapshot {
    pub fn restore(&self) -> (&WhitespaceRequest, &str) {
        (&self.request, &self.output)
    }
}

/// Evaluates one request. Deterministic and free of hidden state.
pub fn evaluate(request: &WhitespaceRequest) -> WhitespaceEvaluation {
    if request.input.is_empty() {
        return WhitespaceEvaluation::Empty;
    }
    if !(MIN_TAB_WIDTH..=MAX_TAB_WIDTH).contains(&request.tab_width) {
        return WhitespaceEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error("Tab width must be from 1 through 8.")],
        };
    }
    let width = request.tab_width as usize;
    let output = match request.action {
        WhitespaceAction::EdgeTrim => request.input.trim().to_owned(),
        WhitespaceAction::PerLineTrim => transform_lines(&request.input, |content| {
            trim_horizontal(content).to_owned()
        }),
        WhitespaceAction::CollapseHorizontalWhitespace => {
            transform_lines(&request.input, |content| collapse(content, false))
        }
        WhitespaceAction::CollapseAllWhitespace => collapse(&request.input, true),
        WhitespaceAction::NormalizeLineEndings => {
            let ending = request.line_ending.value();
            let mut output = String::with_capacity(request.input.len());
            for line in lines(&request.input) {
                output.push_str(line.content);
                if !line.ending.is_empty() {
                    output.push_str(ending);
                }
            }
            output
        }
        WhitespaceAction::TabsToSpaces => {
            transform_lines(&request.input, |content| expand_tabs(content, width))
        }
        WhitespaceAction::SpacesToTabs => transform_lines(&request.input, |content| {
            compress_leading_spaces(content, width)
        }),
        WhitespaceAction::RemoveBlankLines => {
            let mut output = String::with_capacity(request.input.len());
            for line in lines(&request.input) {
                if !is_blank(line.content) {
                    output.push_str(line.content);
                    output.push_str(line.ending);
                }
            }
            output
        }
        WhitespaceAction::Dedent => dedent(&request.input, width),
    };
    WhitespaceEvaluation::Valid { output }
}

/// One line split on scalar `\n`/`\r`; `\r\n` is a single ending. A trailing
/// empty line without an ending is not produced.
struct Line<'a> {
    content: &'a str,
    ending: &'a str,
}

fn lines(input: &str) -> Vec<Line<'_>> {
    if input.is_empty() {
        return Vec::new();
    }
    // UTF-8 continuation bytes are >= 0x80, so byte scanning for 0x0A/0x0D is
    // equivalent to scanning Unicode scalars and always lands on char
    // boundaries.
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                result.push(Line {
                    content: &input[start..index],
                    ending: "\n",
                });
                index += 1;
                start = index;
            }
            b'\r' => {
                if index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
                    result.push(Line {
                        content: &input[start..index],
                        ending: "\r\n",
                    });
                    index += 2;
                } else {
                    result.push(Line {
                        content: &input[start..index],
                        ending: "\r",
                    });
                    index += 1;
                }
                start = index;
            }
            _ => index += 1,
        }
    }
    if start < bytes.len() {
        result.push(Line {
            content: &input[start..],
            ending: "",
        });
    }
    result
}

/// Whitespace the Swift baseline's `.whitespaces` set trims: horizontal
/// whitespace only, excluding every line-ending code point.
fn is_horizontal_whitespace(character: char) -> bool {
    character.is_whitespace()
        && !matches!(
            character,
            '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
        )
}

fn trim_horizontal(value: &str) -> &str {
    value.trim_matches(is_horizontal_whitespace)
}

/// A line is blank when every character is whitespace (empty is blank).
fn is_blank(content: &str) -> bool {
    content.chars().all(char::is_whitespace)
}

fn transform_lines(input: &str, transform: impl Fn(&str) -> String) -> String {
    let mut output = String::with_capacity(input.len());
    for line in lines(input) {
        output.push_str(&transform(line.content));
        output.push_str(line.ending);
    }
    output
}

/// Collapses every run of whitespace to one space. When `including_line_endings`
/// is false, scalar `\n`/`\r` are treated as content.
fn collapse(input: &str, including_line_endings: bool) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_run = false;
    for character in input.chars() {
        let is_line_ending = character == '\n' || character == '\r';
        let should_collapse =
            character.is_whitespace() && (including_line_endings || !is_line_ending);
        if should_collapse {
            if !in_run {
                result.push(' ');
            }
            in_run = true;
        } else {
            result.push(character);
            in_run = false;
        }
    }
    result
}

/// Expands tabs against visual tab stops. Non-tab characters advance one column.
fn expand_tabs(content: &str, width: usize) -> String {
    let mut result = String::with_capacity(content.len());
    let mut column = 0usize;
    for character in content.chars() {
        if character == '\t' {
            let count = width - column % width;
            for _ in 0..count {
                result.push(' ');
            }
            column += count;
        } else {
            result.push(character);
            column += 1;
        }
    }
    result
}

/// Rewrites only the leading indentation, replacing a full tab stop of spaces
/// with a tab and leaving everything after the first non-indent character
/// untouched.
fn compress_leading_spaces(content: &str, width: usize) -> String {
    let characters: Vec<char> = content.chars().collect();
    let mut result = String::with_capacity(content.len());
    let mut column = 0usize;
    let mut index = 0usize;
    while index < characters.len() {
        let character = characters[index];
        if character == '\t' {
            result.push('\t');
            column += width - column % width;
            index += 1;
        } else if character == ' ' {
            let stop = width - column % width;
            let mut cursor = index;
            let mut available = 0usize;
            while cursor < characters.len() && characters[cursor] == ' ' && available < stop {
                available += 1;
                cursor += 1;
            }
            if available == stop {
                result.push('\t');
                column += stop;
            } else {
                for _ in 0..available {
                    result.push(' ');
                }
                column += available;
            }
            index = cursor;
        } else {
            break;
        }
    }
    for character in &characters[index..] {
        result.push(*character);
    }
    result
}

/// Removes the common leading indentation from every non-blank line. Blank
/// lines are preserved exactly and an all-blank or unindented input is returned
/// unchanged.
fn dedent(input: &str, width: usize) -> String {
    let parsed = lines(input);
    let non_blank: Vec<&Line<'_>> = parsed
        .iter()
        .filter(|line| !is_blank(line.content))
        .collect();
    if non_blank.is_empty() {
        return input.to_owned();
    }
    let removable = non_blank
        .iter()
        .map(|line| indentation_width(line.content, width))
        .min()
        .unwrap_or(0);
    if removable == 0 {
        return input.to_owned();
    }
    let mut result = String::with_capacity(input.len());
    for line in &parsed {
        if is_blank(line.content) {
            result.push_str(line.content);
        } else {
            result.push_str(&removing_indent(removable, line.content, width));
        }
        result.push_str(line.ending);
    }
    result
}

fn indentation_width(content: &str, width: usize) -> usize {
    let mut column = 0usize;
    for character in content.chars() {
        match character {
            ' ' => column += 1,
            '\t' => column += width - column % width,
            _ => break,
        }
    }
    column
}

/// Removes exactly `amount` visual columns of leading indentation. When a tab
/// would cross the boundary, the leftover columns are restored as spaces.
fn removing_indent(amount: usize, content: &str, width: usize) -> String {
    let characters: Vec<char> = content.chars().collect();
    let mut removed = 0usize;
    let mut index = 0usize;
    let mut replacement = String::new();
    while index < characters.len() && removed < amount {
        let character = characters[index];
        let advance = match character {
            ' ' => 1,
            '\t' => width - removed % width,
            _ => break,
        };
        if removed + advance > amount {
            for _ in 0..(removed + advance - amount) {
                replacement.push(' ');
            }
        }
        removed += advance;
        index += 1;
    }
    let mut result = replacement;
    for character in &characters[index..] {
        result.push(*character);
    }
    result
}

/// The Whitespace Conversion Utility's identity for the shared [`Utility`] trait.
pub struct Whitespace;

impl Utility for Whitespace {
    type Request = WhitespaceRequest;
    type Evaluation = WhitespaceEvaluation;
    type Snapshot = WhitespaceSnapshot;

    const ID: &'static str = WHITESPACE_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = WHITESPACE_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> WhitespaceEvaluation {
        WhitespaceEvaluation::Empty
    }

    fn evaluate(request: &WhitespaceRequest) -> WhitespaceEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &WhitespaceEvaluation) -> bool {
        matches!(evaluation, WhitespaceEvaluation::Empty)
    }

    fn snapshot(
        request: &WhitespaceRequest,
        evaluation: &WhitespaceEvaluation,
    ) -> Option<WhitespaceSnapshot> {
        evaluation.output().map(|output| WhitespaceSnapshot {
            request: request.clone(),
            output: output.to_owned(),
        })
    }

    fn restore(snapshot: &WhitespaceSnapshot) -> (WhitespaceRequest, WhitespaceEvaluation) {
        (
            snapshot.request.clone(),
            WhitespaceEvaluation::Valid {
                output: snapshot.output.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(input: &str, action: WhitespaceAction) -> WhitespaceRequest {
        WhitespaceRequest {
            input: input.to_owned(),
            action,
            line_ending: WhitespaceLineEnding::Lf,
            tab_width: DEFAULT_TAB_WIDTH,
        }
    }

    fn request_with(
        input: &str,
        action: WhitespaceAction,
        line_ending: WhitespaceLineEnding,
        tab_width: u8,
    ) -> WhitespaceRequest {
        WhitespaceRequest {
            input: input.to_owned(),
            action,
            line_ending,
            tab_width,
        }
    }

    fn output(request: &WhitespaceRequest) -> String {
        evaluate(request).output().expect("valid output").to_owned()
    }

    #[test]
    fn every_action_has_a_distinct_identity_and_uses_tab_width_explicitly() {
        assert_eq!(WhitespaceAction::ALL.len(), 9);
        let mut seen = std::collections::BTreeSet::new();
        for action in WhitespaceAction::ALL {
            assert!(seen.insert(format!("{action:?}")), "action repeats");
        }
        let using: Vec<_> = WhitespaceAction::ALL
            .into_iter()
            .filter(|action| action.uses_tab_width())
            .collect();
        assert_eq!(
            using,
            vec![
                WhitespaceAction::TabsToSpaces,
                WhitespaceAction::SpacesToTabs,
                WhitespaceAction::Dedent,
            ]
        );
        assert_eq!(WhitespaceLineEnding::Lf.value(), "\n");
        assert_eq!(WhitespaceLineEnding::Crlf.value(), "\r\n");
        assert_eq!(WhitespaceLineEnding::Cr.value(), "\r");
    }

    #[test]
    fn edge_trim_removes_surrounding_whitespace_and_newlines() {
        assert_eq!(
            output(&request("  café \n", WhitespaceAction::EdgeTrim)),
            "café"
        );
        assert_eq!(
            output(&request("\t\n  hi  \r\n", WhitespaceAction::EdgeTrim)),
            "hi"
        );
        // Unicode whitespace (NBSP, ideographic space) is trimmed too.
        assert_eq!(
            output(&request("\u{00a0}café\u{3000}", WhitespaceAction::EdgeTrim)),
            "café"
        );
        assert_eq!(output(&request("   ", WhitespaceAction::EdgeTrim)), "");
        // A deliberate no-op stays a valid operation.
        assert_eq!(output(&request("cafe", WhitespaceAction::EdgeTrim)), "cafe");
    }

    #[test]
    fn per_line_trim_trims_each_line_but_preserves_endings() {
        assert_eq!(
            output(&request("  a  \r\n\tb\t\r", WhitespaceAction::PerLineTrim)),
            "a\r\nb\r"
        );
        assert_eq!(
            output(&request("  \n x", WhitespaceAction::PerLineTrim)),
            "\nx"
        );
        assert_eq!(
            output(&request(
                "\u{3000}café\u{3000}\n",
                WhitespaceAction::PerLineTrim
            )),
            "café\n"
        );
        assert_eq!(
            output(&request("  x  ", WhitespaceAction::PerLineTrim)),
            "x"
        );
    }

    #[test]
    fn collapse_horizontal_whitespace_collapses_runs_within_each_line() {
        assert_eq!(
            output(&request(
                "a\t  b\n c",
                WhitespaceAction::CollapseHorizontalWhitespace
            )),
            "a b\n c"
        );
        // Unicode whitespace collapses, but line endings are preserved.
        assert_eq!(
            output(&request(
                "a\u{00a0}\u{00a0}b",
                WhitespaceAction::CollapseHorizontalWhitespace
            )),
            "a b"
        );
        assert_eq!(
            output(&request(
                "   leading and trailing   ",
                WhitespaceAction::CollapseHorizontalWhitespace
            )),
            " leading and trailing "
        );
        assert_eq!(
            output(&request(
                "a \n\tb",
                WhitespaceAction::CollapseHorizontalWhitespace
            )),
            "a \n b"
        );
    }

    #[test]
    fn collapse_all_whitespace_flattens_every_run_including_line_endings() {
        assert_eq!(
            output(&request(
                " a\n\t b ",
                WhitespaceAction::CollapseAllWhitespace
            )),
            " a b "
        );
        assert_eq!(
            output(&request(
                "a\r\n\r\nb",
                WhitespaceAction::CollapseAllWhitespace
            )),
            "a b"
        );
        assert_eq!(
            output(&request(
                "café\u{00a0}\n\u{3000}au lait",
                WhitespaceAction::CollapseAllWhitespace
            )),
            "café au lait"
        );
    }

    #[test]
    fn normalize_line_endings_honors_each_target_and_lone_carriage_return() {
        let input = "a\r\nb\rc\n";
        assert_eq!(
            output(&request_with(
                input,
                WhitespaceAction::NormalizeLineEndings,
                WhitespaceLineEnding::Lf,
                4
            )),
            "a\nb\nc\n"
        );
        assert_eq!(
            output(&request_with(
                input,
                WhitespaceAction::NormalizeLineEndings,
                WhitespaceLineEnding::Crlf,
                4
            )),
            "a\r\nb\r\nc\r\n"
        );
        assert_eq!(
            output(&request_with(
                input,
                WhitespaceAction::NormalizeLineEndings,
                WhitespaceLineEnding::Cr,
                4
            )),
            "a\rb\rc\r"
        );
        // A missing final ending is not invented.
        assert_eq!(
            output(&request_with(
                "a\r\nb",
                WhitespaceAction::NormalizeLineEndings,
                WhitespaceLineEnding::Lf,
                4
            )),
            "a\nb"
        );
        assert_eq!(
            output(&request_with(
                "\n\n",
                WhitespaceAction::NormalizeLineEndings,
                WhitespaceLineEnding::Lf,
                4
            )),
            "\n\n"
        );
    }

    #[test]
    fn tabs_to_spaces_uses_tab_stops_and_each_width() {
        assert_eq!(
            output(&request_with(
                "a\tb\n\t👩🏽\u{200D}💻",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                4
            )),
            "a   b\n    👩🏽\u{200D}💻"
        );
        // A single-scalar non-ASCII character still advances one column.
        assert_eq!(
            output(&request_with(
                "é\tb",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                4
            )),
            "é   b"
        );
        assert_eq!(
            output(&request_with(
                "a\tb",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                1
            )),
            "a b"
        );
        assert_eq!(
            output(&request_with(
                "a\tb",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                8
            )),
            "a       b"
        );
        assert_eq!(
            output(&request_with(
                "\t\tx",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                4
            )),
            "        x"
        );
        assert_eq!(
            output(&request_with(
                "ab\tc",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                4
            )),
            "ab  c"
        );
    }

    #[test]
    fn spaces_to_tabs_only_rewrites_leading_indentation() {
        assert_eq!(
            output(&request_with(
                "        code  aligned",
                WhitespaceAction::SpacesToTabs,
                WhitespaceLineEnding::Lf,
                4
            )),
            "\t\tcode  aligned"
        );
        // A partial tab stop of spaces is left as spaces.
        assert_eq!(
            output(&request_with(
                "     x",
                WhitespaceAction::SpacesToTabs,
                WhitespaceLineEnding::Lf,
                4
            )),
            "\t x"
        );
        // Existing tabs in the indentation are preserved.
        assert_eq!(
            output(&request_with(
                "\t  x",
                WhitespaceAction::SpacesToTabs,
                WhitespaceLineEnding::Lf,
                4
            )),
            "\t  x"
        );
        // Spaces after content are untouched.
        assert_eq!(
            output(&request_with(
                "  a  b",
                WhitespaceAction::SpacesToTabs,
                WhitespaceLineEnding::Lf,
                4
            )),
            "  a  b"
        );
        assert_eq!(
            output(&request_with(
                "  a",
                WhitespaceAction::SpacesToTabs,
                WhitespaceLineEnding::Lf,
                1
            )),
            "\t\ta"
        );
        assert_eq!(
            output(&request_with(
                "        x",
                WhitespaceAction::SpacesToTabs,
                WhitespaceLineEnding::Lf,
                8
            )),
            "\tx"
        );
    }

    #[test]
    fn remove_blank_lines_drops_whitespace_only_lines() {
        assert_eq!(
            output(&request("a\n \n\tb\n", WhitespaceAction::RemoveBlankLines)),
            "a\n\tb\n"
        );
        // Unicode whitespace makes a line blank too.
        assert_eq!(
            output(&request(
                "a\n\u{3000}\nb",
                WhitespaceAction::RemoveBlankLines
            )),
            "a\nb"
        );
        assert_eq!(
            output(&request("a\r\n\r\nb", WhitespaceAction::RemoveBlankLines)),
            "a\r\nb"
        );
        assert_eq!(
            output(&request("\n\n", WhitespaceAction::RemoveBlankLines)),
            ""
        );
        assert_eq!(
            output(&request("   ", WhitespaceAction::RemoveBlankLines)),
            ""
        );
    }

    #[test]
    fn dedent_removes_the_common_indentation_with_visual_columns() {
        let input = "\talpha\n  beta\n\t  gamma\n   \n";
        assert_eq!(
            output(&request_with(
                input,
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                4
            )),
            "  alpha\nbeta\n    gamma\n   \n"
        );
        // An all-blank input is returned unchanged.
        assert_eq!(
            output(&request_with(
                " \t\n\t ",
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                4
            )),
            " \t\n\t "
        );
        // No common indentation is a no-op.
        assert_eq!(
            output(&request_with(
                "a\nb",
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                4
            )),
            "a\nb"
        );
        assert_eq!(
            output(&request_with(
                "  a\n    b\n",
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                4
            )),
            "a\n  b\n"
        );
        assert_eq!(
            output(&request_with(
                "\ta\n\t\tb",
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                4
            )),
            "a\n\tb"
        );
        // A tab crossing the removal boundary leaves spaces behind.
        assert_eq!(
            output(&request_with(
                "   x\n\ty",
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                4
            )),
            "x\n y"
        );
        assert_eq!(
            output(&request_with(
                "  a\n b",
                WhitespaceAction::Dedent,
                WhitespaceLineEnding::Lf,
                1
            )),
            " a\nb"
        );
    }

    #[test]
    fn unicode_graphemes_survive_each_action() {
        let family = "👩🏽\u{200D}💻";
        let combining = "Cafe\u{301}";
        assert_eq!(
            output(&request(
                &format!("  {family} {combining}  "),
                WhitespaceAction::EdgeTrim
            )),
            format!("{family} {combining}")
        );
        assert_eq!(
            output(&request(
                &format!("  {family}  {combining}  "),
                WhitespaceAction::CollapseHorizontalWhitespace
            )),
            format!(" {family} {combining} ")
        );
        assert_eq!(
            output(&request(
                &format!("{family}\n{combining}"),
                WhitespaceAction::NormalizeLineEndings
            )),
            format!("{family}\n{combining}")
        );
    }

    #[test]
    fn empty_input_is_neutral_for_every_action_and_width() {
        for action in WhitespaceAction::ALL {
            assert_eq!(evaluate(&request("", action)), WhitespaceEvaluation::Empty);
        }
        assert_eq!(
            evaluate(&request_with(
                "",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                0
            )),
            WhitespaceEvaluation::Empty
        );
    }

    #[test]
    fn tab_width_outside_one_through_eight_is_invalid_for_every_action() {
        for width in [0u8, 9, 255] {
            for action in WhitespaceAction::ALL {
                let evaluation =
                    evaluate(&request_with("x", action, WhitespaceLineEnding::Lf, width));
                assert!(!evaluation.is_valid_operation(), "{action:?} at {width}");
                assert!(evaluation.output().is_none());
                assert_eq!(evaluation.diagnostics().len(), 1);
                assert!(evaluation.diagnostics()[0].message.contains("1 through 8"));
            }
        }
        for width in [1u8, 8] {
            assert!(evaluate(&request_with(
                "x",
                WhitespaceAction::TabsToSpaces,
                WhitespaceLineEnding::Lf,
                width
            ))
            .is_valid_operation());
        }
    }

    #[test]
    fn snapshot_round_trips_exactly_without_reevaluation() {
        let request = request_with(
            "a\r\nb\rc\n",
            WhitespaceAction::NormalizeLineEndings,
            WhitespaceLineEnding::Crlf,
            8,
        );
        let evaluation = evaluate(&request);
        let snapshot =
            <Whitespace as Utility>::snapshot(&request, &evaluation).expect("settled snapshot");
        let encoded = serde_json::to_string(&snapshot).expect("snapshot serializes");
        let decoded: WhitespaceSnapshot =
            serde_json::from_str(&encoded).expect("snapshot deserializes");
        assert_eq!(decoded, snapshot);
        assert_eq!(decoded.restore().1, "a\r\nb\r\nc\r\n");

        let (restored_request, restored_evaluation) = <Whitespace as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.output(), Some("a\r\nb\r\nc\r\n"));
    }

    #[test]
    fn a_deliberate_no_op_still_snapshots() {
        let request = request("cafe", WhitespaceAction::EdgeTrim);
        let evaluation = evaluate(&request);
        let snapshot =
            <Whitespace as Utility>::snapshot(&request, &evaluation).expect("no-op records");
        assert_eq!(snapshot.restore().1, "cafe");
        assert_eq!(snapshot.restore().0, &request);
    }

    #[test]
    fn neutral_or_invalid_operations_never_snapshot() {
        let neutral = evaluate(&request("", WhitespaceAction::Dedent));
        assert!(<Whitespace as Utility>::snapshot(
            &request("", WhitespaceAction::Dedent),
            &neutral
        )
        .is_none());

        let invalid_request =
            request_with("x", WhitespaceAction::Dedent, WhitespaceLineEnding::Lf, 0);
        let invalid = evaluate(&invalid_request);
        assert!(<Whitespace as Utility>::snapshot(&invalid_request, &invalid).is_none());
    }

    #[test]
    fn every_action_snapshots_a_valid_operation() {
        let input = "  a\t b\n\n c  ";
        for action in WhitespaceAction::ALL {
            let request = request_with(input, action, WhitespaceLineEnding::Lf, 4);
            let evaluation = evaluate(&request);
            assert!(evaluation.is_valid_operation(), "{action:?}");
            let snapshot = <Whitespace as Utility>::snapshot(&request, &evaluation)
                .unwrap_or_else(|| panic!("{action:?} must snapshot"));
            assert_eq!(snapshot.output, evaluation.output().unwrap());
        }
    }
}
