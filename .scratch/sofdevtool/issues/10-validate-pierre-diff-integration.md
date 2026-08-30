Type: prototype
Status: resolved
Assignee: Codex
Blocked by: 01

## Question

Can PierreDiffsSwift serve as SofDevTool's Text Diff renderer without violating the native, offline, accessible, responsive, and agent-verifiable architecture? Build a focused disposable integration and validate a pinned compilable release, fully offline operation, unified and split views, intraline highlighting, keyboard navigation, VoiceOver/accessibility behavior, large-input responsiveness, whitespace/case-ignore preprocessing, binary and app-size impact, bundled JavaScript behavior, and required MIT/Apache-2.0 notices. Decide whether to adopt Pierre, wrap it behind a replaceable renderer boundary, or reject it for a native implementation.

## Comments

### Disposable integration ready for owner review — 2026-08-28

- Prototype: [PierreDiffsSwift validation spike](../prototypes/pierre-diff-spike/README.md)
- Evidence: [PierreDiffsSwift validation evidence](../prototypes/pierre-diff-spike/EVIDENCE.md)
- Exact pin: published `PierreDiffsSwift` `1.2.4`, revision `c2249d7890de957a96480711152d90a06fa1222b`; debug and release builds pass on Apple Silicon macOS 26.2 with the macOS 14 deployment floor.
- Automated observations: split/unified and intraline rendering work; a 10,000-line sample completes in roughly 1.8–2.5 seconds in debug; the same stress render completes with all network access denied; the accessibility tree exposes structured filename, line-number, and changed-text content.
- Material cautions: approximately 12.25 MB executable-plus-JavaScript overhead versus a minimal SwiftUI baseline; readiness fires twice on initial stress launches; WebKit developer extras are enabled unconditionally; package release/changelog numbering is inconsistent; the bundle includes Apache-2.0, MIT, BSD-3-Clause, and ISC code but ships only the wrapper's MIT license.
- Provisional recommendation: if the owner accepts the interaction tradeoffs, contain Pierre inside one deep `TextDiffRenderer` module and exact-pin it. Keep ignore semantics in the Text Diff domain module. Reject Pierre if VoiceOver/keyboard behavior, large-diff scrolling, or normalized ignore-mode display fails review.
- Remaining HITL gate: owner must review keyboard focus, VoiceOver order/verbosity, 10,000-line scrolling/layout switching, and whether ignored changes may be shown using normalized text. Ticket remains claimed until that feedback is recorded.

### Owner feedback and Unicode/emoji correction — 2026-08-28

- The owner reported that the prototype otherwise seemed fine and identified the Unicode/emoji visualization as the remaining problem. The screenshot also showed accented-text mojibake, narrowing the issue beyond font styling.
- Root cause: the wrapper fed JavaScript `atob()` bytes directly to `JSON.parse()` without UTF-8 decoding. A documented patch in the disposable vendored `1.2.4` revision now converts the bytes with `TextDecoder`; the bridge regression preserves accented text and complete emoji sequences.
- A second rendering constraint remained: Pierre's intraline spans split multi-scalar emoji graphemes and prevent WebKit from shaping ZWJ sequences into one glyph. The prototype now retains word-level intraline highlighting for ordinary text and single-scalar emoji, but automatically uses disclosed whole-line highlighting when either input contains a multi-scalar emoji (including ZWJ, skin-tone, flag, and keycap sequences).
- Verification: the bridge regression passes; four Swift policy tests pass; the rebuilt Unicode sample's rendered accessibility tree contains intact `café`, `Straße`, `👩🏽‍💻`, `CAFÉ`, `STRASSE`, and `👩🏽‍🚀`, reports `Emoji-safe whole-line highlighting`, and contains no mojibake.
- The corrected prototype is reopened on the Unicode sample. Ticket remains claimed pending the owner's final visual confirmation of the corrected combined emoji appearance.

### Owner decision — 2026-08-28

The owner approved the corrected Unicode and emoji presentation without further changes.

## Answer

Adopt PierreDiffsSwift as SofDevTool's Text Diff renderer only behind one replaceable `TextDiffRenderer` module. The Text Diff workspace and domain logic must not import PierreDiffsSwift directly. The renderer module owns the third-party import, `WKWebView` lifecycle, split/unified and overflow translation, intraline policy, readiness/error translation, accessibility accommodations, bundled notices, and dependency-specific verification. Its interface accepts display-ready old/new text, filename, display mode, and renderer callbacks; it does not expose Pierre types.

Do not consume the published `1.2.4` release unchanged. Use an exact upstream release containing an equivalent UTF-8 bridge correction, or pin a maintained fork by immutable revision. The required correction converts JavaScript `atob()` output into a byte array and decodes it with `TextDecoder` before `JSON.parse()`. Keep a regression that transports accented text and multi-scalar emoji across the real bridge.

Use word-level intraline highlighting for ordinary text and single-scalar emoji. When either input contains a multi-scalar emoji grapheme—including ZWJ, skin-tone, flag, or keycap sequences—use clearly disclosed whole-line highlighting so WebKit can shape the emoji as one glyph. The owner approved this fallback. Keep ignore-case and ignore-whitespace semantics in the Text Diff domain module; Pierre receives the resulting display-ready comparison text.

The dependency is otherwise accepted on the validated evidence: macOS 14 and Swift 6 compilation, split and unified views, offline rendering under denied network access, structured accessibility content, keyboard use, owner-approved presentation, and completion of the 10,000-line stress scenario in roughly 1.8–2.5 seconds in the debug spike. Treat readiness callbacks as idempotent because the spike observed two initial ready events.

Accept the measured approximate 12.25 MB executable-plus-bundled-JavaScript overhead for the first release. Disable WebKit developer extras in the production integration or carry a documented reason if an upstream release still forces them. Generate and check in a complete third-party notice inventory from the exact JavaScript bundle lockfile: the wrapper's MIT license alone is insufficient because bundled runtime code also carries Apache-2.0, BSD-3-Clause, ISC, and additional MIT obligations.

Prototype and full evidence: [PierreDiffsSwift validation spike](../prototypes/pierre-diff-spike/README.md) and [PierreDiffsSwift validation evidence](../prototypes/pierre-diff-spike/EVIDENCE.md).
