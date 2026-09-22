# Ticket 05 implementation evidence

Status: implemented candidate, awaiting coordinator integration and final independent review. Native interaction is not claimed here.

## Changes

- `rust/crates/app/src/utilities/regex.rs`: one dedicated engine thread per Regex workspace, at most one running evaluation and one replaceable pending request. Debounce occurs in that worker. Its completed-result slot holds at most one result. The UI polls only while current work or an unread completion exists, and stops after settlement, clear or restore. Each edit, clear and restore invalidates the worker's revision. Session publication and History recording occur only after the current revision passes the final UI-side gate.
- `rust/crates/core/src/utilities/regex.rs`: cancellation checks between validation, compilation, engine iteration, capture construction and replacement stages. An individual regex engine call is not interrupted. Cancelled work returns no partial evaluation; all existing resource refusals and dialect/replacement guidance remain. The visible engine label and module comment no longer claim the entire workflow is linear time.
- `rust/crates/core/src/session.rs`: `publish(revision, evaluation)` accepts an externally completed result only for the current request. The existing synchronous `resolve` path still uses the same gate. Snapshot consumption remains one-shot, and clear/restore advance revisions.

## Verification (offline, isolated Cargo target)

- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo test -p sofdevtool-core utilities::regex --lib`: 20 passed. Includes an actual 8,192-match named-capture/replacement request and cancellation without partial publication. Existing pattern/program/text/template/match/capture/output limit tests, syntax, flags and replacement tests passed.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo test -p sofdevtool-app utilities::regex --lib`: 6 passed. Controlled completion order proves a running request plus 40 rapidly replaced pending requests yield only the final result and one snapshot; clear and restore reject running/pending results and snapshots. The worker state reports no current work after invalidation, allowing the UI poller to stop. An unread completion keeps the poller active after engine work ends; the activity check reads running state before the completion slot to avoid missing a result published between observations.
- `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target CARGO_NET_OFFLINE=true cargo test -p sofdevtool-core session::tests --lib`: 9 passed, including externally completed stale/current publication and one-shot snapshot behavior.
- `rustfmt --edition 2021 --check` on the three owned Rust files: passed.

No native launch, UI responsiveness observation, full workspace gate or macOS 14/15 runtime check was performed by this worker; those remain coordinator/final acceptance work. The tests assert scheduling order and actual engine execution, not a hard deadline or interruption inside an engine call.

## Earlier failed attempt

The prior pi 0.83.0 / opencode-go Kimi K3 attempt, dispatched from `5748a16`, stopped during source inspection with HTTP 429 `GoUsageLimitError`. It changed no product files and supplied no test evidence. Its local session remains in ignored `.artifacts/pi-rust-only/ticket-05/session.jsonl`. The owner's 2026-09-23 parallel Sol/Luna authorization superseded the pi quota blocker; the implementation and evidence above come from this later worker attempt.
