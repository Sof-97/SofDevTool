# 06: Synchronize color controls, output and retained operations

**What to build:** Each channel adjustment updates the latest requested color consistently in text, swatch, conversions, Copy and settled History.

**Blocked by:** [01: Use sofui's owned component interfaces in JSON and the gallery](01-sofui-json-gallery.md).

**Status:** claimed

- [ ] Use the documented editor event semantics or deliberate scheduling; do not depend on silent assignment emitting an edit.
- [ ] Repeated adjustments before and after settlement accumulate from the latest requested color; invalid text does not leave stale output copyable.
- [ ] Expose reusable channel interaction through sofui and demonstrate it in the gallery, keeping sRGB parsing, formatting and validation in the Utility.
- [ ] Exercise the actual control/event path, clamping, settlement and feedback-loop prevention; one settled operation creates at most one snapshot.

## Testing

Color workspace plus real component events and controlled settlement, backed by existing sRGB core contracts.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 43, 44, 50, 59, 63, 68.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Implementation handoff

GPT-6 Sol completed the synchronized workspace and public NumericStepper.
Real keyboard events, Copy, clamping, controlled settlement and one-shot
History passed the focused GPUI test. All nine sofui tests, app all-target
compilation, scoped Clippy and formatting passed.
[Evidence](../evidence/ticket-06.md). Native and final Astra acceptance remain
pending; Color ownership transfers to03 after this integration.
