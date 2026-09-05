use tauri::{
    AppHandle, Emitter,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{
    commands, dock_commands, shell_service::ShellService, storage_service::StorageService,
    window_service,
};

pub const TRAY_ID: &str = "desktop-organizer-tray";

const NEW_DRAWER: &str = "new_drawer";
const OPEN_ADMIN: &str = "open_admin";
const SHOW_ALL: &str = "show_all";
const HIDE_ALL: &str = "hide_all";
const OPEN_DRAWERS: &str = "open_drawers";
const SHOW_DOCK: &str = "show_dock";
const HIDE_DOCK: &str = "hide_dock";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";

pub fn setup(app: &AppHandle) -> Result<(), String> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let new_drawer = MenuItem::with_id(app, NEW_DRAWER, "Nuevo cajón", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let open_admin = MenuItem::with_id(app, OPEN_ADMIN, "Abrir administrador", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let show_all = MenuItem::with_id(
        app,
        SHOW_ALL,
        "Mostrar todos los cajones",
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let hide_all = MenuItem::with_id(
        app,
        HIDE_ALL,
        "Ocultar todos los cajones",
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let open_drawers = MenuItem::with_id(
        app,
        OPEN_DRAWERS,
        "Abrir carpeta Cajones",
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let show_dock = MenuItem::with_id(app, SHOW_DOCK, "Mostrar Dock", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let hide_dock = MenuItem::with_id(app, HIDE_DOCK, "Ocultar Dock", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let settings = MenuItem::with_id(app, SETTINGS, "Configuración", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, QUIT, "Salir", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let separator_one = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let separator_two = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let separator_dock = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let menu = Menu::with_items(
        app,
        &[
            &new_drawer,
            &open_admin,
            &separator_one,
            &show_all,
            &hide_all,
            &open_drawers,
            &separator_dock,
            &show_dock,
            &hide_dock,
            &settings,
            &separator_two,
            &quit,
        ],
    )
    .map_err(|error| format!("No se pudo crear el menú del área de notificación: {error}"))?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Desktop Organizer")
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && let Err(error) = window_service::show_admin_window(tray.app_handle())
            {
                eprintln!(
                    "No se pudo abrir el administrador desde el área de notificación: {error}"
                );
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .build(app)
        .map_err(|error| format!("No se pudo crear el icono del área de notificación: {error}"))?;
    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let result = match event.id().as_ref() {
        NEW_DRAWER => window_service::show_admin_window(app).and_then(|()| {
            app.emit("admin:focus-new-drawer", ())
                .map_err(|error| error.to_string())
        }),
        OPEN_ADMIN => window_service::show_admin_window(app),
        SHOW_ALL => commands::set_all_drawers_visibility(app, false).map(|_| ()),
        HIDE_ALL => commands::set_all_drawers_visibility(app, true).map(|_| ()),
        OPEN_DRAWERS => {
            StorageService::paths().and_then(|paths| ShellService::open(&paths.drawers))
        }
        SHOW_DOCK => dock_commands::set_visibility(app, true),
        HIDE_DOCK => dock_commands::set_visibility(app, false),
        SETTINGS => window_service::show_admin_window(app).and_then(|()| {
            app.emit("admin:open-settings", ())
                .map_err(|error| error.to_string())
        }),
        QUIT => {
            commands::capture_open_drawer_geometry(app);
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        eprintln!("Falló una acción del área de notificación: {error}");
    }
}
