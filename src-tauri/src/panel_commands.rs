//! Comandos de los Paneles organizadores.
//!
//! Regla absoluta de este módulo: **un Panel nunca mueve, copia ni borra nada
//! del disco**. Todos sus elementos son referencias. La administración de
//! almacenamiento físico sigue siendo exclusiva del módulo Cajones, y el Dock
//! mantiene la suya propia. Acá no se llama a `StorageService::move_safely`,
//! ni a `dock_repository::materialize`, ni a ninguna operación de archivos.

use std::{collections::HashSet, fs, path::PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::{
    commands::{commit, now_millis},
    icon_service::IconService,
    item_repository::{self, AddItemFailure},
    model::{
        DrawerItemType, PANEL_MAX_ICON, PANEL_MIN_ICON, PANEL_MIN_OPACITY, Panel, PanelAlignment,
        PanelGeometry, PanelGeometryInput, PanelItem, PanelPatch, PersistedState,
    },
    monitor_service::{self},
    panel_service,
    shell_service::ShellService,
    state::AppState,
    storage_service::comparable_path,
    window_service,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelAddResult {
    pub panel: Panel,
    pub added: usize,
    pub duplicates: usize,
    pub failures: Vec<AddItemFailure>,
}

fn panel_from_snapshot(snapshot: &PersistedState, id: &str) -> Result<Panel, String> {
    snapshot
        .panels
        .iter()
        .find(|panel| panel.id == id)
        .cloned()
        .ok_or_else(|| "El panel solicitado ya no existe".to_owned())
}

fn panel_item(panel: &Panel, item_id: &str) -> Result<PanelItem, String> {
    panel
        .items
        .iter()
        .find(|item| item.id == item_id)
        .cloned()
        .ok_or_else(|| "Ese acceso ya no está en el panel".to_owned())
}

fn replace_panel(state: &State<'_, AppState>, next: Panel) -> Result<PersistedState, String> {
    state.update(|app_state| {
        let panel = app_state
            .panels
            .iter_mut()
            .find(|panel| panel.id == next.id)
            .ok_or_else(|| "El panel solicitado ya no existe".to_owned())?;
        *panel = next;
        Ok(())
    })
}

/// Sólo la ventana del propio Panel puede tocar su contenido.
fn validate_panel_window(window: &WebviewWindow, panel_id: &str) -> Result<(), String> {
    match panel_service::panel_id_from_label(window.label()) {
        Some(caller) if caller == panel_id => Ok(()),
        Some(_) => Err("La ventana no coincide con el panel solicitado".to_owned()),
        None => Err("Esta ventana no puede modificar el contenido de un panel".to_owned()),
    }
}

/// El Administrador y la propia ventana del Panel pueden cambiar su configuración.
fn validate_caller(window: &WebviewWindow, panel_id: &str) -> Result<(), String> {
    if window.label() == "admin" {
        return Ok(());
    }
    validate_panel_window(window, panel_id)
}

fn validate_panel_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("El panel necesita un nombre".to_owned());
    }
    if trimmed.chars().count() > 60 {
        return Err("El nombre del panel es demasiado largo".to_owned());
    }
    Ok(trimmed.to_owned())
}

fn validate_color(color: &str) -> Result<String, String> {
    let valid = color.len() == 7
        && color.starts_with('#')
        && color.chars().skip(1).all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(color.to_ascii_uppercase())
    } else {
        Err("El color debe tener formato hexadecimal #RRGGBB".to_owned())
    }
}

fn next_order(panel: &Panel) -> u32 {
    panel
        .items
        .iter()
        .map(|item| item.order)
        .max()
        .map_or(0, |value| value.saturating_add(1))
}

/// Bloquear contenido impide reordenar, renombrar y quitar accesos, pero nunca
/// impide abrirlos. Es independiente del bloqueo de posición.
fn ensure_content_unlocked(panel: &Panel) -> Result<(), String> {
    if panel.lock_content {
        Err("El contenido del panel está bloqueado".to_owned())
    } else {
        Ok(())
    }
}

fn resequence(panel: &mut Panel) {
    for (index, item) in panel.items.iter_mut().enumerate() {
        item.order = u32::try_from(index).unwrap_or(u32::MAX);
    }
}

// --- Ciclo de vida del Panel ------------------------------------------------

#[tauri::command]
pub async fn create_panel(
    name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    let name = validate_panel_name(&name)?;
    let current = state.snapshot()?;
    let (x, y, monitor_id) =
        monitor_service::default_placement(&window, current.drawers.len() + current.panels.len())?;
    let mut panel = Panel::new(name, x, y, monitor_id, now_millis());
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    monitor_service::normalize_panel(&mut panel, &monitors);

    let snapshot = state.update(|app_state| {
        app_state.panels.push(panel.clone());
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    panel_service::show_panel_window(&app, &panel)?;
    Ok(panel)
}

#[tauri::command]
pub fn update_panel(
    id: String,
    patch: PanelPatch,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_caller(&window, &id)?;
    let name = patch.name.as_deref().map(validate_panel_name).transpose()?;
    let color = patch.color.as_deref().map(validate_color).transpose()?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &id)?;
    if let Some(name) = name {
        panel.name = name;
    }
    if let Some(locked) = patch.locked {
        panel.locked = locked;
    }
    if let Some(color) = color {
        panel.color = color;
    }
    if let Some(opacity) = patch.opacity {
        if !opacity.is_finite() {
            return Err("La opacidad no es un número válido".to_owned());
        }
        panel.opacity = opacity.clamp(PANEL_MIN_OPACITY, 1.0);
    }
    if let Some(density) = patch.density {
        panel.density = density;
    }
    if let Some(icon_mode) = patch.icon_mode {
        panel.icon_mode = icon_mode;
    }
    if let Some(size) = patch.manual_icon_size {
        if !size.is_finite() {
            return Err("El tamaño de icono no es un número válido".to_owned());
        }
        panel.manual_icon_size = size.clamp(PANEL_MIN_ICON, PANEL_MAX_ICON);
    }
    if let Some(snap_enabled) = patch.snap_enabled {
        panel.snap_enabled = snap_enabled;
    }
    if let Some(lock_content) = patch.lock_content {
        panel.lock_content = lock_content;
    }
    if let Some(header_mode) = patch.header_mode {
        panel.header_mode = header_mode;
    }
    if let Some(show_title) = patch.show_title {
        panel.show_title = show_title;
    }
    if let Some(background_style) = patch.background_style {
        panel.background_style = background_style;
    }
    panel.updated_at = now_millis();

    let snapshot = replace_panel(&state, panel.clone())?;
    commit(&app, &snapshot)?;
    if let Some(panel_window) = app.get_webview_window(&panel_service::panel_window_label(&id)) {
        panel_service::apply_panel_window_state(&panel_window, &panel)?;
    }
    Ok(panel)
}

#[tauri::command]
pub fn set_panel_hidden(
    id: String,
    hidden: bool,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_caller(&window, &id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &id)?;
    panel.hidden = hidden;
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel.clone())?;
    commit(&app, &snapshot)?;
    if hidden {
        panel_service::hide_panel_window(&app, &id)?;
    } else {
        panel_service::show_panel_window(&app, &panel)?;
    }
    Ok(panel)
}

#[tauri::command]
pub fn set_all_panels_hidden(
    hidden: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    set_all_panels_visibility(&app, &state, hidden)
}

pub(crate) fn set_all_panels_visibility(
    app: &AppHandle,
    state: &State<'_, AppState>,
    hidden: bool,
) -> Result<PersistedState, String> {
    let now = now_millis();
    let snapshot = state.update(|app_state| {
        for panel in &mut app_state.panels {
            if panel.hidden != hidden {
                panel.hidden = hidden;
                panel.updated_at = now;
            }
        }
        Ok(())
    })?;
    commit(app, &snapshot)?;
    for panel in &snapshot.panels {
        let result = if hidden {
            panel_service::hide_panel_window(app, &panel.id)
        } else {
            panel_service::show_panel_window(app, panel)
        };
        if let Err(error) = result {
            eprintln!("No se pudo actualizar el panel {}: {error}", panel.id);
        }
    }
    Ok(snapshot)
}

/// Acción de bandeja: mostrar u ocultar todos los Paneles.
pub(crate) fn set_visibility_from_tray(app: &AppHandle, hidden: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    set_all_panels_visibility(app, &state, hidden).map(|_| ())
}

/// Crea un Panel desde la bandeja, sin pasar por el Administrador.
pub(crate) fn create_panel_from_tray(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let snapshot = state.snapshot()?;
    let reference = app
        .get_webview_window("admin")
        .ok_or_else(|| "No se encontró la ventana administradora".to_owned())?;
    let index = snapshot.drawers.len() + snapshot.panels.len();
    let (x, y, monitor_id) = monitor_service::default_placement(&reference, index)?;
    let name = format!("Panel {}", snapshot.panels.len() + 1);
    let mut panel = Panel::new(name, x, y, monitor_id, now_millis());
    if let Ok(monitors) = reference.available_monitors() {
        monitor_service::normalize_panel(&mut panel, &monitors);
    }
    let snapshot = state.update(|app_state| {
        app_state.panels.push(panel.clone());
        Ok(())
    })?;
    commit(app, &snapshot)?;
    panel_service::show_panel_window(app, &panel)
}

/// Eliminar un Panel borra únicamente su ventana y sus referencias.
/// Ningún archivo, carpeta ni programa del usuario se ve afectado.
#[tauri::command]
pub fn delete_panel(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    let snapshot = state.update(|app_state| {
        let before = app_state.panels.len();
        app_state.panels.retain(|panel| panel.id != id);
        if app_state.panels.len() == before {
            return Err("El panel solicitado ya no existe".to_owned());
        }
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    panel_service::destroy_panel_window(&app, &id);
    Ok(snapshot)
}

/// Cerrar la ventana de un Panel equivale a ocultarlo: la configuración y sus
/// referencias se conservan y se recupera desde el Administrador o la bandeja.
pub(crate) fn mark_panel_hidden(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let target = id.to_owned();
    let now = now_millis();
    let updated = state.update(move |app_state| {
        if let Some(panel) = app_state.panels.iter_mut().find(|panel| panel.id == target) {
            panel.hidden = true;
            panel.updated_at = now;
        }
        Ok(())
    });
    match updated {
        Ok(snapshot) => {
            if let Err(error) = commit(app, &snapshot) {
                eprintln!("No se pudo guardar el panel oculto: {error}");
            }
        }
        Err(error) => eprintln!("No se pudo marcar el panel como oculto: {error}"),
    }
}

// --- Geometría --------------------------------------------------------------

/// `suspend_snap` llega en true cuando el usuario mantenía Alt al empezar a
/// mover: ese gesto se mueve libre aunque el imantado esté activado.
#[tauri::command]
pub fn begin_panel_drag(
    suspend_snap: bool,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let id = panel_service::panel_id_from_label(window.label())
        .ok_or_else(|| "Esta ventana no es un panel".to_owned())?;
    let panel = panel_from_snapshot(&state.snapshot()?, id)?;
    if panel.locked {
        return Ok(());
    }
    panel_service::set_snap_suspended(suspend_snap);
    window
        .start_dragging()
        .map_err(|error| format!("No se pudo mover el panel: {error}"))
}

/// Guarda posición y tamaño reales. La ventana llama a esto con debounce, así
/// que el save no se escribe por cada píxel movido.
#[tauri::command]
pub fn record_panel_geometry(
    geometry: PanelGeometryInput,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_panel_window(&window, &geometry.id)?;
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
        let panel = app_state
            .panels
            .iter_mut()
            .find(|panel| panel.id == geometry.id)
            .ok_or_else(|| "El panel solicitado ya no existe".to_owned())?;
        panel.x = geometry.x;
        panel.y = geometry.y;
        panel.width = geometry.width;
        panel.height = geometry.height;
        if let Some(monitor_id) = monitor_id {
            panel.monitor_id = monitor_id;
        }
        panel.updated_at = updated_at;
        Ok(())
    })?;

    // Imantado: al terminar el gesto, alineamos con los bordes del área útil y
    // con los demás Paneles del mismo monitor. Alt durante el movimiento lo
    // suspende para ese gesto.
    let suspended = panel_service::snap_suspended();
    let panel = panel_from_snapshot(&snapshot, &geometry.id)?;
    if !suspended
        && let Ok(monitors) = window.available_monitors()
        && let Some((x, y)) = panel_service::snapped_position(&panel, &snapshot.panels, &monitors)
        && (x != panel.x || y != panel.y)
    {
        let snapped = state.update(move |app_state| {
            if let Some(stored) = app_state
                .panels
                .iter_mut()
                .find(|stored| stored.id == geometry.id)
            {
                stored.x = x;
                stored.y = y;
            }
            Ok(())
        })?;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        return commit(&app, &snapped);
    }

    commit(&app, &snapshot)
}

/// Expande el Panel al área útil del monitor, o lo devuelve al tamaño anterior.
/// No usa el maximizado de Windows: es un resize controlado.
#[tauri::command]
pub fn set_panel_expanded(
    id: String,
    expanded: bool,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_caller(&window, &id)?;
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let panel = panel_from_snapshot(&state.snapshot()?, &id)?;
    if panel.locked {
        return Err("El panel está bloqueado: desbloqueá la posición primero".to_owned());
    }

    let target = if expanded {
        panel_service::expanded_geometry(&panel, &monitors)
            .ok_or_else(|| "No se pudo calcular el área útil del monitor".to_owned())?
    } else {
        panel.previous_geometry.ok_or_else(|| {
            "No hay un tamaño anterior guardado para restaurar".to_owned()
        })?
    };
    let previous = if expanded {
        Some(PanelGeometry {
            x: panel.x,
            y: panel.y,
            width: panel.width,
            height: panel.height,
        })
    } else {
        None
    };

    let now = now_millis();
    let snapshot = state.update(move |app_state| {
        let stored = app_state
            .panels
            .iter_mut()
            .find(|stored| stored.id == id)
            .ok_or_else(|| "El panel solicitado ya no existe".to_owned())?;
        if expanded {
            stored.previous_geometry = previous;
        } else {
            stored.previous_geometry = None;
        }
        stored.expanded = expanded;
        stored.x = target.x;
        stored.y = target.y;
        stored.width = target.width;
        stored.height = target.height;
        stored.updated_at = now;
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    let updated = snapshot
        .panels
        .iter()
        .find(|stored| stored.id == panel.id)
        .cloned()
        .ok_or_else(|| "El panel solicitado ya no existe".to_owned())?;
    panel_service::apply_geometry(&app, &updated, target)?;
    Ok(updated)
}

/// Recoloca el Panel dentro de su monitor. Se usa cuando cambia el DPI o la
/// configuración de monitores, para que nunca quede fuera de pantalla.
#[tauri::command]
pub fn relayout_panel(
    id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_caller(&window, &id)?;
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &id)?;
    monitor_service::normalize_panel(&mut panel, &monitors);
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel.clone())?;
    commit(&app, &snapshot)?;
    if let Some(panel_window) = app.get_webview_window(&panel_service::panel_window_label(&id)) {
        panel_service::apply_panel_window_state(&panel_window, &panel)?;
        panel_service::apply_panel_size(&panel_window, &panel)?;
        let _ = panel_window.set_position(tauri::PhysicalPosition::new(panel.x, panel.y));
    }
    Ok(panel)
}

// --- Contenido --------------------------------------------------------------

/// Agrega referencias. Jamás mueve ni copia el original: si arrastrás
/// `Escritorio\popes`, `popes` sigue estando en el Escritorio.
#[tauri::command]
pub fn add_panel_items(
    panel_id: String,
    paths: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PanelAddResult, String> {
    validate_panel_window(&window, &panel_id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    let mut known: HashSet<String> = panel
        .items
        .iter()
        .map(|item| comparable_path(&item.path))
        .collect();
    let mut order = next_order(&panel);
    let now = now_millis();
    let mut added = 0_usize;
    let mut duplicates = 0_usize;
    let mut failures = Vec::new();

    for raw in paths {
        let path = PathBuf::from(raw.trim());
        if path.as_os_str().is_empty() {
            continue;
        }
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                failures.push(AddItemFailure {
                    path: path.to_string_lossy().into_owned(),
                    message: format!("Windows no pudo leer el elemento: {error}"),
                });
                continue;
            }
        };
        if !known.insert(comparable_path(&path)) {
            duplicates += 1;
            continue;
        }
        let item_type = item_repository::classify(&path, &metadata);
        let display_name = item_repository::display_name(&path, &item_type);
        let icon_key = IconService::key_for(&path);
        let _ = IconService::ensure(&app, &path, &icon_key);
        panel.items.push(PanelItem {
            id: Uuid::new_v4().to_string(),
            item_type,
            display_name,
            path,
            icon_key,
            order,
            available: true,
            drawer_id: None,
            created_at: now,
        });
        order = order.saturating_add(1);
        added += 1;
    }

    panel.updated_at = now;
    let snapshot = replace_panel(&state, panel)?;
    if added > 0 {
        commit(&app, &snapshot)?;
    }
    Ok(PanelAddResult {
        panel: panel_from_snapshot(&snapshot, &panel_id)?,
        added,
        duplicates,
        failures,
    })
}

/// Coloca un Cajón como elemento especial dentro del Panel.
#[tauri::command]
pub fn add_panel_drawer(
    panel_id: String,
    drawer_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_caller(&window, &panel_id)?;
    let snapshot = state.snapshot()?;
    let drawer = snapshot
        .drawers
        .iter()
        .find(|drawer| drawer.id == drawer_id)
        .cloned()
        .ok_or_else(|| "Ese cajón ya no existe".to_owned())?;
    let mut panel = panel_from_snapshot(&snapshot, &panel_id)?;
    if panel
        .items
        .iter()
        .any(|item| item.drawer_id.as_deref() == Some(drawer_id.as_str()))
    {
        return Err("Ese cajón ya está en el panel".to_owned());
    }
    let now = now_millis();
    let icon_key = IconService::key_for(&drawer.folder_path);
    let _ = IconService::ensure(&app, &drawer.folder_path, &icon_key);
    let order = next_order(&panel);
    panel.items.push(PanelItem {
        id: Uuid::new_v4().to_string(),
        item_type: DrawerItemType::Folder,
        display_name: drawer.name.clone(),
        path: drawer.folder_path.clone(),
        icon_key,
        order,
        available: true,
        drawer_id: Some(drawer_id),
        created_at: now,
    });
    panel.updated_at = now;
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

/// Quitar del Panel elimina sólo la referencia. Nunca borra ni mueve nada.
#[tauri::command]
pub fn remove_panel_item(
    panel_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_panel_window(&window, &panel_id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    ensure_content_unlocked(&panel)?;
    let before = panel.items.len();
    panel.items.retain(|item| item.id != item_id);
    if panel.items.len() == before {
        return Err("Ese acceso ya no está en el panel".to_owned());
    }
    resequence(&mut panel);
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

/// Cambia sólo el nombre visible dentro del Panel. El archivo real no se toca.
#[tauri::command]
pub fn rename_panel_item(
    panel_id: String,
    item_id: String,
    name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_panel_window(&window, &panel_id)?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("El nombre visible no puede quedar vacío".to_owned());
    }
    if trimmed.chars().count() > 80 {
        return Err("El nombre visible es demasiado largo".to_owned());
    }
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    ensure_content_unlocked(&panel)?;
    let item = panel
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .ok_or_else(|| "Ese acceso ya no está en el panel".to_owned())?;
    item.display_name = trimmed.to_owned();
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

#[tauri::command]
pub fn reorder_panel_items(
    panel_id: String,
    ordered_item_ids: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_panel_window(&window, &panel_id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    ensure_content_unlocked(&panel)?;
    let existing: HashSet<&str> = panel.items.iter().map(|item| item.id.as_str()).collect();
    let requested: HashSet<&str> = ordered_item_ids.iter().map(String::as_str).collect();
    if existing != requested {
        return Err("El nuevo orden no coincide con los accesos del panel".to_owned());
    }
    panel.items.sort_by_key(|item| {
        ordered_item_ids
            .iter()
            .position(|id| id == &item.id)
            .unwrap_or(usize::MAX)
    });
    resequence(&mut panel);
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

/// Doble clic. Un Cajón se muestra y se trae al frente; el resto se abre con
/// la aplicación predeterminada de Windows.
#[tauri::command]
pub fn open_panel_item(
    panel_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_panel_window(&window, &panel_id)?;
    let snapshot = state.snapshot()?;
    let panel = panel_from_snapshot(&snapshot, &panel_id)?;
    let item = panel_item(&panel, &item_id)?;

    if let Some(drawer_id) = item.drawer_id.as_deref() {
        let now = now_millis();
        let target = drawer_id.to_owned();
        let updated = state.update(|app_state| {
            let drawer = app_state
                .drawers
                .iter_mut()
                .find(|drawer| drawer.id == target)
                .ok_or_else(|| "Ese cajón ya no existe".to_owned())?;
            drawer.hidden = false;
            drawer.updated_at = now;
            Ok(())
        })?;
        commit(&app, &updated)?;
        let drawer = updated
            .drawers
            .iter()
            .find(|drawer| drawer.id == drawer_id)
            .cloned()
            .ok_or_else(|| "Ese cajón ya no existe".to_owned())?;
        return window_service::show_drawer_window(&app, &drawer);
    }

    if !item.path.exists() {
        mark_availability(&app, &state, &panel_id, &item_id, false)?;
        return Err(format!(
            "“{}” ya no está disponible en {}",
            item.display_name,
            item.path.display()
        ));
    }
    if !item.available {
        mark_availability(&app, &state, &panel_id, &item_id, true)?;
    }
    ShellService::open(&item.path)
}

fn mark_availability(
    app: &AppHandle,
    state: &State<'_, AppState>,
    panel_id: &str,
    item_id: &str,
    available: bool,
) -> Result<(), String> {
    let panel_id = panel_id.to_owned();
    let item_id = item_id.to_owned();
    let now = now_millis();
    let snapshot = state.update(move |app_state| {
        if let Some(panel) = app_state.panels.iter_mut().find(|panel| panel.id == panel_id) {
            if let Some(item) = panel.items.iter_mut().find(|item| item.id == item_id) {
                item.available = available;
            }
            panel.updated_at = now;
        }
        Ok(())
    })?;
    commit(app, &snapshot)
}

#[tauri::command]
pub fn open_panel_item_location(
    panel_id: String,
    item_id: String,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_panel_window(&window, &panel_id)?;
    let panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    let item = panel_item(&panel, &item_id)?;
    let parent = item
        .path
        .parent()
        .ok_or_else(|| "Ese acceso no tiene una ubicación padre válida".to_owned())?;
    if !parent.exists() {
        return Err("La ubicación de ese acceso ya no existe".to_owned());
    }
    ShellService::open(parent)
}

/// Reparar un acceso roto sólo actualiza la referencia guardada.
#[tauri::command]
pub async fn repair_panel_item(
    panel_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_panel_window(&window, &panel_id)?;
    let panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    let item = panel_item(&panel, &item_id)?;
    let picker = app.dialog().file().set_title("Buscar nueva ubicación");
    let selected = if item.item_type == DrawerItemType::Folder {
        picker.blocking_pick_folder()
    } else {
        picker.blocking_pick_file()
    };
    let Some(selected) = selected else {
        return panel_from_snapshot(&state.snapshot()?, &panel_id);
    };
    let path = selected
        .into_path()
        .map_err(|_| "La ubicación seleccionada no es una ruta local válida".to_owned())?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("Windows no pudo leer la nueva ubicación: {error}"))?;
    let item_type = item_repository::classify(&path, &metadata);
    let icon_key = IconService::key_for(&path);
    let _ = IconService::ensure(&app, &path, &icon_key);

    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    let stored = panel
        .items
        .iter_mut()
        .find(|value| value.id == item_id)
        .ok_or_else(|| "Ese acceso ya no está en el panel".to_owned())?;
    stored.path = path;
    stored.item_type = item_type;
    stored.icon_key = icon_key;
    stored.available = true;
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

#[tauri::command]
pub fn get_panel_item_icon(
    panel_id: String,
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    validate_panel_window(&window, &panel_id)?;
    let panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    let item = panel_item(&panel, &item_id)?;
    if !item.path.exists() {
        return Ok(None);
    }
    IconService::data_url(&app, &item.path, &item.icon_key)
}

/// Sin vigilantes de disco: la disponibilidad se revisa cuando el Panel se
/// muestra o cuando el usuario lo pide.
#[tauri::command]
pub fn refresh_panel_availability(
    panel_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_panel_window(&window, &panel_id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    let mut changed = false;
    for item in &mut panel.items {
        let available = item.path.exists();
        if item.available != available {
            item.available = available;
            changed = true;
        }
    }
    if !changed {
        return Ok(panel);
    }
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}


/// Quita varios accesos de una sola vez (selección múltiple).
/// Como siempre, sólo elimina referencias: ningún archivo real se toca.
#[tauri::command]
pub fn remove_panel_items(
    panel_id: String,
    item_ids: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_panel_window(&window, &panel_id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    ensure_content_unlocked(&panel)?;
    let targets: HashSet<&str> = item_ids.iter().map(String::as_str).collect();
    let before = panel.items.len();
    panel.items.retain(|item| !targets.contains(item.id.as_str()));
    if panel.items.len() == before {
        return Err("Ninguno de esos accesos está en el panel".to_owned());
    }
    resequence(&mut panel);
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

/// Duplica un Panel con todos sus accesos.
///
/// Copia sólo referencias: no duplica carpetas, archivos ni ejecutables. El
/// Panel nuevo recibe identificadores propios y es completamente independiente.
#[tauri::command]
pub async fn duplicate_panel(
    id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    let source = panel_from_snapshot(&state.snapshot()?, &id)?;
    let now = now_millis();
    let mut copy = source.clone();
    copy.id = Uuid::new_v4().to_string();
    copy.name = format!("{} copia", source.name);
    copy.x = source.x.saturating_add(28);
    copy.y = source.y.saturating_add(28);
    copy.expanded = false;
    copy.previous_geometry = None;
    copy.created_at = now;
    copy.updated_at = now;
    for item in &mut copy.items {
        item.id = Uuid::new_v4().to_string();
        item.created_at = now;
    }
    if let Ok(monitors) = window.available_monitors() {
        monitor_service::normalize_panel(&mut copy, &monitors);
    }

    let snapshot = state.update(|app_state| {
        app_state.panels.push(copy.clone());
        Ok(())
    })?;
    commit(&app, &snapshot)?;
    panel_service::show_panel_window(&app, &copy)?;
    Ok(copy)
}

/// Revisa referencias y refresca iconos sin vigilantes en segundo plano.
#[tauri::command]
pub fn refresh_panel(
    panel_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Panel, String> {
    validate_caller(&window, &panel_id)?;
    let mut panel = panel_from_snapshot(&state.snapshot()?, &panel_id)?;
    for item in &mut panel.items {
        item.available = item.path.exists();
        if item.available {
            item.icon_key = IconService::key_for(&item.path);
            let _ = IconService::ensure(&app, &item.path, &item.icon_key);
        }
    }
    panel.updated_at = now_millis();
    let snapshot = replace_panel(&state, panel)?;
    commit(&app, &snapshot)?;
    panel_from_snapshot(&snapshot, &panel_id)
}

/// Alinea o distribuye los Paneles visibles y desbloqueados.
#[tauri::command]
pub fn align_panels(
    alignment: PanelAlignment,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PersistedState, String> {
    if window.label() != "admin" {
        return Err("Sólo el Administrador puede alinear paneles".to_owned());
    }
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let snapshot = state.snapshot()?;
    let mut targets: Vec<Panel> = snapshot
        .panels
        .iter()
        .filter(|panel| !panel.hidden && !panel.locked)
        .cloned()
        .collect();
    if targets.len() < 2 {
        return Err("Hacen falta al menos dos paneles visibles y desbloqueados".to_owned());
    }
    targets.sort_by_key(|panel| (panel.x, panel.y));

    let placements = compute_alignment(alignment, &targets, &monitors);
    let now = now_millis();
    let reposition = monitors.clone();
    let updated = state.update(move |app_state| {
        for (id, x, y) in &placements {
            if let Some(panel) = app_state.panels.iter_mut().find(|panel| &panel.id == id) {
                panel.x = *x;
                panel.y = *y;
                panel.updated_at = now;
            }
        }
        for panel in &mut app_state.panels {
            monitor_service::normalize_panel(panel, &reposition);
        }
        Ok(())
    })?;
    commit(&app, &updated)?;
    for panel in updated.panels.iter().filter(|panel| !panel.hidden) {
        if let Some(panel_window) =
            app.get_webview_window(&panel_service::panel_window_label(&panel.id))
        {
            let _ = panel_window.set_position(tauri::PhysicalPosition::new(panel.x, panel.y));
        }
    }
    Ok(updated)
}

fn compute_alignment(
    alignment: PanelAlignment,
    panels: &[Panel],
    monitors: &[tauri::Monitor],
) -> Vec<(String, i32, i32)> {
    let scale_of = |panel: &Panel| {
        monitor_service::monitor_for(&panel.monitor_id, panel.x, panel.y, monitors)
            .map(tauri::Monitor::scale_factor)
            .unwrap_or(1.0)
            .max(0.1)
    };
    let mut placements = Vec::new();
    match alignment {
        PanelAlignment::Left => {
            let left = panels.iter().map(|panel| panel.x).min().unwrap_or_default();
            for panel in panels {
                placements.push((panel.id.clone(), left, panel.y));
            }
        }
        PanelAlignment::Top => {
            let top = panels.iter().map(|panel| panel.y).min().unwrap_or_default();
            for panel in panels {
                placements.push((panel.id.clone(), panel.x, top));
            }
        }
        PanelAlignment::DistributeHorizontally => {
            let mut sorted: Vec<&Panel> = panels.iter().collect();
            sorted.sort_by_key(|panel| panel.x);
            let mut cursor = sorted.first().map(|panel| panel.x).unwrap_or_default();
            for panel in sorted {
                placements.push((panel.id.clone(), cursor, panel.y));
                cursor += (panel.width * scale_of(panel)).round() as i32 + 12;
            }
        }
        PanelAlignment::DistributeVertically => {
            let mut sorted: Vec<&Panel> = panels.iter().collect();
            sorted.sort_by_key(|panel| panel.y);
            let mut cursor = sorted.first().map(|panel| panel.y).unwrap_or_default();
            for panel in sorted {
                placements.push((panel.id.clone(), panel.x, cursor));
                cursor += (panel.height * scale_of(panel)).round() as i32 + 12;
            }
        }
    }
    placements
}
