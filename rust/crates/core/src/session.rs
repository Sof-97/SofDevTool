//! Revision-gated evaluation session shared by every Utility.
//!
//! The session owns the current request, the last settled evaluation and the
//! revision bookkeeping that rejects obsolete asynchronous results. It is
//! GPUI-independent, so the stale-result and one-shot-snapshot policy is tested
//! once for all Utilities.

use crate::utility::Utility;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// The request equals the current one; nothing changed.
    Unchanged,
    /// A new revision was scheduled; resolve it with this revision id.
    Scheduled(u64),
}

pub struct Session<U: Utility> {
    revision: u64,
    evaluation_epoch: u64,
    request: Option<U::Request>,
    evaluation: U::Evaluation,
    consumed_revision: Option<u64>,
}

impl<U: Utility> Default for Session<U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Utility> Session<U> {
    pub fn new() -> Self {
        Self {
            revision: 0,
            evaluation_epoch: 0,
            request: None,
            evaluation: U::neutral(),
            consumed_revision: None,
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

    pub fn request(&self) -> Option<&U::Request> {
        self.request.as_ref()
    }

    pub fn evaluation(&self) -> &U::Evaluation {
        &self.evaluation
    }

    /// Submits a request. An unchanged request is a no-op; a new request
    /// immediately clears the evaluation so no stale result stays visible.
    pub fn submit(&mut self, request: U::Request) -> SubmitOutcome {
        if self.request.as_ref() == Some(&request) {
            return SubmitOutcome::Unchanged;
        }
        self.revision += 1;
        self.request = Some(request);
        self.evaluation = U::neutral();
        self.consumed_revision = None;
        self.evaluation_epoch += 1;
        SubmitOutcome::Scheduled(self.revision)
    }

    /// Settles the evaluation for `revision`. Returns `None` when the revision
    /// is obsolete, so an out-of-order completion can never publish a result.
    pub fn resolve(&mut self, revision: u64) -> Option<&U::Evaluation> {
        if revision != self.revision {
            return None;
        }
        let request = self.request.as_ref()?;
        let evaluation = U::evaluate(request);
        self.publish(revision, evaluation)
    }

    /// Publishes a completed evaluation produced outside the UI thread. The
    /// caller still has to check the revision here, even if it cancelled old
    /// work earlier: an engine call may finish after a newer request arrives.
    pub fn publish(&mut self, revision: u64, evaluation: U::Evaluation) -> Option<&U::Evaluation> {
        if revision != self.revision || self.request.is_none() {
            return None;
        }
        self.evaluation = evaluation;
        self.evaluation_epoch += 1;
        Some(&self.evaluation)
    }

    /// Supplies exactly one snapshot for a current settled valid operation.
    /// Repeated calls for the same revision return `None`, so an operation is
    /// recorded at most once.
    pub fn take_snapshot(&mut self) -> Option<U::Snapshot> {
        if self.consumed_revision == Some(self.revision) {
            return None;
        }
        let snapshot = U::snapshot(self.request.as_ref()?, &self.evaluation)?;
        self.consumed_revision = Some(self.revision);
        Some(snapshot)
    }

    /// Restores captured state directly. It never reevaluates the request and
    /// invalidates every pending completion from the prior revision.
    pub fn restore(&mut self, snapshot: U::Snapshot) {
        let (request, evaluation) = U::restore(&snapshot);
        self.revision += 1;
        self.request = Some(request);
        self.evaluation = evaluation;
        self.consumed_revision = Some(self.revision);
        self.evaluation_epoch += 1;
    }

    /// Clears the request back to neutral. Any pending completion is obsolete.
    pub fn clear(&mut self) {
        self.revision += 1;
        self.request = None;
        self.evaluation = U::neutral();
        self.consumed_revision = None;
        self.evaluation_epoch += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{Json, JsonEvaluation, JsonMode, JsonRequest};

    type JsonSession = Session<Json>;

    fn request(input: &str) -> JsonRequest {
        JsonRequest::new(input, JsonMode::Format)
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
    fn externally_completed_evaluation_obeys_revision_and_snapshot_gate() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(first) = session.submit(request("1")) else {
            panic!("expected first revision");
        };
        let old = <Json as Utility>::evaluate(&request("1"));
        let SubmitOutcome::Scheduled(second) = session.submit(request("2")) else {
            panic!("expected second revision");
        };
        assert!(session.publish(first, old).is_none());
        assert_eq!(session.evaluation(), &JsonEvaluation::Empty);
        assert!(session.take_snapshot().is_none());

        let current = <Json as Utility>::evaluate(&request("2"));
        assert!(session.publish(second, current).is_some());
        assert_eq!(session.evaluation().output(), Some("2"));
        assert!(session.take_snapshot().is_some());
        assert!(session.take_snapshot().is_none());

        session.clear();
        assert!(session
            .publish(second, <Json as Utility>::evaluate(&request("2")))
            .is_none());
        assert_eq!(session.evaluation(), &JsonEvaluation::Empty);
    }

    #[test]
    fn invalid_current_input_settles_as_diagnostics() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(revision) = session.submit(request("{")) else {
            panic!("expected a scheduled revision");
        };
        let settled = session.resolve(revision).expect("current revision settles");
        assert!(!settled.diagnostics().is_empty());
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

    #[test]
    fn settled_operation_yields_one_exact_snapshot_without_reevaluation() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(revision) = session.submit(request(r#"{"b":2,"a":1}"#)) else {
            panic!("expected a scheduled revision");
        };
        session.resolve(revision);
        let snapshot = session.take_snapshot().expect("settled snapshot");
        assert_eq!(snapshot.restore().1, "{\n  \"b\": 2,\n  \"a\": 1\n}");
        assert!(
            session.take_snapshot().is_none(),
            "an operation records at most once"
        );
    }

    #[test]
    fn invalid_or_neutral_operations_never_snapshot() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(revision) = session.submit(request("{")) else {
            panic!("expected a scheduled revision");
        };
        session.resolve(revision);
        assert!(session.take_snapshot().is_none());
    }

    #[test]
    fn restore_rejects_old_completion_and_does_not_record_again() {
        let mut session = JsonSession::new();
        let SubmitOutcome::Scheduled(old) = session.submit(request("1")) else {
            panic!("expected a scheduled revision");
        };
        session.restore(crate::json::JsonSnapshot {
            request: request("ignored"),
            output: "hand-authored".into(),
        });
        assert!(session.resolve(old).is_none());
        assert_eq!(session.evaluation().output(), Some("hand-authored"));
        assert!(
            session.take_snapshot().is_none(),
            "restore never records a new snapshot"
        );
    }
}
