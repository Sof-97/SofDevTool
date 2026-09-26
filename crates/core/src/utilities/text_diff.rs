//! The Text Diff Utility's GPUI-independent request/snapshot contract.
//!
//! The diff itself is rendered by the application-owned web renderer; the core
//! only owns the exact comparison inputs and the versioned History payload so a
//! retained comparison restores byte-for-byte without rerunning the renderer's
//! asynchronous readiness handshake.

use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::utility::Utility;

/// Immutable catalog identity of the Text Diff Utility.
pub const TEXT_DIFF_UTILITY_ID: &str = "text-diff";

/// Schema version of [`TextDiffSnapshot`].
pub const TEXT_DIFF_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDiffMode {
    Split,
    Unified,
}

impl TextDiffMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDiffRequest {
    pub old: String,
    pub new: String,
    pub mode: TextDiffMode,
}

impl TextDiffRequest {
    pub fn new(old: impl Into<String>, new: impl Into<String>, mode: TextDiffMode) -> Self {
        Self {
            old: old.into(),
            new: new.into(),
            mode,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextDiffEvaluation {
    /// Both sides are empty: nothing has been compared.
    Empty,
    Valid {
        request: TextDiffRequest,
    },
    Invalid {
        diagnostics: Vec<Diagnostic>,
    },
}

impl TextDiffEvaluation {
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            TextDiffEvaluation::Invalid { diagnostics } => diagnostics,
            _ => &[],
        }
    }

    pub fn is_valid_operation(&self) -> bool {
        matches!(self, TextDiffEvaluation::Valid { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDiffSnapshot {
    pub old: String,
    pub new: String,
    pub mode: TextDiffMode,
}

impl TextDiffSnapshot {
    pub fn restore(&self) -> (&str, &str, TextDiffMode) {
        (&self.old, &self.new, self.mode)
    }
}

/// The Text Diff Utility's identity for the shared [`Utility`] trait.
pub struct TextDiff;

impl Utility for TextDiff {
    type Request = TextDiffRequest;
    type Evaluation = TextDiffEvaluation;
    type Snapshot = TextDiffSnapshot;

    const ID: &'static str = TEXT_DIFF_UTILITY_ID;
    const SNAPSHOT_VERSION: u32 = TEXT_DIFF_SNAPSHOT_SCHEMA_VERSION;

    fn neutral() -> TextDiffEvaluation {
        TextDiffEvaluation::Empty
    }

    fn evaluate(request: &TextDiffRequest) -> TextDiffEvaluation {
        if request.old.is_empty() && request.new.is_empty() {
            return TextDiffEvaluation::Empty;
        }
        TextDiffEvaluation::Valid {
            request: request.clone(),
        }
    }

    fn is_neutral(evaluation: &TextDiffEvaluation) -> bool {
        matches!(evaluation, TextDiffEvaluation::Empty)
    }

    fn snapshot(
        request: &TextDiffRequest,
        evaluation: &TextDiffEvaluation,
    ) -> Option<TextDiffSnapshot> {
        evaluation.is_valid_operation().then(|| TextDiffSnapshot {
            old: request.old.clone(),
            new: request.new.clone(),
            mode: request.mode,
        })
    }

    fn restore(snapshot: &TextDiffSnapshot) -> (TextDiffRequest, TextDiffEvaluation) {
        let request =
            TextDiffRequest::new(snapshot.old.clone(), snapshot.new.clone(), snapshot.mode);
        (request.clone(), TextDiffEvaluation::Valid { request })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_comparison_is_neutral() {
        let request = TextDiffRequest::new("", "", TextDiffMode::Split);
        assert_eq!(
            <TextDiff as Utility>::evaluate(&request),
            TextDiffEvaluation::Empty
        );
        assert!(<TextDiff as Utility>::snapshot(&request, &TextDiffEvaluation::Empty).is_none());
    }

    #[test]
    fn one_sided_comparison_is_valid() {
        let request = TextDiffRequest::new("old", "", TextDiffMode::Unified);
        let evaluation = <TextDiff as Utility>::evaluate(&request);
        assert!(evaluation.is_valid_operation());
        let snapshot = <TextDiff as Utility>::snapshot(&request, &evaluation).expect("snapshot");
        assert_eq!(snapshot.restore(), ("old", "", TextDiffMode::Unified));
    }

    #[test]
    fn snapshot_round_trips_without_reevaluation() {
        let request =
            TextDiffRequest::new("let café = 1\n", "let café = 2\n👨‍👩‍👧‍👦\n", TextDiffMode::Split);
        let evaluation = <TextDiff as Utility>::evaluate(&request);
        let snapshot = <TextDiff as Utility>::snapshot(&request, &evaluation).expect("snapshot");
        let (restored_request, restored_evaluation) = <TextDiff as Utility>::restore(&snapshot);
        assert_eq!(restored_request, request);
        assert!(restored_evaluation.is_valid_operation());
        // Restoring preserves the exact bytes, including the complex emoji.
        assert_eq!(restored_request.new, "let café = 2\n👨‍👩‍👧‍👦\n");
    }
}
