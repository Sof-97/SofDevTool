# Ticket 07 — generator restore confirmation

## Scope

Identifier Generator and Sample Data protect different nonempty workspace state
before applying a History snapshot. Restore equivalence includes both the
strongly typed configuration and the captured generated output.

## Implementation

- Identifier Generator treats a settled result, edited controls, namespace,
  name, or inspection input as meaningful state. It compares the current
  `WorkspaceSignature` and generated values with the snapshot before applying.
- Sample Data treats a settled result or any schema/row-count/format change as
  meaningful, including invalid edited schemas. It compares the current
  configuration and output with the captured snapshot.
- Exact-equivalent and neutral sessions apply without a warning. A different
  batch with matching controls asks for confirmation. Confirmation restores
  captured output through `Session::restore`, which invalidates obsolete
  revisions and consumes the restored revision so it cannot produce a new
  History snapshot.
- Added GPUI workspace tests that invoke `request_restore`, `cancel_restore`
  and `confirm_restore` for both Utilities. They check same-controls/different-
  output confirmation, preserved live state after cancellation, exact captured
  controls/output after confirmation, unchanged History retention, no History
  clock calls, and no new session snapshot. Sample Data also exercises an
  edited invalid field schema; empty and equivalent sessions apply directly.

## Verification

- Coordinator froze commit `06846a1` plus the two candidate source files in
  `/private/tmp/sofdevtool-wave3-5twn1s5j/rust` and ran `scripts/verify` with an
  independent offline Cargo target. The full gate passed: formatting, Clippy
  with warnings denied, 304 tests (82 app, 190 core, 23 JSON contracts, 9 sofui),
  and Debug app/gallery builds. Both actual-workspace restore tests passed.
  Log: `.artifacts/parallel-rust-only/wave3/verify.log`.
- Native runtime verification was not run; native launch is coordinator-owned.

## Remaining checks

The final Astra review remains outstanding. macOS 14/15 runtime remains unverified.

## Coordinator native acceptance

The frozen03+06+07 Debug bundle ran on macOS26.2 arm64 from `/private/tmp`
with a fresh isolated support root. Identifier Generator produced two UUID
batches with identical controls. Restoring the first asked for confirmation;
Cancel preserved the second UUID, verified through Copy All pasted into the
inspection field. Confirm restored the exact first UUID and cleared the
inspection input; Copy All verified its captured value. History remained at
two entries.

Sample Data generated two batches with identical fields/settings. Restoring
the first prompted; Cancel preserved the second output, and Confirm restored
the first captured fictional data with two History entries remaining.
Restoring the equivalent first batch again did not prompt. Clearing a field
name made the schema invalid; restore then prompted and Cancel preserved
that edited invalid schema. No real app data was accessed. The test app quit
successfully. Logs/profile metadata: `.artifacts/parallel-rust-only/wave3/`.
