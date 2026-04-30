// Local HTTP listener. v1 ingress — accepts PDFs over HTTP from the
// web app on the same machine and dispatches to the printer. The
// listener binds to 127.0.0.1 only so it's never exposed to the LAN.
//
// API:
//
//   GET  /health                  -> { ok, version, listening, hostname,
//                                       platform, helperInstalled,
//                                       defaultPrinter, printers, agentId }
//                                    The web app calls this on page load to
//                                    decide whether to route prints through
//                                    the agent and to render the printer
//                                    picker / status UI.
//
//   POST /print                   -> body = PDF bytes (Content-Type: application/pdf)
//                                    query = ?printer=<name>   (optional, defaults to OS default)
//                                    query = ?job_name=<name>  (optional, shows in spooler UI)
//                                    response = { ok, printer, job_name } on success
//
// CORS is wide-open for localhost/127.0.0.1 origins so the web app
// (running anywhere) can POST to the agent. In v2 we replace this
// with an authenticated WebSocket so cross-origin / cross-host setups
// also work.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;

use crate::printer;

pub const DEFAULT_PORT: u16 = 9988;

// 25 MB cap per print job. Realistic single-card PDFs are <1MB; even
// a 100-page bulk batch shouldn't exceed 5MB. The limit is there to
// keep an erroneous client from filling memory with a hostile body.
const MAX_BODY_BYTES: usize = 25 * 1024 * 1024;

pub async fn serve(
    port: u16,
    state: Arc<Mutex<crate::AgentState>>,
    app: tauri::AppHandle,
) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = Router::new()
        .route("/health", get(health))
        .route("/print", post(print_handler))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(AgentCtx { app });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    {
        let mut s = state.lock().await;
        s.listening = true;
        s.listener_port = port;
    }
    tracing::info!(%addr, "agent listening");

    axum::serve(listener, router).await?;
    {
        let mut s = state.lock().await;
        s.listening = false;
    }
    Ok(())
}

#[derive(Clone)]
struct AgentCtx {
    app: tauri::AppHandle,
}

// Payload emitted to the React frontend after every print attempt.
// Drives the activity feed + toast notifications in the agent UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrintEvent {
    started_at: String,
    printer: String,
    job_name: Option<String>,
    ok: bool,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    ok: bool,
    version: &'static str,
    listening: bool,
    /// Human-readable computer name. Drives the agent's display name
    /// in the web app's settings → Printers → Agents view (e.g.
    /// "Casino Pier — DESKTOP-3F8K2L1").
    hostname: String,
    /// "windows" / "macos" / "linux" — surfaces the host OS so the
    /// settings UI can show a small platform badge per agent.
    platform: &'static str,
    /// Stable per-machine identifier for showing/grouping the agent
    /// in the web app even across hostname changes. Random + cached
    /// in OS-appropriate config dir on first run; here we settle for
    /// hostname so v1 doesn't need filesystem persistence.
    agent_id: String,
    /// Whether the SumatraPDF (Windows) / `lp` (mac/linux) helper is
    /// available. Drives the "PDF helper missing" warning in the
    /// agent picker UI.
    helper_installed: bool,
    /// OS default printer, if set.
    default_printer: Option<String>,
    /// Every printer the agent can see.
    printers: Vec<String>,
}

async fn health() -> impl IntoResponse {
    let hostname = std::env::var("COMPUTERNAME") // Windows
        .or_else(|_| std::env::var("HOSTNAME")) // most Unix shells
        .or_else(|_| {
            // Fallback for macOS where neither is set in the
            // environment by default.
            std::process::Command::new("hostname")
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .map_err(|_| std::env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "unknown".to_string());

    Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        listening: true,
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        },
        agent_id: hostname.clone(),
        hostname,
        helper_installed: printer::helper_installed(),
        default_printer: printer::default_printer().ok().flatten(),
        printers: printer::list_printers().unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
struct PrintQuery {
    printer: Option<String>,
    job_name: Option<String>,
}

#[derive(Serialize)]
struct PrintResponse {
    ok: bool,
    printer: String,
    job_name: Option<String>,
}

async fn print_handler(
    State(ctx): State<AgentCtx>,
    Query(q): Query<PrintQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<PrintResponse>, (StatusCode, String)> {
    let started_at = chrono_iso_now();

    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty body".into()));
    }

    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !ct.starts_with("application/pdf") {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("expected Content-Type: application/pdf, got '{ct}'"),
        ));
    }

    // Naive PDF sniff — first 5 bytes should be `%PDF-`. Saves us a
    // round-trip to the print spooler when the body is corrupted.
    if body.len() < 5 || &body[..5] != b"%PDF-" {
        return Err((
            StatusCode::BAD_REQUEST,
            "body does not look like a PDF (missing %PDF- signature)".into(),
        ));
    }

    let printer_name = match q.printer.as_deref() {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => printer::default_printer()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                (
                    StatusCode::PRECONDITION_FAILED,
                    "no printer specified and no OS default configured".to_string(),
                )
            })?,
    };

    let result = printer::print_pdf_bytes(&body, &printer_name).await;

    // Always emit a print event — success and failure both feed the
    // activity feed in the agent UI. Failure also flows back to the
    // HTTP caller so the web app can surface a toast there too.
    let event = PrintEvent {
        started_at,
        printer: printer_name.clone(),
        job_name: q.job_name.clone(),
        ok: result.is_ok(),
        error: result.as_ref().err().map(|e| e.to_string()),
    };
    let _ = ctx.app.emit("print", event);

    result.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PrintResponse {
        ok: true,
        printer: printer_name,
        job_name: q.job_name,
    }))
}

// Returns the current time as an ISO 8601 string. We avoid pulling
// in `chrono` for one date format — `SystemTime` + manual formatting
// keeps the agent's binary smaller.
fn chrono_iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let ms = now.subsec_millis();
    let datetime = secs_to_iso(secs);
    format!("{datetime}.{ms:03}Z")
}

fn secs_to_iso(secs: u64) -> String {
    // Days since 1970-01-01 (Thursday). Civil-from-days is the
    // Howard Hinnant algorithm.
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
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        y_civil, m_civil, d, h, m, s
    )
}
