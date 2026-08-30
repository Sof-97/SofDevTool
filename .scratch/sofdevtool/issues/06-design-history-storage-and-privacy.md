Type: grilling
Status: resolved
Assignee: Codex
Blocked by: 01, 05

## Question

How should configurable on-device Utility History behave and be stored? Begin from the settled policy: enabled by default, at most 25 inputs and results per Utility, per-Utility opt-out, JWT history disabled by default, and Clear Utility plus Clear All controls. Decide lifecycle, deletion semantics, data representation, failure behavior, Settings UX, and whether any data class requires stronger protection, using the selected stack's local persistence capabilities.

## Comments

### Design tree — round 1, awaiting owner answers (2026-08-28)

The current frontier covers four independent choices: the recording-control hierarchy and retention behavior, the restore interaction, the persistence representation, and the protection level for arbitrary developer payloads.

Recommendations presented to the owner:

1. Provide a global **Record Utility History** switch plus per-Utility switches. Turning either off stops new recording immediately but retains existing entries; deletion remains an explicit separate action. Keep the scaffold's retention cap fixed at the newest 25 entries per Utility rather than adding a count preference.
2. Treat a History entry as a past Utility Operation, not an autosaved workspace. Opening an entry previews it; an explicit Restore action replaces that Utility's current workspace state, warns only when it would discard a different non-empty session, and does not itself create a new History entry.
3. Store History through a small Foundation/Codable repository in Application Support, with an application-level envelope and one atomically replaced file per Utility. Keep Utility-owned snapshot payloads opaque to the repository. Store only small non-payload preferences in UserDefaults; do not adopt SwiftData for this bounded heterogeneous collection.
4. Treat every History payload as potentially sensitive: never log, index, sync, export, or expose it outside the app. Rely on the user's macOS account, app-container/file permissions, and FileVault for at-rest protection in the scaffold; do not add custom encryption or put arbitrary payloads in Keychain. Keep JWT History disabled by default and let the owner disable any other Utility.

### Owner decision — round 1 (2026-08-28)

- Accepted the recommended global and per-Utility recording controls, retain-on-disable behavior, and fixed 25-entry cap.
- Accepted explicit preview/restore behavior, including a warning only when restoration would replace a different non-empty session and no new entry merely from restoration.
- Accepted the bounded Foundation/Codable repository in Application Support, with one atomically written file per Utility and UserDefaults limited to small preferences.
- Rejected any special data-protection machinery as unnecessary overhead. History uses ordinary application storage with no Keychain integration, custom encryption, or protection-specific infrastructure.

### Design tree — round 2, awaiting owner answers (2026-08-28)

The next frontier covers deletion semantics, persistence failures, unavailable snapshot schemas and Utilities, and History placement in the Workbench.

Recommendations presented to the owner:

5. **Clear Utility** permanently deletes that Utility's stored entries after a lightweight confirmation. **Clear All** uses a stronger confirmation naming the scope. Neither action has undo or moves data to Trash; both delete persisted and in-memory History while leaving the current workspace session unchanged.
6. History failure never invalidates a successful Utility result. Atomic writes preserve the last good file; a recording failure shows a non-modal warning and pauses further recording for that Utility until a later successful retry or relaunch. A malformed per-Utility file is isolated, shown as unavailable, and can be deleted by Clear Utility; the app neither crashes nor silently replaces it.
7. If a Utility cannot decode an older snapshot schema, keep the entry visible as unavailable with its timestamp and allow deletion, but do not guess a migration. History files belonging to a Utility ID absent from the current registry remain untouched and hidden; Clear All still removes them, and they become visible again if that Utility returns.
8. Present History in a shell-owned inspector for the selected Utility rather than adding another top-level catalog scope. The inspector lists newest first, supports preview/restore and Clear Utility, and can be hidden without losing the current workspace. Settings owns the global switch, per-Utility switches, and Clear All.

### Owner decision — round 2 (2026-08-28)

- Accepted permanent Clear Utility and Clear All behavior while preserving the current workspace, and requested press-and-hold confirmation.
- Accepted non-modal recording failures, preservation of the last valid atomic file, and isolation of malformed History to its owning Utility.
- Accepted unavailable-but-deletable entries for unsupported snapshot schemas and hidden-but-preserved files for Utility IDs absent from the current registry.
- Accepted a shell-owned, collapsible History inspector for the selected Utility, with recording controls and Clear All in Settings.

### Design tree — final round, awaiting owner answers (2026-08-28)

The final frontier specifies the accessible press-and-hold behavior, History Entry presentation metadata, restoration semantics, and recording/write semantics.

Recommendations presented to the owner:

9. Use press-and-hold as the pointer interaction for destructive confirmation: one second for Clear Utility and two seconds for Clear All, with visible progress and cancellation on early release, pointer exit, or Escape. Keyboard activation and assistive technologies open an ordinary destructive confirmation alert, so sustained pointer input is never the only route.
10. A Utility History Entry stores an app-owned entry ID and capture timestamp plus the immutable Utility ID, snapshot schema version, and opaque snapshot bytes. The inspector labels entries by local date/time and asks the Utility to render the decoded preview; it does not persist a duplicate derived title or searchable excerpt.
11. Restore applies the captured controls, input, and result exactly as represented by the snapshot without automatically rerunning the Utility. The restored state is not recorded again; only a later completed operation caused by the owner can create a new entry.
12. Record every distinct completed Utility Operation, including a deliberate repeat with identical data. Serialize writes per Utility immediately after completion, trim to the newest 25 before atomically replacing its JSON file, and use UTC timestamps plus stable entry IDs so ordering does not rely on timestamp uniqueness.

### Owner decision — final round (2026-08-28)

The owner accepted recommendations 9–12 without exceptions. The design tree is complete.

## Answer

Utility History is a bounded, configurable, on-device record of completed Utility Operations. It is enabled globally by default, and every History-capable Utility has its own switch. Turning the global or per-Utility switch off stops new recording immediately but preserves existing entries. The global switch does not erase or rewrite the saved per-Utility choices. The first scaffold uses a fixed maximum of the newest 25 entries per Utility rather than a configurable count. JWT Decoder supports History but defaults off; the other first-release Utilities default on.

### History Entries and recording

Every distinct completed Utility Operation creates one Utility History Entry, including a deliberate repeat of identical data. Invalid attempts, intermediate edits, restoration alone, and merely viewing an entry create nothing.

The application-owned envelope contains a stable entry ID, UTC capture timestamp, immutable Utility ID, snapshot schema version, and opaque Utility-owned snapshot bytes. The repository does not inspect the payload or duplicate a derived title or searchable excerpt. The inspector labels entries using local date and time; after decoding, the owning Utility renders the preview.

Writes are serialized independently per Utility and attempted immediately after a completed operation. Before persistence, the repository orders entries deterministically by timestamp and stable ID and trims them to the newest 25.

### Persistence and protection boundary

Use a small Foundation/Codable repository, not SwiftData. Store one versioned JSON file per Utility in Application Support and atomically replace that file after a successful write. Store only small recording preferences in UserDefaults. The per-Utility file boundary keeps deletion, corruption, and concurrent writes isolated while leaving each Utility in charge of its own snapshot schemas and migrations.

No Keychain integration, custom encryption, file-protection feature, export system, or other security-specific persistence machinery belongs in the scaffold. History is ordinary local application data. Existing product boundaries still exclude accounts, cloud synchronization, and telemetry.

### Presentation and restoration

The shell presents History in a collapsible inspector for the selected Utility, ordered newest first. Hiding the inspector does not alter the current Utility Workspace Session. Selecting an entry previews it; Restore explicitly applies the controls, input, and result represented by the snapshot without automatically rerunning the Utility.

Restore warns only when it would replace a different non-empty workspace session. It does not itself record another entry. A later completed operation initiated by the owner may record normally. Settings owns the global recording switch, retained per-Utility switches, and Clear All; the inspector owns Clear Utility.

### Deletion

Clear Utility permanently removes that Utility's persisted and in-memory History while leaving its current workspace session unchanged. Its pointer confirmation requires a one-second press-and-hold. Clear All permanently removes every History file, including files for Utilities absent from the current registry, and requires a two-second hold. Both controls show hold progress and cancel on early release, pointer exit, or Escape.

Sustained pointer input is not the only deletion path. Keyboard activation and assistive technologies use an ordinary destructive confirmation alert. Deletion has no undo and does not move History to Trash.

### Failure and compatibility behavior

A History failure never invalidates or hides a successful Utility result. Atomic replacement preserves the last valid file when a write fails. The shell shows a non-modal warning and pauses further recording for that Utility until a later successful retry or application relaunch.

A malformed file affects only its Utility: History for that Utility is shown as unavailable and remains removable through Clear Utility; the application neither crashes nor silently replaces the file. If a Utility cannot decode an older snapshot schema, the entry remains visible as unavailable with its timestamp and may be deleted, but the repository never guesses a migration.

Files whose Utility IDs are absent from the current registry remain untouched and hidden. They become visible again if the Utility returns, while Clear All removes them with all other History.
