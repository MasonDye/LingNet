//! 网络环境识别。
//!
//! 需求: 只有当处于校园网时才允许操作 —
//!   - WiFi: 当前连接的 SSID 必须在允许列表内 (默认 HNJM-Student-X)。
//!   - 有线: 以太网卡能访问到 SRun 门户 (返回 SRunFlag) —— 即"有线特征正确"。
//!
//! 判定结果同时给出应当绑定的网卡 / 源 IP，供后续认证请求使用，
//! 以绕开本机可能存在的 VPN(TUN) / 策略路由劫持。

use std::net::IpAddr;

use serde::Serialize;

use crate::srun::{self, Bind};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectResult {
    /// 是否允许继续 (SSID 命中 或 有线特征命中)。
    pub allowed: bool,
    /// "wifi" | "wired" | "none"
    pub kind: String,
    /// 当前 WiFi SSID (若在 WiFi 上)。
    pub ssid: Option<String>,
    /// 选定的网卡名。
    pub interface: Option<String>,
    /// 选定网卡的源 IPv4。
    pub source_ip: Option<String>,
    /// 面向用户的说明。
    pub reason: String,
}

impl DetectResult {
    fn none(reason: impl Into<String>) -> Self {
        DetectResult {
            allowed: false,
            kind: "none".into(),
            ssid: None,
            interface: None,
            source_ip: None,
            reason: reason.into(),
        }
    }

    /// 从判定结果导出认证请求应使用的绑定目标。
    pub fn bind(&self) -> Bind {
        Bind {
            interface: self.interface.clone(),
            source_ip: self.source_ip.as_ref().and_then(|s| s.parse().ok()),
        }
    }
}

struct Iface {
    name: String,
    ipv4: IpAddr,
}

fn is_virtual(name: &str) -> bool {
    let n = name.to_lowercase();
    const V: &[&str] = &[
        "lo", "docker", "br-", "veth", "virbr", "tun", "tap", "sing", "wg",
        "utun", "ppp", "zt", "tailscale", "vmnet", "vboxnet", "hyper-v",
        "vethernet", "loopback", "isatap", "teredo",
    ];
    V.iter().any(|p| n.starts_with(p) || n.contains(p))
}

fn is_wifi_name(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("wl") // wlan0 / wlp*
        || n.contains("wi-fi")
        || n.contains("wifi")
        || n.contains("wlan")
        || n.contains("wireless")
        || n.contains("无线")
}

fn is_ethernet_name(name: &str) -> bool {
    if is_virtual(name) || is_wifi_name(name) {
        return false;
    }
    let n = name.to_lowercase();
    n.starts_with("en") // eno1 / enp* / ens* / eth*
        || n.starts_with("eth")
        || n.contains("ethernet")
        || n.contains("以太网")
        || n.contains("local area connection")
}

/// 列出带 IPv4 的物理网卡。
fn list_ifaces() -> Vec<Iface> {
    let mut out = Vec::new();
    if let Ok(addrs) = if_addrs::get_if_addrs() {
        for a in addrs {
            if a.is_loopback() {
                continue;
            }
            if let IpAddr::V4(_) = a.ip() {
                out.push(Iface { name: a.name.clone(), ipv4: a.ip() });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SSID 读取 (跨平台)
// ---------------------------------------------------------------------------

/// 返回 (ssid, wifi 网卡名)。任一未知则为 None。
#[cfg(target_os = "linux")]
fn active_wifi() -> (Option<String>, Option<String>) {
    use crate::sys::command;
    // 优先 nmcli: `DEVICE:TYPE:STATE:CONNECTION`
    if let Ok(o) = command("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID,DEVICE", "dev", "wifi"])
        .output()
    {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                // 形如 `yes:HNJM-Student-X:wlan0`
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() >= 2 && parts[0] == "yes" {
                    let ssid = parts[1].trim().to_string();
                    let dev = parts.get(2).map(|s| s.trim().to_string());
                    if !ssid.is_empty() {
                        return (Some(ssid), dev);
                    }
                }
            }
        }
    }
    // 回退 iwgetid
    if let Ok(o) = command("iwgetid").args(["-r"]).output() {
        if o.status.success() {
            let ssid = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !ssid.is_empty() {
                return (Some(ssid), None);
            }
        }
    }
    (None, None)
}

#[cfg(target_os = "windows")]
fn active_wifi() -> (Option<String>, Option<String>) {
    use crate::sys::command;
    // `netsh wlan show interfaces` 输出含 SSID 与网卡名。
    if let Ok(o) = command("netsh").args(["wlan", "show", "interfaces"]).output() {
        let text = decode_console(&o.stdout);
        let mut ssid = None;
        let mut name = None;
        for line in text.lines() {
            let l = line.trim();
            let lower = l.to_lowercase();
            // 只取以 "SSID" 开头的行 (排除 "BSSID")。
            if lower.starts_with("ssid") && !lower.starts_with("bssid") {
                if let Some((_, v)) = l.split_once(':') {
                    ssid = Some(v.trim().to_string());
                }
            } else if lower.starts_with("name") || l.starts_with("名称") {
                if let Some((_, v)) = l.split_once(':') {
                    name = Some(v.trim().to_string());
                }
            }
        }
        let ssid = ssid.filter(|s| !s.is_empty());
        return (ssid, name);
    }
    (None, None)
}

#[cfg(target_os = "windows")]
fn decode_console(bytes: &[u8]) -> String {
    // netsh 在中文系统上输出 GBK；尽量按 UTF-8 解析，失败则有损转换。
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(bytes).to_string(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn active_wifi() -> (Option<String>, Option<String>) {
    (None, None)
}

/// 当前连接的 WiFi SSID（若在 WiFi 上）。供 WiFi 自动连接模块判定用。
pub fn active_ssid() -> Option<String> {
    active_wifi().0
}

// ---------------------------------------------------------------------------
// 主判定
// ---------------------------------------------------------------------------

/// `ssid_allowlist`: 允许的校园 WiFi SSID (默认应包含 "HNJM-Student-X")。
pub async fn detect(ssid_allowlist: &[String]) -> DetectResult {
    let ifaces = list_ifaces();
    let (ssid, wifi_dev) = active_wifi();

    // ---- 1) WiFi 路径: SSID 必须命中允许列表 ----
    if let Some(ref current) = ssid {
        let hit = ssid_allowlist.iter().any(|s| s.eq_ignore_ascii_case(current));
        if hit {
            // 选定 wifi 网卡的源 IP。
            let wifi_iface = ifaces.iter().find(|i| {
                wifi_dev.as_deref().map(|d| d == i.name).unwrap_or(false) || is_wifi_name(&i.name)
            });
            let (interface, source_ip) = match wifi_iface {
                Some(i) => (Some(i.name.clone()), Some(i.ipv4.to_string())),
                None => (wifi_dev.clone(), None),
            };
            return DetectResult {
                allowed: true,
                kind: "wifi".into(),
                ssid: Some(current.clone()),
                interface,
                source_ip,
                reason: format!("已连接校园 WiFi「{current}」"),
            };
        }
    }

    // ---- 2) 有线路径: 找到能访问 SRun 门户的以太网卡 ----
    for i in ifaces.iter().filter(|i| is_ethernet_name(&i.name)) {
        let bind = Bind::interface(i.name.clone());
        if srun::probe_portal(&bind).await {
            return DetectResult {
                allowed: true,
                kind: "wired".into(),
                ssid: ssid.clone(),
                interface: Some(i.name.clone()),
                source_ip: Some(i.ipv4.to_string()),
                reason: format!("已识别校园有线网络 (网卡 {}，门户可达)", i.name),
            };
        }
    }

    // ---- 3) 都不满足 ----
    let mut r = DetectResult::none(match &ssid {
        Some(s) => format!("当前 WiFi「{s}」不是校园网，且未检测到校园有线网络"),
        None => "未检测到校园 WiFi 或有线网络".into(),
    });
    r.ssid = ssid;
    r
}
