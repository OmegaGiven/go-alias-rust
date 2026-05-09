#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ogdevdesk_service::server::{
    DesktopToolCloser, DesktopToolOpener, ServerConfig, run_server_blocking,
};
use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct DesktopServer {
    base_url: String,
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_tool_window,
            close_current_window
        ])
        .setup(|app| {
            let port = pick_local_port().unwrap_or(17654);
            configure_desktop_environment();
            let base_url = format!("http://127.0.0.1:{port}");
            app.manage(DesktopServer {
                base_url: base_url.clone(),
            });

            let static_dir = resolve_static_dir(app);
            let app_handle = app.handle().clone();
            let opener_base_url = base_url.clone();
            let desktop_tool_opener: DesktopToolOpener = std::sync::Arc::new(move |tool| {
                open_desktop_tool_window(&app_handle, &opener_base_url, &tool)
            });
            let close_app_handle = app.handle().clone();
            let desktop_tool_closer: DesktopToolCloser =
                std::sync::Arc::new(move |tool| close_desktop_tool_window(&close_app_handle, &tool));
            std::thread::spawn(move || {
                if let Err(err) = run_server_blocking(
                    ServerConfig::local(port, static_dir)
                        .with_desktop_tool_handlers(desktop_tool_opener, desktop_tool_closer),
                ) {
                    eprintln!("OGdevDesk desktop server failed: {err}");
                }
            });

            let url = format!("desktop_shell.html#port={port}&path=/requests");
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App(url.into()),
            )
            .title("OGdevDesk")
            .inner_size(1280.0, 860.0)
            .min_inner_size(900.0, 620.0)
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run OGdevDesk desktop app");
}

#[tauri::command]
fn open_tool_window(
    app: tauri::AppHandle,
    server: tauri::State<'_, DesktopServer>,
    tool: String,
) -> Result<(), String> {
    open_desktop_tool_window(&app, &server.base_url, &tool)
}

fn open_desktop_tool_window(
    app: &tauri::AppHandle,
    base_url: &str,
    tool: &str,
) -> Result<(), String> {
    let spec = desktop_tool_spec(tool).ok_or_else(|| format!("Unknown desktop tool: {tool}"))?;
    if let Some(window) = app.get_webview_window(spec.label) {
        window.set_always_on_top(true).map_err(|err| err.to_string())?;
        window.show().map_err(|err| err.to_string())?;
        window.set_focus().map_err(|err| err.to_string())?;
        return Ok(());
    }

    let port = base_url
        .rsplit_once(':')
        .map(|(_, port)| port)
        .ok_or_else(|| format!("Invalid desktop server URL: {base_url}"))?;
    let tool_url = format!("desktop_tool.html#port={port}&tool={}", spec.path);
    let window = WebviewWindowBuilder::new(
        app,
        spec.label,
        WebviewUrl::App(tool_url.into()),
    )
    .title(spec.title)
    .inner_size(spec.width, spec.height)
    .min_inner_size(spec.min_width, spec.min_height)
    .resizable(true)
    .always_on_top(true)
    .visible(true)
    .build()
    .map_err(|err| err.to_string())?;
    window.center().map_err(|err| err.to_string())?;
    window.set_always_on_top(true).map_err(|err| err.to_string())?;
    window.show().map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
fn close_current_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|err| err.to_string())
}

fn close_desktop_tool_window(app: &tauri::AppHandle, tool: &str) -> Result<(), String> {
    let spec = desktop_tool_spec(tool).ok_or_else(|| format!("Unknown desktop tool: {tool}"))?;
    let Some(window) = app.get_webview_window(spec.label) else {
        return Ok(());
    };
    window.close().map_err(|err| err.to_string())
}

struct DesktopToolSpec {
    label: &'static str,
    path: &'static str,
    title: &'static str,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
}

fn desktop_tool_spec(tool: &str) -> Option<DesktopToolSpec> {
    match tool {
        "appearance" => Some(DesktopToolSpec {
            label: "tool-appearance",
            path: "appearance",
            title: "Appearance",
            width: 500.0,
            height: 720.0,
            min_width: 420.0,
            min_height: 520.0,
        }),
        "calculator" => Some(DesktopToolSpec {
            label: "tool-calculator",
            path: "calculator",
            title: "Calculator",
            width: 390.0,
            height: 620.0,
            min_width: 340.0,
            min_height: 500.0,
        }),
        "jwt" => Some(DesktopToolSpec {
            label: "tool-jwt",
            path: "jwt",
            title: "JWT Decoder",
            width: 520.0,
            height: 720.0,
            min_width: 420.0,
            min_height: 520.0,
        }),
        "scratchpad" => Some(DesktopToolSpec {
            label: "tool-scratchpad",
            path: "scratchpad",
            title: "Scratch Pad",
            width: 520.0,
            height: 460.0,
            min_width: 360.0,
            min_height: 320.0,
        }),
        "ai" => Some(DesktopToolSpec {
            label: "tool-ai",
            path: "ai",
            title: "AI Assistant",
            width: 580.0,
            height: 760.0,
            min_width: 460.0,
            min_height: 560.0,
        }),
        "documentation" => Some(DesktopToolSpec {
            label: "tool-documentation",
            path: "documentation",
            title: "Documentation",
            width: 620.0,
            height: 720.0,
            min_width: 460.0,
            min_height: 480.0,
        }),
        _ => None,
    }
}

fn pick_local_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|listener| listener.local_addr().ok())
        .map(|addr| addr.port())
}

fn configure_desktop_environment() {
    let db_path = std::env::var("OGDEVDESK_DB_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(default_desktop_db_path);
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
        let _ = std::env::set_current_dir(parent);
    }

    // Desktop mode is configured before the embedded Actix server starts.
    unsafe {
        std::env::set_var("OGDEVDESK_MODE", "desktop");
        std::env::set_var("OGDEVDESK_DB_PATH", db_path);
    }
}

fn default_desktop_db_path() -> PathBuf {
    let app_dir = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("OGdevDesk")
    } else if cfg!(target_os = "macos") {
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("OGdevDesk")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local").join("share"))
            .join("ogdevdesk")
    };

    app_dir.join("ogdevdesk.db")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_static_dir(app: &tauri::App) -> String {
    if let Ok(static_dir) = std::env::var("OGDEVDESK_STATIC_DIR") {
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
