# GPUI Kit: upstream Unicode deletion check

Date: 2026-09-26. Read-only investigation of upstream; no issue or PR submitted.

## Result

No ready-to-adopt upstream fix was identified in the checked release and current
main implementation. This is a bounded finding, not proof that no related work
exists anywhere upstream.

- GitHub's latest-release API reports **v0.6.6**, published
  `2026-09-21T13:28:30Z`: [official release](https://github.com/longbridge/gpui-kit/releases/tag/v0.6.6).
  No later published release was returned by that endpoint.
- Main resolved to `bcb6a0f3a6fd9e7ac9688ad653023b34f2f16bb6`.
  [Backspace and Delete](https://github.com/longbridge/gpui-kit/blob/bcb6a0f3a6fd9e7ac9688ad653023b34f2f16bb6/crates/base/src/input/base/state.rs#L1779)
  still call `previous_boundary` and `next_boundary`.
- [Those boundary methods](https://github.com/longbridge/gpui-kit/blob/bcb6a0f3a6fd9e7ac9688ad653023b34f2f16bb6/crates/base/src/input/base/state.rs#L3204)
  adjust byte offsets through `cursor_boundary`, which calls `clip_offset`,
  handles registered inline tokens, and keeps CRLF together.
- [clip_offset](https://github.com/longbridge/gpui-kit/blob/bcb6a0f3a6fd9e7ac9688ad653023b34f2f16bb6/crates/base/src/input/base/rope_ext.rs#L424)
  clips to UTF-8 character boundaries (`is_char_boundary`, `floor_char_boundary`,
  `ceil_char_boundary`), not extended grapheme cluster boundaries.
  Thus the inspected main path does not automatically make a decomposed accent
  or a joined family emoji a single deletion unit. This is source evidence;
  main was not built or exercised natively during this investigation.

## Related official capability

[Issue #3110](https://github.com/longbridge/gpui-kit/issues/3110) describes atomic
inline tokens for mentions and application-defined resources. Main now consults
token boundaries in the editing path. That is a different abstraction: plain
Unicode text is not automatically registered as tokens. Treating every grapheme
as an application-managed token would introduce new application editing work,
and is not an established upstream Unicode deletion fix.

## Search scope and limits

GitHub issue/PR search (all states) for `repo:longbridge/gpui-kit grapheme`
returned three results: #3057 (website font fallback), #2949 (inline-code
rendering), and #3110 (atomic tokens). Search for `backspace` returned twelve
results, covering paired brackets, tokens, password word deletion, completions,
undo, time fields, replacement boundaries and older keyboard work. No matching
extended-grapheme deletion fix was identified in those results. Search coverage
is keyword-based and does not exclude differently named or unindexed work.

The local pinned API and native/test behavior are investigated separately by
the coordinating session. This note does not approve a custom editing exception,
change the tracker decision, or establish migration/native acceptance.
