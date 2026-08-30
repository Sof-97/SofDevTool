# SofDevTool prototype patch

This is a disposable vendored copy of the published `PierreDiffsSwift` `1.2.4` revision `c2249d7890de957a96480711152d90a06fa1222b`.

The prototype changes one transport detail in `DiffWebViewCoordinator.callJavaScript`: bytes returned by JavaScript `atob()` are decoded with `TextDecoder` as UTF-8 before `JSON.parse()`. The upstream implementation passes the binary/Latin-1 string directly to `JSON.parse()`, corrupting accented characters and emoji.

This directory is evidence that the defect is fixable. It is not a production dependency policy. SofDevTool should prefer an upstream release containing an equivalent regression-tested fix; otherwise any maintained fork must remain isolated inside the `TextDiffRenderer` module.
