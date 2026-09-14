//! 开机自启 —— 统一入口 (预留扩展口)。
//!
//! 目前基于 `tauri-plugin-autostart` (Windows 注册表 Run 项 / Linux XDG autostart)。
//! 后续若要改用其它机制 (systemd user unit、任务计划程序、launchd 等)，
//! 只需替换本文件的 `apply` / `is_enabled` 两个函数，其余代码无需改动。

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// 按需注册 / 注销开机自启。
pub fn apply(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let mgr = app.autolaunch();
    let currently = mgr.is_enabled().unwrap_or(false);
    if enabled && !currently {
        mgr.enable().map_err(|e| format!("开启开机自启失败: {e}"))?;
    } else if !enabled && currently {
        mgr.disable().map_err(|e| format!("关闭开机自启失败: {e}"))?;
    }
    Ok(())
}

/// 查询系统层面是否已注册开机自启。
#[allow(dead_code)]
pub fn is_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}
