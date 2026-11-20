#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use mediadesk_core::api::AppState;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = match AppState::bootstrap(None, None).await {
                    Ok(state) => state,
                    Err(err) => {
                        eprintln!("MediaDesk bootstrap failed: {err}");
                        return;
                    }
                };
                let listener = match TcpListener::bind("127.0.0.1:0").await {
                    Ok(listener) => listener,
                    Err(err) => {
                        eprintln!("MediaDesk could not bind local server: {err}");
                        return;
                    }
                };
                let port = match listener.local_addr() {
                    Ok(addr) => addr.port(),
                    Err(err) => {
                        eprintln!("MediaDesk could not read local address: {err}");
                        return;
                    }
                };
                let workspace = state.workspace();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = mediadesk_core::serve(listener, state).await {
                        eprintln!("MediaDesk server stopped: {err}");
                    }
                });

                // Wait until the local server accepts connections before pointing
                // the webview at it, otherwise the first paint is a connection error.
                for _ in 0..100 {
                    if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }

                let url = format!("http://127.0.0.1:{port}");
                println!("MediaDesk workspace: {}", workspace.display());
                println!("MediaDesk server: {url}");

                let window_handle = handle.clone();
                let window_url = url.clone();
                if let Err(err) = handle.run_on_main_thread(move || match window_url.parse() {
                    Ok(url) => {
                        let result = WebviewWindowBuilder::new(
                            &window_handle,
                            "main",
                            WebviewUrl::External(url),
                        )
                        .title("媒材工坊")
                        .inner_size(1320.0, 900.0)
                        .min_inner_size(960.0, 640.0)
                        .center()
                        .build();
                        if let Err(err) = result {
                            eprintln!("MediaDesk window creation failed: {err}");
                        }
                    }
                    Err(err) => eprintln!("MediaDesk invalid local URL: {err}"),
                }) {
                    eprintln!("MediaDesk could not schedule window creation: {err}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running MediaDesk");
}
