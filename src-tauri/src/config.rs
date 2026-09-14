//! 配置持久化。
//!
//! 存放位置: 平台标准 app config 目录
//!   - Linux:   ~/.config/lingnet/config.json
//!   - Windows: %APPDATA%\lingnet\config.json
//!
//! 说明: 学号/密码保存在本地配置文件中，仅供本机自动登录使用。
//! 密码在磁盘上做了简单可逆编码 (仅防肩窥/明文扫描，非强加密)。
//! 若日后需要接入系统密钥链，`load`/`save` 是唯一改动点。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_true() -> bool { true }
fn default_acid() -> String { "3".into() }
fn default_ssids() -> Vec<String> { vec!["HNJM-Student-X".into()] }

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// 学号。
    pub username: String,
    /// 密码 (在磁盘上以 obfuscate 编码存储)。
    pub password: String,
    /// 启动软件后自动登录。
    pub auto_login: bool,
    /// 开机自启 (预留开关，具体注册由 autostart 插件负责)。
    pub autostart: bool,
    /// 自动连接校园 WiFi (开放网络)，默认关闭。
    #[serde(default)]
    pub auto_connect_wifi: bool,
    /// 记住的可用 ac_id (有线默认 3)。
    #[serde(default = "default_acid")]
    pub ac_id: String,
    /// 允许的校园 WiFi SSID 列表。
    #[serde(default = "default_ssids")]
    pub ssid_allowlist: Vec<String>,
    /// 登录成功后保持后台运行 (预留)。
    #[serde(default = "default_true")]
    pub keep_running: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            username: String::new(),
            password: String::new(),
            auto_login: true,
            autostart: false,
            auto_connect_wifi: false,
            ac_id: default_acid(),
            ssid_allowlist: default_ssids(),
            keep_running: true,
        }
    }
}

/// 传给前端 / 从前端接收的配置 (密码为明文，仅在内存/IPC 中)。
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDto {
    pub username: String,
    pub password: String,
    pub auto_login: bool,
    pub autostart: bool,
    pub auto_connect_wifi: bool,
    pub ac_id: String,
    pub ssid_allowlist: Vec<String>,
    pub keep_running: bool,
    /// 是否已保存过密码 (前端用于占位显示)。
    pub has_password: bool,
}

impl Config {
    pub fn to_dto(&self) -> ConfigDto {
        ConfigDto {
            username: self.username.clone(),
            password: self.password_plain(),
            auto_login: self.auto_login,
            autostart: self.autostart,
            auto_connect_wifi: self.auto_connect_wifi,
            ac_id: self.ac_id.clone(),
            ssid_allowlist: self.ssid_allowlist.clone(),
            keep_running: self.keep_running,
            has_password: !self.password.is_empty(),
        }
    }

    pub fn apply_dto(&mut self, dto: ConfigDto) {
        self.username = dto.username.trim().to_string();
        // 空密码表示"不修改" (前端占位)，非空才覆盖。
        if !dto.password.is_empty() {
            self.password = obfuscate(&dto.password);
        }
        self.auto_login = dto.auto_login;
        self.autostart = dto.autostart;
        self.auto_connect_wifi = dto.auto_connect_wifi;
        if !dto.ac_id.trim().is_empty() {
            self.ac_id = dto.ac_id.trim().to_string();
        }
        if !dto.ssid_allowlist.is_empty() {
            self.ssid_allowlist = dto
                .ssid_allowlist
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        self.keep_running = dto.keep_running;
    }

    pub fn password_plain(&self) -> String {
        deobfuscate(&self.password)
    }
}

fn config_path() -> Result<PathBuf, String> {
    let dir = dirs_config_dir().ok_or("无法定位配置目录")?;
    Ok(dir.join("lingnet").join("config.json"))
}

/// 跨平台 config 目录 (避免额外依赖 dirs crate)。
fn dirs_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(x));
        }
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
    }
}

pub fn load() -> Config {
    match config_path().and_then(|p| std::fs::read_to_string(&p).map_err(|e| e.to_string())) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("写入配置失败: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 轻量可逆编码 (XOR + base64)，仅防明文扫描，不是安全加密。
// ---------------------------------------------------------------------------

const XOR_KEY: &[u8] = b"LingNet::HNJM::v1";

fn xor(data: &[u8]) -> Vec<u8> {
    data.iter().enumerate().map(|(i, b)| b ^ XOR_KEY[i % XOR_KEY.len()]).collect()
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(raw: &[u8]) -> String {
    let mut out = String::new();
    for chunk in raw.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn b64_decode(s: &str) -> Vec<u8> {
    let idx = |c: u8| -> Option<u32> { B64.iter().position(|&x| x == c).map(|p| p as u32) };
    let cleaned: Vec<u8> = s.bytes().filter(|&c| c != b'=' && !c.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in cleaned.chunks(4) {
        let mut n = 0u32;
        let mut bits = 0;
        for &c in chunk {
            if let Some(v) = idx(c) {
                n = n << 6 | v;
                bits += 6;
            }
        }
        // 对齐到字节。
        n <<= 6 * (4 - chunk.len());
        bits += 6 * (4 - chunk.len());
        let _ = bits;
        let bytes = [(n >> 16 & 0xff) as u8, (n >> 8 & 0xff) as u8, (n & 0xff) as u8];
        let take = chunk.len().saturating_sub(1);
        out.extend_from_slice(&bytes[..take]);
    }
    out
}

fn obfuscate(plain: &str) -> String {
    b64_encode(&xor(plain.as_bytes()))
}

fn deobfuscate(stored: &str) -> String {
    if stored.is_empty() {
        return String::new();
    }
    String::from_utf8(xor(&b64_decode(stored))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obfuscate_roundtrip() {
        for p in ["Demo@2024", "", "简单密码123!@#", "a"] {
            let enc = obfuscate(p);
            assert_eq!(deobfuscate(&enc), p, "failed for {p:?}");
        }
    }
}
