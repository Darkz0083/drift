//! The mark a person chose to stand in for a kept site's favicon.
//!
//! One file per essential, named by the hex of the tab id so an id can never
//! escape this directory. The page is handed a `data:` URL and never a path.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// An icon larger than this is refused rather than copied.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

fn directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("essentials"))
        .map_err(|_| "unavailable".to_owned())
}

/// The tab id as a filename: hex of its bytes, so no id can contain a
/// separator or a dot that would climb out of the directory.
fn stem(id: &str) -> String {
    id.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else {
        None
    }
}

fn mime(extension: &str) -> &'static str {
    match extension {
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "image/gif",
    }
}

fn data_url(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let extension = image_extension(&bytes)?;
    Some(format!(
        "data:{};base64,{}",
        mime(extension),
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

fn stored(dir: &Path, id: &str) -> Option<PathBuf> {
    let stem = stem(id);
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_stem()
                .is_some_and(|name| name.to_string_lossy() == stem)
        })
}

fn remove_stored(dir: &Path, id: &str) -> Result<(), String> {
    let stem = stem(id);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_stem()
            .is_some_and(|name| name.to_string_lossy() == stem)
        {
            std::fs::remove_file(&path).map_err(|_| "unavailable".to_owned())?;
        }
    }
    Ok(())
}

/// Asks for an image, checks it, and stores it as this essential's mark.
///
/// `Ok(None)` means the picker was dismissed and nothing changed.
#[tauri::command]
#[specta::specta]
pub(crate) async fn essential_icon_choose(
    app: AppHandle,
    id: String,
) -> Result<Option<String>, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
        .pick_file(move |path| {
            let _ = sender.send(path);
        });
    let Some(chosen) = receiver.await.map_err(|_| "unavailable".to_owned())? else {
        return Ok(None);
    };
    let source = chosen.into_path().map_err(|_| "invalid".to_owned())?;

    let size = std::fs::metadata(&source)
        .map_err(|_| "unavailable".to_owned())?
        .len();
    if size == 0 || size > MAX_BYTES {
        return Err("too_large".to_owned());
    }
    let bytes = std::fs::read(&source).map_err(|_| "unavailable".to_owned())?;
    let extension = image_extension(&bytes).ok_or_else(|| "not_an_image".to_owned())?;

    let dir = directory(&app)?;
    std::fs::create_dir_all(&dir).map_err(|_| "unavailable".to_owned())?;
    remove_stored(&dir, &id)?;
    let target = dir.join(format!("{}.{extension}", stem(&id)));
    std::fs::write(&target, &bytes).map_err(|_| "unavailable".to_owned())?;
    Ok(Some(data_url(&target).ok_or_else(|| "unavailable".to_owned())?))
}

/// The stored mark for one essential, if the person chose one.
#[tauri::command]
#[specta::specta]
pub(crate) fn essential_icon_current(app: AppHandle, id: String) -> Option<String> {
    let dir = directory(&app).ok()?;
    stored(&dir, &id).and_then(|path| data_url(&path))
}

/// Forgets the chosen mark so the site's own favicon returns.
#[tauri::command]
#[specta::specta]
pub(crate) fn essential_icon_clear(app: AppHandle, id: String) -> Result<(), String> {
    let dir = directory(&app)?;
    remove_stored(&dir, &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_can_never_escape_the_directory() {
        assert_eq!(stem("a/b"), "612f62");
        assert_eq!(stem(".."), "2e2e");
        assert!(!stem("../etc/passwd").contains('/'));
        assert!(!stem("x.png").contains('.'));
    }

    #[test]
    fn only_real_images_are_accepted() {
        assert_eq!(
            image_extension(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
            Some("png")
        );
        assert_eq!(image_extension(b"not an image"), None);
    }

    #[test]
    fn clearing_removes_only_this_essentials_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(format!("{}.png", stem("one"))), b"x").unwrap();
        std::fs::write(dir.path().join(format!("{}.png", stem("two"))), b"y").unwrap();
        remove_stored(dir.path(), "one").unwrap();
        assert!(stored(dir.path(), "one").is_none());
        assert!(stored(dir.path(), "two").is_some());
    }
}
