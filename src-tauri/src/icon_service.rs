use std::{
    ffi::c_void,
    fs,
    io::BufWriter,
    mem::size_of,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use tauri::{AppHandle, Manager};
use windows::{
    Win32::{
        Foundation::{RPC_E_CHANGED_MODE, SIZE},
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS,
            DeleteDC, DeleteObject, GetDIBits, GetObjectW, HGDIOBJ,
        },
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize, IBindCtx},
        UI::Shell::{
            IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
            SIIGBF_ICONONLY,
        },
    },
    core::PCWSTR,
};

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

const CACHE_VERSION: &str = "v1";
const ICON_EDGE: i32 = 64;

pub struct IconService;

impl IconService {
    pub fn key_for(path: &Path) -> String {
        let normalized = path.to_string_lossy().replace('/', "\\").to_lowercase();
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in normalized.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{CACHE_VERSION}-{hash:016x}")
    }

    pub fn ensure(app: &AppHandle, path: &Path, icon_key: &str) -> Result<Option<PathBuf>, String> {
        let destination = Self::cache_path(app, icon_key)?;
        if destination.is_file() {
            return Ok(Some(destination));
        }

        let directory = destination
            .parent()
            .ok_or_else(|| "La caché de iconos no tiene un directorio válido".to_owned())?;
        fs::create_dir_all(directory)
            .map_err(|error| format!("No se pudo crear la caché de iconos: {error}"))?;
        let temporary = destination.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));

        match extract_windows_icon(path, &temporary) {
            Ok(()) => {
                if destination.exists() {
                    let _ = fs::remove_file(&temporary);
                } else {
                    if let Err(error) = fs::rename(&temporary, &destination) {
                        if destination.exists() {
                            let _ = fs::remove_file(&temporary);
                        } else {
                            return Err(format!("No se pudo guardar el icono en caché: {error}"));
                        }
                    }
                }
                Ok(Some(destination))
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                eprintln!("No se pudo extraer el icono de {}: {error}", path.display());
                Ok(None)
            }
        }
    }

    pub fn data_url(
        app: &AppHandle,
        path: &Path,
        icon_key: &str,
    ) -> Result<Option<String>, String> {
        let Some(cache_path) = Self::ensure(app, path, icon_key)? else {
            return Ok(None);
        };
        let bytes = fs::read(&cache_path)
            .map_err(|error| format!("No se pudo leer el icono en caché: {error}"))?;
        Ok(Some(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(bytes)
        )))
    }

    fn cache_path(app: &AppHandle, icon_key: &str) -> Result<PathBuf, String> {
        if icon_key.is_empty()
            || !icon_key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err("La clave del icono no es válida".to_owned());
        }
        app.path()
            .app_data_dir()
            .map(|directory| {
                directory
                    .join("cache")
                    .join("icons")
                    .join(format!("{icon_key}.png"))
            })
            .map_err(|error| format!("No se pudo resolver la caché de iconos: {error}"))
    }
}

fn extract_windows_icon(path: &Path, destination: &Path) -> Result<(), String> {
    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let initialization = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let should_uninitialize = initialization.is_ok();
    if initialization.is_err() && initialization != RPC_E_CHANGED_MODE {
        return Err(format!(
            "No se pudo iniciar Windows Shell: {initialization:?}"
        ));
    }

    let result = (|| -> Result<(), String> {
        let factory: IShellItemImageFactory =
            unsafe { SHCreateItemFromParsingName(PCWSTR(wide_path.as_ptr()), None::<&IBindCtx>) }
                .map_err(|error| format!("Windows Shell no encontró el elemento: {error}"))?;
        let bitmap = unsafe {
            factory.GetImage(
                SIZE {
                    cx: ICON_EDGE,
                    cy: ICON_EDGE,
                },
                SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
            )
        }
        .map_err(|error| format!("Windows Shell no entregó un icono: {error}"))?;

        let mut bitmap_info = BITMAP::default();
        let object_size = i32::try_from(size_of::<BITMAP>())
            .map_err(|_| "El tamaño del bitmap no es válido".to_owned())?;
        let object_result = unsafe {
            GetObjectW(
                HGDIOBJ(bitmap.0),
                object_size,
                Some((&raw mut bitmap_info).cast::<c_void>()),
            )
        };
        if object_result == 0 {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
            }
            return Err("Windows no pudo describir el icono".to_owned());
        }

        let width = bitmap_info.bmWidth.unsigned_abs();
        let height = bitmap_info.bmHeight.unsigned_abs();
        let pixel_count = usize::try_from(width)
            .ok()
            .and_then(|value| value.checked_mul(usize::try_from(height).ok()?))
            .ok_or_else(|| "El tamaño del icono es inválido".to_owned())?;
        let mut pixels = vec![0_u8; pixel_count.saturating_mul(4)];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).unwrap_or_default(),
                biWidth: i32::try_from(width).unwrap_or(ICON_EDGE),
                biHeight: -i32::try_from(height).unwrap_or(ICON_EDGE),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let device_context = unsafe { CreateCompatibleDC(None) };
        if device_context.0.is_null() {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
            }
            return Err("Windows no pudo crear el contexto gráfico del icono".to_owned());
        }
        let scan_lines = unsafe {
            GetDIBits(
                device_context,
                bitmap,
                0,
                height,
                Some(pixels.as_mut_ptr().cast::<c_void>()),
                &raw mut info,
                DIB_RGB_COLORS,
            )
        };
        unsafe {
            let _ = DeleteDC(device_context);
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
        }
        if scan_lines == 0 {
            return Err("Windows no pudo leer los píxeles del icono".to_owned());
        }

        let has_alpha = pixels.chunks_exact(4).any(|pixel| pixel[3] != 0);
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            if !has_alpha {
                pixel[3] = 255;
            } else if pixel[3] > 0 && pixel[3] < 255 {
                let alpha = u16::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = ((u16::from(*channel) * 255 / alpha).min(255)) as u8;
                }
            }
        }

        let file = fs::File::create(destination)
            .map_err(|error| format!("No se pudo crear el archivo de caché: {error}"))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("No se pudo preparar el PNG: {error}"))?;
        writer
            .write_image_data(&pixels)
            .map_err(|error| format!("No se pudo escribir el PNG: {error}"))
    })();

    if should_uninitialize {
        unsafe { CoUninitialize() };
    }
    result
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::extract_windows_icon;
    use uuid::Uuid;

    #[test]
    fn extracts_a_real_windows_shell_icon_as_png() {
        let executable = std::env::current_exe().expect("test executable should have a path");
        let destination = std::env::temp_dir().join(format!(
            "desktop-organizer-shell-icon-{}.png",
            Uuid::new_v4()
        ));

        extract_windows_icon(&executable, &destination)
            .expect("Windows Shell should provide the executable icon");
        let bytes = fs::read(&destination).expect("cached icon should be readable");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");

        fs::remove_file(destination).expect("test icon should be removed");
    }
}
