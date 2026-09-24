use std::{
    collections::HashSet,
    ffi::c_void,
    fs,
    io::BufWriter,
    mem::size_of,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use tauri::{AppHandle, Manager};

use crate::storage_service::atomic_replace;
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
            SIIGBF_ICONONLY, SIIGBF_THUMBNAILONLY,
        },
    },
    core::PCWSTR,
};

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

const CACHE_VERSION: &str = "v1";
const ICON_EDGE: i32 = 64;
const MAX_ICON_CACHE_FILES: usize = 2_048;
const MAX_ICON_CACHE_AGE: Duration = Duration::from_secs(90 * 24 * 60 * 60);

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
        let preview = is_image_file(path);
        let cache_key = if preview {
            format!("{icon_key}-preview")
        } else {
            icon_key.to_owned()
        };
        let destination = Self::cache_path(app, &cache_key)?;
        if destination.is_file() && (!preview || !image_is_newer(path, &destination)) {
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
                if preview && destination.exists() {
                    if let Err(error) = atomic_replace(&temporary, &destination) {
                        let _ = fs::remove_file(&temporary);
                        return Err(error);
                    }
                } else if destination.exists() {
                    let _ = fs::remove_file(&temporary);
                } else if let Err(error) = fs::rename(&temporary, &destination) {
                    if destination.exists() {
                        let _ = fs::remove_file(&temporary);
                    } else {
                        return Err(format!("No se pudo guardar el icono en caché: {error}"));
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

    pub fn prune(app: &AppHandle, retained_keys: &HashSet<String>) -> Result<(), String> {
        let directory = Self::cache_directory(app)?;
        prune_cache_directory(&directory, retained_keys, MAX_ICON_CACHE_FILES)
    }

    fn cache_path(app: &AppHandle, icon_key: &str) -> Result<PathBuf, String> {
        if icon_key.is_empty()
            || !icon_key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err("La clave del icono no es válida".to_owned());
        }
        Self::cache_directory(app).map(|directory| directory.join(format!("{icon_key}.png")))
    }

    fn cache_directory(app: &AppHandle) -> Result<PathBuf, String> {
        app.path()
            .app_data_dir()
            .map(|directory| directory.join("cache").join("icons"))
            .map_err(|error| format!("No se pudo resolver la caché de iconos: {error}"))
    }
}

fn prune_cache_directory(
    directory: &Path,
    retained_keys: &HashSet<String>,
    max_files: usize,
) -> Result<(), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "No se pudo revisar la caché {}: {error}",
                directory.display()
            ));
        }
    };
    let mut cached = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("tmp") {
            let _ = fs::remove_file(path);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("png") {
            continue;
        }
        let key = path
            .file_stem()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        cached.push((path, key, modified));
    }

    cached.sort_by_key(|(_, _, modified)| *modified);
    let mut remaining = cached.len();
    for (path, key, modified) in cached {
        if retained_keys.contains(&key)
            || key
                .strip_suffix("-preview")
                .is_some_and(|base| retained_keys.contains(base))
        {
            continue;
        }
        let expired = SystemTime::now()
            .duration_since(modified)
            .is_ok_and(|age| age > MAX_ICON_CACHE_AGE);
        if (remaining > max_files || expired) && fs::remove_file(&path).is_ok() {
            remaining = remaining.saturating_sub(1);
        }
    }
    Ok(())
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "bmp"
                    | "webp"
                    | "tif"
                    | "tiff"
                    | "heic"
                    | "heif"
                    | "avif"
            )
        })
}

fn image_is_newer(source: &Path, cached: &Path) -> bool {
    match (fs::metadata(source), fs::metadata(cached)) {
        (Ok(source), Ok(cached)) => match (source.modified(), cached.modified()) {
            (Ok(source), Ok(cached)) => source > cached,
            _ => true,
        },
        _ => true,
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
        let size = SIZE {
            cx: ICON_EDGE,
            cy: ICON_EDGE,
        };
        let thumbnail = is_image_file(path);
        let bitmap = if thumbnail {
            unsafe { factory.GetImage(size, SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK) }.or_else(
                |_| unsafe { factory.GetImage(size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK) },
            )
        } else {
            unsafe { factory.GetImage(size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK) }
        }
        .map_err(|error| format!("Windows Shell no entregó una vista previa: {error}"))?;

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
    use std::{collections::HashSet, fs};

    use super::{extract_windows_icon, is_image_file, prune_cache_directory};
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

    #[test]
    fn image_files_receive_a_real_thumbnail() {
        let source =
            std::env::temp_dir().join(format!("desktop-organizer-image-{}.png", Uuid::new_v4()));
        let destination = source.with_extension("preview.png");
        let pixels = vec![240_u8, 20, 170, 255].repeat(256 * 256);
        {
            let file = fs::File::create(&source).expect("source image should be writable");
            let mut encoder = png::Encoder::new(file, 256, 256);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("PNG header");
            writer.write_image_data(&pixels).expect("PNG pixels");
        }
        assert!(is_image_file(&source));
        extract_windows_icon(&source, &destination)
            .expect("Windows should provide the image thumbnail");
        let mut decoder = png::Decoder::new(fs::File::open(&destination).expect("preview"));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().expect("preview header");
        let mut output = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut output).expect("preview pixels");
        let center = ((info.height / 2 * info.width + info.width / 2) * 4) as usize;
        assert!(
            output[center] > 180 && output[center + 1] < 100 && output[center + 2] > 110,
            "the center should show the magenta source image"
        );
        fs::remove_file(source).expect("remove source");
        fs::remove_file(destination).expect("remove preview");
    }

    #[test]
    fn cache_pruning_keeps_referenced_icons_and_limits_obsolete_entries() {
        let root = std::env::temp_dir().join(format!("desktop-organizer-icons-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("cache fixture should exist");
        for index in 0..5 {
            fs::write(root.join(format!("v1-{index}.png")), b"png")
                .expect("cache fixture should be writable");
        }
        fs::write(root.join("unfinished.tmp"), b"temporary")
            .expect("temporary fixture should be writable");
        let retained = HashSet::from(["v1-0".to_owned()]);

        prune_cache_directory(&root, &retained, 2).expect("cache pruning should work");

        assert!(root.join("v1-0.png").is_file());
        assert!(!root.join("unfinished.tmp").exists());
        assert!(fs::read_dir(&root).expect("cache should read").count() <= 2);
        fs::remove_dir_all(root).expect("cache fixture should be removed");
    }
}
