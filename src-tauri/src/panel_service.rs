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

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::{
    model::{
        PANEL_MIN_HEIGHT, PANEL_MIN_WIDTH, PANEL_SAFETY_MARGIN, PANEL_SNAP_THRESHOLD, Panel,
        PanelGeometry,
    },
    monitor_service::{monitor_for, monitor_id, panel_limits},
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
    let (minimum_width, minimum_height) = panel.minimum_size();

    WebviewWindowBuilder::new(app, label, url)
        .title(&panel.name)
        .inner_size(panel.width, panel.height)
        .min_inner_size(minimum_width, minimum_height)
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
    let (minimum_width, minimum_height) = panel.minimum_size();
    window
        .set_min_size(Some(LogicalSize::new(minimum_width, minimum_height)))
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


// --- Imantado a bordes y a otros Paneles ------------------------------------

/// Alt mantenido al empezar a mover: desactiva el imantado sólo para ese gesto.
static SNAP_SUSPENDED: AtomicBool = AtomicBool::new(false);

pub fn set_snap_suspended(suspended: bool) {
    SNAP_SUSPENDED.store(suspended, Ordering::SeqCst);
}

pub fn snap_suspended() -> bool {
    SNAP_SUSPENDED.swap(false, Ordering::SeqCst)
}

/// Devuelve el candidato más cercano dentro de la tolerancia, si existe.
pub(crate) fn best_candidate(value: i32, candidates: &[i32], threshold: i32) -> Option<i32> {
    candidates
        .iter()
        .copied()
        .map(|candidate| (candidate, (candidate - value).abs()))
        .filter(|(_, distance)| *distance <= threshold)
        .min_by_key(|(_, distance)| *distance)
        .map(|(candidate, _)| candidate)
}

/// Imanta el Panel a los bordes del área útil y a los bordes de los demás
/// Paneles visibles del mismo monitor. Trabaja en píxeles físicos.
pub fn snapped_position(
    panel: &Panel,
    others: &[Panel],
    monitors: &[Monitor],
) -> Option<(i32, i32)> {
    if !panel.snap_enabled || panel.expanded {
        return None;
    }
    let monitor = monitor_for(&panel.monitor_id, panel.x, panel.y, monitors)?;
    let scale = monitor.scale_factor().max(0.1);
    let threshold = (PANEL_SNAP_THRESHOLD * scale).round() as i32;
    let margin = (PANEL_SAFETY_MARGIN * scale).round() as i32;
    let area = monitor.work_area();
    let width = (panel.width * scale).round() as i32;
    let height = (panel.height * scale).round() as i32;
    let area_right = area.position.x + i32::try_from(area.size.width).unwrap_or(i32::MAX);
    let area_bottom = area.position.y + i32::try_from(area.size.height).unwrap_or(i32::MAX);

    let mut xs = vec![
        area.position.x,
        area.position.x + margin,
        area_right - width,
        area_right - width - margin,
    ];
    let mut ys = vec![
        area.position.y,
        area.position.y + margin,
        area_bottom - height,
        area_bottom - height - margin,
    ];

    let target_monitor = monitor_id(monitor);
    for other in others {
        if other.id == panel.id || other.hidden || other.monitor_id != target_monitor {
            continue;
        }
        let other_width = (other.width * scale).round() as i32;
        let other_height = (other.height * scale).round() as i32;
        // Bordes alineados y bordes pegados, en los dos ejes.
        xs.push(other.x);
        xs.push(other.x + other_width - width);
        xs.push(other.x + other_width);
        xs.push(other.x - width);
        ys.push(other.y);
        ys.push(other.y + other_height - height);
        ys.push(other.y + other_height);
        ys.push(other.y - height);
    }

    let x = best_candidate(panel.x, &xs, threshold);
    let y = best_candidate(panel.y, &ys, threshold);
    if x.is_none() && y.is_none() {
        return None;
    }
    Some((x.unwrap_or(panel.x), y.unwrap_or(panel.y)))
}

/// Geometría que ocupa prácticamente el área útil del monitor del Panel.
/// No usa el maximizado de Windows: es un resize controlado, así que el Panel
/// sigue sin bordes, sin barra de tareas y sin estados raros de ventana.
pub fn expanded_geometry(panel: &Panel, monitors: &[Monitor]) -> Option<PanelGeometry> {
    let monitor = monitor_for(&panel.monitor_id, panel.x, panel.y, monitors)?;
    let scale = monitor.scale_factor().max(0.1);
    let area = monitor.work_area();
    let margin = (PANEL_SAFETY_MARGIN * scale).round() as i32;
    Some(PanelGeometry {
        x: area.position.x + margin,
        y: area.position.y + margin,
        width: (f64::from(area.size.width) - f64::from(margin) * 2.0) / scale,
        height: (f64::from(area.size.height) - f64::from(margin) * 2.0) / scale,
    })
}

/// Mueve y redimensiona la ventana de un Panel a una geometría concreta.
pub fn apply_geometry(
    app: &AppHandle,
    panel: &Panel,
    geometry: PanelGeometry,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window(&panel_window_label(&panel.id)) else {
        return Ok(());
    };
    window
        .set_size(LogicalSize::new(geometry.width, geometry.height))
        .map_err(|error| format!("No se pudo redimensionar el panel: {error}"))?;
    window
        .set_position(PhysicalPosition::new(geometry.x, geometry.y))
        .map_err(|error| format!("No se pudo reubicar el panel: {error}"))
}

#[cfg(test)]
mod tests {
    use super::best_candidate;
    use crate::model::{Panel, PanelDensity, PanelHeaderMode};

    #[test]
    fn el_imantado_elige_el_borde_mas_cercano_dentro_de_la_tolerancia() {
        let candidatos = [0, 500, 980];
        assert_eq!(best_candidate(8, &candidatos, 14), Some(0));
        assert_eq!(best_candidate(492, &candidatos, 14), Some(500));
        // Fuera de la tolerancia el Panel se queda donde el usuario lo soltó.
        assert_eq!(best_candidate(240, &candidatos, 14), None);
    }

    #[test]
    fn una_cabecera_compacta_permite_un_panel_mas_chico() {
        let mut normal = Panel::new("A".to_owned(), 0, 0, "m".to_owned(), 1);
        let mut compacto = Panel::new("B".to_owned(), 0, 0, "m".to_owned(), 1);
        compacto.header_mode = PanelHeaderMode::Compact;
        compacto.density = PanelDensity::Compact;
        normal.header_mode = PanelHeaderMode::Normal;
        normal.density = PanelDensity::Wide;

        let (_, alto_normal) = normal.minimum_size();
        let (_, alto_compacto) = compacto.minimum_size();
        assert!(
            alto_compacto < alto_normal,
            "compacto {alto_compacto} debería ser menor que normal {alto_normal}"
        );
    }
}
