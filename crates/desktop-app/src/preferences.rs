use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Manager};

const FILE_NAME: &str = "close-to-tray";
const MAX_BYTES: u64 = 8;

pub struct ClosePreference {
    enabled: AtomicBool,
    path: Option<PathBuf>,
}

impl ClosePreference {
    pub fn load(app: &AppHandle) -> Self {
        app.path().app_config_dir().map_or_else(
            |_| Self {
                enabled: AtomicBool::new(true),
                path: None,
            },
            |directory| Self::at(directory.join(FILE_NAME)),
        )
    }

    fn at(path: PathBuf) -> Self {
        let enabled = read(&path).unwrap_or(true);
        Self {
            enabled: AtomicBool::new(enabled),
            path: Some(path),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn set(&self, enabled: bool) -> io::Result<()> {
        let path = self
            .path
            .as_deref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config path unavailable"))?;
        write(path, enabled)?;
        self.enabled.store(enabled, Ordering::Release);
        Ok(())
    }
}

fn read(path: &Path) -> io::Result<bool> {
    let file = OpenOptions::new().read(true).open(path)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "setting too large",
        ));
    }
    let mut value = String::new();
    file.take(MAX_BYTES + 1).read_to_string(&mut value)?;
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid setting",
        )),
    }
}

fn write(path: &Path, enabled: bool) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "setting path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!("{FILE_NAME}.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(if enabled { b"true\n" } else { b"false\n" })?;
        file.sync_all()?;
        drop(file);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_defaults_true_and_round_trips_without_browser_storage() {
        let root =
            std::env::temp_dir().join(format!("codex-accounts-preference-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let path = root.join(FILE_NAME);
        let preference = ClosePreference::at(path.clone());
        assert!(preference.enabled());
        preference.set(false).unwrap();
        assert!(!ClosePreference::at(path).enabled());
        fs::remove_dir_all(root).unwrap();
    }
}
