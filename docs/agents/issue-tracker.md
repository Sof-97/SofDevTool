# Local issue tracker

For Rust migration implementation and review, read the approved [specification](../../.scratch/sofdevtool-rust-migration/spec.md), then the assigned Markdown ticket in [issues](../../.scratch/sofdevtool-rust-migration/issues/). The [index](../../.scratch/sofdevtool-rust-migration/ticket-review.md) summarizes the breakdown; each ticket's `Blocked by` field owns its dependencies.

The orchestrator owns canonical ticket state: `ready-for-agent` means available after blockers are accepted; `claimed` means assigned; `resolved` means independently reviewed, integrated and verified. A candidate commit alone does not resolve a ticket. Workers report implementation and review readiness without changing ticket state or acceptance criteria.

Record assignments, checkout paths, branch names, worker/reviewer thread IDs, immutable base/candidate/integrated SHAs, gates and blockers in the [operational register](../../.scratch/sofdevtool-rust-migration/operations.md). Review uses the assigned ticket and fixed base/candidate commits, with separate Standards and Spec findings. Follow [orchestrator instructions](../../.scratch/sofdevtool-rust-migration/orchestrator-prompt.md) for acceptance and scheduling.

This tracker is local Markdown. Commit and integrate locally through GitButler; remote publication is outside this migration assignment. Earlier Swift issues remain historical product references.
