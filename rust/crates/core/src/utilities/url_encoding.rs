//! The URL Encoding Utility's GPUI-independent request/result/snapshot contract.
//!
//! Encoding is over the exact UTF-8 bytes of the input. Two separately named
//! modes preserve only their contract-defined characters:
//!
//! * Path Segment keeps the RFC 3986 unreserved set plus `pchar`, although `/`
//!   itself is still encoded because a segment must not introduce a separator.
//! * Query Value keeps only the unreserved set, so query delimiters such as `&`
//!   and `=` are encoded.
//!
//! Spaces always become `%20` and literal `%` is encoded to `%25`. Percent bytes
//! are uppercase. Decoding is a strict UTF-8 percent decode: malformed triplets
//! and non-UTF-8 byte sequences are rejected, and a literal `+` stays a plus.
//! This is not `application/x-www-form-urlencoded` and not URL-safe Base64.

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the URL Encoding Utility.
pub const URL_ENCODING_UTILITY_ID: &str = "url-encoding";

/// Schema version of [`UrlEncodingSnapshot`].
pub const URL_ENCODING_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UrlEncodingDirection {
    Encode,
    Decode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlEncodingMode {
    PathSegment,
    QueryValue,
}

/// The complete, strongly typed input for one URL Encoding operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlEncodingRequest {
    pub input: String,
    pub direction: UrlEncodingDirection,
    pub mode: UrlEncodingMode,
}

impl UrlEncodingRequest {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            direction: UrlEncodingDirection::Encode,
            mode: UrlEncodingMode::PathSegment,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlEncodingEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl UrlEncodingEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            UrlEncodingEvaluation::Valid { output } => Some(output),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            UrlEncodingEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, UrlEncodingEvaluation::Valid { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlEncodingSnapshot {
    pub request: UrlEncodingRequest,
    pub output: String,
}

impl UrlEncodingSnapshot {
    pub fn restore(&self) -> (&UrlEncodingRequest, &str) {
        (&self.request, &self.output)
    }
}

pub fn evaluate(request: &UrlEncodingRequest) -> UrlEncodingEvaluation {
    if request.input.is_empty() {
        return UrlEncodingEvaluation::Empty;
    }
    match request.direction {
        UrlEncodingDirection::Encode => UrlEncodingEvaluation::Valid {
            output: encode(&request.input, request.mode),
        },
        UrlEncodingDirection::Decode => match decode(&request.input) {
            Ok(text) => UrlEncodingEvaluation::Valid { output: text },
            Err(message) => UrlEncodingEvaluation::Invalid {
                diagnostics: vec![Diagnostic::error(message)],
            },
        },
    }
}

fn encode(input: &str, mode: UrlEncodingMode) -> String {
    const HEXADECIMAL: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        let preserved =
            is_unreserved(byte) || (mode == UrlEncodingMode::PathSegment && is_path_pchar(byte));
        if preserved {
            output.push(byte as char);
        } else {
            output.push('%');
            output.push(HEXADECIMAL[(byte >> 4) as usize] as char);
            output.push(HEXADECIMAL[(byte & 0x0F) as usize] as char);
        }
    }
    output
}

fn is_unreserved(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
    )
}

fn is_path_pchar(byte: u8) -> bool {
    matches!(
        byte,
        b':' | b'@' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

fn decode(input: &str) -> Result<String, String> {
    let source = input.as_bytes();
    let mut decoded = Vec::with_capacity(source.len());
    let mut index = 0;
    while index < source.len() {
        if source[index] != b'%' {
            decoded.push(source[index]);
            index += 1;
            continue;
        }
        if index + 2 >= source.len() {
            return Err(
                "Percent encoding contains an incomplete or invalid %HH triplet.".to_owned(),
            );
        }
        let (Some(high), Some(low)) = (
            hexadecimal_value(source[index + 1]),
            hexadecimal_value(source[index + 2]),
        ) else {
            return Err(
                "Percent encoding contains an incomplete or invalid %HH triplet.".to_owned(),
            );
        };
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded).map_err(|_| "Decoded bytes are not valid UTF-8 text.".to_owned())
}

fn hexadecimal_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// The URL Encoding Utility's identity for the shared [`Utility`] trait.
pub struct UrlEncoding;

impl Utility for UrlEncoding {
    type Request = UrlEncodingRequest;
    type Evaluation = UrlEncodingEvaluation;
    type Snapshot = UrlEncodingSnapshot;

    const ID: &'static str = URL_ENCODING_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = URL_ENCODING_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> UrlEncodingEvaluation {
        UrlEncodingEvaluation::Empty
    }

    fn evaluate(request: &UrlEncodingRequest) -> UrlEncodingEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &UrlEncodingEvaluation) -> bool {
        matches!(evaluation, UrlEncodingEvaluation::Empty)
    }

    fn snapshot(
        request: &UrlEncodingRequest,
        evaluation: &UrlEncodingEvaluation,
    ) -> Option<UrlEncodingSnapshot> {
        evaluation.output().map(|output| UrlEncodingSnapshot {
            request: request.clone(),
            output: output.to_owned(),
        })
    }

    fn restore(snapshot: &UrlEncodingSnapshot) -> (UrlEncodingRequest, UrlEncodingEvaluation) {
        (
            snapshot.request.clone(),
            UrlEncodingEvaluation::Valid {
                output: snapshot.output.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_request(input: &str, mode: UrlEncodingMode) -> UrlEncodingRequest {
        UrlEncodingRequest {
            input: input.to_owned(),
            direction: UrlEncodingDirection::Encode,
            mode,
        }
    }

    fn decode_request(input: &str) -> UrlEncodingRequest {
        UrlEncodingRequest {
            input: input.to_owned(),
            direction: UrlEncodingDirection::Decode,
            mode: UrlEncodingMode::PathSegment,
        }
    }

    fn output(request: &UrlEncodingRequest) -> String {
        evaluate(request).output().expect("valid output").to_owned()
    }

    #[test]
    fn path_segment_preserves_unreserved_and_pchar() {
        // Hand-derived from RFC 3986: unreserved plus ':@!$&'()*+,;=' stay,
        // while '/', '?', '#', '[' and ']' are percent-encoded.
        assert_eq!(
            output(&encode_request("AZaz09-._~", UrlEncodingMode::PathSegment)),
            "AZaz09-._~"
        );
        assert_eq!(
            output(&encode_request(
                ":@!$&'()*+,;=",
                UrlEncodingMode::PathSegment
            )),
            ":@!$&'()*+,;="
        );
        assert_eq!(
            output(&encode_request("a/b?c#d[e]", UrlEncodingMode::PathSegment)),
            "a%2Fb%3Fc%23d%5Be%5D"
        );
        assert_eq!(
            output(&encode_request("%", UrlEncodingMode::PathSegment)),
            "%25"
        );
    }

    #[test]
    fn query_value_encodes_reserved_delimiters() {
        assert_eq!(
            output(&encode_request(
                ":@!$&'()*+,;=",
                UrlEncodingMode::QueryValue
            )),
            "%3A%40%21%24%26%27%28%29%2A%2B%2C%3B%3D"
        );
        assert_eq!(
            output(&encode_request("a=b&c=d", UrlEncodingMode::QueryValue)),
            "a%3Db%26c%3Dd"
        );
        assert_eq!(
            output(&encode_request("q?x/y#z", UrlEncodingMode::QueryValue)),
            "q%3Fx%2Fy%23z"
        );
        assert_eq!(
            output(&encode_request("100%", UrlEncodingMode::QueryValue)),
            "100%25"
        );
    }

    #[test]
    fn spaces_become_percent_twenty_and_plus_is_mode_dependent() {
        assert_eq!(
            output(&encode_request("a b", UrlEncodingMode::PathSegment)),
            "a%20b"
        );
        assert_eq!(
            output(&encode_request("a b", UrlEncodingMode::QueryValue)),
            "a%20b"
        );
        // A literal plus is a path `pchar` but a query delimiter.
        assert_eq!(
            output(&encode_request("a+b", UrlEncodingMode::PathSegment)),
            "a+b"
        );
        assert_eq!(
            output(&encode_request("a+b", UrlEncodingMode::QueryValue)),
            "a%2Bb"
        );
    }

    #[test]
    fn utf8_bytes_are_uppercase_percent_encoded() {
        assert_eq!(
            output(&encode_request("café", UrlEncodingMode::PathSegment)),
            "caf%C3%A9"
        );
        // `e` plus U+0301 COMBINING ACUTE ACCENT.
        assert_eq!(
            output(&encode_request("e\u{0301}", UrlEncodingMode::PathSegment)),
            "e%CC%81"
        );
        assert_eq!(
            output(&encode_request("日本語", UrlEncodingMode::PathSegment)),
            "%E6%97%A5%E6%9C%AC%E8%AA%9E"
        );
        assert_eq!(
            output(&encode_request("😀", UrlEncodingMode::PathSegment)),
            "%F0%9F%98%80"
        );
    }

    #[test]
    fn complex_emoji_sequence_round_trips_through_utf8_bytes() {
        // Man + ZWJ + woman + ZWJ + girl + ZWJ + boy.
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
        let encoded = "%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D\
                       %F0%9F%91%A7%E2%80%8D%F0%9F%91%A6";
        assert_eq!(
            output(&encode_request(family, UrlEncodingMode::PathSegment)),
            encoded
        );
        assert_eq!(output(&decode_request(encoded)), family);
    }

    #[test]
    fn decode_is_case_insensitive_and_never_treats_plus_as_space() {
        assert_eq!(output(&decode_request("a%20b")), "a b");
        assert_eq!(output(&decode_request("a+b")), "a+b");
        assert_eq!(output(&decode_request("+")), "+");
        assert_eq!(output(&decode_request("%2f%2F")), "//");
        assert_eq!(output(&decode_request("%C3%A9")), "é");
        assert_eq!(output(&decode_request("%c3%a9")), "é");
    }

    #[test]
    fn malformed_or_non_utf8_triplets_are_rejected() {
        for input in ["%", "%2", "%GG", "%2G", "abc%", "100%"] {
            let evaluation = evaluate(&decode_request(input));
            assert!(!evaluation.is_valid_operation(), "{input} must be invalid");
            assert!(evaluation.output().is_none());
            assert_eq!(evaluation.diagnostics().len(), 1);
        }
        let evaluation = evaluate(&decode_request("%FF"));
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.output().is_none());
        assert!(evaluation.diagnostics()[0].message.contains("UTF-8"));
    }

    #[test]
    fn empty_input_is_neutral_in_both_directions() {
        assert_eq!(
            evaluate(&encode_request("", UrlEncodingMode::PathSegment)),
            UrlEncodingEvaluation::Empty
        );
        assert_eq!(evaluate(&decode_request("")), UrlEncodingEvaluation::Empty);
    }

    #[test]
    fn snapshot_round_trips_without_reevaluation() {
        let request = encode_request("café", UrlEncodingMode::QueryValue);
        let evaluation = evaluate(&request);
        let snapshot = <UrlEncoding as Utility>::snapshot(&request, &evaluation).expect("snapshot");
        assert_eq!(snapshot.restore().1, "caf%C3%A9");
        let (restored_request, restored_evaluation) = <UrlEncoding as Utility>::restore(&snapshot);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.output(), Some("caf%C3%A9"));
    }

    #[test]
    fn invalid_operations_never_snapshot() {
        let request = decode_request("%GG");
        let evaluation = evaluate(&request);
        assert!(<UrlEncoding as Utility>::snapshot(&request, &evaluation).is_none());
        assert!(<UrlEncoding as Utility>::is_neutral(&UrlEncoding::neutral()));
    }

    #[test]
    fn session_rejects_stale_revisions_and_snapshots_once() {
        use crate::session::{Session, SubmitOutcome};

        let mut session = Session::<UrlEncoding>::new();
        let SubmitOutcome::Scheduled(first) =
            session.submit(encode_request("a b", UrlEncodingMode::PathSegment))
        else {
            panic!("expected a scheduled revision");
        };
        let SubmitOutcome::Scheduled(second) =
            session.submit(encode_request("a/b", UrlEncodingMode::PathSegment))
        else {
            panic!("expected a scheduled revision");
        };
        assert!(
            session.resolve(first).is_none(),
            "stale revision must not settle"
        );
        let settled = session.resolve(second).expect("current revision settles");
        assert_eq!(settled.output(), Some("a%2Fb"));

        let snapshot = session.take_snapshot().expect("settled snapshot");
        assert_eq!(snapshot.output, "a%2Fb");
        assert!(
            session.take_snapshot().is_none(),
            "an operation records at most once"
        );
    }
}
