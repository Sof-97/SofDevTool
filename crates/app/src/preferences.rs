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
use crate::utilities::random_string::RandomStringControls;

const SHORTCUT_FILE: &str = "launcher-shortcut.v1.json";
const SHORTCUT_SCHEMA_VERSION: u8 = 1;
const HISTORY_FILE: &str = "history-preferences.v1.json";
const HISTORY_SCHEMA_VERSION: u8 = 1;
const WORKSPACE_FILE: &str = "workspace-preferences.v1.json";
const WORKSPACE_SCHEMA_VERSION: u8 = 1;
const HISTORY_LAYOUT_FILE: &str = "history-layout.v1.json";
const HISTORY_LAYOUT_SCHEMA_VERSION: u8 = 1;
const RANDOM_STRING_CONTROLS_FILE: &str = "random-string-controls.v1.json";
const RANDOM_STRING_CONTROLS_SCHEMA_VERSION: u8 = 1;

/// Fresh Rust-only persistence for the Workbench catalog and theme.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePreferencesData {
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub recents: Vec<String>,
}

impl Default for WorkspacePreferencesData {
    fn default() -> Self {
        Self {
            // Fresh profiles start in System appearance. The stored value is an
            // appearance mode (`system`/`light`/`dark`); retired Graphite and
            // Catppuccin values are mapped to Dark when read.
            theme: crate::appearance::AppearanceMode::System
                .stored_value()
                .to_owned(),
            favorites: Vec::new(),
            recents: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspacePreferences {
    root: PathBuf,
}

impl WorkspacePreferences {
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

    /// Loads the stored preferences. Missing or malformed data yields safe
    /// defaults (System appearance, no favorites, no recents).
    pub fn load(&self) -> WorkspacePreferencesData {
        let Ok(contents) = fs::read_to_string(self.path()) else {
            return WorkspacePreferencesData::default();
        };
        let Ok(record) = serde_json::from_str::<WorkspaceRecord>(&contents) else {
            return WorkspacePreferencesData::default();
        };
        if record.version != WORKSPACE_SCHEMA_VERSION {
            return WorkspacePreferencesData::default();
        }
        record.data
    }

    pub fn save(&self, data: &WorkspacePreferencesData) -> Result<(), PreferenceError> {
        fs::create_dir_all(&self.root).map_err(PreferenceError::CreateDirectory)?;
        let record = WorkspaceRecord {
            version: WORKSPACE_SCHEMA_VERSION,
            data: data.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&record).map_err(PreferenceError::Encode)?;
        let path = self.path();
        let temporary = self
            .root
            .join(format!(".{WORKSPACE_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, serialized).map_err(PreferenceError::Write)?;
        fs::rename(&temporary, &path).map_err(PreferenceError::Replace)
    }

    fn path(&self) -> PathBuf {
        self.root.join(WORKSPACE_FILE)
    }
}

#[derive(Deserialize, Serialize)]
struct WorkspaceRecord {
    version: u8,
    data: WorkspacePreferencesData,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryLayoutPreferences {
    root: PathBuf,
}

#[derive(Deserialize, Serialize)]
struct HistoryLayoutRecord {
    version: u8,
    visible: std::collections::BTreeMap<String, bool>,
}

impl HistoryLayoutPreferences {
    pub fn application_support() -> Result<Self, PreferenceError> {
        identity::application_support_root()
            .map(Self::new)
            .ok_or(PreferenceError::UnavailableRoot)
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn load(&self) -> std::collections::BTreeMap<String, bool> {
        let Ok(contents) = fs::read_to_string(self.root.join(HISTORY_LAYOUT_FILE)) else {
            return Default::default();
        };
        let Ok(record) = serde_json::from_str::<HistoryLayoutRecord>(&contents) else {
            return Default::default();
        };
        if record.version != HISTORY_LAYOUT_SCHEMA_VERSION {
            return Default::default();
        }
        record.visible
    }

    pub fn save(
        &self,
        visible: &std::collections::BTreeMap<String, bool>,
    ) -> Result<(), PreferenceError> {
        fs::create_dir_all(&self.root).map_err(PreferenceError::CreateDirectory)?;
        let serialized = serde_json::to_vec_pretty(&HistoryLayoutRecord {
            version: HISTORY_LAYOUT_SCHEMA_VERSION,
            visible: visible.clone(),
        })
        .map_err(PreferenceError::Encode)?;
        let temporary = self
            .root
            .join(format!(".{HISTORY_LAYOUT_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, serialized).map_err(PreferenceError::Write)?;
        fs::rename(&temporary, self.root.join(HISTORY_LAYOUT_FILE))
            .map_err(PreferenceError::Replace)
    }
}

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

/// Fresh Rust-only persistence for the Random String Utility's saved controls.
///
/// Ordinary preferences carry the control configuration only: generated
/// values and generation bookkeeping never appear here, and loading never
/// generates output or records History. Missing data is a neutral first-run
/// state; malformed, version-invalid or out-of-range data falls back to the
/// defaults with an honest diagnostic and is left untouched on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomStringControlsPreferences {
    root: PathBuf,
}

impl RandomStringControlsPreferences {
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

    /// Loads the stored controls for startup, converting every unusable state
    /// into the defaults plus a readable diagnostic.
    pub fn load_for_startup(&self) -> RandomStringControlsStartup {
        match self.load() {
            Ok(controls) => RandomStringControlsStartup {
                controls,
                diagnostic: None,
            },
            Err(error) => RandomStringControlsStartup {
                controls: RandomStringControls::default(),
                diagnostic: Some(format!(
                    "Random String controls could not be loaded; using the defaults for this launch because {error}"
                )),
            },
        }
    }

    /// Loads the stored controls. A missing file yields the defaults; a
    /// malformed, unsupported-version or out-of-range file is an error and is
    /// never silently overwritten.
    pub fn load(&self) -> Result<RandomStringControls, PreferenceError> {
        let contents = match fs::read_to_string(self.path()) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(RandomStringControls::default());
            }
            Err(error) => return Err(PreferenceError::Read(error)),
        };
        let record: RandomStringControlsRecord =
            serde_json::from_str(&contents).map_err(PreferenceError::Decode)?;
        if record.version != RANDOM_STRING_CONTROLS_SCHEMA_VERSION {
            return Err(PreferenceError::UnsupportedVersion(record.version));
        }
        if !record.controls.is_supported() {
            return Err(PreferenceError::InvalidControls);
        }
        Ok(record.controls)
    }

    /// Persists the latest controls atomically in the same directory. A failed
    /// write leaves any previously valid file in place and is reported, never
    /// presented as a successful save.
    pub fn save(&self, controls: &RandomStringControls) -> Result<(), PreferenceError> {
        if !controls.is_supported() {
            return Err(PreferenceError::InvalidControls);
        }
        fs::create_dir_all(&self.root).map_err(PreferenceError::CreateDirectory)?;
        let record = RandomStringControlsRecord {
            version: RANDOM_STRING_CONTROLS_SCHEMA_VERSION,
            controls: controls.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&record).map_err(PreferenceError::Encode)?;
        let path = self.path();
        let temporary = self.root.join(format!(
            ".{RANDOM_STRING_CONTROLS_FILE}.{}.tmp",
            std::process::id()
        ));
        fs::write(&temporary, serialized).map_err(PreferenceError::Write)?;
        fs::rename(&temporary, &path).map_err(PreferenceError::Replace)
    }

    fn path(&self) -> PathBuf {
        self.root.join(RANDOM_STRING_CONTROLS_FILE)
    }
}

/// The startup outcome for Random String controls: the configuration to apply
/// (stored or default) and an honest diagnostic when stored data was unusable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomStringControlsStartup {
    pub controls: RandomStringControls,
    pub diagnostic: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct RandomStringControlsRecord {
    version: u8,
    controls: RandomStringControls,
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
    /// The stored control configuration is outside the supported ranges.
    InvalidControls,
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
            Self::InvalidControls => {
                write!(
                    formatter,
                    "it declares controls outside the supported ranges"
                )
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

    #[test]
    fn random_string_controls_round_trip_and_default_safely() {
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-rust-rs-controls-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let preferences = RandomStringControlsPreferences::new(root.clone());

        // A missing file is the neutral first-run state: defaults, no diagnostic.
        let startup = preferences.load_for_startup();
        assert_eq!(startup.controls, RandomStringControls::default());
        assert_eq!(startup.diagnostic, None);

        let controls = RandomStringControls {
            length: 64,
            count: 3,
            uppercase: false,
            lowercase: true,
            digits: false,
            symbols: true,
            exclude_ambiguous: false,
            custom_alphabet: "αβ".to_owned(),
        };
        preferences.save(&controls).expect("save controls");
        let startup = preferences.load_for_startup();
        assert_eq!(startup.controls, controls);
        assert_eq!(startup.diagnostic, None);

        // Malformed data falls back to defaults with a diagnostic, untouched.
        fs::write(root.join(RANDOM_STRING_CONTROLS_FILE), "not json")
            .expect("write malformed data");
        let startup = preferences.load_for_startup();
        assert_eq!(startup.controls, RandomStringControls::default());
        assert!(startup
            .diagnostic
            .expect("diagnostic")
            .contains("could not be loaded"));
        assert_eq!(
            fs::read_to_string(root.join(RANDOM_STRING_CONTROLS_FILE))
                .expect("malformed file remains"),
            "not json"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn workspace_preferences_round_trip_and_default_safely() {
        let root = std::env::temp_dir().join(format!(
            "sofdevtool-rust-workspace-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let preferences = WorkspacePreferences::new(root.clone());

        // Missing data yields safe defaults (System appearance, no favorites/recents).
        assert_eq!(preferences.load(), WorkspacePreferencesData::default());

        let data = WorkspacePreferencesData {
            theme: "catppuccin".to_owned(),
            favorites: vec!["json".to_owned()],
            recents: vec!["base64".to_owned(), "json".to_owned()],
        };
        preferences.save(&data).expect("save workspace preferences");
        assert_eq!(preferences.load(), data);

        // Malformed data falls back to defaults rather than failing startup.
        fs::write(preferences.path(), "not json").expect("write malformed data");
        assert_eq!(preferences.load(), WorkspacePreferencesData::default());
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
