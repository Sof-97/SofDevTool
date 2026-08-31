# Regex execution bounds on Swift/macOS

Researched 2026-08-30 from Apple and ICU primary documentation, the current local macOS SDK, and the current SofDevTool project shape.

## Recommendation

Implement the Regex Utility with a small, replaceable **in-process ICU C backend**, not directly with `NSRegularExpression` and not with a helper process.

For every execution, the backend should:

1. reject pattern, input, replacement, match-count, capture-count, and replacement-output sizes above explicit product limits before unbounded allocation;
2. create an operation-local `URegularExpression` and set a nonzero `uregex_setTimeLimit` engine-step budget plus an explicit nonzero `uregex_setStackLimit` heap-backtracking budget;
3. install both `uregex_setMatchCallback` and `uregex_setFindProgressCallback`, backed by an operation-owned cancellation/deadline context;
4. run on a bounded serial worker away from `MainActor`, coalescing or cancelling superseded requests instead of accumulating detached work;
5. map `U_REGEX_TIME_OUT`, `U_REGEX_STACK_OVERFLOW`, and `U_REGEX_STOPPED_BY_CALLER` to distinct app-owned outcomes, and allow only the current input revision to publish; and
6. enumerate matches and build replacement preview incrementally so result-count and output-size caps can stop the operation before collecting an unbounded array or string.

ICU documents exponential behavior for some backtracking patterns and explicitly recommends a time limit for user-supplied expressions. Its time limit is a count of engine processing steps, not a strict duration: elapsed time depends on CPU and pattern. The default is unlimited. [`uregex_setTimeLimit`](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/uregex_8h.html#details), [ICU performance guidance](https://unicode-org.github.io/icu/userguide/strings/regexp.html#performance-tips)

This design gives the app a real, deterministic **work budget** and keeps the UI responsive. It does not claim a mathematically exact wall-clock cutoff. That stronger guarantee would require process isolation, discussed below.

## Why `NSRegularExpression` alone is insufficient

Apple's most general API, `enumerateMatches(in:options:range:using:)`, supports `.reportProgress`. During a long-running operation Foundation periodically invokes the block with a progress flag, and the block may set `stop` to end processing. This is useful for cooperative cancellation and is materially better than racing an uncancellable synchronous call against `Task.sleep`. [Apple `enumerateMatches` documentation](https://developer.apple.com/documentation/foundation/nsregularexpression/enumeratematches(in:options:range:using:))

However, the public `NSRegularExpression` surface does not expose a numeric engine-step limit or a backtracking-memory limit. Its progress callback has no documented invocation interval. Therefore it can support cancellation and a callback-checked deadline, but it cannot express or test the explicit ICU work and heap budgets required by this Utility.

Simply moving `matches(in:)` or `firstMatch(in:)` into a Swift `Task` is not an execution bound. Cancelling a task is cooperative; a synchronous regex call that never checks cancellation can continue consuming a worker after the UI has discarded it. Racing that call against a timeout only bounds how long the caller waits, not how long the regex runs, and repeated edits could accumulate stale work.

Foundation remains useful as behavioral context: Apple documents ICU syntax for its regular-expression searches, and Swift's accepted Regex proposal describes `NSRegularExpression` as an API around ICU. [Apple lower-level regex documentation](https://developer.apple.com/library/archive/documentation/StringsTextFonts/Conceptual/TextAndWebiPhoneOS/LowerLevelText-HandlingTechnologies/LowerLevelText-HandlingTechnologies.html#//apple_ref/doc/uid/TP40009542-CH4-SW17), [Swift Evolution SE-0350](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0350-regex-type-overview.md#foundation-integration)

## What the ICU controls actually guarantee

### Engine work

`uregex_setTimeLimit` fails a match after a configured number of engine steps. ICU marks the API stable since ICU 4.0. Zero means unlimited, and unlimited is the default. Step counts are suitable for a deterministic policy and deterministic low-limit tests, but must not be labelled as milliseconds. [`uregex_setTimeLimit`](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/uregex_8h.html#details)

### Backtracking memory

`uregex_setStackLimit` limits bytes used by ICU's **heap** backtracking stack and reports overflow through the match error. ICU says a limit is enabled by default; its user guide identifies the current default as 8 MB. Set the value explicitly so SofDevTool's behavior is not an undocumented consequence of the ICU version shipped by a particular macOS release. [`uregex_setStackLimit`](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/uregex_8h.html#details), [ICU heap and stack guidance](https://unicode-org.github.io/icu/userguide/strings/regexp.html#heap-and-stack-usage)

This limit does not cap every allocation the Utility can make. Pattern/input ceilings, maximum matches/captures, and maximum replacement-preview length remain necessary app-level policy.

### Cancellation and elapsed deadline

`URegexMatchCallback` is invoked periodically inside matching and receives accumulated engine steps. Returning false terminates the match. `URegexFindProgressCallback` covers `find`, `findNext`, `split`, and some replacement operations while the engine scans candidate starting positions; ICU explicitly says it is not guaranteed to run at every character. A full all-matches/replacement operation should install both callbacks. [ICU callback contracts](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/uregex_8h.html#details)

Both callbacks should consult only an operation context: an atomic cancellation flag and a monotonic deadline. ICU says callbacks must not call other functions on the same `URegularExpression`. Callback refusal is reported as `U_REGEX_STOPPED_BY_CALLER`; time and heap limits have separate `U_REGEX_TIME_OUT` and `U_REGEX_STACK_OVERFLOW` errors. [ICU error codes](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/utypes_8h.html)

The callback deadline is a secondary wall-time guard, not an absolute real-time promise: termination latency includes ICU's undocumented interval between callback points. The nonzero step budget is the primary work guarantee.

## Fit with the current app architecture

The direct ICU design preserves the handoff's exactly-three-target shape:

- keep `import ICU` and every `URegularExpression` detail inside one `ICURegexBackend` implementation in `SofDevTool/Utilities/Regex`;
- expose only app-owned, strongly typed pattern/options/input/result/error values to the Regex domain and workspace;
- inject a `RegexExecutionPolicy` and a narrow backend seam for deterministic tests;
- keep each ICU matcher operation-local or serially confined; never share one mutable matcher concurrently; and
- preserve the existing session-owned debounce/cancellation/revision rule, with at most one active backend operation per Regex session.

This adds no Swift Package Manager dependency, runtime download, network use, or new build target. It uses the system ICU library already exposed by the macOS SDK. RE2 would offer linear-time matching, but its own documentation says it omits backreferences and look-around; adopting it would change the promised visible ICU-compatible dialect rather than merely make the implementation safer. [RE2 README](https://github.com/google/re2/blob/main/README.md)

PCRE2 also has resource controls, but it is a different regex flavor and would add a bundled dependency. It offers no product advantage here over the platform ICU API that already supplies the needed controls. [PCRE2 limits](https://www.pcre.org/current/doc/html/pcre2limits.html)

### Local SDK evidence

On the current Xcode installation, `xcrun --sdk macosx --show-sdk-path` resolves to `MacOSX26.5.sdk`. That SDK:

- declares the system Swift module `ICU` in `usr/include/unicode.modulemap` and links it to `icucore`;
- declares `uregex_setTimeLimit`, `uregex_setStackLimit`, `uregex_setMatchCallback`, and `uregex_setFindProgressCallback` in `usr/include/unicode/uregex.h`;
- exports those symbols from `usr/lib/libicucore.A.tbd`; and
- successfully type-checks a Swift file importing `ICU` and referencing all four APIs for `-target arm64-apple-macos14.0`.

The project file currently contains exactly the app, Swift Testing, and UI-test native targets and sets `MACOSX_DEPLOYMENT_TARGET = 14.0`. These are **build-SDK and project-shape facts**, not runtime proof on macOS 14 or 15. The ICU APIs are marked stable since ICU 4.0/4.6, and Apple has long documented system `uregex.h` availability, but the final evidence must still include an actual macOS 14 runtime check. [Apple system ICU documentation](https://developer.apple.com/library/archive/documentation/StringsTextFonts/Conceptual/TextAndWebiPhoneOS/LowerLevelText-HandlingTechnologies/LowerLevelText-HandlingTechnologies.html#//apple_ref/doc/uid/TP40009542-CH4-SW17)

## Why not a helper process

A separate process is the only robust way to enforce an absolute wall-clock cutoff against a native engine that stops responding to cooperative callbacks: the parent can terminate the worker. It also contains a native crash or memory corruption.

That isolation has a real architectural price. Apple's supported XPC-service approach starts a separate service executable and instructs developers to add an XPC Service target. Apple's embedded command-line helper guidance likewise starts by creating a command-line-tool target. Either would violate SofDevTool's current exactly-three-target contract and add IPC, lifecycle, signing, packaging, and test surfaces. [Apple XPC service guidance](https://developer.apple.com/documentation/xpc/creating-xpc-services), [Apple embedded helper guidance](https://developer.apple.com/documentation/xcode/embedding-a-helper-tool-in-a-sandboxed-app)

Do not disguise the app executable as its own hidden helper or depend on an arbitrary system command. That would avoid the target count only cosmetically while introducing fragile lifecycle and protocol behavior.

Revisit process isolation only if the product requirement changes from “pathological input cannot freeze the app” to an absolute deadline or containment against engine crashes. That change should explicitly reopen the three-target architecture decision.

## Resource policy to settle in the specification

The architecture can be decided now; production values still need a short performance prototype because ICU step units are deliberately not wall-clock units. Record all of these as named, injected constants:

- maximum pattern UTF-16 length;
- maximum test-text UTF-16 length;
- maximum replacement-template UTF-16 length;
- ICU engine-step limit per matching operation;
- ICU heap-backtracking limit in bytes;
- monotonic elapsed deadline for the complete user operation;
- maximum returned matches and total capture ranges; and
- maximum replacement-preview UTF-16 length.

Choose the production values by running representative valid patterns and pathological failing patterns on the slowest supported Mac available. Do not assert that one ICU step equals one millisecond or write a test that depends on that approximation. The UI should identify a resource-limit result as such and preserve the user's inputs; it must not present a partial match/replacement set as a completed valid Utility Operation.

## Verification plan

1. **Build boundary:** keep an `ICURegexBackend`-only import test and run the ordinary project gates. A deployment-target build is not a macOS 14 runtime check.
2. **Deterministic step bound:** inject a very small nonzero step limit and a known catastrophic failing fixture such as a nested-quantifier pattern. Assert the app-owned timeout outcome, no snapshot, and no late publication. Do not assert an exact duration.
3. **Heap bound:** inject a deliberately small backtracking limit and assert the distinct stack-overflow outcome.
4. **Cancellation callbacks:** inject a callback policy that flips to stop after a known callback count. Cover both expensive matching and long find scanning, and assert `U_REGEX_STOPPED_BY_CALLER` mapping.
5. **Operation caps:** independently test pattern/input/replacement, match/capture, and preview-output ceilings, including zero-length matches and output-expanding replacements.
6. **Revision race:** use a controllable fake backend to complete an older request after a newer request and prove only the latest revision can update result or History.
7. **Responsiveness:** in the focused UI suite, start a pathological operation and prove a separate control remains usable using predicate waits, not sleeps; then prove cancel or supersession prevents stale output.
8. **Compatibility:** run ICU syntax/capture/replacement/Unicode fixtures on the currently verified host, then repeat the runtime suite on actual macOS 14 and 15 hosts before claiming support there.

## Decision summary

- **Use:** direct system ICU C API behind an app-owned Regex backend seam.
- **Bound:** engine steps, ICU heap backtracking, total input/output size, result cardinality, cooperative cancellation, and an elapsed deadline.
- **Schedule:** off `MainActor`, one bounded/coalescing worker per session, revision-gated publication.
- **Avoid:** timeout races around uncancellable Foundation calls, unbounded detached tasks, alternate regex flavors, and a helper target under the current architecture.
- **Evidence boundary:** current SDK compile compatibility is confirmed; production numeric limits and macOS 14/15 runtime behavior remain to be verified.
