fn main() {
    println!("cargo:rerun-if-env-changed=SOFDEVTOOL_BUILD_ID");
    println!("cargo:rerun-if-env-changed=SOFDEVTOOL_REVISION");
    println!("cargo:rerun-if-env-changed=SOFDEVTOOL_SOURCE_STATE");

    // An ordinary Cargo build cannot prove worktree cleanliness when the Git
    // index may be synthetic. Packaging supplies freshly inspected metadata;
    // absent those inputs, disclose unknown instead of retaining stale values.
    let revision = std::env::var("SOFDEVTOOL_REVISION").unwrap_or_else(|_| "unknown".to_owned());
    let state = std::env::var("SOFDEVTOOL_SOURCE_STATE").unwrap_or_else(|_| "unknown".to_owned());
    let build = std::env::var("SOFDEVTOOL_BUILD_ID").unwrap_or_else(|_| "1".to_owned());
    assert!(!build.is_empty() && build.chars().all(|character| character.is_ascii_digit()));
    assert!(matches!(state.as_str(), "clean" | "dirty" | "unknown"));
    assert!(
        revision == "unknown"
            || (revision.len() == 40
                && revision
                    .chars()
                    .all(|character| character.is_ascii_hexdigit()))
    );
    println!("cargo:rustc-env=SOFDEVTOOL_BUILD_ID={build}");
    println!("cargo:rustc-env=SOFDEVTOOL_REVISION={revision}");
    println!("cargo:rustc-env=SOFDEVTOOL_SOURCE_STATE={state}");
}
