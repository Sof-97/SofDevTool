# 19: Finish sofui extraction readiness and remove compatibility scaffolding

**What to build:** A separate consumer can run the documented sofui components and gallery without SofDevTool code, resources, persistence or lifecycle.

**Blocked by:** [13: Redesign JSON, YAML/JSON, Base64, URL Encoding and JWT](13-text-inspection-workspaces-redesign.md); [14: Redesign Hashes, Timestamps, Case and Whitespace Conversion](14-conversion-workspaces-redesign.md); [15: Redesign Identifier Generator, Random String and Sample Data](15-generator-workspaces-redesign.md); [16: Redesign Regex and Color Conversion without regressing corrections](16-regex-color-redesign.md); [18: Redesign Text Diff and preserve native WebView interactions](18-text-diff-redesign.md).

**Status:** ready-for-agent

- [ ] Contract the expanded APIs only after every application consumer has migrated; remove obsolete UI and History coordination adapters and temporary preview scaffolding.
- [ ] Audit exported components for product wording, retention policy, Utility types, preference/file access and application startup; keep only reusable presentation/interaction logic.
- [ ] Build/run the standalone gallery and an independent consumer using declared metadata/dependencies; no application/core imports or product resource paths are required.
- [ ] Document setup, mounting, IDs/focus, text events/undo, holds, tokens and dependency/licenses; exercise public interactions. Keep independent versioning; create no external repository or published package.

## Testing

Independent consumer and gallery builds/runs plus public behavior tests and a dependency/resource ownership audit.

Test observable behavior at this existing seam using the parent specification's testing contract. Preserve independent expectations and isolate application data; record native observations separately from build and automated-test evidence.

## Specification and approval

**Parent:** [Rust-only SofDevTool and sofui specification](../spec.md). Read its relevant contracts and shared invariants before implementation; this ticket does not replace exact Utility limits or ownership/privacy rules.

**User stories:** 15, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67.

**Breakdown approval:** Owner approved the 22-ticket breakdown on 2026-09-22; see the [approved index](../ticket-proposal.md).

Publication is documentation only. Begin implementation only under a subsequent execution instruction and after this ticket's blockers are accepted. The native preview's visual approval is a separate requirement in ticket 11.
