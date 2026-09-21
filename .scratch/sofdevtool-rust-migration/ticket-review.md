# Rust migration ticket review

Status: approved — all 22 tickets published locally as ready-for-agent

The owner approved the specification, testing boundaries, ticket granularity and blocking edges. Implementation is delegated to a new orchestrator session; no ticket has been started here.

## Approved test boundaries

1. Utility-owned request/result/snapshot contracts, using independent fixtures and deterministic clock/random inputs.
2. Application sessions plus temporary persistence, covering recording, restore, preferences, isolation and obsolete results.
3. Real macOS application flows for text editing/IME, Launcher/Spaces/Dock and embedded WebView behavior; focused automation plus explicit manual evidence where required.

## Approved breakdown

Each numbered entry links to an individual published ticket. Blockers are technical dependencies, not an instruction to run multiple writers in one checkout.

1. **[Run JSON in a GPUI Workbench with reusable text controls](issues/01-json-editor-foundation.md)**
   - **Blocked by:** None.
   - **What it delivers:** Open a separately identified Rust application, paste JSON, format it and copy the result through controls also demonstrated in an independent component gallery.

2. **[Compare text in an embedded web renderer inside GPUI](issues/02-web-diff-proof.md)**
   - **Blocked by:** 01.
   - **What it delivers:** Open a minimal Text Diff workspace in the same application and compare Unicode text using locally bundled web rendering.

3. **[Open Utilities through the macOS Launcher](issues/03-native-launcher.md)**
   - **Blocked by:** 01.
   - **What it delivers:** Invoke a configurable global shortcut from another app, find JSON or another registered Utility and open it in the Workbench without disturbing the prior app when dismissing.

4. **[Record and restore JSON operations in fresh Rust History](issues/04-json-history.md)**
   - **Blocked by:** 02, 03.
   - **What it delivers:** Use JSON in the proven native shell, complete operations, inspect retained entries and explicitly restore exactly what was captured after relaunch.

5. **[Manage History retention, deletion and storage failures](issues/05-history-management.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Control recording, clear retained entries deliberately and continue using Utilities when a History file is damaged or unwritable.

6. **[Encode and decode UTF-8 with Base64](issues/06-base64.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Select Base64, encode or decode text with explicit alphabet and padding controls, copy the result and restore an exact operation.

7. **[Navigate retained Utility sessions with themed components](issues/07-workbench-themes.md)**
   - **Blocked by:** 06.
   - **What it delivers:** Switch between JSON and Base64 through searchable Library, Recent and Favorites scopes, preserving sessions and applying a coherent dark theme across Workbench, Launcher and Settings.

8. **[Encode path segments and query values](issues/08-url-encoding.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Convert URL text through separately named Path Segment and Query Value modes, copy the output and restore it from History.

9. **[Generate text hashes with explicit execution](issues/09-hashes.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Choose a digest and representation, explicitly hash the exact input bytes and restore the recorded digest.

10. **[Generate and inspect UUID identifiers](issues/10-uuid-identifiers.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Open Identifier Generator, generate or inspect UUIDs with version-specific controls and restore complete batches without regeneration.

11. **[Add ULID and KSUID to Identifier Generator](issues/11-ulid-ksuid.md)**
   - **Blocked by:** 10.
   - **What it delivers:** Select ULID or KSUID within Identifier Generator, generate or inspect values and restore complete results with the selected format.

12. **[Generate cryptographically random strings](issues/12-random-string.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Configure an alphabet and batch, generate random strings, copy individual results or all results and restore exactly the generated values.

13. **[Convert YAML and JSON with fidelity diagnostics](issues/13-yaml-json.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Convert one YAML or JSON document, understand unsupported or lossy cases and restore the exact conversion.

14. **[Convert timestamps without timezone guesses](issues/14-timestamps.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Interpret Unix/ISO/local time explicitly, inspect the same instant in selected zones and restore the captured instant.

15. **[Inspect JWT segments with History off by default](issues/15-jwt-decoder.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Paste a JWT, inspect readable header and payload separately and optionally record it only after explicitly enabling History.

16. **[Test patterns and replacements with Rust regex](issues/16-rust-regex.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Test a pattern against text, inspect matches/captures and replacement preview with clearly identified Rust regex semantics.

17. **[Convert text case with inspectable words](issues/17-case-conversion.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Inspect detected words and convert developer text through the existing nine case styles while preserving Unicode.

18. **[Transform whitespace with explicit actions](issues/18-whitespace-conversion.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Choose a named whitespace operation, inspect the result and restore exact input/options/output.

19. **[Convert and select sRGB colors](issues/19-color-conversion.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Enter a supported CSS color or select one interactively, inspect synchronized representations and restore the captured color.

20. **[Generate fictional structured sample data](issues/20-sample-data.md)**
   - **Blocked by:** 10.
   - **What it delivers:** Edit typed fields, generate fictional JSON or CSV rows and restore the exact schema and generated output.

21. **[Complete Text Diff sessions and History](issues/21-text-diff.md)**
   - **Blocked by:** 04.
   - **What it delivers:** Compare user-entered text in split or unified mode and restore the exact comparison through normal Utility History.

22. **[Package and verify the complete Rust Developer Toolbox](issues/22-release-acceptance.md)**
   - **Blocked by:** 05, 07, 08, 09, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21.
   - **What it delivers:** Launch the complete Rust application from a normal macOS bundle, use all fifteen Utilities offline and retain new Rust settings/History without affecting Swift.

## Milestones and frontier

- Technical proof: 01, then 02 and 03. All run in the same pinned GPUI application. Failure keeps the corresponding ticket unresolved and blocks broad migration.
- First persistent vertical slice: 04, followed by complete History management (05), a second Utility (06), and full navigation/theme behavior (07).
- Catalog completion: 08–21. Independent Utility slices depend on 04; Identifier format expansion and Sample Data have additional genuine dependencies.
- Packaged daily-use acceptance: 22, blocked by the terminal slices. Earlier prerequisites are inherited transitively.
- Initial frontier is 01 only. After 01, 02 and 03 are available. No task is claimed or implemented now.

## Approval and publication

The owner approved the test boundaries and breakdown in the current conversation. The specification and 22 individual tickets are ready-for-agent. Relative links and dependency order have been verified; the old Swift map and issues are unchanged. The index is derived from the individual ticket blockers, which remain authoritative.

## Source precedence

The current conversation fixes macOS-only delivery, new visual identity with preserved flows, owner-maintained GPUI components, later extraction, fresh data, Rust regex, deferred full accessibility and retained web diff. The migration specification governs the new product; previous Swift records remain historical behavior references. New technical evidence may refine implementation choices without reopening these decisions absent a concrete conflict.
