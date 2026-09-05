//! Ventanas de los Paneles organizadores.
//!
//! Cada Panel raíz es una ventana Tauri independiente (`panel-<id>`), sin
//! decoración pero **redimensionable de verdad**: al no pedir decoraciones y sí
//! `resizable`, Tauri instala sus bordes nativos de arrastre, de modo que el
//! usuario agarra bordes y esquinas y Windows hace el resize real. El
//! movimiento también es nativo (`start_dragging` desde la cabecera).
//!
//! Los Paneles pertenecen al escritorio: nunca son `always_on_top` y no ocupan
//! la barra de tareas.

use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::{
    model::{PANEL_MIN_HEIGHT, PANEL_MIN_WIDTH, Panel},
    monitor_service::{monitor_for, panel_limits},
};

const PANEL_WINDOW_PREFIX: &str = "panel-";

pub fn panel_window_label(id: &str) -> String {
    format!("{PANEL_WINDOW_PREFIX}{id}")
}

pub fn panel_id_from_label(label: &str) -> Option<&str> {
    label.strip_prefix(PANEL_WINDOW_PREFIX)
}

fn available_monitors(app: &AppHandle) -> Vec<Monitor> {
    app.get_webview_window("admin")
        .or_else(|| app.webview_windows().into_values().next())
        .and_then(|window| window.available_monitors().ok())
        .unwrap_or_default()
}

pub fn create_panel_window(
    app: &AppHandle,
    panel: &Panel,
    monitors: &[Monitor],
) -> Result<WebviewWindow, String> {
    let label = panel_window_label(&panel.id);
    if let Some(existing) = app.get_webview_window(&label) {
        return Ok(existing);
    }

    let (maximum_width, maximum_height) = monitor_for(&panel.monitor_id, panel.x, panel.y, monitors)
        .map(panel_limits)
        .unwrap_or((
            panel.width.max(PANEL_MIN_WIDTH),
            panel.height.max(PANEL_MIN_HEIGHT),
        ));
    let url = WebviewUrl::App(format!("index.html?panel={}", panel.id).into());

    WebviewWindowBuilder::new(app, label, url)
        .title(&panel.name)
        .inner_size(panel.width, panel.height)
        .min_inner_size(PANEL_MIN_WIDTH, PANEL_MIN_HEIGHT)
        .max_inner_size(maximum_width, maximum_height)
        // Bloquear un Panel apaga el resize nativo; desbloquearlo lo devuelve.
        .resizable(!panel.locked)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        // Los Paneles no llenan la barra de tareas ni ensucian Alt+Tab.
        .skip_taskbar(true)
        // Pertenecen al escritorio: nunca por encima de Chrome, DaVinci ni nada.
        .always_on_top(false)
        .visible(false)
        .build()
        .map_err(|error| format!("No se pudo crear la ventana del panel: {error}"))
}

/// Aplica al vuelo el estado del Panel sobre su ventana ya creada.
pub fn apply_panel_window_state(window: &WebviewWindow, panel: &Panel) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| format!("No se pudo detectar el monitor del panel: {error}"))?;
    let (maximum_width, maximum_height) = monitor.as_ref().map(panel_limits).unwrap_or((
        panel.width.max(PANEL_MIN_WIDTH),
        panel.height.max(PANEL_MIN_HEIGHT),
    ));

    window
        .set_title(&panel.name)
        .map_err(|error| format!("No se pudo actualizar el título del panel: {error}"))?;
    window
        .set_min_size(Some(LogicalSize::new(PANEL_MIN_WIDTH, PANEL_MIN_HEIGHT)))
        .map_err(|error| format!("No se pudo establecer el mínimo del panel: {error}"))?;
    window
        .set_max_size(Some(LogicalSize::new(maximum_width, maximum_height)))
        .map_err(|error| format!("No se pudo establecer el máximo del panel: {error}"))?;
    window
        .set_resizable(!panel.locked)
        .map_err(|error| format!("No se pudo actualizar el bloqueo del panel: {error}"))
}

/// Ajusta únicamente el tamaño de la ventana cuando el estado lo cambió desde
/// afuera (por ejemplo al normalizar tras un cambio de monitor).
pub fn apply_panel_size(window: &WebviewWindow, panel: &Panel) -> Result<(), String> {
    window
        .set_size(LogicalSize::new(panel.width, panel.height))
        .map_err(|error| format!("No se pudo actualizar el tamaño del panel: {error}"))
}

pub fn show_panel_window(app: &AppHandle, panel: &Panel) -> Result<(), String> {
    let label = panel_window_label(&panel.id);
    let window = if let Some(window) = app.get_webview_window(&label) {
        window
    } else {
        create_panel_window(app, panel, &available_monitors(app))?
    };

    apply_panel_window_state(&window, panel)?;
    window
        .set_position(PhysicalPosition::new(panel.x, panel.y))
        .map_err(|error| format!("No se pudo restaurar la posición del panel: {error}"))?;
    window
        .show()
        .map_err(|error| format!("No se pudo mostrar el panel: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("No se pudo enfocar el panel: {error}"))
}

pub fn hide_panel_window(app: &AppHandle, id: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&panel_window_label(id)) {
        window
            .hide()
            .map_err(|error| format!("No se pudo ocultar el panel: {error}"))?;
    }
    Ok(())
}

/// Destruye la ventana de un Panel eliminado. Nunca toca archivos del usuario.
pub fn destroy_panel_window(app: &AppHandle, id: &str) {
    if let Some(window) = app.get_webview_window(&panel_window_label(id)) {
        let _ = window.destroy();
    }
}

/// Recrea y coloca todas las ventanas de Paneles visibles al iniciar.
pub fn restore_panels(app: &AppHandle, panels: &[Panel], monitors: &[Monitor]) {
    for panel in panels.iter().filter(|panel| !panel.hidden) {
        match create_panel_window(app, panel, monitors) {
            Ok(window) => {
                let restored = apply_panel_window_state(&window, panel)
                    .and_then(|()| {
                        window
                            .set_position(PhysicalPosition::new(panel.x, panel.y))
                            .map_err(|error| {
                                format!("No se pudo restaurar la posición del panel: {error}")
                            })
                    })
                    .and_then(|()| {
                        window
                            .show()
                            .map_err(|error| format!("No se pudo mostrar el panel: {error}"))
                    });
                if let Err(error) = restored {
                    eprintln!("No se pudo restaurar el panel {}: {error}", panel.id);
                }
            }
            Err(error) => eprintln!("No se pudo recrear el panel {}: {error}", panel.id),
        }
    }
}
