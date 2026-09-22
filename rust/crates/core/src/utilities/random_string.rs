//! The Random String Utility's GPUI-independent request/result/snapshot contract.
//!
//! Generation uses system cryptographic randomness and unbiased rejection
//! sampling over a deduplicated alphabet of scalar values. The engine accepts
//! injected randomness so tests are deterministic; production passes [`OsRng`].
//! Random strings are for test data; this Utility makes no password-management
//! claim and reports entropy only when it can be calculated.

use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Random String Utility.
pub const RANDOM_STRING_UTILITY_ID: &str = "random-string";

/// Schema version of [`RandomStringSnapshot`].
pub const RANDOM_STRING_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Inclusive bounds for the generated length, matching the product baseline.
pub const MIN_LENGTH: usize = 1;
pub const MAX_LENGTH: usize = 4096;

/// Inclusive bounds for the batch size, matching the product baseline.
pub const MIN_COUNT: usize = 1;
pub const MAX_COUNT: usize = 100;

const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGITS: &str = "0123456789";
const SAFE_SYMBOLS: &str = "-._~!@#$%^&*";

/// Characters removed when ambiguous exclusion is enabled.
const AMBIGUOUS: &str = "0O1lI|`'\"";

/// One strongly typed Random String configuration.
///
/// `nonce` is a monotonic per-generation counter. It does not affect the
/// alphabet or the sampled values; it makes two deliberate identical requests
/// distinct to the shared revision session so each generates and records
/// separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomStringRequest {
    pub length: usize,
    pub count: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
    pub custom_alphabet: String,
    #[serde(default)]
    pub nonce: u64,
}

impl Default for RandomStringRequest {
    fn default() -> Self {
        Self {
            length: 20,
            count: 1,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: true,
            custom_alphabet: String::new(),
            nonce: 0,
        }
    }
}

impl RandomStringRequest {
    /// The configuration with the nonce cleared, for display and comparison.
    pub fn configuration(&self) -> Self {
        Self {
            nonce: 0,
            ..self.clone()
        }
    }
}

/// The typed outcome shown to the user.
#[derive(Clone, Debug, PartialEq)]
pub enum RandomStringEvaluation {
    /// Nothing has been generated yet.
    Empty,
    Valid {
        values: Vec<String>,
        /// `length * log2(alphabet size)`, present only when calculable.
        entropy_bits: Option<f64>,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl RandomStringEvaluation {
    pub fn values(&self) -> &[String] {
        match self {
            RandomStringEvaluation::Valid { values, .. } => values,
            _ => &[],
        }
    }

    pub fn entropy_bits(&self) -> Option<f64> {
        match self {
            RandomStringEvaluation::Valid { entropy_bits, .. } => *entropy_bits,
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            RandomStringEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, RandomStringEvaluation::Valid { .. })
    }
}

/// The exact generated values, captured so preview and restore never rerun.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RandomStringSnapshot {
    pub request: RandomStringRequest,
    pub values: Vec<String>,
    pub entropy_bits: Option<f64>,
}

impl RandomStringSnapshot {
    pub fn restore(&self) -> (&RandomStringRequest, &[String]) {
        (&self.request, &self.values)
    }
}

/// The deduplicated alphabet for a configuration, preserving first-seen order.
///
/// Enabled character classes come first, then the custom characters, with
/// ambiguous characters removed when requested.
pub fn alphabet(request: &RandomStringRequest) -> Vec<char> {
    let mut characters: Vec<char> = Vec::new();
    if request.uppercase {
        characters.extend(UPPERCASE.chars());
    }
    if request.lowercase {
        characters.extend(LOWERCASE.chars());
    }
    if request.digits {
        characters.extend(DIGITS.chars());
    }
    if request.symbols {
        characters.extend(SAFE_SYMBOLS.chars());
    }
    characters.extend(request.custom_alphabet.chars());

    let mut result = Vec::with_capacity(characters.len());
    for character in characters {
        if request.exclude_ambiguous && AMBIGUOUS.contains(character) {
            continue;
        }
        if !result.contains(&character) {
            result.push(character);
        }
    }
    result
}

/// Shannon entropy per generated result, or `None` when it is not calculable
/// (an alphabet smaller than two symbols has no entropy).
pub fn entropy_bits(length: usize, alphabet_size: usize) -> Option<f64> {
    if alphabet_size < 2 {
        return None;
    }
    Some(length as f64 * (alphabet_size as f64).log2())
}

/// Generates a batch using the supplied randomness source.
///
/// Tests use a seeded or scripted [`RngCore`]; production passes [`OsRng`].
pub fn generate_with<R: RngCore>(
    request: &RandomStringRequest,
    rng: &mut R,
) -> RandomStringEvaluation {
    let alphabet = alphabet(request);
    if let Err(diagnostics) = validate(request, &alphabet) {
        return RandomStringEvaluation::Invalid { diagnostics };
    }
    let mut values = Vec::with_capacity(request.count);
    for _ in 0..request.count {
        values.push(generate_value(&alphabet, request.length, rng));
    }
    RandomStringEvaluation::Valid {
        values,
        entropy_bits: entropy_bits(request.length, alphabet.len()),
    }
}

fn validate(request: &RandomStringRequest, alphabet: &[char]) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&request.length) {
        diagnostics.push(Diagnostic::error(format!(
            "Length must be {MIN_LENGTH}–{MAX_LENGTH}."
        )));
    }
    if !(MIN_COUNT..=MAX_COUNT).contains(&request.count) {
        diagnostics.push(Diagnostic::error(format!(
            "Count must be {MIN_COUNT}–{MAX_COUNT}."
        )));
    }
    if alphabet.len() < 2 {
        diagnostics.push(Diagnostic::error(
            "Choose at least two distinct characters.",
        ));
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn generate_value<R: RngCore>(alphabet: &[char], length: usize, rng: &mut R) -> String {
    let mut value = String::with_capacity(length);
    for _ in 0..length {
        value.push(alphabet[sample_index(alphabet.len(), rng)]);
    }
    value
}

/// Unbiased index sampling by rejection.
///
/// Draws a full `u32` and rejects the tail that is not a whole multiple of the
/// alphabet size, so every symbol is equally likely (no modulo bias).
fn sample_index<R: RngCore>(alphabet_size: usize, rng: &mut R) -> usize {
    debug_assert!(alphabet_size >= 2);
    let modulus = alphabet_size as u64;
    let draws = 1u64 << 32;
    let limit = draws - (draws % modulus);
    loop {
        let draw = rng.next_u32() as u64;
        if draw < limit {
            return (draw % modulus) as usize;
        }
    }
}

/// The Random String Utility's identity for the shared [`Utility`] trait.
pub struct RandomString;

impl Utility for RandomString {
    type Request = RandomStringRequest;
    type Evaluation = RandomStringEvaluation;
    type Snapshot = RandomStringSnapshot;

    const ID: &'static str = RANDOM_STRING_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = RANDOM_STRING_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> RandomStringEvaluation {
        RandomStringEvaluation::Empty
    }

    fn evaluate(request: &RandomStringRequest) -> RandomStringEvaluation {
        generate_with(request, &mut OsRng)
    }

    fn is_neutral(evaluation: &RandomStringEvaluation) -> bool {
        matches!(evaluation, RandomStringEvaluation::Empty)
    }

    fn snapshot(
        request: &RandomStringRequest,
        evaluation: &RandomStringEvaluation,
    ) -> Option<RandomStringSnapshot> {
        let RandomStringEvaluation::Valid {
            values,
            entropy_bits,
        } = evaluation
        else {
            return None;
        };
        Some(RandomStringSnapshot {
            request: request.clone(),
            values: values.clone(),
            entropy_bits: *entropy_bits,
        })
    }

    fn restore(snapshot: &RandomStringSnapshot) -> (RandomStringRequest, RandomStringEvaluation) {
        (
            snapshot.request.clone(),
            RandomStringEvaluation::Valid {
                values: snapshot.values.clone(),
                entropy_bits: snapshot.entropy_bits,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::{Error, RngCore, SeedableRng};

    use super::*;

    /// A scripted `RngCore` that yields a fixed sequence, so rejection can be
    /// tested without any probabilistic quality assertion.
    struct SequenceRng {
        values: Vec<u32>,
        index: usize,
    }

    impl SequenceRng {
        fn new(values: Vec<u32>) -> Self {
            Self { values, index: 0 }
        }
    }

    impl RngCore for SequenceRng {
        fn next_u32(&mut self) -> u32 {
            let value = self.values[self.index.min(self.values.len() - 1)];
            self.index += 1;
            value
        }

        fn next_u64(&mut self) -> u64 {
            ((self.next_u32() as u64) << 32) | self.next_u32() as u64
        }

        fn fill_bytes(&mut self, destination: &mut [u8]) {
            for chunk in destination.chunks_mut(4) {
                let bytes = self.next_u32().to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
        }

        fn try_fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), Error> {
            self.fill_bytes(destination);
            Ok(())
        }
    }

    fn only_custom(custom: &str) -> RandomStringRequest {
        RandomStringRequest {
            length: 1,
            count: 1,
            uppercase: false,
            lowercase: false,
            digits: false,
            symbols: false,
            exclude_ambiguous: false,
            custom_alphabet: custom.to_owned(),
            nonce: 0,
        }
    }

    #[test]
    fn defaults_are_the_documented_product_defaults() {
        let request = RandomStringRequest::default();
        assert_eq!(request.length, 20);
        assert_eq!(request.count, 1);
        assert!(request.uppercase);
        assert!(request.lowercase);
        assert!(request.digits);
        assert!(request.symbols);
        assert!(request.exclude_ambiguous);
        assert!(request.custom_alphabet.is_empty());
    }

    #[test]
    fn default_alphabet_covers_every_class_and_removes_ambiguous_characters() {
        let request = RandomStringRequest::default();
        let characters = alphabet(&request);

        assert_eq!(characters.len(), 69);
        assert!(characters.contains(&'A'));
        assert!(characters.contains(&'z'));
        assert!(characters.contains(&'5'));
        assert!(characters.contains(&'#'));
        for ambiguous in AMBIGUOUS.chars() {
            assert!(
                !characters.contains(&ambiguous),
                "ambiguous {ambiguous:?} must be excluded"
            );
        }
    }

    #[test]
    fn character_classes_toggle_their_contributions() {
        let request = RandomStringRequest {
            uppercase: false,
            digits: false,
            symbols: false,
            exclude_ambiguous: false,
            ..RandomStringRequest::default()
        };
        let characters = alphabet(&request);

        assert!(characters
            .iter()
            .all(|character| character.is_ascii_lowercase()));
        assert_eq!(characters.len(), 26);
    }

    #[test]
    fn ambiguous_exclusion_can_be_disabled_and_custom_characters_are_deduplicated() {
        let mut request = only_custom("aabbc0O1lI");
        request.exclude_ambiguous = true;
        let characters = alphabet(&request);
        // 'a','b','c' survive; duplicates collapse; ambiguous 0 O 1 l I vanish.
        assert_eq!(characters, vec!['a', 'b', 'c']);

        request.exclude_ambiguous = false;
        let characters = alphabet(&request);
        assert_eq!(characters, vec!['a', 'b', 'c', '0', 'O', '1', 'l', 'I']);
    }

    #[test]
    fn unicode_custom_characters_are_single_scalar_values() {
        let request = RandomStringRequest {
            length: 4,
            ..only_custom("αβγ😀")
        };
        let characters = alphabet(&request);
        assert_eq!(characters, vec!['α', 'β', 'γ', '😀']);

        let mut rng = StdRng::seed_from_u64(7);
        let evaluation = generate_with(&request, &mut rng);
        let values = evaluation.values();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].chars().count(), 4);
        assert!(values[0]
            .chars()
            .all(|character| characters.contains(&character)));
    }

    #[test]
    fn entropy_is_calculable_for_a_real_alphabet_and_not_below_two_symbols() {
        assert_eq!(entropy_bits(20, 2), Some(20.0));
        assert_eq!(entropy_bits(1, 4), Some(2.0));
        assert_eq!(entropy_bits(5, 1), None);
        assert_eq!(entropy_bits(5, 0), None);

        let request = RandomStringRequest::default();
        let characters = alphabet(&request);
        let expected = 20.0 * (characters.len() as f64).log2();
        assert_eq!(
            entropy_bits(request.length, characters.len()),
            Some(expected)
        );
    }

    #[test]
    fn validation_rejects_length_count_and_short_alphabets() {
        let too_short = only_custom("a");
        let evaluation = generate_with(&too_short, &mut StdRng::seed_from_u64(0));
        assert!(!evaluation.is_valid_operation());
        assert_eq!(evaluation.diagnostics().len(), 1);
        assert!(evaluation.diagnostics()[0].message.contains("two distinct"));

        let mut zero_length = only_custom("ab");
        zero_length.length = 0;
        let evaluation = generate_with(&zero_length, &mut StdRng::seed_from_u64(0));
        assert_eq!(evaluation.diagnostics().len(), 1);
        assert!(evaluation.diagnostics()[0].message.contains("Length"));

        let mut too_long = only_custom("ab");
        too_long.length = MAX_LENGTH + 1;
        assert!(generate_with(&too_long, &mut StdRng::seed_from_u64(0))
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Length")));

        let mut zero_count = only_custom("ab");
        zero_count.count = 0;
        assert!(generate_with(&zero_count, &mut StdRng::seed_from_u64(0))
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Count")));

        let mut too_many = only_custom("ab");
        too_many.count = MAX_COUNT + 1;
        assert!(generate_with(&too_many, &mut StdRng::seed_from_u64(0))
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Count")));
    }

    #[test]
    fn every_generated_symbol_and_length_stays_within_the_alphabet_and_bounds() {
        let request = RandomStringRequest {
            length: 12,
            count: 4,
            ..RandomStringRequest::default()
        };

        let mut rng = StdRng::seed_from_u64(2026);
        let evaluation = generate_with(&request, &mut rng);
        let characters = alphabet(&request);
        let values = evaluation.values();

        assert_eq!(values.len(), 4);
        for value in values {
            assert_eq!(value.chars().count(), 12);
            assert!(value
                .chars()
                .all(|character| characters.contains(&character)));
        }
        assert_eq!(
            evaluation.entropy_bits(),
            entropy_bits(12, characters.len())
        );
    }

    #[test]
    fn identical_seeds_produce_identical_values_and_different_seeds_differ() {
        let request = RandomStringRequest::default();
        let first = generate_with(&request, &mut StdRng::seed_from_u64(99));
        let second = generate_with(&request, &mut StdRng::seed_from_u64(99));
        let third = generate_with(&request, &mut StdRng::seed_from_u64(100));

        assert_eq!(first.values(), second.values());
        assert_ne!(first.values(), third.values());
    }

    #[test]
    fn rejection_sampling_discards_the_biased_tail_before_accepting() {
        // Alphabet of three: the accepted range is [0, 2^32 - (2^32 % 3)), so
        // u32::MAX falls in the rejected tail and must be skipped.
        let request = only_custom("abc");
        let mut rng = SequenceRng::new(vec![u32::MAX, 0]);

        let evaluation = generate_with(&request, &mut rng);

        assert_eq!(evaluation.values(), &["a".to_owned()]);
        assert_eq!(rng.index, 2, "the biased draw must be rejected and redrawn");
    }

    #[test]
    fn snapshot_round_trips_without_reevaluation_and_is_versioned() {
        let request = RandomStringRequest {
            count: 2,
            nonce: 5,
            ..RandomStringRequest::default()
        };

        let mut rng = StdRng::seed_from_u64(3);
        let evaluation = generate_with(&request, &mut rng);
        let snapshot =
            <RandomString as Utility>::snapshot(&request, &evaluation).expect("valid snapshot");

        assert_eq!(snapshot.restore().0, &request);
        assert_eq!(snapshot.restore().1, evaluation.values());

        let (restored_request, restored_evaluation) = <RandomString as Utility>::restore(&snapshot);
        assert_eq!(restored_request, request);
        assert_eq!(restored_evaluation.values(), evaluation.values());
        assert_eq!(
            restored_evaluation.entropy_bits(),
            evaluation.entropy_bits()
        );
        assert_eq!(RandomString::SNAPSHOT_VERSION, 1);
        assert_eq!(RandomString::ID, "random-string");
    }

    #[test]
    fn invalid_and_neutral_evaluations_never_snapshot() {
        let invalid = generate_with(&only_custom("a"), &mut StdRng::seed_from_u64(0));
        assert!(
            <RandomString as Utility>::snapshot(&RandomStringRequest::default(), &invalid)
                .is_none()
        );
        assert!(<RandomString as Utility>::snapshot(
            &RandomStringRequest::default(),
            &<RandomString as Utility>::neutral()
        )
        .is_none());
    }

    #[test]
    fn the_nonce_makes_deliberate_identical_requests_distinct() {
        let request = RandomStringRequest::default();
        let repeat = RandomStringRequest {
            nonce: request.nonce + 1,
            ..request.clone()
        };
        assert_ne!(request, repeat);
        assert_eq!(request.configuration(), repeat.configuration());
    }
}
