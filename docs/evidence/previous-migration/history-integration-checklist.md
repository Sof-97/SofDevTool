# Ticket 04 integration checklist

Read-only preparation while tickets 02 and 03 are implemented. This is guidance
for the approved ticket, not a change to its acceptance criteria.

- JsonSession currently permits repeated resolve(current_revision), reevaluating
  every time. Expose each accepted valid settlement for recording exactly once;
  reject duplicate resolutions/consumption and stale, invalid and empty work.
- Restore the captured request and output directly, invalidate pending revisions,
  update controls without triggering evaluation, and never offer a new snapshot.
  Test restore with an old debounce pending and deliver that completion afterward.
- JsonSnapshot output is text, including unquoted query strings and the approved
  boolean result 1/0. Never parse or reformat saved output. A hand-authored snapshot
  differing from fresh evaluation proves restore does not execute.
- If evaluation moves off-thread, capture request plus revision and gate both
  output and History publication. Tests complete B before A after submitting A/B.
- Inject temporary Rust roots, fixed timestamps and entry IDs. Retain newest25
  with deterministic ID tie order, reload exact payloads, and preserve a Swift
  namespace sentinel. File version and snapshot version are separate.
- Write a same-directory temporary file, then atomically replace; publish memory
  only after successful persistence. Inject failure before replace and retain
  old file, entries and visible result. Never overwrite malformed stored files.
- Ticket05 owns controls/deletion/recovery UI; establish safe persistence first.

The owner-directed IME evidence exception remains in force; recheck editor,
WebView and Launcher together without reopening Japanese input configuration.
