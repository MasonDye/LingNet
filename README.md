# 灵网登录器 · 河南经贸校园网专版 (LingNet)

河南经贸校园网 **深澜 SRun** 认证的自动登录桌面小工具。基于 **Tauri 2 + React 19 + [Appica UI](https://appica.dev/ui)**，支持 Windows 与 Linux。

提前保存学号 / 密码后，启动即自动登录；也可手动点击中心的大圆形开关一键连入。内置 **WiFi SSID / 有线特征识别**，只有在校园网环境下才会执行登录。

---

## 功能

| 功能 | 说明 |
| --- | --- |
| 自动登录 | 启动软件后自动完成 SRun 认证（可在界面开关） |
| 手动登录 | 点击中心大圆形按钮即可连入 / 断开 |
| 代拨确认 | 联通代拨账号登录后轮询 `srun_portal_diallog`，确认「代拨成功」才算真正联网 |
| 自动连接校园 WiFi | 独立开关（默认关）：不在校园网时自动加入开放 WiFi 再登录 |
| 网络识别 | WiFi 校验 SSID（默认 `HNJM-Student-X`）；有线校验能否访问校园门户 |
| ac_id 自动探测 | 有线默认 `ac_id=3`；失败自动尝试其它值并记住 |
| 系统托盘 | 常驻托盘图标；点击唤起窗口，关闭窗口=收进托盘后台运行，托盘菜单可退出 |
| 开机自启 | 可选，随系统启动（预留了统一扩展入口，便于后续替换实现） |
| 网卡绑定 | Linux 走 `SO_BINDTODEVICE`，绕开 VPN(TUN) / 策略路由劫持 |
| 无闪窗 | Windows 下调用 `netsh` 等命令加 `CREATE_NO_WINDOW`，消除 cmd 黑窗闪动 |
| 单实例 | 重复启动时聚焦已有窗口 |

---

## 目录结构

```
LingNet/
├── index.html                # 前端入口
├── vite.config.ts            # Vite + Tailwind v4 + React
├── src/                      # React 前端 (Appica UI)
│   ├── main.tsx              # 挂载 + ThemeProvider
│   ├── App.tsx               # 主界面 (状态机 / 轮询 / 事件)
│   ├── index.css             # Tailwind + Appica 样式
│   ├── lib/api.ts            # Tauri 命令封装 (+ 浏览器 mock)
│   └── components/
│       ├── PowerButton.tsx   # 中心大圆形开关
│       └── Settings.tsx      # 账号 / SSID / 高级设置
└── src-tauri/                # Rust 后端
    ├── tauri.conf.json       # 窗口 360×600、打包配置
    ├── capabilities/         # 权限声明
    ├── examples/live_login.rs# 活体端到端测试
    └── src/
        ├── srun.rs           # SRun 认证协议 (加密 / 登录 / 注销 / 状态)
        ├── netdetect.rs      # 网络识别 (SSID / 有线特征 / 网卡绑定)
        ├── config.rs         # 配置持久化 (学号 / 密码 / ac_id)
        ├── autostart.rs      # 开机自启统一入口 (扩展口)
        ├── commands.rs       # Tauri 命令
        └── lib.rs / main.rs  # 入口
```

---

## 开发

前置依赖：**Node ≥ 20、Rust ≥ 1.77**；Linux 另需 `webkit2gtk-4.1`、`libsoup-3.0`、`gtk3` 等 Tauri 系统库。

```bash
npm install            # 安装前端依赖
npm run app:dev        # 启动 Tauri 开发模式 (热重载)
```

仅预览界面（无需 Tauri，走内置 mock 数据）：

```bash
npm run dev            # http://localhost:1420
```

---

## 打包

```bash
npm run app:build
```

- **Linux**：产物在 `src-tauri/target/release/bundle/`（`deb/`、`rpm/`、`appimage/`）。
- **Windows**：需在 Windows 上执行（或配置交叉工具链），产物为 `nsis/*.exe` 与 `msi/*.msi`。

> 本仓库在 Linux 上开发，Windows 安装包请在 Windows 环境执行 `npm run app:build` 生成。

> ⚠️ **AppImage 打包**：tauri 会从 GitHub 下载 `linuxdeploy`，网络不佳时可能失败（`failed to run linuxdeploy`）。
> 建议加环境变量并在失败时重试：
> ```bash
> APPIMAGE_EXTRACT_AND_RUN=1 npm run app:build -- --bundles appimage
> ```
> `deb` / `rpm` 不依赖该工具，总能正常产出。

> ❗ **必须用 `tauri build`（即 `npm run app:build`）打包**，不要直接 `cargo build`——
> 后者不会把前端资源嵌入二进制，运行时会去连开发服务器 `localhost:1420` 而显示 “Connection refused”。

---

## 测试

```bash
cd src-tauri
cargo test                         # 加密算法与配置的单元测试
cargo run --example live_login     # 连真实网关走完整登录流程 (需在校园网内)
cargo run --example live_login -- <学号> <密码>
```

`cargo test` 中的 `info_matches_reference` / `hmac_md5_known` 用固定向量校验 Rust 实现与门户 JS **逐字节一致**。

---

## 技术说明：SRun 认证协议

网关：`http://10.16.1.114`（`SRunCGIAuthIntfSvr V1.18 B20240604`）。

```
1. GET /cgi-bin/get_challenge   -> token(challenge) + online_ip
2. hmd5   = HMAC-MD5(password, token)
   info   = "{SRBX1}" + srun_base64( xxtea( json(userinfo), token ) )
   chksum = SHA1( token+user token+hmd5 token+acid token+ip token+n token+type token+info )
3. GET /cgi-bin/srun_portal      -> action=login, 携带以上参数
4. GET /cgi-bin/rad_user_info    -> 校验在线状态
注销: GET /cgi-bin/srun_portal?action=logout&username=&ip=&ac_id=
```

- `srun_base64` 使用自定义码表：`LVoJPiCN2R8G90yg+hmFHuacZ1OWMnrsSTXkYpUq/3dlbfKwv6xztjI7DeBE45QA`
- `xxtea` 为深澜改造版（`delta = 0x9E3779B9`）。
- **ac_id**：门户默认重定向给的 `ac_id=1` 对有线网无效，正确值为 **3**；程序会自动探测并记住。

---

## 界面与窗口

- 窗口固定 **360×600**，**禁止**调整大小 / 最大化 / 全屏；页面缩放（Ctrl+滚轮、Ctrl±0）与 F11 也被屏蔽。
- 使用**自定义标题栏**（`decorations: false` + `TitleBar.tsx`），不显示系统操作栏；标题栏可拖动窗口，右侧仅最小化与关闭。
- **Linux 灰屏/白屏修复**：部分 GPU / 驱动 / 合成器下 WebKitGTK 的 DMABUF 渲染会导致 webview 不上屏。
  `main.rs` 启动时默认设置 `WEBKIT_DISABLE_DMABUF_RENDERER=1`（用户已手动设置则尊重）。如仍异常，可再试
  `WEBKIT_DISABLE_COMPOSITING_MODE=1`（强制软件渲染）。

---

## 安全与隐私

- 学号 / 密码仅保存在本机配置目录：
  - Linux：`~/.config/lingnet/config.json`
  - Windows：`%APPDATA%\lingnet\config.json`
- 密码在磁盘上做了简单可逆编码（仅防明文扫描，非强加密）。若需接入系统密钥链，只改 `config.rs` 的 `load/save` 即可。
- 不上传任何数据，所有请求只发往校园网关。
