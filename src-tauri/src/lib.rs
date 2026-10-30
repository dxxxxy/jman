use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use jman_core::config::Config;
use jman_core::doctor::{self, Report};
use jman_core::download::{self, Distribution, MajorVersion, Package, Progress};
use jman_core::setup::{self, SetupStatus};
use jman_core::remove::{self, Removal};
use jman_core::{Jdk, detect, link, paths};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::path::BaseDirectory;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewWindowBuilder, WindowEvent};

mod tray;

pub const ELEVATED_SETUP_ARG: &str = "--elevated-setup";
pub const ELEVATED_REMOVE_ARG: &str = "--elevated-remove";

type CmdResult<T> = Result<T, String>;

/// Runs blocking jman-core work off the async runtime, turning errors into strings for the UI.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

/// Tells the UI and the tray that the selected JDK or the JDK list changed.
fn notify_changed(app: &AppHandle) {
    tray::refresh(app);
    let _ = app.emit("jdks-changed", ());
}

#[tauri::command]
async fn list_jdks() -> CmdResult<Vec<Jdk>> {
    blocking(|| Ok(detect::detect_all())).await
}

#[tauri::command]
async fn current_jdk() -> CmdResult<Option<Jdk>> {
    blocking(|| Ok(jman_core::current())).await
}

#[tauri::command]
async fn use_jdk(app: AppHandle, id: String) -> CmdResult<Jdk> {
    let jdk = blocking(move || jman_core::use_jdk(&id)).await?;
    notify_changed(&app);
    Ok(jdk)
}

#[tauri::command]
async fn setup_status() -> CmdResult<SetupStatus> {
    blocking(|| Ok(setup::status())).await
}

#[tauri::command]
async fn doctor() -> CmdResult<Report> {
    blocking(|| Ok(doctor::run())).await
}

/// The bundled `jman` CLI: a resource in installed builds, a sibling binary in dev builds.
fn cli_source(app: &AppHandle) -> Option<PathBuf> {
    let name = paths::exe_name("jman");
    let bundled = app.path().resolve(format!("cli/{name}"), BaseDirectory::Resource).ok();
    let sibling = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.join(&name)));
    [bundled, sibling].into_iter().flatten().find(|p| p.is_file())
}

#[tauri::command]
async fn run_setup(app: AppHandle) -> CmdResult<SetupStatus> {
    let cli = cli_source(&app);
    let status = blocking(move || {
        if let Some(cli) = cli {
            setup::install_cli(&cli)?;
        }
        setup::apply_user()?;
        #[cfg(windows)]
        if setup::status().needs_elevation {
            use jman_core::setup::windows;
            if windows::can_write_system() {
                windows::apply_system()?;
            } else {
                let code = windows::run_elevated(&std::env::current_exe()?, ELEVATED_SETUP_ARG)?;
                anyhow::ensure!(code == 0, "elevated setup failed (exit code {code})");
            }
        }
        Ok(setup::status())
    })
    .await?;
    notify_changed(&app);
    Ok(status)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadOptions {
    distributions: &'static [Distribution],
    versions: Vec<MajorVersion>,
}

#[tauri::command]
async fn download_options() -> CmdResult<DownloadOptions> {
    blocking(|| Ok(DownloadOptions { distributions: download::DISTRIBUTIONS, versions: download::major_versions()? }))
        .await
}

#[tauri::command]
async fn find_package(distribution: String, major: u32) -> CmdResult<Option<Package>> {
    blocking(move || download::find_package(&distribution, major)).await
}

/// Downloads in flight. Their progress `Channel` belongs to the webview, so the window is
/// only hidden, not destroyed, while one runs.
static DOWNLOADS: AtomicUsize = AtomicUsize::new(0);

struct DownloadGuard(AppHandle);

impl DownloadGuard {
    fn new(app: &AppHandle) -> Self {
        DOWNLOADS.fetch_add(1, Ordering::SeqCst);
        Self(app.clone())
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        // the window was closed during the download: free the webview now
        if DOWNLOADS.fetch_sub(1, Ordering::SeqCst) == 1
            && let Some(window) = self.0.get_webview_window("main")
            && !window.is_visible().unwrap_or(true)
        {
            let _ = window.destroy();
        }
    }
}

#[tauri::command]
async fn install_jdk(app: AppHandle, distribution: String, major: u32, on_progress: Channel<Progress>) -> CmdResult<Jdk> {
    let _guard = DownloadGuard::new(&app);
    let jdk = blocking(move || {
        let pkg = download::find_package(&distribution, major)?
            .ok_or_else(|| anyhow::anyhow!("no {distribution} {major} build for this platform"))?;
        download::install(&pkg, |p| {
            let _ = on_progress.send(p);
        })
    })
    .await?;
    notify_changed(&app);
    Ok(jdk)
}

#[tauri::command]
async fn removal_plan(id: String) -> CmdResult<Removal> {
    blocking(move || remove::plan(&find_jdk(&id)?)).await
}

#[tauri::command]
async fn remove_jdk(app: AppHandle, id: String) -> CmdResult<()> {
    blocking(move || {
        let jdk = find_jdk(&id)?;
        let removal = remove::plan(&jdk)?;
        remove::execute(&jdk, &removal, elevate_delete)
    })
    .await?;
    notify_changed(&app);
    Ok(())
}

fn find_jdk(id: &str) -> anyhow::Result<Jdk> {
    detect::detect_all().into_iter().find(|j| j.id == id).ok_or_else(|| anyhow::anyhow!("unknown JDK {id}"))
}

/// Deletes a JDK folder from an elevated copy of the app (Windows).
fn elevate_delete(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()?;
        let args = format!("{ELEVATED_REMOVE_ARG} \"{}\"", path.display());
        let code = setup::windows::run_elevated(&exe, &args)?;
        anyhow::ensure!(code == 0, "elevated delete failed (exit code {code})");
        Ok(())
    }
    #[cfg(not(windows))]
    anyhow::bail!("cannot elevate to delete {}", path.display())
}

#[tauri::command]
async fn get_config() -> CmdResult<Config> {
    blocking(|| Ok(Config::load())).await
}

#[tauri::command]
async fn set_extra_scan_dirs(app: AppHandle, dirs: Vec<PathBuf>) -> CmdResult<()> {
    blocking(move || Config { extra_scan_dirs: dirs }.save()).await?;
    notify_changed(&app);
    Ok(())
}

/// Shows the main window, creating it if it was closed. Closing destroys the webview so
/// jman idles in the tray without WebView2's processes.
pub fn open_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.first() else { return };
    let builder = match WebviewWindowBuilder::from_config(app, config) {
        Ok(builder) => builder,
        Err(_) => return,
    };
    // the window starts hidden and is shown once the page has loaded, so there's no blank frame
    let builder = builder.on_page_load(|window, payload| {
        if payload.event() == PageLoadEvent::Finished {
            let _ = window.show();
            let _ = window.set_focus();
        }
    });
    // setting browser args replaces Tauri's defaults, so they're repeated here;
    // the UI is too simple to need the GPU process
    #[cfg(windows)]
    let builder = builder
        .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-gpu");
    let _ = builder.build();
}

/// Picks up switches made outside the app (the CLI, another instance) and refreshes the UI and tray.
fn watch_current_link(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = link::current_target();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let now = link::current_target();
            if now != last {
                last = now;
                notify_changed(&app);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // must be the first plugin: a second launch just focuses the running app
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| open_main_window(app)))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            tray::create(app.handle())?;
            watch_current_link(app.handle().clone());
            open_main_window(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && DOWNLOADS.load(Ordering::SeqCst) > 0
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_jdks,
            current_jdk,
            use_jdk,
            setup_status,
            doctor,
            run_setup,
            download_options,
            find_package,
            install_jdk,
            removal_plan,
            remove_jdk,
            get_config,
            set_extra_scan_dirs,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| match event {
            // closing the last window keeps jman in the tray; only Quit (`app.exit`) has a code
            RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => open_main_window(_app),
            _ => {}
        });
}
