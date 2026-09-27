# Native verification record

Status: second native pass complete on macOS 26.2; target macOS 14/15 acceptance remains unverified.

## First packaged build

2026-09-26, macOS 26.2 (25C56). Debug 0.2.0 build 1, base revision `9f678b920cc88e13760ebb0cb2be43bf8f042cc4`, source dirty. Bundle `/tmp/sofdevtool-redesign-preview/SofDevTool Debug.app`; isolated support root `/tmp/sofdevtool-redesign-profile`. The process exited normally before repackaging. This is not macOS 14/15 acceptance.

Observed through native Computer Use, not inferred from tests:

- JSON starts with History closed. Valid live evaluation, Sort keys, invalid-input result clearing, explicit typed restore and confirmation cancellation work. Escape from the narrow History sheet returns keyboard input to the editor. A complex extended grapheme deletes as one unit; Undo restores it.
- History collected settled valid operations while hidden. Selecting a retained row did not restore it. Restore remained an explicit action. No destructive History confirmation was accepted.
- Random String configuration fits the approximately 1000x700 window without the former expanding blank field. Explicit Generate produced a two-result batch. Changing a checkbox and switching Utility retained the batch and controls. JSON input also survived switching.
- Text Diff rendered a synthetic change in Split and Unified. Dragging the vertical divider changed source/comparison heights. The renderer was hidden under the narrow History sheet and returned, correctly positioned, after Escape.
- Settings opened without reentrant failure. System, Light and Dark worked. A global recording override retained individual switch choices and displayed the effective disabled state. JWT remained disabled by default. Clear All opened a confirmation; Cancel dismissed it. Changing appearance in a second Settings window updated the first window's selected segment.
- Launcher search, Down/Up and Enter opened the intended Utility without losing its session.

Native findings sent back to implementation:

1. Newly used icons were blank while existing icons rendered.
2. JSON indentation Select expanded to full width and displaced Sort keys.
3. Catalog scope overflowed and Identifier Generator was truncated.
4. Widening/narrowing with History visible removed the inline panel without immediately showing the sheet; explicit hide/show worked.
5. Text Diff source panes only filled their minimum widths, leaving unused space beside them.

The first pass exercised 1536x1084 and 1728x1084 logical windows, then approximately 1002x702. Exact target-size checks, repaired layout checks, horizontal pane resizing, copy feedback and visibility persistence across relaunch remain for the second pass.

Use a newly packaged bundle with exact revision and a distinct `SOFDEVTOOL_RUST_SUPPORT_ROOT`. Use synthetic inputs only. Do not copy, export or log personal History payloads. Record observations, counts and UI state, not payloads.

## Corrected packaged build

2026-09-26, same isolated profile and bundle path, same dirty base revision. Executable SHA256: `fd7a0485a35fadd04dd652da99fb8a74484f4e3f7e206b349196cd14eec11553`. The corrected process exited normally after verification.

- All five first-pass layout findings were repaired and checked in the running bundle. Kit icons render; the JSON Select and Sort checkbox share a compact row; catalog scopes and Identifier Generator fit; History switches automatically between inline and sheet presentation; Text Diff sources fill the available width.
- Exact logical sizes 1000x700 and 1280x800 were observed, alongside 1536x1084. JSON History retained its open preference across process restart. Narrowing opened the populated sheet automatically; widening restored the inline panel without another toggle.
- Dragging the JSON horizontal divider resized both panes. Copy result showed Copied feedback; explicit Paste inserted that formatted result. Keyboard selection of four-space indentation changed the actual output.
- Text Diff rendered a synthetic changed line at 1000x700 with full-width source panes. Opening History hid the WebView; Escape restored the comparison at the correct bounds.
- Random String controls and results fit at 1000x700. Explicit Generate produced two results. Launcher Escape returned to that workspace with the batch present.

## Limits and automated evidence

`make verify` passed after the final source fixes: formatting, Clippy with warnings denied, Debug build, 340 Rust tests and five packaging tests. Independent source review by codex:gpt-6-astra@medium found no remaining blocking issue after repairs. Tests and source review are not substitutes for native evidence.

Not exercised: native macOS 14/15, IME composition, destructive History confirmation acceptance, exact byte-for-byte Random String batch retention, Random String Copy all, and a complete manual pass through every Utility. Unicode Backspace/Undo, retained batch count, Settings synchronization, and vertical Text Diff resizing were exercised in the first pass. No personal History payload was read, exported or logged. Clipboard copy testing left synthetic JSON in the system clipboard.

## Scenario checklist reference

- Confirm bundle identity and source revision before recording acceptance.
- Inspect 1000x700, 1280x800 and large-window layouts. Confirm catalog, Utility controls, text panes and History remain reachable without overlap.
- JSON: valid and invalid input, indentation, Sort keys, live result, pane-local copy feedback, Unicode deletion, undo/redo and retained session across Utility switch.
- History: default closed, visibility preserved across Utility switch and relaunch, panel visibility independent of global/per-Utility recording, effective status, explicit restore and confirmation cancellation.
- Random String: content-height configuration, checkbox and numeric controls, explicit Generate, results receive remaining space, copy action, exact retained batch after Utility switch.
- Text Diff: original/updated editing, split/unified mode, pane resizing, comparison resize, renderer visible and aligned after layout changes, History sheet occlusion/focus recovery when narrow.
- Launcher: search, arrows, Enter, Escape and return to the chosen Utility.
- Settings: appearance controls, System/Light/Dark, recording controls, global override presentation, no deletion without confirmation.

Record any unexercised scenario explicitly. Automated gate results are separate in baseline.md and the implementation issue.
