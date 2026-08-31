Type: task
Status: resolved
Assignee: Codex
Blocked by: 09

## What to build

Implement Timestamps as a local conversion Utility for Unix seconds, Unix milliseconds, ISO 8601, UTC, and local time. The owner sees what representation was detected, chooses the output timezone explicitly, and inserts the current instant only through a Now action.

Follow the [remaining Utilities implementation specification](../remaining-utilities-implementation-spec.md), [Define first-release Utility contracts](02-define-first-release-utility-contracts.md), and the [implementation handoff](../implementation-handoff.md).

## Acceptance criteria

- [x] The stable Utility Definition appears under Format & Convert with History enabled by default and date/time/epoch/Unix search aliases.
- [x] Auto and manual modes implement the approved digit-count, offset/timezone, fractional-second, ambiguity, and daylight-saving rules; auto-detection always displays its inference and never guesses.
- [x] Results show the same instant as locale-stable ISO 8601, explicit UTC, and explicit local/named-timezone values. The chosen timezone name or identifier is always visible.
- [x] Now is an explicit action backed by an injectable clock. Merely opening the Utility or changing unrelated controls never silently replaces the input with the current time.
- [x] Negative epochs, fractional seconds, boundary magnitudes, daylight-saving transitions, and invalid local times have documented, tested behavior.
- [x] Empty input is neutral; stale asynchronous/debounced results never publish. Paste, Copy Result, Clear, timezone, and mode controls are explicit and keyboard accessible.
- [x] A completed valid conversion emits one versioned snapshot. Preview and explicit restore reproduce the captured instant, detected representation, controls, and rendered result without rerecording or substituting a new Now value.
- [x] Tests use fixed clocks, fixed timezone databases/identifiers, POSIX-stable locales, known epoch vectors, Unicode-independent formatting, snapshot round trips, and registry discovery.
- [x] `scripts/verify --full` passes and evidence distinguishes the current verified host from pending runtime checks on other macOS versions.

## Answer

The independent vertical slice is implemented under `SofDevTool/Utilities/Timestamps` with Auto and four explicit input modes, locale-stable rendering, explicit named-timezone output, strict ambiguity rules, DST gap/fold rejection, injectable-clock Now, debounced revision gating, explicit Paste/Copy/Clear controls, and schema-version-1 restore of the captured instant without substituting a later clock value or rerecording.

The separate contract suite passed known epoch and negative/fractional vectors, Auto digit boundaries, offset requirements, integer-only milliseconds, non-exponential seconds, Europe/Rome DST gap and fold fixtures, locale-stable output, fixed-clock Now, snapshot round trip, and stale revision rejection. The combined focused URL Encoding, Hashes, and Timestamps test command passed on Apple Silicon macOS 26.2 (25C56), Xcode 26.6 (17F113), Apple Swift 6.3.3. Shared Registry/search and Utility-owned History preview integration are complete. The authoritative `scripts/verify --full` gate passed 84 unit/contract tests and 3 UI tests on that host. macOS 14 and 15 runtime verification remains pending on those hosts.
