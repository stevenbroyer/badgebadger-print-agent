// Tauri entry point — boots the local HTTP listener on a worker
// thread, registers Tauri commands the React UI uses to read agent
// status, and wires the autostart plugin so the agent launches on
// login. The HTTP listener is the v1 ingress: web-app POSTs PDFs
// to localhost:9988/print and we shell them out to the OS print
// spooler. v2 will replace it with an outbound WebSocket so we
// work in deployments where the operator's machine isn't on the
// same network as the server.

mod auth;
mod http_server;
mod printer;
mod rate_limit;

use std::sync::Arc;

use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub listener_port: u16,
    pub listening: bool,
    pub default_printer: Option<String>,
    pub printers: Vec<String>,
    /// Whether SumatraPDF is detected on PATH or in a known install
    /// location. Required on Windows for reliable PDF printing —
    /// stock Win10/11 has Edge as the default PDF handler and Edge
    /// doesn't implement the `printto` shell verb. The UI surfaces
    /// an install link in the setup checklist when this is false.
    /// On macOS / Linux we don't need it (CUPS handles PDFs natively
    /// via `lp`), so it's reported true by default on those platforms.
    pub helper_installed: bool,
}

#[derive(Debug, Default)]
pub struct AgentState {
    pub listening: bool,
    pub listener_port: u16,
    /// Pairing token. Empty until `setup` runs `auth::load_or_create_token`.
    /// Surfaced to the React UI via the `get_pairing_token` Tauri command
    /// so the operator can copy it into the BadgeBadger web app to
    /// authorise this workstation.
    pub pairing_token: String,
}

pub type SharedState = Arc<Mutex<AgentState>>;

#[tauri::command]
async fn get_status(state: tauri::State<'_, SharedState>) -> Result<AgentStatus, String> {
    let s = state.lock().await;
    let printers = printer::list_printers().unwrap_or_default();
    let default_printer = printer::default_printer().ok().flatten();
    let helper_installed = printer::helper_installed();
    Ok(AgentStatus {
        listener_port: s.listener_port,
        listening: s.listening,
        default_printer,
        printers,
        helper_installed,
    })
}

#[tauri::command]
async fn get_pairing_token(state: tauri::State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    if s.pairing_token.is_empty() {
        return Err("pairing token not yet loaded".into());
    }
    Ok(s.pairing_token.clone())
}

#[tauri::command]
async fn test_print(
    app: tauri::AppHandle,
    printer_name: Option<String>,
) -> Result<String, String> {
    // Submit a hand-rolled minimal CR-80 PDF to the chosen printer
    // (or OS default if `printer_name` is None / empty). Lets the
    // operator confirm the agent's print dispatch works without
    // involving the web app. Emits the same `print` event the HTTP
    // path uses so test prints land in the in-app activity feed.
    let started_at = http_server_iso_now();
    let pdf = test_card_pdf();
    let printer = match printer_name {
        Some(p) if !p.trim().is_empty() => p,
        _ => match printer::default_printer() {
            Ok(Some(p)) => p,
            _ => {
                let msg = "No printer selected and no OS default configured. Pick one from the dropdown or set a system default.".to_string();
                emit_print(&app, &started_at, "(unset)", Some("Test print"), false, Some(&msg));
                return Err(msg);
            }
        },
    };
    let result = printer::print_pdf_bytes(&pdf, &printer).await;
    match &result {
        Ok(()) => {
            emit_print(&app, &started_at, &printer, Some("Test print"), true, None);
            Ok(format!("Sent test card to '{}'.", printer))
        }
        Err(e) => {
            let msg = e.to_string();
            emit_print(
                &app,
                &started_at,
                &printer,
                Some("Test print"),
                false,
                Some(&msg),
            );
            Err(format!("test print failed: {msg}"))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibPrintEvent<'a> {
    started_at: &'a str,
    printer: &'a str,
    job_name: Option<&'a str>,
    ok: bool,
    error: Option<&'a str>,
}

fn emit_print(
    app: &tauri::AppHandle,
    started_at: &str,
    printer: &str,
    job_name: Option<&str>,
    ok: bool,
    error: Option<&str>,
) {
    let _ = app.emit(
        "print",
        LibPrintEvent {
            started_at,
            printer,
            job_name,
            ok,
            error,
        },
    );
}

// Local copy of the http_server's ISO timestamp helper. Both call
// sites are tiny so duplicating beats reorganising the module
// hierarchy for one function.
fn http_server_iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let ms = now.subsec_millis();
    let days = secs / 86_400;
    let time_of_day = secs % 86_400;
    let h = time_of_day / 3_600;
    let m = (time_of_day / 60) % 60;
    let s = time_of_day % 60;
    let z = days as i64 + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_civil = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_civil = if m_civil <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y_civil, m_civil, d, h, m, s, ms
    )
}

// Minimal valid PDF rendered at CR-80 dimensions (242.65 × 153.07pt =
// 85.6 × 54mm landscape). A literal byte string instead of a bundled
// file so the agent has zero external assets to ship — the bytes
// below are a complete, valid PDF that opens in Preview / Adobe.
// Single page, single Helvetica string. Printers see it as a normal
// landscape CR-80 PDF and lay it out edge-to-edge.
fn test_card_pdf() -> Vec<u8> {
    const PDF: &[u8] = b"%PDF-1.4\n\
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n\
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 242.65 153.07]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n\
4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n\
5 0 obj<</Length 81>>stream\n\
BT /F1 22 Tf 30 95 Td (BadgeBadger) Tj 0 -28 Td /F1 12 Tf (Test print succeeded) Tj ET\n\
endstream\n\
endobj\n\
xref\n\
0 6\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000054 00000 n \n\
0000000099 00000 n \n\
0000000199 00000 n \n\
0000000252 00000 n \n\
trailer<</Size 6/Root 1 0 R>>\n\
startxref\n\
381\n\
%%EOF\n";
    PDF.to_vec()
}

// Reveal the main window from a tray-driven entry point (left-click on
// the tray icon, or "Open" in the tray menu). Idempotent — safe to call
// when the window is already visible.
fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn run() {
    // Verbose logs in dev (cargo run / tauri dev), warn-and-up in
    // release. Operators won't normally see these but they help when
    // we're debugging a customer's install.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    if cfg!(debug_assertions) {
                        "info,badgebadger_print_agent=debug".into()
                    } else {
                        "warn".into()
                    }
                }),
        )
        .try_init()
        .ok();

    let shared_state: SharedState = Arc::new(Mutex::new(AgentState::default()));
    let listener_state = shared_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            // macOS-only enum, ignored on Windows — autostart on
            // Windows uses the registry Run key under the hood.
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_pairing_token,
            test_print
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let token_state = listener_state.clone();
            // Load (or generate) the pairing token, then start the
            // HTTP listener with it. Both happen on the Tauri runtime
            // so they shut down cleanly with the app.
            tauri::async_runtime::spawn(async move {
                let token = match auth::load_or_create_token(&app_handle).await {
                    Ok(t) => t,
                    Err(err) => {
                        tracing::error!(?err, "could not load/create pairing token");
                        return;
                    }
                };
                {
                    let mut s = token_state.lock().await;
                    s.pairing_token = token.clone();
                }
                if let Err(err) = http_server::serve(
                    http_server::DEFAULT_PORT,
                    listener_state,
                    app_handle,
                    token,
                )
                .await
                {
                    tracing::error!(?err, "http listener stopped");
                }
            });

            // System tray. Keeps the agent reachable when the window is
            // hidden — left-click the tray icon reveals the window, the
            // tray menu offers Open / Quit. The X button on the window
            // hides instead of exits (see on_window_event below) so the
            // agent stays running per the footer's promise.
            let open_item =
                MenuItem::with_id(app, "open", "Open BadgeBadger", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("BadgeBadger Print Agent")
                .on_menu_event(|app_handle, event| match event.id().as_ref() {
                    "quit" => app_handle.exit(0),
                    "open" => show_main_window(app_handle),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it; the agent stays in the tray
            // and continues serving HTTP. Use the tray's "Quit" item to
            // actually exit. Matches the footer copy in the React UI.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
