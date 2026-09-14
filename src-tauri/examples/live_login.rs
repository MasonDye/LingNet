//! 活体端到端测试：直连校园网关走完整登录流程。
//!
//! 运行:
//!   cargo run --example live_login -- <学号> <密码>
//! 未提供参数时使用内置测试账号。

use lingnet_lib::{netdetect, srun};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (user, pass) = match (args.get(1), args.get(2)) {
        (Some(u), Some(p)) => (u.clone(), p.clone()),
        _ => {
            eprintln!("用法: cargo run --example live_login -- <学号> <密码>");
            return;
        }
    };

    println!("== 1) 网络识别 ==");
    let detect = netdetect::detect(&["HNJM-Student-X".to_string()]).await;
    println!("{detect:#?}");
    if !detect.allowed {
        println!("未识别到校园网，终止。");
        return;
    }
    let bind = detect.bind();

    println!("\n== 2) 在线状态 ==");
    match srun::rad_user_info(&bind).await {
        Ok(r) => println!("error={} user={}", r.error, r.user_name),
        Err(e) => println!("查询失败: {e}"),
    }

    println!("\n== 3) 登录 (ac_id 探测) ==");
    for acid in ["3", "1", "2"] {
        match srun::login_once(&bind, &user, &pass, acid).await {
            Ok(r) => {
                println!(
                    "ac_id={acid}: error={} suc={} msg={}",
                    r.error, r.suc_msg, r.error_msg
                );
                if r.error == "ok" || r.suc_msg == "login_ok" || r.error == "speed_limit_error" {
                    println!("  => 命中 ac_id={acid}");
                    break;
                }
            }
            Err(e) => println!("ac_id={acid}: 请求失败 {e}"),
        }
    }

    println!("\n== 4) 复查在线状态 ==");
    match srun::rad_user_info(&bind).await {
        Ok(r) => {
            println!("error={} user={} ip={} pppoe_dial={}", r.error, r.user_name, r.online_ip, r.pppoe_dial);
            if r.pppoe_dial == "1" {
                println!("\n== 5) 代拨查询 ==");
                match srun::dial_status(&bind, &user).await {
                    Ok(d) => println!("code={} message={}", d.code, d.message),
                    Err(e) => println!("代拨查询失败: {e}"),
                }
            }
        }
        Err(e) => println!("查询失败: {e}"),
    }
}
