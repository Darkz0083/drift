//! The image a person chose to stand behind their new tab.
//!
//! One file in the app's own data directory under a fixed name. The page never
//! supplies a path: the picker reports what the person chose, the bytes are
//! checked and copied in, and the page is handed a `data:` URL. Nothing here
//! reads or writes anywhere else.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// A background larger than this is refused rather than copied.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

fn directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("newtab"))
        .map_err(|_| "unavailable".to_owned())
}

/// The extension a file may carry, decided by its magic bytes rather than its
/// name: a text file renamed `.png` is not an image and is refused.
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

/// The stored bytes as a URL the page can put in an `<img>` without the app
/// having to open a filesystem protocol.
fn data_url(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let extension = image_extension(&bytes)?;
    Some(format!(
        "data:{};base64,{}",
        mime(extension),
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// Removes every stored background, whatever extension it was copied under.
fn clear_files(dir: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("background.") {
            std::fs::remove_file(entry.path()).map_err(|_| "unavailable".to_owned())?;
        }
    }
    Ok(())
}

fn current_file(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("background."))
        })
}

/// Asks for an image, checks it, stores it, and returns it as a `data:` URL.
///
/// `Ok(None)` means the person dismissed the picker; the stored background is
/// left exactly as it was.
#[tauri::command]
#[specta::specta]
pub(crate) async fn newtab_background_choose(app: AppHandle) -> Result<Option<String>, String> {
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
    clear_files(&dir)?;
    let target = dir.join(format!("background.{extension}"));
    std::fs::write(&target, &bytes).map_err(|_| "unavailable".to_owned())?;
    Ok(Some(data_url(&target).ok_or_else(|| "unavailable".to_owned())?))
}

/// The stored background as a `data:` URL, if the person has chosen one.
#[tauri::command]
#[specta::specta]
pub(crate) fn newtab_background_current(app: AppHandle) -> Option<String> {
    let dir = directory(&app).ok()?;
    current_file(&dir).and_then(|path| data_url(&path))
}

/// Forgets the stored background so the new tab returns to its sky.
#[tauri::command]
#[specta::specta]
pub(crate) fn newtab_background_clear(app: AppHandle) -> Result<(), String> {
    let dir = directory(&app)?;
    clear_files(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_is_recognised_by_its_bytes_not_its_name() {
        assert_eq!(
            image_extension(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0]),
            Some("png")
        );
        assert_eq!(image_extension(&[0xff, 0xd8, 0xff, 0xe0]), Some("jpg"));
        assert_eq!(image_extension(b"GIF89a...."), Some("gif"));
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(b"WEBPVP8 ");
        assert_eq!(image_extension(&webp), Some("webp"));
    }

    #[test]
    fn anything_else_is_refused() {
        assert_eq!(image_extension(b""), None);
        assert_eq!(image_extension(b"<html>not an image</html>"), None);
        assert_eq!(image_extension(&[0x89, b'P']), None);
    }

    #[test]
    fn clearing_removes_only_the_stored_background() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("background.png"), b"x").unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"keep").unwrap();
        clear_files(dir.path()).unwrap();
        assert!(!dir.path().join("background.png").exists());
        assert!(dir.path().join("notes.txt").exists());
        assert_eq!(current_file(dir.path()), None);
    }

    #[test]
    fn a_stored_image_becomes_a_data_url_of_its_own_type() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("background.png");
        std::fs::write(&path, [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2]).unwrap();
        let url = data_url(&path).expect("a png reads back");
        assert!(url.starts_with("data:image/png;base64,"), "{url}");
    }
}
