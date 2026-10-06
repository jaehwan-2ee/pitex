// GUI subsystem in release builds — no console window when launched from
// Explorer or the Start Menu. Debug builds keep the console for stderr logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    configure_runtime_environment();
    #[cfg(windows)]
    if let Some(icon) = std::env::var_os("PITEX_RUNTIME_SMOKE_ICON") {
        match decode_runtime_icon(std::path::Path::new(&icon)) {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("Bundled icon decode failed: {error}");
                std::process::exit(2);
            }
        }
    }
    std::process::exit(pitex_shell::run(pitex_shell::identity::app_version(
        env!("CARGO_PKG_VERSION"),
    )));
}

/// The bundled layout keeps DLLs in `bin/` beside the exe; GLib/GdkPixbuf
/// resolve `../share` and `../lib` relative to their own DLL. Explicitly
/// select the bundled schemas and loader cache so an inherited MSYS2 setting cannot hide
/// the application's portable runtime. Fontconfig also needs a setting:
/// MSYS2's stock
/// `fonts.conf` names `/ucrt64/...` paths that do not exist off-MSYS2, so the
/// bundle ships a minimal `etc/fonts/fonts.conf` and it is pointed at
/// explicitly before GTK initializes.
#[cfg(windows)]
fn configure_runtime_environment() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(prefix) = exe.parent().and_then(|bin| bin.parent()) else {
        return;
    };
    for (variable, path) in
        bundled_runtime_environment(prefix, |name| std::env::var_os(name).is_some())
    {
        std::env::set_var(variable, path);
    }
}

#[cfg(not(windows))]
fn configure_runtime_environment() {}

#[cfg(any(windows, test))]
fn bundled_runtime_environment(
    prefix: &std::path::Path,
    variable_is_set: impl Fn(&str) -> bool,
) -> Vec<(&'static str, std::path::PathBuf)> {
    let mut settings = Vec::new();
    let fonts_conf = prefix.join("etc/fonts/fonts.conf");
    if fonts_conf.is_file() && !variable_is_set("FONTCONFIG_FILE") {
        settings.push(("FONTCONFIG_FILE", fonts_conf));
    }
    let schemas = prefix.join("share/glib-2.0/schemas");
    if schemas.join("gschemas.compiled").is_file() {
        settings.push(("GSETTINGS_SCHEMA_DIR", schemas));
    }
    let pixbuf_cache = prefix.join("lib/gdk-pixbuf-2.0/2.10.0/loaders.cache");
    if pixbuf_cache.is_file() {
        settings.push(("GDK_PIXBUF_MODULE_FILE", pixbuf_cache));
    }
    settings
}

/// Release acceptance decodes a real packaged SVG with the same DLLs and
/// environment as ordinary startup. The optional hook exits before GTK
/// opens a window, so loader errors are visible in the runner's stderr.
#[cfg(windows)]
fn decode_runtime_icon(path: &std::path::Path) -> Result<(), String> {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    #[repr(C)]
    struct PixbufError {
        domain: u32,
        code: i32,
        message: *mut c_char,
    }
    #[link(name = "gdk_pixbuf-2.0")]
    extern "C" {
        fn gdk_pixbuf_new_from_file(
            filename: *const c_char,
            error: *mut *mut PixbufError,
        ) -> *mut std::ffi::c_void;
    }
    #[link(name = "gobject-2.0")]
    extern "C" {
        fn g_object_unref(object: *mut std::ffi::c_void);
    }
    #[link(name = "glib-2.0")]
    extern "C" {
        fn g_error_free(error: *mut PixbufError);
    }

    let filename = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    let mut error = std::ptr::null_mut();
    // GTK/GdkPixbuf filename APIs use UTF-8 on Windows, unlike Win32 APIs.
    let image = unsafe { gdk_pixbuf_new_from_file(filename.as_ptr(), &mut error) };
    let message = if error.is_null() {
        None
    } else {
        let message = unsafe { CStr::from_ptr((*error).message) }
            .to_string_lossy()
            .into_owned();
        unsafe { g_error_free(error) };
        Some(message)
    };
    if image.is_null() {
        Err(message.unwrap_or_else(|| format!("Could not decode {}", path.display())))
    } else {
        unsafe { g_object_unref(image) };
        Ok(())
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::bundled_runtime_environment;
    use std::path::PathBuf;

    struct RuntimeDirectory(PathBuf);
    impl RuntimeDirectory {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("pitex-runtime-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(path.join("etc/fonts")).unwrap();
            std::fs::create_dir_all(path.join("share/glib-2.0/schemas")).unwrap();
            std::fs::create_dir_all(path.join("lib/gdk-pixbuf-2.0/2.10.0")).unwrap();
            Self(path)
        }
    }
    impl Drop for RuntimeDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn portable_runtime_requires_resources_and_preserves_font_override() {
        let runtime = RuntimeDirectory::new();
        assert!(bundled_runtime_environment(&runtime.0, |_| false).is_empty());
        let font = runtime.0.join("etc/fonts/fonts.conf");
        let schemas = runtime.0.join("share/glib-2.0/schemas");
        let pixbuf_cache = runtime.0.join("lib/gdk-pixbuf-2.0/2.10.0/loaders.cache");
        std::fs::write(&font, "<fontconfig/>").unwrap();
        std::fs::write(schemas.join("gschemas.compiled"), b"compiled").unwrap();
        std::fs::write(&pixbuf_cache, b"loaders").unwrap();
        assert_eq!(
            bundled_runtime_environment(&runtime.0, |_| false),
            vec![
                ("FONTCONFIG_FILE", font),
                ("GSETTINGS_SCHEMA_DIR", schemas.clone()),
                ("GDK_PIXBUF_MODULE_FILE", pixbuf_cache.clone()),
            ]
        );
        assert_eq!(
            bundled_runtime_environment(&runtime.0, |_| true),
            vec![
                ("GSETTINGS_SCHEMA_DIR", schemas),
                ("GDK_PIXBUF_MODULE_FILE", pixbuf_cache)
            ]
        );
    }
}
