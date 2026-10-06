//! Preload entry points for the real IBus/GTK keyboard regression harness.
//! Compile the application's adapter directly so the probe cannot diverge.
#![cfg(target_os = "linux")]

#[path = "../../../Linux/crates/pitex/src/ibus_compat.rs"]
mod ibus_compat;

/// The Python audit calls this before GTK initializes its input context.
#[no_mangle]
pub extern "C" fn pitex_ibus_initialize() {
    ibus_compat::initialize();
}
