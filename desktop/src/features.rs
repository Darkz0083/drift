//! Feature gates as the person set them.
//!
//! The flags live in the frontend's preferences, which is where they are edited
//! and where the surfaces are hidden. Hiding a surface is not the same as
//! closing it, so the same flags are pushed here as they load and change, and
//! a command a feature owns is refused at the boundary however it was reached.

use std::collections::HashSet;
use std::sync::RwLock;

use tauri::WebviewWindow;

/// The features that gate anything. A name outside this list is refused, so a
/// page cannot invent a flag and switch something off that it does not own.
pub(crate) const FEATURES: &[&str] = &["ai", "work", "notes", "tasks", "time"];

/// The features currently switched off. Empty until the frontend says
/// otherwise, which is deliberate: before preferences have loaded there is
/// nothing on screen to reach, and failing closed would blank the browser.
static OFF: RwLock<Option<HashSet<String>>> = RwLock::new(None);

/// Whether a feature is switched off. Unknown features read as on.
pub(crate) fn off(feature: &str) -> bool {
    OFF.read()
        .ok()
        .and_then(|set| set.as_ref().map(|set| set.contains(feature)))
        .unwrap_or(false)
}

/// The feature a browser surface belongs to, if it belongs to one.
pub(crate) fn page_feature(page: &str) -> Option<&'static str> {
    match page {
        "work" => Some("work"),
        "notes" => Some("notes"),
        "tasks" => Some("tasks"),
        "time" => Some("time"),
        _ => None,
    }
}

/// Records a feature's state. Answers false when the caller or the name is not
/// one this module owns, so a refusal is visible rather than silent.
#[tauri::command]
#[specta::specta]
pub(crate) fn feature_set_enabled(
    caller: WebviewWindow,
    feature: String,
    enabled: bool,
) -> bool {
    if !crate::authorize(&caller, crate::CallerPolicy::Main, "feature_set_enabled") {
        return false;
    }
    if !FEATURES.contains(&feature.as_str()) {
        return false;
    }
    let Ok(mut set) = OFF.write() else {
        return false;
    };
    let set = set.get_or_insert_with(HashSet::new);
    if enabled {
        set.remove(&feature);
    } else {
        set.insert(feature);
    }
    true
}

/// Forgets what was recorded, for tests.
#[cfg(test)]
pub(crate) fn reset() {
    if let Ok(mut set) = OFF.write() {
        *set = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_off_before_the_frontend_says_so() {
        reset();
        assert!(!off("work"));
        assert!(!off("notes"));
    }

    #[test]
    fn a_surface_knows_which_feature_owns_it() {
        assert_eq!(page_feature("notes"), Some("notes"));
        assert_eq!(page_feature("settings"), None);
        assert_eq!(page_feature("history"), None);
    }

    #[test]
    fn only_the_known_features_gate_anything() {
        assert!(FEATURES.contains(&"work"));
        assert!(!FEATURES.contains(&"history"));
    }
}
