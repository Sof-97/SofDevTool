# Rust-only completion: pi trial

## Current execution state

The active workflow is parallel GPT-6 Sol/Luna implementation followed by a
comprehensive GPT-6 Astra review after all implementations. The owner approved
preview `b7e28f0`; the Rust workspace is at the root and Swift retirement is
integrated. See the [execution contract](parallel-execution.md) and the latest
entries below. The pi trial and its stopping-point notes are dated history.


The owner authorized dedicated pi prompts with Kimi K3 after approving and committing the specification/tickets. This execution instruction supersedes the earlier documentation-only stopping point for dispatched work. The native-preview approval gate remains unchanged.

## Trial protocol

- Worker: pi 0.83.0, provider opencode-go, model kimi-k3, thinking high; a dedicated session and prompt per ticket.
- Initial sequence: 08, then 05, then 01. Review each candidate before dispatching the next. One implementation writer in the checkout.
- Coordinator owns ticket status, independent acceptance and GitButler commits. Workers leave uncommitted candidates and evidence, never self-resolve tickets.
- Provider credentials are supplied only to the worker process using the existing local OpenCode Go authentication. Credentials are never copied into prompts, tracked files or logs.
- Session and transport logs live in ignored build artifacts. Committed evidence contains only task outcomes and synthetic test results.

## Dispatch 08

- Ticket: [Random String preferences](issues/08-random-string-preferences.md).
- Prompt: [worker assignment](prompts/08-random-string-preferences.md).
- Checkout: current SofDevTool checkout, branch feat/rust-gpui-migration.
- Product base: 280ce18 (committed specification and tickets).
- State: completed, independently accepted and integrated in local commit `5748a16`. After the initial automatic review rejection, the owner explicitly approved sending task prompts, the specification and necessary private SofDevTool source context to OpenCode Go / Kimi K3 for these tickets. Credentials and personal application data remain excluded.
- Session: ignored local artifacts under `.artifacts/pi-rust-only/ticket-08`; dedicated session plus redacted transport log. Provider/model are fixed explicitly; no fallback is allowed.
- Setup evidence: installed pi 0.83.0 supports provider opencode-go and lists kimi-k3 with reasoning support. Authenticated Kimi K3 execution completed with exit 0 in 1,707.4 seconds. No subscription-limit or price claim is made.
- Automatic review reason: authorization to launch the model did not explicitly cover disclosure of this private repository to that destination. No workaround or alternate launch was attempted.

- Acceptance: [ticket 08 evidence](evidence/ticket-08.md). Standard gate: 270 tests plus formatting/Clippy/Debug build; independent native relaunch, malformed-load and save-failure checks passed. No coordinator product-code edits were needed.

## Dispatch 05

- Ticket: [Responsive Regex](issues/05-responsive-regex.md). No blockers.
- Prompt: [worker assignment](prompts/05-responsive-regex.md).
- Base: `5748a16` on `feat/rust-gpui-migration`; ticket 08 accepted and committed.
- State: worker stopped during source inspection; OpenCode Go usage limit reached.
- Session: `.artifacts/pi-rust-only/ticket-05`; same explicit disclosure
  authorization, credentials confined to process environment.
- Provider response: HTTP 429 `GoUsageLimitError` / `Go usage limit exceeded`.
  The pi process exited 0 after 164.3 seconds, but its final model message is an
  error; this is not a completed implementation. No product changes were made.
- Ticket returned to `ready-for-agent` with the attempt recorded in
  [evidence](evidence/ticket-05.md). Its session is preserved for resumption.
- Ticket 01 was not dispatched. No fallback model/provider or repeated quota
  attempts were used. The approved trial is incomplete pending provider capacity.
- The local driver now detects model errors in message events in addition to
  process exit codes. No reset time or subscription price is inferred.

## Local delivery at this stopping point

- `5f713f8`: all 22 dedicated prompts and the execution boundary.
- `5748a16`: accepted ticket 08 implementation and independent native evidence.
- The accompanying tracking commit records the provider blocker.
- No remote push, pull request or merge. No real application data was used.
- Only ticket 08 is resolved in this initiative; 05 and 01 still require the
  remaining trial implementation/review, and the rest remain undispatched.


## Parallel completion authorization — 2026-09-23

The owner superseded the pi trial with implementation of all remaining tickets
using parallel GPT-6 Sol/Luna subagents, followed by one comprehensive GPT-6
Astra review after all implementations. See [current execution contract](parallel-execution.md).
The native preview gate, local-only delivery and privacy boundaries remain.

Initial assignments from `54be475`:
- 01: GPT-6 Sol, sofui owned interfaces and JSON/gallery first consumers.
- 05: GPT-6 Sol, bounded background Regex execution and revision safety.
- 07: queued for GPT-6 Luna, generator restore protection (third worker dispatch awaiting an available agent slot).

All workers have disjoint file ownership, leave candidates uncommitted and do
not modify ticket statuses. The coordinator integrates and runs shared gates.

### Integration wave 1 and wave 2

- 01: Sol candidate complete; 7 sofui public/grapheme tests and 57 app tests,
  app/gallery compile and owned formatting passed. Native acceptance and final
  Astra review pending. Package rename retains the temporary app dependency alias.
- 05: Sol candidate complete; 20 core Regex, 9 shared session and 6 app Regex
  tests passed. Coordinator requested finite idle polling and a completion-race
  correction; both are included. Native acceptance/final Astra review pending.
- 02: assigned to the Regex worker (GPT-6 Sol), History coordination and first
  JSON/Base64/Settings consumers. JSON ownership handed over after01 completion.
- 10: next UI-lane assignment after01 integration, application-wide themes;
  Settings/History/JSON remain owned by02.
- The service refused a third agent thread; two Sol implementation lanes continue.
  07 remains queued; no third worker or early Astra review was started.

- Wave1 commits: `dca5671` execution protocol, `a7b32aa` ticket01, `860e6a9` ticket05.
- 10 dispatched to sofui_foundation (GPT-6 Sol): ui themes/editor integration,
  gallery/tests, app Workbench/Launcher/main. 02 retains Settings/JSON/History/Base64.

### Continued integration and dispatch

- Both Sol workers resumed after a usage interruption; no reset credit was used.
- 10 integrated after its sofui tests/Clippy and app compilation passed.
- 02 supplied 68 passing app tests and an all-target check. Coordinator default
  gate found a Clippy diagnostic in retry; its owner is correcting that plus
  the Clear All unknown-file basename edge before handoff.
- 06 dispatched to the UI Sol lane: synchronized color workspace and reusable
  numeric/channel interaction, disjoint from History files.
- Native frozen `860e6a9` JSON and Regex acceptance is recorded in01/05 evidence.
  The gallery Computer Use selection stalled for an extended period; no gallery
  interaction success is claimed from that attempt.

- 10 commit: `8d59068`.
- 02 integration corrections passed: 69 app tests and all-target Clippy.
- A third slot became available after the previous completed reviewer thread
  left the live inventory. GPT-6 Luna now implements07 on Identifier Generator
  and Sample Data only; History migration04 must wait for its handoff.

- 02 commit: `2fdfeb1`; frozen combined default gate passed290 tests and builds.
- Native02 pause/retry and10 theme appearance observations recorded.
- 03 dispatch: eight conversion/inspection workspaces, with Color held until06
  handoff. 07 received integration feedback to test real workspace confirmation
  callbacks rather than only a pure decision helper.
- Shared Cargo compilation is temporarily serialized while06 finalizes its
  component export and tests; root snapshot builds are complete.

- 06 integrated `bdf1a3a`; Color ownership explicitly handed to03.
- 09 dispatched to UI Sol: generic list/confirmation and component-owned hold
  timing, JSON/Settings first consumers, public gallery/tests.
- 07 Luna is strengthening tests at the actual workspace restore callbacks
  before integration; its evidence was moved to the canonical initiative path.

- 03 integrated `06846a1`; 07 frozen candidate passed the coordinator's combined
  default gate: 304 tests, formatting, Clippy and Debug app/gallery builds.
- Coordinator snapshot builds now use an independent Cargo target to avoid
  stale artifacts when workers and frozen source trees compile concurrently.
- 04 dispatched to the History Sol lane: Random String, Regex and Text Diff
  first; Identifier/Sample Data ownership follows the07 integration handoff.

- 07 integrated `068460c`; Identifier/Sample Data handed to04.
- 17 dispatched to Luna: owned editable Text Diff assets, exact dependency
  rebuild, actual notices and renderer-boundary validation. The Swift wrapper
  remains until the replacement pipeline is verified and21 retires it.

- 04 integrated `c1007e6`; native06/07 evidence committed `e1aa178`.
- 09 frozen combined gate passed310 tests and all default checks; Settings
  test roots now use an atomic sequence to avoid parallel directory collision.
- 11 dispatched to UI Sol: new native preview example with real public sofui
  controls and synthetic data. Owner approval of the runnable candidate still
  explicitly gates12 and the full visual rollout.

- 09 commit `aa48cbc`; combined gate evidence/inventory `509653b`.
- 17 integrated `8da4421`; independent coordinator asset reproducibility passed.
- 11 native candidate `b7e28f0` presented with Graphite/Frappé screenshots and
  verified representative interactions. Explicit owner approval requested;
  full visual rollout remains pending that answer.
- 20 dispatched to the History Sol lane after all other source writers froze.
  Cargo/crates/toolchain/config now reside at the repository root; packaging
  and profile metadata are in progress. Root uses a frozen pre-move snapshot
  for independent gates and native checks.

- Owner approved native candidate `b7e28f0` on 2026-09-23. Ticket 12 dispatched to the UI Sol lane; this replaces the pending approval state above.
- Ticket 20 source is frozen after root full gate and temporary installation checks. Nine independent retained core vectors are frozen separately for Swift retirement.

- Root workspace integrated `24f4fcc`, retained independent vectors `0a26d00`, shell redesign `ae00b69`, owner-approved Swift retirement `55702fd`, generic segmented selection `5a053df`. Ticket 13 integrated after94app tests;14/15/18 are in parallel implementation.
- Native shell checkpoint recorded in ticket12 evidence; Cmd-W/fullscreen keyboard correction assigned separately. Global shortcut registration was observed, synthesized invocation remains unresolved.
- Owner explicitly approved the exact scoped retired-file deletion after automatic approval rejections; deletion completed while preserving six excluded personal/metadata files. Native temporary bundle launch also received explicit owner approval, including the isolated terminal launch.

- Wave8 commits: `110a94d` generators15, `1209cdd` conversions14, `b82cbee` Text Diff18, `e3cb13b` native window menu correction. Shared gate compiled all app tests; 101/102 passed initially, and the remaining Hashes interaction passed after correcting test event sequencing. A new full combined gate remains required.
- Ticket16 is dispatched to the Regex Sol lane. Ticket19 independent-consumer preparation runs in the UI Sol lane; compatibility removal waits for the final consumers in16. Root owns native and final installed Release acceptance.
