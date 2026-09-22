# 03: Propagate History changes through conversion and inspection workspaces

**What to build:** Clearing History from Settings immediately invalidates cached entries in eight more Utilities, including workspaces that are open but not visible.

**Blocked by:** [02: Coordinate JSON and Base64 History with enforced recording recovery](02-history-coordination-recovery.md).

**Status:** ready-for-agent

- [ ] Migrate YAML/JSON, URL Encoding, Hashes, Timestamps, JWT Decoder, Case Conversion, Whitespace Conversion and Color Conversion to the shared coordinator.
- [ ] Remove the migrated workspaces' duplicate coordination while keeping Utility-owned snapshot decoding, preview and restore concrete.
- [ ] Exercise selection, pending restore, Clear Utility, Clear All and partial failure across visible and hidden sessions; preserve input, result and JWT's recording-off default.
- [ ] Keep remaining legacy consumers working and the application gate green after this migration batch.

## Testing

Real application workspace subscriptions with temporary storage; representative multi-workspace invalidation plus per-Utility restore contracts.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 37, 38, 41, 42, 51, 52.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
