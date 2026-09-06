use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

use crate::{
    dock_service,
    log_service::LogService,
    model::{DockState, PersistedState, Preferences, SCHEMA_VERSION},
    monitor_service::{self, MonitorInfo},
    panel_service,
    persistence::{BackupInfo, PersistenceService},
    shell_service::ShellService,
    state::AppState,
    storage_service::{StoragePathsInfo, StorageService},
    window_service,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCenter {
    pub save_valid: bool,
    pub backups: Vec<BackupInfo>,
    pub storage: StoragePathsInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthItem {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub app_version: String,
    pub schema_version: u32,
    pub data_path: String,
    pub save_valid: bool,
    pub drawer_count: usize,
    pub dock_item_count: usize,
    pub panel_count: usize,
    pub monitors: Vec<MonitorInfo>,
    pub start_with_windows: bool,
    pub updater_status: String,
    pub backup_count: usize,
    pub health: Vec<HealthItem>,
    pub text: String,
}

fn require_admin(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "admin" {
        Ok(())
    } else {
        Err("Esta acción sólo está disponible desde Configuración".to_owned())
    }
}

#[tauri::command]
pub fn get_backup_center(window: WebviewWindow) -> Result<BackupCenter, String> {
    require_admin(&window)?;
    let paths = StorageService::paths()?;
    StorageService::ensure_layout(&paths)?;
    let save_valid = paths.master_save.is_file()
        && PersistenceService::load_external(&paths.master_save).is_ok();
    Ok(BackupCenter {
        save_valid,
        backups: PersistenceService::list_backups()?,
        storage: StoragePathsInfo::from(&paths),
    })
}

#[tauri::command]
pub fn create_configuration_backup(window: WebviewWindow) -> Result<BackupInfo, String> {
    require_admin(&window)?;
    let backup = PersistenceService::create_backup_now()?;
    let _ = LogService::info("Backup manual de configuración creado");
    Ok(backup)
}

#[tauri::command]
pub fn restore_configuration_backup(
    file_name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    require_admin(&window)?;
    let restored = PersistenceService::restore_backup(&file_name)?;
    apply_runtime_state(&app, &window, &state, restored).map_err(|error| {
        format!("El backup es válido, pero no se pudo reconstruir la interfaz: {error}")
    })
}

#[tauri::command]
pub fn delete_configuration_backup(file_name: String, window: WebviewWindow) -> Result<(), String> {
    require_admin(&window)?;
    PersistenceService::delete_backup(&file_name)
}

#[tauri::command]
pub async fn export_configuration_backup(
    file_name: String,
    window: WebviewWindow,
    app: AppHandle,
) -> Result<Option<String>, String> {
    require_admin(&window)?;
    let state = PersistenceService::load_backup(&file_name)
        .map_err(|error| format!("El backup seleccionado no es válido: {error}"))?;
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Exportar backup de Desktop Organizer")
        .set_file_name(&file_name)
        .add_filter("Configuración JSON", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let destination = selected
        .into_path()
        .map_err(|error| format!("El destino elegido no es una ruta local: {error}"))?;
    PersistenceService::export_to(&state, &destination)?;
    Ok(Some(destination.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn prepare_application_update(window: WebviewWindow) -> Result<BackupInfo, String> {
    require_admin(&window)?;
    let backup = PersistenceService::create_backup_now()?;
    let _ = LogService::info("Backup automático creado antes de instalar una actualización");
    Ok(backup)
}

#[tauri::command]
pub fn record_update_check(
    notified_version: Option<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Preferences, String> {
    require_admin(&window)?;
    let timestamp = crate::commands::now_millis();
    let snapshot = state.update(|current| {
        current.preferences.last_update_check = timestamp;
        if let Some(version) = notified_version.filter(|value| !value.trim().is_empty()) {
            current.preferences.last_notified_version = Some(version);
        }
        Ok(())
    })?;
    crate::commands::commit(&app, &snapshot)?;
    Ok(snapshot.preferences)
}

#[tauri::command]
pub fn open_data_root(window: WebviewWindow) -> Result<(), String> {
    require_admin(&window)?;
    ShellService::open(&StorageService::paths()?.root)
}

#[tauri::command]
pub fn open_logs_root(window: WebviewWindow) -> Result<(), String> {
    require_admin(&window)?;
    let paths = StorageService::paths()?;
    StorageService::ensure_layout(&paths)?;
    ShellService::open(&paths.logs)
}

#[tauri::command]
pub fn clear_logs(window: WebviewWindow) -> Result<(), String> {
    require_admin(&window)?;
    LogService::clear()
}

#[tauri::command]
pub fn check_application_health(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<HealthItem>, String> {
    require_admin(&window)?;
    health_items(&state.snapshot()?)
}

#[tauri::command]
pub fn get_diagnostic_report(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DiagnosticReport, String> {
    require_admin(&window)?;
    let snapshot = state.snapshot()?;
    let paths = StorageService::paths()?;
    let backups = PersistenceService::list_backups()?;
    let monitors = monitor_service::monitor_infos(&window)?;
    let save_valid = PersistenceService::load_external(&paths.master_save).is_ok();
    let start_with_windows = app.autolaunch().is_enabled().unwrap_or(false);
    let health = health_items(&snapshot)?;
    let data_path = redact_user_profile(&paths.root);
    let scales = monitors
        .iter()
        .map(|monitor| format!("{}: {:.0}%", monitor.name, monitor.scale_factor * 100.0))
        .collect::<Vec<_>>()
        .join(", ");
    let text = format!(
        "Desktop Organizer: {}\nWindows: local\nSchema: {}\nConfiguración: {}\nCajones: {}\nDock items: {}\nPaneles: {}\nMonitores: {} ({})\nInicio con Windows: {}\nBackups: {}\nDatos: {}\nActualizador: firma Tauri configurada, canal Stable",
        env!("CARGO_PKG_VERSION"),
        SCHEMA_VERSION,
        if save_valid { "OK" } else { "con problemas" },
        snapshot.drawers.len(),
        snapshot.dock.items.len(),
        snapshot.panels.len(),
        monitors.len(),
        scales,
        if start_with_windows { "sí" } else { "no" },
        backups.len(),
        data_path,
    );
    Ok(DiagnosticReport {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        schema_version: SCHEMA_VERSION,
        data_path,
        save_valid,
        drawer_count: snapshot.drawers.len(),
        dock_item_count: snapshot.dock.items.len(),
        panel_count: snapshot.panels.len(),
        monitors,
        start_with_windows,
        updater_status: "Firma Tauri configurada · GitHub Releases · Stable".to_owned(),
        backup_count: backups.len(),
        health,
        text,
    })
}

#[tauri::command]
pub fn reset_visual_configuration(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    require_admin(&window)?;
    let _ = PersistenceService::create_backup_now()?;
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let mut next = state.snapshot()?;
    for (index, drawer) in next.drawers.iter_mut().enumerate() {
        let (x, y, monitor_id) = monitor_service::default_placement(&window, index)?;
        drawer.x = x;
        drawer.y = y;
        drawer.monitor_id = monitor_id;
        drawer.width = crate::model::DEFAULT_DRAWER_WIDTH;
        drawer.height = crate::model::DEFAULT_DRAWER_HEIGHT;
        drawer.expanded_width = drawer.width;
        drawer.expanded_height = drawer.height;
        drawer.collapsed = false;
        drawer.hidden = false;
        drawer.locked = false;
        drawer.color = crate::model::DEFAULT_COLOR.to_owned();
        drawer.opacity = 0.94;
        drawer.icon_size = Default::default();
        monitor_service::normalize_drawer(drawer, &monitors);
    }
    let drawer_count = next.drawers.len();
    for (index, panel) in next.panels.iter_mut().enumerate() {
        let (x, y, monitor_id) = monitor_service::default_placement(&window, index + drawer_count)?;
        panel.x = x;
        panel.y = y;
        panel.monitor_id = monitor_id;
        panel.width = crate::model::PANEL_DEFAULT_WIDTH;
        panel.height = crate::model::PANEL_DEFAULT_HEIGHT;
        panel.locked = false;
        panel.hidden = false;
        panel.color = crate::model::DEFAULT_COLOR.to_owned();
        panel.opacity = crate::model::DEFAULT_PANEL_OPACITY;
        panel.density = Default::default();
        panel.icon_mode = Default::default();
        panel.manual_icon_size = crate::model::DEFAULT_MANUAL_ICON_SIZE;
        panel.snap_enabled = true;
        panel.lock_content = false;
        panel.header_mode = Default::default();
        panel.show_title = true;
        panel.background_style = Default::default();
        panel.expanded = false;
        panel.previous_geometry = None;
        monitor_service::normalize_panel(panel, &monitors);
    }
    let items = std::mem::take(&mut next.dock.items);
    let dock = DockState {
        items,
        ..DockState::default()
    };
    next.dock = dock;
    next.preferences.performance_mode = false;
    next.preferences.animation_mode = Default::default();
    apply_runtime_state(&app, &window, &state, next)
}

fn health_items(state: &PersistedState) -> Result<Vec<HealthItem>, String> {
    let paths = StorageService::paths()?;
    let layout = StorageService::ensure_layout(&paths);
    let save = PersistenceService::load_external(&paths.master_save);
    let backups = PersistenceService::list_backups();
    let invalid_backups = backups
        .as_ref()
        .map(|items| items.iter().filter(|item| !item.valid).count())
        .unwrap_or_default();
    Ok(vec![
        HealthItem {
            label: "Configuración".to_owned(),
            ok: save.is_ok(),
            detail: save.err().unwrap_or_else(|| "Save válido".to_owned()),
        },
        HealthItem {
            label: "Cajones".to_owned(),
            ok: paths.drawers.is_dir(),
            detail: format!("{} cajones configurados", state.drawers.len()),
        },
        HealthItem {
            label: "Backups".to_owned(),
            ok: backups.is_ok() && invalid_backups == 0,
            detail: match backups {
                Ok(items) => format!("{} copias; {} inválidas", items.len(), invalid_backups),
                Err(error) => error,
            },
        },
        HealthItem {
            label: "Carpetas internas".to_owned(),
            ok: layout.is_ok(),
            detail: layout
                .err()
                .unwrap_or_else(|| "Estructura disponible".to_owned()),
        },
        HealthItem {
            label: "Actualizador".to_owned(),
            ok: true,
            detail: "GitHub Releases con verificación de firma configurada".to_owned(),
        },
    ])
}

pub(crate) fn apply_runtime_state(
    app: &AppHandle,
    window: &WebviewWindow,
    state: &State<'_, AppState>,
    mut next: PersistedState,
) -> Result<PersistedState, String> {
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    for drawer in &mut next.drawers {
        monitor_service::normalize_drawer(drawer, &monitors);
        for item in &mut drawer.items {
            item.available = item.path.exists();
        }
    }
    for panel in &mut next.panels {
        monitor_service::normalize_panel(panel, &monitors);
        for item in &mut panel.items {
            item.available = item.path.exists();
        }
    }
    monitor_service::spread_overlapping_panels(&mut next.panels, &monitors);
    for item in &mut next.dock.items {
        item.available = item.path.exists();
    }
    let previous_dock = state.snapshot()?.dock;
    let storage = StorageService::paths()?;
    crate::dock_repository::write_metadata(&next.dock, &storage)?;
    let stored = state.update(|current| {
        *current = next.clone();
        Ok(())
    })?;
    PersistenceService::save(app, &stored)?;

    for managed in app.webview_windows().into_values() {
        let label = managed.label();
        if label.starts_with("drawer-") || label.starts_with("panel-") {
            let _ = managed.destroy();
        }
    }
    for drawer in stored.drawers.iter().filter(|drawer| !drawer.hidden) {
        window_service::show_drawer_window(app, drawer)?;
    }
    panel_service::restore_panels(app, &stored.panels, &monitors);
    let mut dock = stored.dock.clone();
    dock_service::refresh(app, &mut dock, false)?;
    if previous_dock.shortcut_enabled != stored.dock.shortcut_enabled
        || previous_dock.shortcut != stored.dock.shortcut
    {
        crate::shortcut_service::replace(
            app,
            previous_dock.shortcut_enabled,
            &previous_dock.shortcut,
            stored.dock.shortcut_enabled,
            &stored.dock.shortcut,
        )?;
    }
    app.emit("dock:changed", &stored.dock)
        .map_err(|error| format!("No se pudo actualizar el Dock: {error}"))?;
    app.emit("drawers:changed", &stored)
        .map_err(|error| format!("No se pudo actualizar la interfaz: {error}"))?;
    let _ = LogService::info("Configuración reconstruida desde una operación de mantenimiento");
    Ok(stored)
}

fn redact_user_profile(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    std::env::var("USERPROFILE")
        .ok()
        .filter(|profile| !profile.is_empty())
        .map(|profile| text.replacen(&profile, "%USERPROFILE%", 1))
        .unwrap_or(text)
}
