// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod kdbx;
mod commands;
mod state;

use state::AppState;
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::Manager;
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use tauri::menu::{Menu, MenuItem};
use tauri_plugin_autostart::MacosLauncher;

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
            database: Mutex::new(None),
            initial_file_path: Mutex::new(None),
            dismissed_breaches: Mutex::new(HashMap::new()),
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
