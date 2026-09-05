mod commands;
mod dock_commands;
mod dock_service;
mod icon_service;
mod item_repository;
mod level_commands;
mod model;
mod monitor_service;
mod persistence;
mod shell_service;
mod shortcut_service;
mod state;
mod storage_service;
mod tray_service;
mod window_service;

use std::{
    collections::HashSet,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
#[cfg(not(debug_assertions))]
use tauri_plugin_autostart::ManagerExt;

use crate::{persistence::PersistenceService, state::AppState, storage_service::StorageService};

pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let launched_by_autostart = args.iter().any(|arg| arg == "--autostart");
            if !launched_by_autostart && let Err(error) = window_service::show_admin_window(app) {
                eprintln!("No se pudo enfocar la instancia existente: {error}");
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    let Ok(snapshot) = app.state::<AppState>().snapshot() else {
                        return;
                    };
                    if snapshot.dock.enabled
                        && snapshot.dock.shortcut_enabled
                        && let Err(error) =
                            dock_commands::set_visibility(app, !snapshot.dock.visible)
                    {
                        eprintln!("No se pudo alternar el Dock con el shortcut: {error}");
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_handle = app.handle().clone();
            let admin = app
                .get_webview_window("admin")
                .ok_or_else(|| std::io::Error::other("No se creó la ventana administradora"))?;
            let monitors = admin.available_monitors()?;
            let storage = StorageService::paths().map_err(std::io::Error::other)?;
            StorageService::ensure_layout(&storage).map_err(std::io::Error::other)?;
            let physical_drawers =
                StorageService::discover_drawers(&storage).map_err(std::io::Error::other)?;
            let master_exists =
                PersistenceService::master_exists().map_err(std::io::Error::other)?;
            let should_offer_disk_recovery = !master_exists && !physical_drawers.is_empty();
            let (mut persisted, startup_notice) = if should_offer_disk_recovery {
                (model::PersistedState::default(), None)
            } else {
                let loaded = PersistenceService::load(&app_handle).map_err(std::io::Error::other)?;
                (loaded.state, loaded.recovery_notice)
            };
            #[cfg(not(debug_assertions))]
            if persisted.preferences.start_with_windows
                && app_handle.autolaunch().is_enabled().unwrap_or(false)
                && let Err(error) = app_handle.autolaunch().enable()
            {
                eprintln!(
                    "No se pudo actualizar la ruta del inicio automático al ejecutable actual: {error}"
                );
            }
            let startup_now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
                .unwrap_or_default();

            for drawer in &mut persisted.drawers {
                StorageService::provision_drawer(drawer, &storage)
                    .map_err(std::io::Error::other)?;
                item_repository::sync_drawer(drawer, startup_now).map_err(std::io::Error::other)?;
                monitor_service::normalize_drawer(drawer, &monitors);
            }
            if !should_offer_disk_recovery {
                PersistenceService::save(&app_handle, &persisted).map_err(std::io::Error::other)?;
            }
            let mut retained_icon_keys: HashSet<String> = persisted
                .drawers
                .iter()
                .flat_map(|drawer| drawer.items.iter().map(|item| item.icon_key.clone()))
                .collect();
            retained_icon_keys.extend(
                persisted
                    .dock
                    .items
                    .iter()
                    .filter(|item| !item.icon_key.is_empty())
                    .map(|item| item.icon_key.clone()),
            );
            if let Err(error) = icon_service::IconService::prune(&app_handle, &retained_icon_keys) {
                eprintln!("No se pudo limitar la caché de iconos: {error}");
            }
            // El Dock siempre arranca escondido: al iniciar sólo se ve el tirador.
            persisted.dock.visible = false;
            app.manage(AppState::new(persisted.clone(), startup_notice));
            if let Err(error) = shortcut_service::register_config(&app_handle, &persisted.dock) {
                eprintln!("No se pudo registrar el shortcut del Dock al iniciar: {error}");
            }

            tray_service::setup(&app_handle).map_err(std::io::Error::other)?;


            for drawer in persisted.drawers.iter().filter(|drawer| !drawer.hidden) {
                match window_service::create_drawer_window(&app_handle, drawer, &monitors) {
                    Ok(window) => {
                        if let Err(error) =
                            window_service::apply_drawer_window_state(&window, drawer)
                                .and_then(|()| {
                                    window
                                        .set_position(tauri::PhysicalPosition::new(
                                            drawer.x, drawer.y,
                                        ))
                                        .map_err(|error| {
                                            format!("No se pudo restaurar la posición: {error}")
                                        })
                                })
                                .and_then(|()| {
                                    window.show().map_err(|error| {
                                        format!("No se pudo mostrar el cajón: {error}")
                                    })
                                })
                        {
                            eprintln!("No se pudo restaurar el cajón {}: {error}", drawer.id);
                        }
                    }
                    Err(error) => {
                        eprintln!("No se pudo recrear el cajón {}: {error}", drawer.id);
                    }
                }
            }

            // El Dock se crea después de los Cajones: si algo suyo falla, los
            // cajones ya quedaron restaurados. Y antes de mostrar el
            // Administrador, para que el tirador no le saque el foco.
            let mut dock = persisted.dock.clone();
            if let Err(error) = dock_service::refresh(&app_handle, &mut dock, false) {
                eprintln!("No se pudo restaurar el Dock: {error}");
            } else if dock.monitor_id != persisted.dock.monitor_id
                || dock.width != persisted.dock.width
                || dock.height != persisted.dock.height
            {
                let corrected = dock.clone();
                let _ = app_handle.state::<AppState>().update(move |app_state| {
                    app_state.dock.monitor_id = corrected.monitor_id;
                    app_state.dock.width = corrected.width;
                    app_state.dock.height = corrected.height;
                    Ok(())
                });
            }

            let launched_by_autostart = std::env::args().any(|arg| arg == "--autostart");
            if launched_by_autostart && persisted.preferences.start_silently {
                window_service::hide_admin_window(&app_handle).map_err(std::io::Error::other)?;
            } else {
                // En Windows, show() durante setup puede ejecutarse antes de
                // que el loop nativo materialice una ventana creada como
                // visible:false. Una única llamada diferida evita que el
                // Administrador quede oculto en un arranque normal.
                let delayed_app = app_handle.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(120));
                    let main_thread_app = delayed_app.clone();
                    let _ = delayed_app.run_on_main_thread(move || {
                        if let Err(error) = window_service::show_admin_window(&main_thread_app) {
                            eprintln!("No se pudo mostrar el administrador al iniciar: {error}");
                        }
                    });
                });
            }

            // Arranque terminado: recién ahora los comandos pueden crear o mover
            // las ventanas del Dock sin competir con este hilo.
            dock_service::mark_ready();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::get_monitors,
            commands::get_storage_info,
            commands::open_drawers_root,
            commands::get_preferences,
            commands::update_preferences,
            commands::get_recovery_status,
            commands::add_drawer_items,
            commands::remove_drawer_item,
            commands::restore_drawer_item,
            commands::move_managed_item,
            commands::set_drawer_icon_size,
            commands::refresh_drawer_availability,
            commands::open_drawer_item,
            commands::open_drawer_folder,
            commands::get_drawer_item_icon,
            commands::create_drawer,
            commands::update_drawer,
            commands::set_drawer_collapsed,
            commands::set_drawer_hidden,
            commands::set_all_drawers_hidden,
            commands::delete_drawer,
            commands::export_configuration,
            commands::import_configuration,
            commands::recover_drawers_from_disk,
            commands::start_fresh_configuration,
            commands::begin_drawer_drag,
            commands::record_drawer_geometry,
            level_commands::load_drawer_level,
            level_commands::add_drawer_items_at_level,
            level_commands::refresh_drawer_level,
            level_commands::open_level_item,
            level_commands::open_level_item_location,
            level_commands::get_level_item_icon,
            level_commands::remove_level_link,
            level_commands::create_subdrawer,
            level_commands::convert_folder_to_subdrawer,
            level_commands::convert_subdrawer_to_folder,
            level_commands::rename_subdrawer,
            level_commands::reorder_drawer_level,
            level_commands::begin_native_item_drag,
            level_commands::restore_level_item,
            level_commands::move_item_between_levels,
            dock_commands::get_dock_state,
            dock_commands::toggle_dock,
            dock_commands::set_dock_visible,
            dock_commands::update_dock_settings,
            dock_commands::set_dock_icon_size,
            dock_commands::add_dock_items,
            dock_commands::add_dock_separator,
            dock_commands::remove_dock_item,
            dock_commands::rename_dock_item,
            dock_commands::repair_dock_item,
            dock_commands::reorder_dock_items,
            dock_commands::open_dock_item,
            dock_commands::open_dock_item_location,
            dock_commands::get_dock_item_icon,
            dock_commands::refresh_dock_availability,
            dock_commands::relayout_dock,
        ])
        .on_window_event(|window, event| {
            let is_dock_window = window.label() == dock_service::DOCK_WINDOW
                || window.label() == dock_service::DOCK_HANDLE_WINDOW;
            if is_dock_window {
                match event {
                    // Alt+F4 sobre el Dock no debe dejar al usuario sin tirador.
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        if window.label() == dock_service::DOCK_WINDOW {
                            let _ = dock_commands::set_visibility(window.app_handle(), false);
                        }
                    }
                    // Cambió el DPI del monitor: recolocamos para que el tirador
                    // nunca quede fuera de la pantalla.
                    WindowEvent::ScaleFactorChanged { .. } => {
                        let app = window.app_handle();
                        if let Ok(mut dock) = app.state::<AppState>().snapshot().map(|s| s.dock)
                            && let Err(error) = dock_service::relayout(app, &mut dock)
                        {
                            eprintln!("No se pudo recolocar el Dock: {error}");
                        }
                    }
                    _ => {}
                }
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(id) = window_service::drawer_id_from_label(window.label()) {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("No se pudo ocultar la ventana del cajón: {error}");
                    }
                    commands::mark_drawer_hidden(window.app_handle(), id);
                } else if window.label() == "admin" {
                    api.prevent_close();
                    commands::capture_open_drawer_geometry(window.app_handle());
                    if let Err(error) = window_service::hide_admin_window(window.app_handle()) {
                        eprintln!("No se pudo ocultar el administrador: {error}");
                    }
                }
            } else if let WindowEvent::Resized(_) = event
                && window.label() == "admin"
                && window.is_minimized().unwrap_or(false)
            {
                let should_hide = window
                    .app_handle()
                    .state::<AppState>()
                    .snapshot()
                    .map(|state| state.preferences.hide_admin_on_minimize)
                    .unwrap_or(true);
                if should_hide
                    && let Err(error) = window_service::hide_admin_window(window.app_handle())
                {
                    eprintln!("No se pudo ocultar el administrador al minimizar: {error}");
                }
            }
        })
        .run(tauri::generate_context!());

    if let Err(error) = application {
        eprintln!("Desktop Organizer finalizó con un error: {error}");
    }
}
