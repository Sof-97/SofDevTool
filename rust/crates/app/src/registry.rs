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
            construct: Some(crate::utilities::base64::construct),
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
            ]
        );
    }

    #[test]
    fn search_matches_name_summary_and_aliases_using_all_terms() {
        let registry = UtilityRegistry::initial();

        assert_eq!(registry.search("format json"), vec![UtilityId::Json]);
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
