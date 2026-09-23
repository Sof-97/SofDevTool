# 17: Make Text Diff assets independently maintainable and locally bundled

**What to build:** Text Diff can be rebuilt from retained editable sources and exact dependencies, then run from its local bundle without the Swift wrapper or source checkout.

**Blocked by:** None (can start immediately).

**Status:** claimed

- [ ] Preserve the JavaScript entry source, exact lockfile, generation recipe and upstream provenance before wrapper retirement.
- [ ] Verify generated resources and actual dependency notices; remove misleading inventory claims while retaining required attribution.
- [ ] Prove UTF-8/complex-emoji transport, idempotent readiness, stale callbacks, failure/recovery and local navigation/resource restrictions; verify string-based adaptations or move them into an equivalent owned source bridge.
- [ ] Document the asset rebuild and show the runtime needs neither Node/npm nor network access; identify structural-only offline evidence accurately.

## Testing

Reproducible asset build/provenance checks and the real application renderer boundary using locally bundled resources.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 07, 19, 53, 54, 69.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.

## Implementation handoff

GPT-6 Luna completed the owned pipeline and renderer boundary. Offline exact
install, reproducible bundle/inventory/notices,89 app tests and strict Clippy
passed. The coordinator requested and verified retained-source MIT attribution
and inventory based on emitted bytes; independent `npm run verify` passed.
See [evidence](../evidence/ticket-17.md). Native renderer acceptance and final
Astra review remain separate.
