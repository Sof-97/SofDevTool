//! The Color Conversion Utility's GPUI-independent contract.
//!
//! Bounded CSS syntax: HEX, integer or percentage RGB(A), and degree/percentage
//! HSL(A). Every accepted value is normalized to 8-bit sRGB channels; wide-gamut
//! color spaces are out of scope. Outputs are deterministic and round-trip.

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Color Conversion Utility.
pub const COLOR_UTILITY_ID: &str = "color-conversion";

/// Schema version of [`ColorSnapshot`].
pub const COLOR_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SrgbColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl SrgbColor {
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        Self {
            red: quantize(red),
            green: quantize(green),
            blue: quantize(blue),
            alpha: quantize(alpha),
        }
    }
}

fn quantize(value: f64) -> f64 {
    (value * 255.0).round() / 255.0
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorOutputs {
    pub hex: String,
    pub rgb: String,
    pub hsl: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorRequest {
    pub source: String,
}

impl ColorRequest {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorEvaluation {
    /// Empty input is neutral: no output and no diagnostics.
    Empty,
    Valid {
        color: SrgbColor,
        outputs: ColorOutputs,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl ColorEvaluation {
    pub fn color(&self) -> Option<SrgbColor> {
        match self {
            ColorEvaluation::Valid { color, .. } => Some(*color),
            _ => None,
        }
    }

    pub fn outputs(&self) -> Option<&ColorOutputs> {
        match self {
            ColorEvaluation::Valid { outputs, .. } => Some(outputs),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            ColorEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, ColorEvaluation::Valid { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorSnapshot {
    pub source: String,
    pub color: SrgbColor,
    pub outputs: ColorOutputs,
}

impl ColorSnapshot {
    pub fn restore(&self) -> (&str, SrgbColor, &ColorOutputs) {
        (&self.source, self.color, &self.outputs)
    }
}

pub fn evaluate(request: &ColorRequest) -> ColorEvaluation {
    let value = request.source.trim();
    if value.is_empty() {
        return ColorEvaluation::Empty;
    }
    match parse(value) {
        Ok(color) => ColorEvaluation::Valid {
            color,
            outputs: outputs_for(color),
        },
        Err(message) => ColorEvaluation::Invalid {
            diagnostics: vec![Diagnostic::error(message)],
        },
    }
}

/// Parses a bounded CSS color into a normalized 8-bit sRGB color.
pub fn parse(source: &str) -> Result<SrgbColor, String> {
    let value = source.trim();
    if value.is_empty() {
        return Err("Enter a HEX, RGB, RGBA, HSL, or HSLA color.".to_owned());
    }
    if let Some(digits) = value.strip_prefix('#') {
        return parse_hex(digits);
    }
    let Some(open) = value.find('(') else {
        return Err(
            "Use #RGB, #RGBA, #RRGGBB, #RRGGBBAA, rgb/rgba, or hsl/hsla syntax.".to_owned(),
        );
    };
    if !value.ends_with(')') {
        return Err(
            "Use #RGB, #RGBA, #RRGGBB, #RRGGBBAA, rgb/rgba, or hsl/hsla syntax.".to_owned(),
        );
    }
    let name = value[..open].to_ascii_lowercase();
    let arguments = &value[open + 1..value.len() - 1];
    match name.as_str() {
        "rgb" | "rgba" => parse_rgb(arguments),
        "hsl" | "hsla" => parse_hsl(arguments),
        _ => Err("Unsupported color function. Use rgb, rgba, hsl, or hsla.".to_owned()),
    }
}

fn parse_hex(digits: &str) -> Result<SrgbColor, String> {
    if !matches!(digits.len(), 3 | 4 | 6 | 8)
        || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("HEX must contain 3, 4, 6, or 8 hexadecimal digits.".to_owned());
    }
    let expanded: String = if digits.len() <= 4 {
        digits.chars().flat_map(|c| [c, c]).collect()
    } else {
        digits.to_owned()
    };
    let channel = |offset: usize| -> f64 {
        let start = offset;
        u8::from_str_radix(&expanded[start..start + 2], 16).expect("validated hex") as f64 / 255.0
    };
    Ok(SrgbColor::new(
        channel(0),
        channel(2),
        channel(4),
        if expanded.len() == 8 { channel(6) } else { 1.0 },
    ))
}

fn parse_rgb(arguments: &str) -> Result<SrgbColor, String> {
    let (values, alpha) = components(arguments);
    if values.len() != 3 {
        return Err("RGB requires exactly three color components.".to_owned());
    }
    let percent = values.iter().all(|value| value.ends_with('%'));
    if !percent && values.iter().any(|value| value.ends_with('%')) {
        return Err("RGB components must be all integers or all percentages.".to_owned());
    }
    let divisor = if percent { 100.0 } else { 255.0 };
    let mut channels = [0.0_f64; 3];
    for (index, token) in values.iter().enumerate() {
        let raw = if percent {
            token.trim_end_matches('%')
        } else {
            token.as_str()
        };
        let number = finite(raw, "RGB component")?;
        if !(0.0..=divisor).contains(&number) || (!percent && number.round() != number) {
            return Err(if percent {
                "RGB percentages must be from 0% through 100%.".to_owned()
            } else {
                "RGB integer components must be from 0 through 255.".to_owned()
            });
        }
        channels[index] = number / divisor;
    }
    Ok(SrgbColor::new(
        channels[0],
        channels[1],
        channels[2],
        parse_alpha(alpha.as_deref())?,
    ))
}

fn parse_hsl(arguments: &str) -> Result<SrgbColor, String> {
    let (values, alpha) = components(arguments);
    if values.len() != 3 {
        return Err("HSL requires hue, saturation, and lightness.".to_owned());
    }
    let hue_token = values[0].to_ascii_lowercase();
    let hue = finite(hue_token.strip_suffix("deg").unwrap_or(&hue_token), "Hue")?;
    if !values[1].ends_with('%') || !values[2].ends_with('%') {
        return Err("HSL saturation and lightness require percentages.".to_owned());
    }
    let saturation = finite(values[1].trim_end_matches('%'), "Saturation")?;
    let lightness = finite(values[2].trim_end_matches('%'), "Lightness")?;
    if !(0.0..=100.0).contains(&saturation) || !(0.0..=100.0).contains(&lightness) {
        return Err("HSL saturation and lightness must be from 0% through 100%.".to_owned());
    }
    let normalized_hue = hue.rem_euclid(360.0) / 360.0;
    let saturation = saturation / 100.0;
    let lightness = lightness / 100.0;
    let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let h = normalized_hue * 6.0;
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h {
        h if h < 1.0 => (c, x, 0.0),
        h if h < 2.0 => (x, c, 0.0),
        h if h < 3.0 => (0.0, c, x),
        h if h < 4.0 => (0.0, x, c),
        h if h < 5.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = lightness - c / 2.0;
    Ok(SrgbColor::new(
        r + m,
        g + m,
        b + m,
        parse_alpha(alpha.as_deref())?,
    ))
}

fn components(input: &str) -> (Vec<String>, Option<String>) {
    let slash_parts: Vec<&str> = input.split('/').collect();
    if slash_parts.len() > 2 {
        return (Vec::new(), None);
    }
    let mut values: Vec<String> = if slash_parts[0].contains(',') {
        slash_parts[0]
            .split(',')
            .map(|part| part.trim().to_owned())
            .collect()
    } else {
        slash_parts[0]
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    };
    let mut alpha_value = slash_parts
        .get(1)
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty());
    if values.len() == 4 && alpha_value.is_none() {
        alpha_value = values.pop();
    }
    (values, alpha_value)
}

fn parse_alpha(token: Option<&str>) -> Result<f64, String> {
    let Some(token) = token else {
        return Ok(1.0);
    };
    let percent = token.ends_with('%');
    let number = finite(
        if percent {
            token.trim_end_matches('%')
        } else {
            token
        },
        "Alpha",
    )?;
    let maximum = if percent { 100.0 } else { 1.0 };
    if !(0.0..=maximum).contains(&number) {
        return Err(if percent {
            "Alpha must be from 0% through 100%.".to_owned()
        } else {
            "Alpha must be from 0 through 1.".to_owned()
        });
    }
    Ok(number / maximum)
}

fn finite(token: &str, name: &str) -> Result<f64, String> {
    match token.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(value),
        _ => Err(format!("{name} must be a finite number.")),
    }
}

/// The synchronized HEX/RGB/HSL representations of a normalized color.
pub fn outputs_for(color: SrgbColor) -> ColorOutputs {
    let channels = [
        (color.red * 255.0).round() as u8,
        (color.green * 255.0).round() as u8,
        (color.blue * 255.0).round() as u8,
    ];
    let alpha_channel = (color.alpha * 255.0).round() as u8;
    let mut hex = String::from("#");
    for channel in &channels {
        hex.push_str(&format!("{channel:02x}"));
    }
    if color.alpha != 1.0 {
        hex.push_str(&format!("{alpha_channel:02x}"));
    }
    let alpha_suffix = if color.alpha == 1.0 {
        String::new()
    } else {
        format!(" / {}", decimal(color.alpha))
    };
    let (hue, saturation, lightness) = rgb_to_hsl(color);
    ColorOutputs {
        hex,
        rgb: format!(
            "rgb({} {} {}{alpha_suffix})",
            channels[0], channels[1], channels[2]
        ),
        hsl: format!(
            "hsl({}deg {}% {}%{alpha_suffix})",
            decimal(hue),
            decimal(saturation * 100.0),
            decimal(lightness * 100.0)
        ),
    }
}

fn rgb_to_hsl(color: SrgbColor) -> (f64, f64, f64) {
    let maximum = color.red.max(color.green).max(color.blue);
    let minimum = color.red.min(color.green).min(color.blue);
    let delta = maximum - minimum;
    let lightness = (maximum + minimum) / 2.0;
    if delta == 0.0 {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let raw = if maximum == color.red {
        ((color.green - color.blue) / delta).rem_euclid(6.0)
    } else if maximum == color.green {
        (color.blue - color.red) / delta + 2.0
    } else {
        (color.red - color.green) / delta + 4.0
    };
    ((raw * 60.0).rem_euclid(360.0), saturation, lightness)
}

/// Formats with at most four decimals, trimming trailing zeros and `-0`.
fn decimal(value: f64) -> String {
    let mut result = format!("{value:.4}");
    while result.contains('.') && result.ends_with('0') {
        result.pop();
    }
    if result.ends_with('.') {
        result.pop();
    }
    if result == "-0" {
        "0".to_owned()
    } else {
        result
    }
}

/// The Color Conversion Utility's identity for the shared [`Utility`] trait.
pub struct ColorConversion;

impl Utility for ColorConversion {
    type Request = ColorRequest;
    type Evaluation = ColorEvaluation;
    type Snapshot = ColorSnapshot;

    const ID: &'static str = COLOR_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = COLOR_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> ColorEvaluation {
        ColorEvaluation::Empty
    }

    fn evaluate(request: &ColorRequest) -> ColorEvaluation {
        evaluate(request)
    }

    fn is_neutral(evaluation: &ColorEvaluation) -> bool {
        matches!(evaluation, ColorEvaluation::Empty)
    }

    fn snapshot(request: &ColorRequest, evaluation: &ColorEvaluation) -> Option<ColorSnapshot> {
        match evaluation {
            ColorEvaluation::Valid { color, outputs } => Some(ColorSnapshot {
                source: request.source.clone(),
                color: *color,
                outputs: outputs.clone(),
            }),
            _ => None,
        }
    }

    fn restore(snapshot: &ColorSnapshot) -> (ColorRequest, ColorEvaluation) {
        (
            ColorRequest::new(snapshot.source.clone()),
            ColorEvaluation::Valid {
                color: snapshot.color,
                outputs: snapshot.outputs.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(source: &str) -> SrgbColor {
        parse(source).expect("valid color")
    }

    fn outputs(source: &str) -> ColorOutputs {
        evaluate(&ColorRequest::new(source))
            .outputs()
            .expect("valid outputs")
            .clone()
    }

    #[test]
    fn hex_and_rgb_produce_the_same_representations() {
        for source in ["#ff0000", "#f00", "rgb(255 0 0)", "rgb(255, 0, 0)"] {
            let outputs = outputs(source);
            assert_eq!(outputs.hex, "#ff0000");
            assert_eq!(outputs.rgb, "rgb(255 0 0)");
            assert_eq!(outputs.hsl, "hsl(0deg 100% 50%)");
        }
    }

    #[test]
    fn shorthand_hex_expands_and_alpha_is_preserved() {
        assert_eq!(parsed("#f00"), parsed("#ff0000"));
        assert_eq!(parsed("#0f08"), parsed("#00ff0088"));
        let outputs = outputs("#00ff0088");
        assert_eq!(outputs.hex, "#00ff0088");
        assert!(outputs.rgb.contains("rgb(0 255 0 / 0.5333)"));
    }

    #[test]
    fn percentages_and_alpha_forms_are_supported() {
        assert_eq!(parsed("rgb(100% 0% 0%)"), parsed("#ff0000"));
        assert_eq!(parsed("rgba(255, 0, 0, 0.5)"), parsed("#ff000080"));
        assert_eq!(parsed("rgb(255 0 0 / 50%)"), parsed("#ff000080"));
    }

    #[test]
    fn hsl_round_trips_through_rgb() {
        assert_eq!(parsed("hsl(120deg 100% 50%)"), parsed("#00ff00"));
        assert_eq!(parsed("hsl(240, 100%, 50%)"), parsed("#0000ff"));
        assert_eq!(outputs("#00ff00").hsl, "hsl(120deg 100% 50%)");
        // Hue wraps around.
        assert_eq!(parsed("hsl(480 100% 50%)"), parsed("#00ff00"));
    }

    #[test]
    fn invalid_sources_produce_diagnostics_and_no_output() {
        for source in [
            "#12",
            "#gggggg",
            "rgb(1, 2)",
            "rgb(1 2 300)",
            "rgb(1 2% 3)",
            "hsl(10 50 50)",
            "hsl(10 200% 50%)",
            "cmyk(0 0 0 0)",
            "rgb(1 2 3",
        ] {
            let evaluation = evaluate(&ColorRequest::new(source));
            assert!(!evaluation.is_valid_operation(), "{source} must be invalid");
            assert!(evaluation.diagnostics().len() == 1);
        }
    }

    #[test]
    fn empty_input_is_neutral() {
        assert_eq!(evaluate(&ColorRequest::new("  ")), ColorEvaluation::Empty);
    }

    #[test]
    fn snapshot_round_trips_without_reevaluation() {
        let request = ColorRequest::new("#336699");
        let evaluation = evaluate(&request);
        let snapshot =
            <ColorConversion as Utility>::snapshot(&request, &evaluation).expect("snapshot");
        assert_eq!(snapshot.outputs.hex, "#336699");
        let (restored_request, restored_evaluation) =
            <ColorConversion as Utility>::restore(&snapshot);
        assert_eq!(restored_request.source, "#336699");
        assert_eq!(restored_evaluation.color(), Some(parsed("#336699")));
    }
}
