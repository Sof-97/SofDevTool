# Ticket 02 native integration observations

Host: macOS 26.2 (25C56), arm64. Computer Use owns desktop interaction;
all input is synthetic. These observations do not claim ticket acceptance.

Proof executable: `/private/tmp/sofdevtool-terra-02-target/debug/sofdevtool`,
launched from `/private/tmp/SofDevToolDiffProof.app`. It opens Text Diff directly
for diagnosis; final product composition remains owned by ticket03.

## First runnable Pierre candidate

- GPUI old/new input controls and native child WebView appear in one window.
- Initial child is blank; clicking Unified causes real Pierre diff hunks to
  appear. Original/updated content includes caffè, family emoji and rainbow flag.
- Switching Split shows old/new diff columns and matching line-change counts.
- The original editor containers visibly clip their multiline content.
- Dragging across the right-hand changed line visibly selects WebView text.
- Cmd+C followed by explicit Paste Original does not copy that selection;
  the prior clipboard value is pasted instead.
- Right-click selected café -> native WK Copy -> explicit Paste Original DOES
  yield exact café. This isolates native copy capability from missing Cmd+C
  responder/menu forwarding. No external menu actions were invoked.

## Assigned corrections

- Queue flush on real bridge readiness must also cover readiness during child
  construction, before the WebView is assigned to state.
- Editor-half containers need flex layout so panels fill the fixed input row.
- Ticket03 owns the standard Edit menu/first-responder integration for Cmd+C.
- Source review additionally requires real revision-correlated callbacks,
  visible asynchronous failure/loading status, disclosed emoji fallback,
  local-only navigation/resource policy, and hidden child when workspace inactive.

Updated candidates require a new initial-render, copy/focus, resize/scroll and
workspace-switch pass. Build and helper tests alone do not establish those flows.

## First combined 02/03 native candidate

Launched the copied debug binary from `/private/tmp/sofdevtool-terra-03-target`
in `/private/tmp/SofDevToolRustTerra.app` through Computer Use. Workbench
sidebar, header and footer render, but both JSON and Text Diff bodies are blank;
AX still exposes their text inputs. Clicking Open Utility Launcher aborts the
process. macOS diagnostic report `sofdevtool-2026-09-22-005804.ips` identifies
SIGABRT and a Rust panic crossing `gpui_macos::window::handle_view_event`.
These are failing native scenarios, despite passing focused source tests and
Clippy. Corrections and re-verification are required before accepting 02/03.

## Isolated ticket02 composition (01:18:40 candidate)

The acceptance candidate now excludes unfinished Launcher/History code. It
retains concrete JSON and Text Diff entities behind two sidebar buttons.
Native observations on the same host:

- Initial split diff appears without clicking a display-mode control; editor
  panels have full height and preserve the supplied Unicode.
- Select `let ready = true` in the native web surface, Cmd+C, then Paste Original:
  the GPUI editor receives exactly `let ready = true`.
- Unified mode displays the expected additions/deletions.
- Paste 100 synthetic lines containing café and a skin-tone ZWJ emoji into
  Updated, then scroll the web surface: lines 72–100 are visibly reachable.
- Switch to JSON: native web child is hidden. Paste a synthetic JSON object:
  formatted output appears with Unicode preserved.
- Debug-only Renderer diagnostics → Test renderer failure clears the diff but
  leaves Loading visible. This is a FAIL requiring correction; successful
  source review did not prove the native callback path.
- Keyboard traversal after returning from the web surface also needs a native
  focus correction/retest. Do not treat these scenarios as completed yet.

## Final passing integration

The direct Wry IPC adapter replaced the fragile Swift-style WebKit handler
shim. Metadata trace confirmed error revision2 and ready revision3 reaching
Rust and the observer. Real Error banner followed by Unified recovery passed.
The temporary trace was removed. The canonical source was rebuilt and bundled;
its uninstrumented executable was copied into the same outside-checkout test
bundle and the Error/recovery scenario passed again.

Tab focus after web interaction and Enter activation of Paste pass after native
parent focus handoff and explicit GPUI button focus. Final resize and switch
away/back checks pass. The current display scale was observed only; no
cross-display transition, OS input setting change or network capture occurred.

Canonical command: `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-03-target rust/scripts/verify`, exit0; 39 tests, fmt, Clippy and Debug.
`rust/scripts/bundle-debug` with the same environment passed.
Full reproducible scenarios and limits: `rust/docs/evidence/ticket-02.md`.
