//! The source-defined Utility catalog shared by the Workbench and Launcher.
//!
//! Every Utility keeps its strong concrete workspace type. The catalog stores a
//! constructor function pointer only so the Workbench can build heterogeneous
//! workspaces on demand; nothing here erases a Utility's request/result type.

use std::rc::Rc;

use gpui::{AnyView, Context, Window};

use crate::clipboard::Clipboard;
use crate::history::HistoryRecorder;
use crate::workbench::Workbench;

/// The stable identity of a source-defined Utility.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UtilityId {
    Json,
    TextDiff,
    Base64,
    UrlEncoding,
    CaseConversion,
    Whitespace,
    Hashes,
    RandomString,
    Color,
    Jwt,
    Identifiers,
    Regex,
    Timestamps,
    SampleData,
    YamlJson,
}

impl UtilityId {
    /// Stable History filename / preference key for this Utility.
    pub const fn slug(self) -> &'static str {
        match self {
            UtilityId::Json => "json",
            UtilityId::TextDiff => "text-diff",
            UtilityId::Base64 => "base64",
            UtilityId::UrlEncoding => "url-encoding",
            UtilityId::CaseConversion => "case-conversion",
            UtilityId::Whitespace => "whitespace-conversion",
            UtilityId::Hashes => "hashes",
            UtilityId::RandomString => "random-string",
            UtilityId::Color => "color-conversion",
            UtilityId::Jwt => "jwt-decoder",
            UtilityId::Identifiers => "identifiers",
            UtilityId::Regex => "rust-regex",
            UtilityId::Timestamps => "timestamps",
            UtilityId::SampleData => "sample-data",
            UtilityId::YamlJson => "yaml-json",
        }
    }
}

/// A request from a discovery surface to reveal one registered Utility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenUtility(pub UtilityId);

/// Builds a Utility's concrete workspace as a type-erased view. Only the
/// Workbench calls this, at the heterogeneous composition boundary.
pub type WorkspaceConstructor =
    fn(&mut Window, &mut Context<Workbench>, Rc<dyn Clipboard>, Rc<HistoryRecorder>) -> AnyView;

/// Immutable discovery metadata. Construction of the concrete workspace stays
/// in the Workbench so each Utility retains its strong concrete type.
#[derive(Clone, Copy, Debug)]
pub struct UtilityDefinition {
    pub id: UtilityId,
    pub name: &'static str,
    pub summary: &'static str,
    pub category: &'static str,
    pub aliases: &'static [&'static str],
    /// Whether History records this Utility by default. JWT opts out.
    pub history_enabled_by_default: bool,
    /// `None` for the two workspaces the Workbench keeps strongly typed.
    pub construct: Option<WorkspaceConstructor>,
}

// Function pointers are deliberately excluded: two definitions describe the same
// Utility when their discovery metadata matches, regardless of constructor identity.
impl PartialEq for UtilityDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.summary == other.summary
            && self.category == other.category
            && self.aliases == other.aliases
    }
}

impl Eq for UtilityDefinition {}

impl UtilityDefinition {
    pub const fn json() -> Self {
        Self {
            id: UtilityId::Json,
            name: "JSON",
            summary: "Format, minify, validate, and query JSON",
            category: "Format & Convert",
            aliases: &["format", "validate", "minify", "query"],
            history_enabled_by_default: true,
            construct: None,
        }
    }

    pub const fn text_diff() -> Self {
        Self {
            id: UtilityId::TextDiff,
            name: "Text Diff",
            summary: "Compare text with split or unified output",
            category: "Text",
            aliases: &["diff", "compare", "patch"],
            history_enabled_by_default: true,
            construct: None,
        }
    }

    pub const fn base64() -> Self {
        Self {
            id: UtilityId::Base64,
            name: "Base64",
            summary: "Encode and decode UTF-8 with explicit alphabet and padding",
            category: "Format & Convert",
            aliases: &["encode", "decode", "base64url"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::base64::construct),
        }
    }

    pub const fn url_encoding() -> Self {
        Self {
            id: UtilityId::UrlEncoding,
            name: "URL Encoding",
            summary: "Percent-encode path segments and query values",
            category: "Format & Convert",
            aliases: &["percent", "urlencode", "escape", "uri"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::url_encoding::construct),
        }
    }

    pub const fn case_conversion() -> Self {
        Self {
            id: UtilityId::CaseConversion,
            name: "Case Conversion",
            summary: "Convert text through nine developer case styles",
            category: "Text",
            aliases: &["camel", "snake", "kebab", "pascal", "title"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::case_conversion::construct),
        }
    }

    pub const fn whitespace() -> Self {
        Self {
            id: UtilityId::Whitespace,
            name: "Whitespace",
            summary: "Trim, collapse, normalize and dedent whitespace",
            category: "Text",
            aliases: &["trim", "tabs", "indent", "line endings"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::whitespace::construct),
        }
    }

    pub const fn hashes() -> Self {
        Self {
            id: UtilityId::Hashes,
            name: "Hashes",
            summary: "SHA-256/384/512, SHA-1 and MD5 with hex or Base64 output",
            category: "Format & Convert",
            aliases: &["sha256", "sha1", "md5", "digest", "checksum"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::hashes::construct),
        }
    }

    pub const fn random_string() -> Self {
        Self {
            id: UtilityId::RandomString,
            name: "Random String",
            summary: "Cryptographically random strings with explicit character controls",
            category: "Generate",
            aliases: &["random", "password", "token", "entropy"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::random_string::construct),
        }
    }

    pub const fn color() -> Self {
        Self {
            id: UtilityId::Color,
            name: "Color Conversion",
            summary: "Convert and select bounded sRGB HEX, RGB(A) and HSL(A)",
            category: "Format & Convert",
            aliases: &["hex", "rgb", "hsl", "colour"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::color::construct),
        }
    }

    pub const fn jwt() -> Self {
        Self {
            id: UtilityId::Jwt,
            name: "JWT Decoder",
            summary: "Inspect readable JWT header and payload segments",
            category: "Inspect",
            aliases: &["token", "jwt", "claims", "bearer"],
            history_enabled_by_default: false,
            construct: Some(crate::utilities::jwt::construct),
        }
    }

    pub const fn identifiers() -> Self {
        Self {
            id: UtilityId::Identifiers,
            name: "Identifier Generator",
            summary: "Generate and inspect UUID identifiers",
            category: "Generate",
            aliases: &["uuid", "guid", "v4", "v7"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::identifiers::construct),
        }
    }

    pub const fn regex() -> Self {
        Self {
            id: UtilityId::Regex,
            name: "Regex",
            summary: "Test patterns and replacements with the Rust regex engine",
            category: "Text",
            aliases: &["regexp", "pattern", "capture", "replace"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::regex::construct),
        }
    }

    pub const fn timestamps() -> Self {
        Self {
            id: UtilityId::Timestamps,
            name: "Timestamps",
            summary: "Interpret Unix, ISO 8601 and named-zone local time",
            category: "Format & Convert",
            aliases: &["time", "date", "unix", "epoch", "iso8601", "timezone"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::timestamps::construct),
        }
    }

    pub const fn sample_data() -> Self {
        Self {
            id: UtilityId::SampleData,
            name: "Sample Data",
            summary: "Generate fictional typed JSON or CSV rows",
            category: "Generate",
            aliases: &["fixture", "mock", "csv", "rows", "test data"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::sample_data::construct),
        }
    }

    pub const fn yaml_json() -> Self {
        Self {
            id: UtilityId::YamlJson,
            name: "YAML / JSON",
            summary: "Convert one YAML 1.2 document to or from JSON",
            category: "Format & Convert",
            aliases: &["yaml", "yml", "convert", "anchors"],
            history_enabled_by_default: true,
            construct: Some(crate::utilities::yaml_json::construct),
        }
    }
}

/// The authoritative, source-defined Utility catalog.
#[derive(Clone, Debug)]
pub struct UtilityRegistry {
    definitions: Vec<UtilityDefinition>,
}

impl UtilityRegistry {
    pub fn initial() -> Self {
        Self {
            definitions: vec![
                UtilityDefinition::json(),
                UtilityDefinition::text_diff(),
                UtilityDefinition::base64(),
                UtilityDefinition::url_encoding(),
                UtilityDefinition::case_conversion(),
                UtilityDefinition::whitespace(),
                UtilityDefinition::hashes(),
                UtilityDefinition::random_string(),
                UtilityDefinition::color(),
                UtilityDefinition::jwt(),
                UtilityDefinition::identifiers(),
                UtilityDefinition::regex(),
                UtilityDefinition::timestamps(),
                UtilityDefinition::sample_data(),
                UtilityDefinition::yaml_json(),
            ],
        }
    }

    pub fn definitions(&self) -> &[UtilityDefinition] {
        &self.definitions
    }

    pub fn definition(&self, id: UtilityId) -> Option<UtilityDefinition> {
        self.definitions
            .iter()
            .copied()
            .find(|definition| definition.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<UtilityId> {
        let terms: Vec<_> = query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        self.definitions
            .iter()
            .filter(|definition| {
                let haystack = format!(
                    "{} {} {} {}",
                    definition.name,
                    definition.summary,
                    definition.category,
                    definition.aliases.join(" ")
                )
                .to_ascii_lowercase();
                terms.iter().all(|term| haystack.contains(term))
            })
            .map(|definition| definition.id)
            .collect()
    }

    pub fn open(&self, id: UtilityId, selected: &mut UtilityId) -> bool {
        if self.definition(id).is_none() {
            return false;
        }
        *selected = id;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concrete_workspaces_are_discoverable() {
        let registry = UtilityRegistry::initial();

        assert_eq!(
            registry.definitions(),
            &[
                UtilityDefinition::json(),
                UtilityDefinition::text_diff(),
                UtilityDefinition::base64(),
                UtilityDefinition::url_encoding(),
                UtilityDefinition::case_conversion(),
                UtilityDefinition::whitespace(),
                UtilityDefinition::hashes(),
                UtilityDefinition::random_string(),
                UtilityDefinition::color(),
                UtilityDefinition::jwt(),
                UtilityDefinition::identifiers(),
                UtilityDefinition::regex(),
                UtilityDefinition::timestamps(),
                UtilityDefinition::sample_data(),
                UtilityDefinition::yaml_json(),
            ]
        );
    }

    #[test]
    fn search_matches_name_summary_and_aliases_using_all_terms() {
        let registry = UtilityRegistry::initial();

        assert_eq!(
            registry.search("format json"),
            vec![UtilityId::Json, UtilityId::YamlJson]
        );
        assert_eq!(registry.search("validate"), vec![UtilityId::Json]);
        assert_eq!(registry.search("compare text"), vec![UtilityId::TextDiff]);
        assert_eq!(registry.search("base64url"), vec![UtilityId::Base64]);
        assert!(registry.search("random json").is_empty());
    }

    #[test]
    fn opening_an_unknown_utility_is_rejected_without_changing_selection() {
        let registry = UtilityRegistry::initial();
        let mut selection = UtilityId::Json;

        assert!(registry.open(UtilityId::TextDiff, &mut selection));
        assert_eq!(selection, UtilityId::TextDiff);
    }
}
