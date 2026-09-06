use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

#[cfg(windows)]
const APP_NAME: &str = "Desktop Organizer";
#[cfg(windows)]
const RUN_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(windows)]
const STARTUP_APPROVED_KEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
#[cfg(windows)]
const STARTUP_APPROVED_ENABLED: [u8; 12] = [
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[cfg(windows)]
fn expected_command() -> Result<String, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Windows no pudo localizar el ejecutable actual: {error}"))?;
    Ok(format!("\"{}\" --autostart", executable.display()))
}

pub fn enable(_app: &AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::{
            enums::{RegType::REG_BINARY, HKEY_CURRENT_USER, KEY_SET_VALUE},
            RegKey, RegValue,
        };

        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        let command = expected_command()?;
        current_user
            .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .and_then(|key| key.set_value(APP_NAME, &command))
            .map_err(|error| format!("Windows no pudo habilitar el inicio automático: {error}"))?;

        if let Ok(key) = current_user.open_subkey_with_flags(STARTUP_APPROVED_KEY, KEY_SET_VALUE) {
            key.set_raw_value(
                APP_NAME,
                &RegValue {
                    vtype: REG_BINARY,
                    bytes: STARTUP_APPROVED_ENABLED.to_vec(),
                },
            )
            .map_err(|error| {
                format!(
                    "Windows no pudo habilitar Desktop Organizer en Aplicaciones de inicio: {error}"
                )
            })?;
        }
        Ok(())
    }

    #[cfg(not(windows))]
    _app.autolaunch()
        .enable()
        .map_err(|error| format!("No se pudo habilitar el inicio automático: {error}"))
}

pub fn disable(app: &AppHandle) -> Result<(), String> {
    app.autolaunch()
        .disable()
        .map_err(|error| format!("Windows no pudo deshabilitar el inicio automático: {error}"))
}

pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    let enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| format!("Windows no pudo consultar el inicio automático: {error}"))?;

    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        if !enabled {
            return Ok(false);
        }
        let registered: String = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN_KEY)
            .and_then(|key| key.get_value(APP_NAME))
            .map_err(|error| format!("Windows no pudo leer el inicio automático: {error}"))?;
        Ok(registered == expected_command()?)
    }

    #[cfg(not(windows))]
    Ok(enabled)
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_command_quotes_paths_with_spaces() {
        let command = format!(
            "\"{}\" --autostart",
            r"C:\Program Files\Desktop Organizer\desktop-organizer.exe"
        );
        assert_eq!(
            command,
            r#""C:\Program Files\Desktop Organizer\desktop-organizer.exe" --autostart"#
        );
    }
}
