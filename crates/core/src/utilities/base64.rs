//! The Base64 Utility's GPUI-independent request/result/snapshot contract.
//!
//! Encoding is over the exact UTF-8 bytes of the input. Decoding validates the
//! selected alphabet, rejects whitespace and misplaced padding, restores
//! omitted padding, and requires the decoded bytes to be valid UTF-8 text.

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Base64 Utility.
pub const BASE64_UTILITY_ID: &str = "base64";

/// Schema version of [`Base64Snapshot`].
pub const BASE64_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Base64Mode {
    Encode,
    Decode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Base64Alphabet {
    Standard,
    UrlSafe,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base64Request {
    pub input: String,
    pub mode: Base64Mode,
    pub alphabet: Base64Alphabet,
    pub padded: bool,
}

impl Base64Request {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            mode: Base64Mode::Encode,
            alphabet: Base64Alphabet::Standard,
            padded: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base64Evaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        output: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl Base64Evaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            Base64Evaluation::Valid { output } => Some(output),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Base64Evaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, Base64Evaluation::Valid { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base64Snapshot {
    pub request: Base64Request,
    pub output: String,
}

impl Base64Snapshot {
    pub fn restore(&self) -> (&Base64Request, &str) {
        (&self.request, &self.output)
    }
}

pub fn evaluate(request: &Base64Request) -> Base64Evaluation {
    if request.input.is_empty() {
        return Base64Evaluation::Empty;
    }
    match request.mode {
        Base64Mode::Encode => Base64Evaluation::Valid {
            output: encode(request.input.as_bytes(), request.alphabet, request.padded),
        },
        Base64Mode::Decode => match decode(&request.input, request.alphabet) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => Base64Evaluation::Valid { output: text },
                Err(_) => Base64Evaluation::Invalid {
                    diagnostics: vec![Diagnostic::error(
                        "The decoded bytes are not valid UTF-8 text.",
                    )],
                },
            },
            Err(message) => Base64Evaluation::Invalid {
                diagnostics: vec![Diagnostic::error(message)],
            },
        },
    }
}

/// Standard-alphabet, padded Base64 of raw bytes, shared by Utilities that need
/// to present a digest as Base64.
pub fn encode_standard(input: &[u8]) -> String {
    encode(input, Base64Alphabet::Standard, true)
}

fn encode(input: &[u8], alphabet: Base64Alphabet, padded: bool) -> String {
    let table = match alphabet {
        Base64Alphabet::Standard => STANDARD,
        Base64Alphabet::UrlSafe => URL_SAFE,
    };
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0] as u32;
        let second = *chunk.get(1).unwrap_or(&0) as u32;
        let third = *chunk.get(2).unwrap_or(&0) as u32;
        let block = (first << 16) | (second << 8) | third;
        output.push(table[((block >> 18) & 63) as usize] as char);
        output.push(table[((block >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            output.push(table[((block >> 6) & 63) as usize] as char);
        } else if padded {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(table[(block & 63) as usize] as char);
        } else if padded {
            output.push('=');
        }
    }
    output
}

fn decode(input: &str, alphabet: Base64Alphabet) -> Result<Vec<u8>, String> {
    let mut values = Vec::with_capacity(input.len());
    let mut padding = 0usize;
    let mut saw_padding = false;
    for character in input.chars() {
        if character.is_whitespace() {
            return Err("Input contains whitespace, which is not valid Base64.".to_owned());
        }
        if character == '=' {
            saw_padding = true;
            padding += 1;
            continue;
        }
        if saw_padding {
            return Err("Input contains data after its padding.".to_owned());
        }
        let Some(value) = value_of(character, alphabet) else {
            return Err(
                "Input contains characters outside the selected Base64 alphabet.".to_owned(),
            );
        };
        values.push(value);
    }

    let remainder = values.len() % 4;
    if remainder == 1 {
        return Err("Invalid Base64 length.".to_owned());
    }
    let expected_padding = match remainder {
        2 => 2,
        3 => 1,
        _ => 0,
    };
    if padding != 0 && padding != expected_padding {
        return Err("Invalid Base64 padding.".to_owned());
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
    Ok(output)
}

fn value_of(character: char, alphabet: Base64Alphabet) -> Option<u8> {
    let byte = character as u8;
    if character.len_utf8() != 1 {
        return None;
    }
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' if alphabet == Base64Alphabet::Standard => Some(62),
        b'/' if alphabet == Base64Alphabet::Standard => Some(63),
        b'-' if alphabet == Base64Alphabet::UrlSafe => Some(62),
        b'_' if alphabet == Base64Alphabet::UrlSafe => Some(63),
        _ => None,
    }
}

/// The Base64 Utility's identity for the shared [`Utility`] trait.
pub struct Base64;

impl Utility for Base64 {
    type Request = Base64Request;
    type Evaluation = Base64Evaluation;
    type Snapshot = Base64Snapshot;

    const ID: &'static str = BASE64_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = BASE64_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> Base64Evaluation {
        Base64Evaluation::Empty
    }

    fn evaluate(request: &Base64Request) -> Base64Evaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &Base64Evaluation) -> bool {
        matches!(evaluation, Base64Evaluation::Empty)
    }

    fn snapshot(request: &Base64Request, evaluation: &Base64Evaluation) -> Option<Base64Snapshot> {
        evaluation.output().map(|output| Base64Snapshot {
            request: request.clone(),
            output: output.to_owned(),
        })
    }

    fn restore(snapshot: &Base64Snapshot) -> (Base64Request, Base64Evaluation) {
        (
            snapshot.request.clone(),
            Base64Evaluation::Valid {
                output: snapshot.output.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_request(input: &str, alphabet: Base64Alphabet, padded: bool) -> Base64Request {
        Base64Request {
            input: input.to_owned(),
            mode: Base64Mode::Encode,
            alphabet,
            padded,
        }
    }

    fn decode_request(input: &str, alphabet: Base64Alphabet) -> Base64Request {
        Base64Request {
            input: input.to_owned(),
            mode: Base64Mode::Decode,
            alphabet,
            padded: true,
        }
    }

    fn output(request: &Base64Request) -> String {
        evaluate(request).output().expect("valid output").to_owned()
    }

    #[test]
    fn published_vectors_encode_and_decode() {
        for (plain, encoded) in [
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(
                output(&encode_request(plain, Base64Alphabet::Standard, true)),
                encoded
            );
            assert_eq!(
                output(&decode_request(encoded, Base64Alphabet::Standard)),
                plain
            );
        }
    }

    #[test]
    fn empty_input_is_neutral() {
        assert_eq!(
            evaluate(&encode_request("", Base64Alphabet::Standard, true)),
            Base64Evaluation::Empty
        );
    }

    #[test]
    fn url_safe_uses_dash_and_underscore() {
        let bytes = [0xfb_u8, 0xff];
        let standard = encode(&bytes, Base64Alphabet::Standard, true);
        assert_eq!(standard, "+/8=");
        assert_eq!(encode(&bytes, Base64Alphabet::UrlSafe, true), "-_8=");
        assert_eq!(decode(&standard, Base64Alphabet::Standard).unwrap(), bytes);
        assert_eq!(decode("-_8=", Base64Alphabet::UrlSafe).unwrap(), bytes);
        // The other alphabet's special characters are rejected.
        assert!(decode("-_8=", Base64Alphabet::Standard).is_err());
    }

    #[test]
    fn padding_is_optional_on_decode_and_configurable_on_encode() {
        assert_eq!(
            output(&encode_request("foobar", Base64Alphabet::Standard, false)),
            "Zm9vYmFy"
        );
        assert_eq!(
            output(&encode_request("foob", Base64Alphabet::Standard, false)),
            "Zm9vYg"
        );
        assert_eq!(
            output(&decode_request("Zm9vYg", Base64Alphabet::Standard)),
            "foob"
        );
        assert_eq!(
            output(&decode_request("Zm9vYg==", Base64Alphabet::Standard)),
            "foob"
        );
    }

    #[test]
    fn unicode_is_encoded_from_utf8_bytes() {
        assert_eq!(
            output(&encode_request("café", Base64Alphabet::Standard, true)),
            "Y2Fmw6k="
        );
        assert_eq!(
            output(&decode_request("Y2Fmw6k=", Base64Alphabet::Standard)),
            "café"
        );
    }

    #[test]
    fn invalid_input_produces_diagnostics_and_no_output() {
        for (input, alphabet) in [
            ("!!!!", Base64Alphabet::Standard),
            ("A", Base64Alphabet::Standard),
            ("Zg= =", Base64Alphabet::Standard),
            ("Zg==Zg==", Base64Alphabet::Standard),
            ("-_8=", Base64Alphabet::Standard),
        ] {
            let evaluation = evaluate(&decode_request(input, alphabet));
            assert!(!evaluation.is_valid_operation(), "{input} must be invalid");
            assert!(evaluation.output().is_none());
            assert_eq!(evaluation.diagnostics().len(), 1);
        }
    }

    #[test]
    fn decoded_non_utf8_bytes_are_rejected() {
        let evaluation = evaluate(&decode_request("/w==", Base64Alphabet::Standard));
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.diagnostics()[0].message.contains("UTF-8"));
    }

    #[test]
    fn snapshot_round_trips_without_reevaluation() {
        let request = encode_request("hello", Base64Alphabet::UrlSafe, false);
        let evaluation = evaluate(&request);
        let snapshot = <Base64 as Utility>::snapshot(&request, &evaluation).expect("snapshot");
        assert_eq!(snapshot.restore().1, "aGVsbG8");
        let (restored_request, restored_evaluation) = <Base64 as Utility>::restore(&snapshot);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.output(), Some("aGVsbG8"));
    }
}
