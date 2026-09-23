# Ticket 02 — embedded Text Diff proof

## Scope and integration

The application retains concrete JSON and Text Diff workspace entities. Wry
0.53.5 creates a WKWebView child of the GPUI window; the custom surface updates
its bounds in prepaint and hides it when JSON is selected. Native web types stay
in the application renderer module. Launcher and History remain unfinished
subsequent slices and are excluded from this integration.

Pierre's vendored bundle SHA-256 is
`7343dfd142566cb0fbaa9a5fe6d6deae69c7143f61842b1b3ec6b3519e5a2a64`.
The full JavaScript license inventory and Wry MIT/Apache attribution are embedded
from `crates/app/src/text_diff/assets/THIRD_PARTY_NOTICES.md`. The HTML embeds
local assets, blocks remote subresources with CSP, and rejects navigation.
The compatibility bridge is checked against exact signatures before use;
revision-correlated callbacks use Wry IPC directly. Complex emoji visibly
disclose the retained whole-line highlight fallback.

## Reproducible native checks

Host: macOS 26.2 (25C56), Apple Silicon arm64; Rust 1.98.1. Root performed
Computer Use checks on the bundled Debug executable outside the checkout at
`/private/tmp/SofDevToolRustTerra.app`. All inputs were synthetic.

1. Open Text Diff: initial split rendering appears without clicking a mode;
   GPUI editors and web text preserve café, family emoji and rainbow flag.
2. Select `let ready = true` in the web surface, Cmd+C, Paste Original:
   GPUI receives the exact selected text. Native context-menu Copy also works.
3. Switch Unified/Split: additions/deletions render in the selected layout.
4. Paste 100 numbered Unicode lines into Updated, scroll the web surface:
   the last lines are reachable while GPUI controls remain in place.
5. Resize the window: child bounds follow its panel without covering controls;
   observed text is aligned at the host's current display scale.
6. Click a GPUI toolbar control after using the web surface, Tab, Enter:
   visible focus moves and the focused Paste action executes.
7. Switch JSON: native child hides; a synthetic JSON object formats correctly.
   Return to Text Diff: the in-memory input remains.
8. In Debug only, Renderer diagnostics → Test renderer failure sends a malformed
   request through the real bridge. The Error banner appears. Click Unified:
   the valid diff returns and both Error and Loading disappear.

The native failure loop caught a fragile Swift-style WebKit handler shim that
lost callbacks after initial rendering. Direct Wry IPC fixed it; metadata-only
tracing confirmed error revision 2 then ready revision 3. Temporary trace code
is removed from the accepted source. Earlier failed candidates are recorded in
`docs/evidence/previous-migration/2026-09-22-webview-native.md`.

## Verification boundaries

Offline containment is supported by embedded assets, CSP and navigation policy;
no network capture or OS network-setting change was performed. Current-display
scaling was observed; cross-display/DPI transitions were not exercised. macOS14
and15 runtime remain unverified. The 14.0 deployment baseline is unchanged.
IME composition remains the previously recorded nonblocking evidence exception.

## Automated verification and review

The final canonical gate and independent Standards/Spec review results are
recorded with the integration in the operational register. Protocol tests cover
stale completion rejection, early/duplicate readiness, initialization failure
preservation and exact bridge-patch compatibility. They complement rather than
replace the native checks above.
