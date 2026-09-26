# 09: Retire sofui and integrate the complete replacement

**What to build:** Produce the single complete Developer Toolbox replacement with no remaining sofui product, custom shared controls or competing theme system.

**Blocked by:** 01: Workbench and official Catppuccin themes, 02: Utility Launcher with kit controls, 03: Settings and Utility History confirmations, 04: JSON, YAML/JSON and JWT workspaces, 05: Encoding, hashing and text transformation workspaces, 06: Identifier, random string, sample data and timestamp workspaces, 07: Regex and Color Conversion workspaces, 08: Text Diff with kit controls and synchronized appearance

**Status:** ready-for-agent

**Type:** task

- [ ] Remove sofui, its gallery, standalone consumer, package/version intent, dedicated isolation tooling and obsolete tests after preserving relevant behavioral coverage.
- [ ] Audit every maintained app surface and dependency for remaining custom controls, wrappers and palette state; only approved app compositions and the specialized Text Diff renderer remain.
- [ ] Update workspace/build/package commands and ownership documentation to the agreed application/core/kit responsibilities.
- [ ] Integrate all work packages without intermediate release, fallback UI or a replacement custom component library.
- [ ] Run make verify-full on the complete tree, retaining app, domain, renderer and bundle checks; resolve failures and record exact revision and results.
- [ ] Keep History/private data untouched and record any unresolved upstream incompatibility as a blocker rather than declaring completion.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
