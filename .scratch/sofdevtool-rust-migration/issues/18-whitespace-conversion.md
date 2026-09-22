# 18: Transform whitespace with explicit actions

Type: task
Status: resolved
Blocked by: 04

Parent: [Rust and GPUI migration specification](../spec.md)

**What to build:** Choose a named whitespace operation, inspect the result and restore exact input/options/output.

## Acceptance criteria

- [ ] Implement the nine existing actions separately, with no generic Clean command.
- [ ] Preserve LF default, LF/CRLF/CR options, tab widths 1–8/default 4, tab-stop expansion and leading-indent-only spaces-to-tabs.
- [ ] Match explicit trim/collapse/blank-line/dedent rules with Unicode-aware fixtures and no silent partial processing.
- [ ] Deliver complete Registry/session/Clipboard/History integration and action-specific boundary/restore tests.
- [ ] Run the relevant documented Rust gate and focused native scenarios for this slice; record actual commands, host and results without presenting compilation as runtime evidence.

## Completion evidence

Resolved. Nine separate named actions with LF default, LF/CRLF/CR targets, tab
width 1–8 default 4, tab-stop expansion and leading-indent-only spaces-to-tabs;
explicit trim/collapse/blank-line/dedent rules with Unicode-aware fixtures and
no silent partial processing. Full Registry/session/Clipboard/History flow.
Evidence:
[`rust/docs/evidence/ticket-18.md`](../../../rust/docs/evidence/ticket-18.md).
