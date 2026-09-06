use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::{
    model::{COLLAPSED_HEIGHT, Drawer, MIN_DRAWER_HEIGHT, MIN_DRAWER_WIDTH},
    monitor_service::{logical_limits, target_monitor},
};

const DRAWER_WINDOW_PREFIX: &str = "drawer-";

pub fn show_admin_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("admin")
        .ok_or_else(|| "No se encontró la ventana administradora".to_owned())?;
    window
        .set_skip_taskbar(false)
        .map_err(|error| format!("No se pudo devolver el administrador a la barra: {error}"))?;
    let _ = window.unminimize();
    window
        .show()
        .map_err(|error| format!("No se pudo mostrar el administrador: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("No se pudo enfocar el administrador: {error}"))
}

pub fn hide_admin_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("admin")
        .ok_or_else(|| "No se encontró la ventana administradora".to_owned())?;
    let _ = window.unminimize();
    window
        .set_skip_taskbar(true)
        .map_err(|error| format!("No se pudo retirar el administrador de la barra: {error}"))?;
    window
        .hide()
        .map_err(|error| format!("No se pudo ocultar el administrador: {error}"))
}

pub fn drawer_window_label(id: &str) -> String {
    format!("{DRAWER_WINDOW_PREFIX}{id}")
}

pub fn drawer_id_from_label(label: &str) -> Option<&str> {
    label.strip_prefix(DRAWER_WINDOW_PREFIX)
}

pub fn create_drawer_window(
    app: &AppHandle,
    drawer: &Drawer,
    monitors: &[Monitor],
) -> Result<WebviewWindow, String> {
    let label = drawer_window_label(&drawer.id);
    if let Some(existing) = app.get_webview_window(&label) {
        return Ok(existing);
    }

    let target = target_monitor(drawer, monitors);
    let (maximum_width, maximum_height) = target.map(logical_limits).unwrap_or((
        drawer.width.max(MIN_DRAWER_WIDTH),
        drawer.expanded_height.max(MIN_DRAWER_HEIGHT),
    ));
    let height = if drawer.collapsed {
        COLLAPSED_HEIGHT
    } else {
        drawer.height
    };
    let url = WebviewUrl::App(format!("index.html?drawer={}", drawer.id).into());

    WebviewWindowBuilder::new(app, label, url)
        .title(&drawer.name)
        .inner_size(drawer.width, height)
        .min_inner_size(
            MIN_DRAWER_WIDTH,
            if drawer.collapsed {
                COLLAPSED_HEIGHT
            } else {
                MIN_DRAWER_HEIGHT
            },
        )
        .max_inner_size(maximum_width, maximum_height)
        .resizable(!drawer.locked && !drawer.collapsed)
        .decorations(false)
        .transparent(true)
        // Sin sombra del sistema: Windows la acompaña con un marco de 1 px
        // recto que asoma en las esquinas por fuera del redondeo. El Dock ya
        // se creaba así.
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(false)
        .visible(false)
        .build()
        .map_err(|error| format!("No se pudo crear la ventana del cajón: {error}"))
}

pub fn apply_drawer_window_state(window: &WebviewWindow, drawer: &Drawer) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| format!("No se pudo detectar el monitor del cajón: {error}"))?;
    let (maximum_width, maximum_height) = monitor.as_ref().map(logical_limits).unwrap_or((
        drawer.width.max(MIN_DRAWER_WIDTH),
        drawer.expanded_height.max(MIN_DRAWER_HEIGHT),
    ));

    window
        .set_title(&drawer.name)
        .map_err(|error| format!("No se pudo actualizar el título: {error}"))?;
    window
        .set_min_size(Some(LogicalSize::new(
            MIN_DRAWER_WIDTH,
            if drawer.collapsed {
                COLLAPSED_HEIGHT
            } else {
                MIN_DRAWER_HEIGHT
            },
        )))
        .map_err(|error| format!("No se pudo establecer el tamaño mínimo: {error}"))?;
    window
        .set_max_size(Some(LogicalSize::new(maximum_width, maximum_height)))
        .map_err(|error| format!("No se pudo establecer el tamaño máximo: {error}"))?;
    window
        .set_size(LogicalSize::new(drawer.width, drawer.height))
        .map_err(|error| format!("No se pudo actualizar el tamaño: {error}"))?;
    window
        .set_resizable(!drawer.locked && !drawer.collapsed)
        .map_err(|error| format!("No se pudo actualizar el bloqueo: {error}"))
}

pub fn show_drawer_window(app: &AppHandle, drawer: &Drawer) -> Result<(), String> {
    let label = drawer_window_label(&drawer.id);
    let window = if let Some(window) = app.get_webview_window(&label) {
        window
    } else {
        let reference = app
            .get_webview_window("admin")
            .ok_or_else(|| "No se encontró la ventana administradora".to_owned())?;
        let monitors = reference
            .available_monitors()
            .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
        create_drawer_window(app, drawer, &monitors)?
    };

    apply_drawer_window_state(&window, drawer)?;
    window
        .set_position(PhysicalPosition::new(drawer.x, drawer.y))
        .map_err(|error| format!("No se pudo restaurar la posición: {error}"))?;
    window
        .show()
        .map_err(|error| format!("No se pudo mostrar el cajón: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("No se pudo enfocar el cajón: {error}"))
}
