//! 系统调用辅助。
//!
//! Windows 下 GUI 程序调用控制台子进程 (netsh 等) 会弹出黑色 cmd 窗口，
//! 这里统一用 `CREATE_NO_WINDOW` 标志创建命令以消除闪窗。

use std::process::Command;

/// 创建一个在 Windows 上不弹出控制台窗口的 `Command`。
pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // https://learn.microsoft.com/windows/win32/procthread/process-creation-flags
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}
