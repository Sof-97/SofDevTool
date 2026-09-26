# 03: Settings and Utility History confirmations

**What to build:** Configure and inspect Utility History and perform destructive actions through standard accessible confirmation dialogs.

**Blocked by:** 01: Workbench and official Catppuccin themes

**Status:** ready-for-agent

**Type:** task

- [ ] Replace hold buttons and confirmation bars with kit dialogs, including cancel, confirm and keyboard focus behavior.
- [ ] Preserve collection/retention settings, per-Utility and global deletion scopes, failure feedback and retry behavior.
- [ ] Clear/restore reconciles visible and hidden History views and invalidates stale selected restore targets.
- [ ] Only completed valid Utility Operations are retained; no payload logging, sync, indexing or export is introduced.
- [ ] Reuse Settings/History tests and isolated storage to prove cancel does not delete, confirm has the intended scope, and restore does not record again.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
