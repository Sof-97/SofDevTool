Type: task
Status: resolved
Blocked by: 09

## What to build

Implement Whitespace Conversion as a local Utility whose transformations remain separate and explicit. The owner chooses exactly one operation at a time and can understand its parameters before changing text.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Format & Convert with History enabled by default and useful whitespace/trim/indent/line-ending aliases.
- [x] Edge trim, per-line trim, horizontal-whitespace collapse, all-whitespace collapse, line-ending normalization, tabs-to-spaces, spaces-to-tabs, blank-line removal, and dedent are separate named actions; there is no generic Clean action combining them.
- [x] Line-ending normalization exposes LF/CRLF/CR with LF as default; tab width defaults to four within one through eight; Tabs to Spaces uses tab stops and Spaces to Tabs changes leading indentation only.
- [x] Dedent uses the common removable indentation of non-blank lines, preserves relative indentation, and has explicit behavior for blank-only and mixed tab/space input.
- [x] Transformations preserve Unicode scalar/grapheme content and make whether trailing final newlines and whitespace-only lines are retained or removed explicit and testable.
- [x] Empty input is neutral. Paste, Copy Result, Clear, operation selection, and operation-specific controls are explicit and keyboard accessible; stale output is never presented as current.
- [x] Each deliberate completed transformation emits one versioned snapshot, including when output happens to equal input. Preview and explicit restore do not rerun or rerecord it.
- [x] Tests provide a hand-reviewed matrix for CRLF/LF/CR, tabs and spaces, non-breaking and Unicode whitespace, blank lines, trailing newlines, mixed indentation, no-op deliberate operations, snapshots, and registry search.
- [x] `scripts/verify --full` passes and the implementation evidence names the verified host.

## Answer

Implemented the Utility-owned engine, explicit-action workspace, versioned snapshot/restore path, and focused contract tests. The hand-reviewed policy preserves original line separators and final-newline boundaries except when the selected action explicitly targets them; tab-aware operations use visual tab stops, and Dedent expands a partially removed tab to the remaining spaces so relative indentation stays stable.

Focused evidence on macOS 26.2: the Whitespace Conversion suite passed 5 tests through the Xcode test target on 2026-08-31. Shared Registry and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. macOS 14 and 15 runtime verification remains pending.
