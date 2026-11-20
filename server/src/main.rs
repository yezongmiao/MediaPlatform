use std::path::PathBuf;
use std::time::Duration;

use mediadesk_core::api::{self, AppState};
use tokio::net::TcpListener;

fn print_help() {
    println!(
        "媒材工坊 MediaDesk Server\n\n\
         Usage: mediadesk-server [OPTIONS]\n\n\
         Options:\n  \
         --workspace <PATH>   使用指定工作区\n  \
         --port <PORT>        监听端口（默认 4317，环境变量 MEDIADESK_PORT）\n  \
         --host <HOST>        监听地址（默认 127.0.0.1）\n  \
         --open               启动后自动打开浏览器\n  \
         --no-open            不自动打开浏览器\n  \
         -h, --help           显示帮助\n  \
         -V, --version        显示版本\n"
    );
}

fn open_browser(url: &str) {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(err) = result {
        eprintln!("无法自动打开浏览器，请手动访问 {url}：{err}");
    }
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let mut port: u16 = std::env::var("MEDIADESK_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4317);
    let mut host = "127.0.0.1".to_string();
    let mut workspace: Option<PathBuf> = None;
    let mut open = std::env::var("MEDIADESK_OPEN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut explicit_open = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workspace" | "-w" => {
                if let Some(value) = args.next() {
                    workspace = Some(PathBuf::from(value));
                } else {
                    eprintln!("--workspace 需要一个路径参数");
                    std::process::exit(2);
                }
            }
            "--port" | "-p" => {
                if let Some(value) = args.next() {
                    match value.parse() {
                        Ok(p) => port = p,
                        Err(_) => {
                            eprintln!("无效端口：{value}");
                            std::process::exit(2);
                        }
                    }
                } else {
                    eprintln!("--port 需要一个数字参数");
                    std::process::exit(2);
                }
            }
            "--host" => {
                if let Some(value) = args.next() {
                    host = value;
                } else {
                    eprintln!("--host 需要一个地址参数");
                    std::process::exit(2);
                }
            }
            "--open" => {
                open = true;
                explicit_open = true;
            }
            "--no-open" => {
                open = false;
                explicit_open = true;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" => {
                println!("mediadesk-server {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            other => {
                eprintln!("未知参数：{other}");
                print_help();
                std::process::exit(2);
            }
        }
    }
    let _ = explicit_open;

    let state = match AppState::bootstrap(workspace, None).await {
        Ok(state) => state,
        Err(err) => {
            eprintln!("启动失败：{}", err.message);
            std::process::exit(1);
        }
    };
    let listener = match TcpListener::bind((host.as_str(), port)).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("无法监听 {host}:{port}：{err}");
            std::process::exit(1);
        }
    };
    let addr = listener.local_addr().expect("local addr");
    let url = format!("http://{addr}");
    println!("媒材工坊 MediaDesk v{}", env!("CARGO_PKG_VERSION"));
    println!("工作区：{}", state.workspace().display());
    println!("服务已启动：{url}");
    if open {
        let url = url.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(350)).await;
            open_browser(&url);
        });
    }

    let router = api::router(state);
    if let Err(err) = axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            println!("\n正在关闭…");
        })
        .await
    {
        eprintln!("服务异常退出：{err}");
        std::process::exit(1);
    }
}
