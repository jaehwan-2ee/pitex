#[cfg(target_os = "linux")]
mod ibus_compat;

fn main() {
    #[cfg(target_os = "linux")]
    ibus_compat::initialize();
    std::process::exit(pitex_shell::run(pitex_shell::identity::app_version(
        env!("CARGO_PKG_VERSION"),
    )));
}
