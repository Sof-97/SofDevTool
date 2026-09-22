//! The JWT Decoder Utility's GPUI-independent request/result/snapshot contract.
//!
//! A JWT is exactly three dot-separated Base64URL segments. This Utility
//! decodes the header and payload segments as UTF-8 JSON and keeps the
//! signature opaque: it is shown as text and never verified. There is no key
//! input, no trust decision and no claim interpretation. Each failure stage
//! (segment count, Base64URL, UTF-8, JSON) produces its own diagnostic.

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the JWT Decoder Utility.
pub const JWT_UTILITY_ID: &str = "jwt-decoder";

/// Schema version of [`JwtSnapshot`].
pub const JWT_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// The prominent disclosure every JWT surface must show.
pub const JWT_NO_VERIFICATION_NOTICE: &str =
    "Decoding only: this does not verify the signature, interpret claims, judge validity, \
     establish trust, or determine fitness for use.";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwtRequest {
    pub input: String,
}

impl JwtRequest {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JwtEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        header: String,
        payload: String,
        signature: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl JwtEvaluation {
    pub fn header(&self) -> Option<&str> {
        match self {
            JwtEvaluation::Valid { header, .. } => Some(header),
            _ => None,
        }
    }

    pub fn payload(&self) -> Option<&str> {
        match self {
            JwtEvaluation::Valid { payload, .. } => Some(payload),
            _ => None,
        }
    }

    pub fn signature(&self) -> Option<&str> {
        match self {
            JwtEvaluation::Valid { signature, .. } => Some(signature),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            JwtEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, JwtEvaluation::Valid { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwtSnapshot {
    pub request: JwtRequest,
    pub header: String,
    pub payload: String,
    pub signature: String,
}

impl JwtSnapshot {
    pub fn restore(&self) -> (&JwtRequest, &str, &str, &str) {
        (&self.request, &self.header, &self.payload, &self.signature)
    }
}

pub fn evaluate(request: &JwtRequest) -> JwtEvaluation {
    if request.input.trim().is_empty() {
        return JwtEvaluation::Empty;
    }

    let segments: Vec<&str> = request.input.split('.').collect();
    if segments.len() != 3 {
        return JwtEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(format!(
                "JWT must contain exactly three dot-separated segments; found {}.",
                segments.len()
            ))],
        };
    }

    let header = match decode_json_segment(segments[0], "header") {
        Ok(header) => header,
        Err(diagnostic) => {
            return JwtEvaluation::Invalid {
                diagnostics: vec![diagnostic],
            }
        }
    };
    let payload = match decode_json_segment(segments[1], "payload") {
        Ok(payload) => payload,
        Err(diagnostic) => {
            return JwtEvaluation::Invalid {
                diagnostics: vec![diagnostic],
            }
        }
    };

    JwtEvaluation::Valid {
        header,
        payload,
        // The signature is opaque: it is preserved verbatim, never decoded or
        // verified.
        signature: segments[2].to_owned(),
    }
}

/// Recursively rebuilds objects with their keys in sorted order, so the
/// pretty-printed output is deterministic regardless of `serde_json`'s
/// `preserve_order` feature (which other workspace crates enable).
fn sorted_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), sorted_value(&map[key]));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sorted_value).collect())
        }
        other => other.clone(),
    }
}

fn decode_json_segment(segment: &str, name: &str) -> Result<String, Diagnostic> {
    let bytes = decode_base64url(segment)
        .ok_or_else(|| Diagnostic::error(format!("The {name} segment is not valid Base64URL.")))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| Diagnostic::error(format!("The {name} segment is not valid UTF-8.")))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Diagnostic::error(format!("The {name} segment is not valid JSON.")))?;
    if !(value.is_object() || value.is_array()) {
        return Err(Diagnostic::error(format!(
            "The {name} segment is not valid JSON."
        )));
    }
    Ok(serde_json::to_string_pretty(&sorted_value(&value))
        .expect("a parsed JSON value always pretty-prints"))
}

/// Decodes a Base64URL segment. Padding is optional and, when present, must be
/// correct. Returns `None` for any character outside the URL-safe alphabet or
/// any length/padding combination that cannot represent whole bytes.
fn decode_base64url(segment: &str) -> Option<Vec<u8>> {
    if segment.is_empty() {
        return None;
    }
    let mut values = Vec::with_capacity(segment.len());
    let mut padding = 0usize;
    let mut saw_padding = false;
    for character in segment.chars() {
        if character == '=' {
            saw_padding = true;
            padding += 1;
            continue;
        }
        if saw_padding {
            return None;
        }
        let value = match character {
            'A'..='Z' => character as u8 - b'A',
            'a'..='z' => character as u8 - b'a' + 26,
            '0'..='9' => character as u8 - b'0' + 52,
            '-' => 62,
            '_' => 63,
            _ => return None,
        };
        values.push(value);
    }

    let remainder = values.len() % 4;
    if remainder == 1 {
        return None;
    }
    let expected_padding = match remainder {
        2 => 2,
        3 => 1,
        _ => 0,
    };
    if padding != 0 && padding != expected_padding {
        return None;
    }

    let mut output = Vec::with_capacity(values.len() / 4 * 3);
    for chunk in values.chunks(4) {
        let first = chunk[0] as u32;
        let second = *chunk.get(1).unwrap_or(&0) as u32;
        let third = *chunk.get(2).unwrap_or(&0) as u32;
        let fourth = *chunk.get(3).unwrap_or(&0) as u32;
        let block = (first << 18) | (second << 12) | (third << 6) | fourth;
        output.push((block >> 16) as u8);
        if chunk.len() > 2 {
            output.push((block >> 8) as u8);
        }
        if chunk.len() > 3 {
            output.push(block as u8);
        }
    }
    Some(output)
}

/// The JWT Decoder Utility's identity for the shared [`Utility`] trait.
pub struct Jwt;

impl Utility for Jwt {
    type Request = JwtRequest;
    type Evaluation = JwtEvaluation;
    type Snapshot = JwtSnapshot;

    const ID: &'static str = JWT_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = JWT_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> JwtEvaluation {
        JwtEvaluation::Empty
    }

    fn evaluate(request: &JwtRequest) -> JwtEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &JwtEvaluation) -> bool {
        matches!(evaluation, JwtEvaluation::Empty)
    }

    fn snapshot(request: &JwtRequest, evaluation: &JwtEvaluation) -> Option<JwtSnapshot> {
        match evaluation {
            JwtEvaluation::Valid {
                header,
                payload,
                signature,
            } => Some(JwtSnapshot {
                request: request.clone(),
                header: header.clone(),
                payload: payload.clone(),
                signature: signature.clone(),
            }),
            _ => None,
        }
    }

    fn restore(snapshot: &JwtSnapshot) -> (JwtRequest, JwtEvaluation) {
        (
            snapshot.request.clone(),
            JwtEvaluation::Valid {
                header: snapshot.header.clone(),
                payload: snapshot.payload.clone(),
                signature: snapshot.signature.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE64URL: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    /// The canonical jwt.io HS256 example. Its signature is intentionally not
    /// verified anywhere in this Utility.
    const PUBLISHED_TOKEN: &str = concat!(
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ".eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYWRtaW4iOnRydWUsImlhdCI6MTUxNjIzOTAyMn0",
        ".SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
    );

    fn request(input: &str) -> JwtRequest {
        JwtRequest::new(input)
    }

    fn evaluation(input: &str) -> JwtEvaluation {
        evaluate(&request(input))
    }

    fn b64url(bytes: &[u8]) -> String {
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let second = *chunk.get(1).unwrap_or(&0) as u32;
            let third = *chunk.get(2).unwrap_or(&0) as u32;
            let block = ((chunk[0] as u32) << 16) | (second << 8) | third;
            output.push(BASE64URL[((block >> 18) & 63) as usize] as char);
            output.push(BASE64URL[((block >> 12) & 63) as usize] as char);
            if chunk.len() > 1 {
                output.push(BASE64URL[((block >> 6) & 63) as usize] as char);
            }
            if chunk.len() > 2 {
                output.push(BASE64URL[(block & 63) as usize] as char);
            }
        }
        output
    }

    #[test]
    fn published_token_decodes_header_and_payload_without_verifying() {
        let evaluation = evaluation(PUBLISHED_TOKEN);
        assert!(evaluation.is_valid_operation());
        assert_eq!(
            evaluation.header(),
            Some("{\n  \"alg\": \"HS256\",\n  \"typ\": \"JWT\"\n}")
        );
        assert_eq!(
            evaluation.payload(),
            Some(
                "{\n  \"admin\": true,\n  \"iat\": 1516239022,\n  \"name\": \"John Doe\",\n  \"sub\": \"1234567890\"\n}"
            )
        );
        // The signature is preserved verbatim, including its URL-safe bytes.
        assert_eq!(
            evaluation.signature(),
            Some("SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c")
        );
    }

    #[test]
    fn unicode_payload_is_decoded_from_utf8_bytes() {
        let payload = serde_json::json!({
            "name": "café 👩🏽‍💻",
            "roles": ["ünïcode"],
        });
        let token = format!(
            "{}.{}.{}",
            b64url(br#"{"alg":"none","typ":"JWT"}"#),
            b64url(payload.to_string().as_bytes()),
            "opaque-signature"
        );

        let evaluation = evaluation(&token);

        assert!(evaluation.is_valid_operation());
        let payload = evaluation.payload().expect("decoded payload");
        assert!(payload.contains("café 👩🏽‍💻"), "got: {payload}");
        assert!(payload.contains("ünïcode"), "got: {payload}");
        assert_eq!(evaluation.signature(), Some("opaque-signature"));
    }

    #[test]
    fn base64url_padding_is_optional_but_must_be_correct() {
        let padded = "eyJhIjoxfQ==.eyJhIjoxfQ==.sig";
        let unpadded = "eyJhIjoxfQ.eyJhIjoxfQ.sig";
        assert_eq!(evaluation(padded), evaluation(unpadded));
        assert!(evaluation(padded).is_valid_operation());
        // Length 1 modulo 4 cannot represent whole bytes.
        assert!(!evaluation("eyJhIjoxfQA.e30.sig").is_valid_operation());
    }

    #[test]
    fn segment_count_diagnostics_report_the_exact_count() {
        for (input, found) in [("only.two", 2), ("a.b.c.d", 4), ("single", 1)] {
            let evaluation = evaluation(input);
            assert!(!evaluation.is_valid_operation());
            assert_eq!(evaluation.diagnostics().len(), 1);
            assert!(
                evaluation.diagnostics()[0]
                    .message
                    .contains(&format!("found {found}.")),
                "got: {}",
                evaluation.diagnostics()[0].message
            );
        }
    }

    #[test]
    fn invalid_base64url_identifies_the_segment() {
        for (input, name) in [
            ("*.e30.signature", "header"),
            ("e30.*.signature", "payload"),
            (".e30.signature", "header"),
        ] {
            let evaluation = evaluation(input);
            assert!(!evaluation.is_valid_operation());
            assert_eq!(evaluation.diagnostics().len(), 1);
            let message = &evaluation.diagnostics()[0].message;
            assert!(
                message.contains(name) && message.contains("not valid Base64URL"),
                "got: {message}"
            );
        }
    }

    #[test]
    fn invalid_json_identifies_the_segment() {
        let not_json = b64url(b"not json");
        for (input, name) in [
            (format!("{not_json}.e30.signature"), "header"),
            (format!("e30.{not_json}.signature"), "payload"),
        ] {
            let evaluation = evaluation(&input);
            assert!(!evaluation.is_valid_operation());
            assert_eq!(evaluation.diagnostics().len(), 1);
            let message = &evaluation.diagnostics()[0].message;
            assert!(
                message.contains(name) && message.contains("not valid JSON"),
                "got: {message}"
            );
        }
    }

    #[test]
    fn non_utf8_bytes_are_reported_as_utf8_failure() {
        let evaluation = evaluation(&format!("{}.e30.signature", b64url(&[0xff])));
        assert!(!evaluation.is_valid_operation());
        assert_eq!(evaluation.diagnostics().len(), 1);
        let message = &evaluation.diagnostics()[0].message;
        assert!(
            message.contains("header") && message.contains("not valid UTF-8"),
            "got: {message}"
        );
    }

    #[test]
    fn empty_or_whitespace_input_is_neutral() {
        for input in ["", "   ", "\n\t"] {
            let evaluation = evaluation(input);
            assert_eq!(evaluation, JwtEvaluation::Empty);
            assert!(<Jwt as Utility>::is_neutral(&evaluation));
            assert!(<Jwt as Utility>::snapshot(&request(input), &evaluation).is_none());
        }
    }

    #[test]
    fn invalid_operations_never_snapshot() {
        for input in ["only.two", "*.e30.sig", "e30.*.sig"] {
            let request = request(input);
            let evaluation = evaluate(&request);
            assert!(!evaluation.is_valid_operation());
            assert!(<Jwt as Utility>::snapshot(&request, &evaluation).is_none());
        }
    }

    #[test]
    fn snapshot_round_trips_exactly_without_reevaluation() {
        let request = request(PUBLISHED_TOKEN);
        let evaluation = evaluate(&request);
        let snapshot =
            <Jwt as Utility>::snapshot(&request, &evaluation).expect("a valid operation snapshots");

        let encoded = serde_json::to_string(&snapshot).expect("snapshot serializes");
        let decoded: JwtSnapshot = serde_json::from_str(&encoded).expect("snapshot deserializes");
        assert_eq!(decoded, snapshot);
        assert_eq!(decoded.restore().0, &request);

        let (restored_request, restored_evaluation) = <Jwt as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation, evaluation);
    }
}
