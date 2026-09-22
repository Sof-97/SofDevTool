//! The Identifier Generator Utility's GPUI-independent request/result/snapshot
//! contract for UUID, ULID and KSUID generation, inspection, validation,
//! normalization and options.
//!
//! Identifier *format* is a separate dimension from the UUID *version*: ULID
//! and KSUID are sibling formats beside UUID, not UUID versions. A format shows
//! only its own controls, and a UUID-version control has no meaning for the
//! other formats.
//!
//! Generation is deliberately non-deterministic. The production
//! [`Utility::evaluate`] reads the system clock and cryptographic randomness,
//! while [`evaluate_with_source`] accepts an injected [`IdentifierSource`] so
//! tests are reproducible without weakening the production path. Snapshots
//! capture the generated values verbatim, so restoring a recorded operation
//! never regenerates and never reads a clock or random source.
//!
//! Process-local monotonic ULID ordering is threaded through the request as the
//! previously generated 128-bit value. The workspace carries it forward between
//! deliberate batches; the engine still generates every value in one pass, so
//! an injected fixed clock makes monotonic behavior deterministic in tests.

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

/// Maximum explicit ordered-sequence KSUID batch size. The last two bytes hold
/// the sequence, so the batch cannot exceed the 16-bit sequence space.
pub const MAXIMUM_ORDERED_KSUID_COUNT: u32 = 65_536;

/// Largest value a 48-bit ULID millisecond timestamp can hold (year 10889).
pub const ULID_MAXIMUM_MILLIS: u64 = (1 << 48) - 1;

/// Seconds between the KSUID epoch (2014-05-13T16:53:20Z) and the Unix epoch.
const KSUID_EPOCH_SECONDS: u64 = 1_400_000_000;

/// Crockford base32, the canonical ULID alphabet (excludes I, L, O and U).
const ULID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Base62, the case-sensitive KSUID alphabet.
const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// 100-nanosecond intervals between the Gregorian (1582-10-15) and Unix epochs.
const GREGORIAN_UNIX_TICKS: u64 = 122_192_928_000_000_000;

/// The published DNS namespace, the baseline's default.
pub const DEFAULT_NAMESPACE: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

/// The identifier format family: UUID, ULID or KSUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierFormat {
    /// RFC 9562 universally unique identifiers.
    Uuid,
    /// Universally Unique Lexicographically Sortable Identifiers.
    Ulid,
    /// K-Sortable Unique IDentifiers.
    Ksuid,
}

impl IdentifierFormat {
    /// Declaration order, used for stable control order.
    pub const ALL: [IdentifierFormat; 3] = [
        IdentifierFormat::Uuid,
        IdentifierFormat::Ulid,
        IdentifierFormat::Ksuid,
    ];

    /// Human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            IdentifierFormat::Uuid => "UUID",
            IdentifierFormat::Ulid => "ULID",
            IdentifierFormat::Ksuid => "KSUID",
        }
    }

    /// Stable index in [`IdentifierFormat::ALL`].
    pub const fn index(self) -> usize {
        match self {
            IdentifierFormat::Uuid => 0,
            IdentifierFormat::Ulid => 1,
            IdentifierFormat::Ksuid => 2,
        }
    }
}

/// How ULID randomness relates to the process-local previous value.
///
/// `Random` draws fresh cryptographic randomness for every value. `Monotonic`
/// reuses the previous 80 random bits, incrementing them when the millisecond
/// timestamp is unchanged, so a same-millisecond batch stays ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UlidMode {
    #[default]
    Random,
    Monotonic,
}

impl UlidMode {
    /// Declaration order, used for stable control order and focus indexing.
    pub const ALL: [UlidMode; 2] = [UlidMode::Random, UlidMode::Monotonic];

    /// Human-readable label, matching the baseline.
    pub const fn label(self) -> &'static str {
        match self {
            UlidMode::Random => "Random",
            UlidMode::Monotonic => "Process-local monotonic",
        }
    }

    /// Stable index in [`UlidMode::ALL`].
    pub const fn index(self) -> usize {
        match self {
            UlidMode::Random => 0,
            UlidMode::Monotonic => 1,
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
    #[serde(default)]
    pub ulid_mode: UlidMode,
    #[serde(default)]
    pub ordered_ksuid: bool,
    /// The previously generated ULID bytes, carried forward so a later
    /// monotonic batch keeps ordering across deliberate Generate actions. It is
    /// derived output, never user input, and is ignored outside monotonic ULID.
    #[serde(default)]
    pub previous_ulid: Option<[u8; 16]>,
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
            ulid_mode: UlidMode::Random,
            ordered_ksuid: false,
            previous_ulid: None,
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
        IdentifierFormat::Ulid => evaluate_ulid(request, source),
        IdentifierFormat::Ksuid => evaluate_ksuid(request, source),
    }
}

/// The largest batch a request may generate. KSUID ordered-sequence batches may
/// use the full 16-bit sequence space; every other format is capped at the
/// baseline generation limit.
fn batch_maximum(request: &IdentifiersRequest) -> u32 {
    if request.format == IdentifierFormat::Ksuid && request.ordered_ksuid {
        MAXIMUM_ORDERED_KSUID_COUNT
    } else {
        MAXIMUM_GENERATED_COUNT
    }
}

fn count_diagnostic(maximum: u32) -> IdentifiersEvaluation {
    IdentifiersEvaluation::Invalid {
        diagnostics: vec![Diagnostic::error(format!(
            "Count must be between 1 and {maximum}."
        ))],
    }
}

fn evaluate_uuid(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    match request.action {
        IdentifierAction::Generate => generate_uuid_batch(request, source),
        IdentifierAction::Inspect => inspect_uuid(request),
    }
}

fn generate_uuid_batch(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    if request.count == 0 || request.count > MAXIMUM_GENERATED_COUNT {
        return count_diagnostic(MAXIMUM_GENERATED_COUNT);
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

fn inspect_uuid(request: &IdentifiersRequest) -> IdentifiersEvaluation {
    match normalize_uuid(&request.input, request.uppercase, request.hyphenated) {
        Ok(normalized) => IdentifiersEvaluation::Valid {
            values: vec![normalized],
        },
        Err(message) => IdentifiersEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(message)],
        },
    }
}

fn evaluate_ulid(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    match request.action {
        IdentifierAction::Generate => generate_ulid_batch(request, source),
        IdentifierAction::Inspect => match decode_ulid(&request.input) {
            Some(bytes) => IdentifiersEvaluation::Valid {
                values: vec![inspect_ulid(&bytes)],
            },
            None => IdentifiersEvaluation::Invalid {
                diagnostics: vec![Diagnostic::error("Enter a canonical 26-character ULID.")],
            },
        },
    }
}

fn generate_ulid_batch(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    if request.count == 0 || request.count > MAXIMUM_GENERATED_COUNT {
        return count_diagnostic(MAXIMUM_GENERATED_COUNT);
    }
    // Random mode draws every value independently; monotonic mode starts from
    // any carried value and continues through the batch.
    let monotonic = request.ulid_mode == UlidMode::Monotonic;
    let mut previous = if monotonic {
        request.previous_ulid
    } else {
        None
    };
    let mut values = Vec::with_capacity(request.count as usize);
    for _ in 0..request.count {
        let bytes = generate_ulid(source, previous);
        values.push(encode_ulid(&bytes));
        if monotonic {
            previous = Some(bytes);
        }
    }
    IdentifiersEvaluation::Valid { values }
}

fn evaluate_ksuid(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    match request.action {
        IdentifierAction::Generate => generate_ksuid_batch(request, source),
        IdentifierAction::Inspect => match decode_ksuid(&request.input) {
            Some(bytes) => IdentifiersEvaluation::Valid {
                values: vec![inspect_ksuid(&bytes)],
            },
            None => IdentifiersEvaluation::Invalid {
                diagnostics: vec![Diagnostic::error(
                    "Enter a canonical, case-sensitive 27-character KSUID.",
                )],
            },
        },
    }
}

fn generate_ksuid_batch(
    request: &IdentifiersRequest,
    source: &dyn IdentifierSource,
) -> IdentifiersEvaluation {
    let maximum = batch_maximum(request);
    if request.count == 0 || request.count > maximum {
        return count_diagnostic(maximum);
    }
    let mut values = Vec::with_capacity(request.count as usize);
    for index in 0..request.count {
        let sequence = if request.ordered_ksuid {
            Some(index as u16)
        } else {
            None
        };
        values.push(generate_ksuid(source, sequence));
    }
    IdentifiersEvaluation::Valid { values }
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

/// Generates one ULID's 16 bytes from the injected instant and randomness.
///
/// The first six bytes are the 48-bit millisecond timestamp; the remaining ten
/// are random. In monotonic mode, when the timestamp matches `previous`, the
/// previous 80-bit random tail is incremented with carry instead of redrawn, so
/// a same-millisecond sequence stays strictly ordered.
fn generate_ulid(source: &dyn IdentifierSource, previous: Option<[u8; 16]>) -> [u8; 16] {
    let mut bytes = random_bytes(source);
    let millis = source.unix_millis() & ULID_MAXIMUM_MILLIS;
    for (index, slot) in bytes.iter_mut().enumerate().take(6) {
        *slot = ((millis >> (8 * (5 - index))) & 0xFF) as u8;
    }
    if let Some(mut previous) = previous {
        if previous[..6] == bytes[..6] {
            for index in (6..16).rev() {
                previous[index] = previous[index].wrapping_add(1);
                if previous[index] != 0 {
                    break;
                }
            }
            bytes = previous;
        }
    }
    bytes
}

/// Encodes 16 bytes as a canonical, uppercase 26-character Crockford ULID.
fn encode_ulid(bytes: &[u8; 16]) -> String {
    let mut output = String::with_capacity(26);
    for group in 0..26 {
        let mut value = 0usize;
        for bit in 0..5 {
            value <<= 1;
            let stream_bit = group * 5 + bit;
            if stream_bit >= 2 {
                let position = stream_bit - 2;
                value |= usize::from((bytes[position / 8] >> (7 - position % 8)) & 1);
            }
        }
        output.push(ULID_ALPHABET[value] as char);
    }
    output
}

/// Strictly decodes a canonical ULID. Input is case-insensitive for
/// validation, but the alphabet, length and 48-bit timestamp bound are strict:
/// exactly 26 Crockford characters whose first two bits are zero.
pub fn decode_ulid(input: &str) -> Option<[u8; 16]> {
    let raw = input.as_bytes();
    if raw.len() != 26 {
        return None;
    }
    let mut bits = [0u8; 130];
    for (index, &byte) in raw.iter().enumerate() {
        let upper = byte.to_ascii_uppercase();
        let value = ULID_ALPHABET
            .iter()
            .position(|candidate| *candidate == upper)?;
        for bit in 0..5 {
            bits[index * 5 + bit] = ((value >> (4 - bit)) & 1) as u8;
        }
    }
    if bits[0] != 0 || bits[1] != 0 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (byte, slot) in bytes.iter_mut().enumerate() {
        let mut value = 0u8;
        for bit in 0..8 {
            value = (value << 1) | bits[2 + byte * 8 + bit];
        }
        *slot = value;
    }
    Some(bytes)
}

/// A human-readable inspection of a valid ULID: its millisecond timestamp and
/// the corresponding UTC instant.
fn inspect_ulid(bytes: &[u8; 16]) -> String {
    let millis = bytes[..6].iter().fold(0u64, |accumulator, byte| {
        (accumulator << 8) | u64::from(*byte)
    });
    let timestamp = format_timestamp(chrono::DateTime::from_timestamp_millis(
        i64::try_from(millis).unwrap_or(i64::MAX),
    ));
    format!("Valid ULID · {millis} ms · {timestamp}")
}

/// Generates one KSUID's canonical base62 string from the injected instant and
/// randomness. When `sequence` is present, it occupies the final two payload
/// bytes, producing an explicit ordered batch.
fn generate_ksuid(source: &dyn IdentifierSource, sequence: Option<u16>) -> String {
    let unix_seconds = source.unix_millis() / 1_000;
    let seconds = unix_seconds
        .saturating_sub(KSUID_EPOCH_SECONDS)
        .min(u64::from(u32::MAX)) as u32;
    let mut bytes = [0u8; 20];
    bytes[..4].copy_from_slice(&seconds.to_be_bytes());
    let mut payload = [0u8; 16];
    source.fill_random(&mut payload);
    bytes[4..].copy_from_slice(&payload);
    if let Some(sequence) = sequence {
        bytes[18..].copy_from_slice(&sequence.to_be_bytes());
    }
    encode_base62(&bytes)
}

/// Strictly decodes a canonical KSUID. Validation is case-sensitive: the exact
/// 27-character base62 string is decoded as written, without case folding.
pub fn decode_ksuid(input: &str) -> Option<[u8; 20]> {
    let characters: Vec<char> = input.chars().collect();
    if characters.len() != 27 {
        return None;
    }
    let mut bytes = [0u8; 20];
    for character in characters {
        let digit = BASE62_ALPHABET
            .iter()
            .position(|candidate| *candidate == character as u8)? as u32;
        let mut carry = digit;
        for byte in bytes.iter_mut().rev() {
            let value = u32::from(*byte) * 62 + carry;
            *byte = (value & 0xFF) as u8;
            carry = value >> 8;
        }
        if carry != 0 {
            return None;
        }
    }
    Some(bytes)
}

/// A human-readable inspection of a valid KSUID: the UTC instant its 32-bit
/// timestamp denotes.
fn inspect_ksuid(bytes: &[u8; 20]) -> String {
    let seconds = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let unix_seconds = u64::from(seconds) + KSUID_EPOCH_SECONDS;
    let timestamp = format_timestamp(chrono::DateTime::from_timestamp(
        i64::try_from(unix_seconds).unwrap_or(i64::MAX),
        0,
    ));
    format!("Valid KSUID · {timestamp}")
}

/// Renders an optional UTC instant as a seconds-precision RFC 3339 string.
fn format_timestamp(timestamp: Option<chrono::DateTime<chrono::Utc>>) -> String {
    timestamp
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| "unknown time".to_owned())
}

/// Encodes 20 bytes as a zero-padded 27-character base62 string.
fn encode_base62(bytes: &[u8; 20]) -> String {
    let mut number = bytes.to_vec();
    let mut encoded: Vec<u8> = Vec::new();
    while number.iter().any(|byte| *byte != 0) {
        let mut quotient = Vec::new();
        let mut remainder = 0u32;
        for byte in &number {
            let value = remainder * 256 + u32::from(*byte);
            let digit = value / 62;
            if !quotient.is_empty() || digit != 0 {
                quotient.push(digit as u8);
            }
            remainder = value % 62;
        }
        encoded.push(BASE62_ALPHABET[remainder as usize]);
        number = quotient;
    }
    let mut output = String::with_capacity(27);
    for _ in 0..27usize.saturating_sub(encoded.len()) {
        output.push('0');
    }
    for byte in encoded.iter().rev() {
        output.push(*byte as char);
    }
    output
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

    fn format_request(format: IdentifierFormat, count: u32) -> IdentifiersRequest {
        IdentifiersRequest {
            action: IdentifierAction::Generate,
            format,
            count,
            ..IdentifiersRequest::default()
        }
    }

    #[test]
    fn format_and_ulid_mode_controls_are_addressable() {
        assert_eq!(IdentifierFormat::ALL.len(), 3);
        assert_eq!(IdentifierFormat::Uuid.index(), 0);
        assert_eq!(IdentifierFormat::Ulid.label(), "ULID");
        assert_eq!(IdentifierFormat::Ksuid.label(), "KSUID");
        assert_eq!(UlidMode::ALL.len(), 2);
        assert_eq!(UlidMode::default(), UlidMode::Random);
        assert_eq!(UlidMode::Monotonic.label(), "Process-local monotonic");
    }

    #[test]
    fn encoders_match_independent_bigint_oracles() {
        // Oracles computed with Python arbitrary-precision integers, not this
        // module's algorithms. KSUID timestamp 300,000,000 and payload 0..15.
        let mut ksuid = [0u8; 20];
        ksuid[..4].copy_from_slice(&300_000_000u32.to_be_bytes());
        for (index, slot) in ksuid[4..].iter_mut().enumerate() {
            *slot = index as u8;
        }
        assert_eq!(encode_base62(&ksuid), "2YBXZHqCjn9u9XPqNaTCk9OXKKF");

        // ULID millisecond 1_700_000_000_123 and 80-bit tail 0..9.
        let millis = 1_700_000_000_123u64;
        let mut ulid = [0u8; 16];
        ulid[..6].copy_from_slice(&[
            ((millis >> 40) & 0xFF) as u8,
            ((millis >> 32) & 0xFF) as u8,
            ((millis >> 24) & 0xFF) as u8,
            ((millis >> 16) & 0xFF) as u8,
            ((millis >> 8) & 0xFF) as u8,
            (millis & 0xFF) as u8,
        ]);
        for (index, slot) in ulid[6..].iter_mut().enumerate() {
            *slot = index as u8;
        }
        assert_eq!(encode_ulid(&ulid), "01HF7YAT3V000G40R40M30E209");
        assert_eq!(decode_ulid("01HF7YAT3V000G40R40M30E209"), Some(ulid));
    }

    #[test]
    fn ulid_encoding_round_trips_and_matches_known_boundaries() {
        // An all-zero 128-bit value is the all-zero 26-character ULID.
        assert_eq!(encode_ulid(&[0u8; 16]), "0".repeat(26));
        assert_eq!(decode_ulid(&"0".repeat(26)), Some([0u8; 16]));

        // 0x01 in the leading byte is value bit 2^120, which lands on the second
        // Crockford character. This is a hand-checked encoding oracle.
        let mut leading = [0u8; 16];
        leading[0] = 0x01;
        let encoded = encode_ulid(&leading);
        assert_eq!(encoded, format!("01{}", "0".repeat(24)));
        assert_eq!(decode_ulid(&encoded), Some(leading));

        // Arbitrary bytes survive an encode/decode round trip.
        let bytes: [u8; 16] = std::array::from_fn(|index| (index as u8).wrapping_mul(17));
        assert_eq!(decode_ulid(&encode_ulid(&bytes)), Some(bytes));
    }

    #[test]
    fn ulid_generation_is_canonical_uppercase_and_carries_the_injected_instant() {
        let millis = 1_700_000_000_123u64;
        let source = FixedSource::new(0x21, millis);
        let request = format_request(IdentifierFormat::Ulid, 1);
        let value = values(&request, &source).remove(0);

        assert_eq!(value.len(), 26);
        assert_eq!(value, value.to_uppercase());
        assert!(value
            .chars()
            .all(|character| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(character)));
        assert!(("01234567").contains(value.chars().next().unwrap()));

        let bytes = decode_ulid(&value).expect("a generated ULID decodes");
        let decoded_millis = bytes[..6].iter().fold(0u64, |accumulator, byte| {
            (accumulator << 8) | u64::from(*byte)
        });
        assert_eq!(decoded_millis, millis);
    }

    #[test]
    fn ulid_timestamp_boundaries_and_overflow_are_canonical() {
        // The largest 48-bit timestamp sets every timestamp bit; the first
        // character is then the largest permitted value, "7".
        let top = FixedSource::new(0x31, ULID_MAXIMUM_MILLIS);
        let top_value = values(&format_request(IdentifierFormat::Ulid, 1), &top).remove(0);
        assert_eq!(top_value.as_bytes()[0], b'7');
        let top_bytes = decode_ulid(&top_value).unwrap();
        assert_eq!(&top_bytes[..6], &[0xFF; 6]);

        // One millisecond past the 48-bit field wraps back to zero instead of
        // producing a first character outside the canonical alphabet.
        let zero = values(
            &format_request(IdentifierFormat::Ulid, 1),
            &FixedSource::new(0x31, 0),
        )
        .remove(0);
        let overflowed = values(
            &format_request(IdentifierFormat::Ulid, 1),
            &FixedSource::new(0x31, ULID_MAXIMUM_MILLIS + 1),
        )
        .remove(0);
        assert_eq!(overflowed, zero);
    }

    #[test]
    fn ulid_monotonic_batches_increment_within_a_fixed_millisecond() {
        let source = FixedSource::new(0x41, 1_700_000_000_123);
        let monotonic = IdentifiersRequest {
            ulid_mode: UlidMode::Monotonic,
            count: 5,
            ..format_request(IdentifierFormat::Ulid, 1)
        };
        let generated = values(&monotonic, &source);
        assert_eq!(generated.len(), 5);
        for pair in generated.windows(2) {
            assert!(pair[0] < pair[1], "monotonic batch must ascend: {pair:?}");
        }
        let timestamps: Vec<u64> = generated
            .iter()
            .map(|value| {
                decode_ulid(value)
                    .unwrap()
                    .iter()
                    .take(6)
                    .fold(0u64, |accumulator, byte| {
                        (accumulator << 8) | u64::from(*byte)
                    })
            })
            .collect();
        assert!(timestamps.windows(2).all(|pair| pair[0] == pair[1]));

        // Random mode redraws, so a fixed source repeats identical values.
        let random = IdentifiersRequest {
            ulid_mode: UlidMode::Random,
            count: 5,
            ..format_request(IdentifierFormat::Ulid, 1)
        };
        let repeated = values(&random, &source);
        assert!(repeated.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn ulid_monotonic_state_continues_across_deliberate_batches() {
        let source = FixedSource::new(0x51, 1_700_000_000_123);
        let first = IdentifiersRequest {
            ulid_mode: UlidMode::Monotonic,
            count: 3,
            ..format_request(IdentifierFormat::Ulid, 1)
        };
        let first_values = values(&first, &source);
        let previous = decode_ulid(first_values.last().unwrap()).expect("decodable tail");

        let second = IdentifiersRequest {
            previous_ulid: Some(previous),
            generation: 1,
            ..first
        };
        let second_values = values(&second, &source);
        assert!(
            second_values[0] > *first_values.last().unwrap(),
            "the next batch continues past the carried value"
        );
    }

    #[test]
    fn ulid_validation_is_strict_and_case_insensitive() {
        let canonical = encode_ulid(&std::array::from_fn(|index| index as u8 + 1));
        assert!(decode_ulid(&canonical).is_some());
        assert!(decode_ulid(&canonical.to_lowercase()).is_some());

        // A first character outside 0..7 overflows the 48-bit timestamp field.
        let overflow = format!("8{}", &canonical[1..]);
        assert!(decode_ulid(&overflow).is_none());

        for invalid in [
            String::new(),
            "0".repeat(25),
            "0".repeat(27),
            format!("{}I", &canonical[..25]),
            format!("{}L", &canonical[..25]),
            format!("{}O", &canonical[..25]),
            format!("{}U", &canonical[..25]),
            format!("{}-", &canonical[..25]),
        ] {
            assert!(
                decode_ulid(&invalid).is_none(),
                "{invalid:?} must be rejected"
            );
        }
    }

    #[test]
    fn ulid_inspection_reports_the_timestamp_or_diagnoses_invalid_input() {
        let source = FixedSource::new(0x61, 1_700_000_000_123);
        let generated = values(&format_request(IdentifierFormat::Ulid, 1), &source).remove(0);
        let request = IdentifiersRequest {
            action: IdentifierAction::Inspect,
            format: IdentifierFormat::Ulid,
            input: generated,
            ..IdentifiersRequest::default()
        };
        let inspection = values(&request, &source).remove(0);
        assert!(inspection.contains("1700000000123 ms"), "{inspection}");
        assert!(inspection.contains("2023-11-14T22:13:20Z"), "{inspection}");

        let invalid = IdentifiersRequest {
            input: "8".to_owned() + &"0".repeat(25),
            ..request
        };
        let evaluation = evaluate_with_source(&invalid, &source);
        assert!(!evaluation.is_valid_operation());
        assert_eq!(evaluation.diagnostics().len(), 1);
    }

    #[test]
    fn ksuid_generation_is_canonical_case_sensitive_and_inspectable() {
        let source = FixedSource::new(0x71, 1_700_000_000_000);
        let value = values(&format_request(IdentifierFormat::Ksuid, 1), &source).remove(0);
        assert_eq!(value.len(), 27);
        assert!(value.chars().all(|character| {
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".contains(character)
        }));

        let bytes = decode_ksuid(&value).expect("a generated KSUID decodes");
        let seconds = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(u64::from(seconds) + KSUID_EPOCH_SECONDS, 1_700_000_000);
        assert!(inspect_ksuid(&bytes).contains("2023-11-14T22:13:20Z"));

        // Case is significant: the same string with a different final letter
        // decodes to a different value instead of being folded.
        let upper = format!("{}A", &value[..26]);
        let lower = format!("{}a", &value[..26]);
        assert!(decode_ksuid(&upper).is_some());
        assert!(decode_ksuid(&lower).is_some());
        assert_ne!(decode_ksuid(&upper), decode_ksuid(&lower));

        for invalid in [
            String::new(),
            "0".repeat(26),
            "0".repeat(28),
            format!("{}!", &value[..26]),
        ] {
            assert!(
                decode_ksuid(&invalid).is_none(),
                "{invalid:?} must be rejected"
            );
        }
    }

    #[test]
    fn ordered_ksuid_batches_embed_the_sequence_and_respect_the_cap() {
        let source = FixedSource::new(0x81, 1_700_000_000_000);
        let ordered = IdentifiersRequest {
            ordered_ksuid: true,
            count: 4,
            ..format_request(IdentifierFormat::Ksuid, 1)
        };
        let generated = values(&ordered, &source);
        assert_eq!(generated.len(), 4);
        for (index, value) in generated.iter().enumerate() {
            let bytes = decode_ksuid(value).expect("an ordered KSUID decodes");
            let sequence = u16::from_be_bytes([bytes[18], bytes[19]]);
            assert_eq!(sequence, index as u16);
        }

        assert_eq!(
            batch_maximum(&ordered),
            MAXIMUM_ORDERED_KSUID_COUNT,
            "ordered KSUID batches use the 16-bit sequence space"
        );
        assert_eq!(
            batch_maximum(&format_request(IdentifierFormat::Ksuid, 1)),
            MAXIMUM_GENERATED_COUNT
        );
        assert_eq!(
            batch_maximum(&format_request(IdentifierFormat::Ulid, 1)),
            MAXIMUM_GENERATED_COUNT
        );

        let too_many = IdentifiersRequest {
            count: MAXIMUM_ORDERED_KSUID_COUNT + 1,
            ..ordered
        };
        let evaluation = evaluate_with_source(&too_many, &source);
        assert!(!evaluation.is_valid_operation());
        assert!(evaluation.diagnostics()[0].message.contains("65536"));

        let unordered_too_many = IdentifiersRequest {
            count: MAXIMUM_GENERATED_COUNT + 1,
            ..format_request(IdentifierFormat::Ksuid, 1)
        };
        assert!(!evaluate_with_source(&unordered_too_many, &source).is_valid_operation());

        let ulid_too_many = format_request(IdentifierFormat::Ulid, MAXIMUM_GENERATED_COUNT + 1);
        assert!(!evaluate_with_source(&ulid_too_many, &source).is_valid_operation());
    }

    #[test]
    fn ordered_ksuid_generates_the_full_sequence_capacity() {
        let source = FixedSource::new(0x91, 1_700_000_000_000);
        let request = IdentifiersRequest {
            ordered_ksuid: true,
            count: MAXIMUM_ORDERED_KSUID_COUNT,
            ..format_request(IdentifierFormat::Ksuid, 1)
        };
        let generated = values(&request, &source);
        assert_eq!(generated.len(), MAXIMUM_ORDERED_KSUID_COUNT as usize);
        let bytes = decode_ksuid(generated.last().unwrap()).unwrap();
        assert_eq!(
            u16::from_be_bytes([bytes[18], bytes[19]]),
            u16::MAX,
            "the last sequence number is 65,535"
        );
    }

    #[test]
    fn ulid_and_ksuid_snapshots_round_trip_without_regeneration() {
        let source = FixedSource::new(0xA5, 1_700_000_000_000);
        for request in [
            IdentifiersRequest {
                ulid_mode: UlidMode::Monotonic,
                count: 3,
                ..format_request(IdentifierFormat::Ulid, 1)
            },
            IdentifiersRequest {
                ordered_ksuid: true,
                count: 3,
                ..format_request(IdentifierFormat::Ksuid, 1)
            },
        ] {
            let evaluation = evaluate_with_source(&request, &source);
            let snapshot = <Identifiers as Utility>::snapshot(&request, &evaluation)
                .expect("a valid batch snapshots");
            assert_eq!(snapshot.values, evaluation.values());

            let encoded = serde_json::to_value(&snapshot).expect("snapshot serializes");
            let decoded: IdentifiersSnapshot =
                serde_json::from_value(encoded).expect("snapshot deserializes");
            assert_eq!(decoded, snapshot);

            // Restore reads only the payload: no clock, randomness or generation.
            let (restored_request, restored_evaluation) =
                <Identifiers as Utility>::restore(&decoded);
            assert_eq!(restored_request, request);
            assert_eq!(restored_evaluation.values(), snapshot.values.as_slice());
        }
    }
}
