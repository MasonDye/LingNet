//! 前端可调用的 Tauri 命令。

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::autostart;
use crate::config::{self, Config, ConfigDto};
use crate::netdetect::{self, DetectResult};
use crate::srun::{self, Bind, SrunResp};

/// 全局配置状态。
pub struct AppState {
    pub config: Mutex<Config>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusResult {
    /// 网络识别结果。
    pub detect: DetectResult,
    /// 是否已在线 (rad_user_info)。
    pub online: bool,
    /// 在线账号 (若在线)。
    pub user_name: Option<String>,
    /// 说明。
    pub message: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResult {
    pub success: bool,
    pub online: bool,
    pub kind: String,
    pub ssid: Option<String>,
    pub user_name: Option<String>,
    /// 实际生效的 ac_id。
    pub ac_id: String,
    pub message: String,
    /// 代拨结果提示（仅联通代拨账号有值），如「代拨成功」。
    pub dial: Option<String>,
}

// ---------------------------------------------------------------------------
// 配置
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_config(state: State<AppState>) -> ConfigDto {
    state.config.lock().unwrap().to_dto()
}

#[tauri::command]
pub fn save_config(app: AppHandle, state: State<AppState>, dto: ConfigDto) -> Result<(), String> {
    let want_autostart = dto.autostart;
    let mut cfg = state.config.lock().unwrap();
    cfg.apply_dto(dto);
    config::save(&cfg)?;
    drop(cfg);
    // 同步开机自启注册。
    autostart::apply(&app, want_autostart)?;
    Ok(())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<(), String> {
    autostart::apply(&app, enabled)?;
    let mut cfg = state.config.lock().unwrap();
    cfg.autostart = enabled;
    config::save(&cfg)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 网络识别 / 状态
// ---------------------------------------------------------------------------

fn allowlist(state: &State<AppState>) -> Vec<String> {
    state.config.lock().unwrap().ssid_allowlist.clone()
}

#[tauri::command]
pub async fn detect_network(state: State<'_, AppState>) -> Result<DetectResult, String> {
    let list = allowlist(&state);
    Ok(netdetect::detect(&list).await)
}

#[tauri::command]
pub async fn status(state: State<'_, AppState>) -> Result<StatusResult, String> {
    let list = allowlist(&state);
    let detect = netdetect::detect(&list).await;

    // 未识别到校园网时，仍尝试用默认绑定查一次在线状态。
    let bind = detect.bind();
    let (online, user_name) = match srun::rad_user_info(&bind).await {
        Ok(r) if r.error == "ok" => (true, Some(r.user_name)),
        _ => (false, None),
    };

    let message = if online {
        format!("已在线：{}", user_name.clone().unwrap_or_default())
    } else if detect.allowed {
        "未登录".into()
    } else {
        detect.reason.clone()
    };

    Ok(StatusResult { detect, online, user_name, message })
}

// ---------------------------------------------------------------------------
// 登录 / 注销
// ---------------------------------------------------------------------------

enum LoginOutcome {
    Success,
    AlreadyOnline,
    AcidMismatch,
    Fail(String),
}

fn classify(resp: &SrunResp) -> LoginOutcome {
    if resp.error == "ok" || resp.suc_msg == "login_ok" {
        return LoginOutcome::Success;
    }
    let e = resp.error.as_str();
    let msg = resp.error_msg.as_str();
    if e.contains("already_online") || resp.suc_msg == "ip_already_online_error" {
        return LoginOutcome::AlreadyOnline;
    }
    if e == "speed_limit_error" {
        // 频繁请求，通常意味着已在线，交由上层复查。
        return LoginOutcome::AlreadyOnline;
    }
    // ac_id 不匹配的几种典型报错。
    if msg.contains("err_code=2")
        || msg.contains("Control policy not found")
        || msg.contains("Nas type not found")
    {
        return LoginOutcome::AcidMismatch;
    }
    LoginOutcome::Fail(translate(resp))
}

/// 把 SRun 报错翻成中文提示。
fn translate(resp: &SrunResp) -> String {
    let raw = if !resp.error_msg.is_empty() { &resp.error_msg } else { &resp.error };
    let map = [
        ("E2531", "账号已欠费"),
        ("E2606", "用户不存在"),
        ("E2612", "密码错误"),
        ("E2620", "已达到最大在线设备数"),
        ("E2616", "账号被冻结"),
        ("E2839", "无法连接认证服务器"),
        ("password_error", "密码错误"),
        ("Password is error", "密码错误"),
        ("userid_error", "学号不存在"),
        ("User not found", "学号不存在"),
        ("not_online_error", "当前未在线"),
        ("ip_already_online_error", "该 IP 已在线"),
    ];
    for (k, v) in map {
        if raw.contains(k) {
            return v.to_string();
        }
    }
    if raw.is_empty() { "登录失败".into() } else { raw.clone() }
}

/// 候选 ac_id 顺序：优先记忆值，其后是常见值。
fn acid_candidates(preferred: &str) -> Vec<String> {
    let mut v = vec![preferred.to_string()];
    for c in ["3", "1", "2", "4", "5", "6"] {
        if !v.iter().any(|x| x == c) {
            v.push(c.to_string());
        }
    }
    v
}

async fn is_online(bind: &Bind) -> Option<String> {
    online_resp(bind).await.map(|r| r.user_name)
}

/// 若在线则返回完整响应 (含 pppoe_dial 等)。
async fn online_resp(bind: &Bind) -> Option<SrunResp> {
    match srun::rad_user_info(bind).await {
        Ok(r) if r.error == "ok" => Some(r),
        _ => None,
    }
}

/// 代拨确认：仅联通代拨账号 (pppoe_dial==1) 需要。轮询 diallog，
/// code=0 代拨成功；code=100 拨号中(继续等)；其它为失败。期间通过事件反馈进度。
async fn confirm_dial(
    app: &AppHandle,
    bind: &Bind,
    resp: &SrunResp,
    username: &str,
) -> Option<srun::DialResult> {
    if resp.pppoe_dial != "1" {
        return None;
    }
    let mut last = srun::DialResult { code: 100, message: "正在拨号中，请稍后…".into() };
    for attempt in 1..=6u32 {
        let _ = app.emit(
            "dial-progress",
            serde_json::json!({ "attempt": attempt, "code": 100, "message": "正在拨号中，请等待…" }),
        );
        if let Ok(d) = srun::dial_status(bind, username).await {
            let _ = app.emit("dial-progress", &d);
            if d.code != 100 {
                return Some(d); // 成功(0) 或失败(其它)
            }
            last = d;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Some(last) // 超时仍在拨号
}

fn fail(detect: &DetectResult, ac_id: String, message: String) -> ConnectResult {
    ConnectResult {
        success: false,
        online: false,
        kind: detect.kind.clone(),
        ssid: detect.ssid.clone(),
        user_name: None,
        ac_id,
        message,
        dial: None,
    }
}

/// 完整登录流程：识别 → (自动WiFi) → 校验/登录(ac_id 探测) → 代拨确认。
#[tauri::command]
pub async fn connect(app: AppHandle, state: State<'_, AppState>) -> Result<ConnectResult, String> {
    let (username, password, preferred_acid, list, auto_wifi) = {
        let cfg = state.config.lock().unwrap();
        (
            cfg.username.clone(),
            cfg.password_plain(),
            cfg.ac_id.clone(),
            cfg.ssid_allowlist.clone(),
            cfg.auto_connect_wifi,
        )
    };

    if username.is_empty() || password.is_empty() {
        return Ok(ConnectResult {
            success: false,
            online: false,
            kind: "none".into(),
            ssid: None,
            user_name: None,
            ac_id: preferred_acid,
            message: "请先在设置中填写学号和密码".into(),
            dial: None,
        });
    }

    // 1) 网络识别 (SSID / 有线特征)。
    let mut detect = netdetect::detect(&list).await;

    // 1.5) 若未在校园网且开启了「自动连接校园 WiFi」，尝试加入校园 WiFi 后重新识别。
    if !detect.allowed && auto_wifi {
        if let Some(ssid) = list.first().cloned() {
            let ssid_cl = ssid.clone();
            let joined = tokio::task::spawn_blocking(move || crate::wifi::connect(&ssid_cl))
                .await
                .unwrap_or_else(|e| Err(format!("WiFi 连接任务失败: {e}")));
            if joined.is_ok() {
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    detect = netdetect::detect(&list).await;
                    if detect.allowed {
                        break;
                    }
                }
            }
        }
    }

    if !detect.allowed {
        return Ok(fail(&detect, preferred_acid, detect.reason.clone()));
    }
    let bind = detect.bind();

    // 2) 到达"在线"状态：已在线直接用；否则带 ac_id 探测登录。
    let mut online = online_resp(&bind).await;
    let mut used_acid = preferred_acid.clone();
    let mut last_msg = String::from("登录失败");

    if online.is_none() {
        for acid in acid_candidates(&preferred_acid) {
            let resp = match srun::login_once(&bind, &username, &password, &acid).await {
                Ok(r) => r,
                Err(e) => {
                    last_msg = e;
                    continue;
                }
            };
            match classify(&resp) {
                LoginOutcome::Success | LoginOutcome::AlreadyOnline => {
                    persist_acid(&state, &acid);
                    used_acid = acid.clone();
                    online = online_resp(&bind).await;
                    if online.is_some() {
                        break;
                    }
                    last_msg = "网关繁忙，请稍后重试".into();
                }
                LoginOutcome::AcidMismatch => {
                    last_msg = "正在尝试其它接入点…".into();
                    continue;
                }
                LoginOutcome::Fail(msg) => {
                    return Ok(fail(&detect, acid, msg));
                }
            }
        }
    }

    let resp = match online {
        Some(r) => r,
        None => return Ok(fail(&detect, used_acid, last_msg)),
    };

    // 3) 代拨确认 (仅联通代拨账号)。
    let user = if resp.user_name.is_empty() { username.clone() } else { resp.user_name.clone() };
    let dial = confirm_dial(&app, &bind, &resp, &username).await;
    let (success, message) = match &dial {
        None => (true, "已连接".to_string()),
        Some(d) if d.code == 0 => {
            (true, if d.message.is_empty() { "代拨成功".into() } else { d.message.clone() })
        }
        Some(d) if d.code == 100 => (true, "已认证，代拨中…请稍候".to_string()),
        Some(d) => (
            false,
            format!("代拨失败：{}", if d.message.is_empty() { "未知原因".into() } else { d.message.clone() }),
        ),
    };

    Ok(ConnectResult {
        success,
        online: true,
        kind: detect.kind.clone(),
        ssid: detect.ssid.clone(),
        user_name: Some(user),
        ac_id: used_acid,
        message,
        dial: dial.map(|d| d.message),
    })
}

fn persist_acid(state: &State<AppState>, acid: &str) {
    let mut cfg = state.config.lock().unwrap();
    if cfg.ac_id != acid {
        cfg.ac_id = acid.to_string();
        let _ = config::save(&cfg);
    }
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<ConnectResult, String> {
    let (username, acid, list) = {
        let cfg = state.config.lock().unwrap();
        (cfg.username.clone(), cfg.ac_id.clone(), cfg.ssid_allowlist.clone())
    };
    let detect = netdetect::detect(&list).await;
    let bind = detect.bind();
    let resp = srun::logout(&bind, &username, &acid).await?;
    let online = is_online(&bind).await.is_some();
    Ok(ConnectResult {
        success: !online,
        online,
        kind: detect.kind.clone(),
        ssid: detect.ssid.clone(),
        user_name: None,
        ac_id: acid,
        message: if online { translate(&resp) } else { "已断开连接".into() },
        dial: None,
    })
}

/// 供启动时自动登录调用 (非命令)。
pub async fn try_auto_login(app: &AppHandle) -> Option<ConnectResult> {
    let state = app.state::<AppState>();
    let auto = {
        let cfg = state.config.lock().unwrap();
        cfg.auto_login && !cfg.username.is_empty() && !cfg.password.is_empty()
    };
    if !auto {
        return None;
    }
    connect(app.clone(), state).await.ok()
}
