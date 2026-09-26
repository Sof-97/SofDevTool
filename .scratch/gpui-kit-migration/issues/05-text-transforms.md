# 05: Encoding, hashing and text transformation workspaces

**What to build:** Use Base64, URL encoding, hashes, case conversion and whitespace conversion through standard kit controls with unchanged results and explicit Clipboard actions.

**Blocked by:** 01: Workbench and official Catppuccin themes

**Status:** ready-for-agent

**Type:** task

- [ ] Migrate all five workspace control/feedback surfaces to the kit, including mode and algorithm choices.
- [ ] Preserve valid/invalid input, current limits and each Utility-owned output contract.
- [ ] Input editing, output selection/copy, keyboard navigation and diagnostic presentation work in both themes.
- [ ] Session switching and History capture/restore preserve existing semantics and do not record incomplete edits.
- [ ] Use existing Utility tests plus representative application interaction scenarios; notify the owner about upstream incompatibilities.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
