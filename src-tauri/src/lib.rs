mod commands;
mod icon_service;
mod item_repository;
mod level_commands;
mod model;
mod monitor_service;
mod persistence;
mod shell_service;
mod state;
mod storage_service;
mod tray_service;
mod window_service;

use std::time::{SystemTime, UNIX_EPOCH};

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
            let mut persisted = if should_offer_disk_recovery {
                model::PersistedState::default()
            } else {
                PersistenceService::load(&app_handle).map_err(std::io::Error::other)?
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
            app.manage(AppState::new(persisted.clone()));

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

            let launched_by_autostart = std::env::args().any(|arg| arg == "--autostart");
            if launched_by_autostart && persisted.preferences.start_silently {
                window_service::hide_admin_window(&app_handle).map_err(std::io::Error::other)?;
            } else {
                window_service::show_admin_window(&app_handle).map_err(std::io::Error::other)?;
            }
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
            level_commands::restore_level_item,
            level_commands::move_item_between_levels,
        ])
        .on_window_event(|window, event| {
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
