//! Debounce-ready evaluation session for the JSON Utility.
//!
//! The session owns the current request, the last settled evaluation and the
//! revision bookkeeping that rejects obsolete asynchronous results. It is
//! GPUI-independent, so the stale-result policy is tested directly.

use crate::json::{evaluate, JsonEvaluation, JsonRequest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// The request equals the current one; nothing changed.
    Unchanged,
    /// A new revision was scheduled; resolve it with this revision id.
    Scheduled(u64),
}

#[derive(Clone, Debug)]
pub struct JsonSession {
    revision: u64,
    evaluation_epoch: u64,
    request: Option<JsonRequest>,
    evaluation: JsonEvaluation,
}

impl Default for JsonSession {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonSession {
    pub fn new() -> Self {
        Self {
            revision: 0,
            evaluation_epoch: 0,
            request: None,
            evaluation: JsonEvaluation::Empty,
        }
    }

    /// The current request revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Increments whenever the visible evaluation changes, including when a
    /// scheduled revision settles.
    pub fn evaluation_epoch(&self) -> u64 {
        self.evaluation_epoch
    }

    pub fn request(&self) -> Option<&JsonRequest> {
        self.request.as_ref()
    }

    pub fn evaluation(&self) -> &JsonEvaluation {
        &self.evaluation
    }

    /// Submits a request. An unchanged request is a no-op; a new request
    /// immediately clears the evaluation so no stale result stays visible.
    pub fn submit(&mut self, request: JsonRequest) -> SubmitOutcome {
        if self.request.as_ref() == Some(&request) {
            return SubmitOutcome::Unchanged;
        }
        self.revision += 1;
        self.request = Some(request);
        self.evaluation = JsonEvaluation::Empty;
        self.evaluation_epoch += 1;
        SubmitOutcome::Scheduled(self.revision)
    }

    /// Settles the evaluation for `revision`. Returns `None` when the revision
    /// is obsolete, so an out-of-order completion can never publish a result.
    pub fn resolve(&mut self, revision: u64) -> Option<&JsonEvaluation> {
        if revision != self.revision {
            return None;
        }
        let request = self.request.as_ref()?;
        self.evaluation = evaluate(request);
        self.evaluation_epoch += 1;
        Some(&self.evaluation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{Indentation, JsonMode, Severity};

    fn request(input: &str) -> JsonRequest {
        JsonRequest {
            input: input.to_string(),
            mode: JsonMode::Format,
            indentation: Indentation::TwoSpaces,
            sort_keys: false,
            query: String::new(),
        }
    }

    #[test]
    fn unchanged_request_is_not_rescheduled() {
        let mut session = JsonSession::new();
        assert!(matches!(
            session.submit(request(r#"{"a":1}"#)),
            SubmitOutcome::Scheduled(1)
        ));
        assert_eq!(
            session.submit(request(r#"{"a":1}"#)),
            SubmitOutcome::Unchanged
        );
        assert_eq!(session.revision(), 1);
    }

    #[test]
    fn submitting_clears_the_previous_evaluation() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(first) = session.submit(request(r#"{"a":1}"#)) else {
            panic!("expected a scheduled revision");
        };
        assert!(session.resolve(first).is_some());
        assert!(session.evaluation().output().is_some());

        session.submit(request(r#"{"b":2}"#));
        assert_eq!(session.evaluation(), &JsonEvaluation::Empty);
    }

    #[test]
    fn obsolete_revision_is_rejected() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(first) = session.submit(request(r#"{"a":1}"#)) else {
            panic!("expected a scheduled revision");
        };
        let SubmitOutcome::Scheduled(second) = session.submit(request(r#"{"b":2}"#)) else {
            panic!("expected a scheduled revision");
        };
        assert!(
            session.resolve(first).is_none(),
            "stale revision must not settle"
        );
        let settled = session.resolve(second).expect("current revision settles");
        assert_eq!(settled.output(), Some("{\n  \"b\": 2\n}"));
    }

    #[test]
    fn invalid_current_input_settles_as_diagnostics() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(revision) = session.submit(request("{")) else {
            panic!("expected a scheduled revision");
        };
        let settled = session.resolve(revision).expect("current revision settles");
        assert!(settled
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error));
        assert!(settled.output().is_none());
    }

    #[test]
    fn epoch_advances_on_submit_and_on_settle() {
        let mut session = JsonSession::new();
        let before = session.evaluation_epoch();
        let SubmitOutcome::Scheduled(revision) = session.submit(request("1")) else {
            panic!("expected a scheduled revision");
        };
        let after_submit = session.evaluation_epoch();
        assert!(after_submit > before);
        session.resolve(revision).unwrap();
        assert!(session.evaluation_epoch() > after_submit);
    }
}
