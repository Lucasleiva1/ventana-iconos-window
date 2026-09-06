//! Icono del área de notificación con identidad propia y permanente.
//!
//! # Por qué no se usa `TrayIconBuilder` de Tauri
//!
//! Windows no identifica un icono de bandeja por su nombre, sino por el par
//! `ventana invisible (HWND) + número (uID)`. La librería que usa Tauri
//! (`tray-icon`) crea una ventana nueva en cada arranque, así que ese par
//! cambia siempre: Windows no puede saber que el icono nuevo es el mismo de
//! antes y lo dibuja al lado del anterior en vez de reemplazarlo. Cuando la
//! aplicación muere sin despedirse —se cuelga, la matan desde el Administrador
//! de tareas— el dibujo viejo queda ahí y se van acumulando.
//!
//! Windows ofrece exactamente el mecanismo que hace falta: el campo `guidItem`
//! con la bandera `NIF_GUID`, que reemplaza al par ventana+número por un
//! identificador fijo. `tray-icon` no lo implementa, así que el icono se
//! registra acá directamente contra la API de Windows.
//!
//! Con un GUID fijo es **imposible** que existan dos iconos: comparten
//! identificador, así que el segundo reemplaza al primero. Además, al arrancar
//! se ejecuta `NIM_DELETE` con ese GUID antes de registrar nada, lo que barre
//! cualquier resto de una ejecución anterior aunque haya terminado de golpe.
//!
//! El menú se arma con la API nativa (`TrackPopupMenu`) porque va atado al
//! icono. Las acciones son exactamente las mismas de antes: sólo cambió quién
//! dibuja el menú, no qué hace cada entrada.

use std::{
    os::windows::ffi::OsStrExt,
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use tauri::{AppHandle, Emitter};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{
                ExtractIconExW, NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
                NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
                GetCursorPos, HICON, MF_SEPARATOR, MF_STRING, PostMessageW, RegisterClassW,
                RegisterWindowMessageW, SetForegroundWindow, TPM_LEFTALIGN, TPM_RETURNCMD,
                TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_DESTROY, WM_ENDSESSION, WM_NULL,
                WM_QUERYENDSESSION, WNDCLASSW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
            },
        },
    },
    core::{GUID, PCWSTR, w},
};

use crate::{
    commands, dock_commands, panel_commands, shell_service::ShellService,
    storage_service::StorageService, window_service,
};

/// Identidad permanente del icono. Generado una única vez: no debe cambiar
/// nunca más, porque es justamente lo que impide que se dupliquen.
const TRAY_GUID: GUID = GUID::from_u128(0x7b3d_1a64_9e52_4c8f_a6d1_2f0b_5e7c_4a93);

/// Mensaje privado que Windows nos manda cuando alguien toca el icono.
const CALLBACK_MESSAGE: u32 = WM_APP + 1;
/// `uID` de respaldo, sólo se usa si el registro por GUID no fue posible.
const FALLBACK_UID: u32 = 1;

/// `NIN_SELECT` y su variante por teclado: el usuario eligió el icono.
const NIN_SELECT_EVENT: u32 = 0x0400;
const NIN_KEYSELECT_EVENT: u32 = 0x0401;
/// `WM_CONTEXTMENU`: clic derecho sobre el icono.
const CONTEXT_MENU_EVENT: u32 = 0x007B;

const ACTION_NEW_DRAWER: u32 = 1;
const ACTION_OPEN_ADMIN: u32 = 2;
const ACTION_SHOW_ALL: u32 = 3;
const ACTION_HIDE_ALL: u32 = 4;
const ACTION_OPEN_DRAWERS: u32 = 5;
const ACTION_SHOW_DOCK: u32 = 6;
const ACTION_HIDE_DOCK: u32 = 7;
const ACTION_NEW_PANEL: u32 = 8;
const ACTION_SHOW_PANELS: u32 = 9;
const ACTION_HIDE_PANELS: u32 = 10;
const ACTION_SETTINGS: u32 = 11;
const ACTION_QUIT: u32 = 12;

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOST_WINDOW_LOW: AtomicU32 = AtomicU32::new(0);
static HOST_WINDOW_HIGH: AtomicU32 = AtomicU32::new(0);
static TASKBAR_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// `false` cuando hubo que caer al modo ventana+número (ver `register_icon`).
static USING_GUID: AtomicBool = AtomicBool::new(true);

fn store_host(window: HWND) {
    let raw = window.0 as usize as u64;
    HOST_WINDOW_LOW.store((raw & 0xFFFF_FFFF) as u32, Ordering::SeqCst);
    HOST_WINDOW_HIGH.store((raw >> 32) as u32, Ordering::SeqCst);
}

fn host_window() -> HWND {
    let low = u64::from(HOST_WINDOW_LOW.load(Ordering::SeqCst));
    let high = u64::from(HOST_WINDOW_HIGH.load(Ordering::SeqCst));
    HWND(((high << 32) | low) as usize as *mut core::ffi::c_void)
}

pub fn setup(app: &AppHandle) -> Result<(), String> {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let _ = APP.set(app.clone());

    let instance = unsafe { GetModuleHandleW(None) }
        .map_err(|error| format!("No se pudo identificar el módulo actual: {error}"))?;
    let class_name = w!("DesktopOrganizerTrayHost");
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    // Registrar dos veces devuelve 0. No se aborta por eso: `setup` corre una
    // sola vez y que la clase ya exista no impide crear la ventana.
    unsafe { RegisterClassW(&window_class) };

    // Ventana propia, nunca visible. No puede ser "message-only" (`HWND_MESSAGE`)
    // porque esas no reciben mensajes de difusión, y necesitamos escuchar
    // `TaskbarCreated` para volver a dibujar el icono si se reinicia el
    // Explorador. `WS_EX_TOOLWINDOW` la mantiene fuera de la barra de tareas.
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name,
            w!("Desktop Organizer"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|error| format!("No se pudo crear la ventana del área de notificación: {error}"))?;
    store_host(window);

    TASKBAR_CREATED_MESSAGE.store(
        unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        Ordering::SeqCst,
    );

    if !register_icon(window) {
        return Err("Windows rechazó el icono del área de notificación".to_owned());
    }
    Ok(())
}

/// Carga el icono real del ejecutable en el tamaño chico que usa la bandeja.
fn load_application_icon() -> HICON {
    let Ok(executable) = std::env::current_exe() else {
        return HICON::default();
    };
    let path: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut small = HICON::default();
    unsafe {
        ExtractIconExW(PCWSTR(path.as_ptr()), 0, None, Some(&mut small), 1);
    }
    small
}

fn base_icon_data(window: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: u32::try_from(size_of::<NOTIFYICONDATAW>()).unwrap_or_default(),
        hWnd: window,
        uID: FALLBACK_UID,
        uCallbackMessage: CALLBACK_MESSAGE,
        ..Default::default()
    }
}

fn write_tooltip(data: &mut NOTIFYICONDATAW) {
    for (slot, value) in data
        .szTip
        .iter_mut()
        .zip("Desktop Organizer".encode_utf16().chain(std::iter::once(0)))
    {
        *slot = value;
    }
}

/// Registra el icono, barriendo primero cualquier resto de una ejecución previa.
///
/// Si Windows rechaza el GUID —ocurre cuando ese GUID quedó asociado a un
/// ejecutable en otra ruta, típico al alternar entre una compilación de
/// desarrollo y la instalada— se cae al modo clásico ventana+número para no
/// quedarse sin icono. En ese modo vuelve a ser posible un fantasma, y por eso
/// el modo preferido siempre es el GUID.
fn register_icon(window: HWND) -> bool {
    let icon = load_application_icon();

    // Barrido previo: borra el icono de la ejecución anterior aunque ese
    // proceso ya no exista. Es lo que evita la acumulación de fantasmas.
    let mut ghost = base_icon_data(window);
    ghost.uFlags = NIF_GUID;
    ghost.guidItem = TRAY_GUID;
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &ghost) };

    let mut data = base_icon_data(window);
    data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_GUID;
    data.guidItem = TRAY_GUID;
    data.hIcon = icon;
    write_tooltip(&mut data);

    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        USING_GUID.store(false, Ordering::SeqCst);
        let mut fallback = base_icon_data(window);
        fallback.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        fallback.hIcon = icon;
        write_tooltip(&mut fallback);
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &fallback) };
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &fallback) }.as_bool() {
            return false;
        }
        data = fallback;
    }

    // Versión 4: Windows manda las coordenadas del clic y distingue el menú
    // contextual del clic izquierdo sin que haya que adivinarlo.
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let _ = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) };
    true
}

/// Retira el icono. Se llama al salir por el menú y cuando Windows cierra
/// sesión; el barrido de `register_icon` cubre el resto de los casos.
pub fn remove_icon() {
    if !INSTALLED.load(Ordering::SeqCst) {
        return;
    }
    let mut data = base_icon_data(host_window());
    if USING_GUID.load(Ordering::SeqCst) {
        data.uFlags = NIF_GUID;
        data.guidItem = TRAY_GUID;
    }
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_created = TASKBAR_CREATED_MESSAGE.load(Ordering::SeqCst);
    if taskbar_created != 0 && message == taskbar_created {
        // El Explorador se reinició y perdió todos los iconos: hay que
        // registrarlo de nuevo o la aplicación queda sin acceso desde la barra.
        register_icon(window);
        return LRESULT(0);
    }

    match message {
        CALLBACK_MESSAGE => {
            // Con la versión 4 el evento viaja en la parte baja de `lparam` y
            // las coordenadas de pantalla del clic en `wparam`.
            let event = (lparam.0 as u32) & 0xFFFF;
            let x = i32::from((wparam.0 as u32 & 0xFFFF) as u16);
            let y = i32::from(((wparam.0 as u32 >> 16) & 0xFFFF) as u16);
            match event {
                NIN_SELECT_EVENT | NIN_KEYSELECT_EVENT => {
                    if let Some(app) = APP.get()
                        && let Err(error) = window_service::show_admin_window(app)
                    {
                        eprintln!(
                            "No se pudo abrir el administrador desde el área de notificación: {error}"
                        );
                    }
                }
                CONTEXT_MENU_EVENT => show_tray_menu(window, x, y),
                _ => {}
            }
            LRESULT(0)
        }
        WM_QUERYENDSESSION => {
            remove_icon();
            LRESULT(1)
        }
        WM_ENDSESSION | WM_DESTROY => {
            remove_icon();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn show_tray_menu(window: HWND, x: i32, y: i32) {
    let Some(app) = APP.get() else {
        return;
    };
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };

    let entries: [(u32, PCWSTR); 12] = [
        (ACTION_NEW_DRAWER, w!("Nuevo cajón")),
        (ACTION_OPEN_ADMIN, w!("Abrir administrador")),
        (ACTION_SHOW_ALL, w!("Mostrar todos los cajones")),
        (ACTION_HIDE_ALL, w!("Ocultar todos los cajones")),
        (ACTION_OPEN_DRAWERS, w!("Abrir carpeta Cajones")),
        (ACTION_SHOW_DOCK, w!("Mostrar Dock")),
        (ACTION_HIDE_DOCK, w!("Ocultar Dock")),
        (ACTION_NEW_PANEL, w!("Nuevo panel")),
        (ACTION_SHOW_PANELS, w!("Mostrar todos los paneles")),
        (ACTION_HIDE_PANELS, w!("Ocultar todos los paneles")),
        (ACTION_SETTINGS, w!("Configuración")),
        (ACTION_QUIT, w!("Salir")),
    ];
    // Separadores en las mismas posiciones que tenía el menú anterior.
    let separator_before = [
        ACTION_SHOW_ALL,
        ACTION_SHOW_DOCK,
        ACTION_NEW_PANEL,
        ACTION_QUIT,
    ];

    for (action, label) in entries {
        if separator_before.contains(&action) {
            let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };
        }
        let _ = unsafe { AppendMenuW(menu, MF_STRING, action as usize, label) };
    }

    // Sin esto el menú se queda abierto cuando el usuario hace clic afuera.
    let _ = unsafe { SetForegroundWindow(window) };

    let (x, y) = if x == 0 && y == 0 {
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_ok() {
            (point.x, point.y)
        } else {
            (0, 0)
        }
    } else {
        (x, y)
    };

    let selected = unsafe {
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            x,
            y,
            None,
            window,
            None,
        )
    };
    let _ = unsafe { DestroyMenu(menu) };
    // Truco documentado por Microsoft para que el menú se cierre del todo.
    let _ = unsafe { PostMessageW(Some(window), WM_NULL, WPARAM(0), LPARAM(0)) };

    let action = u32::try_from(selected.0).unwrap_or_default();
    if action != 0 {
        run_action(app, action);
    }
}

fn run_action(app: &AppHandle, action: u32) {
    let result = match action {
        ACTION_NEW_DRAWER => window_service::show_admin_window(app).and_then(|()| {
            app.emit("admin:focus-new-drawer", ())
                .map_err(|error| error.to_string())
        }),
        ACTION_OPEN_ADMIN => window_service::show_admin_window(app),
        ACTION_SHOW_ALL => commands::set_all_drawers_visibility(app, false).map(|_| ()),
        ACTION_HIDE_ALL => commands::set_all_drawers_visibility(app, true).map(|_| ()),
        ACTION_OPEN_DRAWERS => {
            StorageService::paths().and_then(|paths| ShellService::open(&paths.drawers))
        }
        ACTION_SHOW_DOCK => dock_commands::set_visibility(app, true),
        ACTION_HIDE_DOCK => dock_commands::set_visibility(app, false),
        ACTION_NEW_PANEL => panel_commands::create_panel_from_tray(app),
        ACTION_SHOW_PANELS => panel_commands::set_visibility_from_tray(app, false),
        ACTION_HIDE_PANELS => panel_commands::set_visibility_from_tray(app, true),
        ACTION_SETTINGS => window_service::show_admin_window(app).and_then(|()| {
            app.emit("admin:open-settings", ())
                .map_err(|error| error.to_string())
        }),
        ACTION_QUIT => {
            panel_commands::capture_open_panel_geometry(app);
            commands::capture_open_drawer_geometry(app);
            let _ = crate::log_service::LogService::info("Salida solicitada desde el System Tray");
            // Retirar el icono antes de cerrar: si se espera al final del
            // proceso, Windows alcanza a dejar el fantasma dibujado.
            remove_icon();
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(error) = result {
        eprintln!("Falló una acción del área de notificación: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        ACTION_HIDE_ALL, ACTION_HIDE_DOCK, ACTION_HIDE_PANELS, ACTION_NEW_DRAWER, ACTION_NEW_PANEL,
        ACTION_OPEN_ADMIN, ACTION_OPEN_DRAWERS, ACTION_QUIT, ACTION_SETTINGS, ACTION_SHOW_ALL,
        ACTION_SHOW_DOCK, ACTION_SHOW_PANELS, TRAY_GUID,
    };

    #[test]
    fn every_menu_action_has_a_distinct_command_id() {
        let ids = [
            ACTION_NEW_DRAWER,
            ACTION_OPEN_ADMIN,
            ACTION_SHOW_ALL,
            ACTION_HIDE_ALL,
            ACTION_OPEN_DRAWERS,
            ACTION_SHOW_DOCK,
            ACTION_HIDE_DOCK,
            ACTION_NEW_PANEL,
            ACTION_SHOW_PANELS,
            ACTION_HIDE_PANELS,
            ACTION_SETTINGS,
            ACTION_QUIT,
        ];
        let mut seen = HashSet::new();
        for id in ids {
            // Cero significa "no se eligió nada" en TrackPopupMenu: ninguna
            // acción puede usar ese valor o se dispararía sola.
            assert_ne!(id, 0, "ninguna acción puede valer 0");
            assert!(seen.insert(id), "el identificador {id} está repetido");
        }
        assert_eq!(seen.len(), 12, "el menú conserva sus doce acciones");
    }

    #[test]
    fn the_tray_identity_is_the_agreed_permanent_guid() {
        // Si este valor cambia, las instalaciones existentes dejan de reconocer
        // su propio icono y vuelve a ser posible la duplicación.
        assert_eq!(
            TRAY_GUID.to_u128(),
            0x7b3d_1a64_9e52_4c8f_a6d1_2f0b_5e7c_4a93
        );
    }
}
