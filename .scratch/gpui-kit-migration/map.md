# GPUI Kit migration

**Status:** ready-for-agent
**Type:** task

Product scope, testing seams and ticket breakdown are approved. [Specification](spec.md) and ten individual tickets are published as ready-for-agent. No source implementation has started. Ready status does not override blocking dependencies.

## Approved testing seams

Primary: existing application interactions from input/generation through results, Utility History and restore. Reuse existing domain contract tests. Supplement with preference/storage checks and direct native acceptance for focus, keyboard, Clipboard, Launcher and WebView behavior. No new general testing layer.

## Tickets

| Ticket | Blocked by | Delivers |
| --- | --- | --- |
| [01 — Workbench and official Catppuccin themes](issues/01-workbench-themes.md) | None | Open the Developer Toolbox with kit-owned Workbench navigation and theme management, preserve Utility organization and sessions, and choose System, Light or Dark appearance across restarts. |
| [02 — Utility Launcher with kit controls](issues/02-utility-launcher.md) | 01 | Search, navigate and open Utilities with standard kit controls while preserving Launcher shortcuts and application/window activation. |
| [03 — Settings and Utility History confirmations](issues/03-settings-history.md) | 01 | Configure and inspect Utility History and perform destructive actions through standard accessible confirmation dialogs. |
| [04 — JSON, YAML/JSON and JWT workspaces](issues/04-structured-text.md) | 01 | Use kit text controls to inspect, convert and format structured input while retaining validation, results and History restoration. |
| [05 — Encoding, hashing and text transformation workspaces](issues/05-text-transforms.md) | 01 | Use Base64, URL encoding, hashes, case conversion and whitespace conversion through standard kit controls with unchanged results and explicit Clipboard actions. |
| [06 — Identifier, random string, sample data and timestamp workspaces](issues/06-generators-time.md) | 01 | Generate and inspect identifiers, random strings and sample data, and convert timestamps using kit controls without changing domain behavior. |
| [07 — Regex and Color Conversion workspaces](issues/07-regex-color.md) | 01 | Inspect regex matches and convert colors using standard kit controls while retaining specialized domain result presentation. |
| [08 — Text Diff with kit controls and synchronized appearance](issues/08-text-diff.md) | 01 | Compare text with the existing specialized renderer while all surrounding controls and effective appearance follow GPUI Kit. |
| [09 — Retire sofui and integrate the complete replacement](issues/09-retire-sofui.md) | 01, 02, 03, 04, 05, 06, 07, 08 | Produce the single complete Developer Toolbox replacement with no remaining sofui product, custom shared controls or competing theme system. |
| [10 — Native acceptance of the complete migration](issues/10-native-acceptance.md) | 09 | Verify the complete migrated application in isolated native sessions and provide an honest completion record for the owner. |

## Integration semantics

01 establishes the consumer/theme conventions. 02–08 depend only on 01 and have no artificial ordering among themselves. These edges express prerequisites, not authorization for simultaneous writers or separate releases. 09 waits for all migrated surfaces and retires the old product. 10 validates the integrated application after the final gate. No shared custom abstraction is prefactored into existence; any necessary local cleanup stays within its owning work package.

## Decisions-so-far

- Owner approved the complete migration, testing seams and ten-ticket breakdown in this conversation on 2026-09-26.
- Deliver one complete replacement; no intermediate product acceptance or releases.
- Use official Catppuccin Latte/Frappe through the kit with System/Light/Dark preferences.
- Remove sofui; retain app compositions and the specialized theme-aware Text Diff renderer.
- Report upstream incompatibilities rather than introducing silent exceptions.

## Current frontier

01–09 are implemented in the working tree; the complete integrated tree passes
`make verify-full` on macOS 26.2 arm64 (see the
[native acceptance record](issues/10-native-acceptance-record.md)). 10 remains
open: native acceptance on macOS 14/15 has **not** been performed, and the
whole-grapheme deletion finding is awaiting an owner decision.
