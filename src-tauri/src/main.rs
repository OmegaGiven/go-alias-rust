#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use go_service::server::{ServerConfig, run_server_blocking};
use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let port = pick_local_port().unwrap_or(17654);
            configure_desktop_environment();

            let static_dir = resolve_static_dir(app);
            std::thread::spawn(move || {
                if let Err(err) = run_server_blocking(ServerConfig::local(port, static_dir)) {
                    eprintln!("Go Alias desktop server failed: {err}");
                }
            });

            let url = format!("http://127.0.0.1:{port}/requests");
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse().expect("desktop URL should be valid")),
            )
            .title("Go Alias")
            .inner_size(1280.0, 860.0)
            .min_inner_size(900.0, 620.0)
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Go Alias desktop app");
}

fn pick_local_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|listener| listener.local_addr().ok())
        .map(|addr| addr.port())
}

fn configure_desktop_environment() {
    let db_path = std::env::var("GO_ALIAS_DB_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(default_desktop_db_path);
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
        let _ = std::env::set_current_dir(parent);
    }

    // Desktop mode is configured before the embedded Actix server starts.
    unsafe {
        std::env::set_var("GO_ALIAS_MODE", "desktop");
        std::env::set_var("GO_ALIAS_DB_PATH", db_path);
    }
}

fn default_desktop_db_path() -> PathBuf {
    let app_dir = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Go Alias")
    } else if cfg!(target_os = "macos") {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("Go Alias")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local").join("share"))
            .join("go-alias")
    };

    app_dir.join("go_service.db")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_static_dir(app: &tauri::App) -> String {
    if let Ok(static_dir) = std::env::var("GO_ALIAS_STATIC_DIR") {
        return static_dir;
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled_static = resource_dir.join("static");
        if bundled_static.exists() {
            return bundled_static.to_string_lossy().to_string();
        }
    }

    let from_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("static");
    if from_manifest.exists() {
        return from_manifest.to_string_lossy().to_string();
    }

    "static".to_string()
}
