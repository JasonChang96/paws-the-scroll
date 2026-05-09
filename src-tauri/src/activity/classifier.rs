//! Classify the current foreground app as social-feed-like or not.
//! Bundle-id-based for V1 (PRD §7 calls browser URL detection a stretch
//! goal). The user can extend the set via Settings.

use serde::{Deserialize, Serialize};

use super::foreground::ForegroundApp;

const SOCIAL_BUNDLE_IDS: &[&str] = &[
    "com.atebits.Tweetie2",
    "com.tinyspeck.slackmacgap",
    "com.hammerandchisel.discord",
    "com.facebook.archon",
    "com.instagram.macos",
    "com.zhiliaoapp.musically",
    "com.tiktok.app",
    "com.facebook.Messenger",
    "com.google.YouTubeMusic",
    "com.reddit.reddit-mac",
    "tv.twitch.desktop",
];

const SOCIAL_NAME_HINTS: &[&str] = &[
    "TikTok",
    "Instagram",
    "Twitter",
    "X (formerly Twitter)",
    "Reddit",
    "YouTube",
    "Discord",
    "Twitch",
    "Facebook",
];

const BROWSER_BUNDLE_IDS: &[&str] = &[
    "com.apple.Safari",
    "com.google.Chrome",
    "com.microsoft.edgemac",
    "com.brave.Browser",
    "company.thebrowser.Browser",
    "org.mozilla.firefox",
    "com.operasoftware.Opera",
    "com.vivaldi.Vivaldi",
    "org.chromium.Chromium",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Social,
    Browser,
    Other,
}

pub fn classify(app: &ForegroundApp, user_extra_social: &[String]) -> AppCategory {
    let bundle = app.bundle_id.as_deref().unwrap_or("");
    let name = app.display_name.as_deref().unwrap_or("");

    if user_extra_social
        .iter()
        .any(|s| s.eq_ignore_ascii_case(bundle) || s.eq_ignore_ascii_case(name))
    {
        return AppCategory::Social;
    }
    if SOCIAL_BUNDLE_IDS.contains(&bundle) {
        return AppCategory::Social;
    }
    if SOCIAL_NAME_HINTS
        .iter()
        .any(|h| name.eq_ignore_ascii_case(h))
    {
        return AppCategory::Social;
    }
    if BROWSER_BUNDLE_IDS.contains(&bundle) {
        return AppCategory::Browser;
    }
    AppCategory::Other
}
