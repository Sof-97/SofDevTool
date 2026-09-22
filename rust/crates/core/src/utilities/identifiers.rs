//! The Identifier Generator Utility's GPUI-independent request/result/snapshot
//! contract for UUID generation, inspection, normalization and options.
//!
//! Identifier *format* is a separate dimension from the UUID *version*: ULID
//! and KSUID are sibling formats beside UUID, not UUID versions, and are added
//! by a later ticket. Only [`IdentifierFormat::Uuid`] is implemented here.
//!
//! Generation is deliberately non-deterministic. The production
//! [`Utility::evaluate`] reads the system clock and cryptographic randomness,
//! while [`evaluate_with_source`] accepts an injected [`IdentifierSource`] so
//! tests are reproducible without weakening the production path. Snapshots
//! capture the generated values verbatim, so restoring a recorded operation
//! never regenerates and never reads a clock or random source.

use digest::Digest;
use md5::Md5;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha1::Sha1;

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Identifier Generator Utility.
pub const IDENTIFIERS_UTILITY_ID: &str = "identifiers";

/// Schema version of [`IdentifiersSnapshot`].
pub const IDENTIFIERS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Maximum generated batch size, preserving the existing Swift contract.
pub const MAXIMUM_GENERATED_COUNT: u32 = 1_000;

/// 100-nanosecond intervals between the Gregorian (1582-10-15) and Unix epochs.
const GREGORIAN_UNIX_TICKS: u64 = 122_192_928_000_000_000;

/// The published DNS namespace, the baseline's default.
pub const DEFAULT_NAMESPACE: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

/// The identifier format family. UUID is the only format in this ticket; ULID
/// and KSUID join it as sibling variants in the following ticket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierFormat {
    /// RFC 9562 universally unique identifiers.
    Uuid,
}

impl IdentifierFormat {
    /// Declaration order, used for stable control order.
    pub const ALL: [IdentifierFormat; 1] = [IdentifierFormat::Uuid];

    /// Human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            IdentifierFormat::Uuid => "UUID",
        }
    }

    /// Stable index in [`IdentifierFormat::ALL`].
    pub const fn index(self) -> usize {
        match self {
            IdentifierFormat::Uuid => 0,
        }
    }
}

/// The supported UUID versions. Version 8 is intentionally absent: the Utility
/// generates only the versions listed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UuidVersion {
    V1,
    V3,
    V4,
    V5,
    V6,
    V7,
}

impl UuidVersion {
    /// Declaration order, used for stable control order and focus indexing.
    pub const ALL: [UuidVersion; 6] = [
        UuidVersion::V1,
        UuidVersion::V3,
        UuidVersion::V4,
        UuidVersion::V5,
        UuidVersion::V6,
        UuidVersion::V7,
    ];

    /// The version nibble stored in the UUID.
    pub const fn number(self) -> u8 {
        match self {
            UuidVersion::V1 => 1,
            UuidVersion::V3 => 3,
            UuidVersion::V4 => 4,
            UuidVersion::V5 => 5,
            UuidVersion::V6 => 6,
            UuidVersion::V7 => 7,
        }
    }

    /// Human-readable label, matching the baseline.
    pub const fn label(self) -> &'static str {
        match self {
            UuidVersion::V1 => "v1",
            UuidVersion::V3 => "v3",
            UuidVersion::V4 => "v4",
            UuidVersion::V5 => "v5",
            UuidVersion::V6 => "v6",
            UuidVersion::V7 => "v7",
        }
    }

    /// Stable index in [`UuidVersion::ALL`].
    pub const fn index(self) -> usize {
        match self {
            UuidVersion::V1 => 0,
            UuidVersion::V3 => 1,
            UuidVersion::V4 => 2,
            UuidVersion::V5 => 3,
            UuidVersion::V6 => 4,
            UuidVersion::V7 => 5,
        }
    }

    /// True when this version requires a namespace UUID and a name.
    pub const fn requires_namespace(self) -> bool {
        matches!(self, UuidVersion::V3 | UuidVersion::V5)
    }
}

/// What an explicit action does: produce a fresh batch, or inspect one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierAction {
    Generate,
    Inspect,
}

/// The complete, strongly typed input for one evaluation.
///
/// `generation` is a monotonic nonce bumped by every explicit Generate or
/// Validate action. It is part of request identity, so a deliberate repeated
/// action is a new revision that records separately instead of being deduped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiersRequest {
    pub action: IdentifierAction,
    pub format: IdentifierFormat,
    pub version: UuidVersion,
    pub count: u32,
    pub uppercase: bool,
    pub hyphenated: bool,
    pub namespace: String,
    pub name: String,
    pub input: String,
    pub generation: u64,
}

impl Default for IdentifiersRequest {
    /// The baseline default: one lowercase, hyphenated UUID v4.
    fn default() -> Self {
        Self {
            action: IdentifierAction::Generate,
            format: IdentifierFormat::Uuid,
            version: UuidVersion::V4,
            count: 1,
            uppercase: false,
            hyphenated: true,
            namespace: DEFAULT_NAMESPACE.to_owned(),
            name: String::new(),
            input: String::new(),
            generation: 0,
        }
    }
}

impl IdentifiersRequest {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The typed outcome shown to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentifiersEvaluation {
    /// No action has run yet: no output and no diagnostics.
    Empty,
    Valid {
        values: Vec<String>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl IdentifiersEvaluation {
    pub fn values(&self) -> &[String] {
        match self {
            IdentifiersEvaluation::Valid { values } => values,
            _ => &[],
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            IdentifiersEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, IdentifiersEvaluation::Valid { .. })
    }
}

/// The Utility-owned payload persisted in History.
///
/// The generated values are stored verbatim so restore never regenerates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiersSnapshot {
    pub request: IdentifiersRequest,
    pub values: Vec<String>,
}

impl IdentifiersSnapshot {
    pub fn restore(&self) -> (&IdentifiersRequest, &[String]) {
        (&self.request, &self.values)
    }
}

/// Injectable clock and randomness for identifier generation.
///
/// Production uses [`SystemIdentifierSource`]; tests implement this trait with
/// fixed bytes and a fixed instant.
pub trait IdentifierSource {
    /// Fills `buffer` with cryptographically strong random bytes.
    fn fill_random(&self, buffer: &mut [u8]);

    /// The current Unix time in milliseconds.
    fn unix_millis(&self) -> u64;
}

/// The production source: system clock plus the operating system CSPRNG.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemIdentifierSource;

impl IdentifierSource for SystemIdentifierSource {
    fn fill_random(&self, buffer: &mut [u8]) {
        rand::rngs::OsRng.fill_bytes(buffer);
    }

    fn unix_millis(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or_default()
    }
}

/// Evaluates `request` using the production clock and cryptographic randomness.
pub fn evaluate(request: &IdentifiersRequest) -> IdentifiersEvaluation {
    evaluate_with_source(request, &SystemIdentifierSource)
}

/// Evaluates `request` against an injected clock and random source.
pub fn evaluate_with_source(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    match request.format {
        IdentifierFormat::Uuid => evaluate_uuid(request, source),
    }
}

fn evaluate_uuid(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    match request.action {
        IdentifierAction::Generate => generate_batch(request, source),
        IdentifierAction::Inspect => inspect(request),
    }
}

fn generate_batch(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    if request.count == 0 || request.count > MAXIMUM_GENERATED_COUNT {
        return IdentifiersEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(format!(
                "Count must be between 1 and {MAXIMUM_GENERATED_COUNT}."
            ))],
        };
    }
    let namespace = if request.version.requires_namespace() {
        match uuid_bytes(&request.namespace) {
            Some(bytes) => Some(bytes),
            None => {
                return IdentifiersEvaluation::Invalid {
                    diagnostics: vec![Diagnostic::error("Namespace must be a valid UUID.")],
                };
            }
        }
    } else {
        None
    };

    let mut values = Vec::with_capacity(request.count as usize);
    for _ in 0..request.count {
        match generate_uuid(request.version, namespace.as_ref(), &request.name, source) {
            Ok(bytes) => values.push(format_uuid(&bytes, request.uppercase, request.hyphenated)),
            Err(message) => {
                return IdentifiersEvaluation::Invalid {
                    diagnostics: vec![Diagnostic::error(message)],
                };
            }
        }
    }
    IdentifiersEvaluation::Valid { values }
}

fn inspect(request: &IdentifiersRequest) -> IdentifiersEvaluation {
    match normalize_uuid(&request.input, request.uppercase, request.hyphenated) {
        Ok(normalized) => IdentifiersEvaluation::Valid {
            values: vec![normalized],
        },
        Err(message) => IdentifiersEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(message)],
        },
    }
}

fn generate_uuid(
    version: UuidVersion,
    namespace: Option<&[u8; 16]>,
    name: &str,
    source: &dyn IdentifierSource,
) -> Result<[u8; 16], String> {
    match version {
        UuidVersion::V1 => Ok(time_uuid(1, source)),
        UuidVersion::V6 => Ok(time_uuid(6, source)),
        UuidVersion::V4 => {
            let mut bytes = random_bytes(source);
            bytes[6] = (bytes[6] & 0x0F) | 0x40;
            bytes[8] = (bytes[8] & 0x3F) | 0x80;
            Ok(bytes)
        }
        UuidVersion::V7 => {
            let mut bytes = random_bytes(source);
            let millis = source.unix_millis();
            for (index, slot) in bytes.iter_mut().enumerate().take(6) {
                *slot = ((millis >> (8 * (5 - index))) & 0xFF) as u8;
            }
            bytes[6] = (bytes[6] & 0x0F) | 0x70;
            bytes[8] = (bytes[8] & 0x3F) | 0x80;
            Ok(bytes)
        }
        UuidVersion::V3 | UuidVersion::V5 => {
            let Some(namespace) = namespace else {
                return Err("Namespace must be a valid UUID.".to_owned());
            };
            Ok(hash_uuid(version.number(), namespace, name))
        }
    }
}

/// A v1 or v6 UUID from the injected instant, per RFC 9562.
fn time_uuid(version: u8, source: &dyn IdentifierSource) -> [u8; 16] {
    let timestamp = source
        .unix_millis()
        .saturating_mul(10_000)
        .saturating_add(GREGORIAN_UNIX_TICKS);
    let random = random_bytes(source);
    let mut bytes = [0u8; 16];
    if version == 1 {
        let low = (timestamp & 0xFFFF_FFFF) as u32;
        let mid = ((timestamp >> 32) & 0xFFFF) as u16;
        let high = (((timestamp >> 48) & 0x0FFF) as u16) | 0x1000;
        bytes[0..4].copy_from_slice(&low.to_be_bytes());
        bytes[4..6].copy_from_slice(&mid.to_be_bytes());
        bytes[6..8].copy_from_slice(&high.to_be_bytes());
    } else {
        bytes[0] = ((timestamp >> 52) & 0xFF) as u8;
        bytes[1] = ((timestamp >> 44) & 0xFF) as u8;
        bytes[2] = ((timestamp >> 36) & 0xFF) as u8;
        bytes[3] = ((timestamp >> 28) & 0xFF) as u8;
        bytes[4] = ((timestamp >> 20) & 0xFF) as u8;
        bytes[5] = ((timestamp >> 12) & 0xFF) as u8;
        bytes[6] = (((timestamp >> 8) & 0x0F) as u8) | 0x60;
        bytes[7] = (timestamp & 0xFF) as u8;
    }
    bytes[8] = (random[0] & 0x3F) | 0x80;
    bytes[9..16].copy_from_slice(&random[1..8]);
    bytes
}

/// A v3 (MD5) or v5 (SHA-1) UUID over the namespace bytes and the UTF-8 name.
fn hash_uuid(version: u8, namespace: &[u8; 16], name: &str) -> [u8; 16] {
    let mut input = Vec::with_capacity(16 + name.len());
    input.extend_from_slice(namespace);
    input.extend_from_slice(name.as_bytes());
    let digest = match version {
        3 => Md5::digest(&input).to_vec(),
        _ => Sha1::digest(&input).to_vec(),
    };
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0F) | (version << 4);
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    bytes
}

fn random_bytes(source: &dyn IdentifierSource) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    source.fill_random(&mut bytes);
    bytes
}

/// Renders 16 bytes as a canonical UUID, honoring case and hyphen controls.
pub fn format_uuid(bytes: &[u8; 16], uppercase: bool, hyphenated: bool) -> String {
    let value = uuid::Uuid::from_bytes(*bytes);
    let text = if hyphenated {
        value.hyphenated().to_string()
    } else {
        value.simple().to_string()
    };
    if uppercase {
        text.to_uppercase()
    } else {
        text
    }
}

/// Strictly parses and normalizes a UUID.
///
/// Hyphens are ignored; exactly 32 hexadecimal digits are required, the version
/// nibble must be 1 through 8 and the variant bits must be `10xx`. The result
/// is re-rendered with the requested case and hyphenation.
pub fn normalize_uuid(input: &str, uppercase: bool, hyphenated: bool) -> Result<String, String> {
    const MESSAGE: &str = "Enter a strict RFC 9562 UUID with a supported variant and version.";
    let Some(bytes) = uuid_bytes(input) else {
        return Err(MESSAGE.to_owned());
    };
    let version = bytes[6] >> 4;
    let variant = bytes[8] & 0xC0;
    if !(1..=8).contains(&version) || variant != 0x80 {
        return Err(MESSAGE.to_owned());
    }
    Ok(format_uuid(&bytes, uppercase, hyphenated))
}

/// Parses exactly 32 hexadecimal digits, ignoring any hyphens, into bytes.
fn uuid_bytes(input: &str) -> Option<[u8; 16]> {
    let compact: String = input
        .chars()
        .filter(|character| *character != '-')
        .collect();
    if compact.len() != 32 {
        return None;
    }
    let bytes = hex::decode(compact.as_str()).ok()?;
    <[u8; 16]>::try_from(bytes.as_slice()).ok()
}

/// The Identifier Generator's identity for the shared [`Utility`] trait.
pub struct Identifiers;

impl Utility for Identifiers {
    type Request = IdentifiersRequest;
    type Evaluation = IdentifiersEvaluation;
    type Snapshot = IdentifiersSnapshot;

    const ID: &'static str = IDENTIFIERS_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = IDENTIFIERS_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> IdentifiersEvaluation {
        IdentifiersEvaluation::Empty
    }

    fn evaluate(request: &IdentifiersRequest) -> IdentifiersEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &IdentifiersEvaluation) -> bool {
        matches!(evaluation, IdentifiersEvaluation::Empty)
    }

    fn snapshot(
        request: &IdentifiersRequest,
        evaluation: &IdentifiersEvaluation,
    ) -> Option<IdentifiersSnapshot> {
        match evaluation {
            IdentifiersEvaluation::Valid { values } => Some(IdentifiersSnapshot {
                request: request.clone(),
                values: values.clone(),
            }),
            _ => None,
        }
    }

    fn restore(snapshot: &IdentifiersSnapshot) -> (IdentifiersRequest, IdentifiersEvaluation) {
        (
            snapshot.request.clone(),
            IdentifiersEvaluation::Valid {
                values: snapshot.values.clone(),
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
        fn new(seed: u8, millis: u64) -> Self {
            Self {
                bytes: (0..64)
                    .map(|index| seed.wrapping_add(index as u8))
                    .collect(),
                millis,
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

    fn generate_request(version: UuidVersion, count: u32) -> IdentifiersRequest {
        IdentifiersRequest {
            action: IdentifierAction::Generate,
            version,
            count,
            ..IdentifiersRequest::default()
        }
    }

    fn values(request: &IdentifiersRequest, source: &dyn IdentifierSource) -> Vec<String> {
        evaluate_with_source(request, source).values().to_vec()
    }

    fn parse(value: &str) -> uuid::Uuid {
        uuid::Uuid::parse_str(value).expect("a generated UUID parses")
    }

    #[test]
    fn default_is_one_lowercase_hyphenated_v4() {
        let request = IdentifiersRequest::default();
        assert_eq!(request.format, IdentifierFormat::Uuid);
        assert_eq!(request.version, UuidVersion::V4);
        assert_eq!(request.count, 1);
        assert!(!request.uppercase);
        assert!(request.hyphenated);

        let generated = values(&request, &FixedSource::new(0, 0));
        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0].len(), 36);
        assert_eq!(generated[0], generated[0].to_lowercase());
        let parsed = parse(&generated[0]);
        assert_eq!(parsed.get_version_num(), 4);
        assert_eq!(parsed.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn every_supported_version_sets_its_version_and_variant_bits() {
        let source = FixedSource::new(0x20, 1_700_000_000_000);
        for version in UuidVersion::ALL {
            let request = generate_request(version, 1);
            let generated = values(&request, &source);
            assert_eq!(generated.len(), 1, "{version:?}");
            let parsed = parse(&generated[0]);
            assert_eq!(
                parsed.get_version_num(),
                version.number() as usize,
                "{version:?}"
            );
            assert_eq!(parsed.get_variant(), uuid::Variant::RFC4122, "{version:?}");
        }
    }

    #[test]
    fn time_based_versions_carry_the_injected_instant() {
        let millis = 1_700_000_000_000u64;
        let source = FixedSource::new(0x11, millis);
        for version in [UuidVersion::V1, UuidVersion::V6, UuidVersion::V7] {
            let request = generate_request(version, 1);
            let generated = values(&request, &source);
            let timestamp = parse(&generated[0])
                .get_timestamp()
                .expect("a time-based UUID carries a timestamp");
            assert_eq!(timestamp.to_unix(), (millis / 1_000, 0), "{version:?}");
        }
    }

    #[test]
    fn published_namespace_vectors_match_for_v3_and_v5() {
        // Published RFC 9562 / Python `uuid` oracle values for the DNS
        // namespace, independent of this implementation.
        let namespace = DEFAULT_NAMESPACE;
        let source = FixedSource::new(0x7F, 0);
        for (version, name, expected) in [
            (
                UuidVersion::V3,
                "www.example.com",
                "5df41881-3aed-3515-88a7-2f4a814cf09e",
            ),
            (
                UuidVersion::V5,
                "www.example.com",
                "2ed6657d-e927-568b-95e1-2665a8aea6a2",
            ),
            (
                UuidVersion::V3,
                "python.org",
                "6fa459ea-ee8a-3ca4-894e-db77e160355e",
            ),
            (
                UuidVersion::V5,
                "python.org",
                "886313e1-3b8a-5372-9b90-0c9aee199e5d",
            ),
        ] {
            let request = IdentifiersRequest {
                action: IdentifierAction::Generate,
                version,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                count: 1,
                ..IdentifiersRequest::default()
            };
            assert_eq!(values(&request, &source), vec![expected.to_owned()]);
        }
    }

    #[test]
    fn v3_and_v5_are_deterministic_and_ignore_randomness() {
        let request = IdentifiersRequest {
            action: IdentifierAction::Generate,
            version: UuidVersion::V5,
            name: "stable".to_owned(),
            ..IdentifiersRequest::default()
        };
        assert_eq!(
            values(&request, &FixedSource::new(1, 0)),
            values(&request, &FixedSource::new(200, 9_999))
        );
    }

    #[test]
    fn v3_and_v5_require_a_valid_namespace() {
        let source = FixedSource::new(0, 0);
        for namespace in ["", "not-a-uuid", "6ba7b810-9dad-11d1-80b4-00c04fd430c"] {
            for version in [UuidVersion::V3, UuidVersion::V5] {
                let request = IdentifiersRequest {
                    namespace: namespace.to_owned(),
                    ..generate_request(version, 1)
                };
                let evaluation = evaluate_with_source(&request, &source);
                assert!(
                    !evaluation.is_valid_operation(),
                    "{version:?} {namespace:?}"
                );
                assert!(evaluation.values().is_empty());
                assert!(evaluation.diagnostics()[0].message.contains("Namespace"));
            }
        }
    }

    #[test]
    fn options_control_case_and_hyphenation() {
        let source = FixedSource::new(0x33, 0);
        let compact = IdentifiersRequest {
            uppercase: true,
            hyphenated: false,
            ..generate_request(UuidVersion::V4, 1)
        };
        let value = values(&compact, &source).remove(0);
        assert_eq!(value.len(), 32);
        assert_eq!(value, value.to_uppercase());
        assert!(value.chars().all(|character| character.is_ascii_hexdigit()));

        let hyphenated = IdentifiersRequest {
            uppercase: false,
            hyphenated: true,
            ..generate_request(UuidVersion::V4, 1)
        };
        let value = values(&hyphenated, &source).remove(0);
        assert_eq!(value.len(), 36);
        assert_eq!(value.matches('-').count(), 4);
        assert_eq!(value, value.to_lowercase());
    }

    #[test]
    fn strict_validation_normalizes_and_rejects_malformed_values() {
        assert_eq!(
            normalize_uuid("5DF41881-3AED-3515-88A7-2F4A814CF09E", false, true).unwrap(),
            "5df41881-3aed-3515-88a7-2f4a814cf09e"
        );
        assert_eq!(
            normalize_uuid("5df418813aed351588a72f4a814cf09e", true, false).unwrap(),
            "5DF418813AED351588A72F4A814CF09E"
        );
        for invalid in [
            "",
            "not-a-uuid",
            "5df41881-3aed-3515-88a7-2f4a814cf09",
            "5df41881-3aed-0515-88a7-2f4a814cf09e", // version 0
            "5df41881-3aed-9515-88a7-2f4a814cf09e", // version 9
            "5df41881-3aed-3515-08a7-2f4a814cf09e", // non-RFC variant
            "5df41881-3aed-3515-88a7-2f4a814cf09g", // non-hex digit
        ] {
            assert!(
                normalize_uuid(invalid, false, true).is_err(),
                "{invalid:?} must be rejected"
            );
        }
    }

    #[test]
    fn inspect_returns_a_normalized_value_or_a_diagnostic() {
        let source = FixedSource::new(0, 0);
        let request = IdentifiersRequest {
            action: IdentifierAction::Inspect,
            input: "5DF41881-3AED-3515-88A7-2F4A814CF09E".to_owned(),
            ..IdentifiersRequest::default()
        };
        assert_eq!(
            values(&request, &source),
            vec!["5df41881-3aed-3515-88a7-2f4a814cf09e".to_owned()]
        );

        let invalid = IdentifiersRequest {
            input: "nope".to_owned(),
            ..request
        };
        let evaluation = evaluate_with_source(&invalid, &source);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.values().is_empty());
        assert_eq!(evaluation.diagnostics().len(), 1);
    }

    #[test]
    fn batch_limits_are_enforced() {
        let source = FixedSource::new(0, 0);
        for count in [0, MAXIMUM_GENERATED_COUNT + 1] {
            let request = generate_request(UuidVersion::V4, count);
            let evaluation = evaluate_with_source(&request, &source);
            assert!(!evaluation.is_valid_operation(), "count {count}");
            assert!(evaluation.diagnostics()[0].message.contains("Count"));
        }
        let maximum = generate_request(UuidVersion::V4, MAXIMUM_GENERATED_COUNT);
        assert_eq!(
            evaluate_with_source(&maximum, &source).values().len(),
            MAXIMUM_GENERATED_COUNT as usize
        );
    }

    #[test]
    fn neutral_and_invalid_operations_never_snapshot() {
        let request = generate_request(UuidVersion::V4, 1);
        assert!(
            <Identifiers as Utility>::snapshot(&request, &IdentifiersEvaluation::Empty).is_none()
        );
        let invalid = evaluate_with_source(
            &generate_request(UuidVersion::V4, 0),
            &FixedSource::new(0, 0),
        );
        assert!(<Identifiers as Utility>::snapshot(&request, &invalid).is_none());
    }

    #[test]
    fn snapshot_round_trips_the_exact_batch_without_regeneration() {
        let source = FixedSource::new(0xAB, 1_700_000_000_000);
        let request = IdentifiersRequest {
            count: 3,
            ..generate_request(UuidVersion::V4, 1)
        };
        let evaluation = evaluate_with_source(&request, &source);
        let snapshot =
            <Identifiers as Utility>::snapshot(&request, &evaluation).expect("settled snapshot");
        assert_eq!(snapshot.values, evaluation.values());
        assert_eq!(snapshot.values.len(), 3);

        let encoded = serde_json::to_value(&snapshot).expect("snapshot serializes");
        let decoded: IdentifiersSnapshot =
            serde_json::from_value(encoded).expect("snapshot deserializes");
        assert_eq!(decoded, snapshot);

        let (restored_request, restored_evaluation) = <Identifiers as Utility>::restore(&decoded);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.values(), snapshot.values.as_slice());
    }

    #[test]
    fn repeated_generation_with_a_new_nonce_records_separately() {
        use crate::session::{Session, SubmitOutcome};

        let mut session = Session::<Identifiers>::new();
        let first = IdentifiersRequest {
            generation: 1,
            ..generate_request(UuidVersion::V4, 1)
        };
        let SubmitOutcome::Scheduled(first_revision) = session.submit(first) else {
            panic!("expected a scheduled revision");
        };
        session.resolve(first_revision);
        assert!(session.take_snapshot().is_some());

        let second = IdentifiersRequest {
            generation: 2,
            ..generate_request(UuidVersion::V4, 1)
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
}
