//! Independent literal vectors retained from the Swift contract suite where
//! the current Rust module tests covered the general rule but not this input.
//!
//! Expected values below are constants transcribed from the former Swift
//! expectations or independently hand-reviewed boundary outputs. These tests
//! exercise public request/evaluation contracts and do not compute their own
//! expected results from the implementation.

use sofdevtool_core::utilities::{
    base64::{self, Base64Mode, Base64Request},
    case_conversion,
    color::{self, ColorRequest},
    hashes::{self, HashAlgorithm, HashRepresentation, HashesRequest},
    jwt::{self, JwtRequest},
    regex::{self, RegexRequest},
    timestamps::{self, TimestampMode, TimestampsRequest},
    url_encoding::{self, UrlEncodingDirection, UrlEncodingMode, UrlEncodingRequest},
};

#[test]
fn cjk_script_switches_cleanly_to_ascii_case_word() {
    assert_eq!(case_conversion::segment("東京Value"), ["東京", "Value"]);
    assert_eq!(
        case_conversion::segment("go👩🏽\u{200D}💻Now"),
        ["go", "👩🏽\u{200D}💻", "Now"]
    );
}

#[test]
fn achromatic_hsla_keeps_the_legacy_eight_bit_output_vector() {
    let evaluation = color::evaluate(&ColorRequest::new("hsla(240deg, 0%, 50%, 0.5)"));
    let output = evaluation.outputs().expect("valid achromatic HSL input");
    assert_eq!(output.hex, "#80808080");
    assert_eq!(output.rgb, "rgb(128 128 128 / 0.502)");
    assert_eq!(output.hsl, "hsl(0deg 0% 50.1961% / 0.502)");

    let teal = color::evaluate(&ColorRequest::new("rgb(0%, 50%, 50% / 25%)"));
    assert_eq!(teal.outputs().unwrap().hex, "#00808040");
}

#[test]
fn unsupported_legacy_color_inputs_remain_outside_the_srgb_contract() {
    for input in [
        "rgb(256 0 0)",
        "rgb(nan 0 0)",
        "hsl(0 101% 50%)",
        "display-p3(1 0 0)",
    ] {
        assert!(
            color::evaluate(&ColorRequest::new(input))
                .outputs()
                .is_none(),
            "{input} must remain unsupported"
        );
    }
}

#[test]
fn base64_decodes_the_retained_accent_and_skin_tone_zwj_vector() {
    let mut request = Base64Request::new("Y2Fmw6kg8J+RqfCfj73igI3wn5K7");
    request.mode = Base64Mode::Decode;
    assert_eq!(base64::evaluate(&request).output(), Some("café 👩🏽‍💻"));
}

#[test]
fn jwt_keeps_an_expired_claim_as_opaque_payload_data() {
    let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJhZG1pbiI6dHJ1ZSwiZXhwIjowLCJuYW1lIjoiY2Fmw6kg8J-RqfCfj73igI3wn5K7In0.opaque-signature";
    let evaluation = jwt::evaluate(&JwtRequest::new(token));

    assert_eq!(
        evaluation.payload(),
        Some("{\n  \"admin\": true,\n  \"exp\": 0,\n  \"name\": \"café 👩🏽‍💻\"\n}")
    );
    assert_eq!(evaluation.signature(), Some("opaque-signature"));
}

#[test]
fn composite_unicode_hashes_match_the_retained_independent_goldens() {
    let input = "café 👩🏽‍💻";
    let expected = [
        (
            HashAlgorithm::Sha256,
            "2df44393dea8fecf872bddf5ae1bd776d959c1a4f93655f5555de284bcf83773",
        ),
        (
            HashAlgorithm::Sha384,
            "9eec581cf74a0324ed7720dd1c3b7fff86ec1c93875fac32e2f7a899187738b1fc458993a1566d351b93131dd72a3d74",
        ),
        (
            HashAlgorithm::Sha512,
            "9b503fdad948a40e64b30b4ee8c81fa15bfdbd4e1b6ce238b7a3753045123d6720266aa19529b441834cbe69dd6cb26d4816ef90f0027b2130f86af4999c8f06",
        ),
        (
            HashAlgorithm::Sha1,
            "79a78ecdc143354466be7d937183fc45199a4439",
        ),
        (
            HashAlgorithm::Md5,
            "096743d25698360d5876d4ed9e294eb4",
        ),
    ];

    for (algorithm, hexadecimal) in expected {
        let request = HashesRequest {
            input: input.to_owned(),
            algorithm,
            representation: HashRepresentation::LowercaseHex,
        };
        assert_eq!(hashes::evaluate(&request).output(), Some(hexadecimal));
    }

    let uppercase = HashesRequest {
        input: input.to_owned(),
        algorithm: HashAlgorithm::Sha256,
        representation: HashRepresentation::UppercaseHex,
    };
    assert_eq!(
        hashes::evaluate(&uppercase).output(),
        Some("2DF44393DEA8FECF872BDDF5AE1BD776D959C1A4F93655F5555DE284BCF83773")
    );

    let base64 = HashesRequest {
        representation: HashRepresentation::Base64,
        ..uppercase
    };
    assert_eq!(
        hashes::evaluate(&base64).output(),
        Some("LfRDk96o/s+HK931rhvXdtlZwaT5NlX1VV3ihLz4N3M=")
    );
}

#[test]
fn negative_half_second_and_rome_2026_dst_boundaries_are_stable() {
    let half_second = timestamps::evaluate(&TimestampsRequest::typed(
        "-0.5",
        TimestampMode::UnixSeconds,
        "Europe/Rome",
    ));
    let instant = half_second.instant().expect("valid fractional epoch");
    assert_eq!(instant.seconds, -1);
    assert_eq!(instant.nanoseconds, 500_000_000);
    assert_eq!(
        half_second.representations().unwrap().iso8601,
        "1969-12-31T23:59:59.500Z"
    );

    for local_time in ["2026-03-29 02:30:00", "2026-10-25 02:30:00"] {
        let evaluation = timestamps::evaluate(&TimestampsRequest::typed(
            local_time,
            TimestampMode::Local,
            "Europe/Rome",
        ));
        assert!(
            !evaluation.is_valid_operation(),
            "{local_time} must be a nonexistent or repeated Rome wall time"
        );
    }

    assert!(timestamps::evaluate(&TimestampsRequest::typed(
        "2026-01-15 12:30:00",
        TimestampMode::Local,
        "Europe/Rome",
    ))
    .is_valid_operation());
}

#[test]
fn percent_decode_combines_decomposed_unicode_cjk_and_literal_plus() {
    let mut request = UrlEncodingRequest::new("e%CC%81+%E6%BC%A2%E5%AD%97");
    request.direction = UrlEncodingDirection::Decode;
    request.mode = UrlEncodingMode::QueryValue;
    assert_eq!(
        url_encoding::evaluate(&request).output(),
        Some("e\u{301}+漢字")
    );
}

#[test]
fn unicode_case_insensitive_capture_and_replacement_vector_uses_rust_named_groups() {
    let mut request = RegexRequest::new(r"(?P<word>café)\s+(\d+)", "CAFÉ 12 and café 34");
    request.flags.case_insensitive = true;
    request.replacement = "$2:$1".to_owned();
    let evaluation = regex::evaluate(&request);

    assert_eq!(
        evaluation
            .matches()
            .iter()
            .map(|matched| matched.value.as_str())
            .collect::<Vec<_>>(),
        ["CAFÉ 12", "café 34"]
    );
    assert_eq!(
        evaluation.matches()[0].captures[1].name.as_deref(),
        Some("word")
    );
    assert_eq!(
        evaluation.matches()[0].captures[1].value.as_deref(),
        Some("CAFÉ")
    );
    assert_eq!(
        evaluation.matches()[0].captures[2].value.as_deref(),
        Some("12")
    );
    assert_eq!(evaluation.replacement(), Some("12:CAFÉ and 34:café"));
}
