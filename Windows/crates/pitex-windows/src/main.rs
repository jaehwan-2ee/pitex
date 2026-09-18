// GUI subsystem in release builds — no console window when launched from
// Explorer or the Start Menu. Debug builds keep the console for stderr logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    configure_runtime_environment();
    std::process::exit(pitex_shell::run());
}

/// The bundled layout keeps DLLs in `bin/` beside the exe; GLib/GdkPixbuf
/// resolve `../share` and `../lib` relative to their own DLL, so schemas and
/// pixbuf loaders need no environment. Fontconfig does: MSYS2's stock
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
    let fonts_conf = prefix.join("etc").join("fonts").join("fonts.conf");
    if fonts_conf.is_file() && std::env::var_os("FONTCONFIG_FILE").is_none() {
        std::env::set_var("FONTCONFIG_FILE", fonts_conf);
    }
}

#[cfg(not(windows))]
fn configure_runtime_environment() {}
