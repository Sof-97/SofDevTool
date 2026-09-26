# 04: JSON, YAML/JSON and JWT workspaces

**What to build:** Use kit text controls to inspect, convert and format structured input while retaining validation, results and History restoration.

**Blocked by:** 01: Workbench and official Catppuccin themes

**Status:** ready-for-agent

**Type:** task

- [ ] Migrate JSON, YAML/JSON and JWT workspace controls and feedback directly to the kit.
- [ ] Preserve each Utility contract, query/format options, current limits and the JWT Decoder distinction from validation.
- [ ] Exercise Unicode deletion, selection, undo/redo, read-only result copy and edit-versus-restore behavior at the app boundary.
- [ ] Preserve session state and prevent stale async results or restored snapshots from producing incorrect or duplicate History entries.
- [ ] Reuse independent domain vectors and report any upstream editing regression without a silent custom fallback.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
