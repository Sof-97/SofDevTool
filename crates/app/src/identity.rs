//! Application identity and build information shared by the native process and packaging.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildProfile {
    Debug,
    Release,
}

impl BuildProfile {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Debug => "SofDevTool Debug",
            Self::Release => "SofDevTool",
        }
    }

    pub const fn bundle_identifier(self) -> &'static str {
        match self {
            Self::Debug => "com.gerardocalia.sofdevtool.debug",
            Self::Release => "com.gerardocalia.sofdevtool",
        }
    }

    pub const fn channel(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    pub fn support_root(self, home: &Path) -> PathBuf {
        home.join("Library")
            .join("Application Support")
            .join(self.bundle_identifier())
    }
}

/// Rust's build profile selects the matching native bundle and data identity.
pub const PROFILE: BuildProfile = if cfg!(debug_assertions) {
    BuildProfile::Debug
} else {
    BuildProfile::Release
};
pub const APP_DISPLAY_NAME: &str = PROFILE.display_name();
pub const BUNDLE_IDENTIFIER: &str = PROFILE.bundle_identifier();
pub const PREFERENCE_DOMAIN: &str = PROFILE.bundle_identifier();
pub const APPLICATION_SUPPORT_NAMESPACE: &str = PROFILE.bundle_identifier();

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_ID: &str = env!("SOFDEVTOOL_BUILD_ID");
pub const REVISION: &str = env!("SOFDEVTOOL_REVISION");
pub const SOURCE_STATE: &str = env!("SOFDEVTOOL_SOURCE_STATE");

pub fn build_description() -> String {
    format!(
        "{} {} ({}, {}, {}, {})",
        APP_DISPLAY_NAME,
        VERSION,
        BUILD_ID,
        PROFILE.channel(),
        REVISION,
        SOURCE_STATE
    )
}

/// An explicit test override is a complete, caller-owned data root. Native
/// profile tests must pass separate override roots for Debug and Release.
pub fn application_support_root() -> Option<PathBuf> {
    if let Some(override_root) = std::env::var_os("SOFDEVTOOL_RUST_SUPPORT_ROOT") {
        return Some(PathBuf::from(override_root));
    }
    std::env::var_os("HOME").map(|home| PROFILE.support_root(Path::new(&home)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_have_distinct_bundle_preference_and_data_identities() {
        let debug = BuildProfile::Debug;
        let release = BuildProfile::Release;
        assert_eq!(debug.display_name(), "SofDevTool Debug");
        assert_eq!(release.display_name(), "SofDevTool");
        assert_ne!(debug.bundle_identifier(), release.bundle_identifier());
        assert_ne!(
            debug.support_root(Path::new("/test-home")),
            release.support_root(Path::new("/test-home"))
        );
        assert_ne!(
            release.support_root(Path::new("/test-home")),
            PathBuf::from("/test-home/Library/Application Support/SofDevTool")
        );
        assert_eq!(PREFERENCE_DOMAIN, BUNDLE_IDENTIFIER);
    }

    #[test]
    fn built_identity_matches_the_cargo_profile_and_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(APP_DISPLAY_NAME, PROFILE.display_name());
        assert_eq!(BUNDLE_IDENTIFIER, PROFILE.bundle_identifier());
        assert!(matches!(SOURCE_STATE, "clean" | "dirty" | "unknown"));
        assert!(build_description().contains(VERSION));
        assert!(build_description().contains(BUILD_ID));
        assert!(build_description().contains(REVISION));
        assert!(build_description().contains(PROFILE.channel()));
        if std::env::var_os("SOFDEVTOOL_EXPECT_UNKNOWN_METADATA").is_some() {
            assert_eq!(REVISION, "unknown");
            assert_eq!(SOURCE_STATE, "unknown");
        }
    }

    #[test]
    fn profile_roots_isolate_history_and_saved_controls_without_touching_legacy_data() {
        use crate::history::{HistoryEntry, HistoryStore};
        use crate::preferences::{
            RandomStringControlsPreferences, WorkspacePreferences, WorkspacePreferencesData,
        };

        let home = std::env::temp_dir().join(format!(
            "sofdevtool-profiles-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let debug = BuildProfile::Debug.support_root(&home);
        let release = BuildProfile::Release.support_root(&home);
        let legacy = home.join("Library/Application Support/SofDevTool");
        std::fs::create_dir_all(&legacy).expect("legacy fixture root");
        std::fs::write(legacy.join("sentinel"), b"untouched").expect("legacy fixture");

        let release_workspace = WorkspacePreferences::new(release.clone());
        let debug_workspace = WorkspacePreferences::new(debug.clone());
        let selected = WorkspacePreferencesData {
            theme: "catppuccin".into(),
            favorites: vec!["random-string".into()],
            recents: vec!["json".into()],
        };
        release_workspace
            .save(&selected)
            .expect("release workspace save");
        assert_eq!(release_workspace.load(), selected);
        assert_eq!(debug_workspace.load(), WorkspacePreferencesData::default());

        let release_controls = RandomStringControlsPreferences::new(release.clone());
        let debug_controls = RandomStringControlsPreferences::new(debug.clone());
        let mut controls = release_controls.load().expect("default controls");
        controls.length = 42;
        release_controls
            .save(&controls)
            .expect("release controls save");
        assert_eq!(
            release_controls.load().expect("release controls load"),
            controls
        );
        assert_ne!(
            debug_controls.load().expect("debug controls load"),
            controls
        );

        let release_history = HistoryStore::new(release.join("History"));
        let debug_history = HistoryStore::new(debug.join("History"));
        release_history
            .record(HistoryEntry {
                id: "profile-1".into(),
                captured_at: "2026-09-23T00:00:00Z".into(),
                utility_id: "json".into(),
                snapshot_version: 1,
                payload: serde_json::json!({"input":"synthetic"}),
            })
            .expect("release history record");
        assert_eq!(
            release_history
                .load("json")
                .expect("release history load")
                .len(),
            1
        );
        assert!(debug_history
            .load("json")
            .expect("debug history load")
            .is_empty());
        assert_eq!(
            std::fs::read(legacy.join("sentinel")).expect("legacy data"),
            b"untouched"
        );
        std::fs::remove_dir_all(home).expect("remove isolated profile fixture");
    }
}
