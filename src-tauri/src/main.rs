// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The app re-launches itself elevated to write the system environment variables
    // and to delete JDKs in folders the user can't write.
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().collect();
        let result = if args.iter().any(|a| a == jman_lib::ELEVATED_SETUP_ARG) {
            Some(jman_core::setup::windows::apply_system())
        } else {
            args.iter().position(|a| a == jman_lib::ELEVATED_REMOVE_ARG).map(|i| match args.get(i + 1) {
                Some(path) => jman_core::remove::delete_elevated(std::path::Path::new(path)),
                None => Err(anyhow::anyhow!("missing path")),
            })
        };
        if let Some(result) = result {
            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
    }

    jman_lib::run()
}
