# PierreDiffsSwift validation spike

> Disposable prototype. It validates a dependency decision and is not production SofDevTool code or UI.

This macOS 14+ SwiftUI executable vendors the published `PierreDiffsSwift` `1.2.4` revision with a documented UTF-8 bridge patch. It exercises split and unified rendering, word-level intraline highlights, editable source text, explicit case/whitespace preprocessing, a 10,000-line stress sample, render-ready timing, and keyboard-accessible native controls. Multi-scalar emoji automatically use whole-line highlighting because Pierre's intraline spans otherwise break the grapheme into separate glyphs.

Run from this directory:

```sh
swift run PierreDiffSpikeApp
```

For a repeatable Unicode launch, pass `--unicode`. For a stress launch, pass `--stress`; add `--unified` to start in unified mode. The process prints a `PIERRE_READY` timing when the WebView reports that rendering completed.

After the first dependency resolution and build, disconnect networking and run the same command again to validate that rendering uses only bundled resources.

Regression checks:

```sh
node Measurements/unicode-bridge-check.mjs
swift test
```

## Owner review checklist

1. Switch between Split and Unified with keyboard focus and confirm both remain readable.
2. Load Small Swift and confirm word-level changes are highlighted.
3. Load Unicode and whitespace. Confirm the combined emoji render as single glyphs. The status line should disclose **Emoji-safe whole-line highlighting**; ordinary samples retain word-level intraline highlighting. Toggle each ignore option separately and confirm the semantics feel right: case uses Unicode case folding; whitespace removes all per-line whitespace while preserving line boundaries.
4. Load 10,000 lines and note the ready time and whether scrolling stays responsive.
5. With VoiceOver enabled, inspect the renderer, line numbers, removed/added content, and control labels. Record what is announced and whether navigation gets trapped inside the web content.
6. Disconnect networking, relaunch, and confirm the same samples render.

The ticket cannot be resolved until the owner completes the human accessibility and interaction review.
