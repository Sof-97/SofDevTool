Type: task
Status: resolved
Blocked by: 09

## What to build

Implement Case Conversion as a local Unicode-preserving Utility. The owner enters text, can see how it was segmented into words, and converts it explicitly to each agreed case style without hidden normalization.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Format & Convert with History enabled by default and aliases for every supported case family.
- [x] camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, Title Case, sentence case, lowercase, and uppercase are distinct visible choices.
- [x] Detected Words is inspectable before or alongside conversion and uses the approved deterministic acronym/digit/Unicode segmentation policy for every output style.
- [x] Conversion preserves valid Unicode text and grapheme clusters, including accented characters, combining marks, non-Latin scripts, and complex emoji; locale-sensitive casing follows one documented stable policy.
- [x] Empty input is neutral. Input changes are debounced where appropriate, invalid/stale states never retain an unrelated result, and large input is not silently truncated.
- [x] Paste, Copy Result, Clear, and case selection are explicit and keyboard accessible. One settled valid conversion emits at most one versioned History snapshot.
- [x] Preview and explicit restore reproduce segmentation, selected case, input, and result without rerunning or rerecording.
- [x] Tests use a hand-reviewed segmentation matrix covering acronyms, digits, punctuation, existing case styles, whitespace, accented/combining Unicode, non-Latin text, and emoji, plus snapshot, stale-revision, and registry tests.
- [x] `scripts/verify --full` passes and the implementation evidence names the verified host.


## Answer

The Utility-owned vertical slice is implemented with all nine case styles, one shared inspectable segmentation result, explicit acronym/digit/punctuation/mixed-separator behavior, complete-`Character` emoji handling, deterministic `en_US_POSIX` casing, a named 1,048,576 UTF-16-unit input cap, revision-gated live conversion, explicit clipboard actions, and versioned restore without rerunning or rerecording. The hand-reviewed matrix covers `HTTPServer`, `version2Value`, mixed separators, `XMLHttpRequest`, precomposed and combining accents, non-Latin text, and complex emoji. Focused JWT, Case Conversion, and Regex tests pass on Apple Silicon macOS 26.2 with Xcode 26.6 / Swift 6.3.3.

Shared Registry and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. macOS 14 and 15 runtime verification remains pending.
