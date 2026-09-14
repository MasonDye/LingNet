//! 自动连接校园 WiFi（开放网络，无 WiFi 密码；认证由 SRun 门户完成）。
//!
//! - Linux:   `nmcli dev wifi connect "<ssid>"`
//! - Windows: 确保存在开放网络配置文件后 `netsh wlan connect name="<ssid>"`
//!
//! 说明: 校园 WiFi (如 HNJM-Student-X) 通常是开放网络，连接本身不需要密码，
//! 上网前的实名认证由本程序的 SRun 登录完成。该功能默认关闭，需用户显式开启。

use crate::sys::command;

/// 当前是否已连接到指定 SSID。
pub fn current_ssid_is(ssid: &str) -> bool {
    crate::netdetect::active_ssid()
        .map(|s| s.eq_ignore_ascii_case(ssid))
        .unwrap_or(false)
}

/// 连接到指定的开放 WiFi。成功返回 Ok(())。
pub fn connect(ssid: &str) -> Result<(), String> {
    if ssid.trim().is_empty() {
        return Err("未配置校园 WiFi 名称".into());
    }
    if current_ssid_is(ssid) {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        connect_linux(ssid)
    }
    #[cfg(target_os = "windows")]
    {
        connect_windows(ssid)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = ssid;
        Err("当前平台不支持自动连接 WiFi".into())
    }
}

#[cfg(target_os = "linux")]
fn connect_linux(ssid: &str) -> Result<(), String> {
    // 先刷新一次扫描（忽略错误），再连接开放网络。
    let _ = command("nmcli").args(["dev", "wifi", "rescan"]).output();
    let out = command("nmcli")
        .args(["dev", "wifi", "connect", ssid])
        .output()
        .map_err(|e| format!("调用 nmcli 失败: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "连接 WiFi 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(target_os = "windows")]
fn connect_windows(ssid: &str) -> Result<(), String> {
    // 先直接尝试连接（若系统已存在该网络的配置文件）。
    if netsh_connect(ssid).is_ok() {
        return Ok(());
    }
    // 否则为开放网络添加一个配置文件后再连接。
    add_open_profile(ssid)?;
    netsh_connect(ssid)
}

#[cfg(target_os = "windows")]
fn netsh_connect(ssid: &str) -> Result<(), String> {
    let out = command("netsh")
        .args([
            "wlan",
            "connect",
            &format!("name={ssid}"),
            &format!("ssid={ssid}"),
        ])
        .output()
        .map_err(|e| format!("调用 netsh 失败: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// 为开放网络（无加密）生成并导入一个 WLAN 配置文件。
#[cfg(target_os = "windows")]
fn add_open_profile(ssid: &str) -> Result<(), String> {
    use std::io::Write;
    // SSID 十六进制（netsh 对含特殊字符的 SSID 更稳）。
    let hex: String = ssid.as_bytes().iter().map(|b| format!("{b:02X}")).collect();
    let xml = format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
  <name>{ssid}</name>
  <SSIDConfig>
    <SSID>
      <hex>{hex}</hex>
      <name>{ssid}</name>
    </SSID>
  </SSIDConfig>
  <connectionType>ESS</connectionType>
  <connectionMode>auto</connectionMode>
  <MSM>
    <security>
      <authEncryption>
        <authentication>open</authentication>
        <encryption>none</encryption>
        <useOneX>false</useOneX>
      </authEncryption>
    </security>
  </MSM>
</WLANProfile>
"#
    );
    let mut path = std::env::temp_dir();
    path.push(format!("lingnet_wlan_{hex}.xml"));
    let mut f = std::fs::File::create(&path).map_err(|e| format!("写入 WLAN 配置失败: {e}"))?;
    f.write_all(xml.as_bytes()).map_err(|e| format!("写入 WLAN 配置失败: {e}"))?;
    let out = command("netsh")
        .args([
            "wlan",
            "add",
            "profile",
            &format!("filename={}", path.display()),
            "user=current",
        ])
        .output()
        .map_err(|e| format!("导入 WLAN 配置失败: {e}"))?;
    let _ = std::fs::remove_file(&path);
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}
