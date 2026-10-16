// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The app re-launches itself elevated to write the system environment variables.
    #[cfg(windows)]
    if std::env::args().any(|a| a == jman_lib::ELEVATED_SETUP_ARG) {
        let code = match jman_core::setup::windows::apply_system() {
            Ok(()) => 0,
            Err(_) => 1,
        };
        std::process::exit(code);
    }

    jman_lib::run()
}
