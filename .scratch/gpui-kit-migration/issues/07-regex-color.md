# 07: Regex and Color Conversion workspaces

**What to build:** Inspect regex matches and convert colors using standard kit controls while retaining specialized domain result presentation.

**Blocked by:** 01: Workbench and official Catppuccin themes

**Status:** ready-for-agent

**Type:** task

- [ ] Migrate Regex and Color Conversion controls/options/diagnostics to the kit.
- [ ] Preserve regex evaluation, flags, limits and current result behavior without conflating domain presentation with a reusable custom widget library.
- [ ] Preserve color formats, channel editing and RGBA previews; data colors are not replaced by theme colors.
- [ ] Use kit tokens for chrome and preserve explicit copy, keyboard interaction, session state and History restore.
- [ ] Check representative valid/invalid inputs, changed controls and both theme appearances using existing domain expectations.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
