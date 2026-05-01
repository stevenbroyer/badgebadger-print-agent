// Pairing token: a 256-bit random secret the operator pastes into
// the BadgeBadger web app to authenticate the local agent. Without
// it any tab the operator visits could POST to localhost:9988/print
// thanks to permissive CORS for our own UI's sake. With it, only
// browsers that have been paired (token in localStorage) can drive
// the printer.
//
// Persistence: a single hex line in the platform's app-local data
// dir. Generated on first run; survives upgrades. Mode 0600 on
// unix. Hex (not base64) so it's selectable as a single word in
// any terminal/UI without surprises.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tauri::Manager;

const TOKEN_BYTES: usize = 32;

/// Returns the pairing token, creating + persisting it on first run.
pub async fn load_or_create_token(app: &tauri::AppHandle) -> Result<String> {
    let path = token_path(app)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    if let Ok(existing) = tokio::fs::read_to_string(&path).await {
        let trimmed = existing.trim().to_string();
        if is_valid_token(&trimmed) {
            return Ok(trimmed);
        }
        tracing::warn!(
            "pairing token at {} was malformed; regenerating",
            path.display()
        );
    }
    let token = generate_token();
    write_token(&path, &token).await?;
    tracing::info!(path = %path.display(), "wrote new pairing token");
    Ok(token)
}

fn is_valid_token(s: &str) -> bool {
    s.len() == TOKEN_BYTES * 2 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn generate_token() -> String {
    let mut buf = [0u8; TOKEN_BYTES];
    getrandom::getrandom(&mut buf).expect("getrandom failed");
    hex_encode(&buf)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

async fn write_token(path: &Path, token: &str) -> Result<()> {
    tokio::fs::write(path, token)
        .await
        .with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(path).await?.permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(path, perms).await?;
    }
    Ok(())
}

fn token_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_local_data_dir()
        .context("could not resolve app-local data dir")?;
    Ok(dir.join("token"))
}

/// Constant-time byte comparison. Used to compare the bearer token
/// the client sent against our stored value without leaking timing
/// signal about how many leading bytes matched.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
