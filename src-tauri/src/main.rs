// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use simple_password_manager::{bridge, state};

use state::AppState;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tauri::Manager;
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use tauri::menu::{Menu, MenuItem};
use tauri_plugin_autostart::MacosLauncher;

// Tauri-aware impl of the bridge's FocusController trait. Kept in the bin
// crate so the lib stays free of WebView/Tauri runtime types.
struct TauriFocusController {
    app_handle: tauri::AppHandle,
}

impl bridge::FocusController for TauriFocusController {
    fn focus_main_window(&self) {
        let Some(window) = self.app_handle.get_webview_window("main") else {
            return;
        };
        let _ = window.unminimize();
        let _ = window.show();
        // Windows refuses focus-steal from a background process unless
        // we toggle always-on-top briefly — same trick KeePassXC uses.
        let _ = window.set_always_on_top(true);
        let _ = window.set_focus();
        let _ = window.set_always_on_top(false);
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(AppState {
            database: Arc::new(Mutex::new(None)),
            initial_file_path: Mutex::new(None),
            dismissed_breaches: Mutex::new(HashMap::new()),
            file_watcher: Mutex::new(None),
            cloud: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::database::get_initial_file_path,
            commands::database::clear_initial_file_path,
            commands::database::create_database,
            commands::database::open_database,
            commands::database::save_database,
            commands::database::close_database,
            commands::database::get_kdf_info,
            commands::database::upgrade_kdf_parameters,
            commands::database::check_database_changes,
            commands::database::merge_database,
            commands::database::analyze_conflicts,
            commands::database::resolve_conflicts,
            commands::database::peek_lock_status,
            commands::database::open_database_in_new_instance,
            commands::database::validate_database_file,
            commands::database::get_groups,
            commands::entry::get_entries,
            commands::entry::get_favorite_entries,
            commands::entry::get_entry,
            commands::entry::create_entry,
            commands::entry::update_entry,
            commands::entry::delete_entry,
            commands::entry::move_entry,
            commands::group::create_group,
            commands::group::rename_group,
            commands::group::move_group,
            commands::group::reorder_group,
            commands::group::delete_group,
            commands::search::search_entries,
            commands::search::search_entries_in_group,
            commands::password::generate_password,
            commands::search::get_dashboard_stats,
            commands::security::check_breached_passwords,
            commands::security::save_dismissed_breach,
            commands::security::get_dismissed_breaches,
            commands::security::clear_dismissed_breach,
            commands::browser_extension::detect_browsers,
            commands::browser_extension::install_native_host,
            commands::browser_extension::uninstall_native_host,
            commands::totp::preview_totp,
            commands::yubikey::list_yubikeys,
            commands::yubikey::enable_yubikey,
            commands::yubikey::disable_yubikey,
            commands::yubikey::yubikey_enabled_for_open_db,
            commands::hello::hello_available,
            commands::hello::hello_is_enrolled,
            commands::hello::hello_store,
            commands::hello::hello_retrieve,
            commands::hello::hello_clear,
            commands::cloud::cloud_signup,
            commands::cloud::cloud_login,
            commands::cloud::cloud_recover,
            commands::cloud::cloud_push,
            commands::cloud::cloud_pull,
            commands::cloud::cloud_status,
            commands::cloud::cloud_disconnect,
            commands::cloud_persistence::cloud_persist_session,
            commands::cloud_persistence::cloud_rehydrate_session,
            commands::cloud_persistence::cloud_forget_session,
        ])
        .setup(|app| {
            // Set up system tray
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Simple Password Manager")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle file associations and CLI flags from the launched process args
            let args: Vec<String> = std::env::args().collect();
            let mut start_minimized = false;
            for arg in args.iter().skip(1) {
                if arg == "--minimized" {
                    start_minimized = true;
                } else if arg.ends_with(".kdbx") {
                    println!("Opening database from file association: {}", arg);
                    if let Ok(mut initial_path) = app.state::<AppState>().initial_file_path.lock() {
                        *initial_path = Some(arg.clone());
                    }
                }
            }

            if start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Boot the browser-extension bridge in the background. The HTTP
            // server stays up for the lifetime of the app; we don't keep the
            // BridgeHandle around because there's nothing to do with it after
            // the bridge.json is written — the OS reclaims sockets at process
            // exit, and remove_bridge_file() in the exit path handles cleanup.
            let app_handle = app.handle().clone();
            let db_handle = app.state::<AppState>().database.clone();
            let focus_controller: Arc<dyn bridge::FocusController> =
                Arc::new(TauriFocusController { app_handle });
            tauri::async_runtime::spawn(async move {
                if let Err(e) = launch_bridge(focus_controller, db_handle).await {
                    eprintln!("[bridge] failed to start: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // On graceful shutdown, wipe the bridge file so a stale token can't be
    // used after this process exits. Tauri returns from `.run()` only when
    // the app is fully torn down, so reaching this point is safe.
    if let Some(dir) = bridge::bridge_state_dir() {
        bridge::remove_bridge_file(&dir);
    }
}

async fn launch_bridge(
    focus_controller: Arc<dyn bridge::FocusController>,
    db_handle: state::DatabaseHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bridge_file = bridge::start(db_handle, focus_controller).await?;

    let dir = bridge::bridge_state_dir()
        .ok_or("could not determine local data dir for bridge.json")?;
    let path = bridge::write_bridge_file(&dir, &bridge_file)?;

    println!(
        "[bridge] listening on 127.0.0.1:{}, state file: {}",
        bridge_file.port,
        path.display()
    );
    Ok(())
}
