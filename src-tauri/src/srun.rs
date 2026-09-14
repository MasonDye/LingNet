//! 深澜 SRun 校园网认证协议实现。
//!
//! 目标网关: http://10.16.1.114  (SRunCGIAuthIntfSvr V1.18 B20240604)
//!
//! 认证流程 (与门户 Portal.js 逐字节核对一致):
//!   1. GET /cgi-bin/get_challenge  -> 取 token(challenge) 和 online_ip
//!   2. hmd5   = HMAC-MD5(password, token)
//!      info   = "{SRBX1}" + srun_base64( xxtea(json(userinfo), token) )
//!      chksum = SHA1( token+user token+hmd5 token+acid token+ip token+n token+type token+info )
//!   3. GET /cgi-bin/srun_portal   -> action=login, 携带上面所有参数
//!   4. GET /cgi-bin/rad_user_info -> 校验是否在线
//!
//! 网卡绑定: Linux 走 SO_BINDTODEVICE(免 root)，其它平台绑定源 IP，
//!          以绕开本机可能存在的策略路由 / VPN(TUN) 劫持。

use std::net::IpAddr;
use std::time::Duration;

use hmac::{Hmac, Mac};
use md5::Md5;
use serde::Serialize;
use sha1::{Digest, Sha1};

#[allow(dead_code)]
pub const PORTAL_HOST: &str = "10.16.1.114";
const BASE: &str = "http://10.16.1.114";
const UA: &str = "Mozilla/5.0 (LingNet; SRun client) AppleWebKit/537.36";

/// 深澜自定义 base64 码表 (顺序被打乱)。
const ALPHA: &[u8] = b"LVoJPiCN2R8G90yg+hmFHuacZ1OWMnrsSTXkYpUq/3dlbfKwv6xztjI7DeBE45QA";
const PAD: u8 = b'=';

type HmacMd5 = Hmac<Md5>;

/// 网卡绑定目标。
#[derive(Clone, Debug, Default)]
pub struct Bind {
    /// Linux: 网卡名 (SO_BINDTODEVICE)。
    pub interface: Option<String>,
    /// 其它平台或回退: 绑定源 IP。
    pub source_ip: Option<IpAddr>,
}

impl Bind {
    pub fn interface(name: impl Into<String>) -> Self {
        Bind { interface: Some(name.into()), source_ip: None }
    }
}

/// get_challenge / srun_portal / rad_user_info 的通用响应片段。
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // ecode / raw 保留供调试与后续扩展
pub struct SrunResp {
    pub error: String,
    pub error_msg: String,
    pub res: String,
    pub suc_msg: String,
    pub ecode: String,
    pub online_ip: String,
    pub challenge: String,
    pub user_name: String,
    /// "1" 表示该账号为运营商代拨(联通PPPoE)类型，登录后需确认代拨结果。
    pub pppoe_dial: String,
    pub raw: serde_json::Value,
}

impl SrunResp {
    fn from_value(v: serde_json::Value) -> Self {
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        // ecode 有时是数字有时是字符串。
        let ecode = match v.get("ecode") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        SrunResp {
            error: s("error"),
            error_msg: s("error_msg"),
            res: s("res"),
            suc_msg: s("suc_msg"),
            ecode,
            online_ip: s("online_ip"),
            challenge: s("challenge"),
            user_name: s("user_name"),
            pppoe_dial: s("pppoe_dial"),
            raw: v,
        }
    }
}

/// 代拨(联通PPPoE)查询结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct DialResult {
    pub code: i64,
    pub message: String,
}

/// 查询代拨结果：GET /v1/srun_portal_diallog?username=<纯学号>（不带 domain）。
/// code=0 代拨成功；code=100 拨号中；其它为失败。返回为普通 JSON（非 jsonp）。
pub async fn dial_status(bind: &Bind, username: &str) -> Result<DialResult, String> {
    let client = build_client(bind).map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(format!("{BASE}/v1/srun_portal_diallog"))
        .query(&[("username", username)])
        .header("Referer", format!("{BASE}/"))
        .send()
        .await
        .map_err(|e| format!("查询代拨失败: {e}"))?;
    let text = resp.text().await.map_err(|e| format!("读取代拨响应失败: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("解析代拨响应失败: {e} — {}", truncate(&text, 160)))?;
    let code = v.get("code").and_then(|x| x.as_i64()).unwrap_or(-1);
    let message = v.get("message").and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok(DialResult { code, message })
}

// ---------------------------------------------------------------------------
// 加密原语
// ---------------------------------------------------------------------------

/// 深澜自定义 base64 (与门户 $.base64.setAlpha 后的 encode 一致)。
fn srun_base64(raw: &[u8]) -> String {
    let mut out = Vec::with_capacity(raw.len() / 3 * 4 + 4);
    let n = raw.len();
    let imax = n - n % 3;
    let mut i = 0;
    while i < imax {
        let b10 = (raw[i] as u32) << 16 | (raw[i + 1] as u32) << 8 | (raw[i + 2] as u32);
        out.push(ALPHA[(b10 >> 18) as usize & 63]);
        out.push(ALPHA[(b10 >> 12) as usize & 63]);
        out.push(ALPHA[(b10 >> 6) as usize & 63]);
        out.push(ALPHA[b10 as usize & 63]);
        i += 3;
    }
    match n - imax {
        1 => {
            let b10 = (raw[i] as u32) << 16;
            out.push(ALPHA[(b10 >> 18) as usize & 63]);
            out.push(ALPHA[(b10 >> 12) as usize & 63]);
            out.push(PAD);
            out.push(PAD);
        }
        2 => {
            let b10 = (raw[i] as u32) << 16 | (raw[i + 1] as u32) << 8;
            out.push(ALPHA[(b10 >> 18) as usize & 63]);
            out.push(ALPHA[(b10 >> 12) as usize & 63]);
            out.push(ALPHA[(b10 >> 6) as usize & 63]);
            out.push(PAD);
        }
        _ => {}
    }
    // 全部落在 ASCII 范围。
    String::from_utf8(out).unwrap()
}

/// 把字节按小端打包为 u32 数组，可选在末尾追加原始长度。
fn to_u32(a: &[u8], append_len: bool) -> Vec<u32> {
    let c = a.len();
    let mut v: Vec<u32> = Vec::new();
    let mut i = 0;
    while i < c {
        let mut x = a[i] as u32;
        if i + 1 < c { x |= (a[i + 1] as u32) << 8; }
        if i + 2 < c { x |= (a[i + 2] as u32) << 16; }
        if i + 3 < c { x |= (a[i + 3] as u32) << 24; }
        v.push(x);
        i += 4;
    }
    if append_len {
        v.push(c as u32);
    }
    v
}

/// u32 数组按小端还原为字节。
fn from_u32(v: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for &x in v {
        out.push((x & 0xff) as u8);
        out.push(((x >> 8) & 0xff) as u8);
        out.push(((x >> 16) & 0xff) as u8);
        out.push(((x >> 24) & 0xff) as u8);
    }
    out
}

/// 深澜改造版 XXTEA 加密 (Portal.js `_encodeUserInfo` 内 `encode`)。
fn xxtea_encode(s: &str, key: &str) -> Vec<u8> {
    if s.is_empty() {
        return Vec::new();
    }
    let mut v = to_u32(s.as_bytes(), true);
    let mut k = to_u32(key.as_bytes(), false);
    while k.len() < 4 {
        k.push(0);
    }
    let n = v.len() - 1;
    let mut z = v[n];
    let c: u32 = 0x9E37_79B9; // 0x86014019 | 0x183639A0
    let mut q = 6 + 52 / (n as u32 + 1);
    let mut d: u32 = 0;
    while q > 0 {
        d = d.wrapping_add(c);
        let e = (d >> 2) & 3;
        for p in 0..n {
            let y = v[p + 1];
            let mut m = (z >> 5) ^ (y << 2);
            m = m.wrapping_add(((y >> 3) ^ (z << 4)) ^ (d ^ y));
            m = m.wrapping_add(k[((p & 3) as u32 ^ e) as usize] ^ z);
            v[p] = v[p].wrapping_add(m);
            z = v[p];
        }
        let p = n;
        let y = v[0];
        let mut m = (z >> 5) ^ (y << 2);
        m = m.wrapping_add(((y >> 3) ^ (z << 4)) ^ (d ^ y));
        m = m.wrapping_add(k[((p & 3) as u32 ^ e) as usize] ^ z);
        v[n] = v[n].wrapping_add(m);
        z = v[n];
        q -= 1;
    }
    from_u32(&v)
}

fn hmac_md5_hex(password: &str, token: &str) -> String {
    let mut mac = HmacMd5::new_from_slice(token.as_bytes()).expect("hmac accepts any key length");
    mac.update(password.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn sha1_hex(s: &str) -> String {
    let mut h = Sha1::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// 门户信息对象。字段顺序必须与 Portal.js 保持一致 (serde 按声明顺序序列化)。
#[derive(Serialize)]
struct UserInfo<'a> {
    username: &'a str,
    password: &'a str,
    ip: &'a str,
    acid: &'a str,
    enc_ver: &'a str,
}

fn build_info(username: &str, password: &str, ip: &str, acid: &str, token: &str) -> String {
    let info = UserInfo { username, password, ip, acid, enc_ver: "srun_bx1" };
    let json = serde_json::to_string(&info).expect("serialize userinfo");
    let encrypted = xxtea_encode(&json, token);
    format!("{{SRBX1}}{}", srun_base64(&encrypted))
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

fn build_client(bind: &Bind) -> reqwest::Result<reqwest::Client> {
    build_client_timeout(bind, 10)
}

fn build_client_timeout(bind: &Bind, timeout_secs: u64) -> reqwest::Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();

    // 优先绑定网卡 (Linux/Android)，否则绑定源 IP。
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "fuchsia"))]
    if let Some(name) = &bind.interface {
        b = b.interface(name);
    }
    if bind.interface.is_none() || cfg!(not(any(target_os = "linux", target_os = "android", target_os = "fuchsia"))) {
        if let Some(ip) = bind.source_ip {
            b = b.local_address(ip);
        }
    }
    b.build()
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// 解析 JSONP: 取第一个 '(' 与最后一个 ')' 之间的 JSON。
fn parse_jsonp(body: &str) -> Result<serde_json::Value, String> {
    let start = body.find('(');
    let end = body.rfind(')');
    let slice = match (start, end) {
        (Some(s), Some(e)) if e > s => &body[s + 1..e],
        _ => body.trim(),
    };
    serde_json::from_str(slice).map_err(|e| format!("解析响应失败: {e} — 原文: {}", truncate(body, 200)))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}

const CB: &str = "jQuery112409";

async fn cgi_get(
    client: &reqwest::Client,
    path: &str,
    params: &[(&str, String)],
) -> Result<serde_json::Value, String> {
    let url = format!("{BASE}{path}");
    let resp = client
        .get(&url)
        .query(params)
        .header("Referer", format!("{BASE}/"))
        .send()
        .await
        .map_err(|e| format!("请求 {path} 失败: {e}"))?;
    let text = resp.text().await.map_err(|e| format!("读取 {path} 响应失败: {e}"))?;
    parse_jsonp(&text)
}

/// 探测门户是否存在于该绑定网卡上 (用于"有线特征"判定)。
/// 返回 true 表示访问 http://10.16.1.114/ 命中了 SRun 门户 (SRunFlag 头)。
pub async fn probe_portal(bind: &Bind) -> bool {
    let client = match build_client_timeout(bind, 4) {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client.get(format!("{BASE}/")).send().await {
        Ok(resp) => {
            // 门户会带 SRunFlag 头，或 302 到 index_*.html。
            let has_flag = resp.headers().get("SRunFlag").is_some()
                || resp.headers().get("srunflag").is_some();
            let is_redirect = resp.status().is_redirection();
            has_flag || is_redirect
        }
        Err(_) => false,
    }
}

/// 查询当前在线状态。
pub async fn rad_user_info(bind: &Bind) -> Result<SrunResp, String> {
    let client = build_client(bind).map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let v = cgi_get(
        &client,
        "/cgi-bin/rad_user_info",
        &[("callback", CB.into()), ("_", now_ms().to_string())],
    )
    .await?;
    Ok(SrunResp::from_value(v))
}

async fn get_challenge(client: &reqwest::Client, username: &str, ip: &str) -> Result<SrunResp, String> {
    let v = cgi_get(
        client,
        "/cgi-bin/get_challenge",
        &[
            ("callback", CB.into()),
            ("username", username.into()),
            ("ip", ip.into()),
            ("_", now_ms().to_string()),
        ],
    )
    .await?;
    Ok(SrunResp::from_value(v))
}

/// 用指定 ac_id 执行一次登录。返回原始响应，由上层判定成功与否。
pub async fn login_once(
    bind: &Bind,
    username: &str,
    password: &str,
    acid: &str,
) -> Result<SrunResp, String> {
    let client = build_client(bind).map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // 1) 取 challenge。
    let ch = get_challenge(&client, username, "").await?;
    if ch.res != "ok" || ch.challenge.is_empty() {
        return Err(format!("获取 challenge 失败: {} {}", ch.error, ch.error_msg));
    }
    let token = ch.challenge;
    let ip = if ch.online_ip.is_empty() { String::new() } else { ch.online_ip.clone() };

    // 2) 计算参数。
    let hmd5 = hmac_md5_hex(password, &token);
    let info = build_info(username, password, &ip, acid, &token);
    let n = "200";
    let typ = "1";
    let chk = format!(
        "{t}{u}{t}{h}{t}{a}{t}{ip}{t}{n}{t}{ty}{t}{i}",
        t = token, u = username, h = hmd5, a = acid, ip = ip, n = n, ty = typ, i = info
    );
    let chksum = sha1_hex(&chk);

    // 3) 提交登录。
    let params = vec![
        ("callback", CB.to_string()),
        ("action", "login".to_string()),
        ("username", username.to_string()),
        ("password", format!("{{MD5}}{hmd5}")),
        ("os", os_name().to_string()),
        ("name", os_platform().to_string()),
        ("nas_ip", String::new()),
        ("double_stack", "0".to_string()),
        ("chksum", chksum),
        ("info", info),
        ("ac_id", acid.to_string()),
        ("ip", ip),
        ("n", n.to_string()),
        ("type", typ.to_string()),
        ("captchaId", String::new()),
        ("captchaVal", String::new()),
        ("_", now_ms().to_string()),
    ];
    let v = cgi_get(&client, "/cgi-bin/srun_portal", &params).await?;
    Ok(SrunResp::from_value(v))
}

/// 注销当前会话。
pub async fn logout(bind: &Bind, username: &str, acid: &str) -> Result<SrunResp, String> {
    let client = build_client(bind).map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    // 先查在线 IP。
    let info = rad_user_info(bind).await.ok();
    let ip = info.map(|r| r.online_ip).unwrap_or_default();
    let params = vec![
        ("callback", CB.to_string()),
        ("action", "logout".to_string()),
        ("username", username.to_string()),
        ("ip", ip),
        ("ac_id", acid.to_string()),
        ("_", now_ms().to_string()),
    ];
    let v = cgi_get(&client, "/cgi-bin/srun_portal", &params).await?;
    Ok(SrunResp::from_value(v))
}

fn os_name() -> &'static str {
    if cfg!(target_os = "windows") { "Windows" } else if cfg!(target_os = "macos") { "macOS" } else { "Linux" }
}
fn os_platform() -> &'static str {
    if cfg!(target_os = "windows") { "Windows" } else if cfg!(target_os = "macos") { "macOS" } else { "Linux" }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 固定向量：token 为一次性 challenge (非凭据)，账号/密码为占位测试值。
    // 期望值由与门户 Portal.js 逐字节核对过的参考实现生成，用于锁定算法不回退。
    const TOKEN: &str = "8992d71deadf658cd835a9a4bce0ee8ad0dfb2877b3a01b1d7179e1db70eeba9";
    const T_USER: &str = "20230000001";
    const T_PASS: &str = "Demo@2024";

    #[test]
    fn info_matches_reference() {
        let got = build_info(T_USER, T_PASS, "10.0.0.1", "1", TOKEN);
        let expected = "{SRBX1}KwfjpJuDUEbfEnZLqmzCxBWSyvtTRMZ4GcNDPLqgAQ4GDYqJ5fNjI0CfzKMblWX40sR9LQpHIQEcH3cE9g+NcgNJ5A+MyF1WU/6yFYUE/xkFb9fOARh/k/lgyxOptuHx7CNUGsBa+h4=";
        assert_eq!(got, expected);
    }

    #[test]
    fn hmac_md5_known() {
        let got = hmac_md5_hex(T_PASS, TOKEN);
        assert_eq!(got, "2927672da9c9ff19aa6ef111030efaa7");
    }

    #[test]
    fn base64_roundtrip_shape() {
        // 门户 base64 使用自定义码表与 '=' 填充。
        assert!(srun_base64(b"abc").is_ascii());
    }
}
