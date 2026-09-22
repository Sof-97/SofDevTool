# 05: Manage History retention, deletion and storage failures

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Control recording, clear retained entries deliberately and continue using Utilities when a History file is damaged or unwritable.

## Acceptance criteria

- [ ] Implement global and per-Utility recording preferences without rewriting one another; disabling recording retains existing entries.
- [ ] Clear Utility uses a one-second pointer hold; Clear All uses two seconds; early release, pointer exit and Escape cancel, while keyboard activation uses confirmation.
- [ ] Deletion leaves active sessions intact and can remove unknown Utility files only within Rust History; legacy Swift data remains untouched.
- [ ] Write failure keeps the current result and last valid file, pauses that Utility and shows a nonmodal warning; a later successful retry or relaunch can recover.
- [ ] Isolate malformed files and unsupported snapshots without silently overwriting them; test retention ordering, failures, deletion boundaries and actual keyboard/pointer interaction.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Implemented; automated gate passes and evidence is recorded in [`rust/docs/evidence/ticket-05.md`](../../../rust/docs/evidence/ticket-05.md). Native host verification (pointer hold, keyboard confirmation, theme/scope/session preservation) is pending the final ticket-22 pass; macOS 14/15 runtime remain unverified.
