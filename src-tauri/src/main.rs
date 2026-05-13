#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ogdevdesk_service::server::{
    DesktopToolCloser, DesktopToolOpener, ServerConfig, run_server_blocking,
};
use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_updater::UpdaterExt;

const UPDATER_PUBKEY_PLACEHOLDER: &str = "OGDEVDESK_UPDATER_PUBLIC_KEY_NOT_CONFIGURED";

struct DesktopServer {
    base_url: String,
}

#[derive(serde::Serialize)]
struct DesktopUpdateInfo {
    available: bool,
    current_version: String,
    version: Option<String>,
    date: Option<String>,
    body: Option<String>,
    target: Option<String>,
}

#[derive(serde::Serialize)]
struct DesktopUpdateInstallResult {
    installed: bool,
    message: String,
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_tool_window,
            close_current_window,
            save_text_file,
            check_for_update,
            install_update,
            open_url_in_browser
        ])
        .setup(|app| {
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let server_listener = reserve_desktop_server_listener().or_else(|err| {
                eprintln!("Failed to reserve a desktop server port: {err}");
                TcpListener::bind("127.0.0.1:17654")
            })?;
            let port = server_listener.local_addr()?.port();
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
                        .with_listener(server_listener)
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

#[tauri::command]
fn save_text_file(suggested_filename: String, contents: String) -> Result<Option<String>, String> {
    let filename = safe_export_filename(&suggested_filename);
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save Export")
        .set_file_name(&filename);

    if let Some(downloads_dir) = download_dir() {
        dialog = dialog.set_directory(downloads_dir);
    }

    let Some(path) = dialog.save_file() else {
        return Ok(None);
    };

    fs::write(&path, contents).map_err(|err| err.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn open_url_in_browser(url: String) -> Result<(), String> {
    let trimmed_url = url.trim();
    if !(trimmed_url.starts_with("http://") || trimmed_url.starts_with("https://")) {
        return Err("Only http and https URLs can be opened from OGdevDesk.".to_string());
    }

    open_with_system_browser(trimmed_url)
}

#[cfg(target_os = "macos")]
fn open_with_system_browser(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_with_system_browser(url: &str) -> Result<(), String> {
    Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_with_system_browser(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
async fn check_for_update(app: tauri::AppHandle) -> Result<DesktopUpdateInfo, String> {
    ensure_desktop_updater_configured(&app)?;

    let update = app
        .updater()
        .map_err(updater_error_message)?
        .check()
        .await
        .map_err(updater_error_message)?;
    let current_version = app.package_info().version.to_string();

    Ok(match update {
        Some(update) => DesktopUpdateInfo {
            available: true,
            current_version: update.current_version,
            version: Some(update.version),
            date: update.date.map(|date| date.to_string()),
            body: update.body,
            target: Some(update.target),
        },
        None => DesktopUpdateInfo {
            available: false,
            current_version,
            version: None,
            date: None,
            body: None,
            target: None,
        },
    })
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<DesktopUpdateInstallResult, String> {
    ensure_desktop_updater_configured(&app)?;

    let update = app
        .updater()
        .map_err(updater_error_message)?
        .check()
        .await
        .map_err(updater_error_message)?;

    let Some(update) = update else {
        return Ok(DesktopUpdateInstallResult {
            installed: false,
            message: "OGdevDesk is already up to date.".to_string(),
        });
    };

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(updater_error_message)?;

    app.restart();
}

fn updater_error_message(error: impl std::fmt::Display) -> String {
    let message = error.to_string();
    message
}

fn ensure_desktop_updater_configured(app: &tauri::AppHandle) -> Result<(), String> {
    let updater_config = app.config().plugins.0.get("updater");
    let pubkey = updater_config
        .and_then(|config| config.get("pubkey"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();

    if pubkey == UPDATER_PUBKEY_PLACEHOLDER || pubkey.trim().is_empty() {
        return Err("Desktop updates are not fully configured yet. Generate a Tauri updater signing key, put the public key in src-tauri/tauri.conf.json, and publish signed updater artifacts.".to_string());
    }

    Ok(())
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

fn reserve_desktop_server_listener() -> std::io::Result<TcpListener> {
    TcpListener::bind("127.0.0.1:80").or_else(|err| {
        eprintln!("Desktop server could not bind 127.0.0.1:80; falling back to an available local port: {err}");
        TcpListener::bind("127.0.0.1:0")
    })
}

fn safe_export_filename(filename: &str) -> String {
    let cleaned = filename
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_start_matches('.')
        .to_string();

    if cleaned.is_empty() {
        "ogdevdesk-export.csv".to_string()
    } else {
        cleaned
    }
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
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn download_dir() -> Option<PathBuf> {
    let path = home_dir().join("Downloads");
    path.exists().then_some(path)
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

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let beside_exe = exe_dir.join("static");
            if beside_exe.exists() {
                return beside_exe.to_string_lossy().to_string();
            }
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
