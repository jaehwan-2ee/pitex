#[cfg(target_os = "linux")]
extern "C" {
    fn pitex_ibus_initialize();
}

fn main() {
    #[cfg(target_os = "linux")]
    unsafe { pitex_ibus_initialize(); }
    std::process::exit(pitex_shell::run(env!("CARGO_PKG_VERSION")));
}
