# 01: Use sofui's owned component interfaces in JSON and the gallery

**What to build:** JSON editing, Copy and keyboard navigation work through documented reusable controls; the gallery uses the same controls without inheriting the application's lifecycle.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Name and independently version the library as sofui 0.1.0; document its future separate release and component-only rule in the agent guide. Move process startup into application/gallery composition.
- [ ] Introduce stable identities, focus/keyboard behavior and explicit silent-assignment versus edit/undo semantics; retain the existing editing engine behind owned interfaces.
- [ ] Adopt the new interfaces in JSON and representative gallery controls, covering repeated labels, redraw, Unicode editing, disabled actions and Copy feedback through public behavior.
- [ ] Keep compatibility adapters for unmigrated consumers so application and gallery builds remain green. Record the remaining consumers for removal in ticket 19.

## Testing

JSON workspace interactions plus public component/gallery behavior; existing editor and grapheme tests remain useful but are not the only evidence.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 15, 55, 56, 57, 58, 59, 64.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
