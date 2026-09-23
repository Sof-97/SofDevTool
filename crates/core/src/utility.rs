//! The strongly typed, GPUI-independent contract every Utility implements.
//!
//! A Utility owns its request, evaluation and snapshot types. The trait is
//! generic (monomorphised), so the application keeps strong concrete types and
//! only erases them when constructing heterogeneous workspaces in the Registry.
//! It is not a universal execute-input interface: there is no shared request
//! shape, and no dynamic dispatch over Utility inputs.

use serde::{de::DeserializeOwned, Serialize};

/// A Utility's typed contract with the shared evaluation session.
///
/// Implementors provide the domain transformation only. Revision bookkeeping,
/// stale-result rejection and one-shot snapshot consumption live in
/// [`crate::session::Session`].
pub trait Utility: 'static {
    /// The complete, strongly typed input for one evaluation.
    type Request: Clone + PartialEq + Serialize + DeserializeOwned + 'static;
    /// The typed outcome shown to the user.
    type Evaluation: Clone + PartialEq + 'static;
    /// The Utility-owned payload persisted in History.
    type Snapshot: Clone + Serialize + DeserializeOwned + 'static;

    /// Stable catalog/History identity. Used for the History filename.
    const ID: &'static str;

    /// Snapshot schema version. The Utility owns decoding and migration.
    const SNAPSHOT_VERSION: u32;

    /// The neutral evaluation for untouched or cleared input.
    fn neutral() -> Self::Evaluation;

    /// Evaluates a request. Must be deterministic and free of hidden state.
    fn evaluate(request: &Self::Request) -> Self::Evaluation;

    /// True when the evaluation represents no operation (empty/neutral input).
    fn is_neutral(evaluation: &Self::Evaluation) -> bool;

    /// The single recordable snapshot for a settled valid operation, or `None`
    /// when the evaluation is invalid, neutral or not a completed operation.
    fn snapshot(request: &Self::Request, evaluation: &Self::Evaluation) -> Option<Self::Snapshot>;

    /// Rebuilds request and evaluation from a stored snapshot without
    /// reevaluating input or reading a clock/random source.
    fn restore(snapshot: &Self::Snapshot) -> (Self::Request, Self::Evaluation);
}
