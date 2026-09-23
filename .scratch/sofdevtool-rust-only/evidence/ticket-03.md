# Ticket 03 implementation evidence

Status: eight-workspace source candidate frozen; shared full app gate awaits the coordinator's stable window after concurrent ticket 07 edits. No native launch or VCS write was performed by this worker.

## Changed behavior

- YAML/JSON, URL Encoding, Hashes, Timestamps, JWT Decoder, Case Conversion, Whitespace Conversion and Color Conversion now retain a `HistoryViewState` and a live `HistorySubscription`. A Settings Clear Utility, Clear All, or Retry immediately reloads actual storage in open workspaces, including open but hidden views. Removed entries lose their selection and pending restore; active controls, input, evaluation and result are not changed by History reconciliation. Color's ticket 06 picker/event behavior was left intact.
- Each workspace applies recording outcomes through the shared coordinator and notifies Settings of status changes. History warnings now describe a Utility's History error without assuming every error is a pause. JWT's local recording gate and default-off policy remain unchanged.
- Selection goes through `HistoryViewState::select`. Both selected restore and confirmation check the exact persisted entry immediately before applying a snapshot, so a late action cannot resurrect an entry deleted in Settings. Each Utility still owns concrete snapshot decode, preview and restore behavior.

## Verification

Commands run offline from `rust/` with `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and `CARGO_NET_OFFLINE=true`.

| Command | Result |
| --- | --- |
| `cargo check -p sofdevtool-app --lib` | Passed for all eight migrated workspace sources, including Color after ticket 06 handoff. |
| `cargo test -p sofdevtool-app --lib utilities::yaml_json::history_tests` | 2 passed: the actual-workspace subscription/recovery regressions described below. |
| `cargo clippy -p sofdevtool-app --lib -- -D warnings` | Passed after ticket 06's sofui changes stabilized. |
| `rustfmt --edition 2021 --check` on all eight owned sources | Passed. |
| `cargo test -p sofdevtool-app --lib` | Full attempt after Color migration was blocked at compilation by half-written ticket 07 `identifiers.rs` GPUI tests; the coordinator is waiting for that owner to freeze before a shared gate. |
| `cargo check -p sofdevtool-app --all-targets` | Earlier attempt was temporarily blocked by concurrently edited Color test API errors; that owner subsequently stabilized Color. Final rerun pending the shared gate window. |
| `cargo clippy -p sofdevtool-app --all-targets -- -D warnings` | Initial run found two YAML test warnings, now fixed, and an active ticket 07 test warning in `sample_data.rs`; final rerun pending that owner's fix and the shared gate window. |

An actual YAML/JSON workspace GPUI test uses an open visible workspace and an open unmounted workspace with one synthetic temporary History root. It passes selected and pending restore invalidation, immediate Clear Utility, partial Clear All when an unknown History path cannot be deleted, preservation of both sessions' input/result, a late confirm/selection, and direct snapshot restore without another record. A second passing actual-workspace test covers deterministic failed write, a further valid evaluation while paused without a History write, successful retry without a duplicate entry, and corrupt file isolation without changing current content.

Core Utility snapshot round-trip tests remain the Utility-owned restore contract checks. This worker has not claimed native pointer, keyboard, visual or macOS 14/15 runtime evidence.
