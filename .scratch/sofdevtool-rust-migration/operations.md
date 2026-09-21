# Rust migration operational register

## Bootstrap — 2026-09-21

- Orchestrator: `thr_54ubtn5szt`; project `proj_usp2wfnybt`; environment `env_vtngs4hpem`; host `host_re5b55pjzz`.
- Initial product base: `4db6495` (Swift PR 5); initial GitButler workspace HEAD: `1fd92ee`.
- Integration branch: `feat/rust-gpui-migration`.
- Live catalog confirms `acp-opencode` / `opencode-go/deepseek-v4.1-flash` and `codex` / `gpt-6-astra`. First real worker session still needs verification.
- BB global concurrency unlimited; host automatic/effective limit 12. Use at most three implementation workers initially, reserving room for Astra review and integration. Desktop work is serialized.
- GitButler status succeeds with access to its local database. Linked-worktree commits are unsupported by the installed skill contract; use independent filesystem checkout copies with their own Git directories and validate GitButler before assignment.
- Approved spec, all 22 tickets and orchestration handoff enter the bootstrap commit. Documentation validation is the bootstrap gate; Swift tests are unnecessary for this documentation-only step.
- Canonical initial frontier: 01 only. No ticket accepted yet.

## Assignments

Pending bootstrap commit and isolated checkout validation.
