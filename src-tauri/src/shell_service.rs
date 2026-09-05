use std::path::Path;

use windows::{
    Win32::{UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL},
    core::{PCWSTR, w},
};

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

pub struct ShellService;

impl ShellService {
    pub fn open(path: &Path) -> Result<(), String> {
        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let instance = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(wide_path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        let result_code = instance.0 as isize;
        if result_code <= 32 {
            Err(format!(
                "Windows no pudo abrir el elemento (código {result_code})"
            ))
        } else {
            Ok(())
        }
    }
}
