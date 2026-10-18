use jman_core::{detect, link};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

const TRAY_ID: &str = "main";
const JDK_PREFIX: &str = "jdk:";

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    let current = link::current_target().map(|t| detect::canonical_key(&t));
    let jdks = detect::detect_all();
    if jdks.is_empty() {
        menu.append(&MenuItem::new(app, "No JDKs found", false, None::<&str>)?)?;
    }
    for jdk in jdks {
        let checked = current.as_ref() == Some(&detect::canonical_key(&jdk.home));
        let label = format!("{}  ({})", jdk.id, jdk.version);
        menu.append(&CheckMenuItem::with_id(app, format!("{JDK_PREFIX}{}", jdk.id), label, true, checked, None::<&str>)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "open", "Open jman", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("jman")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::show_main_window(app),
            "quit" => app.exit(0),
            id => {
                if let Some(jdk) = id.strip_prefix(JDK_PREFIX) {
                    let jdk = jdk.to_string();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = crate::use_jdk(app, jdk).await;
                    });
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Rebuilds the menu so the check marks and the JDK list are current.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}
