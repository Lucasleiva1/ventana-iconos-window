//! Comandos del Dock.
//!
//! El Dock es un lanzador respaldado por su propia carpeta física en Documentos.

use std::{collections::HashSet, fs, path::PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::{
    commands::{commit, now_millis},
    dock_repository,
    dock_service::{self, DOCK_HANDLE_WINDOW, DOCK_WINDOW},
    icon_service::IconService,
    item_repository::{self, AddItemFailure},
    model::{
        DOCK_MAX_BORDER_RADIUS, DOCK_MAX_HANDLE_HEIGHT, DOCK_MAX_HANDLE_WIDTH,
        DOCK_MAX_HORIZONTAL_OFFSET, DOCK_MAX_MANUAL_WIDTH, DOCK_MIN_BORDER_RADIUS,
        DOCK_MIN_HANDLE_HEIGHT, DOCK_MIN_HANDLE_OPACITY, DOCK_MIN_HANDLE_WIDTH,
        DOCK_MIN_MANUAL_WIDTH, DOCK_MIN_OPACITY, DockItem, DockItemKind, DockPatch, DockState,
        IconSize, PersistedState,
    },
    shell_service::ShellService,
    shortcut_service,
    state::AppState,
    storage_service::StorageService,
};

const DOCK_CHANGED_EVENT: &str = "dock:changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockAddResult {
    pub dock: DockState,
    pub added: usize,
    pub duplicates: usize,
    pub failures: Vec<AddItemFailure>,
}

/// Ventanas autorizadas a operar sobre el Dock. Cualquier otra ventana —
/// incluido un cajón — no puede tocar su contenido.
fn validate_caller(window: &WebviewWindow, allow_admin: bool) -> Result<(), String> {
    let label = window.label();
    let allowed =
        label == DOCK_WINDOW || label == DOCK_HANDLE_WINDOW || (allow_admin && label == "admin");
    if allowed {
        Ok(())
    } else {
        Err("Esta ventana no puede operar sobre el Dock".to_owned())
    }
}

pub(crate) fn emit_dock(app: &AppHandle, dock: &DockState) -> Result<(), String> {
    app.emit(DOCK_CHANGED_EVENT, dock)
        .map_err(|error| format!("No se pudo sincronizar el Dock entre ventanas: {error}"))
}

/// Aplica un cambio al Dock, reconcilia sus ventanas y devuelve el estado ya
/// corregido (monitor real, tamaño real) guardado de vuelta en memoria.
fn apply<F>(
    app: &AppHandle,
    state: &State<'_, AppState>,
    animate_hide: bool,
    mutate: F,
) -> Result<DockState, String>
where
    F: FnOnce(&mut DockState) -> Result<(), String>,
{
    let snapshot = state.update(|app_state| {
        mutate(&mut app_state.dock)?;
        app_state.dock.updated_at = now_millis();
        Ok(())
    })?;
    let mut dock = snapshot.dock;
    // Durante el arranque las ventanas las crea y ubica `setup`. Un comando que
    // llegue antes sólo actualiza el estado en memoria.
    if !dock_service::is_ready() {
        emit_dock(app, &dock)?;
        return Ok(dock);
    }
    dock_service::refresh(app, &mut dock, animate_hide)?;
    let reconciled = dock.clone();
    state.update(move |app_state| {
        app_state.dock.monitor_id = reconciled.monitor_id;
        app_state.dock.width = reconciled.width;
        app_state.dock.height = reconciled.height;
        app_state.dock.visible = reconciled.visible;
        Ok(())
    })?;
    emit_dock(app, &dock)?;
    Ok(dock)
}

/// Persiste el estado completo y avisa a todas las ventanas.
fn persist(app: &AppHandle, state: &State<'_, AppState>) -> Result<(), String> {
    let snapshot = state.snapshot()?;
    let storage = StorageService::paths()?;
    dock_repository::write_metadata(&snapshot.dock, &storage)?;
    commit(app, &snapshot)
}

#[tauri::command]
pub fn get_dock_state(state: State<'_, AppState>) -> Result<DockState, String> {
    Ok(state.snapshot()?.dock)
}

/// Un solo clic en el tirador. Si el Dock acaba de cerrarse solo por perder el
/// foco con ese mismo clic, no vuelve a abrirse.
#[tauri::command]
pub fn toggle_dock(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, true)?;
    let current = state.snapshot()?.dock;
    if !current.enabled {
        return Err("El Dock está desactivado".to_owned());
    }
    let target = !current.visible;
    apply(&app, &state, true, move |dock| {
        dock.visible = target;
        Ok(())
    })
}

#[tauri::command]
pub fn set_dock_visible(
    visible: bool,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, true)?;
    if !state.snapshot()?.dock.enabled {
        return Err("El Dock está desactivado".to_owned());
    }
    apply(&app, &state, true, move |dock| {
        dock.visible = visible;
        Ok(())
    })
}

/// Desplaza el tirador durante un gesto. Actualiza memoria y ventanas en cada
/// tramo, pero difiere la escritura a disco hasta que el usuario lo suelta.
#[tauri::command]
pub fn move_dock_handle(
    delta_x: f64,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    if window.label() != DOCK_HANDLE_WINDOW {
        return Err("Sólo el tirador puede iniciar este movimiento".to_owned());
    }
    ensure_finite(delta_x, "desplazamiento horizontal")?;
    let mut dock = state.snapshot()?.dock;
    if !dock.enabled {
        return Err("El Dock está desactivado".to_owned());
    }
    if !dock_service::is_ready() {
        return Ok(dock);
    }

    dock_service::move_handle_by(&app, &mut dock, delta_x)?;
    let moved = dock.clone();
    state.update(move |app_state| {
        app_state.dock.monitor_id = moved.monitor_id;
        app_state.dock.handle_position = moved.handle_position;
        app_state.dock.handle_offset = moved.handle_offset;
        app_state.dock.width = moved.width;
        app_state.dock.height = moved.height;
        Ok(())
    })?;
    Ok(dock)
}

/// Guarda una sola vez la posición final de un arrastre y sincroniza las demás
/// ventanas para que el Administrador muestre inmediatamente el valor nuevo.
#[tauri::command]
pub fn finish_dock_handle_drag(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    if window.label() != DOCK_HANDLE_WINDOW {
        return Err("Sólo el tirador puede finalizar este movimiento".to_owned());
    }
    let snapshot = state.update(|app_state| {
        app_state.dock.updated_at = now_millis();
        Ok(())
    })?;
    persist(&app, &state)?;
    emit_dock(&app, &snapshot.dock)?;
    Ok(snapshot.dock)
}

/// Mostrar/ocultar desde el System Tray o desde cualquier punto interno.
pub(crate) fn set_visibility(app: &AppHandle, visible: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    if !state.snapshot()?.dock.enabled {
        return Err("El Dock está desactivado".to_owned());
    }
    apply(app, &state, true, move |dock| {
        dock.visible = visible;
        Ok(())
    })
    .map(|_| ())
}

#[tauri::command]
pub fn update_dock_settings(
    patch: DockPatch,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, true)?;
    if let Some(value) = patch.opacity {
        ensure_finite(value, "opacidad")?;
    }
    if let Some(value) = patch.border_radius {
        ensure_finite(value, "radio de borde")?;
    }
    if let Some(value) = patch.manual_width {
        ensure_finite(value, "ancho manual")?;
    }
    if let Some(value) = patch.handle_width {
        ensure_finite(value, "ancho del tirador")?;
    }
    if let Some(value) = patch.handle_height {
        ensure_finite(value, "alto del tirador")?;
    }
    if let Some(value) = patch.handle_opacity {
        ensure_finite(value, "opacidad del tirador")?;
    }
    if let Some(value) = patch.handle_offset {
        ensure_finite(value, "desplazamiento del tirador")?;
    }
    if let Some(value) = &patch.background_color {
        normalize_hex_color(value)?;
    }
    let previous = state.snapshot()?.dock;
    let next_shortcut_enabled = patch.shortcut_enabled.unwrap_or(previous.shortcut_enabled);
    let next_shortcut = patch
        .shortcut
        .as_deref()
        .unwrap_or(&previous.shortcut)
        .trim()
        .to_owned();
    if next_shortcut_enabled != previous.shortcut_enabled || next_shortcut != previous.shortcut {
        shortcut_service::replace(
            &app,
            previous.shortcut_enabled,
            &previous.shortcut,
            next_shortcut_enabled,
            &next_shortcut,
        )?;
    }
    let dock = apply(&app, &state, false, move |dock| {
        if let Some(enabled) = patch.enabled {
            dock.enabled = enabled;
            if !enabled {
                dock.visible = false;
            }
        }
        if let Some(monitor) = patch.monitor_id {
            dock.monitor_id = monitor;
        }
        if let Some(icon_size) = patch.icon_size {
            dock.icon_size = icon_size;
        }
        if let Some(opacity) = patch.opacity {
            if !opacity.is_finite() {
                return Err("La opacidad no es un número válido".to_owned());
            }
            dock.opacity = opacity.clamp(DOCK_MIN_OPACITY, 1.0);
        }
        if let Some(color) = patch.background_color {
            dock.background_color = normalize_hex_color(&color)?;
        }
        if let Some(radius) = patch.border_radius {
            ensure_finite(radius, "radio de borde")?;
            dock.border_radius = radius.clamp(DOCK_MIN_BORDER_RADIUS, DOCK_MAX_BORDER_RADIUS);
        }
        if let Some(width_mode) = patch.width_mode {
            dock.width_mode = width_mode;
        }
        if let Some(width) = patch.manual_width {
            ensure_finite(width, "ancho manual")?;
            dock.manual_width = width.clamp(DOCK_MIN_MANUAL_WIDTH, DOCK_MAX_MANUAL_WIDTH);
        }
        if let Some(spacing) = patch.spacing {
            dock.spacing = spacing;
        }
        if let Some(width) = patch.handle_width {
            ensure_finite(width, "ancho del tirador")?;
            dock.handle_width = width.clamp(DOCK_MIN_HANDLE_WIDTH, DOCK_MAX_HANDLE_WIDTH);
        }
        if let Some(height) = patch.handle_height {
            ensure_finite(height, "alto del tirador")?;
            dock.handle_height = height.clamp(DOCK_MIN_HANDLE_HEIGHT, DOCK_MAX_HANDLE_HEIGHT);
        }
        if let Some(opacity) = patch.handle_opacity {
            ensure_finite(opacity, "opacidad del tirador")?;
            dock.handle_opacity = opacity.clamp(DOCK_MIN_HANDLE_OPACITY, 1.0);
        }
        if let Some(position) = patch.handle_position {
            dock.handle_position = position;
        }
        if let Some(offset) = patch.handle_offset {
            ensure_finite(offset, "desplazamiento del tirador")?;
            dock.handle_offset =
                offset.clamp(-DOCK_MAX_HORIZONTAL_OFFSET, DOCK_MAX_HORIZONTAL_OFFSET);
        }
        if let Some(mode) = patch.animation_mode {
            dock.animation_mode = mode;
        }
        if let Some(performance_mode) = patch.performance_mode {
            dock.performance_mode = performance_mode;
        }
        if let Some(blur) = patch.blur {
            dock.blur = blur;
        }
        if dock.performance_mode {
            dock.blur = false;
        }
        dock.shortcut_enabled = next_shortcut_enabled;
        dock.shortcut = next_shortcut;
        if let Some(hide_after_open) = patch.hide_after_open {
            dock.hide_after_open = hide_after_open;
        }
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

#[tauri::command]
pub fn set_dock_icon_size(
    icon_size: IconSize,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, true)?;
    let dock = apply(&app, &state, false, move |dock| {
        dock.icon_size = icon_size;
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

fn next_order(dock: &DockState) -> u32 {
    dock.items
        .iter()
        .map(|item| item.order)
        .max()
        .map_or(0, |value| value.saturating_add(1))
}

/// Agrega accesos a la carpeta física del Dock. Los elementos del Escritorio
/// se mueven; para orígenes externos se guarda una copia del acceso o un `.lnk`.
#[tauri::command]
pub fn add_dock_items(
    paths: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockAddResult, String> {
    validate_caller(&window, false)?;
    let snapshot = state.snapshot()?;
    let storage = StorageService::paths()?;
    let mut known = HashSet::new();
    let mut order = next_order(&snapshot.dock);
    let now = now_millis();
    let mut added = Vec::new();
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
        let source_key = path.to_string_lossy().replace('/', "\\").to_lowercase();
        if !known.insert(source_key) {
            duplicates += 1;
            continue;
        }
        let item_type = item_repository::classify(&path, &metadata);
        let display = item_repository::display_name(&path, &item_type);
        let destination = match dock_repository::materialize(&path, &display, &storage) {
            Ok(destination) => destination,
            Err(message) => {
                failures.push(AddItemFailure {
                    path: path.to_string_lossy().into_owned(),
                    message,
                });
                continue;
            }
        };
        let stored_metadata = match fs::metadata(&destination) {
            Ok(metadata) => metadata,
            Err(error) => {
                failures.push(AddItemFailure {
                    path: path.to_string_lossy().into_owned(),
                    message: format!(
                        "El acceso se guardó, pero Windows no pudo volver a leerlo: {error}"
                    ),
                });
                continue;
            }
        };
        let stored_type = item_repository::classify(&destination, &stored_metadata);
        let icon_key = IconService::key_for(&destination);
        let _ = IconService::ensure(&app, &destination, &icon_key);
        added.push(DockItem {
            id: Uuid::new_v4().to_string(),
            kind: DockItemKind::Shortcut,
            item_type: stored_type,
            display_name: display,
            path: destination,
            icon_key,
            order,
            available: true,
            created_at: now,
        });
        order = order.saturating_add(1);
    }

    let added_count = added.len();
    let dock = apply(&app, &state, false, move |dock| {
        dock.items.extend(added);
        Ok(())
    })?;
    if added_count > 0 {
        persist(&app, &state)?;
    }
    Ok(DockAddResult {
        dock,
        added: added_count,
        duplicates,
        failures,
    })
}

/// Restaura el elemento físico al Escritorio y recién entonces lo quita del Dock.
#[tauri::command]
pub fn remove_dock_item(
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, false)?;
    let current = dock_item(&state.snapshot()?, &item_id)?;
    let storage = StorageService::paths()?;
    dock_repository::restore_to_desktop(&current, &storage)?;
    let dock = apply(&app, &state, false, move |dock| {
        let before = dock.items.len();
        dock.items.retain(|item| item.id != item_id);
        if dock.items.len() == before {
            return Err("Ese acceso ya no está en el Dock".to_owned());
        }
        for (index, item) in dock.items.iter_mut().enumerate() {
            item.order = u32::try_from(index).unwrap_or(u32::MAX);
        }
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

#[tauri::command]
pub fn reorder_dock_items(
    ordered_item_ids: Vec<String>,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, false)?;
    let dock = apply(&app, &state, false, move |dock| {
        let existing: HashSet<&str> = dock.items.iter().map(|item| item.id.as_str()).collect();
        let requested: HashSet<&str> = ordered_item_ids.iter().map(String::as_str).collect();
        if existing != requested {
            return Err("El nuevo orden no coincide con los accesos del Dock".to_owned());
        }
        dock.items.sort_by_key(|item| {
            ordered_item_ids
                .iter()
                .position(|id| id == &item.id)
                .unwrap_or(usize::MAX)
        });
        for (index, item) in dock.items.iter_mut().enumerate() {
            item.order = u32::try_from(index).unwrap_or(u32::MAX);
        }
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

fn dock_item(snapshot: &PersistedState, item_id: &str) -> Result<DockItem, String> {
    snapshot
        .dock
        .items
        .iter()
        .find(|item| item.id == item_id)
        .cloned()
        .ok_or_else(|| "Ese acceso ya no está en el Dock".to_owned())
}

fn ensure_shortcut(item: &DockItem) -> Result<(), String> {
    if item.kind == DockItemKind::Separator {
        Err("Los separadores no ejecutan ninguna acción".to_owned())
    } else {
        Ok(())
    }
}

fn ensure_finite(value: f64, label: &str) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("El {label} no es un número válido"))
    }
}

fn normalize_hex_color(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        Ok(trimmed.to_ascii_uppercase())
    } else {
        Err("El color del Dock debe tener formato #RRGGBB".to_owned())
    }
}

#[tauri::command]
pub fn add_dock_separator(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, false)?;
    let order = next_order(&state.snapshot()?.dock);
    let separator = DockItem {
        id: Uuid::new_v4().to_string(),
        kind: DockItemKind::Separator,
        item_type: crate::model::DrawerItemType::File,
        display_name: "Separador".to_owned(),
        path: PathBuf::new(),
        icon_key: String::new(),
        order,
        available: true,
        created_at: now_millis(),
    };
    let dock = apply(&app, &state, false, move |dock| {
        dock.items.push(separator);
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

#[tauri::command]
pub fn rename_dock_item(
    item_id: String,
    name: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, false)?;
    let value = name.trim();
    if value.is_empty() || value.chars().count() > 80 {
        return Err("El nombre visual debe tener entre 1 y 80 caracteres".to_owned());
    }
    let target = item_id;
    let value = value.to_owned();
    let dock = apply(&app, &state, false, move |dock| {
        let item = dock
            .items
            .iter_mut()
            .find(|item| item.id == target)
            .ok_or_else(|| "Ese elemento ya no está en el Dock".to_owned())?;
        item.display_name = value;
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

#[tauri::command]
pub async fn repair_dock_item(
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, false)?;
    let item = dock_item(&state.snapshot()?, &item_id)?;
    ensure_shortcut(&item)?;
    let picker = app.dialog().file().set_title("Buscar nueva ubicación");
    let selected = if item.item_type == crate::model::DrawerItemType::Folder {
        picker.blocking_pick_folder()
    } else {
        picker.blocking_pick_file()
    };
    let Some(selected) = selected else {
        return Ok(state.snapshot()?.dock);
    };
    let path = selected
        .into_path()
        .map_err(|_| "La ubicación seleccionada no es una ruta local válida".to_owned())?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("Windows no pudo leer la nueva ubicación: {error}"))?;
    let item_type = item_repository::classify(&path, &metadata);
    let display_name = item_repository::display_name(&path, &item_type);
    let storage = StorageService::paths()?;
    let stored_path = dock_repository::materialize(&path, &display_name, &storage)?;
    let stored_metadata = fs::metadata(&stored_path)
        .map_err(|error| format!("Windows no pudo leer el acceso reparado: {error}"))?;
    let stored_type = item_repository::classify(&stored_path, &stored_metadata);
    let icon_key = IconService::key_for(&stored_path);
    let _ = IconService::ensure(&app, &stored_path, &icon_key);
    let target = item_id;
    let dock = apply(&app, &state, false, move |dock| {
        let stored = dock
            .items
            .iter_mut()
            .find(|value| value.id == target)
            .ok_or_else(|| "Ese acceso ya no está en el Dock".to_owned())?;
        stored.path = stored_path;
        stored.item_type = stored_type;
        stored.icon_key = icon_key;
        stored.available = true;
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

/// Un solo clic abre. Si el acceso apunta a algo que ya no existe, el elemento
/// se marca como no disponible pero jamás se borra solo.
#[tauri::command]
pub fn open_dock_item(
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, false)?;
    let item = dock_item(&state.snapshot()?, &item_id)?;
    ensure_shortcut(&item)?;
    if !item.path.exists() {
        let target = item_id.clone();
        apply(&app, &state, false, move |dock| {
            if let Some(stored) = dock.items.iter_mut().find(|value| value.id == target) {
                stored.available = false;
            }
            Ok(())
        })?;
        persist(&app, &state)?;
        return Err(format!(
            "“{}” ya no está disponible en {}",
            item.display_name,
            item.path.display()
        ));
    }

    ShellService::open(&item.path)?;

    let snapshot = state.snapshot()?;
    let restore_availability = !item.available;
    let hide_after_open = snapshot.dock.hide_after_open;
    let target = item_id;
    let dock = apply(&app, &state, true, move |dock| {
        if restore_availability
            && let Some(stored) = dock.items.iter_mut().find(|value| value.id == target)
        {
            stored.available = true;
        }
        if hide_after_open {
            dock.visible = false;
        }
        Ok(())
    })?;
    if restore_availability {
        persist(&app, &state)?;
    }
    Ok(dock)
}

#[tauri::command]
pub fn open_dock_item_location(
    item_id: String,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_caller(&window, false)?;
    let item = dock_item(&state.snapshot()?, &item_id)?;
    ensure_shortcut(&item)?;
    let parent = item
        .path
        .parent()
        .ok_or_else(|| "Ese acceso no tiene una ubicación padre válida".to_owned())?;
    if !parent.exists() {
        return Err("La ubicación de ese acceso ya no existe".to_owned());
    }
    ShellService::open(parent)
}

#[tauri::command]
pub fn get_dock_item_icon(
    item_id: String,
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    validate_caller(&window, false)?;
    let item = dock_item(&state.snapshot()?, &item_id)?;
    ensure_shortcut(&item)?;
    if !item.path.exists() {
        return Ok(None);
    }
    IconService::data_url(&app, &item.path, &item.icon_key)
}

/// Sincroniza el primer nivel de la carpeta física. Así aparecen los elementos
/// agregados manualmente y se retiran de la vista los que ya no están allí.
#[tauri::command]
pub fn refresh_dock_availability(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, true)?;
    let mut synchronized = state.snapshot()?.dock;
    let storage = StorageService::paths()?;
    dock_repository::sync_dock(&mut synchronized, &storage, now_millis())?;
    for item in &synchronized.items {
        if item.kind != DockItemKind::Separator {
            let _ = IconService::ensure(&app, &item.path, &item.icon_key);
        }
    }
    let synchronized_items = synchronized.items;
    let dock = apply(&app, &state, false, move |dock| {
        dock.items = synchronized_items;
        Ok(())
    })?;
    persist(&app, &state)?;
    Ok(dock)
}

/// Reubica el Dock cuando cambian los monitores, la resolución o el DPI.
#[tauri::command]
pub fn relayout_dock(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DockState, String> {
    validate_caller(&window, true)?;
    apply(&app, &state, false, |_| Ok(()))
}
