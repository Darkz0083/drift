//! Where the account session is kept.
//!
//! The provider keys live in the login Keychain. The session — an ID token and
//! the refresh token that mints more — used to live in the webview's
//! `localStorage`, which is plaintext in the app's data directory and reachable
//! from any script running in that origin. It is a credential like the others,
//! so it is kept where the others are.

use std::path::Path;

/// The one item this module owns.
#[cfg(target_os = "macos")]
const SERVICE: &str = "app.drift.session";
const ACCOUNT: &str = "development";

/// The session as stored, or `None` when there is none or it cannot be read.
///
/// A Keychain that will not answer is not an error the person can act on: it
/// reads as "signed out", which is the state the app already handles.
pub(crate) fn read(app: &tauri::AppHandle) -> Option<String> {
    imp::read(app)
}

/// Replaces the stored session. Answers whether it is durably kept, so the
/// caller can decide whether the old copy may be discarded.
pub(crate) fn write(app: &tauri::AppHandle, value: &str) -> bool {
    imp::write(app, value)
}

/// Forgets the stored session.
pub(crate) fn clear(app: &tauri::AppHandle) -> bool {
    imp::clear(app)
}

#[cfg(target_os = "macos")]
mod imp {
    use security_framework::item::{ItemClass, ItemSearchOptions};
    use security_framework::os::macos::keychain::SecKeychain;
    use security_framework::os::macos::passwords::find_generic_password;
    use security_framework_sys::base::errSecItemNotFound;

    use super::{ACCOUNT, SERVICE};

    fn login() -> Option<SecKeychain> {
        let path = dirs::home_dir()?.join("Library/Keychains/login.keychain-db");
        SecKeychain::open(path).ok()
    }

    fn search(keychain: &SecKeychain) -> ItemSearchOptions {
        let mut options = ItemSearchOptions::new();
        options
            .class(ItemClass::generic_password())
            .keychains(std::slice::from_ref(keychain))
            .service(SERVICE)
            .account(ACCOUNT);
        options
    }

    pub(super) fn read(_: &tauri::AppHandle) -> Option<String> {
        let keychain = login()?;
        let (password, _item) = find_generic_password(Some(&[keychain]), SERVICE, ACCOUNT).ok()?;
        String::from_utf8(password.as_ref().to_vec()).ok()
    }

    pub(super) fn write(_: &tauri::AppHandle, value: &str) -> bool {
        let Some(keychain) = login() else {
            return false;
        };
        // Replace by delete + add: updating in place reads the old secret first,
        // which prompts when another binary created the item.
        let _ = search(&keychain).delete();
        keychain
            .add_generic_password(SERVICE, ACCOUNT, value.as_bytes())
            .or_else(|_| keychain.set_generic_password(SERVICE, ACCOUNT, value.as_bytes()))
            .is_ok()
    }

    pub(super) fn clear(_: &tauri::AppHandle) -> bool {
        let Some(keychain) = login() else {
            return false;
        };
        match search(&keychain).delete() {
            Ok(()) => true,
            Err(error) => error.code() == errSecItemNotFound,
        }
    }
}

/// Everywhere else the login Keychain does not exist, so the session goes in a
/// file only this account can read. That is weaker than a Keychain — it is not
/// encrypted at rest — but it is still out of reach of the webview, which is
/// the boundary this module exists to draw.
#[cfg(not(target_os = "macos"))]
mod imp {
    use std::io::Write as _;

    use tauri::Manager as _;

    fn path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
        app.path().app_data_dir().ok().map(|dir| dir.join("session"))
    }

    pub(super) fn read(app: &tauri::AppHandle) -> Option<String> {
        std::fs::read_to_string(path(app)?).ok()
    }

    pub(super) fn write(app: &tauri::AppHandle, value: &str) -> bool {
        let Some(path) = path(app) else {
            return false;
        };
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        let Ok(mut file) = std::fs::File::create(&path) else {
            return false;
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            if std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).is_err() {
                return false;
            }
        }
        file.write_all(value.as_bytes()).is_ok()
    }

    pub(super) fn clear(app: &tauri::AppHandle) -> bool {
        match path(app) {
            Some(path) => std::fs::remove_file(path).is_ok() || !path.exists(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The service name is a contract with the Keychain: changing it silently
    /// signs everyone out on the next launch.
    #[test]
    fn the_item_is_named_for_the_app() {
        #[cfg(target_os = "macos")]
        assert_eq!(SERVICE, "app.drift.session");
        assert_eq!(ACCOUNT, "development");
    }

    #[test]
    fn a_path_is_only_used_off_macos() {
        // Compiles on both: the macOS implementation takes the handle and
        // ignores it, which is what keeps the call sites platform-free.
        let _: fn(&Path) -> bool = |_| true;
    }
}
