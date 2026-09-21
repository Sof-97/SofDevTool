# Unfinished work at the owner-requested stop

These patches preserve preliminary code; they are not applied to the accepted
application and are not evidence of completed tickets. Do not apply blindly.

- `ticket-03-unfinished.patch`: source-only delta from isolated
  `/private/tmp/sofdevtool-terra-03` against the accepted02 tree. Launcher stopped
  crashing after action-registration fixes, but the native popup was not visibly
  presented in the last03 check. Click-away/full-screen/global-shortcut/Dock
  acceptance remains incomplete. Some layout fixes already entered02. Merge
  current02 renderer/composition carefully; regenerate lockfile after manifests.
- `ticket-04-unfinished.patch`: app-owned History storage and typed JSON
  snapshot/session work from `/private/tmp/sofdevtool-terra-02`. Exact restore and
  one-shot snapshots have tests; app UI recording/preview/confirmation and
  relaunch acceptance are not integrated. Module exports and storage ownership
  must be composed deliberately. Last-good atomic-failure coverage and policy
  integration need review. This work does not bypass03 as a blocker.

Original isolated trees remain available at the paths above. No further work
was scheduled after completing02.
