#[cfg(target_os = "linux")]
mod ibus_compat;

fn main() {
    #[cfg(target_os = "linux")]
    ibus_compat::initialize();
    std::process::exit(pitex_shell::run(env!("CARGO_PKG_VERSION")));
}
