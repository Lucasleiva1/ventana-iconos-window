//! Ventanas del Dock y del tirador.
//!
//! El Dock son dos ventanas coordinadas y persistentes:
//!
//! * `dock-handle`: el tirador. Es diminuto (58x14 lógicos), vive siempre en el
//!   borde inferior del área útil y nunca se destruye mientras el Dock esté
//!   activado. Su tamaño es exactamente el del pixel visible, así que no deja
//!   zonas transparentes capturando clics sobre el escritorio.
//! * `dock`: la barra. Se crea una sola vez y se muestra/oculta con `show()` y
//!   `hide()`; nunca se recrea por apertura. Mientras está oculta la ventana no
//!   existe en pantalla, así que no puede robar clics del escritorio.
//!
//! Todas las posiciones se calculan en píxeles físicos a partir del `work_area`
//! real del monitor (que Windows ya entrega sin la barra de tareas) y del
//! `scale_factor` de ese monitor, de modo que el resultado es correcto a
//! cualquier DPI y con la barra de tareas en cualquier borde.
//! La prioridad visual replica a la barra de tareas: queda sobre ventanas
//! normales, pero baja detrás de una aplicación fullscreen del mismo monitor.

use std::{
    mem::size_of,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::Duration,
};

use tauri::{
    AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::Shell::{
        QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        SHQueryUserNotificationState,
    },
    UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    },
};

use crate::{
    model::{
        DOCK_CELL_PADDING, DOCK_HANDLE_GAP, DOCK_HANDLE_HEIGHT, DOCK_HANDLE_WIDTH,
        DOCK_MIN_MANUAL_WIDTH, DOCK_MIN_WIDTH, DOCK_PADDING_X, DOCK_PADDING_Y, DOCK_SIDE_MARGIN,
        DockAnimationMode, DockHandlePosition, DockItemKind, DockState, DockWidthMode,
    },
    monitor_service::monitor_id,
    state::AppState,
};

pub const DOCK_WINDOW: &str = "dock";
pub const DOCK_HANDLE_WINDOW: &str = "dock-handle";

/// Duración de la animación de salida antes de ocultar realmente la ventana.
const HIDE_ANIMATION_MS: u64 = 200;
/// El estado fullscreen no expone una señal confiable para todas las clases de
/// ventana de Windows. Un muestreo de baja frecuencia mantiene la prioridad
/// tipo barra de tareas sin el polling de 4 Hz usado en versiones anteriores.
const FULLSCREEN_GUARD_INTERVAL_MS: u64 = 1_000;

/// Generación de visibilidad: cancela una ocultación programada si el usuario
/// vuelve a abrir el Dock antes de que termine la animación de salida.
static VISIBILITY_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Cerrojo de arranque.
///
/// Las ventanas del Dock sólo pueden crearse o moverse desde el hilo principal
/// una vez que el arranque terminó y el bucle de eventos está corriendo. Los
/// webviews cargan y llaman a comandos del Dock apenas nacen, es decir mientras
/// `setup` todavía se está ejecutando: sin este cerrojo, ese comando intentaba
/// crear las ventanas desde otro hilo al mismo tiempo que el arranque, y la
/// aplicación quedaba trabada con ventanas duplicadas y sin posicionar.
static WINDOWS_READY: AtomicBool = AtomicBool::new(false);

/// Un solo observador de pantalla completa por proceso.
static FULLSCREEN_GUARD_STARTED: AtomicBool = AtomicBool::new(false);
static FOREGROUND_FULLSCREEN: AtomicBool = AtomicBool::new(false);

/// El arranque terminó: ya es seguro tocar las ventanas desde los comandos.
pub fn mark_ready() {
    WINDOWS_READY.store(true, Ordering::SeqCst);
}

pub fn is_ready() -> bool {
    WINDOWS_READY.load(Ordering::SeqCst)
}

/// Mantiene la misma regla visual de la barra de tareas sin tomar el foco:
/// topmost sobre ventanas normales y por debajo de fullscreen.
pub fn start_fullscreen_guard(app: &AppHandle) {
    if FULLSCREEN_GUARD_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let watched_app = app.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(FULLSCREEN_GUARD_INTERVAL_MS));
            if watched_app.get_webview_window(DOCK_HANDLE_WINDOW).is_none() {
                continue;
            }
            let fullscreen =
                shell_reports_fullscreen() || foreground_covers_handle_monitor(&watched_app);
            let previous = FOREGROUND_FULLSCREEN.swap(fullscreen, Ordering::SeqCst);
            if previous == fullscreen {
                continue;
            }
            let callback_app = watched_app.clone();
            if watched_app
                .run_on_main_thread(move || apply_taskbar_priority(&callback_app, fullscreen))
                .is_err()
            {
                break;
            }
        }
    });
}

fn sync_taskbar_priority(app: &AppHandle) -> bool {
    let fullscreen = shell_reports_fullscreen() || foreground_covers_handle_monitor(app);
    FOREGROUND_FULLSCREEN.store(fullscreen, Ordering::SeqCst);
    apply_taskbar_priority(app, fullscreen);
    fullscreen
}

/// Le pregunta a Windows si hay una aplicación ocupando la pantalla completa.
///
/// Es exactamente la misma señal que usa el propio Windows para no mostrar
/// notificaciones encima de un video a pantalla completa, así que reconoce
/// también los reproductores sin bordes que no coinciden con el rectángulo
/// exacto del monitor.
fn shell_reports_fullscreen() -> bool {
    let Ok(state) = (unsafe { SHQueryUserNotificationState() }) else {
        return false;
    };
    state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE
}

/// Aplica al Dock la misma jerarquía que la barra de tareas de Windows.
///
/// Con una aplicación a pantalla completa la barra de tareas no se ve, y el
/// tirador tampoco debe verse. Bajarle la prioridad no alcanza —según el
/// reproductor puede seguir dibujándose encima—, así que directamente se
/// esconde y vuelve al salir.
fn apply_taskbar_priority(app: &AppHandle, fullscreen: bool) {
    for label in [DOCK_WINDOW, DOCK_HANDLE_WINDOW] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_always_on_top(!fullscreen);
        }
    }

    if fullscreen {
        for label in [DOCK_WINDOW, DOCK_HANDLE_WINDOW] {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.hide();
            }
        }
        return;
    }

    let Ok(dock) = app.state::<AppState>().snapshot().map(|state| state.dock) else {
        return;
    };
    if !dock.enabled {
        return;
    }
    if let Some(handle) = app.get_webview_window(DOCK_HANDLE_WINDOW) {
        let _ = handle.show();
    }
    if dock.visible
        && let Some(window) = app.get_webview_window(DOCK_WINDOW)
    {
        let _ = window.show();
    }
}

fn foreground_covers_handle_monitor(app: &AppHandle) -> bool {
    let Some(handle) = app.get_webview_window(DOCK_HANDLE_WINDOW) else {
        return false;
    };
    let Ok(handle_hwnd) = handle.hwnd() else {
        return false;
    };
    let foreground = unsafe { GetForegroundWindow() };
    if foreground == HWND::default()
        || foreground == handle_hwnd
        || !unsafe { IsWindowVisible(foreground) }.as_bool()
        || unsafe { IsIconic(foreground) }.as_bool()
    {
        return false;
    }

    let mut foreground_process = 0_u32;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut foreground_process)) };
    if foreground_process == std::process::id() {
        return false;
    }

    let foreground_monitor = unsafe { MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST) };
    let handle_monitor = unsafe { MonitorFromWindow(handle_hwnd, MONITOR_DEFAULTTONEAREST) };
    if foreground_monitor != handle_monitor {
        return false;
    }

    let mut window_rect = RECT::default();
    if unsafe { GetWindowRect(foreground, &mut window_rect) }.is_err() {
        return false;
    }
    let mut monitor_info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(u32::MAX),
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(foreground_monitor, &mut monitor_info) }.as_bool() {
        return false;
    }
    rect_covers_monitor(window_rect, monitor_info.rcMonitor)
}

fn rect_covers_monitor(window: RECT, monitor: RECT) -> bool {
    const TOLERANCE: i32 = 2;
    window.left <= monitor.left + TOLERANCE
        && window.top <= monitor.top + TOLERANCE
        && window.right >= monitor.right - TOLERANCE
        && window.bottom >= monitor.bottom - TOLERANCE
}

/// Tamaño lógico del Dock: crece con el contenido hasta el ancho disponible.
pub fn logical_size(dock: &DockState, maximum_width: f64) -> (f64, f64) {
    let cell = dock.icon_size.dock_pixels() + DOCK_CELL_PADDING;
    let height = cell + DOCK_PADDING_Y * 2.0;
    let item_widths: Vec<f64> = dock
        .items
        .iter()
        .map(|item| match item.kind {
            DockItemKind::Shortcut => cell,
            DockItemKind::Separator => 12.0,
        })
        .collect();
    let content = if item_widths.is_empty() {
        0.0
    } else {
        item_widths.iter().sum::<f64>()
            + (item_widths.len().saturating_sub(1) as f64) * dock.spacing.logical_gap()
    };
    let cap = maximum_width.max(DOCK_MIN_MANUAL_WIDTH);
    let width = match dock.width_mode {
        DockWidthMode::Automatic => (content + DOCK_PADDING_X * 2.0).max(DOCK_MIN_WIDTH),
        DockWidthMode::Manual => dock.manual_width.max(DOCK_MIN_MANUAL_WIDTH),
    }
    .min(cap);
    (width, height)
}

fn reference_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("admin")
        .or_else(|| app.get_webview_window(DOCK_HANDLE_WINDOW))
        .or_else(|| app.get_webview_window(DOCK_WINDOW))
        .or_else(|| app.webview_windows().into_values().next())
}

/// Monitor donde vive el Dock. Si el monitor guardado ya no existe, cae al
/// principal y corrige `dock.monitor_id` para que quede persistido.
fn resolve_monitor(app: &AppHandle, dock: &mut DockState) -> Result<Option<Monitor>, String> {
    let Some(window) = reference_window(app) else {
        return Ok(None);
    };
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    if monitors.is_empty() {
        return Ok(None);
    }
    let chosen = monitors
        .iter()
        .find(|monitor| monitor_id(monitor) == dock.monitor_id)
        .cloned()
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| monitors.first().cloned());
    if let Some(monitor) = &chosen {
        dock.monitor_id = monitor_id(monitor);
    }
    Ok(chosen)
}

fn build_window(
    app: &AppHandle,
    label: &str,
    view: &str,
    width: f64,
    height: f64,
) -> Result<WebviewWindow, String> {
    WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(format!("index.html?view={view}").into()),
    )
    .title("Desktop Organizer")
    .inner_size(width, height)
    // Sin un mínimo explícito y sin quitar los botones de sistema, Windows
    // aplica su tamaño mínimo de ventana (136x39) y el tirador nace deformado.
    .min_inner_size(DOCK_HANDLE_WIDTH, DOCK_HANDLE_HEIGHT)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .visible(false)
    // NO usar `.focused(false)`: con esa bandera Windows deja de entregarle a
    // la ventana los archivos arrastrados desde el Explorador. Verificado.
    .build()
    .map_err(|error| format!("No se pudo crear la ventana {label} del Dock: {error}"))
}

/// Crea las dos ventanas si faltan. Es idempotente: con single-instance nunca
/// puede haber un segundo Dock ni un segundo tirador.
pub fn ensure_windows(app: &AppHandle, dock: &DockState) -> Result<(), String> {
    if app.get_webview_window(DOCK_HANDLE_WINDOW).is_none() {
        build_window(
            app,
            DOCK_HANDLE_WINDOW,
            "dock-handle",
            dock.handle_width,
            dock.handle_height,
        )
        .or_else(|error| existing_or_error(app, DOCK_HANDLE_WINDOW, error))?;
    }
    if app.get_webview_window(DOCK_WINDOW).is_none() {
        let (width, height) = logical_size(dock, f64::MAX);
        build_window(app, DOCK_WINDOW, "dock", width, height)
            .or_else(|error| existing_or_error(app, DOCK_WINDOW, error))?;
    }
    Ok(())
}

/// Nunca duplicamos una ventana del Dock: si otro hilo se adelantó a crearla,
/// nos quedamos con la que ya existe en lugar de fabricar una segunda.
fn existing_or_error(app: &AppHandle, label: &str, error: String) -> Result<WebviewWindow, String> {
    app.get_webview_window(label).ok_or(error)
}

fn handle_anchor_x(
    position: DockHandlePosition,
    area_left: i32,
    area_right: i32,
    handle_width: i32,
    side_margin: i32,
) -> i32 {
    match position {
        DockHandlePosition::Left => area_left + side_margin,
        DockHandlePosition::Center => area_left + (area_right - area_left - handle_width) / 2,
        DockHandlePosition::Right => area_right - side_margin - handle_width,
    }
}

fn aligned_dock_x(
    handle_x: i32,
    handle_width: i32,
    dock_width: i32,
    area_left: i32,
    area_right: i32,
) -> i32 {
    let desired = handle_x + handle_width / 2 - dock_width / 2;
    desired.clamp(area_left, area_right - dock_width)
}

fn store_handle_position(
    dock: &mut DockState,
    handle_x: i32,
    scale: f64,
    area_left: i32,
    area_right: i32,
    handle_width: i32,
    side_margin: i32,
) {
    let candidates = [
        DockHandlePosition::Left,
        DockHandlePosition::Center,
        DockHandlePosition::Right,
    ];
    let (position, anchor) = candidates
        .into_iter()
        .map(|position| {
            let anchor =
                handle_anchor_x(position, area_left, area_right, handle_width, side_margin);
            (position, anchor)
        })
        .min_by_key(|(_, anchor)| i64::from(handle_x).abs_diff(i64::from(*anchor)))
        .unwrap_or((DockHandlePosition::Center, handle_x));
    dock.handle_position = position;
    dock.handle_offset = f64::from(handle_x - anchor) / scale;
}

/// Destruye las ventanas del Dock. Se usa al desactivarlo desde el
/// Administrador; `destroy` evita el manejador de cierre que las conserva.
pub fn destroy_windows(app: &AppHandle) {
    for label in [DOCK_WINDOW, DOCK_HANDLE_WINDOW] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.destroy();
        }
    }
}

/// Recalcula tamaño y posición de ambas ventanas en píxeles físicos.
fn layout(app: &AppHandle, dock: &mut DockState) -> Result<(), String> {
    let Some(monitor) = resolve_monitor(app, dock)? else {
        return Ok(());
    };
    let scale = monitor.scale_factor().max(0.1);
    let area = monitor.work_area();
    let available = f64::from(area.size.width) / scale - DOCK_SIDE_MARGIN * 2.0;
    let (width, height) = logical_size(dock, available);
    dock.width = width;
    dock.height = height;

    let handle_width = (dock.handle_width * scale).round().max(1.0) as i32;
    let handle_height = (dock.handle_height * scale).round().max(1.0) as i32;
    let dock_width = (width * scale).round().max(1.0) as i32;
    let dock_height = (height * scale).round().max(1.0) as i32;
    let area_right = area.position.x + i32::try_from(area.size.width).unwrap_or(i32::MAX);
    let area_bottom = area.position.y + i32::try_from(area.size.height).unwrap_or(i32::MAX);

    let side_margin = (16.0 * scale).round() as i32;
    let anchored_x = handle_anchor_x(
        dock.handle_position,
        area.position.x,
        area_right,
        handle_width,
        side_margin,
    );
    let requested_handle_x = anchored_x + (dock.handle_offset * scale).round() as i32;
    let handle_x = requested_handle_x.clamp(area.position.x, area_right - handle_width);
    let handle_y = area_bottom - handle_height;
    let dock_x = aligned_dock_x(
        handle_x,
        handle_width,
        dock_width,
        area.position.x,
        area_right,
    );
    let dock_y =
        (handle_y - dock_height - (DOCK_HANDLE_GAP * scale).round() as i32).max(area.position.y);

    if let Some(window) = app.get_webview_window(DOCK_HANDLE_WINDOW) {
        window
            .set_size(PhysicalSize::new(handle_width, handle_height))
            .map_err(|error| format!("No se pudo dimensionar el tirador: {error}"))?;
        window
            .set_position(PhysicalPosition::new(handle_x, handle_y))
            .map_err(|error| format!("No se pudo ubicar el tirador: {error}"))?;
    }
    if let Some(window) = app.get_webview_window(DOCK_WINDOW) {
        window
            .set_size(PhysicalSize::new(dock_width, dock_height))
            .map_err(|error| format!("No se pudo dimensionar el Dock: {error}"))?;
        window
            .set_position(PhysicalPosition::new(dock_x, dock_y))
            .map_err(|error| format!("No se pudo ubicar el Dock: {error}"))?;
    }
    Ok(())
}

/// Mueve el tirador sólo sobre el eje horizontal del monitor elegido.
///
/// `delta_x_logical` llega desde `PointerEvent.screenX`, que usa píxeles
/// lógicos. La conversión al DPI del monitor se hace acá y la posición real se
/// normaliza tras cada movimiento para que arrastrar contra un borde no deje
/// un desplazamiento acumulado invisible.
pub fn move_handle_by(
    app: &AppHandle,
    dock: &mut DockState,
    delta_x_logical: f64,
) -> Result<(), String> {
    let Some(monitor) = resolve_monitor(app, dock)? else {
        return Ok(());
    };
    let scale = monitor.scale_factor().max(0.1);
    let area = monitor.work_area();
    let area_right = area.position.x + i32::try_from(area.size.width).unwrap_or(i32::MAX);
    let handle_width = (dock.handle_width * scale).round().max(1.0) as i32;
    let side_margin = (16.0 * scale).round() as i32;
    let fallback_x = handle_anchor_x(
        dock.handle_position,
        area.position.x,
        area_right,
        handle_width,
        side_margin,
    ) + (dock.handle_offset * scale).round() as i32;
    let current_x = app
        .get_webview_window(DOCK_HANDLE_WINDOW)
        .and_then(|window| window.outer_position().ok())
        .map_or(fallback_x, |position| position.x);
    let delta_physical = (delta_x_logical * scale).round() as i32;
    let handle_x = current_x
        .saturating_add(delta_physical)
        .clamp(area.position.x, area_right - handle_width);

    store_handle_position(
        dock,
        handle_x,
        scale,
        area.position.x,
        area_right,
        handle_width,
        side_margin,
    );
    layout(app, dock)
}

fn cancel_pending_hide() -> u64 {
    VISIBILITY_EPOCH.fetch_add(1, Ordering::SeqCst) + 1
}

/// Oculta la ventana del Dock una vez terminada la animación de salida.
/// Si el usuario vuelve a abrirlo antes, la ocultación programada se descarta.
fn schedule_hide(app: &AppHandle, delay_ms: u64) {
    let epoch = cancel_pending_hide();
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay_ms));
        if VISIBILITY_EPOCH.load(Ordering::SeqCst) != epoch {
            return;
        }
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if VISIBILITY_EPOCH.load(Ordering::SeqCst) != epoch {
                return;
            }
            if let Some(window) = handle.get_webview_window(DOCK_WINDOW) {
                let _ = window.hide();
            }
        });
    });
}

fn hide_now(app: &AppHandle) {
    cancel_pending_hide();
    if let Some(window) = app.get_webview_window(DOCK_WINDOW) {
        let _ = window.hide();
    }
}

/// Deja las ventanas del Dock coherentes con `dock`.
///
/// `animate_hide` conserva la ventana visible unos milisegundos para que el
/// webview pueda reproducir la animación de salida antes de esconderla.
pub fn refresh(app: &AppHandle, dock: &mut DockState, animate_hide: bool) -> Result<(), String> {
    if !dock.enabled {
        dock.visible = false;
        destroy_windows(app);
        return Ok(());
    }

    ensure_windows(app, dock)?;
    layout(app, dock)?;
    let fullscreen = sync_taskbar_priority(app);

    if let Some(handle) = app.get_webview_window(DOCK_HANDLE_WINDOW)
        && !handle.is_visible().unwrap_or(false)
    {
        handle
            .show()
            .map_err(|error| format!("No se pudo mostrar el tirador: {error}"))?;
        // Windows puede recortar el tamaño de una ventana recién creada; al
        // reaplicarlo ya mostrada, el tirador queda exactamente de 58x14.
        layout(app, dock)?;
    }

    let Some(window) = app.get_webview_window(DOCK_WINDOW) else {
        return Ok(());
    };
    if dock.visible {
        cancel_pending_hide();
        let was_visible = window.is_visible().unwrap_or(false);
        window
            .show()
            .map_err(|error| format!("No se pudo mostrar el Dock: {error}"))?;
        // Sólo tomamos el foco al aparecer. Si ya estaba abierto no se lo
        // robamos al programa que el usuario acaba de lanzar desde el Dock.
        if !was_visible && !fullscreen {
            let _ = window.set_focus();
        }
    } else if animate_hide && window.is_visible().unwrap_or(false) {
        let delay = if dock.performance_mode {
            1
        } else {
            match dock.animation_mode {
                DockAnimationMode::Normal => HIDE_ANIMATION_MS,
                DockAnimationMode::Reduced => 90,
                DockAnimationMode::Disabled => 1,
            }
        };
        schedule_hide(app, delay);
    } else {
        hide_now(app);
    }
    Ok(())
}

/// Reubica el Dock sin tocar su visibilidad. Se usa cuando cambia el DPI o la
/// configuración de monitores, para que el tirador nunca quede fuera de pantalla.
pub fn relayout(app: &AppHandle, dock: &mut DockState) -> Result<(), String> {
    if !dock.enabled {
        return Ok(());
    }
    ensure_windows(app, dock)?;
    layout(app, dock)?;
    sync_taskbar_priority(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{aligned_dock_x, logical_size, rect_covers_monitor};
    use crate::model::{DockItem, DockItemKind, DockState, DockWidthMode, DrawerItemType};
    use windows::Win32::Foundation::RECT;

    fn item(index: u32, kind: DockItemKind) -> DockItem {
        DockItem {
            id: format!("item-{index}"),
            kind,
            item_type: DrawerItemType::File,
            display_name: format!("Elemento {index}"),
            path: PathBuf::from(format!("C:\\fixture\\{index}.txt")),
            icon_key: format!("v1-{index}"),
            order: index,
            available: true,
            created_at: 1,
        }
    }

    #[test]
    fn three_hundred_items_never_expand_past_the_work_area() {
        let dock = DockState {
            items: (0..300)
                .map(|index| item(index, DockItemKind::Shortcut))
                .collect(),
            ..DockState::default()
        };

        let (width, _) = logical_size(&dock, 1180.0);

        assert_eq!(width, 1180.0);
    }

    #[test]
    fn manual_width_is_always_capped_by_the_work_area() {
        let dock = DockState {
            width_mode: DockWidthMode::Manual,
            manual_width: 1600.0,
            ..DockState::default()
        };

        let (width, _) = logical_size(&dock, 920.0);

        assert_eq!(width, 920.0);
    }

    #[test]
    fn separators_are_narrower_than_shortcuts_but_remain_in_layout() {
        let with_shortcuts = DockState {
            items: vec![
                item(0, DockItemKind::Shortcut),
                item(1, DockItemKind::Shortcut),
            ],
            ..DockState::default()
        };
        let mut with_separator = with_shortcuts.clone();
        with_separator.items[1].kind = DockItemKind::Separator;

        let (shortcut_width, _) = logical_size(&with_shortcuts, 2000.0);
        let (separator_width, _) = logical_size(&with_separator, 2000.0);

        assert!(separator_width <= shortcut_width);
        assert!(separator_width >= crate::model::DOCK_MIN_WIDTH);
    }

    #[test]
    fn dock_follows_a_centered_handle() {
        assert_eq!(aligned_dock_x(931, 58, 480, 0, 1920), 720);
    }

    #[test]
    fn dock_stays_fully_visible_at_the_left_edge() {
        assert_eq!(aligned_dock_x(0, 58, 480, 0, 1920), 0);
    }

    #[test]
    fn dock_stays_fully_visible_at_the_right_edge() {
        assert_eq!(aligned_dock_x(1862, 58, 480, 0, 1920), 1440);
    }

    #[test]
    fn fullscreen_rect_covers_the_monitor() {
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(rect_covers_monitor(monitor, monitor));
    }

    #[test]
    fn maximized_window_that_leaves_taskbar_visible_is_not_fullscreen() {
        let window = RECT {
            left: -1,
            top: -1,
            right: 1921,
            bottom: 1041,
        };
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(!rect_covers_monitor(window, monitor));
    }

    #[test]
    fn fullscreen_detection_accepts_small_border_rounding() {
        let window = RECT {
            left: 1,
            top: 1,
            right: 1919,
            bottom: 1079,
        };
        let monitor = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        assert!(rect_covers_monitor(window, monitor));
    }
}
