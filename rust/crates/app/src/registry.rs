//! The source-defined Utility catalog shared by the Workbench and Launcher.

/// The stable identity of a source-defined Utility.
///
/// This stays deliberately narrow while only JSON is integrated. `TextDiff`
/// is reserved for ticket 02's concrete workspace; it never appears in the
/// catalog until that workspace is available.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UtilityId {
    Json,
    TextDiff,
}

/// A request from a discovery surface to reveal one registered Utility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenUtility(pub UtilityId);

/// Immutable discovery metadata. Construction of the concrete workspace stays
/// in the Workbench so each Utility retains its strong concrete type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtilityDefinition {
    pub id: UtilityId,
    pub name: &'static str,
    pub summary: &'static str,
    pub category: &'static str,
    pub aliases: &'static [&'static str],
}

impl UtilityDefinition {
    pub const fn json() -> Self {
        Self {
            id: UtilityId::Json,
            name: "JSON",
            summary: "Format, minify, validate, and query JSON",
            category: "Format & Convert",
            aliases: &["format", "validate", "minify", "query"],
        }
    }

    pub const fn text_diff() -> Self {
        Self {
            id: UtilityId::TextDiff,
            name: "Text Diff",
            summary: "Compare text with split or unified output",
            category: "Text",
            aliases: &["diff", "compare", "patch"],
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
            definitions: vec![UtilityDefinition::json(), UtilityDefinition::text_diff()],
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
    fn concrete_json_and_text_diff_workspaces_are_discoverable() {
        let registry = UtilityRegistry::initial();

        assert_eq!(
            registry.definitions(),
            &[UtilityDefinition::json(), UtilityDefinition::text_diff()]
        );
    }

    #[test]
    fn search_matches_name_summary_and_aliases_using_all_terms() {
        let registry = UtilityRegistry::initial();

        assert_eq!(registry.search("format json"), vec![UtilityId::Json]);
        assert_eq!(registry.search("validate"), vec![UtilityId::Json]);
        assert_eq!(registry.search("compare text"), vec![UtilityId::TextDiff]);
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
