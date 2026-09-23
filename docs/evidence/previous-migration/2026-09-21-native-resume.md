# Ticket 01 native verification resumed

Host: macOS 26.2 (25C56), arm64. This is host runtime evidence only; macOS 14/15 runtime remains unverified.

The owner explicitly authorized launching the Rust app/gallery, Computer Use screenshots, and editing/Clipboard scenarios with synthetic data in the current Codex task. This supersedes the prior suspended native permission request. Desktop control remains serialized under the orchestrator.

Candidate under observation: `c193c74b93b1b0f31ab209d00b6fe94107c4ae2c`, independent worker checkout `/Users/gerardo/.bb/thread-storage/rust-migration-thr_54ubtn5szt/worker-01`. The actual Debug bundle was launched via Computer Use. Gallery was launched independently via a temporary local bundle containing the candidate's prebuilt gallery executable.

## Observed passes on the original candidate

- Neutral empty workbench and disabled Copy Result.
- Explicit paste of JSON containing caffè, family/rainbow emoji and a musical symbol; formatted output preserved exact Unicode and two-space indentation.
- Cmd+Z removed the pasted input; Cmd+Shift+Z restored it and the result.
- Copy Result followed by Clear and the explicit Paste button restored the exact formatted output as input.
- Minify produced compact JSON; Tab from Minify visibly focused Query, and Return activated Query.
- JSON Pointer `/items/1` returned `1` for the boolean fixture, consistent with the approved baseline.
- Invalid `{broken` cleared the result, disabled Copy Result and displayed an inline line 1/column 2 diagnostic.
- Shift+Down visibly selected multiple input lines.
- A 160-row document scrolled independently within the editor; scrollbar drag and Cmd+Up returned to the beginning with the caret in view.
- Gallery independently displayed owner components, diagnostics and disabled controls. Tab visibly moved focus from Primary to Secondary and skipped Disabled to Variant.

## Native findings requiring correction

1. Backspace splits extended grapheme clusters. Reproduction: paste `{"text":"👨‍👩‍👧‍👦"}`, Cmd+Right, Left twice, Backspace. Actual input becomes `{"text":"👨‍👩‍👧‍"}`. Undo restores the original. The code/build review alone had not exposed this behavior.
2. At the default gallery window size, the bottom Empty state panel shrinks and clips `Nothing to show yet`.

GPT-5.6 Terra worker `/root/foundation_audit` owns a bounded correction in `/private/tmp/sofdevtool-terra-01/rust`. The reviewed baseline remains immutable. No dependent ticket is accepted or started by this observation.

## Evidence pending at the initial finding

The corrected grapheme, read-only, Query and gallery checks are recorded below. Real IME composition/commit/cancel remains pending; permission to temporarily add a composing Japanese input source has been requested, not inferred. Do not substitute ordinary Unicode paste for IME evidence.

Additional original-candidate observations: decomposed `e` + combining acute is also split by Backspace (leaving `e`); Copy Result visibly reports `Copied to Clipboard`; selecting all result text then Backspace and typing does not mutate the read-only result. Cmd+C from the selected result followed by Paste restores the exact result as input.

## Corrected candidate observations

The Terra correction was independently reviewed in separate Astra contexts
`/root/standards_review` and `/root/spec_review`. Both report no remaining source
findings after correcting capture routing, preserving the undo caret, and
delegating ordinary scalar/exact-selection deletion to the upstream editor.

Final Debug build observations through Computer Use:

- Family emoji Backspace removes the entire grapheme. Undo then typing `X`
  produces family emoji followed by `X`, proving the original caret is restored.
- Backward and forward deletion remove decomposed `e` plus combining acute
  together (observed on the preceding correction; final change narrows only
  ordinary scalar/exact-selection routing).
- Selecting only the final scalar of the family emoji and Backspace removes the
  entire grapheme; Undo restores the full input.
- Backspace cannot mutate selected read-only result content.
- Query `/textx`, Backspace -> `/text` still works and returns the expected value.
- Gallery `Nothing to show yet` is fully visible after making the fixed-height
  container flex and nonshrinking.

The implementation uses `capture_action` and the public `EntityInputHandler`
explicit UTF-16 replacement API; no dependency fork or vendor modification.
Extended-grapheme deletion is one atomic undo step. Ordinary deletion retains
upstream behavior. Five helper regressions were added alongside the fix; they
were not independently run red before implementation. Native reproduction is
the recorded red evidence, and the real-control checks above establish routing.

IME composition/commit/cancel remains unverified. The owner has not yet answered
the separate request to temporarily add a composing Japanese input source.
Ticket 01 remains claimed, and 02/03 remain blocked pending acceptance.

Final gate: `scripts/verify --full` exit 0, 35 tests, fmt/Clippy and Debug/Release. Main-checkout bundle build passed after byte-verifying transferred source against the tested candidate. [Original gate log](2026-09-21-terra-full-gate.log).
