//! Frontmost-application snapshot on macOS via
//! `NSWorkspace.frontmostApplication`. Returns `None` on non-macOS so the
//! scheduler can run as a no-op in CI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForegroundApp {
    pub bundle_id: Option<String>,
    pub display_name: Option<String>,
    pub process_id: Option<i32>,
}

#[cfg(target_os = "macos")]
pub fn current_foreground_app() -> Option<ForegroundApp> {
    use objc2_app_kit::NSWorkspace;
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let bundle_id = app.bundleIdentifier().map(|s| s.to_string());
    let display_name = app.localizedName().map(|s| s.to_string());
    let process_id = app.processIdentifier();
    Some(ForegroundApp {
        bundle_id,
        display_name,
        process_id: Some(process_id),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn current_foreground_app() -> Option<ForegroundApp> {
    None
}
