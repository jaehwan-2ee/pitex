use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") { return; }
    println!("cargo:rustc-link-arg=-Wl,--export-dynamic-symbol=ibus_input_context_process_key_event");
}
