# SofDevTool maintained dependency snapshot

This is an immutable vendored copy of the published `PierreDiffsSwift` `1.2.4` revision `c2249d7890de957a96480711152d90a06fa1222b`, maintained for SofDevTool because the equivalent correction is not available in an upstream release.

The snapshot decodes bytes returned by JavaScript `atob()` with `TextDecoder` as UTF-8 before `JSON.parse()`. The upstream implementation passes the binary/Latin-1 string directly to `JSON.parse()`, corrupting accented characters and emoji.

It also distinguishes bridge availability from completed rendering, disables WebKit developer extras, and exposes a renderer-error callback. Only `PierreTextDiffRenderer` imports this package. Review the exact revision, UTF-8 regression, offline build, accessibility smoke, bundle lockfile, and `SofDevTool/Resources/ThirdPartyNotices.md` before updating the snapshot.
