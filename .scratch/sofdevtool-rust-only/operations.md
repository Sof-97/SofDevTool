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
- State: completed, independently accepted and integrated in the accompanying local ticket commit. After the initial automatic review rejection, the owner explicitly approved sending task prompts, the specification and necessary private SofDevTool source context to OpenCode Go / Kimi K3 for these tickets. Credentials and personal application data remain excluded.
- Session: ignored local artifacts under `.artifacts/pi-rust-only/ticket-08`; dedicated session plus redacted transport log. Provider/model are fixed explicitly; no fallback is allowed.
- Setup evidence: installed pi 0.83.0 supports provider opencode-go and lists kimi-k3 with reasoning support. Authenticated Kimi K3 execution completed with exit 0 in 1,707.4 seconds. No subscription-limit or price claim is made.
- Automatic review reason: authorization to launch the model did not explicitly cover disclosure of this private repository to that destination. No workaround or alternate launch was attempted.

- Acceptance: [ticket 08 evidence](evidence/ticket-08.md). Standard gate: 270 tests plus formatting/Clippy/Debug build; independent native relaunch, malformed-load and save-failure checks passed. No coordinator product-code edits were needed.
