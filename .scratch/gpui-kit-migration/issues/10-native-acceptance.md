# 10: Native acceptance of the complete migration

**What to build:** Verify the complete migrated application in isolated native sessions and provide an honest completion record for the owner.

**Blocked by:** 09: Retire sofui and integrate the complete replacement

**Status:** ready-for-agent

**Type:** task

- [ ] Exercise all fifteen Utilities with representative input/result and changed-control scenarios; verify session switching and completed-operation History capture/restore.
- [ ] Exercise System/Light/Dark, live system appearance changes, both old preference migrations and restart persistence.
- [ ] Exercise Launcher activation, window reopening, keyboard navigation, Unicode editing, selection, undo/redo, explicit Clipboard actions and destructive dialog cancellation/confirmation.
- [ ] Exercise Text Diff appearance, focus, resize, restore and recovery in the packaged application offline.
- [ ] Record build identity, OS and direct observations separately from automated checks and owner-reported evidence; do not claim macOS 14/15 acceptance without direct evidence.
- [ ] Completion requires the full migration and resolution or explicit owner decision on reported incompatibilities; missing native evidence is stated and not replaced by build success.
- [ ] Do not overwrite the normal installed app or personal data for testing; use isolated profiles and a temporary bundle destination.

## Delivery constraint

Part of one atomic integration effort. This work package is verifiable by its scenarios but is not an independently accepted or shipped intermediate product. Only the final integrated tree is promised green. No compatibility wrappers or expand-contract rollout are authorized.
