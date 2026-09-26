# 08: Text Diff with kit controls and synchronized appearance

**What to build:** Compare text with the existing specialized renderer while all surrounding controls and effective appearance follow GPUI Kit.

**Blocked by:** 01: Workbench and official Catppuccin themes

**Status:** ready-for-agent

**Type:** task

- [ ] Replace custom input/options/actions/feedback controls with kit components while retaining comparison capabilities and current limits.
- [ ] Retain the specialized renderer and native embedding; do not assume an experimental adapter is required.
- [ ] Propagate initial and changed Latte/Frappe effective appearance to the renderer, including System changes; remove independent hardcoded dark chrome.
- [ ] Preserve snapshots, restored inputs, comparison refresh, explicit Clipboard actions and session lifecycle.
- [ ] Keep assets reproducible, bundled and usable offline; exercise native focus, resize, keyboard interaction and recovery separately from build evidence.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
