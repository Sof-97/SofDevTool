# Issue tracker: Local Markdown

Issues and specs for this repo live as Markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`.
- The spec is `.scratch/<feature-slug>/spec.md`.
- Implementation tickets are separate files at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`.
- Triage state is a `Status:` line near the top of each ticket; use the strings in `triage-labels.md`.
- Append comments under a `## Comments` heading in the ticket.

## Skill operations

- To publish an issue or spec, create the corresponding Markdown file under `.scratch/<feature-slug>/`.
- To fetch a ticket, read the referenced path.

## Wayfinding

- The map is `.scratch/<effort>/map.md`; each child ticket is `.scratch/<effort>/issues/NN-<slug>.md`.
- A `Type:` line records `research`, `prototype`, `grilling`, or `task`.
- A `Blocked by: NN, NN` line names prerequisite tickets. A ticket is unblocked when all listed tickets are resolved.
- Scan open, unblocked, unclaimed tickets by number to find the frontier.
- To claim a ticket, set `Status: claimed` and save before working.
- To resolve it, append an `## Answer`, set `Status: resolved`, and add a context pointer to the map's Decisions-so-far.
