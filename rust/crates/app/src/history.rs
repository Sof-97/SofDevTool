//! Fresh, Rust-only History storage.
//!
//! One versioned JSON file per Utility under the Rust Application Support
//! namespace. Payloads are Utility-owned, opaque to this module and never
//! logged, indexed, synced or exported. Writes replace the file atomically and
//! a failed write keeps the last valid file, pauses that Utility and never
//! overwrites a malformed file.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::Mutex;

use gpui::App;
use serde::{Deserialize, Serialize};

/// Newest entries retained per Utility.
pub const RETENTION: usize = 25;

/// On-disk schema version for the History file envelope.
pub const FILE_VERSION: u32 = 1;

const FILE_SUFFIX: &str = ".history.v1.json";

/// One retained, settled Utility Operation.
///
/// The payload is the Utility-owned snapshot. This module never interprets it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub captured_at: String,
    pub utility_id: String,
    pub snapshot_version: u32,
    pub payload: serde_json::Value,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HistoryError {
    /// The Utility id is not a safe single filename component.
    InvalidUtility,
    /// The stored file is malformed or belongs to another Utility/version.
    Corrupt,
    /// The file could not be read or written.
    Unavailable,
    /// A prior write failed; ordinary operations cannot resume recording.
    Paused,
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtility => write!(formatter, "the Utility identity is not storable"),
            Self::Corrupt => write!(formatter, "the stored History file is not valid"),
            Self::Unavailable => write!(formatter, "the History file could not be read or written"),
            Self::Paused => write!(formatter, "recording is paused until Retry succeeds"),
        }
    }
}

/// Outcome of a Clear All attempt. Every listed Utility was checked, even if
/// another deletion failed; callers can reconcile views with actual storage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClearReport {
    pub cleared_ids: Vec<String>,
    pub failed_ids: Vec<String>,
}

/// A single-owner History repository for one Rust Application Support root.
///
/// Writes are serialized in-process. Every mutation replaces the file
/// atomically; a failed mutation marks the Utility paused until a successful
/// retry or relaunch.
#[derive(Debug)]
pub struct HistoryStore {
    root: PathBuf,
    paused: Mutex<BTreeSet<String>>,
    mutation: Mutex<()>,
    #[cfg(test)]
    fail_before_replace: Mutex<usize>,
}

impl HistoryStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            paused: Mutex::new(BTreeSet::new()),
            mutation: Mutex::new(()),
            #[cfg(test)]
            fail_before_replace: Mutex::new(0),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Loads the retained entries for `utility_id`, newest first.
    ///
    /// A missing file is an empty History. A malformed file, or one that
    /// declares a different Utility, is reported as corrupt and left untouched.
    pub fn load(&self, utility_id: &str) -> Result<Vec<HistoryEntry>, HistoryError> {
        let path = self.path(utility_id)?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(HistoryError::Unavailable),
        };
        let file: HistoryFile =
            serde_json::from_slice(&bytes).map_err(|_| HistoryError::Corrupt)?;
        if file.version != FILE_VERSION || file.utility_id != utility_id {
            return Err(HistoryError::Corrupt);
        }
        if file
            .entries
            .iter()
            .any(|entry| entry.utility_id != utility_id)
        {
            return Err(HistoryError::Corrupt);
        }
        Ok(file.entries)
    }

    /// Appends one settled operation, retaining the newest [`RETENTION`]
    /// entries. Returns the retained set on success.
    pub fn record(&self, entry: HistoryEntry) -> Result<Vec<HistoryEntry>, HistoryError> {
        let _mutation = self
            .mutation
            .lock()
            .expect("History mutation lock poisoned");
        let utility_id = entry.utility_id.clone();
        if self.is_paused(&utility_id) {
            return Err(HistoryError::Paused);
        }
        let result = self.append(entry);
        if result.is_err() {
            self.pause(&utility_id);
        }
        result
    }

    fn append(&self, entry: HistoryEntry) -> Result<Vec<HistoryEntry>, HistoryError> {
        let utility_id = entry.utility_id.clone();
        let mut entries = self.load(&utility_id)?;
        entries.push(entry);
        entries.sort_by(|a, b| {
            b.captured_at
                .cmp(&a.captured_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        entries.truncate(RETENTION);
        self.write(&utility_id, &entries)?;
        Ok(entries)
    }

    /// Removes all retained entries for `utility_id`. The current workspace is
    /// untouched; only the Rust History file is affected.
    pub fn clear(&self, utility_id: &str) -> Result<(), HistoryError> {
        let _mutation = self
            .mutation
            .lock()
            .expect("History mutation lock poisoned");
        if utility_id.is_empty()
            || utility_id == "."
            || utility_id == ".."
            || utility_id.contains('/')
        {
            return Err(HistoryError::InvalidUtility);
        }
        let path = self.root.join(format!("{utility_id}{FILE_SUFFIX}"));
        self.remove_path(utility_id, &path)
    }

    fn remove_path(&self, utility_id: &str, path: &Path) -> Result<(), HistoryError> {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(HistoryError::Unavailable),
        }
        self.resume(utility_id);
        Ok(())
    }

    /// Removes every History file under this root, including files for
    /// Utilities that are no longer registered. Nothing outside the root is
    /// ever touched.
    pub fn clear_all(&self) -> Result<(), HistoryError> {
        let report = self.clear_all_report()?;
        if report.failed_ids.is_empty() {
            Ok(())
        } else {
            Err(HistoryError::Unavailable)
        }
    }

    pub fn clear_all_report(&self) -> Result<ClearReport, HistoryError> {
        let _mutation = self
            .mutation
            .lock()
            .expect("History mutation lock poisoned");
        let mut report = ClearReport::default();
        for (id, path) in self.stored_history_paths()? {
            match self.remove_path(&id, &path) {
                Ok(()) => report.cleared_ids.push(id),
                Err(_) => report.failed_ids.push(id),
            }
        }
        self.paused
            .lock()
            .expect("History pause lock poisoned")
            .retain(|id| report.failed_ids.contains(id));
        Ok(report)
    }

    /// Every Utility id that has a stored History file, including unknown ones.
    pub fn stored_utility_ids(&self) -> Result<Vec<String>, HistoryError> {
        Ok(self
            .stored_history_paths()?
            .into_iter()
            .map(|(id, _)| id)
            .collect())
    }

    /// Paths come only from directory entries beneath this store's root. Clear
    /// All can therefore remove legacy/unknown filename IDs without relaxing
    /// the strict IDs accepted for new reads and writes.
    fn stored_history_paths(&self) -> Result<Vec<(String, PathBuf)>, HistoryError> {
        let directory = match fs::read_dir(&self.root) {
            Ok(directory) => directory,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(HistoryError::Unavailable),
        };
        let mut paths = Vec::new();
        for entry in directory {
            let entry = entry.map_err(|_| HistoryError::Unavailable)?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(id) = name.strip_suffix(FILE_SUFFIX) {
                if !id.is_empty() {
                    paths.push((id.to_owned(), entry.path()));
                }
            }
        }
        paths.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(paths)
    }

    /// True when a failed write paused recording for `utility_id`.
    pub fn is_paused(&self, utility_id: &str) -> bool {
        self.paused
            .lock()
            .map(|paused| paused.contains(utility_id))
            .unwrap_or(false)
    }

    /// Retries persistence of exactly the retained set. No new operation is
    /// created. Corrupt files stay untouched and the pause remains in force.
    pub fn retry(&self, utility_id: &str) -> Result<Vec<HistoryEntry>, HistoryError> {
        let _mutation = self
            .mutation
            .lock()
            .expect("History mutation lock poisoned");
        let entries = self.load(utility_id)?;
        if self.is_paused(utility_id) {
            self.write(utility_id, &entries)?;
            self.resume(utility_id);
        }
        Ok(entries)
    }

    fn resume(&self, utility_id: &str) {
        if let Ok(mut paused) = self.paused.lock() {
            paused.remove(utility_id);
        }
    }

    fn pause(&self, utility_id: &str) {
        if let Ok(mut paused) = self.paused.lock() {
            paused.insert(utility_id.to_owned());
        }
    }

    fn write(&self, utility_id: &str, entries: &[HistoryEntry]) -> Result<(), HistoryError> {
        let path = self.path(utility_id)?;
        fs::create_dir_all(&self.root).map_err(|_| HistoryError::Unavailable)?;
        let serialized = serde_json::to_vec_pretty(&HistoryFile {
            version: FILE_VERSION,
            utility_id: utility_id.to_owned(),
            entries: entries.to_vec(),
        })
        .map_err(|_| HistoryError::Unavailable)?;
        // Same-directory temporary file, then atomic replacement. A crash or a
        // failed replace leaves the previous valid file in place.
        let temporary = self.root.join(format!(
            ".{utility_id}{FILE_SUFFIX}.{}.tmp",
            std::process::id()
        ));
        if fs::write(&temporary, serialized).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(HistoryError::Unavailable);
        }
        #[cfg(test)]
        {
            let mut failures = self.fail_before_replace.lock().expect("write fault lock");
            if *failures > 0 {
                *failures -= 1;
                let _ = fs::remove_file(&temporary);
                return Err(HistoryError::Unavailable);
            }
        }
        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(()),
            Err(_) => {
                let _ = fs::remove_file(&temporary);
                Err(HistoryError::Unavailable)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn fail_next_writes(&self, count: usize) {
        *self.fail_before_replace.lock().expect("write fault lock") = count;
    }

    fn path(&self, utility_id: &str) -> Result<PathBuf, HistoryError> {
        if utility_id.is_empty()
            || !utility_id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(HistoryError::InvalidUtility);
        }
        Ok(self.root.join(format!("{utility_id}{FILE_SUFFIX}")))
    }
}

#[derive(Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    utility_id: String,
    entries: Vec<HistoryEntry>,
}

/// Injectable source of History timestamps and entry ids.
///
/// Production uses the system clock and cryptographic-free unique ids; tests
/// inject fixed values so retention and round-trips are deterministic.
pub trait HistoryClock: 'static {
    /// ISO 8601 UTC, fixed width, so lexicographic order equals chronological order.
    fn captured_at(&self) -> String;
    /// A stable, unique entry id.
    fn next_id(&self) -> String;
}

/// The production clock: system time plus an in-process sequence.
pub struct SystemClock {
    sequence: std::cell::Cell<u64>,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            sequence: std::cell::Cell::new(0),
        }
    }
}

impl HistoryClock for SystemClock {
    fn captured_at(&self) -> String {
        format_iso8601_utc(system_seconds())
    }

    fn next_id(&self) -> String {
        let sequence = self.sequence.get();
        self.sequence.set(sequence.wrapping_add(1));
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("{nanos:020}{sequence:04}")
    }
}

fn system_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

/// Formats Unix seconds as `YYYY-MM-DDTHH:MM:SSZ` (proleptic Gregorian, UTC).
fn format_iso8601_utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to a civil date.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let shifted = days_since_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// A callback that persists the recording policy after a change.
pub type PolicyPersister = Box<dyn Fn(&HistoryPolicy)>;

type ChangeCallback = Rc<dyn Fn(&mut App)>;

enum SubscriberKind {
    Utility(String),
    Status,
}

#[derive(Default)]
struct HistorySubscribers {
    next_id: u64,
    callbacks: BTreeMap<u64, (SubscriberKind, ChangeCallback)>,
}

/// Keeps one workspace's History invalidation callback registered. Dropping a
/// workspace drops its subscription, including when it was hidden before close.
pub struct HistorySubscription {
    id: u64,
    subscribers: Weak<RefCell<HistorySubscribers>>,
}

impl Drop for HistorySubscription {
    fn drop(&mut self) {
        if let Some(subscribers) = self.subscribers.upgrade() {
            subscribers.borrow_mut().callbacks.remove(&self.id);
        }
    }
}

/// Cached presentation state for one Utility's History inspector. This state
/// is independent of the Utility's live request/result and can be reconciled
/// after Settings changes without touching the active workspace.
#[derive(Default)]
pub struct HistoryViewState {
    pub entries: Vec<HistoryEntry>,
    pub selected: Option<String>,
    pub pending_restore: Option<HistoryEntry>,
    pub error: Option<String>,
}

impl HistoryViewState {
    pub fn load(recorder: &HistoryRecorder, utility_id: &str) -> Self {
        let mut state = Self::default();
        state.reconcile(recorder, utility_id);
        state
    }

    /// Reloads actual storage after a clear, partial clear or retry. Selection
    /// and pending restore survive only while their exact entries still exist.
    pub fn reconcile(&mut self, recorder: &HistoryRecorder, utility_id: &str) {
        match recorder.load(utility_id) {
            Ok(entries) => {
                self.entries = entries;
                self.retain_valid_references();
                self.error = recorder
                    .store()
                    .is_paused(utility_id)
                    .then(|| HistoryError::Paused.to_string());
            }
            Err(error) => {
                self.entries.clear();
                self.selected = None;
                self.pending_restore = None;
                self.error = Some(error.to_string());
            }
        }
    }

    /// Applies the recording boundary's outcome without altering the active
    /// Utility evaluation. A paused or failed write leaves cached retained
    /// entries intact and exposes its warning.
    pub fn apply_record(
        &mut self,
        recorder: &HistoryRecorder,
        utility_id: &str,
        result: Result<Vec<HistoryEntry>, HistoryError>,
    ) {
        match result {
            Ok(entries) => {
                self.entries = entries;
                self.retain_valid_references();
                self.error = recorder
                    .store()
                    .is_paused(utility_id)
                    .then(|| HistoryError::Paused.to_string());
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn retain_valid_references(&mut self) {
        if !self
            .entries
            .iter()
            .any(|entry| self.selected.as_deref() == Some(entry.id.as_str()))
        {
            self.selected = None;
        }
        if !self
            .entries
            .iter()
            .any(|entry| self.pending_restore.as_ref() == Some(entry))
        {
            self.pending_restore = None;
        }
    }

    pub fn select(&mut self, id: &str) -> bool {
        if self.entries.iter().any(|entry| entry.id == id) {
            self.selected = Some(id.to_owned());
            true
        } else {
            false
        }
    }

    /// Checks the exact persisted entry before a restore, so a late click or
    /// confirmation cannot restore an entry already deleted in Settings.
    pub fn retained(
        &self,
        recorder: &HistoryRecorder,
        utility_id: &str,
        entry: &HistoryEntry,
    ) -> bool {
        recorder
            .load(utility_id)
            .ok()
            .is_some_and(|entries| entries.contains(entry))
    }
}

/// Application-owned recording facade over [`HistoryStore`].
///
/// A Utility records through this type; it owns the entry identity and
/// timestamp so the storage layer stays free of clock/random policy. Recording
/// is gated by [`HistoryPolicy`]: a global switch plus per-Utility overrides,
/// with a per-Utility default supplied by the Registry.
pub struct HistoryRecorder {
    store: HistoryStore,
    clock: Box<dyn HistoryClock>,
    policy: RefCell<HistoryPolicy>,
    persist: Option<PolicyPersister>,
    subscribers: Rc<RefCell<HistorySubscribers>>,
}

/// Global and per-Utility recording preferences. Disabling recording never
/// deletes existing entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryPolicy {
    pub global_enabled: bool,
    #[serde(default)]
    pub per_utility: BTreeMap<String, bool>,
    /// Registry-provided defaults, keyed by Utility id. JWT opts out.
    #[serde(default)]
    pub defaults: BTreeMap<String, bool>,
}

impl Default for HistoryPolicy {
    fn default() -> Self {
        Self {
            global_enabled: true,
            per_utility: BTreeMap::new(),
            defaults: BTreeMap::new(),
        }
    }
}

impl HistoryPolicy {
    pub fn is_recording(&self, utility_id: &str) -> bool {
        if !self.global_enabled {
            return false;
        }
        self.per_utility
            .get(utility_id)
            .or_else(|| self.defaults.get(utility_id))
            .copied()
            .unwrap_or(true)
    }
}

impl HistoryRecorder {
    pub fn new(store: HistoryStore, clock: Box<dyn HistoryClock>) -> Self {
        Self::with_policy(store, clock, HistoryPolicy::default(), None)
    }

    pub fn with_policy(
        store: HistoryStore,
        clock: Box<dyn HistoryClock>,
        policy: HistoryPolicy,
        persist: Option<PolicyPersister>,
    ) -> Self {
        Self {
            store,
            clock,
            policy: RefCell::new(policy),
            persist,
            subscribers: Rc::new(RefCell::new(HistorySubscribers::default())),
        }
    }

    pub fn store(&self) -> &HistoryStore {
        &self.store
    }

    pub fn policy(&self) -> HistoryPolicy {
        self.policy.borrow().clone()
    }

    pub fn is_recording(&self, utility_id: &str) -> bool {
        self.policy.borrow().is_recording(utility_id)
    }

    pub fn set_global_enabled(&self, enabled: bool) {
        self.policy.borrow_mut().global_enabled = enabled;
        self.persist();
    }

    pub fn set_utility_enabled(&self, utility_id: &str, enabled: bool) {
        self.policy
            .borrow_mut()
            .per_utility
            .insert(utility_id.to_owned(), enabled);
        self.persist();
    }

    pub fn load(&self, utility_id: &str) -> Result<Vec<HistoryEntry>, HistoryError> {
        self.store.load(utility_id)
    }

    /// Registers a live workspace for storage changes to its Utility. The
    /// callback runs synchronously after Settings mutation, including failure
    /// paths, so the view can reconcile its cached rows and restore selection.
    pub fn subscribe(
        &self,
        utility_id: &str,
        callback: impl Fn(&mut App) + 'static,
    ) -> HistorySubscription {
        let mut subscribers = self.subscribers.borrow_mut();
        let id = subscribers.next_id;
        subscribers.next_id += 1;
        subscribers.callbacks.insert(
            id,
            (
                SubscriberKind::Utility(utility_id.to_owned()),
                Rc::new(callback),
            ),
        );
        HistorySubscription {
            id,
            subscribers: Rc::downgrade(&self.subscribers),
        }
    }

    /// Observes recording status changes initiated by workspaces. Settings
    /// uses this to redraw its paused badges without waiting for window focus.
    pub fn subscribe_status(&self, callback: impl Fn(&mut App) + 'static) -> HistorySubscription {
        let mut subscribers = self.subscribers.borrow_mut();
        let id = subscribers.next_id;
        subscribers.next_id += 1;
        subscribers
            .callbacks
            .insert(id, (SubscriberKind::Status, Rc::new(callback)));
        HistorySubscription {
            id,
            subscribers: Rc::downgrade(&self.subscribers),
        }
    }

    fn notify(&self, utility_id: Option<&str>, cx: &mut App) {
        let callbacks: Vec<ChangeCallback> = self
            .subscribers
            .borrow()
            .callbacks
            .values()
            .filter(|(kind, _)| match kind {
                SubscriberKind::Utility(id) => utility_id.is_none_or(|changed| id == changed),
                SubscriberKind::Status => false,
            })
            .map(|(_, callback)| Rc::clone(callback))
            .collect();
        for callback in callbacks {
            callback(cx);
        }
    }

    pub fn notify_status(&self, cx: &mut App) {
        let callbacks: Vec<ChangeCallback> = self
            .subscribers
            .borrow()
            .callbacks
            .values()
            .filter(|(kind, _)| matches!(kind, SubscriberKind::Status))
            .map(|(_, callback)| Rc::clone(callback))
            .collect();
        for callback in callbacks {
            callback(cx);
        }
    }

    pub fn clear_utility(&self, utility_id: &str, cx: &mut App) -> Result<(), HistoryError> {
        let result = self.store.clear(utility_id);
        self.notify(Some(utility_id), cx);
        result
    }

    pub fn clear_all(&self, cx: &mut App) -> Result<ClearReport, HistoryError> {
        let report = self.store.clear_all_report();
        self.notify(None, cx);
        report
    }

    pub fn retry_recording(
        &self,
        utility_id: &str,
        cx: &mut App,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        let result = self.store.retry(utility_id);
        self.notify(Some(utility_id), cx);
        result
    }

    fn persist(&self) {
        if let Some(persist) = &self.persist {
            persist(&self.policy.borrow());
        }
    }

    /// Records one settled operation with a fresh id and timestamp. Recording a
    /// disabled Utility is a no-op that returns the retained entries unchanged.
    pub fn record(
        &self,
        utility_id: &str,
        snapshot_version: u32,
        payload: serde_json::Value,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
        if !self.is_recording(utility_id) {
            return self.store.load(utility_id);
        }
        if self.store.is_paused(utility_id) {
            return Err(HistoryError::Paused);
        }
        self.store.record(HistoryEntry {
            id: self.clock.next_id(),
            captured_at: self.clock.captured_at(),
            utility_id: utility_id.to_owned(),
            snapshot_version,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sofdevtool_core::json::{Json, JsonMode, JsonRequest};
    use sofdevtool_core::session::{Session, SubmitOutcome};
    use sofdevtool_core::utility::Utility;

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sofdevtool-history-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ))
    }

    fn entry(utility_id: &str, id: &str, captured_at: &str, version: u32) -> HistoryEntry {
        HistoryEntry {
            id: id.to_owned(),
            captured_at: captured_at.to_owned(),
            utility_id: utility_id.to_owned(),
            snapshot_version: version,
            payload: serde_json::json!({ "id": id }),
        }
    }

    #[test]
    fn a_recorded_operation_round_trips_exactly() {
        let root = temporary_root("round-trip");
        let store = HistoryStore::new(root.clone());
        let record = entry("json", "a", "2026-01-01T00:00:01Z", 1);

        let retained = store.record(record.clone()).expect("record");

        assert_eq!(retained, vec![record.clone()]);
        assert_eq!(store.load("json").expect("load"), vec![record]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retention_keeps_the_newest_twenty_five_with_stable_tie_order() {
        let root = temporary_root("retention");
        let store = HistoryStore::new(root.clone());
        for index in 0..30u32 {
            let mut record = entry("json", &format!("{index:02}"), "2026-01-01T00:00:00Z", 1);
            // Same timestamp for every entry exercises the id tie-break.
            record.payload = serde_json::json!({ "index": index });
            store.record(record).expect("record");
        }

        let entries = store.load("json").expect("load");
        assert_eq!(entries.len(), RETENTION);
        // Newest first, tie-broken by descending id.
        assert_eq!(entries[0].id, "29");
        assert_eq!(entries[RETENTION - 1].id, "05");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_files_are_isolated_and_never_overwritten() {
        let root = temporary_root("corrupt");
        fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("json{FILE_SUFFIX}"));
        fs::write(&path, b"not json").unwrap();
        let store = HistoryStore::new(root.clone());

        assert_eq!(
            store.record(entry("json", "a", "t", 1)),
            Err(HistoryError::Corrupt)
        );
        assert_eq!(fs::read(&path).unwrap(), b"not json");
        assert!(store.is_paused("json"));
        // Another Utility is unaffected.
        assert!(store.record(entry("base64", "a", "t", 1)).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_write_keeps_the_last_valid_file_and_pauses_recording() {
        let root = temporary_root("readonly");
        let store = HistoryStore::new(root.clone());
        store
            .record(entry("json", "a", "2026-01-01T00:00:00Z", 1))
            .expect("first record");
        // Make the directory read-only so the next atomic replace fails.
        let mut permissions = fs::metadata(&root).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&root, permissions).unwrap();

        let failed = store.record(entry("json", "b", "2026-01-01T00:00:02Z", 1));

        let mut permissions = fs::metadata(&root).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&root, permissions).unwrap();

        assert_eq!(failed, Err(HistoryError::Unavailable));
        assert!(store.is_paused("json"));
        assert_eq!(store.load("json").expect("load").len(), 1);
        store
            .retry("json")
            .expect("retry writes unchanged retained set");
        assert!(!store.is_paused("json"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_write_blocks_ordinary_recording_until_successful_retry() {
        let root = temporary_root("paused-retry");
        let store = HistoryStore::new(root.clone());
        let original = entry("json", "original", "2026-01-01T00:00:00Z", 1);
        store.record(original.clone()).expect("initial record");
        let path = root.join(format!("json{FILE_SUFFIX}"));
        let before_failure = fs::read(&path).expect("last valid file");
        store.fail_next_writes(2);

        assert_eq!(
            store.record(entry("json", "failed", "2026-01-01T00:00:01Z", 1)),
            Err(HistoryError::Unavailable)
        );
        assert_eq!(fs::read(&path).unwrap(), before_failure);
        assert_eq!(
            store.record(entry("json", "ordinary", "2026-01-01T00:00:02Z", 1)),
            Err(HistoryError::Paused)
        );
        assert_eq!(store.retry("json"), Err(HistoryError::Unavailable));
        assert!(store.is_paused("json"));
        assert_eq!(fs::read(&path).unwrap(), before_failure);

        assert_eq!(store.retry("json"), Ok(vec![original.clone()]));
        assert!(!store.is_paused("json"));
        assert_eq!(store.load("json"), Ok(vec![original]));
        assert_eq!(
            store
                .record(entry("json", "later", "2026-01-01T00:00:03Z", 1))
                .expect("record after recovery")
                .len(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_storage_stays_untouched_on_retry_and_relaunch() {
        let root = temporary_root("corrupt-retry");
        fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("json{FILE_SUFFIX}"));
        fs::write(&path, b"malformed payload").unwrap();
        let store = HistoryStore::new(root.clone());
        assert_eq!(
            store.record(entry("json", "a", "t", 1)),
            Err(HistoryError::Corrupt)
        );
        assert_eq!(store.retry("json"), Err(HistoryError::Corrupt));
        assert!(store.is_paused("json"));
        assert_eq!(fs::read(&path).unwrap(), b"malformed payload");

        let relaunched = HistoryStore::new(root.clone());
        assert!(!relaunched.is_paused("json"));
        assert_eq!(relaunched.load("json"), Err(HistoryError::Corrupt));
        assert_eq!(
            relaunched.record(entry("json", "b", "t", 1)),
            Err(HistoryError::Corrupt)
        );
        assert_eq!(fs::read(&path).unwrap(), b"malformed payload");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn partial_clear_reports_actual_files_and_preserves_failed_target() {
        let root = temporary_root("partial-clear");
        let store = HistoryStore::new(root.clone());
        store.record(entry("json", "a", "t", 1)).unwrap();
        store.record(entry("base64", "b", "t", 1)).unwrap();
        // A directory with the History suffix simulates one deterministic
        // removal failure while the other files are independently removable.
        let failed = root.join(format!("unknown{FILE_SUFFIX}"));
        fs::create_dir(&failed).unwrap();
        let report = store.clear_all_report().expect("enumeration succeeded");
        assert_eq!(report.cleared_ids, vec!["base64", "json"]);
        assert_eq!(report.failed_ids, vec!["unknown"]);
        assert!(store.load("json").unwrap().is_empty());
        assert!(store.load("base64").unwrap().is_empty());
        assert!(failed.is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shared_view_reconcile_invalidates_removed_selection_and_pending_restore() {
        let root = temporary_root("view-reconcile");
        let recorder = HistoryRecorder::new(HistoryStore::new(root.clone()), Box::new(FixedClock));
        let retained = recorder
            .store()
            .record(entry("json", "one", "t", 99))
            .unwrap()[0]
            .clone();
        recorder
            .store()
            .record(entry("base64", "two", "t", 1))
            .unwrap();
        let mut json = HistoryViewState::load(&recorder, "json");
        let mut base64 = HistoryViewState::load(&recorder, "base64");
        assert!(json.select("one"));
        json.pending_restore = Some(retained.clone());
        assert!(json.retained(&recorder, "json", &retained));
        assert!(!json.select("missing"));

        recorder.store().clear("json").unwrap();
        json.reconcile(&recorder, "json");
        base64.reconcile(&recorder, "base64");
        assert!(json.entries.is_empty());
        assert!(json.selected.is_none());
        assert!(json.pending_restore.is_none());
        assert!(!json.retained(&recorder, "json", &retained));
        assert_eq!(base64.entries.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_set_change_invalidates_an_evicted_restore_target() {
        let root = temporary_root("view-retention");
        let recorder = HistoryRecorder::new(HistoryStore::new(root.clone()), Box::new(FixedClock));
        let old = entry("json", "old", "2026-01-01T00:00:00Z", 1);
        recorder.store().record(old.clone()).unwrap();
        let mut view = HistoryViewState::load(&recorder, "json");
        assert!(view.select("old"));
        view.pending_restore = Some(old);
        for index in 0..RETENTION {
            let retained = recorder
                .store()
                .record(entry(
                    "json",
                    &format!("new-{index:02}"),
                    "2026-01-02T00:00:00Z",
                    1,
                ))
                .unwrap();
            view.apply_record(&recorder, "json", Ok(retained));
        }
        assert_eq!(view.entries.len(), RETENTION);
        assert!(view.selected.is_none());
        assert!(view.pending_restore.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_snapshots_remain_listed_as_stored_entries() {
        let root = temporary_root("unknown-snapshot");
        let store = HistoryStore::new(root.clone());
        store
            .record(entry("json", "a", "2026-01-01T00:00:00Z", 99))
            .expect("record");
        let entries = store.load("json").expect("load");
        assert_eq!(entries[0].snapshot_version, 99);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsafe_utility_identities_are_rejected() {
        let root = temporary_root("unsafe");
        let store = HistoryStore::new(root.clone());
        assert_eq!(store.load("../swift"), Err(HistoryError::InvalidUtility));
        assert_eq!(store.load(""), Err(HistoryError::InvalidUtility));
        assert_eq!(store.load("JSON"), Err(HistoryError::InvalidUtility));
    }

    #[test]
    fn clear_only_removes_this_root_and_lists_unknown_utilities() {
        let root = temporary_root("clear");
        let store = HistoryStore::new(root.clone());
        store.record(entry("json", "a", "t", 1)).expect("json");
        store
            .record(entry("legacy-unknown", "b", "t", 1))
            .expect("unknown");

        assert_eq!(
            store.stored_utility_ids().expect("ids"),
            vec!["json".to_owned(), "legacy-unknown".to_owned()]
        );
        store.clear("json").expect("clear json");
        assert!(store.load("json").expect("load").is_empty());
        assert_eq!(
            store.stored_utility_ids().expect("ids"),
            vec!["legacy-unknown".to_owned()]
        );

        store.clear_all().expect("clear all");
        assert!(store.stored_utility_ids().expect("ids").is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clear_all_removes_legacy_filenames_outside_current_utility_id_rules() {
        let root = temporary_root("legacy-clear");
        let store = HistoryStore::new(root.clone());
        store.record(entry("json", "known", "t", 1)).unwrap();
        let legacy = root.join(format!("LegacyUtility{FILE_SUFFIX}"));
        fs::write(&legacy, b"opaque legacy bytes").unwrap();
        assert_eq!(
            store.load("LegacyUtility"),
            Err(HistoryError::InvalidUtility)
        );
        assert_eq!(
            store.stored_utility_ids().unwrap(),
            vec!["LegacyUtility", "json"]
        );
        let report = store.clear_all_report().unwrap();
        assert_eq!(report.cleared_ids, vec!["LegacyUtility", "json"]);
        assert!(report.failed_ids.is_empty());
        assert!(!legacy.exists());
        assert!(store.stored_utility_ids().unwrap().is_empty());
        fs::write(&legacy, b"opaque legacy bytes").unwrap();
        store
            .clear("LegacyUtility")
            .expect("single unknown Utility clear");
        assert!(!legacy.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn entries_for_another_utility_in_the_same_file_are_corrupt() {
        let root = temporary_root("mismatch");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(format!("json{FILE_SUFFIX}")),
            br#"{"version":1,"utility_id":"json","entries":[{"id":"x","captured_at":"t","utility_id":"base64","snapshot_version":1,"payload":null}]}"#,
        )
        .unwrap();
        let store = HistoryStore::new(root.clone());
        assert_eq!(store.load("json"), Err(HistoryError::Corrupt));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn iso8601_formatting_is_fixed_width_and_lexicographically_ordered() {
        assert_eq!(format_iso8601_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_iso8601_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_iso8601_utc(1_700_000_000), "2023-11-14T22:13:20Z");
        assert!(format_iso8601_utc(9) < format_iso8601_utc(10));
    }

    struct FixedClock;

    impl HistoryClock for FixedClock {
        fn captured_at(&self) -> String {
            "2026-01-01T00:00:00Z".to_owned()
        }
        fn next_id(&self) -> String {
            "fixed".to_owned()
        }
    }

    #[test]
    fn recorder_stamps_entries_from_its_injected_clock() {
        let root = temporary_root("recorder");
        let recorder = HistoryRecorder::new(HistoryStore::new(root.clone()), Box::new(FixedClock));

        let entries = recorder
            .record("json", 1, serde_json::json!({ "output": "{}" }))
            .expect("record");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "fixed");
        assert_eq!(entries[0].captured_at, "2026-01-01T00:00:00Z");
        assert_eq!(entries[0].snapshot_version, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recorder_pause_keeps_live_result_and_does_not_duplicate_on_retry() {
        let root = temporary_root("session-pause");
        let recorder = HistoryRecorder::new(HistoryStore::new(root.clone()), Box::new(FixedClock));
        let mut session = Session::<Json>::new();
        let first = JsonRequest::new(r#"{"first":1}"#, JsonMode::Format);
        let SubmitOutcome::Scheduled(first_revision) = session.submit(first) else {
            panic!("first operation");
        };
        session.resolve(first_revision).unwrap();
        let first_payload = serde_json::to_value(session.take_snapshot().unwrap()).unwrap();
        recorder
            .record(Json::ID, Json::SNAPSHOT_VERSION, first_payload)
            .unwrap();

        recorder.store().fail_next_writes(1);
        let second = JsonRequest::new(r#"{"second":2}"#, JsonMode::Format);
        let SubmitOutcome::Scheduled(second_revision) = session.submit(second) else {
            panic!("second operation");
        };
        session.resolve(second_revision).unwrap();
        let visible = session.evaluation().clone();
        let second_payload = serde_json::to_value(session.take_snapshot().unwrap()).unwrap();
        assert_eq!(
            recorder.record(Json::ID, Json::SNAPSHOT_VERSION, second_payload),
            Err(HistoryError::Unavailable)
        );
        assert_eq!(session.evaluation(), &visible);
        assert!(recorder.store().is_paused(Json::ID));
        assert_eq!(recorder.load(Json::ID).unwrap().len(), 1);

        let third = JsonRequest::new(r#"{"third":3}"#, JsonMode::Format);
        let SubmitOutcome::Scheduled(third_revision) = session.submit(third) else {
            panic!("third operation");
        };
        session.resolve(third_revision).unwrap();
        let third_payload = serde_json::to_value(session.take_snapshot().unwrap()).unwrap();
        assert_eq!(
            recorder.record(Json::ID, Json::SNAPSHOT_VERSION, third_payload),
            Err(HistoryError::Paused)
        );
        assert_eq!(recorder.load(Json::ID).unwrap().len(), 1);
        assert_eq!(recorder.store().retry(Json::ID).unwrap().len(), 1);
        assert!(!recorder.store().is_paused(Json::ID));
        assert_eq!(recorder.load(Json::ID).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn global_and_utility_recording_preferences_remain_independent() {
        let root = temporary_root("policy");
        let mut policy = HistoryPolicy::default();
        policy.defaults.insert("jwt".into(), false);
        let recorder = HistoryRecorder::with_policy(
            HistoryStore::new(root.clone()),
            Box::new(FixedClock),
            policy,
            None,
        );
        assert!(!recorder.is_recording("jwt"));
        recorder.record("jwt", 1, serde_json::Value::Null).unwrap();
        assert!(recorder.load("jwt").unwrap().is_empty());

        recorder.set_global_enabled(false);
        recorder.set_utility_enabled("json", true);
        assert!(!recorder.is_recording("json"));
        recorder.record("json", 1, serde_json::Value::Null).unwrap();
        assert!(recorder.load("json").unwrap().is_empty());
        recorder.set_global_enabled(true);
        assert!(recorder.is_recording("json"));
        assert!(!recorder.is_recording("jwt"));
        recorder.set_utility_enabled("jwt", true);
        assert!(recorder.is_recording("jwt"));
        recorder.record("jwt", 1, serde_json::Value::Null).unwrap();
        assert_eq!(recorder.load("jwt").unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
