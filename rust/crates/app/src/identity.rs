//! Fresh Rust identity and storage/preference roots.
//!
//! These deliberately differ from the Swift product's identity and
//! Application Support namespace so no Swift data is read, migrated or written.

use std::path::PathBuf;

/// Display name shown in the title bar.
pub const APP_DISPLAY_NAME: &str = "SofDevTool (Rust)";

/// Bundle identifier for the Rust product. Distinct from the Swift bundle.
pub const BUNDLE_IDENTIFIER: &str = "com.gerardo.sofdevtool.rust";

/// UserDefaults/preference domain for the Rust product.
pub const PREFERENCE_DOMAIN: &str = "com.gerardo.sofdevtool.rust";

/// Application Support directory name owned by the Rust product.
pub const APPLICATION_SUPPORT_NAMESPACE: &str = "SofDevToolRust";

/// Resolves the Rust product's Application Support root.
///
/// Nothing writes here yet; History and preferences arrive in later tickets.
pub fn application_support_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(APPLICATION_SUPPORT_NAMESPACE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_distinct_from_the_swift_product() {
        assert_ne!(APPLICATION_SUPPORT_NAMESPACE, "SofDevTool");
        assert!(BUNDLE_IDENTIFIER.ends_with(".rust"));
        assert!(PREFERENCE_DOMAIN.ends_with(".rust"));
    }

    #[test]
    fn support_root_ends_with_the_rust_namespace() {
        if let Some(root) = application_support_root() {
            assert!(root.ends_with(APPLICATION_SUPPORT_NAMESPACE));
        }
    }
}
