# Ticket 20 — root workspace, distinct bundles and isolated installation

Implementation candidate on 2026-09-23. The Cargo workspace, lockfile, pinned `rust-toolchain.toml`, `.cargo/config.toml` (macOS 14 target), crates and Rust evidence were moved from `rust/` to the repository root; no Swift product, test, vendor or planning tree was retired. `sofdevtool-app` and `sofdevtool-core` inherit application version **0.2.0** from root `Cargo.toml`; `sofui` remains independently versioned **0.1.0**. Root `Makefile` exposes the eight specified commands, with `scripts/rust/` as the maintained Rust script implementation while old Swift-only root scripts remain for ticket 21 retirement. `README.md` now gives preliminary root usage and a temporary install override.

## Product identity and resources

- `identity::BuildProfile` selects Release `SofDevTool` / `com.gerardocalia.sofdevtool` or Debug `SofDevTool Debug` / `com.gerardocalia.sofdevtool.debug` from the Cargo build profile. The bundle ID is also the preference domain and normal Application Support directory name. The existing `SOFDEVTOOL_RUST_SUPPORT_ROOT` seam remains an exact caller-owned override; isolated tests use separate roots. The old Swift `~/Library/Application Support/SofDevTool` is not read or changed.
- `platform/macos/icons/{release,debug}.iconset` are copies of the established Xcode PNG artwork; the 1024-pixel source hashes remain `7827889df51dd46823dd6613b467b62df9c6f7ee40f1ec45506c43387b6a7038` and `d989c8cacec1a31ab97e8e695fe0de6cdf5a6952575067e2ef5cfda38c063c01`. The packager writes modern ICNS PNG chunks from those unchanged files. `/usr/bin/iconutil -c iconset` decoded the Release output in the packaging test. The original asset-catalog iconsets themselves were rejected by `iconutil -c icns` on this host, so repackaging did not alter the raster artwork.
- Each bundle contains its icon, app-owned Catppuccin notice, Text Diff generated dependency notices and retained source license. Text Diff HTML/JS and notice text are embedded by the renderer; the installed runtime needs no Node/npm or checkout files. The app title/header exposes version, channel, revision and source state.
- `scripts/rust/source-metadata.py` compares actual worktree blobs and modes with `HEAD`, filtering index-only distortions; it never infers a clean source from GitButler's synthetic index. Packaging requires a 40-character Git revision and a known clean/dirty state, rechecks it after Cargo build, and writes version/build/revision/channel/state to `Info.plist`. The default deliberate bundle build ID is `1`, overridable with positive numeric `SOFDEVTOOL_BUILD_ID`. Ordinary direct Cargo builds deliberately expose `revision=unknown, source=unknown` rather than allowing stale clean metadata after symbolic-ref commits. Cargo tracks the explicit packaging metadata environment inputs; a focused test exercises the package-to-direct transition.

## Checks actually run

All commands below used repository-root cwd, `CARGO_TARGET_DIR=/private/tmp/sofdevtool-terra-target`, and `CARGO_NET_OFFLINE=true` where Cargo ran. Native app launch was not performed.

| Check | Observed result |
| --- | --- |
| `make help` and `cargo metadata --no-deps` | Exit 0; app/core 0.2.0, sofui 0.1.0. Lockfile records those versions. |
| `make verify` | Exit 0: rustfmt, strict Clippy, workspace tests, isolated packaging tests, Debug app/gallery builds. |
| `npm_config_offline=true make verify-full` | Exit 0 after the retained fixture was formatted: default gate plus Release app/gallery build, offline `npm ci`, reproducible Text Diff bundle/dependency/notice check, actual Debug/Release packaging and a post-package direct-Cargo unknown-metadata regression. |
| `make release` with nondefault target directory | Exit 0; `artifacts/SofDevTool.app` had `CFBundleIdentifier=com.gerardocalia.sofdevtool`, version 0.2.0, build 1, channel release, Git revision `16ef581faab4c37e2eab74b79441acba39d2ec9a`, source state `dirty` (correct for this implementation checkout). Debug bundle had the `.debug` ID and separate icon/artifact path. Cargo JSON selected the executable under the overridden target directory. |
| `make install INSTALL_DESTINATION=/private/tmp/.../Applications` | Exit 0; installed only `SofDevTool.app` in the temporary destination with Release ID and the same revision. The test directory was removed. Neither `~/Applications` nor legacy app/data was touched. |
| `SOFDEVTOOL_BUILD_ID=0 make release` | Nonzero exit before build/replace; existing Release `Info.plist` SHA-256 unchanged. |
| Focused direct-Cargo metadata test after packaging | Exit 0, asserting direct build reports `unknown` revision/state, not the prior package's clean/dirty claim. |
| `scripts/rust/test_packaging.py` | Five tests pass: exact profile plist/notice/icon resources and iconutil decode; temporary install replaces only valid Release while preserving Debug/unrelated bundles; invalid destination and failed repackage preserve prior contents; package source revision equals actual HEAD with known state; a chmod-only change is detected despite identical blob bytes. The fifth test was added after the combined full gate and passed in a focused rerun. |

The independent retained-fixture owner formatted `crates/core/tests/retained_contract_vectors.rs` and reported its focused 8/8 test pass; a subsequent combined `make verify-full` passed. The later chmod-only packaging test also passed with a clean `cargo fmt --all --check`. The pre-existing transitive `block 0.1.6` future-incompatibility notice remains a warning; first-party Clippy passed with warnings denied.

`make run` and `make gallery` were implemented but intentionally not invoked in this source/packaging lane because they launch native windows. Default `~/Applications` installation and native Release relaunch/persistence remain ticket 22 acceptance. The independent external sofui consumer is ticket 19's gate, not claimed here. macOS 14/15 runtime was not exercised.

## Clean-worktree command acceptance — 2026-09-23

The coordinator later checked out committed correction
`df95bd39b281e4d7e44375ef8c7543417b01490e` in a separate managed Git
worktree at `/Users/gerardo/.codex/worktrees/sofdevtool-release-verification/SofDevTool`.
This was a real clean checkout, not a copied-source simulation. The following
commands ran there with the shared offline Cargo target; its final Git diff was
empty and porcelain status had no entries:

| Root Make target | Observed result |
| --- | --- |
| `make help` | Passed. |
| `make format` | Passed; `git diff --exit-code` remained clean. |
| `make verify` | Passed; [log](../../../.artifacts/parallel-rust-only/final/fresh-verify.log). |
| `make verify-full` | Passed: 339 Rust tests, five packaging tests, strict Clippy, copied-out sofui isolation, reproducible assets, Debug/Release bundles; [log](../../../.artifacts/parallel-rust-only/final/fresh-verify-full.log). |
| `make release` | Passed; Release 0.2.0 build 1 identified revision `df95bd39b281e4d7e44375ef8c7543417b01490e` and `source clean`; [log](../../../.artifacts/parallel-rust-only/final/fresh-release.log). |
| `make install INSTALL_DESTINATION=/private/tmp/sofdevtool-fresh-install-b73c2a7` | Passed and placed only the Release bundle in that temporary destination with the same clean revision; [log](../../../.artifacts/parallel-rust-only/final/fresh-install.log). |
| `make gallery` | Launched the standalone gallery binary, which stayed alive until the coordinator intentionally sent Ctrl-C. The resulting Make interrupt exit 2 reflects that stop, not a gallery failure. Native public interactions and the corrected numeric-label layout are separately recorded in [ticket 19](ticket-19.md). |
| `make run` | Built and opened the Debug 0.2.0 bundle with clean `df95bd3` metadata; [log](../../../.artifacts/parallel-rust-only/final/fresh-run.log). Computer Use observed its actual Graphite window. With `SOFDEVTOOL_RUST_SUPPORT_ROOT` set to a new temporary root, synthetic JSON `{"make_run_profile":"isolated"}` produced a result and one History entry; only that root's `History/json.history.v1.json` appeared. Cmd-Q ended the process. |

This directly closes ticket 20's root command-surface requirement. The Debug
launch used isolated synthetic data and does not prove physical global shortcut,
Dock/Spaces or macOS 14/15 behavior. Default home Applications Release delivery
and remaining native acceptance are recorded separately in [ticket 22](ticket-22.md).
