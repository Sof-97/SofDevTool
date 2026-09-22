//! The Hashes Utility's GPUI-independent request/result/snapshot contract.
//!
//! A digest is always taken over the exact UTF-8 bytes of the input text, with
//! no trimming or normalization. Choosing an algorithm and representation is a
//! deliberate action: the caller decides when to evaluate, so untouched input
//! stays neutral while an explicit Hash of empty input hashes zero bytes as one
//! valid operation. Files, HMAC and password hashing are out of scope.

use digest::Digest;
use md5::Md5;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Hashes Utility.
pub const HASHES_UTILITY_ID: &str = "hashes";

/// Schema version of [`HashesSnapshot`].
pub const HASHES_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// A supported digest. SHA-1 and MD5 are legacy and not for new designs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha256,
    Sha384,
    Sha512,
    Sha1,
    Md5,
}

impl HashAlgorithm {
    /// Every algorithm in display order.
    pub const ALL: [HashAlgorithm; 5] = [
        HashAlgorithm::Sha256,
        HashAlgorithm::Sha384,
        HashAlgorithm::Sha512,
        HashAlgorithm::Sha1,
        HashAlgorithm::Md5,
    ];

    /// The stable, human-facing name.
    pub fn label(self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha384 => "SHA-384",
            HashAlgorithm::Sha512 => "SHA-512",
            HashAlgorithm::Sha1 => "SHA-1 (Legacy)",
            HashAlgorithm::Md5 => "MD5 (Legacy)",
        }
    }

    /// True for algorithms retained only for compatibility.
    pub fn is_legacy(self) -> bool {
        matches!(self, HashAlgorithm::Sha1 | HashAlgorithm::Md5)
    }

    /// The legacy warning, or an empty string for a current algorithm.
    pub fn notice(self) -> &'static str {
        if self.is_legacy() {
            "Legacy algorithm — not for security, password hashing, authentication, or new designs."
        } else {
            ""
        }
    }
}

/// How a digest's bytes are presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashRepresentation {
    LowercaseHex,
    UppercaseHex,
    Base64,
}

impl HashRepresentation {
    /// Every representation in display order.
    pub const ALL: [HashRepresentation; 3] = [
        HashRepresentation::LowercaseHex,
        HashRepresentation::UppercaseHex,
        HashRepresentation::Base64,
    ];

    /// The stable, human-facing name.
    pub fn label(self) -> &'static str {
        match self {
            HashRepresentation::LowercaseHex => "Lowercase hex",
            HashRepresentation::UppercaseHex => "Uppercase hex",
            HashRepresentation::Base64 => "Base64",
        }
    }
}

/// The complete input for one digest operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashesRequest {
    pub input: String,
    pub algorithm: HashAlgorithm,
    pub representation: HashRepresentation,
}

impl HashesRequest {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            algorithm: HashAlgorithm::Sha256,
            representation: HashRepresentation::LowercaseHex,
        }
    }
}

/// The typed outcome shown to the user.
///
/// Hashing a valid Rust `String` cannot fail, so [`HashesEvaluation::Invalid`]
/// exists only to keep the shared contract uniform; evaluation never produces
/// it, and the presentation layer still renders its diagnostics if it appears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HashesEvaluation {
    /// Untouched input is neutral: no digest and no diagnostics.
    Empty,
    Valid {
        output: String,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl HashesEvaluation {
    pub fn output(&self) -> Option<&str> {
        match self {
            HashesEvaluation::Valid { output } => Some(output),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            HashesEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, HashesEvaluation::Valid { .. })
    }
}

/// The persisted form of one settled digest operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashesSnapshot {
    pub request: HashesRequest,
    pub output: String,
}

impl HashesSnapshot {
    pub fn restore(&self) -> (&HashesRequest, &str) {
        (&self.request, &self.output)
    }
}

/// Hashes the exact UTF-8 bytes of `request.input` at the requested algorithm
/// and representation. Empty input is a valid operation over zero bytes.
pub fn evaluate(request: &HashesRequest) -> HashesEvaluation {
    let digest = digest(request.algorithm, request.input.as_bytes());
    HashesEvaluation::Valid {
        output: render(&digest, request.representation),
    }
}

fn digest(algorithm: HashAlgorithm, bytes: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Sha256 => Sha256::digest(bytes).to_vec(),
        HashAlgorithm::Sha384 => Sha384::digest(bytes).to_vec(),
        HashAlgorithm::Sha512 => Sha512::digest(bytes).to_vec(),
        HashAlgorithm::Sha1 => Sha1::digest(bytes).to_vec(),
        HashAlgorithm::Md5 => Md5::digest(bytes).to_vec(),
    }
}

fn render(bytes: &[u8], representation: HashRepresentation) -> String {
    match representation {
        HashRepresentation::LowercaseHex => hex::encode(bytes),
        HashRepresentation::UppercaseHex => hex::encode_upper(bytes),
        HashRepresentation::Base64 => crate::utilities::base64::encode_standard(bytes),
    }
}

/// The Hashes Utility's identity for the shared [`Utility`] trait.
pub struct Hashes;

impl Utility for Hashes {
    type Request = HashesRequest;
    type Evaluation = HashesEvaluation;
    type Snapshot = HashesSnapshot;

    const ID: &'static str = HASHES_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = HASHES_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> HashesEvaluation {
        HashesEvaluation::Empty
    }

    fn evaluate(request: &HashesRequest) -> HashesEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &HashesEvaluation) -> bool {
        matches!(evaluation, HashesEvaluation::Empty)
    }

    fn snapshot(request: &HashesRequest, evaluation: &HashesEvaluation) -> Option<HashesSnapshot> {
        evaluation.output().map(|output| HashesSnapshot {
            request: request.clone(),
            output: output.to_owned(),
        })
    }

    fn restore(snapshot: &HashesSnapshot) -> (HashesRequest, HashesEvaluation) {
        (
            snapshot.request.clone(),
            HashesEvaluation::Valid {
                output: snapshot.output.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, SubmitOutcome};

    fn request(
        input: &str,
        algorithm: HashAlgorithm,
        representation: HashRepresentation,
    ) -> HashesRequest {
        HashesRequest {
            input: input.to_owned(),
            algorithm,
            representation,
        }
    }

    fn output(request: &HashesRequest) -> String {
        evaluate(request)
            .output()
            .expect("hashing always yields output")
            .to_owned()
    }

    fn hex_of(input: &str, algorithm: HashAlgorithm, representation: HashRepresentation) -> String {
        output(&request(input, algorithm, representation))
    }

    /// Published NIST vectors for SHA-256/384/512 over "abc", plus the
    /// empty-message SHA-256 vector.
    #[test]
    fn published_sha2_vectors_are_exact() {
        assert_eq!(
            hex_of(
                "abc",
                HashAlgorithm::Sha256,
                HashRepresentation::LowercaseHex
            ),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex_of(
                "abc",
                HashAlgorithm::Sha384,
                HashRepresentation::LowercaseHex
            ),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            hex_of(
                "abc",
                HashAlgorithm::Sha512,
                HashRepresentation::LowercaseHex
            ),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            hex_of("", HashAlgorithm::Sha256, HashRepresentation::LowercaseHex),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// Published RFC/AUTHORS vectors for SHA-1 and MD5, including empty input.
    #[test]
    fn published_legacy_vectors_are_exact() {
        assert_eq!(
            hex_of("abc", HashAlgorithm::Sha1, HashRepresentation::LowercaseHex),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex_of("", HashAlgorithm::Sha1, HashRepresentation::LowercaseHex),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        assert_eq!(
            hex_of("abc", HashAlgorithm::Md5, HashRepresentation::LowercaseHex),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hex_of("", HashAlgorithm::Md5, HashRepresentation::LowercaseHex),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
    }

    /// Multi-byte text is hashed as its exact UTF-8 bytes, not as code points
    /// or locale-encoded characters.
    #[test]
    fn unicode_is_hashed_from_exact_utf8_bytes() {
        let expected_cafe = "850f7dc43910ff890f8879c0ed26fe697c93a067ad93a7d50f466a7028a9bf4e";
        assert_eq!(
            hex_of(
                "café",
                HashAlgorithm::Sha256,
                HashRepresentation::LowercaseHex
            ),
            expected_cafe
        );
        assert_eq!(
            hex_of(
                "café",
                HashAlgorithm::Sha1,
                HashRepresentation::LowercaseHex
            ),
            "f424452a9673918c6f09b0cdd35b20be8e6ae7d7"
        );
        assert_eq!(
            hex_of("café", HashAlgorithm::Md5, HashRepresentation::LowercaseHex),
            "07117fe4a1ebd544965dc19573183da2"
        );
        // A four-byte emoji stays one byte sequence.
        assert_eq!(
            hex_of(
                "🦀",
                HashAlgorithm::Sha256,
                HashRepresentation::LowercaseHex
            ),
            "7224c588fa9887541bea6fc37a50363ce1c229547ba65c110095ad23b68c902d"
        );
        // No normalization is applied: precomposed and decomposed "café" are
        // different byte sequences and therefore different digests.
        assert_eq!(
            hex_of(
                "cafe\u{301}",
                HashAlgorithm::Sha256,
                HashRepresentation::LowercaseHex
            ),
            "81ef060bcd98adc7824eb5c1ada83c32491b16018e11e79f00ab9d09e04b015a"
        );
    }

    /// The same digest appears as lowercase/uppercase hex and Base64.
    #[test]
    fn representations_share_one_digest() {
        let lower = hex_of(
            "abc",
            HashAlgorithm::Sha256,
            HashRepresentation::LowercaseHex,
        );
        let upper = hex_of(
            "abc",
            HashAlgorithm::Sha256,
            HashRepresentation::UppercaseHex,
        );
        let base64 = hex_of("abc", HashAlgorithm::Sha256, HashRepresentation::Base64);

        assert_eq!(lower, lower.to_lowercase());
        assert_eq!(upper, lower.to_uppercase());
        assert_eq!(base64, "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=");
        // Round-tripping the hex bytes and re-encoding gives the Base64 form.
        let decoded = hex::decode(&lower).expect("lowercase hex");
        assert_eq!(crate::utilities::base64::encode_standard(&decoded), base64);
    }

    /// An explicit Hash of empty input is one valid operation over zero bytes,
    /// distinct from the neutral, untouched state.
    #[test]
    fn explicit_empty_input_is_a_valid_operation() {
        let evaluation = evaluate(&request(
            "",
            HashAlgorithm::Sha256,
            HashRepresentation::LowercaseHex,
        ));
        assert!(evaluation.is_valid_operation());
        assert_eq!(
            evaluation.output(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert!(evaluation.diagnostics().is_empty());
        assert_eq!(<Hashes as Utility>::neutral(), HashesEvaluation::Empty);
        assert!(<Hashes as Utility>::is_neutral(&HashesEvaluation::Empty));
    }

    /// Identical inputs always hash identically; a deliberate repeat is a new
    /// operation and records separately rather than being suppressed.
    #[test]
    fn deliberate_repeats_are_deterministic_and_record_separately() {
        let desired = request(
            "repeat me",
            HashAlgorithm::Md5,
            HashRepresentation::LowercaseHex,
        );
        assert_eq!(evaluate(&desired), evaluate(&desired));

        let mut session: Session<Hashes> = Session::new();
        for _ in 0..2 {
            session.clear();
            let SubmitOutcome::Scheduled(revision) = session.submit(desired.clone()) else {
                panic!("a cleared session always schedules");
            };
            session.resolve(revision);
            let snapshot = session.take_snapshot().expect("each repeat records once");
            assert_eq!(snapshot.output, "ea305cffa396d1f36caf007113a204e6");
        }
    }

    /// Neutral and invalid evaluations never produce a recordable snapshot.
    #[test]
    fn invalid_or_neutral_never_snapshots() {
        let desired = request(
            "data",
            HashAlgorithm::Sha256,
            HashRepresentation::LowercaseHex,
        );
        assert_eq!(
            <Hashes as Utility>::snapshot(&desired, &HashesEvaluation::Empty),
            None
        );
        let invalid = HashesEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error("not hashed")],
        };
        assert!(!invalid.is_valid_operation());
        assert_eq!(invalid.output(), None);
        assert_eq!(invalid.diagnostics().len(), 1);
        assert_eq!(<Hashes as Utility>::snapshot(&desired, &invalid), None);
        // A session with no request can never snapshot.
        let mut session: Session<Hashes> = Session::new();
        assert!(session.take_snapshot().is_none());
    }

    /// A snapshot serializes, deserializes and restores exactly, without
    /// reevaluating the request.
    #[test]
    fn snapshot_round_trips_exactly() {
        let desired = request("café", HashAlgorithm::Sha512, HashRepresentation::Base64);
        let evaluation = evaluate(&desired);
        let snapshot = <Hashes as Utility>::snapshot(&desired, &evaluation).expect("snapshot");

        let json = serde_json::to_string(&snapshot).expect("serialize");
        let decoded: HashesSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, snapshot);

        let (restored_request, restored_evaluation) = <Hashes as Utility>::restore(&decoded);
        assert_eq!(restored_request, desired);
        assert_eq!(restored_evaluation.output(), evaluation.output());
        assert_eq!(decoded.restore().1, evaluation.output().unwrap());
    }

    /// Legacy algorithms are labelled and warned about; current ones are not.
    #[test]
    fn legacy_algorithms_are_labelled() {
        assert!(HashAlgorithm::Sha1.is_legacy());
        assert!(HashAlgorithm::Md5.is_legacy());
        assert!(!HashAlgorithm::Sha256.is_legacy());
        assert!(!HashAlgorithm::Sha384.is_legacy());
        assert!(!HashAlgorithm::Sha512.is_legacy());
        assert!(HashAlgorithm::Sha1.notice().contains("Legacy"));
        assert_eq!(HashAlgorithm::Sha256.notice(), "");
        assert!(HashAlgorithm::Sha1.label().contains("Legacy"));
        assert_eq!(HashAlgorithm::ALL.len(), 5);
        assert_eq!(HashRepresentation::ALL.len(), 3);
    }

    /// The catalog identity and schema version are stable.
    #[test]
    fn identity_and_schema_version_are_stable() {
        assert_eq!(<Hashes as Utility>::ID, "hashes");
        assert_eq!(<Hashes as Utility>::SNAPSHOT_VERSION, 1);
    }
}
