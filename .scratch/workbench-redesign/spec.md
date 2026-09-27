# Workbench interface refinement

Approved by the owner in this chat on 2026-09-26. Implementation authorized after the visual proposal review.

## Design target

Reference: /Users/gerardo/.codex/visualizations/2026/09/26/01a0de8e-ae04-7683-82ab-f1a4d442d936/sofdevtool-interface-proposal.html

Preserve the grouped catalog, retained Utility sessions, native GPUI Kit ownership and the existing Utility domain contracts. Use compact navigation rows, one Utility heading, pane-local actions, semantic controls and an optional History inspector. Move appearance controls into Settings. Keep exact build provenance available.

History visibility is a UI preference, separate from recording. New visibility preferences default closed. Effective recording status must account for both global and per-Utility settings. Restore remains explicit. Recording defaults, retention, valid-operation capture, deletion confirmations and payload privacy remain unchanged.

Random String configuration occupies only its content height. Results receive remaining height. Text Diff retains its specialized renderer. Text editor and comparison splits use kit resize support where applicable. At constrained widths, preserve usable editors and make History accessible without overlapping content.

## Scope

Workbench, shared app-owned History presentation, JSON, Random String, Text Diff, Launcher and Settings. Apply shared presentation changes consistently to other consumers. Do not alter core engines, snapshot semantics, Clipboard policy or the approved grapheme deletion adapter.

## Acceptance

- Native layout at 1000x700, 1280x800 and a large window, with no overlapping controls or inaccessible actions.
- Supported System, Light and Dark appearance modes remain available.
- Keyboard navigation, accessible action names, visible focus, Utility session retention and explicit Clipboard actions remain functional.
- Showing or hiding History does not change recording. Effective recording status reflects global and Utility settings.
- History restoration and confirmed deletion retain their existing behavior.
- JSON live validation, Random String explicit generation and Text Diff editing/comparison remain intact.
- Run make verify and inspect the actual packaged native build. Do not treat compilation as native evidence or claim macOS 14/15 acceptance on a different host.

## Execution plan

1. Ground: reuse completed UI/code review and inspect concrete kit APIs.
2. Sketch: independent Astra and Sol design packages.
3. Agree: cross-judge and synthesize technical shape within approved design.
4. Implement: one Sol owner in this isolated worktree, checked in ordered slices.
5. Scrap: redesign only if evidence invalidates the selected architecture.
6. Review, native verification, local delivery; publishing requires the exact remote approval.

## Throughput checkpoint

- Blocking first steps: select architecture and inspect pinned kit APIs before code writes.
- Independent workstreams: design candidates are independent; implementation is coupled.
- Shared mutable state: one implementation owner writes shared UI/preferences and their callers; reviewers are read-only.
- Smallest safe decomposition: shared shell and History presentation, JSON/Random String, then Text Diff/Launcher/Settings, each checked before proceeding.
