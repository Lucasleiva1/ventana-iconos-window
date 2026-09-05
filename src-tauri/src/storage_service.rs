use std::{
    ffi::{OsStr, c_void},
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Component, Path, PathBuf, Prefix},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use windows::{
    Win32::{
        Storage::FileSystem::{
            FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, GetFileAttributesW,
            INVALID_FILE_ATTRIBUTES, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            MoveFileExW, SetFileAttributesW,
        },
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Desktop, FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    },
    core::{GUID, PCWSTR},
};

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

use crate::model::Drawer;

pub const APP_FOLDER_NAME: &str = "Desktop Organizer";
pub const DRAWERS_FOLDER_NAME: &str = "Cajones";
pub const BACKUPS_FOLDER_NAME: &str = "Backups";
pub const MASTER_SAVE_NAME: &str = "desktop-organizer-save.json";
pub const DRAWER_METADATA_NAME: &str = ".drawer.json";
const TRANSFER_PREFIX: &str = ".desktop-organizer-transfer-";
const DRAWER_METADATA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct StoragePaths {
    pub documents: PathBuf,
    pub desktop: PathBuf,
    pub root: PathBuf,
    pub drawers: PathBuf,
    pub backups: PathBuf,
    pub master_save: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePathsInfo {
    pub documents: PathBuf,
    pub desktop: PathBuf,
    pub root: PathBuf,
    pub drawers: PathBuf,
    pub master_save: PathBuf,
}

impl From<&StoragePaths> for StoragePathsInfo {
    fn from(paths: &StoragePaths) -> Self {
        Self {
            documents: paths.documents.clone(),
            desktop: paths.desktop.clone(),
            root: paths.root.clone(),
            drawers: paths.drawers.clone(),
            master_save: paths.master_save.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerMetadata {
    pub drawer_id: String,
    pub format_version: u32,
    pub name: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct RecoveredDrawerSeed {
    pub id: Option<String>,
    pub name: String,
    pub folder_path: PathBuf,
    pub created_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TransferOutcome {
    pub warning: Option<String>,
}

pub struct StorageService;

impl StorageService {
    pub fn paths() -> Result<StoragePaths, String> {
        let documents = known_folder(&FOLDERID_Documents)?;
        let desktop = known_folder(&FOLDERID_Desktop)?;
        let root = documents.join(APP_FOLDER_NAME);
        Ok(StoragePaths {
            desktop,
            drawers: root.join(DRAWERS_FOLDER_NAME),
            backups: root.join(BACKUPS_FOLDER_NAME),
            master_save: root.join(MASTER_SAVE_NAME),
            documents,
            root,
        })
    }

    pub fn ensure_layout(paths: &StoragePaths) -> Result<(), String> {
        fs::create_dir_all(&paths.drawers)
            .map_err(|error| format!("No se pudo crear {}: {error}", paths.drawers.display()))?;
        fs::create_dir_all(&paths.backups)
            .map_err(|error| format!("No se pudo crear {}: {error}", paths.backups.display()))
    }

    pub fn provision_drawer(drawer: &mut Drawer, paths: &StoragePaths) -> Result<(), String> {
        Self::ensure_layout(paths)?;

        if drawer.folder_path.as_os_str().is_empty()
            || !is_within(&drawer.folder_path, &paths.drawers)
        {
            if let Some(existing) = Self::discover_drawers(paths)?
                .into_iter()
                .find(|candidate| candidate.id.as_deref() == Some(drawer.id.as_str()))
            {
                drawer.folder_path = existing.folder_path;
            } else {
                drawer.folder_path = unique_drawer_path(paths, &drawer.name)?;
            }
        } else if !drawer.folder_path.exists()
            && let Some(existing) = Self::discover_drawers(paths)?
                .into_iter()
                .find(|candidate| candidate.id.as_deref() == Some(drawer.id.as_str()))
        {
            drawer.folder_path = existing.folder_path;
        }

        if drawer.folder_path.exists() {
            if !drawer.folder_path.is_dir() {
                return Err(format!(
                    "La ubicación física del cajón no es una carpeta: {}",
                    drawer.folder_path.display()
                ));
            }
            if let Some(metadata) = read_metadata(&drawer.folder_path)?
                && metadata.drawer_id != drawer.id
            {
                return Err(format!(
                    "La carpeta {} pertenece a otro cajón; no se modificó",
                    drawer.folder_path.display()
                ));
            }
        } else {
            fs::create_dir_all(&drawer.folder_path).map_err(|error| {
                format!(
                    "No se pudo crear la carpeta física {}: {error}",
                    drawer.folder_path.display()
                )
            })?;
        }

        Self::write_drawer_metadata(drawer)
    }

    pub fn rename_drawer_folder(
        drawer: &Drawer,
        next_name: &str,
        paths: &StoragePaths,
    ) -> Result<PathBuf, String> {
        if !drawer.folder_path.is_dir() {
            return Err("La carpeta física del cajón no está disponible".to_owned());
        }
        let target = paths.drawers.join(sanitize_folder_name(next_name));
        if comparable_path(&target) == comparable_path(&drawer.folder_path) {
            return Ok(drawer.folder_path.clone());
        }
        if target.exists() {
            return Err(format!(
                "Ya existe una carpeta llamada “{}”. No se sobrescribió nada.",
                target
                    .file_name()
                    .unwrap_or_else(|| OsStr::new(next_name))
                    .to_string_lossy()
            ));
        }
        fs::rename(&drawer.folder_path, &target).map_err(|error| {
            format!("No se pudo renombrar la carpeta física del cajón: {error}")
        })?;
        Ok(target)
    }

    pub fn write_drawer_metadata(drawer: &Drawer) -> Result<(), String> {
        Self::write_folder_metadata(
            &drawer.folder_path,
            &drawer.id,
            &drawer.name,
            drawer.created_at,
        )
    }

    pub fn write_folder_metadata(
        folder: &Path,
        drawer_id: &str,
        name: &str,
        created_at: u64,
    ) -> Result<(), String> {
        let metadata = DrawerMetadata {
            drawer_id: drawer_id.to_owned(),
            format_version: DRAWER_METADATA_VERSION,
            name: name.to_owned(),
            created_at,
        };
        let destination = folder.join(DRAWER_METADATA_NAME);
        let temporary = folder.join(format!("{DRAWER_METADATA_NAME}.{}.tmp", Uuid::new_v4()));
        let json = serde_json::to_vec_pretty(&metadata)
            .map_err(|error| format!("No se pudo serializar la metadata del cajón: {error}"))?;
        write_synced(&temporary, &json)?;
        if let Err(error) = atomic_replace(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        set_hidden(&destination)
    }

    pub fn read_folder_metadata(folder: &Path) -> Result<Option<DrawerMetadata>, String> {
        read_metadata(folder)
    }

    pub fn remove_folder_metadata(folder: &Path) -> Result<(), String> {
        let metadata = folder.join(DRAWER_METADATA_NAME);
        match fs::remove_file(&metadata) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "No se pudo convertir el subcajón en carpeta normal: {error}"
            )),
        }
    }

    pub fn rename_managed_folder(folder: &Path, next_name: &str) -> Result<PathBuf, String> {
        Self::validate_windows_folder_name(next_name)?;
        let parent = folder
            .parent()
            .ok_or_else(|| "La carpeta no tiene una ubicación padre válida".to_owned())?;
        let target = parent.join(next_name);
        if comparable_path(&target) == comparable_path(folder) {
            return Ok(folder.to_path_buf());
        }
        if target.exists() {
            return Err(format!(
                "Ya existe una carpeta llamada “{next_name}”. No se sobrescribió nada."
            ));
        }
        fs::rename(folder, &target)
            .map_err(|error| format!("Windows no pudo renombrar la carpeta: {error}"))?;
        Ok(target)
    }

    pub fn validate_windows_folder_name(name: &str) -> Result<(), String> {
        if name.is_empty() || name.trim() != name {
            return Err(
                "El nombre no puede estar vacío ni comenzar o terminar con espacios".to_owned(),
            );
        }
        if name.ends_with('.') {
            return Err("Windows no permite nombres de carpeta terminados en punto".to_owned());
        }
        if name.chars().count() > 80 {
            return Err("El nombre no puede superar los 80 caracteres".to_owned());
        }
        if name
            .chars()
            .any(|character| character.is_control() || "<>:\"/\\|?*".contains(character))
        {
            return Err(
                "El nombre contiene caracteres no permitidos por Windows: < > : \" / \\ | ? *"
                    .to_owned(),
            );
        }
        let device_name = name.split('.').next().unwrap_or(name);
        if is_reserved_device_name(device_name) {
            return Err(format!(
                "“{name}” es un nombre reservado por Windows. Elegí otro nombre."
            ));
        }
        Ok(())
    }

    pub fn discover_drawers(paths: &StoragePaths) -> Result<Vec<RecoveredDrawerSeed>, String> {
        if !paths.drawers.exists() {
            return Ok(Vec::new());
        }
        let mut recovered = Vec::new();
        let entries = fs::read_dir(&paths.drawers).map_err(|error| {
            format!(
                "No se pudo leer la carpeta de cajones {}: {error}",
                paths.drawers.display()
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("No se pudo leer un cajón: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("No se pudo identificar un cajón: {error}"))?;
            if !file_type.is_dir() {
                continue;
            }
            let folder_path = entry.path();
            let metadata = match read_metadata(&folder_path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    eprintln!("{error}. La carpeta se ofrecerá para recuperación por su nombre.");
                    None
                }
            };
            let fallback_name = entry.file_name().to_string_lossy().into_owned();
            recovered.push(RecoveredDrawerSeed {
                id: metadata.as_ref().map(|value| value.drawer_id.clone()),
                name: metadata
                    .as_ref()
                    .map(|value| value.name.clone())
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or(fallback_name),
                created_at: metadata.as_ref().map(|value| value.created_at),
                folder_path,
            });
        }
        recovered.sort_by_key(|left| left.name.to_lowercase());
        Ok(recovered)
    }

    pub fn move_safely(source: &Path, destination: &Path) -> Result<TransferOutcome, String> {
        if destination.exists() {
            return Err(format!(
                "Ya existe “{}” en el destino. No se sobrescribió ni fusionó nada.",
                destination
                    .file_name()
                    .unwrap_or(destination.as_os_str())
                    .to_string_lossy()
            ));
        }
        if !source.exists() {
            return Err("El elemento de origen ya no existe".to_owned());
        }

        match fs::rename(source, destination) {
            Ok(()) => {
                return Ok(TransferOutcome { warning: None });
            }
            Err(error) if same_volume(source, destination) => {
                return Err(format!("Windows no pudo mover el elemento: {error}"));
            }
            Err(_) => {}
        }

        let parent = destination
            .parent()
            .ok_or_else(|| "El destino no tiene una carpeta válida".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("No se pudo preparar el destino: {error}"))?;
        let staging = parent.join(format!("{TRANSFER_PREFIX}{}", Uuid::new_v4()));
        match copy_entry_verified(source, &staging) {
            Ok(_) => {}
            Err(error) => {
                cleanup_staging(&staging, parent);
                return Err(format!(
                    "La copia entre volúmenes no terminó correctamente; el origen permanece intacto. {error}"
                ));
            }
        }

        if let Err(error) = fs::rename(&staging, destination) {
            cleanup_staging(&staging, parent);
            return Err(format!(
                "No se pudo confirmar el destino; el origen permanece intacto. {error}"
            ));
        }

        let source_is_directory = source.is_dir();
        let removal = if source_is_directory {
            fs::remove_dir_all(source)
        } else {
            fs::remove_file(source)
        };
        let warning = removal.err().map(|error| {
            format!(
                "La copia fue verificada, pero Windows no pudo retirar el origen. Hay una copia segura en el cajón y el original continúa en {}: {error}",
                source.display()
            )
        });
        Ok(TransferOutcome { warning })
    }

    pub fn is_desktop_item(path: &Path, desktop: &Path) -> bool {
        comparable_path(path) != comparable_path(desktop) && is_within(path, desktop)
    }

    pub fn is_internal_file_name(name: &OsStr) -> bool {
        let value = name.to_string_lossy();
        value.eq_ignore_ascii_case(DRAWER_METADATA_NAME)
            || value.starts_with(&format!("{DRAWER_METADATA_NAME}."))
            || value.starts_with(TRANSFER_PREFIX)
    }
}

pub fn atomic_replace(source: &Path, destination: &Path) -> Result<(), String> {
    let source_wide = to_wide(source.as_os_str());
    let destination_wide = to_wide(destination.as_os_str());
    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|error| {
        format!(
            "No se pudo confirmar {} de forma atómica: {error}",
            destination.display()
        )
    })
}

pub fn comparable_path(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

pub fn is_within(path: &Path, parent: &Path) -> bool {
    let path_key = comparable_path(path);
    let parent_key = comparable_path(parent);
    path_key == parent_key
        || path_key
            .strip_prefix(&parent_key)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn unique_drawer_path(paths: &StoragePaths, name: &str) -> Result<PathBuf, String> {
    let base = sanitize_folder_name(name);
    for index in 1..=10_000_u32 {
        let folder_name = if index == 1 {
            base.clone()
        } else {
            format!("{base} ({index})")
        };
        let candidate = paths.drawers.join(folder_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("No se pudo encontrar un nombre libre para la carpeta del cajón".to_owned())
}

fn sanitize_folder_name(name: &str) -> String {
    let mut sanitized: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_control() || "<>:\"/\\|?*".contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    sanitized = sanitized.trim_matches([' ', '.']).to_owned();
    if sanitized.is_empty() {
        sanitized = "Cajón".to_owned();
    }
    if is_reserved_device_name(sanitized.split('.').next().unwrap_or(&sanitized)) {
        sanitized.push_str(" - Cajón");
    }
    sanitized
}

fn is_reserved_device_name(name: &str) -> bool {
    [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .iter()
    .any(|value| name.eq_ignore_ascii_case(value))
}

fn read_metadata(folder: &Path) -> Result<Option<DrawerMetadata>, String> {
    let path = folder.join(DRAWER_METADATA_NAME);
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "No se pudo leer la metadata de {}: {error}",
                folder.display()
            ));
        }
    };
    let metadata: DrawerMetadata = serde_json::from_str(&contents)
        .map_err(|error| format!("La metadata de {} no es válida: {error}", folder.display()))?;
    if metadata.format_version > DRAWER_METADATA_VERSION {
        return Err(format!(
            "La metadata de {} usa una versión más nueva",
            folder.display()
        ));
    }
    Ok(Some(metadata))
}

fn known_folder(id: &GUID) -> Result<PathBuf, String> {
    let raw = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }
        .map_err(|error| format!("Windows no pudo resolver una carpeta conocida: {error}"))?;
    let converted = unsafe { raw.to_string() };
    unsafe { CoTaskMemFree(Some(raw.0.cast::<c_void>())) };
    converted
        .map(PathBuf::from)
        .map_err(|error| format!("Windows devolvió una ruta no válida: {error}"))
}

fn set_hidden(path: &Path) -> Result<(), String> {
    let wide = to_wide(path.as_os_str());
    let attributes = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(format!(
            "No se pudieron leer los atributos de {}",
            path.display()
        ));
    }
    unsafe {
        SetFileAttributesW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(attributes | FILE_ATTRIBUTE_HIDDEN.0),
        )
    }
    .map_err(|error| format!("No se pudo ocultar la metadata interna: {error}"))
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let file = fs::File::create(path)
        .map_err(|error| format!("No se pudo crear {}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(bytes)
        .map_err(|error| format!("No se pudo escribir {}: {error}", path.display()))?;
    writer
        .flush()
        .map_err(|error| format!("No se pudo completar {}: {error}", path.display()))?;
    writer
        .into_inner()
        .map_err(|error| format!("No se pudo completar {}: {error}", path.display()))?
        .sync_all()
        .map_err(|error| format!("No se pudo sincronizar {}: {error}", path.display()))
}

fn copy_entry_verified(source: &Path, destination: &Path) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("No se pudo leer {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{} es un vínculo de filesystem y no se movió automáticamente",
            source.display()
        ));
    }
    if metadata.is_file() {
        return copy_file_verified(source, destination, &metadata);
    }
    if !metadata.is_dir() {
        return Err(format!(
            "{} no es un archivo o carpeta normal",
            source.display()
        ));
    }

    fs::create_dir(destination)
        .map_err(|error| format!("No se pudo crear {}: {error}", destination.display()))?;
    let mut bytes = 0_u64;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("No se pudo leer {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("No se pudo leer una entrada: {error}"))?;
        bytes = bytes.saturating_add(copy_entry_verified(
            &entry.path(),
            &destination.join(entry.file_name()),
        )?);
    }
    fs::set_permissions(destination, metadata.permissions()).map_err(|error| {
        format!(
            "No se pudieron conservar los permisos de {}: {error}",
            destination.display()
        )
    })?;
    Ok(bytes)
}

fn copy_file_verified(
    source: &Path,
    destination: &Path,
    metadata: &fs::Metadata,
) -> Result<u64, String> {
    let source_file = fs::File::open(source)
        .map_err(|error| format!("No se pudo abrir {}: {error}", source.display()))?;
    let destination_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| format!("No se pudo crear {}: {error}", destination.display()))?;
    let mut reader = BufReader::new(source_file);
    let mut writer = BufWriter::new(destination_file);
    let mut source_hash = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut copied = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Falló la lectura de {}: {error}", source.display()))?;
        if read == 0 {
            break;
        }
        source_hash.update(&buffer[..read]);
        writer
            .write_all(&buffer[..read])
            .map_err(|error| format!("Falló la escritura de {}: {error}", destination.display()))?;
        copied = copied.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
    }
    writer
        .flush()
        .map_err(|error| format!("No se pudo completar la copia: {error}"))?;
    writer
        .into_inner()
        .map_err(|error| format!("No se pudo completar la copia: {error}"))?
        .sync_all()
        .map_err(|error| format!("No se pudo sincronizar la copia: {error}"))?;

    let destination_metadata = fs::metadata(destination)
        .map_err(|error| format!("No se pudo verificar el destino: {error}"))?;
    if copied != metadata.len() || destination_metadata.len() != metadata.len() {
        return Err("El tamaño copiado no coincide con el original".to_owned());
    }
    let destination_hash = hash_file(destination)?;
    if source_hash.finalize().as_slice() != destination_hash.as_slice() {
        return Err("La verificación SHA-256 de la copia no coincide".to_owned());
    }
    fs::set_permissions(destination, metadata.permissions()).map_err(|error| {
        format!(
            "No se pudieron conservar los permisos de {}: {error}",
            destination.display()
        )
    })?;
    Ok(copied)
}

fn hash_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut reader = BufReader::new(
        fs::File::open(path)
            .map_err(|error| format!("No se pudo verificar {}: {error}", path.display()))?,
    );
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("No se pudo verificar {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_vec())
}

fn cleanup_staging(staging: &Path, expected_parent: &Path) {
    let valid = staging.parent() == Some(expected_parent)
        && staging
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(TRANSFER_PREFIX));
    if !valid || !staging.exists() {
        return;
    }
    if staging.is_dir() {
        let _ = fs::remove_dir_all(staging);
    } else {
        let _ = fs::remove_file(staging);
    }
}

fn same_volume(source: &Path, destination: &Path) -> bool {
    volume_key(source) == volume_key(destination)
}

fn volume_key(path: &Path) -> Option<String> {
    let absolute = fs::canonicalize(path)
        .or_else(|_| path.canonicalize())
        .unwrap_or_else(|_| path.to_path_buf());
    let prefix = absolute.components().next()?;
    let Component::Prefix(prefix) = prefix else {
        return None;
    };
    Some(match prefix.kind() {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
            format!("disk:{}", char::from(letter).to_ascii_lowercase())
        }
        Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => format!(
            "unc:{}\\{}",
            server.to_string_lossy().to_lowercase(),
            share.to_string_lossy().to_lowercase()
        ),
        other => format!("{other:?}").to_lowercase(),
    })
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{StoragePaths, StorageService, copy_entry_verified};
    use crate::model::Drawer;
    use uuid::Uuid;

    fn fixture_paths(root: &std::path::Path) -> StoragePaths {
        let documents = root.join("Documents");
        let desktop = root.join("Desktop");
        let app_root = documents.join("Desktop Organizer");
        StoragePaths {
            documents,
            desktop,
            drawers: app_root.join("Cajones"),
            backups: app_root.join("Backups"),
            master_save: app_root.join("desktop-organizer-save.json"),
            root: app_root,
        }
    }

    #[test]
    fn provision_creates_a_real_drawer_folder_and_discovery_recovers_its_identity() {
        let root =
            std::env::temp_dir().join(format!("desktop-organizer-provision-{}", Uuid::new_v4()));
        let paths = fixture_paths(&root);
        let mut drawer = Drawer::new("VIDEO".to_owned(), 0, 0, "monitor".to_owned(), 10);
        let id = drawer.id.clone();

        StorageService::provision_drawer(&mut drawer, &paths)
            .expect("drawer should be provisioned");
        assert!(drawer.folder_path.is_dir());
        assert!(drawer.folder_path.join(".drawer.json").is_file());
        let recovered = StorageService::discover_drawers(&paths).expect("discovery should work");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].id.as_deref(), Some(id.as_str()));
        assert_eq!(recovered[0].name, "VIDEO");
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn same_volume_move_never_overwrites_a_conflict() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-move-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root should exist");
        let source = root.join("origen.txt");
        let destination = root.join("destino.txt");
        fs::write(&source, b"original").expect("source should exist");
        fs::write(&destination, b"existing").expect("destination should exist");

        assert!(StorageService::move_safely(&source, &destination).is_err());
        assert_eq!(
            fs::read(&source).expect("source should remain"),
            b"original"
        );
        assert_eq!(
            fs::read(&destination).expect("destination should remain"),
            b"existing"
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn failed_copy_keeps_the_source_intact() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-copy-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root should exist");
        let source = root.join("origen.txt");
        let blocking_parent = root.join("no-es-carpeta");
        fs::write(&source, b"safe data").expect("source should exist");
        fs::write(&blocking_parent, b"file").expect("blocking file should exist");

        assert!(copy_entry_verified(&source, &blocking_parent.join("destino")).is_err());
        assert_eq!(
            fs::read(&source).expect("source should remain"),
            b"safe data"
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }

    #[test]
    fn windows_folder_names_reject_reserved_and_unsafe_values() {
        for invalid in ["", " CON", "CON", "nul.txt", "fin.", "a/b", "a:b", "cola "] {
            assert!(StorageService::validate_windows_folder_name(invalid).is_err());
        }
        assert!(StorageService::validate_windows_folder_name("Clientes 2026").is_ok());
    }

    #[test]
    fn converting_subdrawer_metadata_never_deletes_folder_content() {
        let root = std::env::temp_dir().join(format!(
            "desktop-organizer-subdrawer-metadata-{}",
            Uuid::new_v4()
        ));
        let folder = root.join("Subcajón");
        fs::create_dir_all(&folder).expect("subdrawer should exist");
        fs::write(folder.join("contenido.txt"), b"safe").expect("content should exist");
        StorageService::write_folder_metadata(&folder, "sub-id", "Subcajón", 1)
            .expect("metadata should exist");
        assert!(
            StorageService::read_folder_metadata(&folder)
                .expect("metadata should read")
                .is_some()
        );
        StorageService::remove_folder_metadata(&folder).expect("metadata should be removed");
        assert_eq!(
            fs::read(folder.join("contenido.txt")).expect("content must remain"),
            b"safe"
        );
        fs::remove_dir_all(root).expect("fixtures should be removed");
    }
}
