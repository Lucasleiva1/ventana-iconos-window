use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::storage_service::StorageService;

const ACTIVE_LOG: &str = "desktop-organizer.log";
const MAX_LOG_BYTES: u64 = 1_048_576;
const MAX_LOG_FILES: usize = 5;
static LOG_LOCK: Mutex<()> = Mutex::new(());

pub struct LogService;

impl LogService {
    pub fn initialize() -> Result<(), String> {
        let paths = StorageService::paths()?;
        fs::create_dir_all(&paths.logs)
            .map_err(|error| format!("No se pudo preparar la carpeta de logs: {error}"))?;
        rotate_if_needed(&paths.logs)?;
        Self::info("Aplicación iniciada")
    }

    pub fn info(message: &str) -> Result<(), String> {
        write_entry("INFO", message)
    }

    pub fn error(message: &str) -> Result<(), String> {
        write_entry("ERROR", message)
    }

    pub fn clear() -> Result<(), String> {
        let _guard = LOG_LOCK
            .lock()
            .map_err(|_| "No se pudo bloquear la limpieza de logs".to_owned())?;
        let paths = StorageService::paths()?;
        let entries = match fs::read_dir(&paths.logs) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("No se pudo leer la carpeta de logs: {error}")),
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if is_managed_log(&path) {
                fs::remove_file(&path)
                    .map_err(|error| format!("No se pudo eliminar {}: {error}", path.display()))?;
            }
        }
        Ok(())
    }
}

fn write_entry(level: &str, message: &str) -> Result<(), String> {
    let _guard = LOG_LOCK
        .lock()
        .map_err(|_| "No se pudo bloquear la escritura del log".to_owned())?;
    let paths = StorageService::paths()?;
    fs::create_dir_all(&paths.logs)
        .map_err(|error| format!("No se pudo preparar la carpeta de logs: {error}"))?;
    rotate_if_needed(&paths.logs)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let sanitized = message.replace(['\r', '\n'], " ");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.logs.join(ACTIVE_LOG))
        .map_err(|error| format!("No se pudo abrir el log: {error}"))?;
    writeln!(file, "{timestamp} [{level}] {sanitized}")
        .map_err(|error| format!("No se pudo escribir el log: {error}"))
}

fn rotate_if_needed(directory: &Path) -> Result<(), String> {
    let active = directory.join(ACTIVE_LOG);
    let oversized = active
        .metadata()
        .map(|metadata| metadata.len() >= MAX_LOG_BYTES)
        .unwrap_or(false);
    if !oversized {
        return Ok(());
    }
    let oldest = rotated_path(directory, MAX_LOG_FILES - 1);
    if oldest.exists() {
        fs::remove_file(&oldest)
            .map_err(|error| format!("No se pudo rotar {}: {error}", oldest.display()))?;
    }
    for index in (1..MAX_LOG_FILES - 1).rev() {
        let source = rotated_path(directory, index);
        if source.exists() {
            fs::rename(&source, rotated_path(directory, index + 1))
                .map_err(|error| format!("No se pudo rotar {}: {error}", source.display()))?;
        }
    }
    fs::rename(&active, rotated_path(directory, 1))
        .map_err(|error| format!("No se pudo rotar {}: {error}", active.display()))
}

fn rotated_path(directory: &Path, index: usize) -> PathBuf {
    directory.join(format!("desktop-organizer.{index}.log"))
}

fn is_managed_log(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name == ACTIVE_LOG
                    || (name.starts_with("desktop-organizer.") && name.ends_with(".log"))
            })
}

#[cfg(test)]
mod tests {
    use super::{ACTIVE_LOG, MAX_LOG_BYTES, is_managed_log, rotate_if_needed};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn rotates_only_managed_logs_and_keeps_a_bounded_set() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-logs-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("fixture directory should exist");
        fs::write(root.join(ACTIVE_LOG), vec![b'x'; MAX_LOG_BYTES as usize])
            .expect("active log should exist");
        fs::write(root.join("personal.txt"), b"safe").expect("unrelated file should exist");
        rotate_if_needed(&root).expect("rotation should work");
        assert!(root.join("desktop-organizer.1.log").is_file());
        assert!(root.join("personal.txt").is_file());
        assert!(!is_managed_log(&root.join("personal.txt")));
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }
}
