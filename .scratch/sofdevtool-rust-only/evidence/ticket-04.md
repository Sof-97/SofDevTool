# Ticket 04 implementation evidence

Status: five-workspace implementation candidate complete; awaiting coordinator integration and final independent review. Ticket 03's eight-workspace History batch was already integrated. No native launch or VCS write was performed by this worker.

## Changed behavior

- Identifier Generator, Random String, Sample Data, Regex and Text Diff now keep a `HistoryViewState` and live `HistorySubscription`. Settings Clear Utility, Clear All and Retry reconcile actual storage in open workspaces, including hidden ones. Deleted rows lose selection and pending confirmation immediately; History reconciliation does not change current generated values, Regex evaluation or comparison inputs.
- Each settled recording outcome updates the shared view and notifies Settings of status changes. List selection uses `HistoryViewState::select`; both selected restore and confirmation verify the exact persisted entry immediately before applying the Utility-owned snapshot. Warnings no longer assume every History failure is a recording pause.
- Identifier and Sample Data retain ticket 07's full configuration-plus-output restore and confirmation behavior. Random String retains ticket 08's independent control preferences and deliberate generation nonce. Regex retains ticket 05's bounded background worker, revision rejection and idle poller. Text Diff keeps its renderer lifecycle; restore now marks the next renderer revision recorded **before** requesting the render, so a synchronous Ready callback cannot create a duplicate History entry, and a later readiness callback still sees that revision as recorded.
- Random String workspace construction accepts an injected controls session internally, allowing actual-workspace tests to use isolated preferences rather than reading the user's Application Support data. Production construction still loads the same saved controls.

## Verification

Commands ran offline from `rust/` with `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and `CARGO_NET_OFFLINE=true`. Tests use temporary History/preference roots and synthetic snapshot fixtures; the Random String test captures generated values instead of asserting a particular cryptographic random output.

| Command | Result |
| --- | --- |
| `cargo test -p sofdevtool-app --lib` | 85 passed, 0 failed on the current shared source. Includes new actual-workspace Random String and Regex History regressions; ticket 07's Identifier and Sample Data actual-workspace restore tests were extended to clear History, reject stale confirmation and preserve captured output. |
| `cargo clippy -p sofdevtool-app --all-targets -- -D warnings` | Passed. |
| `rustfmt --edition 2021 --check` on the five owned Rust sources | Passed. |

The Random String test generates a real batch, confirms that restore reproduces its exact captured values without a new History entry, and checks that Clear Utility removes selected/pending rows without changing a current batch. A deliberate later generation still records separately. The Regex test controls a running engine call and later winning revision: clearing History leaves the in-flight session revision intact, a deleted entry cannot be confirmed, the stale engine result is discarded, and only the winning settled request records. Restoring that result produces no second entry. Existing worker tests cover one running/one replaceable pending request and idle poller completion.

The Identifier and Sample Data tests verify actual workspace confirmation and exact output restoration, then clear History and attempt a late confirmation; captured current output survives and no deleted row remains selectable. Text Diff's no-duplicate restoration follows its revision guard and existing renderer protocol tests. Direct native Text Diff/WebView interaction, pointer/keyboard presentation, and macOS 14/15 runtime checks were not performed by this worker.

## Coordinator integrated gate

Ticket04 integrated in `c1007e6`. Frozen `e1aa178` plus09 passed the full
310-test default gate, including all five migrated consumers, formatting,
strict Clippy and Debug builds. Log:
`.artifacts/parallel-rust-only/wave5/verify.log`. An earlier04-only run had one
Settings temporary-root write failure;09's test-isolation fix and subsequent
full pass supersede it. This is automated evidence, not a native WebView claim.

Native Text Diff restore was also verified on frozen `b7e28f0`: confirming a
previous comparison restored exact Unicode old/new text, preserved its mode
and left four retained entries unchanged. Navigating away and back retained
that state. See [renderer evidence](ticket-17.md).
