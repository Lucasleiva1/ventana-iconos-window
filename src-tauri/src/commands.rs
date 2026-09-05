use std::{
    collections::HashSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

use crate::{
    icon_service::IconService,
    item_repository::{self, AddItemsResult},
    model::{
        COLLAPSED_HEIGHT, Drawer, DrawerGeometryInput, DrawerItem, DrawerPatch, IconSize,
        PersistedState, Preferences, PreferencesPatch, StorageMode,
    },
    monitor_service::{self, MonitorInfo},
    persistence::PersistenceService,
    shell_service::ShellService,
    state::AppState,
    storage_service::{StoragePathsInfo, StorageService, comparable_path},
    window_service,
};

const STATE_CHANGED_EVENT: &str = "drawers:changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryStatus {
    pub master_save_exists: bool,
    pub recoverable_drawers: usize,
    pub storage: StoragePathsInfo,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemActionResult {
    pub drawer: Drawer,
    pub message: String,
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

pub(crate) fn commit(app: &AppHandle, snapshot: &PersistedState) -> Result<(), String> {
    PersistenceService::save(app, snapshot)?;
    app.emit(STATE_CHANGED_EVENT, snapshot)
        .map_err(|error| format!("No se pudo sincronizar el estado entre ventanas: {error}"))
}

fn drawer_from_snapshot(snapshot: &PersistedState, id: &str) -> Result<Drawer, String> {
    snapshot
        .drawers
        .iter()
        .find(|drawer| drawer.id == id)
        .cloned()
        .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())
}

fn drawer_item_from_snapshot(
    snapshot: &PersistedState,
    drawer_id: &str,
    item_id: &str,
) -> Result<DrawerItem, String> {
    drawer_from_snapshot(snapshot, drawer_id)?
        .items
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| "El elemento solicitado ya no existe en este cajón".to_owned())
}

fn replace_drawer(state: &State<'_, AppState>, next: Drawer) -> Result<PersistedState, String> {
    state.update(|app_state| {
        let drawer = app_state
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == next.id)
            .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
        *drawer = next;
        Ok(())
    })
}

fn validate_drawer_caller(window: &WebviewWindow, drawer_id: &str) -> Result<(), String> {
    let caller_id = window_service::drawer_id_from_label(window.label())
        .ok_or_else(|| "Esta ventana no puede modificar el contenido de un cajón".to_owned())?;
    if caller_id == drawer_id {
        Ok(())
    } else {
        Err("La ventana no coincide con el cajón solicitado".to_owned())
    }
}

fn validate_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    StorageService::validate_windows_folder_name(trimmed)?;
    Ok(trimmed.to_owned())
}

fn validate_color(color: &str) -> Result<String, String> {
    let valid = color.len() == 7
        && color.starts_with('#')
        && color
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_hexdigit());
    if valid {
        Ok(color.to_ascii_uppercase())
    } else {
        Err("El color debe tener formato hexadecimal #RRGGBB".to_owned())
    }
}

#[tauri::command]
pub fn get_app_state(state: State<'_, AppState>) -> Result<PersistedState, String> {
    state.snapshot()
}

#[tauri::command]
pub fn get_monitors(window: WebviewWindow) -> Result<Vec<MonitorInfo>, String> {
    monitor_service::monitor_infos(&window)
}

#[tauri::command]
pub fn get_storage_info() -> Result<StoragePathsInfo, String> {
    let paths = StorageService::paths()?;
    Ok(StoragePathsInfo::from(&paths))
}

#[tauri::command]
pub fn open_drawers_root() -> Result<(), String> {
    ShellService::open(&StorageService::paths()?.drawers)
}

#[tauri::command]
pub fn open_dock_root() -> Result<(), String> {
    ShellService::open(&StorageService::paths()?.dock)
}

#[tauri::command]
pub fn get_preferences(app: AppHandle, state: State<'_, AppState>) -> Result<Preferences, String> {
    let mut preferences = state.snapshot()?.preferences;
    preferences.start_with_windows = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| format!("Windows no pudo consultar el inicio automático: {error}"))?;
    Ok(preferences)
}

#[tauri::command]
pub fn update_preferences(
    patch: PreferencesPatch,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Preferences, String> {
    let previous = state.snapshot()?.preferences;
    let mut next = previous.clone();
    if let Some(value) = patch.start_with_windows {
        if value {
            app.autolaunch().enable().map_err(|error| {
                format!("Windows no pudo habilitar el inicio automático por usuario: {error}")
            })?;
        } else {
            app.autolaunch().disable().map_err(|error| {
                format!("Windows no pudo deshabilitar el inicio automático: {error}")
            })?;
        }
        next.start_with_windows = value;
    }
    if let Some(value) = patch.hide_admin_on_minimize {
        next.hide_admin_on_minimize = value;
    }
    if let Some(value) = patch.start_silently {
        next.start_silently = value;
    }
    let snapshot = state.update(|current| {
        current.preferences = next.clone();
        Ok(())
    })?;
    if let Err(error) = commit(&app, &snapshot) {
        if patch.start_with_windows.is_some() {
            let rollback = if previous.start_with_windows {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };
            if let Err(rollback_error) = rollback {
                return Err(format!(
                    "{error}. Además, Windows no pudo restaurar el inicio automático anterior: {rollback_error}"
                ));
            }
        }
        return Err(error);
    }
    Ok(next)
}

#[tauri::command]
pub fn get_recovery_status(state: State<'_, AppState>) -> Result<RecoveryStatus, String> {
    let paths = StorageService::paths()?;
    let snapshot = state.snapshot()?;
    let represented: HashSet<String> = snapshot
        .drawers
        .iter()
        .map(|drawer| comparable_path(&drawer.folder_path))
        .collect();
    let recoverable_drawers = StorageService::discover_drawers(&paths)?
        .into_iter()
        .filter(|drawer| !represented.contains(&comparable_path(&drawer.folder_path)))
        .count();
    Ok(RecoveryStatus {
        master_save_exists: PersistenceService::master_exists()?,
        recoverable_drawers,
        storage: StoragePathsInfo::from(&paths),
        notice: state.startup_notice(),
    })
}

#[tauri::command]
pub fn add_drawer_items(
    drawer_id: String,
    paths: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AddItemsResult, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let storage = StorageService::paths()?;
    let mut drawer = drawer_from_snapshot(&state.snapshot()?, &drawer_id)?;
    let result = item_repository::add_paths(&mut drawer, &storage.desktop, paths, now_millis());
    if result.added.is_empty() {
        return Ok(result);
    }
    for item in &result.added {
        let _ = IconService::ensure(&app, &item.path, &item.icon_key);
    }
    let snapshot = replace_drawer(&state, drawer)?;
    commit(&app, &snapshot).map_err(|error| {
        format!(
            "Los elementos quedaron seguros en su ubicación final, pero no se pudo actualizar el guardado maestro: {error}"
        )
    })?;
    Ok(result)
}

#[tauri::command]
pub fn remove_drawer_item(
    drawer_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let mut drawer = drawer_from_snapshot(&state.snapshot()?, &drawer_id)?;
    item_repository::remove_link(&mut drawer, &item_id, now_millis())?;
    let snapshot = replace_drawer(&state, drawer)?;
    commit(&app, &snapshot)?;
    drawer_from_snapshot(&snapshot, &drawer_id)
}

#[tauri::command]
pub fn restore_drawer_item(
    drawer_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ItemActionResult, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let storage = StorageService::paths()?;
    let mut drawer = drawer_from_snapshot(&state.snapshot()?, &drawer_id)?;
    let item = drawer_item_from_snapshot(&state.snapshot()?, &drawer_id, &item_id)?;
    if item.storage_mode != StorageMode::Managed {
        return Err("Los vínculos externos no necesitan restaurarse".to_owned());
    }
    let file_name = item
        .path
        .file_name()
        .ok_or_else(|| "El elemento no tiene un nombre válido".to_owned())?;
    let destination = storage.desktop.join(file_name);
    let outcome = StorageService::move_safely(&item.path, &destination)?;
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let snapshot = replace_drawer(&state, drawer)?;
    commit(&app, &snapshot).map_err(|error| {
        format!("El elemento se restauró en el Escritorio, pero falló el guardado maestro: {error}")
    })?;
    let message = outcome
        .warning
        .unwrap_or_else(|| format!("Restaurado en {}", destination.display()));
    Ok(ItemActionResult {
        drawer: drawer_from_snapshot(&snapshot, &drawer_id)?,
        message,
    })
}

#[tauri::command]
pub async fn move_managed_item(
    drawer_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ItemActionResult>, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Mover elemento a otra ubicación")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let folder = selected
        .into_path()
        .map_err(|error| format!("La carpeta elegida no es una ruta local: {error}"))?;
    let mut drawer = drawer_from_snapshot(&state.snapshot()?, &drawer_id)?;
    let item = drawer_item_from_snapshot(&state.snapshot()?, &drawer_id, &item_id)?;
    if item.storage_mode != StorageMode::Managed {
        return Err(
            "Sólo los elementos guardados físicamente se pueden mover desde el cajón".to_owned(),
        );
    }
    let file_name = item
        .path
        .file_name()
        .ok_or_else(|| "El elemento no tiene un nombre válido".to_owned())?;
    let destination = folder.join(file_name);
    let outcome = StorageService::move_safely(&item.path, &destination)?;
    item_repository::sync_drawer(&mut drawer, now_millis())?;
    let snapshot = replace_drawer(&state, drawer)?;
    commit(&app, &snapshot).map_err(|error| {
        format!("El elemento se movió, pero falló el guardado maestro: {error}")
    })?;
    let message = outcome
        .warning
        .unwrap_or_else(|| format!("Movido a {}", destination.display()));
    Ok(Some(ItemActionResult {
        drawer: drawer_from_snapshot(&snapshot, &drawer_id)?,
        message,
    }))
}

#[tauri::command]
pub fn set_drawer_icon_size(
    drawer_id: String,
    icon_size: IconSize,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let updated_at = now_millis();
    let snapshot = state.update(|app_state| {
        let drawer = app_state
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == drawer_id)
            .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
        drawer.icon_size = icon_size;
        drawer.updated_at = updated_at;
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    drawer_from_snapshot(&snapshot, &drawer_id)
}

#[tauri::command]
pub fn refresh_drawer_availability(
    drawer_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let mut drawer = drawer_from_snapshot(&state.snapshot()?, &drawer_id)?;
    let changed = item_repository::sync_drawer(&mut drawer, now_millis())?;
    if changed {
        let snapshot = replace_drawer(&state, drawer)?;
        commit(&app, &snapshot)?;
        drawer_from_snapshot(&snapshot, &drawer_id)
    } else {
        Ok(drawer)
    }
}

#[tauri::command]
pub fn open_drawer_item(
    drawer_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let item = drawer_item_from_snapshot(&state.snapshot()?, &drawer_id, &item_id)?;
    if !item.path.exists() {
        let snapshot = state.update(|app_state| {
            let drawer = app_state
                .drawers
                .iter_mut()
                .find(|drawer| drawer.id == drawer_id)
                .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
            if let Some(stored_item) = drawer.items.iter_mut().find(|value| value.id == item_id) {
                stored_item.available = false;
            }
            drawer.updated_at = now_millis();
            Ok(())
        })?;
        commit(&app, &snapshot)?;
        return Err("El elemento ya no está disponible en su ubicación guardada".to_owned());
    }
    if !item.available {
        let snapshot = state.update(|app_state| {
            if let Some(drawer) = app_state
                .drawers
                .iter_mut()
                .find(|drawer| drawer.id == drawer_id)
            {
                if let Some(stored_item) = drawer.items.iter_mut().find(|value| value.id == item_id)
                {
                    stored_item.available = true;
                }
                drawer.updated_at = now_millis();
            }
            Ok(())
        })?;
        commit(&app, &snapshot)?;
    }
    ShellService::open(&item.path)
}

#[tauri::command]
pub fn open_drawer_folder(drawer_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let drawer = drawer_from_snapshot(&state.snapshot()?, &drawer_id)?;
    ShellService::open(&drawer.folder_path)
}

#[tauri::command]
pub fn get_drawer_item_icon(
    drawer_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    validate_drawer_caller(&window, &drawer_id)?;
    let item = drawer_item_from_snapshot(&state.snapshot()?, &drawer_id, &item_id)?;
    if !item.path.exists() {
        return Ok(None);
    }
    IconService::data_url(&app, &item.path, &item.icon_key)
}

#[tauri::command]
pub async fn create_drawer(
    name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    let name = validate_name(&name)?;
    let current = state.snapshot()?;
    let (x, y, monitor_id) = monitor_service::default_placement(&window, current.drawers.len())?;
    let mut drawer = Drawer::new(name, x, y, monitor_id, now_millis());
    let storage = StorageService::paths()?;
    StorageService::provision_drawer(&mut drawer, &storage)?;
    let snapshot = state.update(|app_state| {
        app_state.drawers.push(drawer.clone());
        Ok(())
    })?;
    commit(&app, &snapshot).map_err(|error| {
        format!(
            "La carpeta física quedó creada y puede recuperarse, pero falló el guardado: {error}"
        )
    })?;
    window_service::show_drawer_window(&app, &drawer)?;
    Ok(drawer)
}

#[tauri::command]
pub fn update_drawer(
    id: String,
    patch: DrawerPatch,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    let name = patch.name.as_deref().map(validate_name).transpose()?;
    let color = patch.color.as_deref().map(validate_color).transpose()?;
    let opacity = patch
        .opacity
        .map(|value| {
            if value.is_finite() && (crate::model::MIN_OPACITY..=1.0).contains(&value) {
                Ok(value)
            } else {
                Err(format!(
                    "La opacidad debe estar entre {}% y 100%",
                    (crate::model::MIN_OPACITY * 100.0).round()
                ))
            }
        })
        .transpose()?;
    let mut drawer = drawer_from_snapshot(&state.snapshot()?, &id)?;
    let storage = StorageService::paths()?;
    if let Some(value) = &name
        && drawer.name != *value
    {
        let previous_folder = drawer.folder_path.clone();
        let next_folder = StorageService::rename_drawer_folder(&drawer, value, &storage)?;
        for item in drawer
            .items
            .iter_mut()
            .filter(|item| item.storage_mode == StorageMode::Managed)
        {
            if let Ok(relative) = item.path.strip_prefix(&previous_folder) {
                item.path = next_folder.join(relative);
                item.icon_key = IconService::key_for(&item.path);
            }
        }
        drawer.folder_path = next_folder;
        drawer.name.clone_from(value);
    }
    if let Some(value) = color {
        drawer.color = value;
    }
    if let Some(value) = opacity {
        drawer.opacity = value;
    }
    if let Some(value) = patch.locked {
        drawer.locked = value;
    }
    drawer.updated_at = now_millis();
    StorageService::write_drawer_metadata(&drawer)?;
    let snapshot = replace_drawer(&state, drawer.clone())?;
    commit(&app, &snapshot).map_err(|error| {
        format!("El cajón físico quedó actualizado, pero falló el guardado maestro: {error}")
    })?;

    if let Some(window) = app.get_webview_window(&window_service::drawer_window_label(&id)) {
        if patch.name.is_some() {
            window
                .set_title(&drawer.name)
                .map_err(|error| format!("No se pudo actualizar el título: {error}"))?;
        }
        if patch.locked.is_some() {
            window
                .set_resizable(!drawer.locked && !drawer.collapsed)
                .map_err(|error| format!("No se pudo actualizar el bloqueo: {error}"))?;
        }
    }
    Ok(drawer)
}

#[tauri::command]
pub fn set_drawer_collapsed(
    id: String,
    collapsed: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    let window = app.get_webview_window(&window_service::drawer_window_label(&id));
    let live_size = window.as_ref().and_then(|window| {
        let size = window.inner_size().ok()?;
        let scale = window.scale_factor().ok()?.max(0.1);
        Some((
            f64::from(size.width) / scale,
            f64::from(size.height) / scale,
        ))
    });
    let updated_at = now_millis();
    let snapshot = state.update(|app_state| {
        let drawer = app_state
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == id)
            .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
        if collapsed && !drawer.collapsed {
            if let Some((width, height)) = live_size {
                drawer.expanded_width = width;
                drawer.expanded_height = height;
            } else {
                drawer.expanded_width = drawer.width;
                drawer.expanded_height = drawer.height;
            }
            drawer.width = drawer.expanded_width;
            drawer.height = COLLAPSED_HEIGHT;
        } else if !collapsed && drawer.collapsed {
            drawer.width = drawer.expanded_width;
            drawer.height = drawer.expanded_height;
        }
        drawer.collapsed = collapsed;
        drawer.updated_at = updated_at;
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    let drawer = drawer_from_snapshot(&snapshot, &id)?;
    if let Some(window) = window {
        window_service::apply_drawer_window_state(&window, &drawer)?;
    }
    Ok(drawer)
}

#[tauri::command]
pub async fn set_drawer_hidden(
    id: String,
    hidden: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Drawer, String> {
    let monitors = if hidden {
        Vec::new()
    } else {
        app.get_webview_window("admin")
            .ok_or_else(|| "No se encontró la ventana administradora".to_owned())?
            .available_monitors()
            .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?
    };
    let updated_at = now_millis();
    let snapshot = state.update(|app_state| {
        let drawer = app_state
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == id)
            .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
        drawer.hidden = hidden;
        drawer.updated_at = updated_at;
        if !hidden {
            monitor_service::normalize_drawer(drawer, &monitors);
        }
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    let drawer = drawer_from_snapshot(&snapshot, &id)?;
    if hidden {
        if let Some(window) = app.get_webview_window(&window_service::drawer_window_label(&id)) {
            window
                .hide()
                .map_err(|error| format!("No se pudo ocultar el cajón: {error}"))?;
        }
    } else {
        window_service::show_drawer_window(&app, &drawer)?;
    }
    Ok(drawer)
}

#[tauri::command]
pub async fn set_all_drawers_hidden(
    hidden: bool,
    app: AppHandle,
) -> Result<PersistedState, String> {
    set_all_drawers_visibility(&app, hidden)
}

pub(crate) fn set_all_drawers_visibility(
    app: &AppHandle,
    hidden: bool,
) -> Result<PersistedState, String> {
    let monitors = app
        .get_webview_window("admin")
        .ok_or_else(|| "No se encontró la ventana administradora".to_owned())?
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let updated_at = now_millis();
    let state = app.state::<AppState>();
    let snapshot = state.update(|app_state| {
        for drawer in &mut app_state.drawers {
            drawer.hidden = hidden;
            drawer.updated_at = updated_at;
            if !hidden {
                monitor_service::normalize_drawer(drawer, &monitors);
            }
        }
        Ok(())
    })?;
    commit(app, &snapshot)?;
    for drawer in &snapshot.drawers {
        if hidden {
            if let Some(window) =
                app.get_webview_window(&window_service::drawer_window_label(&drawer.id))
                && let Err(error) = window.hide()
            {
                eprintln!("No se pudo ocultar el cajón {}: {error}", drawer.id);
            }
        } else if let Err(error) = window_service::show_drawer_window(app, drawer) {
            eprintln!("No se pudo mostrar el cajón {}: {error}", drawer.id);
        }
    }
    Ok(snapshot)
}

#[tauri::command]
pub fn delete_drawer(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    let snapshot = state.update(|app_state| {
        let original_length = app_state.drawers.len();
        app_state.drawers.retain(|drawer| drawer.id != id);
        if app_state.drawers.len() == original_length {
            return Err("El cajón solicitado ya no existe".to_owned());
        }
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    if let Some(window) = app.get_webview_window(&window_service::drawer_window_label(&id)) {
        window.destroy().map_err(|error| {
            format!("El cajón visual se eliminó, pero su ventana no pudo cerrarse: {error}")
        })?;
    }
    Ok(snapshot)
}

#[tauri::command]
pub async fn export_configuration(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Exportar configuración de Desktop Organizer")
        .set_file_name(format!(
            "Desktop-Organizer-Backup-v{}.json",
            env!("CARGO_PKG_VERSION")
        ))
        .add_filter("Configuración JSON", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("El destino elegido no es una ruta local: {error}"))?;
    PersistenceService::export_to(&state.snapshot()?, &path)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn import_configuration(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<PersistedState>, String> {
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Importar configuración de Desktop Organizer")
        .add_filter("Configuración JSON", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("El archivo elegido no es una ruta local: {error}"))?;
    let imported = PersistenceService::load_external(&path)?;
    let storage = StorageService::paths()?;
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let mut merged = state.snapshot()?;
    merge_imported_state(&mut merged, imported, &storage.drawers, now_millis())?;
    for drawer in &mut merged.drawers {
        StorageService::provision_drawer(drawer, &storage)?;
        item_repository::sync_drawer(drawer, now_millis())?;
        monitor_service::normalize_drawer(drawer, &monitors);
    }
    let snapshot = state.update(|current| {
        *current = merged.clone();
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    for drawer in &snapshot.drawers {
        if !drawer.hidden {
            window_service::show_drawer_window(&app, drawer)?;
        }
    }
    Ok(Some(snapshot))
}

#[tauri::command]
pub async fn recover_drawers_from_disk(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    let storage = StorageService::paths()?;
    let seeds = StorageService::discover_drawers(&storage)?;
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let mut snapshot = state.snapshot()?;
    let mut represented_paths: HashSet<String> = snapshot
        .drawers
        .iter()
        .map(|drawer| comparable_path(&drawer.folder_path))
        .collect();
    let mut used_ids: HashSet<String> = snapshot
        .drawers
        .iter()
        .map(|drawer| drawer.id.clone())
        .collect();
    for seed in seeds {
        if represented_paths.contains(&comparable_path(&seed.folder_path)) {
            continue;
        }
        let (x, y, monitor_id) =
            monitor_service::default_placement(&window, snapshot.drawers.len())?;
        let recovered_name: String = seed.name.trim().chars().take(80).collect();
        let mut drawer = Drawer::new(
            if recovered_name.is_empty() {
                "Cajón recuperado".to_owned()
            } else {
                recovered_name
            },
            x,
            y,
            monitor_id,
            seed.created_at.unwrap_or_else(now_millis),
        );
        if let Some(id) = seed
            .id
            .filter(|id| !id.trim().is_empty() && !used_ids.contains(id))
        {
            drawer.id = id;
        }
        drawer.folder_path = seed.folder_path;
        drawer.updated_at = now_millis();
        StorageService::provision_drawer(&mut drawer, &storage)?;
        item_repository::sync_drawer(&mut drawer, now_millis())?;
        monitor_service::normalize_drawer(&mut drawer, &monitors);
        represented_paths.insert(comparable_path(&drawer.folder_path));
        used_ids.insert(drawer.id.clone());
        snapshot.drawers.push(drawer);
    }
    let stored = state.update(|current| {
        *current = snapshot.clone();
        Ok(())
    })?;
    commit(&app, &stored)?;
    for drawer in &stored.drawers {
        if !drawer.hidden {
            window_service::show_drawer_window(&app, drawer)?;
        }
    }
    Ok(stored)
}

#[tauri::command]
pub fn start_fresh_configuration(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    if PersistenceService::master_exists()? {
        return Err("Ya existe un guardado maestro. Esta opción sólo se habilita cuando falta la configuración.".to_owned());
    }
    let snapshot = state.update(|current| {
        *current = PersistedState::default();
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    Ok(snapshot)
}

fn merge_imported_state(
    current: &mut PersistedState,
    imported: PersistedState,
    drawers_root: &Path,
    now: u64,
) -> Result<(), String> {
    current.preferences.hide_admin_on_minimize = imported.preferences.hide_admin_on_minimize;
    current.preferences.start_silently = imported.preferences.start_silently;
    let mut imported_ids = HashSet::new();
    for mut incoming in imported.drawers {
        if incoming.id.trim().is_empty() || !imported_ids.insert(incoming.id.clone()) {
            return Err(
                "La configuración importada contiene identificadores de cajón vacíos o repetidos"
                    .to_owned(),
            );
        }
        for item in &incoming.items {
            if item.storage_mode == StorageMode::Linked
                && (!item.path.is_absolute() || item.path.as_os_str().is_empty())
            {
                return Err(format!(
                    "El vínculo “{}” no tiene una ruta absoluta válida",
                    item.display_name
                ));
            }
        }
        if let Some(existing) = current
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == incoming.id)
        {
            existing.name = validate_name(&incoming.name)?;
            existing.x = incoming.x;
            existing.y = incoming.y;
            existing.width = incoming.width;
            existing.height = incoming.height;
            existing.expanded_width = incoming.expanded_width;
            existing.expanded_height = incoming.expanded_height;
            existing.collapsed = incoming.collapsed;
            existing.hidden = incoming.hidden;
            existing.locked = incoming.locked;
            existing.color = validate_color(&incoming.color)?;
            if !incoming.opacity.is_finite()
                || !(crate::model::MIN_OPACITY..=1.0).contains(&incoming.opacity)
            {
                return Err(format!(
                    "El cajón “{}” tiene una opacidad inválida",
                    incoming.name
                ));
            }
            existing.opacity = incoming.opacity;
            existing.monitor_id = incoming.monitor_id;
            existing.icon_size = incoming.icon_size;
            existing.level_orders = incoming.level_orders.clone();
            let known: HashSet<String> = existing
                .items
                .iter()
                .map(|item| comparable_path(&item.path))
                .collect();
            existing
                .items
                .extend(incoming.items.into_iter().filter(|item| {
                    item.storage_mode == StorageMode::Linked
                        && !known.contains(&comparable_path(&item.path))
                }));
            existing.updated_at = now;
        } else {
            incoming.name = validate_name(&incoming.name)?;
            incoming.color = validate_color(&incoming.color)?;
            if !incoming.opacity.is_finite()
                || !(crate::model::MIN_OPACITY..=1.0).contains(&incoming.opacity)
            {
                return Err(format!(
                    "El cajón “{}” tiene una opacidad inválida",
                    incoming.name
                ));
            }
            incoming
                .items
                .retain(|item| item.storage_mode == StorageMode::Linked);
            for item in &mut incoming.items {
                item.drawer_id = incoming.id.clone();
            }
            if !crate::storage_service::is_within(&incoming.folder_path, drawers_root)
                || !incoming.folder_path.is_dir()
            {
                incoming.folder_path = Default::default();
            }
            incoming.updated_at = now;
            current.drawers.push(incoming);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn begin_drawer_drag(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    let id = window_service::drawer_id_from_label(window.label())
        .ok_or_else(|| "Esta ventana no es un cajón".to_owned())?;
    let drawer = drawer_from_snapshot(&state.snapshot()?, id)?;
    if drawer.locked {
        return Ok(());
    }
    window
        .start_dragging()
        .map_err(|error| format!("No se pudo mover el cajón: {error}"))
}

#[tauri::command]
pub fn record_drawer_geometry(
    geometry: DrawerGeometryInput,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let caller_id = window_service::drawer_id_from_label(window.label())
        .ok_or_else(|| "Esta ventana no puede modificar la geometría de un cajón".to_owned())?;
    if caller_id != geometry.id {
        return Err("La ventana no coincide con el cajón solicitado".to_owned());
    }
    if !geometry.width.is_finite()
        || !geometry.height.is_finite()
        || geometry.width <= 0.0
        || geometry.height <= 0.0
    {
        return Err("Windows informó un tamaño de ventana inválido".to_owned());
    }
    let monitor_id = window
        .current_monitor()
        .map_err(|error| format!("No se pudo detectar el monitor actual: {error}"))?
        .as_ref()
        .map(monitor_service::monitor_id);
    let updated_at = now_millis();
    let snapshot = state.update(|app_state| {
        let drawer = app_state
            .drawers
            .iter_mut()
            .find(|drawer| drawer.id == geometry.id)
            .ok_or_else(|| "El cajón solicitado ya no existe".to_owned())?;
        drawer.x = geometry.x;
        drawer.y = geometry.y;
        if let Some(value) = &monitor_id {
            drawer.monitor_id.clone_from(value);
        }
        if !drawer.collapsed {
            drawer.width = geometry.width;
            drawer.height = geometry.height;
            drawer.expanded_width = geometry.width;
            drawer.expanded_height = geometry.height;
        }
        drawer.updated_at = updated_at;
        Ok(())
    })?;
    commit(&app, &snapshot)
}

pub fn mark_drawer_hidden(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let updated_at = now_millis();
    match state.update(|app_state| {
        if let Some(drawer) = app_state.drawers.iter_mut().find(|drawer| drawer.id == id) {
            drawer.hidden = true;
            drawer.updated_at = updated_at;
        }
        Ok(())
    }) {
        Ok(snapshot) => {
            if let Err(error) = commit(app, &snapshot) {
                eprintln!("No se pudo guardar el cajón oculto: {error}");
            }
        }
        Err(error) => eprintln!("No se pudo actualizar el cajón oculto: {error}"),
    }
}

pub fn capture_open_drawer_geometry(app: &AppHandle) {
    struct Geometry {
        id: String,
        x: i32,
        y: i32,
        width: f64,
        height: f64,
        monitor_id: Option<String>,
    }
    let geometries: Vec<Geometry> = app
        .webview_windows()
        .into_values()
        .filter_map(|window| {
            let id = window_service::drawer_id_from_label(window.label())?.to_owned();
            let position = window.outer_position().ok()?;
            let size = window.inner_size().ok()?;
            let scale = window.scale_factor().ok()?.max(0.1);
            let monitor_id = window
                .current_monitor()
                .ok()
                .flatten()
                .as_ref()
                .map(monitor_service::monitor_id);
            Some(Geometry {
                id,
                x: position.x,
                y: position.y,
                width: f64::from(size.width) / scale,
                height: f64::from(size.height) / scale,
                monitor_id,
            })
        })
        .collect();
    let state = app.state::<AppState>();
    let timestamp = now_millis();
    if let Ok(snapshot) = state.update(|app_state| {
        for geometry in &geometries {
            if let Some(drawer) = app_state
                .drawers
                .iter_mut()
                .find(|drawer| drawer.id == geometry.id)
            {
                drawer.x = geometry.x;
                drawer.y = geometry.y;
                if let Some(value) = &geometry.monitor_id {
                    drawer.monitor_id.clone_from(value);
                }
                if !drawer.collapsed {
                    drawer.width = geometry.width;
                    drawer.height = geometry.height;
                    drawer.expanded_width = geometry.width;
                    drawer.expanded_height = geometry.height;
                }
                drawer.updated_at = timestamp;
            }
        }
        Ok(())
    }) && let Err(error) = PersistenceService::save(app, &snapshot)
    {
        eprintln!("No se pudo guardar la geometría antes de salir: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_imported_state, validate_color, validate_name};
    use crate::model::{Drawer, PersistedState, Preferences, SCHEMA_VERSION};
    use std::path::Path;

    #[test]
    fn validates_and_trims_drawer_names() {
        assert_eq!(validate_name("  VIDEO  ").as_deref(), Ok("VIDEO"));
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"A".repeat(81)).is_err());
    }

    #[test]
    fn accepts_only_six_digit_hex_colors() {
        assert_eq!(validate_color("#29aBcD").as_deref(), Ok("#29ABCD"));
        assert!(validate_color("293548").is_err());
        assert!(validate_color("#12345G").is_err());
    }

    #[test]
    fn import_merges_without_deleting_existing_drawers() {
        let current_drawer = Drawer::new("Actual".to_owned(), 0, 0, "monitor".to_owned(), 1);
        let imported_drawer = Drawer::new("Importado".to_owned(), 20, 20, "monitor".to_owned(), 2);
        let mut current = PersistedState {
            schema_version: SCHEMA_VERSION,
            drawers: vec![current_drawer],
            preferences: Default::default(),
            dock: Default::default(),
            panels: Vec::new(),
        };
        let imported = PersistedState {
            schema_version: SCHEMA_VERSION,
            drawers: vec![imported_drawer],
            preferences: Preferences {
                start_with_windows: true,
                hide_admin_on_minimize: false,
                start_silently: false,
            },
            dock: Default::default(),
            panels: Vec::new(),
        };
        merge_imported_state(&mut current, imported, Path::new("C:\\Cajones"), 3)
            .expect("merge should work");
        assert_eq!(current.drawers.len(), 2);
        assert!(current.drawers.iter().any(|drawer| drawer.name == "Actual"));
        assert!(
            current
                .drawers
                .iter()
                .any(|drawer| drawer.name == "Importado")
        );
        assert!(!current.preferences.start_with_windows);
        assert!(!current.preferences.hide_admin_on_minimize);
        assert!(!current.preferences.start_silently);
    }
}
