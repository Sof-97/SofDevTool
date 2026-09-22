//! Fresh, Rust-only History storage.
//!
//! One versioned JSON file per Utility under the Rust Application Support
//! namespace. Payloads are Utility-owned, opaque to this module and never
//! logged, indexed, synced or exported. Writes replace the file atomically and
//! a failed write keeps the last valid file, pauses that Utility and never
//! overwrites a malformed file.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtility => write!(formatter, "the Utility identity is not storable"),
            Self::Corrupt => write!(formatter, "the stored History file is not valid"),
            Self::Unavailable => write!(formatter, "the History file could not be read or written"),
        }
    }
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
}

impl HistoryStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            paused: Mutex::new(BTreeSet::new()),
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
        let utility_id = entry.utility_id.clone();
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
        let path = self.path(utility_id)?;
        match fs::remove_file(&path) {
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
        let ids = self.stored_utility_ids()?;
        for id in ids {
            let path = self.root.join(format!("{id}{FILE_SUFFIX}"));
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => return Err(HistoryError::Unavailable),
            }
        }
        if let Ok(mut paused) = self.paused.lock() {
            paused.clear();
        }
        Ok(())
    }

    /// Every Utility id that has a stored History file, including unknown ones.
    pub fn stored_utility_ids(&self) -> Result<Vec<String>, HistoryError> {
        let directory = match fs::read_dir(&self.root) {
            Ok(directory) => directory,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(HistoryError::Unavailable),
        };
        let mut ids = Vec::new();
        for entry in directory {
            let entry = entry.map_err(|_| HistoryError::Unavailable)?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if let Some(id) = name.strip_suffix(FILE_SUFFIX) {
                if !id.is_empty() {
                    ids.push(id.to_owned());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// True when a failed write paused recording for `utility_id`.
    pub fn is_paused(&self, utility_id: &str) -> bool {
        self.paused
            .lock()
            .map(|paused| paused.contains(utility_id))
            .unwrap_or(false)
    }

    /// Clears a pause after a successful retry. Relaunch also clears it.
    pub fn resume(&self, utility_id: &str) {
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
        fs::write(&temporary, serialized).map_err(|_| HistoryError::Unavailable)?;
        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(()),
            Err(_) => {
                let _ = fs::remove_file(&temporary);
                Err(HistoryError::Unavailable)
            }
        }
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

/// Application-owned recording facade over [`HistoryStore`].
///
/// A Utility records through this type; it owns the entry identity and
/// timestamp so the storage layer stays free of clock/random policy.
pub struct HistoryRecorder {
    store: HistoryStore,
    clock: Box<dyn HistoryClock>,
}

impl HistoryRecorder {
    pub fn new(store: HistoryStore, clock: Box<dyn HistoryClock>) -> Self {
        Self { store, clock }
    }

    pub fn store(&self) -> &HistoryStore {
        &self.store
    }

    pub fn load(&self, utility_id: &str) -> Result<Vec<HistoryEntry>, HistoryError> {
        self.store.load(utility_id)
    }

    /// Records one settled operation with a fresh id and timestamp.
    pub fn record(
        &self,
        utility_id: &str,
        snapshot_version: u32,
        payload: serde_json::Value,
    ) -> Result<Vec<HistoryEntry>, HistoryError> {
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
        store.resume("json");
        assert!(!store.is_paused("json"));
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
}
