//! Contract tests for the JSON Utility boundary.
//!
//! These tests exercise the public request/result surface only. Expected values are
//! hand-authored fixtures or literals from JSON semantics, never recomputed from the
//! engine implementation.

use std::fs;
use std::path::PathBuf;

use sofdevtool_core::json::{
    evaluate, Diagnostic, Indentation, JsonEvaluation, JsonMode, JsonRequest, JsonSnapshot,
    Severity, JSON_SNAPSHOT_SCHEMA_VERSION, JSON_UTILITY_ID, MAX_NESTING_DEPTH,
};

fn req(input: &str, mode: JsonMode) -> JsonRequest {
    JsonRequest {
        input: input.to_string(),
        mode,
        indentation: Indentation::TwoSpaces,
        sort_keys: false,
        query: String::new(),
    }
}

fn output_of(evaluation: &JsonEvaluation) -> &str {
    match evaluation {
        JsonEvaluation::Valid { output } => output,
        other => panic!("expected valid output, got {other:?}"),
    }
}

fn diagnostics_of(evaluation: &JsonEvaluation) -> &[Diagnostic] {
    match evaluation {
        JsonEvaluation::Invalid { diagnostics } => diagnostics,
        other => panic!("expected invalid diagnostics, got {other:?}"),
    }
}

#[test]
fn empty_input_is_neutral() {
    assert_eq!(evaluate(&req("", JsonMode::Format)), JsonEvaluation::Empty);
    assert_eq!(
        evaluate(&req("   \n\t ", JsonMode::Format)),
        JsonEvaluation::Empty
    );
    assert_eq!(evaluate(&req("", JsonMode::Query)), JsonEvaluation::Empty);
}

#[test]
fn format_uses_two_space_indentation_by_default() {
    let input = r#"{"b":1,"a":[1,2,{"c":3}]}"#;
    let expected =
        "{\n  \"b\": 1,\n  \"a\": [\n    1,\n    2,\n    {\n      \"c\": 3\n    }\n  ]\n}";
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Format))),
        expected
    );
}

#[test]
fn format_honours_four_space_indentation() {
    let mut request = req(r#"{"a":{"b":1}}"#, JsonMode::Format);
    request.indentation = Indentation::FourSpaces;
    let expected = "{\n    \"a\": {\n        \"b\": 1\n    }\n}";
    assert_eq!(output_of(&evaluate(&request)), expected);
}

#[test]
fn minify_removes_insignificant_whitespace() {
    let input = "{\n  \"a\": 1,\n  \"b\": [1, 2]\n}";
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Minify))),
        r#"{"a":1,"b":[1,2]}"#
    );
}

#[test]
fn sort_keys_is_recursive_and_preserves_array_order() {
    let input = r#"{"z":1,"a":{"d":4,"b":2},"arr":[3,1,2]}"#;
    let mut request = req(input, JsonMode::Format);
    request.sort_keys = true;
    let expected = "{\n  \"a\": {\n    \"b\": 2,\n    \"d\": 4\n  },\n  \"arr\": [\n    3,\n    1,\n    2\n  ],\n  \"z\": 1\n}";
    assert_eq!(output_of(&evaluate(&request)), expected);
}

#[test]
fn preserves_numeric_meaning() {
    // Numbers are re-emitted from their exact source literal, so large integers,
    // decimals and exponent spellings all survive unchanged.
    let input = r#"{"big":123456789012345678901234567890,"float":1.0,"exp":1e3,"negzero":-0.0}"#;
    let expected = "{\n  \"big\": 123456789012345678901234567890,\n  \"float\": 1.0,\n  \"exp\": 1e3,\n  \"negzero\": -0.0\n}";
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Format))),
        expected
    );
}

#[test]
fn a_user_object_key_named_like_serde_private_number_stays_an_object() {
    // Regression: `serde_json`'s `arbitrary_precision` mode would rewrite this
    // user object into the scalar 123 and reject the non-numeric variant.
    let input = r#"{"$serde_json::private::Number":"123"}"#;
    let expected = "{\n  \"$serde_json::private::Number\": \"123\"\n}";
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Format))),
        expected
    );

    let nested = r#"{"a":{"$serde_json::private::Number":"hello"}}"#;
    let nested_expected = "{\n  \"a\": {\n    \"$serde_json::private::Number\": \"hello\"\n  }\n}";
    assert_eq!(
        output_of(&evaluate(&req(nested, JsonMode::Format))),
        nested_expected
    );
}

#[test]
fn unicode_escapes_and_surrogate_pairs_decode() {
    let input = r#"{"emoji":"\ud83d\udc69\u200d\ud83d\udcbb","accent":"caf\u00e9"}"#;
    let expected = "{\n  \"emoji\": \"👩‍💻\",\n  \"accent\": \"café\"\n}";
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Format))),
        expected
    );
}

#[test]
fn duplicate_object_keys_keep_the_last_value() {
    let input = r#"{"a":1,"a":2}"#;
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Minify))),
        r#"{"a":2}"#
    );
}

#[test]
fn preserves_unicode_and_complex_emoji_verbatim() {
    let input = "{\"text\":\"caffè 👩‍👩‍👧‍👦 🏳️‍🌈 𝄞\"}";
    let expected = "{\n  \"text\": \"caffè 👩‍👩‍👧‍👦 🏳️‍🌈 𝄞\"\n}";
    assert_eq!(
        output_of(&evaluate(&req(input, JsonMode::Format))),
        expected
    );
}

#[test]
fn invalid_input_reports_line_and_column() {
    let input = "{\n  \"a\": 1,\n  \"b\": ,\n}";
    let evaluation = evaluate(&req(input, JsonMode::Format));
    let diagnostics = diagnostics_of(&evaluation);
    assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
    let location = diagnostics
        .iter()
        .find_map(|d| d.location)
        .expect("an error diagnostic should carry a source location");
    assert_eq!(location.line, 3);
    assert!(
        location.column >= 7 && location.column <= 10,
        "column was {}",
        location.column
    );
}

#[test]
fn invalid_input_never_returns_output() {
    let evaluation = evaluate(&req("{", JsonMode::Format));
    assert!(matches!(evaluation, JsonEvaluation::Invalid { .. }));
}

#[test]
fn query_supports_json_pointer() {
    let input = r#"{"store":{"book":[{"title":"A"},{"title":"B"}]}}"#;
    let mut request = req(input, JsonMode::Query);
    request.query = "/store/book/1/title".to_string();
    assert_eq!(output_of(&evaluate(&request)), "B");
}

#[test]
fn query_pointer_unescapes_tokens() {
    let input = r#"{"a/b":{"~key":7}}"#;
    let mut request = req(input, JsonMode::Query);
    request.query = "/a~1b/~0key".to_string();
    assert_eq!(output_of(&evaluate(&request)), "7");
}

#[test]
fn query_supports_dot_and_bracket_paths() {
    let input = r#"{"items":[{"name":"x"},{"name":"y"}]}"#;
    let mut request = req(input, JsonMode::Query);
    request.query = "items[1].name".to_string();
    assert_eq!(output_of(&evaluate(&request)), "y");
}

#[test]
fn query_of_container_renders_structured_output() {
    // Swift baseline: queried containers are always recursively sorted with
    // two-space indentation, independent of the Format options.
    let input = r#"{"b":1,"a":2}"#;
    let mut request = req(input, JsonMode::Query);
    request.query = String::new();
    request.indentation = Indentation::FourSpaces;
    request.sort_keys = false;
    assert_eq!(
        output_of(&evaluate(&request)),
        "{\n  \"a\": 2,\n  \"b\": 1\n}"
    );
}

#[test]
fn nesting_limit_counts_containers_consistently() {
    // Only containers count toward the budget, so a scalar leaf and an empty
    // leaf are limited identically (128 allowed, 129 refused).
    let leaf_at_limit = format!(
        "{}{}{}",
        "[".repeat(MAX_NESTING_DEPTH),
        "0",
        "]".repeat(MAX_NESTING_DEPTH)
    );
    assert!(matches!(
        evaluate(&req(&leaf_at_limit, JsonMode::Minify)),
        JsonEvaluation::Valid { .. }
    ));

    let leaf_too_deep = format!(
        "{}{}{}",
        "[".repeat(MAX_NESTING_DEPTH + 1),
        "0",
        "]".repeat(MAX_NESTING_DEPTH + 1)
    );
    assert!(matches!(
        evaluate(&req(&leaf_too_deep, JsonMode::Minify)),
        JsonEvaluation::Invalid { .. }
    ));

    let empty_at_limit = format!(
        "{}{}",
        "[".repeat(MAX_NESTING_DEPTH),
        "]".repeat(MAX_NESTING_DEPTH)
    );
    assert!(matches!(
        evaluate(&req(&empty_at_limit, JsonMode::Minify)),
        JsonEvaluation::Valid { .. }
    ));

    let empty_too_deep = format!(
        "{}{}",
        "[".repeat(MAX_NESTING_DEPTH + 1),
        "]".repeat(MAX_NESTING_DEPTH + 1)
    );
    let evaluation = evaluate(&req(&empty_too_deep, JsonMode::Minify));
    let diagnostics = diagnostics_of(&evaluation);
    assert!(diagnostics
        .iter()
        .any(|d| d.message.contains("nesting depth")));
    assert!(evaluation.output().is_none());
}

#[test]
fn nesting_limit_applies_to_objects_too() {
    let at_limit = format!(
        "{}0{}",
        "{\"a\":".repeat(MAX_NESTING_DEPTH),
        "}".repeat(MAX_NESTING_DEPTH)
    );
    assert!(matches!(
        evaluate(&req(&at_limit, JsonMode::Minify)),
        JsonEvaluation::Valid { .. }
    ));

    let too_deep = format!(
        "{}0{}",
        "{\"a\":".repeat(MAX_NESTING_DEPTH + 1),
        "}".repeat(MAX_NESTING_DEPTH + 1)
    );
    assert!(matches!(
        evaluate(&req(&too_deep, JsonMode::Minify)),
        JsonEvaluation::Invalid { .. }
    ));
}

#[test]
fn hostile_deep_input_is_refused_without_crashing() {
    // 50_000 nested arrays previously aborted the process with a stack overflow.
    let input = format!("{}{}", "[".repeat(50_000), "]".repeat(50_000));
    let evaluation = evaluate(&req(&input, JsonMode::Format));
    let diagnostics = diagnostics_of(&evaluation);
    assert!(diagnostics
        .iter()
        .any(|d| d.message.contains("nesting depth")));
    assert!(evaluation.output().is_none());
}

#[test]
fn query_of_null_and_scalars_is_literal() {
    // Baseline compatibility: queried booleans render as NSNumber-style 1/0.
    let input = r#"{"n":null,"t":true,"f":false,"s":"hi","n2":2.5}"#;
    for (path, expected) in [
        ("n", "null"),
        ("t", "1"),
        ("f", "0"),
        ("s", "hi"),
        ("n2", "2.5"),
    ] {
        let mut request = req(input, JsonMode::Query);
        request.query = path.to_string();
        assert_eq!(output_of(&evaluate(&request)), expected, "path {path}");
    }
}

#[test]
fn query_missing_path_is_an_error() {
    let input = r#"{"a":1}"#;
    let mut request = req(input, JsonMode::Query);
    request.query = "a.b".to_string();
    let evaluation = evaluate(&request);
    let diagnostics = diagnostics_of(&evaluation);
    assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn snapshot_round_trips_without_duplicating_input() {
    let request = req(r#"{"a":1}"#, JsonMode::Format);
    let snapshot = JsonSnapshot {
        output: "{\n  \"a\": 1\n}".to_string(),
        request: request.clone(),
    };
    let encoded = serde_json::to_string(&snapshot).unwrap();
    let decoded: JsonSnapshot = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, snapshot);
    // The input lives only on the request; the snapshot adds no second copy.
    assert_eq!(decoded.request.input, r#"{"a":1}"#);
    let encoded_value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let keys: Vec<&str> = encoded_value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, ["output", "request"]);
    assert_eq!(JSON_UTILITY_ID, "json");
    assert_eq!(JSON_SNAPSHOT_SCHEMA_VERSION, 1);
}

#[test]
fn fixture_corpus_matches_expected_results() {
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        #[serde(default = "default_mode")]
        mode: String,
        #[serde(default = "default_indentation")]
        indentation: u8,
        #[serde(default)]
        sort_keys: bool,
        #[serde(default)]
        input: String,
        #[serde(default)]
        query: String,
        #[serde(default)]
        expected_output: Option<String>,
        #[serde(default)]
        expected_error: Option<ExpectedError>,
    }

    #[derive(serde::Deserialize)]
    struct ExpectedError {
        message_contains: String,
        #[serde(default)]
        line: Option<u32>,
    }

    fn default_mode() -> String {
        "format".to_string()
    }
    fn default_indentation() -> u8 {
        2
    }

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cases");
    let mut cases: Vec<Case> = Vec::new();
    for entry in fs::read_dir(&dir).expect("fixtures/cases must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap();
        let case: Case = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("fixture {} is malformed: {e}", path.display()));
        cases.push(case);
    }
    assert!(cases.len() >= 8, "expected a meaningful fixture corpus");

    for case in cases {
        let mode = match case.mode.as_str() {
            "format" => JsonMode::Format,
            "minify" => JsonMode::Minify,
            "query" => JsonMode::Query,
            other => panic!("unknown mode {other} in {}", case.name),
        };
        let indentation = match case.indentation {
            2 => Indentation::TwoSpaces,
            4 => Indentation::FourSpaces,
            other => panic!("unknown indentation {other} in {}", case.name),
        };
        let request = JsonRequest {
            input: case.input.clone(),
            mode,
            indentation,
            sort_keys: case.sort_keys,
            query: case.query.clone(),
        };
        let evaluation = evaluate(&request);
        if let Some(expected) = &case.expected_output {
            let actual = match &evaluation {
                JsonEvaluation::Valid { output } => output.clone(),
                JsonEvaluation::Empty => String::new(),
                JsonEvaluation::Invalid { .. } => String::new(),
            };
            assert_eq!(actual, *expected, "fixture {} output mismatch", case.name);
        }
        if let Some(expected_error) = &case.expected_error {
            let diagnostics = diagnostics_of(&evaluation);
            let matched = diagnostics.iter().any(|d| {
                d.message.contains(&expected_error.message_contains)
                    && expected_error
                        .line
                        .map(|line| d.location.map(|l| l.line == line).unwrap_or(false))
                        .unwrap_or(true)
            });
            assert!(
                matched,
                "fixture {} expected error containing {:?}, got {diagnostics:?}",
                case.name, expected_error.message_contains
            );
        }
    }
}
