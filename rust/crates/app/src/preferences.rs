//! Fresh Rust-only persistence for application preferences.
//!
//! This module deliberately stores only the Launcher chord under the Rust
//! Application Support namespace. It neither reads nor names Swift preference
//! files or domains.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::identity;
use crate::shortcut::Shortcut;

const SHORTCUT_FILE: &str = "launcher-shortcut.v1.json";
const SHORTCUT_SCHEMA_VERSION: u8 = 1;
const HISTORY_FILE: &str = "history-preferences.v1.json";
const HISTORY_SCHEMA_VERSION: u8 = 1;

/// Fresh Rust-only persistence for History recording preferences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPreferences {
    root: PathBuf,
}

impl HistoryPreferences {
    pub fn application_support() -> Result<Self, PreferenceError> {
        identity::application_support_root()
            .map(Self::new)
            .ok_or(PreferenceError::UnavailableRoot)
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Loads the stored policy. A missing or malformed file falls back to the
    /// safe default (recording enabled) rather than failing startup.
    pub fn load(&self) -> crate::history::HistoryPolicy {
        let Ok(contents) = fs::read_to_string(self.path()) else {
            return crate::history::HistoryPolicy::default();
        };
        let Ok(record) = serde_json::from_str::<HistoryRecord>(&contents) else {
            return crate::history::HistoryPolicy::default();
        };
        if record.version != HISTORY_SCHEMA_VERSION {
            return crate::history::HistoryPolicy::default();
        }
        record.policy
    }

    /// Replaces the policy atomically in the same directory.
    pub fn save(&self, policy: &crate::history::HistoryPolicy) -> Result<(), PreferenceError> {
        fs::create_dir_all(&self.root).map_err(PreferenceError::CreateDirectory)?;
        let record = HistoryRecord {
            version: HISTORY_SCHEMA_VERSION,
            policy: policy.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&record).map_err(PreferenceError::Encode)?;
        let path = self.path();
        let temporary = self
            .root
            .join(format!(".{HISTORY_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, serialized).map_err(PreferenceError::Write)?;
        fs::rename(&temporary, &path).map_err(PreferenceError::Replace)
    }

    fn path(&self) -> PathBuf {
        self.root.join(HISTORY_FILE)
    }
}

#[derive(Deserialize, Serialize)]
struct HistoryRecord {
    version: u8,
    policy: crate::history::HistoryPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortcutPreferences {
    root: PathBuf,
}

impl ShortcutPreferences {
    /// Uses the fresh Rust Application Support namespace. A missing home
    /// directory is represented as a diagnostic rather than silently falling
    /// back to any Swift location.
    pub fn application_support() -> Result<Self, PreferenceError> {
        identity::application_support_root()
            .map(Self::new)
            .ok_or(PreferenceError::UnavailableRoot)
    }

    /// Test and composition seam for a caller-owned Rust-only directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load_for_startup(&self) -> StartupShortcut {
        match self.load() {
            Ok(shortcut) => StartupShortcut {
                shortcut,
                diagnostic: None,
            },
            Err(error) => StartupShortcut {
                shortcut: None,
                diagnostic: Some(format!(
                    "Launcher shortcut preference could not be loaded; using the default for this launch. {error}"
                )),
            },
        }
    }

    pub fn load(&self) -> Result<Option<Shortcut>, PreferenceError> {
        let path = self.path();
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(PreferenceError::Read(error)),
        };
        let record: ShortcutRecord =
            serde_json::from_str(&contents).map_err(PreferenceError::Decode)?;
        if record.version != SHORTCUT_SCHEMA_VERSION {
            return Err(PreferenceError::UnsupportedVersion(record.version));
        }
        let shortcut = Shortcut::new(record.key_code, record.modifiers, record.display_name);
        shortcut
            .validate()
            .map_err(PreferenceError::InvalidShortcut)?;
        Ok(Some(shortcut))
    }

    /// Persists after a successful native registration. Replacement happens
    /// atomically in the same directory, leaving a prior valid preference in
    /// place when serialization or writing fails.
    pub fn save(&self, shortcut: &Shortcut) -> Result<(), PreferenceError> {
        shortcut
            .validate()
            .map_err(PreferenceError::InvalidShortcut)?;
        fs::create_dir_all(&self.root).map_err(PreferenceError::CreateDirectory)?;
        let record = ShortcutRecord {
            version: SHORTCUT_SCHEMA_VERSION,
            key_code: shortcut.key_code,
            modifiers: shortcut.modifiers,
            display_name: shortcut.display_name.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&record).map_err(PreferenceError::Encode)?;
        let path = self.path();
        let temporary = self
            .root
            .join(format!(".{SHORTCUT_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, serialized).map_err(PreferenceError::Write)?;
        fs::rename(&temporary, &path).map_err(PreferenceError::Replace)
    }

    fn path(&self) -> PathBuf {
        self.root.join(SHORTCUT_FILE)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupShortcut {
    pub shortcut: Option<Shortcut>,
    pub diagnostic: Option<String>,
}

#[derive(Debug)]
pub enum PreferenceError {
    UnavailableRoot,
    Read(io::Error),
    Decode(serde_json::Error),
    UnsupportedVersion(u8),
    InvalidShortcut(crate::shortcut::ShortcutError),
    CreateDirectory(io::Error),
    Encode(serde_json::Error),
    Write(io::Error),
    Replace(io::Error),
}

impl fmt::Display for PreferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnavailableRoot => write!(
                formatter,
                "the Rust Application Support directory is unavailable"
            ),
            Self::Read(error) => write!(formatter, "could not read it: {error}"),
            Self::Decode(error) => {
                write!(formatter, "it is not valid Rust preference data: {error}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "it uses unsupported schema version {version}")
            }
            Self::InvalidShortcut(error) => {
                write!(formatter, "it contains an invalid shortcut: {error}")
            }
            Self::CreateDirectory(error) => {
                write!(formatter, "could not create its directory: {error}")
            }
            Self::Encode(error) => write!(formatter, "could not encode it: {error}"),
            Self::Write(error) => write!(formatter, "could not write it: {error}"),
            Self::Replace(error) => write!(formatter, "could not replace it: {error}"),
        }
    }
}

impl std::error::Error for PreferenceError {}

#[derive(Deserialize, Serialize)]
struct ShortcutRecord {
    version: u8,
    key_code: u32,
    modifiers: u32,
    display_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut::{CONTROL, OPTION};

    fn temporary_preferences(name: &str) -> ShortcutPreferences {
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-rust-preferences-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        ShortcutPreferences::new(root)
    }

    #[test]
    fn a_saved_rust_shortcut_reloads_from_the_same_fresh_namespace() {
        let preferences = temporary_preferences("reload");
        let shortcut = Shortcut::new(0, CONTROL | OPTION, "⌃⌥A");

        preferences.save(&shortcut).expect("save shortcut");

        assert_eq!(preferences.load().expect("load shortcut"), Some(shortcut));
        fs::remove_dir_all(preferences.root()).expect("remove test directory");
    }

    #[test]
    fn malformed_data_becomes_a_startup_diagnostic_and_never_a_shortcut() {
        let preferences = temporary_preferences("malformed");
        fs::create_dir_all(preferences.root()).expect("create test directory");
        fs::write(preferences.path(), "not json").expect("write malformed data");

        let loaded = preferences.load_for_startup();

        assert_eq!(loaded.shortcut, None);
        assert!(loaded
            .diagnostic
            .expect("diagnostic")
            .contains("could not be loaded"));
        fs::remove_dir_all(preferences.root()).expect("remove test directory");
    }
}
