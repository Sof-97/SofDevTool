# Ticket 08 evidence: Remember Random String controls independently of History

Status: accepted by the coordinator after independent source review and isolated
native verification; integrated in the accompanying local ticket commit. Host: macOS 26.2 (25C56) arm64, Rust 1.98.1 (pinned toolchain),
`CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target`, `CARGO_NET_OFFLINE=true`.

## Behavior changed

Before (`HEAD` workspace constructor): `RandomStringWorkspace::new` hardcoded
`length: 20, count: 1, uppercase: true, …` and no Random String preference store
existed (`RandomStringControls` absent from `preferences.rs`). Every workspace
construction reset the controls to defaults; the configuration survived only as
part of recorded History snapshots, so with History disabled or never recorded,
relaunch always lost the controls.

After:

- The workspace opens a `RandomStringControlsSession` over a new
  `RandomStringControlsPreferences` store (`random-string-controls.v1.json`,
  versioned record, atomic same-directory replace) in the existing profile
  data-root seam (`identity::application_support_root()`, i.e. the
  `SOFDEVTOOL_RUST_SUPPORT_ROOT` override seam ticket 20 will use).
- Construction applies the stored controls (or the documented defaults:
  length 20, count 1, upper/lowercase, digits, safe symbols, ambiguous
  exclusion on) and sets the custom-alphabet text through silent assignment
  (`TextField::set_text`, which emits no `Change` event per the pinned
  gpui-base 0.6.1 source). Loading never generates output, never records
  History, and never writes the preference file.
- Every actual control change saves immediately and independently of Generate
  and History recording: class toggles, length/count steppers (only when the
  clamped value actually changes), custom-alphabet edits (user `Change`
  events, including Paste custom via `replace_all`), and History restore
  (restored controls become the latest saved configuration; restoring is not
  a Utility Operation and records nothing). Generate and Clear results do not
  re-save unchanged controls; unchanged values write nothing.
- Ordinary preferences carry controls only. The persisted
  `RandomStringControls` schema has exactly the eight control fields; the
  generation nonce, generated values, entropy and entry bookkeeping are
  structurally excluded (asserted on the serialized JSON).
- Loading distinguishes states honestly: missing file = neutral first run
  (defaults, no diagnostic); malformed JSON, unsupported schema version, or
  out-of-range length/count (outside 1–4,096 / 1–100, core-validated via
  `RandomStringRequest::has_supported_ranges`) = defaults plus a visible
  non-modal warning, with the unusable file left untouched on disk.
- A failed save (deterministic read-only-directory injection) keeps the
  visible configuration, shows "could not be saved … applies to this launch
  only", writes nothing, and is cleared by the next successful save. The
  visible Utility result is never affected.
- Debug/Release identity (`identity.rs`) is untouched per ticket boundaries;
  profile scoping is exercised through two isolated data roots.

## Regression and test commands

All commands run from `rust/` (pre-ticket-20 location) with the coordinator's
environment. Baseline before changes: 257 tests green (43 app + 186 core +
23 JSON contract + 5 ui).

- `cargo test --workspace --offline` — PASS. 270 tests (55 app, 187 core,
  23 JSON contract, 5 ui), 0 failed. Baseline was 257; +13 new.
- `scripts/format` (cargo fmt --all) — applied; only the three touched files
  differ from HEAD (verified by object-hash comparison across all
  `rust/crates`/`rust/scripts` sources, since raw git status/diff shows the
  known GitButler synthetic-index deletion/untracked pairs).
- `scripts/verify` — PASS: rustfmt check, clippy with warnings denied, all
  tests, Debug build of bins/examples. ("gate passed"; pre-existing
  future-incompat note for `block v0.1.6` unchanged.)

New tests (app `utilities::random_string::tests`, `preferences::tests`, core
`utilities::random_string::tests`):

- `default_controls_match_the_documented_product_defaults` — defaults equal
  the core `RandomStringRequest::default()` configuration.
- `ordinary_preferences_carry_controls_only_never_values_or_bookkeeping` —
  serialized controls contain exactly 8 fields; no nonce/values/entropy.
- `a_fresh_launch_opens_neutral_and_writes_nothing` — missing file ⇒ defaults,
  no diagnostic, no file written; a fresh generation session stays `Empty`
  with no snapshot.
- `saved_controls_survive_relaunch_with_history_disabled_and_record_nothing` —
  reconstruction under an isolated root with `global_enabled: false`; controls
  recovered; History empty and no History directory created.
- `every_supported_control_change_persists_immediately` — each knob (length,
  count, four class toggles, ambiguous exclusion, Unicode custom alphabet)
  changed, reopened, recovered.
- `unchanged_controls_write_nothing`.
- `a_failed_save_is_reported_and_the_next_successful_save_clears_the_status` —
  read-only root ⇒ status reports failure, configuration preserved, no file;
  next change retries and persists, status cleared.
- `malformed_controls_fall_back_to_defaults_with_a_diagnostic_and_stay_untouched`.
- `an_unsupported_schema_version_falls_back_to_defaults_with_a_diagnostic`.
- `out_of_range_stored_controls_are_rejected_entirely` — length 0/4097,
  count 0/101 rejected wholesale with a ranges diagnostic.
- `controls_are_scoped_to_their_profile_data_root` — two roots isolate state;
  the untouched root is never created by opening.
- `preferences::tests::random_string_controls_round_trip_and_default_safely` —
  store-level round trip, missing/malformed handling.
- core `supported_ranges_cover_only_the_documented_length_and_count_bounds`.

Old-behavior demonstration: the defect was structural — the pre-change
constructor (see `HEAD:rust/crates/app/src/utilities/random_string.rs`,
`length: 20, count: 1, …` literals) had no persistence seam, so no existing
test seam could express "relaunch recovers controls" to fail first. The new
session seam's reconstruction tests are the regression protection; each was
run against the implementation and passes.

## Worker evidence limits

- Automated evidence covers the GPUI-independent coordination seam
  (preferences store + controls session + core ranges) with temporary data
  roots. No GPUI window was constructed in tests; the workspace wiring
  (construction load, per-control save calls, warning banner) is covered by
  compilation and code review, not by interaction automation.
- A successful compile and green tests do not prove native behavior; no app
  launch was performed against any real profile. No personal Application
  Support data, credentials, or real History were read.
- The save-on-restore choice (History restore updates the saved controls as
  the latest configuration) is a deliberate interpretation of "latest
  controls"; flagged for reviewer confirmation.

## Worker handoff checks (completed below except ticket 20 profile identities)

- Launch the real app under an isolated `SOFDEVTOOL_RUST_SUPPORT_ROOT`, change
  controls without generating, relaunch, and observe recovery with History
  disabled; confirm the warning banner on a malformed controls file and on a
  forced save failure.
- Ticket 20's Debug/Release profile split should exercise the same store
  through both bundle identities' data roots.

## Coordinator acceptance — 2026-09-23

- Fixed review base: specification commit `280ce18`; candidate changes are the
  three Rust files listed in this report. No blocking Standards or Spec findings.
- Standards: strongly typed Utility controls remain app-owned; no UI-library
  domain logic, automatic Clipboard access or generated-result persistence was
  introduced. Dependencies and product identity remain unchanged.
- Spec: all eight controls persist independently of generation/History; loading
  is neutral, validates versions/ranges and retains malformed files. Keeping
  restored controls as the latest configuration is consistent with the contract.
- Reviewed the worker's standard verification transcript: formatting, Clippy,
  270 tests and Debug build passed. Independently built `rust/scripts/bundle-debug`
  with the same offline Cargo cache. No Release/full gate is claimed.
- Native Computer Use on macOS 26.2: launched the Debug bundle with a newly
  created temporary `SOFDEVTOOL_RUST_SUPPORT_ROOT`, and seeded History recording
  disabled. All UI interactions and files used synthetic data.
- Changed every class toggle and ambiguous exclusion to off, length to 21,
  count to 2, and custom alphabet to `KIMI08xyz` through the actual controls.
  Without Generate, the file contained exactly those eight controls. Quit and
  relaunched: all settings appeared restored; Generated Results showed 0 and
  History showed 0/25.
- Then generated two synthetic results. With the temporary directory read-only,
  Length + kept length 22 in the UI and both existing results, displayed the
  explicit save-failure banner, and left the previous length 21 on disk. After
  restoring directory permissions, Count + persisted length 22/count 3 and
  cleared the banner. History remained empty and no History directory existed.
- A third launch with a deliberately malformed preference fixture displayed
  the load warning and all defaults (length 20/count 1, enabled classes and
  exclusion, empty custom alphabet), with zero results. The malformed fixture
  remained unchanged. The test app was quit after verification.
- The current `History: on` workspace button controls panel visibility; recording
  was disabled by the isolated History policy, as confirmed by no History writes
  even after explicit generation.
- Native UI was directly observed in this task; screenshots were not added to
  source control. Debug/Release identity separation is still ticket 20's scope.
- Worker: pi 0.83.0 / opencode-go / kimi-k3, thinking high; exit 0 after
  1,707.4 seconds (28m27s), no coordinator product-code corrections.
