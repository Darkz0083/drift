//! The launcher's shortcut, as Linux builds it.
//!
//! `tauri-plugin-global-shortcut` is a macOS and Windows dependency, so the
//! real recorder is not available here. Linux owns its global shortcut through
//! `linux_global_shortcuts` instead, which registers the launcher from the
//! keymap. This module keeps the commands and their types — so the frame's
//! surface is identical on every platform — and reports the shortcut as
//! managed elsewhere rather than pretending it can be recorded here.
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

pub const DEFAULT: &str = "CmdOrCtrl+Shift+Space";

/// The modifier Linux cannot watch, kept so the frame's settings render.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DoubleTap {
    #[default]
    Off,
    Command,
    Option,
}

/// Why a recorded shortcut was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Rejection {
    Invalid,
    NeedsModifier,
    System,
    AppCommand,
    Unavailable,
    /// This platform's shortcut is managed elsewhere.
    Unsupported,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct LauncherTrigger {
    pub shortcut: String,
    pub default_shortcut: String,
    pub registered: bool,
    pub editable: bool,
    pub double_tap: DoubleTap,
    pub double_tap_supported: bool,
    pub accessibility: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerChange {
    Applied {
        trigger: LauncherTrigger,
    },
    Rejected {
        reason: Rejection,
        trigger: LauncherTrigger,
    },
}

/// Managed state the composition root installs. Linux keeps the shortcut in
/// the keymap, so there is nothing to hold here.
#[derive(Default)]
pub struct Trigger;

fn reported() -> LauncherTrigger {
    LauncherTrigger {
        shortcut: DEFAULT.to_owned(),
        default_shortcut: DEFAULT.to_owned(),
        // `linux_global_shortcuts` decides this from the keymap, not here.
        registered: false,
        editable: false,
        double_tap: DoubleTap::Off,
        double_tap_supported: false,
        accessibility: false,
    }
}

pub fn install(_app: &AppHandle, _keymap_override: Option<String>) {}

pub fn snapshot(_app: &AppHandle) -> LauncherTrigger {
    reported()
}

pub fn set_shortcut(_app: &AppHandle, _accelerator: &str) -> TriggerChange {
    TriggerChange::Rejected {
        reason: Rejection::Unsupported,
        trigger: reported(),
    }
}

pub fn recording(_app: &AppHandle, _active: bool) {}

pub fn set_double_tap(_app: &AppHandle, _mode: DoubleTap) -> LauncherTrigger {
    reported()
}

pub fn open_accessibility_settings() {}
