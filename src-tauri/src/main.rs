// Windows release 下隐藏控制台窗口。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux/WebKitGTK：部分 GPU / 驱动 / 合成器下 DMABUF 渲染会导致
    // 白屏 / 灰屏（webview 有内容但不上屏）。禁用之以保证稳定渲染。
    // 用户若手动设置了该变量则尊重其取值。
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    lingnet_lib::run()
}
