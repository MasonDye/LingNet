//! LingNet —— 校园网自动登录器 (核心库)。

mod autostart;
mod commands;
pub mod config;
pub mod netdetect;
pub mod srun;
mod sys;
mod wifi;

use std::sync::Mutex;

use commands::AppState;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

/// 显示并聚焦主窗口。
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config::load();

    tauri::Builder::default()
        // 单实例：再次启动时聚焦已有窗口，避免多开。
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(AppState { config: Mutex::new(cfg) })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::set_autostart,
            commands::detect_network,
            commands::status,
            commands::connect,
            commands::disconnect,
        ])
        // 关闭按钮 = 收进托盘（后台保持运行，可自动重连）。
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            // ---- 托盘图标 ----
            let show_item = MenuItemBuilder::with_id("show", "显示主界面").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show_item, &quit_item]).build()?;

            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("灵网登录器")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;

            // ---- 启动后异步尝试自动登录 (不阻塞窗口显示) ----
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(res) = commands::try_auto_login(&handle).await {
                    let _ = handle.emit("auto-login-result", res);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
