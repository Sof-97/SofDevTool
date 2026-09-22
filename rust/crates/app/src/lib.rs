//! The Rust Developer Toolbox application.
//!
//! The application owns Registry, workspaces, History policy, preferences,
//! platform composition and lifecycle. Ticket 01 ships the JSON workspace only.

pub mod clipboard;
pub mod history;
pub mod identity;
pub mod json_workspace;
pub mod launcher;
pub mod native_window;
pub mod preferences;
pub mod registry;
pub mod settings;
pub mod shortcut;
pub mod text_diff;
pub mod workbench;
