// Local HTTP listener. v1 ingress — accepts PDFs over HTTP from the
// web app on the same machine and dispatches to the printer. The
// listener binds to 127.0.0.1 only so it's never exposed to the LAN.
//
// Security model (v1):
//
// 1. Bind to loopback. Nothing on the LAN can reach us.
// 2. CORS is an explicit allowlist of badgebadger origins (env-overridable
//    via BADGEBADGER_AGENT_ORIGINS). Replaces the v0 wide-open allow_origin(Any)
//    that let any tab the operator visited drive the printer.
// 3. /print requires `Authorization: Bearer <token>`. Token is per-install
//    random 256-bit hex, persisted in the OS app-data dir, surfaced via
//    the agent UI's "Pair with web app" button. Web pastes it once.
// 4. /print rejects requests without an `Origin` header in the allowlist.
//    Closes off curl-from-malware drive-by where CORS isn't enforced
//    by a browser.
// 5. /print is rate-limited (60/min steady, burst 20). Caps a hostile or
//    runaway tab without affecting realistic bulk prints.
// 6. /health stays unauthenticated so the web app can probe agent presence
//    before pairing. The endpoint discloses hostname + printer list, both
//    already visible to anyone with shell access.
//
// API:
//
//   GET  /health  -> { ok, version, listening, hostname, platform,
//                      helperInstalled, defaultPrinter, printers, agentId,
//                      authRequired }
//                    The web app calls this on page load to decide whether
//                    to route prints through the agent and to render the
//                    printer picker / status UI.
//
//   POST /print   -> body = PDF bytes (Content-Type: application/pdf)
//                    headers = Authorization: Bearer <token>
//                    query = ?printer=<name>   (optional)
//                    query = ?job_name=<name>  (optional)
//                    response = { ok, printer, job_name } on success
//
// CORS is wide-open for localhost/127.0.0.1 origins so the web app
// (running anywhere) can POST to the agent. In v2 we replace this
// with an authenticated WebSocket so cross-origin / cross-host setups
// also work.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Query, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

use crate::auth;
use crate::printer;
use crate::rate_limit::RateLimiter;

pub const DEFAULT_PORT: u16 = 9988;

// 25 MB cap per print job. Realistic single-card PDFs are <1MB; even
// a 100-page bulk batch shouldn't exceed 5MB. The limit is there to
// keep an erroneous client from filling memory with a hostile body.
const MAX_BODY_BYTES: usize = 25 * 1024 * 1024;

// Default origin allowlist when BADGEBADGER_AGENT_ORIGINS isn't set.
// Production hosts + localhost dev. Anyone running their own deployment
// adds theirs via the env var.
const DEFAULT_ORIGINS: &[&str] = &[
    "https://ids.postudios.io",
    "https://app.badgebadger.com",
    "https://www.badgebadger.com",
    "http://localhost:3000",
    "http://127.0.0.1:3000",
];

pub async fn serve(
    port: u16,
    state: Arc<Mutex<crate::AgentState>>,
    app: tauri::AppHandle,
    pairing_token: String,
) -> anyhow::Result<()> {
    let allowed_origins = resolve_allowed_origins();
    let cors = build_cors_layer(&allowed_origins);

    let ctx = AgentCtx {
        app,
        pairing_token: Arc::new(pairing_token),
        allowed_origins: Arc::new(allowed_origins),
        rate_limiter: Arc::new(RateLimiter::new()),
    };

    // /print sits behind origin + auth + rate-limit middleware.
    // /health is unauth so the web app can detect our presence before
    // the operator pastes the token. Both routers share AgentCtx via
    // with_state at the bottom of the chain.
    let protected: Router<AgentCtx> = Router::new()
        .route("/print", post(print_handler))
        .route_layer(middleware::from_fn_with_state(ctx.clone(), require_origin))
        .route_layer(middleware::from_fn_with_state(ctx.clone(), require_auth))
        .route_layer(middleware::from_fn_with_state(
            ctx.clone(),
            check_rate_limit,
        ));
    let public: Router<AgentCtx> = Router::new().route("/health", get(health));

    let router = public
        .merge(protected)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(ctx);

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
    pairing_token: Arc<String>,
    allowed_origins: Arc<Vec<String>>,
    rate_limiter: Arc<RateLimiter>,
}

fn resolve_allowed_origins() -> Vec<String> {
    if let Ok(raw) = std::env::var("BADGEBADGER_AGENT_ORIGINS") {
        let custom: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !custom.is_empty() {
            tracing::info!(
                count = custom.len(),
                "using BADGEBADGER_AGENT_ORIGINS allowlist"
            );
            return custom;
        }
    }
    DEFAULT_ORIGINS.iter().map(|s| s.to_string()).collect()
}

fn build_cors_layer(origins: &[String]) -> CorsLayer {
    let header_values: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(header_values)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("x-agent-mode"),
        ])
}

// ──────────────────────── middleware ────────────────────────

async fn require_origin(
    State(ctx): State<AgentCtx>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if origin.is_empty() {
        return Err((
            StatusCode::FORBIDDEN,
            "Origin header required for /print".to_string(),
        ));
    }
    if !ctx.allowed_origins.iter().any(|a| a.as_str() == origin) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Origin '{origin}' not allowed"),
        ));
    }
    Ok(next.run(req).await)
}

async fn require_auth(
    State(ctx): State<AgentCtx>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let provided = header.strip_prefix("Bearer ").unwrap_or("").trim();
    if provided.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "missing bearer token"));
    }
    if !auth::constant_time_eq(provided.as_bytes(), ctx.pairing_token.as_bytes()) {
        return Err((StatusCode::UNAUTHORIZED, "invalid bearer token"));
    }
    Ok(next.run(req).await)
}

async fn check_rate_limit(
    State(ctx): State<AgentCtx>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    if !ctx.rate_limiter.check() {
        return Err((StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded"));
    }
    Ok(next.run(req).await)
}

// ──────────────────────── handlers ────────────────────────

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
    /// Always true on agent versions ≥ 0.2 — tells the web app to
    /// require an Authorization header. Older agents don't ship this
    /// field; the web app treats absence as "auth not required" so
    /// upgrades don't break.
    auth_required: bool,
    /// Protocol version. v1 = CORS allowlist + bearer token. Web
    /// reads this to gate per-card POST + retry features.
    protocol: &'static str,
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
        auth_required: true,
        protocol: "1",
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
