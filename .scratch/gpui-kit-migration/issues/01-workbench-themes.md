# 01: Workbench and official Catppuccin themes

**What to build:** Open the Developer Toolbox with kit-owned Workbench navigation and theme management, preserve Utility organization and sessions, and choose System, Light or Dark appearance across restarts.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

**Type:** task

- [ ] Confirm and pin the latest stable kit at implementation start; record compatibility with the current GPUI runtime.
- [ ] Initialize and mount the kit from the consumer and replace shell controls and application chrome with kit components/tokens without introducing compatibility wrappers.
- [ ] Bundle unmodified official Latte/Frappe presets; Light, Dark and live System appearance work offline.
- [ ] Migrate both old theme preferences to Dark; fresh profiles use System; subsequent choices and unrelated preferences survive relaunch with Debug/Release isolation.
- [ ] Workbench navigation, favorites/recents and retained Utility Workspace Sessions preserve their observable behavior.
- [ ] Provide preference and shell interaction checks; note that the final green build is owned by integration, not by an intermediate mixed product.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
