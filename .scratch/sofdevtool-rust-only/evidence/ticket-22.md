# Ticket 22 — final Release acceptance

This record separates the latest installed Release's fifteen-Utility representative smoke, the owner's subsequent physical Launcher/Spaces/Dock checks, the earlier corrected Text Diff run, and detailed native observations of a preceding candidate. Owner-reported manual passes are identified separately from Computer Use observations. The source correction commits include `df95bd39b281e4d7e44375ef8c7543417b01490e` for the two Text Diff review findings and `33d68b3` for the Carbon pressed-event fix, followed by packaging documentation commit `dcc6f0b`. The latest packaged revision is GitButler's synthetic `HEAD` `b4e17030ffa8e37b51bb661befe2e6728a044b9b`, which must not be confused with a branch commit. The earlier corrected installed Release used synthetic revision `b73c2a7b8937a8f58c820fa538beb487d8dac9de`. The still earlier `d9523ebebca78fc79774ad24cfacde8f43a51ac4` Release predates the Text Diff corrections; its more detailed native Utility walkthrough remains useful coverage but cannot by itself certify the latest binary.

## Final-source automated gate

On macOS 26.2 (25C56), arm64, with pinned Rust 1.98.1, `make verify-full` passed from the repository root with Cargo using `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target` and `CARGO_NET_OFFLINE=true`: [latest full log](../../../.artifacts/parallel-rust-only/final/verify-full-r4.log). The gate checked rustfmt, first-party Clippy with warnings denied, 339 Rust tests, five Python packaging/install tests, Debug and Release application/gallery builds, a copied-out sofui gallery and independent consumer, reproducible Text Diff assets/dependency notices after `npm ci`, and Debug/Release bundle packaging. An additional direct-Cargo identity test passed. The gate's packaged bundles reported app version 0.2.0, build 1, the appropriate `debug`/`release` channel, synthetic revision `b4e17030ffa8e37b51bb661befe2e6728a044b9b`, and `source clean`. The transitive `block 0.1.6` future-incompatibility notice did not fail the gate; first-party Clippy had no warnings.

## Latest installed Release: native r4 observations and limits

The coordinator ran default `make install` again; the [r4 install log](../../../.artifacts/parallel-rust-only/final/install-r4.log) identifies `/Users/gerardo/Applications/SofDevTool.app` as Release 0.2.0 build 1, revision `b4e17030ffa8e37b51bb661befe2e6728a044b9b`, `source clean`. The [native r4 record](../../../.artifacts/parallel-rust-only/final/native-r4.json) records executable SHA-256 `24d2645b47021748226be8f3919e9fe1bc062730f92b710a7955fbcfe5e2a905`, PID 6591, and isolated profile `/private/tmp/sofdevtool-final-profile-0jm89bq9`; the app ran from `/tmp` outside the checkout. The installed binary displayed the b4 clean Release identity. JSON showed three persisted synthetic History entries. At 09:20 UTC, selecting the historical 08:37:17 `caffè` query and confirming Restore recovered exact input `{"items":[{"name":"one"},{"name":"caffè"}]}`, query `items[1].name`, and result `caffè`; the app was left on that state for the owner's manual checks.

From 09:13–09:19 UTC, the coordinator exercised one representative flow in **all fifteen Utilities on this same installed r4 binary**, observing the accessibility state and rendered screen. These are final-binary smoke observations, not a claim that every mode, limit or invalid-input variant in the earlier detailed matrix was rerun:

| Utility | Representative installed-r4 observation |
| --- | --- |
| JSON | `{"final":true}` formatted as pretty JSON. |
| Base64 | `café` encoded to `Y2Fmw6k=`. |
| URL Encoding | Path Segment `a/b` encoded to `a%2Fb`. |
| Hashes | SHA-256 of `abc` showed the standard `ba7816…15ad` digest. |
| Color Conversion | `#102030` RGB Copy yielded `rgb(16 32 48)`. |
| Timestamps | `1700000000` yielded `2023-11-14T22:13:20Z`. |
| YAML / JSON | Legacy YAML `yes` remained a string. |
| JWT Decoder | The synthetic token showed `sub=fixture`, with recording off. |
| Case Conversion | `HTTPServer` became `httpServer` in the selected style. |
| Whitespace | Edge Trim of `  café 👩🏽‍💻  ` yielded `café 👩🏽‍💻`, retaining the interior space. |
| Regex | `\w+` with replacement `[$0]` on `café` produced `[café]`. |
| Random String | Saved Custom `KLMN567xyz`, Count 2 and Length 21 produced two values after Generate. Copy All pasted into Whitespace as **two LF-separated lines**, exactly `yz5Lx66My6Ny5KKN6NyL7\nN65yN7zN77Kx767LMNzLx`; this resolves the earlier candidate's newline-paste uncertainty for this r4 operation. |
| Identifier Generator | UUID v4 Generate produced `2be361ec-f079-4d5f-b606-7a756f20963a`; Copy All pasted into Inspect/Validate and returned the same value, with History recording observed. |
| Sample Data | JSON Generate showed fictional records with `name`, `email@example.invalid` and `id`, with History recording observed. |
| Text Diff | Both editors opened empty despite six prior History entries. Entering Original `A\ncafé` and Updated `B\ncafé` showed a bounded Split comparison with `-1`/`+1` and unchanged `café`. Two actions spaced over 200 ms produced two new settled History rows. |

The earlier candidate's detailed invalid-input, restore and configuration checks remain documented below. The r4 smoke establishes a representative path through every Utility on the corrected installed binary; it does not imply exhaustive native coverage of all modes.

The Launcher button opened the native panel and Escape closed it. Cmd-W left PID 6591 running; CUA's `getApp` reopen returned the same process with the restored JSON query state. That observation establishes state in the same live process, **not** return through a literal Dock click. Direct system Dock automation timed out with CUA error 10005. A CUA-synthesized Ctrl-Alt-Space still did not open the Launcher after the Carbon pressed-event correction. This synthetic failure alone cannot establish how the physical chord is delivered. The owner's later manual result is recorded separately below.

## Owner-reported physical checks and clean local branch

The coordinator left the same installed r4 Release open outside the checkout on
the isolated profile `/private/tmp/sofdevtool-final-profile-0jm89bq9`, with the
synthetic JSON `items[1].name` query and `caffè` result restored. The latest
handoff process was PID 14600. The coordinator reverified bundle identity
`com.gerardocalia.sofdevtool`, version 0.2.0 build 1, clean packaged revision
`b4e17030ffa8e37b51bb661befe2e6728a044b9b` and the same executable
SHA-256 `24d2645b47021748226be8f3919e9fe1bc062730f92b710a7955fbcfe5e2a905`.
The owner was asked to try three physical checks: Control-Option-Space from
another app; interaction from another Space/full-screen context; and Cmd-W
followed by a **literal Dock click** returning to the retained JSON state.
The owner replied “Si sembra funzionare tutto.” This is an **owner-reported
pass of the requested checks**, not an independently captured CUA trace of
those actions. Earlier CUA synthetic-key failure and Dock timeout remain
accurate observations about that automation path.

Before this documentation closeout, `feat/rust-gpui-migration` was at
`89fd00793e366578588473b1f23eee37a7dc9eeb`; its base ancestry and tree
equivalence had passed, and all 272 tracked worktree files matched current
`HEAD` in bytes and mode. The apparent 335 GitButler uncommitted changes came
from an index still resembling the retired pre-root workspace, not live edits:
the 264 non-ignored paths reported untracked were already committed in
`HEAD`; genuine non-ignored untracked files numbered zero. With explicit user
authorization, the coordinator made an index backup at
`/private/tmp/sofdevtool-index-backup-i3_lce4r/index`, used the documented
`but teardown`/snapshot route, then the narrowly scoped raw-Git exception
`git restore --staged --source=HEAD -- .` when teardown alone left the stale
index; `but setup` restored managed mode. Afterward Git porcelain status was
empty, GitButler reported zero uncommitted changes, and the branch remained
local and unpublished. Ignored personal files were preserved; no product file
was changed by this reconciliation. This proves the branch state at
`89fd007` before these documentation-only closeout edits. The coordinator
owns committing the closeout and repeating the final clean-branch check; no
future commit hash is asserted here.

## Earlier corrected Text Diff Release: actual native observations

The coordinator then ran the default `make install` destination. The [corrected install log](../../../.artifacts/parallel-rust-only/final/install-r3.log) identifies `/Users/gerardo/Applications/SofDevTool.app` as Release 0.2.0 build 1, revision `b73c2a7b8937a8f58c820fa538beb487d8dac9de`, `source clean`. Executable SHA-256 is `80c22eeddd7156919a82297508fe9e1b638dec843471cca828bdc4b49cc3b16b`. The app was launched outside the checkout with the same isolated test profile `/private/tmp/sofdevtool-final-profile-0jm89bq9`; the [native launch record](../../../.artifacts/parallel-rust-only/final/native-r3.json) identifies PID 99426. No personal History or preferences were used.

- After restarting this corrected installed binary, JSON History still contained three synthetic entries. Selecting the prior `caffè` query entry and restoring from an empty session recovered the exact JSON input, `items[1].name` query, and `caffè` result without another History entry.
- Random String's four disabled presets, Length 21, Count 2 and Custom `KLMN567xyz` survived relaunch; its two generated batches remained in History. The selected Frappé theme also survived.
- A fresh Text Diff opened with both editors empty and no History. Entering `let total = 1` versus `let total = 2` with the same Unicode `café 👩🏽‍💻` line produced one settled History entry. Unified rendered the bounded comparison with the disclosed Unicode fallback; Split also worked. In the actual WKWebView comparison, double-clicking `café`, pressing Cmd-C, then pasting into the GPUI Updated editor produced exactly `café`. Cmd-Z restored the original Updated fixture. This corrected native check resolves the earlier Debug run's inconclusive synthetic Cmd-C attempts. See [ticket 18](ticket-18.md) for the source-level empty-start and settlement regression checks.
- With 100 synthetic Unicode lines in each editor (`line N: café 👩🏽‍💻 original` / `updated`), an actual window-edge CUA drag resized the window from 3072×2168 to 2244×1504. Screenshots showed the WebView bounded below the Comparison header and above the Unicode warning, without overlapping History or the sidebar. Ten scroll pages inside the comparison reached lines 93–100 while the editors and surrounding chrome stayed fixed. This is native resize, clipping and comparison scrolling evidence on the corrected Release.
- From that 100-line Split comparison, the coordinator selected the earlier two-line comparison in History. Restore selected presented Restore/Cancel; confirming restored exact Original `let total = 1` and Updated `let total = 2` fixture text, including the Unicode line. History remained at six entries, so restore added no operation.

These observations prove installed Release launch, resource-backed Text Diff rendering, native WebView selection/key Copy, and isolated profile persistence on this host. They do not imply every prior Utility row below was rerun on the corrected binary.

## Installed preceding candidate: actual native observations

The coordinator used the default `~/Applications/SofDevTool.app` installation, whose [install log](../../../.artifacts/parallel-rust-only/final/install.log) reports Release 0.2.0 build 1 and revision `d9523ebebca78fc79774ad24cfacde8f43a51ac4`. Executable SHA-256 was `4ac34f321bf7e4bd5595fe3f783aeedcfdf600e3bd365a3c9de64a892645696d`. The app ran outside the checkout with working directory `/private/tmp`, isolated `SOFDEVTOOL_RUST_SUPPORT_ROOT=/private/tmp/sofdevtool-final-profile-0jm89bq9`, and PID 96250. These are synthetic test inputs; personal History and preferences were not inspected. The coordinator later used explicit Cmd-Q and confirmed that PID exited. Native observations below came from this **preceding** candidate on macOS 26.2 arm64.

| Utility | Observed in the installed preceding candidate | Still unverified in this native run |
| --- | --- | --- |
| JSON | Formatted the synthetic Unicode fixture; query `items[1].name` produced and copied `caffè`, which pasted exactly into Base64. | Minify and malformed-input path. |
| Text Diff | This preceding candidate's detailed run was deferred; earlier Debug WebView evidence is in [ticket 18](ticket-18.md). The corrected Release's separate Text Diff checks, including History restore, are above. | The preceding candidate did not receive its own detailed Text Diff run. |
| Base64 | `café` encoded and copied as `Y2Fmw6k=`; decoding returned `café`. Invalid `-_8=` cleared output, added no History entry, and disabled Copy without changing the known Clipboard sentinel. | URL-safe padded/unpadded emoji case. |
| URL Encoding | Path Segment `a/b?c#d[e]` became `a%2Fb%3Fc%23d%5Be%5D`; Query Value `a+b` became `a%2Bb`; `%C3%A9` decoded to `é`. Invalid `%` cleared output and added no History entry. | Exact Clipboard preservation on the invalid `%` case. |
| Case Conversion | `HTTPServer` became `http_server`; decomposed `Cafe\u0301Bar` became `cafe\u0301_bar` with the combining mark retained. | The other seven style choices in native UI. |
| Whitespace | Tabs-to-spaces at width four yielded `a   b\n    👩🏽‍💻` for the synthetic tab/emoji fixture. | LF normalization and stepper boundary interaction. |
| Hashes | Explicit empty-input SHA-256 gave the standard `e3b0c442…b855` digest; `abc` with SHA-256/Base64 gave `ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=`. | Native SHA-1 legacy warning. |
| Random String | Four presets were off, Custom was `KLMN567xyz`, Length 21 and Count 2. Two explicit Generates each produced two values and two History operations; Copy All contained the current values. | The paste route filtered newlines, so exact LF separation was **not** established there. |
| Color Conversion | Two R increments changed `#102030` to `#1a2030`; RGB Copy pasted exactly `rgb(26 32 48)`. Invalid `rgb(1, 2)` removed Copy buttons and added no History entry. | Native channel bounds and all representation/History restore cases. |
| JWT Decoder | Synthetic HS256 token displayed `sub=fixture`; recording remained off and History empty. | Invalid-token and native Copy Header path. |
| Identifier Generator | UUID v4 and v7 output shapes, two monotonic ULID batches from deliberate Generates, and restore confirmation were observed. Cancel preserved the second batch and its copied multiline output; Confirm restored the first without a History delta. | The restored first batch was not independently pasted for exact-output comparison; validation mode remains untested here. |
| Regex | Named captures for `Doe, John` and replacement preview `John Doe` appeared. Unsupported look-behind produced an invalid state with no output or History delta. | Native demanding long-running responsiveness/limits on this Release candidate; controlled automated checks cover them separately. |
| Timestamps | `1700000000` yielded `2023-11-14T22:13:20Z`. Eleven-digit Auto input showed ambiguity with empty result. Europe/Rome `2024-03-31T02:30:00` showed the nonexistent-DST diagnostic, disabled Copy and added no History entry. | Exact complete Copy text and other zone/mode combinations. |
| Sample Data | Defaults showed fictional JSON `name`/`email`/`id` fields. CSV header rendered quoted labels (`"name","email","id"`); the runbook's unquoted expectation was incorrect, not a product defect. Blank name produced no output or History delta; restore prompted, and Cancel preserved the invalid blank-name state. | Exact CSV/JSON bytes and Confirm restore on this run. |
| YAML / JSON | `enabled: true`, legacy `yes`, and `message: café` preserved Boolean/string types. Reverse conversion produced `enabled: true\nmessage: café\n`; duplicate `a` key gave an empty invalid result. | Exact Copy bytes in both modes. |

The coordinator also switched to Frappé and observed a separate Settings window in that theme, with global History enabled and JWT recording off. Cmd-W closed Settings and returned to Workbench. Ctrl-Cmd-F entered fullscreen (AX window controls disappeared), the Launcher button opened a native Frappé panel there, searching `json` and pressing Return returned to JSON with its earlier input, query and `caffè` result intact, and Ctrl-Cmd-F exited fullscreen (controls returned). A synthesized Ctrl-Alt-Space did not open the Launcher; this does not prove whether a physical global shortcut fails. Explicit Cmd-Q terminated PID 96250. Other Spaces/Dock interactions were not exercised in that run. Relaunch persistence and renderer runtime were subsequently checked on the corrected Release above.

## Acceptance limits

The owner's report closes the three requested physical Launcher/Spaces/Dock
checks on this installed r4 binary; it does not turn them into coordinator
screen-captured observations. Some native catalog subcases listed above were
not rerun on r4, but the final-binary representative pass, independent core
contracts and full gate cover the agreed acceptance scope. The accepted macOS
26.2 arm64 host does not establish macOS 14/15 runtime, other DPI/display
configurations or all IME behavior; the specified IME exception remains.
Full VoiceOver certification is outside scope. No personal History was read.
The independent [final review](final-review.md) reports source findings and
their resolution separately from native evidence. No push, pull request,
merge or sofui publication occurred.
