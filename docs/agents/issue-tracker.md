# Local issue tracker

For current Rust-only work, start at the [planning index](../../.scratch/sofdevtool-rust-only/README.md), [canonical specification](../../.scratch/sofdevtool-rust-only/spec.md) and [approved ticket index](../../.scratch/sofdevtool-rust-only/ticket-proposal.md). Each ticket owns its blocking edges and acceptance criteria. The [operational register](../../.scratch/sofdevtool-rust-only/operations.md) owns dispatch and acceptance; it takes precedence over older assignment prose. Use the [execution contract](../../.scratch/sofdevtool-rust-only/parallel-execution.md) when coordinating shared-checkout work.

The preceding migration's planning and Swift implementation are retired from the maintained tree. Git history preserves the tracked sources. Its [dated evidence](../evidence/previous-migration/README.md) is retained for context, not as current acceptance evidence.

The orchestrator owns canonical ticket state: `ready-for-agent` means available after blockers are accepted; `claimed` means assigned; `resolved` means independently reviewed, integrated and verified. A candidate commit alone does not resolve a ticket. Workers report implementation and review readiness without changing ticket state or acceptance criteria.

Review uses the assigned ticket and fixed base/candidate commits, with separate Standards and Spec findings. A dependency graph is not authorization to begin implementation or launch concurrent writers.

This tracker is local Markdown. Commit and integrate locally through GitButler; remote publication is outside this initiative. Earlier Swift issues are historical product references in Git history.
