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

- Bootstrap commit: `5ed34cc460d9c5cabcd0b795aab79ee25d931bf6`. Gate passed: 22 tickets present and all local Markdown links resolve.
- Independent checkout validation: copied tracked files and independent `.git`, registered with `but setup`; `but branch new` successfully created worker branch above the bootstrap. No linked worktree or raw Git writes used.

| Ticket / role | Thread | Checkout | Branch | Base | Candidate / integrated | State |
| --- | --- | --- | --- | --- | --- | --- |
| 01 implementation | `thr_tpymn94mfd` | `/Users/gerardo/.bb/thread-storage/rust-migration-thr_54ubtn5szt/worker-01` | `feat/rust-01-json` | `5ed34cc460d9c5cabcd0b795aab79ee25d931bf6` | Pending | claimed |
| Independent Astra reviewer | `thr_v6himghfju` | `/Users/gerardo/.bb/thread-storage/rust-migration-thr_54ubtn5szt/reviewer` | stable bootstrap snapshot | `5ed34cc460d9c5cabcd0b795aab79ee25d931bf6` | Pending | preparing criteria; no acceptance |

Desktop lease: worker 01 for focused editor scenarios. Other threads must coordinate before native interaction.

## Session evidence and current impediment

- Real worker execution confirmed: `opencode-go/deepseek-v4.1-flash`, ACP session `ses_f3a8aeda9ffeLzsuoo1wMvdppD`, effective permissions `accept-edits` (the provider-supported mode below the parent's `auto` ceiling). Astra execution confirmed `gpt-6-astra`, `auto`. Execution metadata saved under `/Users/gerardo/.bb/thread-storage/rust-migration-thr_54ubtn5szt/`.
- Reviewer preparation complete: `/Users/gerardo/.bb/thread-storage/rust-migration-thr_54ubtn5szt/reviewer-preparation-01.md`; independent Standards/Spec acceptance matrix, no candidate acceptance.
- Skill access interaction `pint_jy8rpwcz5p` was approved once using the normal BB interaction API, within the owner's explicit instruction to apply implement. BB confirms resolved/allow_once.
- Worker completed dependency discovery but three turns ended without a candidate. Detailed events show external `read` calls rejected with `The user rejected permission to use this specific tool call.` In turn 3, events 560/562/564 contain this rejection; no pending interaction remains. This is not evidence of invalid credentials or an unavailable model.
- Requested owner clarification for the refused Cargo dependency-source and reviewer-report reads. Worker instructed to respect the refusals and continue only independent checkout-local JSON/core work. Ticket 01 stays claimed, all dependent tickets remain blocked. No product gate or native acceptance has passed.
