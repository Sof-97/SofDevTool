# Independent cross-judge

Read-only comparison of `astra.md` and `sol.md` against the approved interface proposal and current SofDevTool source. No native UI acceptance or implementation was performed.

| Criterion (1-5) | Astra | Sol | Judgment |
| --- | ---: | ---: | --- |
| Utility, privacy, and GPUI Kit boundaries | 5 | 5 | Both keep typed Utility execution, snapshots, restoration, clipboard and History policy with their current owners. Both leave the multiline adapter and Text Diff renderer as explicit exceptions. |
| Coherent single-owner state without generic executor | 5 | 4 | Both give Workbench one visibility owner and avoid an erased execute API. Astra's `Entity<WorkspaceLayout>` includes active Utility and measured width in the same presentation owner; Sol's `Rc<InspectorVisibility>` owns only visibility and requires shell/workspace coordination for responsive placement. |
| Minimum API and maintenance complexity | 4 | 3 | Both migrate constructors and add one preference file. Astra reuses GPUI `Context::observe` and `cx.notify`; Sol adds `RefCell`, a subscriber map, a subscription token, callback dispatch, and reentrancy rules for state that GPUI can already observe. Astra's broader layout entity needs disciplined idempotent updates and a small, typed API. |
| Installed Kit feasibility, resize, sheet and focus | 4 | 3 | Both correctly identify pinned Kit resize and sheet surfaces. Astra verifies `WindowExt` transitions and addresses the button-opened focus problem, sheet dismissal, Utility switch, and live sheet content in greater detail. Neither supplies a compiled integration or native proof that a Kit sheet covers the Wry WebView. |
| Approved scope and verification | 5 | 4 | Both cover catalog, JSON, Random String, Text Diff, Settings, Launcher, History separation and root/native gates. Astra is more explicit about all retained workspaces, default-closed persisted visibility, status changes after retry/clear, renderer hiding, and restore-to-select synchronization. |
| **Total** | **23/25** | **19/25** | **Use Astra as the base.** |

## Decision

The candidates converge on the important product architecture: one Workbench heading controls History visibility, a separate effective recording label reports policy and pause state, and each Utility keeps its own inspector and typed operation behavior. The difference is notification structure. Choose Astra's `Entity<WorkspaceLayout>` because GPUI 0.3.6 already provides `Context::observe`, retained `Subscription`, and `Context::notify`; another synchronous subscriber registry would duplicate that mechanism. Current `HistoryRecorder` has callback subscribers for storage updates, but that precedent does not require a new event bus for GPUI-owned presentation state.

Keep Astra's entity narrowly scoped. Prefer `UtilityId` throughout the in-memory interface, converting to slugs only at the preference boundary. Make visibility and active/width setters idempotent; persist only visibility changes. Avoid notifying all 15 workspaces on every pixel of resize when placement has not changed. Have one window-level coordinator own the active Kit sheet transition, while the selected Utility supplies its existing typed History content and restore callbacks. Do not open/close a sheet during rendering or synchronously re-enter an entity from its observer.

## Grafts from Sol

- Make save failure visible without including payload data, while keeping the in-memory visibility choice for this launch. Ignore unknown Utility slugs and recover to all-closed on missing or invalid layout preferences.
- Keep Sol's explicit statement that the normal-width inspector can resize, and that split sizes remain session-only until native dragging is verified. Preserve focused editor handles before the History button takes focus.
- Verify keyboard reachability and accessible names for icon-only favorites and History actions. The approved proposal requires this; tooltip text alone is insufficient.

## Remaining implementation risks

The Kit `Root::open_sheet_at` captures the *currently* focused handle and `close_sheet` restores it. Clicking the shell action may already have moved focus, so the previous editor must be tracked explicitly. The sheet and WebView relationship is a native geometry/z-order question: Text Diff currently calls `renderer.set_active`, which maps to Wry `webview.set_visible`; hide the native surface while the sheet is present and restore it on close, Utility switch and Launcher activation. Exercise overlay close, Escape, resize across the threshold, and Utility switching with the sheet open. Neither candidate's API sketch is compile proof; check the exact icon variants, `SelectState` delegate wiring and `ResizableState` signatures during implementation.

Verification should include `make verify` and actual macOS interaction at the proposed widths and themes, with History open/closed, recording overrides, paused state, JSON restore, Random String generation controls, Text Diff drag/focus, extended-grapheme deletion and retained sessions. A green root gate is not native acceptance.
