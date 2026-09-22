# Rust-only completion: pi trial

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
