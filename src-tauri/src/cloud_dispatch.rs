// Cross-device print dispatch — the agent's outbound poll loop.
//
// The BadgeBadger web app stores queued prints in `agent_print_jobs`
// (target_device_id, signed PDF URL, target_printer, …). This loop
// hits GET /api/agent/jobs every couple of seconds, downloads any
// claimed PDFs, prints them locally, and POSTs an ack so the
// dispatching browser knows whether the job landed.
//
// Auth: the same hex bearer token that's used by /print on the local
// HTTP listener. The web app paired this device by writing the token
// into `agent_devices`; the server's auth middleware looks it up
// there.
//
// Backoff strategy:
//   * 2s while jobs were processed this tick (operator is actively
//     using the system; stay snappy)
//   * 5s after a quiet tick (no jobs available)
//   * 30s after a string of empty polls (idle workstation, save CPU)
//   * 60s after auth failure (the agent isn't paired yet, no point
//     hammering the server) — automatically drops back to 5s as soon
//     as a poll succeeds, so re-pair recovery is fast.

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use tauri::Emitter;

use crate::printer;

const DEFAULT_BASE_URL: &str = "https://hq.badgebadger.app";

#[derive(Debug, Deserialize)]
struct PollResponse {
    jobs: Vec<RemoteJob>,
    #[serde(default)]
    poll_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RemoteJob {
    id: String,
    target_printer: String,
    download_url: String,
    #[serde(default)]
    employee_name: Option<String>,
    #[serde(default)]
    template_name: Option<String>,
    #[serde(default)]
    job_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PrintEventOut<'a> {
    started_at: &'a str,
    printer: &'a str,
    job_name: Option<&'a str>,
    employee_name: Option<&'a str>,
    template_name: Option<&'a str>,
    ok: bool,
    error: Option<&'a str>,
}

pub async fn run(app_handle: tauri::AppHandle, token: String) {
    let base_url =
        std::env::var("BADGEBADGER_API_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let client = match reqwest::Client::builder()
        .user_agent(concat!("badgebadger-print-agent/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            tracing::error!(?err, "could not build reqwest client; cloud dispatch disabled");
            return;
        }
    };

    let mut idle_streak: u32 = 0;
    let mut auth_failed = false;
    loop {
        match poll_once(&client, &base_url, &token).await {
            Ok(jobs) => {
                auth_failed = false;
                if jobs.is_empty() {
                    idle_streak = idle_streak.saturating_add(1);
                } else {
                    idle_streak = 0;
                    for job in jobs {
                        process_job(&app_handle, &client, &base_url, &token, job).await;
                    }
                }
            }
            Err(err) => {
                if err.to_string().contains("401") || err.to_string().contains("403") {
                    if !auth_failed {
                        tracing::warn!(
                            "cloud dispatch auth failed — pair this agent in the BadgeBadger web app to enable remote prints"
                        );
                        auth_failed = true;
                    }
                } else {
                    tracing::debug!(?err, "cloud dispatch poll failed; will retry");
                }
                idle_streak = idle_streak.saturating_add(1);
            }
        }
        let sleep_ms = if auth_failed {
            60_000
        } else if idle_streak == 0 {
            2_000
        } else if idle_streak < 5 {
            5_000
        } else {
            30_000
        };
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }
}

async fn poll_once(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
) -> Result<Vec<RemoteJob>> {
    let url = format!("{base_url}/api/agent/jobs?limit=5");
    let res = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .context("poll request failed")?;
    if !res.status().is_success() {
        anyhow::bail!("poll status {}", res.status());
    }
    let body: PollResponse = res.json().await.context("poll response not JSON")?;
    let _ = body.poll_interval_ms; // hint only; we keep our own backoff
    Ok(body.jobs)
}

async fn process_job(
    app_handle: &tauri::AppHandle,
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    job: RemoteJob,
) {
    let started_at = iso_now();
    // Guardrail: verify the requested printer is actually connected
    // right now. The dispatch endpoint checked this against the snapshot
    // at queue time, but printers can be unplugged between queue and
    // claim. Acking failed with a clear message is much better than
    // silently retargeting to a similar-named printer.
    let connected = printer::list_printers().unwrap_or_default();
    if !connected.iter().any(|p| p == &job.target_printer) {
        let msg = format!(
            "printer '{}' is not connected to this machine",
            job.target_printer
        );
        emit_event(
            app_handle,
            &started_at,
            &job.target_printer,
            job.job_name.as_deref(),
            job.employee_name.as_deref(),
            job.template_name.as_deref(),
            false,
            Some(&msg),
        );
        post_ack(client, base_url, token, &job.id, false, Some(&msg)).await;
        return;
    }

    let pdf_bytes = match download_pdf(client, &job.download_url).await {
        Ok(b) => b,
        Err(err) => {
            let msg = format!("could not download PDF: {err}");
            emit_event(
                app_handle,
                &started_at,
                &job.target_printer,
                job.job_name.as_deref(),
                job.employee_name.as_deref(),
                job.template_name.as_deref(),
                false,
                Some(&msg),
            );
            post_ack(client, base_url, token, &job.id, false, Some(&msg)).await;
            return;
        }
    };

    match printer::print_pdf_bytes(&pdf_bytes, &job.target_printer).await {
        Ok(()) => {
            emit_event(
                app_handle,
                &started_at,
                &job.target_printer,
                job.job_name.as_deref(),
                job.employee_name.as_deref(),
                job.template_name.as_deref(),
                true,
                None,
            );
            post_ack(client, base_url, token, &job.id, true, None).await;
        }
        Err(err) => {
            let msg = err.to_string();
            emit_event(
                app_handle,
                &started_at,
                &job.target_printer,
                job.job_name.as_deref(),
                job.employee_name.as_deref(),
                job.template_name.as_deref(),
                false,
                Some(&msg),
            );
            post_ack(client, base_url, token, &job.id, false, Some(&msg)).await;
        }
    }
}

async fn download_pdf(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let res = client
        .get(url)
        .send()
        .await
        .context("PDF download request failed")?;
    if !res.status().is_success() {
        anyhow::bail!("PDF download status {}", res.status());
    }
    let bytes = res.bytes().await.context("PDF body read failed")?;
    Ok(bytes.to_vec())
}

async fn post_ack(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    job_id: &str,
    ok: bool,
    error: Option<&str>,
) {
    let url = format!("{base_url}/api/agent/jobs/{job_id}/ack");
    let body = serde_json::json!({ "ok": ok, "error": error });
    if let Err(err) = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
    {
        tracing::warn!(?err, job_id, "ack post failed; the next poll will treat the job as terminal anyway");
    }
}

fn emit_event(
    app_handle: &tauri::AppHandle,
    started_at: &str,
    printer: &str,
    job_name: Option<&str>,
    employee_name: Option<&str>,
    template_name: Option<&str>,
    ok: bool,
    error: Option<&str>,
) {
    let _ = app_handle.emit(
        "print",
        PrintEventOut {
            started_at,
            printer,
            job_name,
            employee_name,
            template_name,
            ok,
            error,
        },
    );
}

fn iso_now() -> String {
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
