# Rust-only SofDevTool and sofui

This is the planning package for the sole Rust application and sofui's later independent release. The approved specification and ticket bodies are retained alongside implementation evidence.

- [Complete specification](spec.md): agreed product scope, the six review findings, architecture, testing and delivery contract.
- [Approved ticket index](ticket-proposal.md): 22 published tickets with blocking edges and story coverage. Each linked ticket contains its accepted criteria and testing guidance.
- [Project tracker conventions](../../docs/agents/issue-tracker.md): local Markdown and triage vocabulary.
- [Dispatch register](operations.md): current coordination, review and acceptance state. `prompts/` contains dated worker dispatch text, including pre-relocation paths; use the current root [agent guide](../../AGENTS.md) for active source and command guidance.

The [dispatch register](operations.md) owns current worker, review and acceptance state. Earlier trial and worker prompts are historical context; credentials and personal application data remain excluded from handoffs.

## Publication and execution boundaries

The owner approved the 22-ticket breakdown on 2026-09-22. Tickets are published as individual Markdown files under `issues/`, numbered from 01 in dependency order. Read the individual ticket and [operational register](operations.md) for its current state.

Ticket triage and scheduling are separate. `ready-for-agent` means eligible after its recorded blockers are accepted; it does not by itself authorize work. Native-preview and release acceptance have their own checkpoints in the ticket series.

The old migration's tickets 01–22 are historical records recoverable from Git history. Numbers in the new index refer only to this initiative. The former follow-up specification 23 was moved here at the owner's request; there is one canonical copy.

The specification owns product behavior. Individual ticket bodies own status, acceptance criteria and blocking edges. The approved index retains the publication approval and navigation links, using its original filename to preserve references. The parent specification's content and status are unchanged by publication.
