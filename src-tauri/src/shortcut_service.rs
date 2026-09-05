use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::model::DockState;

/// Reemplaza únicamente el shortcut administrado por Desktop Organizer.
/// Si Windows rechaza la combinación nueva, se restaura la anterior y el
/// usuario recibe un error comprensible en vez de un fallo silencioso.
pub fn replace(
    app: &AppHandle,
    previous_enabled: bool,
    previous_shortcut: &str,
    enabled: bool,
    shortcut: &str,
) -> Result<(), String> {
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|error| format!("No se pudo liberar el atajo anterior: {error}"))?;

    if !enabled {
        return Ok(());
    }

    let value = shortcut.trim();
    if value.is_empty() {
        restore(app, previous_enabled, previous_shortcut);
        return Err("Elegí una combinación para el shortcut del Dock".to_owned());
    }
    if let Err(error) = manager.register(value) {
        restore(app, previous_enabled, previous_shortcut);
        return Err(format!(
            "Windows no pudo registrar “{value}”. Puede estar en uso por otra aplicación o no ser una combinación válida: {error}"
        ));
    }
    Ok(())
}

pub fn register_config(app: &AppHandle, dock: &DockState) -> Result<(), String> {
    replace(app, false, "", dock.shortcut_enabled, &dock.shortcut)
}

fn restore(app: &AppHandle, enabled: bool, shortcut: &str) {
    if enabled && !shortcut.trim().is_empty() {
        let _ = app.global_shortcut().register(shortcut.trim());
    }
}
