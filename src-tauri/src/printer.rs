// Printer integration. Two platform-specific paths:
//
// • Windows: enumerate printers via `EnumPrinters` (winspool API),
//   fetch the OS default via `GetDefaultPrinter`, print PDFs by
//   shelling out to a CLI PDF tool. v1 prefers SumatraPDF
//   (`SumatraPDF.exe -print-to <queue> -silent <file>`) because:
//
//     1. It's the only widely-available Windows PDF handler with a
//        documented, scriptable CLI for printing to a named queue.
//     2. Microsoft Edge — Win10/11's default PDF handler — does NOT
//        implement the `printto` shell verb. Falling back to
//        `ShellExecuteW("printto", ...)` is a silent no-op on stock
//        installs (the call returns success but no print happens).
//     3. Adobe Reader / Foxit DO implement `printto` but require
//        their own install. SumatraPDF is a single 6MB exe with no
//        bundled cruft.
//
//   Operators install SumatraPDF themselves (free, GPLv3, link in
//   the agent README + setup wizard). If it's not on PATH or in a
//   common install dir, we fall back to ShellExecute("printto") as
//   a last resort + return a clear error so the user knows what to
//   install.
//
//   **TODO (v0.2 / production)**: replace SumatraPDF with
//   `pdfium-render` (BSD) for in-process rendering → bitmap → Win32
//   `WritePrinter`. Removes the SumatraPDF install requirement and
//   gives us full control over scaling, bleed, and color management.
//   Adds ~8MB to the agent binary; worthwhile for the multi-tenant
//   product.
//
// • macOS / Linux: use the `lp` command (CUPS). Built into macOS,
//   nearly always present on Linux. The agent's value on these
//   platforms is mostly the local-HTTP listener + pairing UX rather
//   than the print backend itself.

use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

#[cfg(target_os = "windows")]
mod windows_printer {
    use anyhow::{Context, anyhow};
    use windows::Win32::Graphics::Printing::{
        EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
        PRINTER_INFO_2W,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::{HSTRING, PCWSTR};

    pub fn list_printers() -> anyhow::Result<Vec<String>> {
        unsafe {
            // Two-pass enumeration: call once with a zero-byte buffer
            // to learn the required size, then call again with that
            // buffer. Standard Win32 idiom.
            let mut needed: u32 = 0;
            let mut returned: u32 = 0;
            let _ = EnumPrintersW(
                PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
                PCWSTR::null(),
                2,
                None,
                &mut needed,
                &mut returned,
            );
            if needed == 0 {
                return Ok(vec![]);
            }
            let mut buffer: Vec<u8> = vec![0; needed as usize];
            EnumPrintersW(
                PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
                PCWSTR::null(),
                2,
                Some(&mut buffer[..]),
                &mut needed,
                &mut returned,
            )
            .map_err(|e| anyhow!("EnumPrintersW failed: {e:?}"))?;

            let mut printers = Vec::with_capacity(returned as usize);
            for i in 0..returned as usize {
                let info = (buffer.as_ptr() as *const PRINTER_INFO_2W).add(i);
                let name_ptr = (*info).pPrinterName;
                if !name_ptr.is_null() {
                    let name = name_ptr
                        .to_string()
                        .unwrap_or_else(|_| "<invalid>".to_string());
                    printers.push(name);
                }
            }
            Ok(printers)
        }
    }

    pub fn default_printer() -> anyhow::Result<Option<String>> {
        unsafe {
            let mut len: u32 = 0;
            let _ = GetDefaultPrinterW(None, &mut len);
            if len == 0 {
                return Ok(None);
            }
            let mut buf = vec![0u16; len as usize];
            GetDefaultPrinterW(Some(windows::core::PWSTR(buf.as_mut_ptr())), &mut len)
                .map_err(|e| anyhow!("GetDefaultPrinterW failed: {e:?}"))?;
            // GetDefaultPrinterW writes a null-terminated string; trim.
            let name = String::from_utf16_lossy(
                &buf[..buf.iter().position(|&c| c == 0).unwrap_or(buf.len())],
            );
            Ok(Some(name))
        }
    }

    /// Locate SumatraPDF on the host. Tries PATH first (most
    /// portable; users who installed via Chocolatey / Scoop end up
    /// here), then a handful of common install locations the
    /// official MSI / portable distribution writes to. Returns None
    /// when SumatraPDF isn't installed; the caller falls back to
    /// ShellExecute("printto").
    pub fn find_sumatra() -> Option<std::path::PathBuf> {
        use std::path::Path;
        // PATH lookup — works for portable installs the user dropped
        // somewhere reachable.
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                let candidate = dir.join("SumatraPDF.exe");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        // Common install locations.
        let well_known = [
            r"C:\Program Files\SumatraPDF\SumatraPDF.exe",
            r"C:\Program Files (x86)\SumatraPDF\SumatraPDF.exe",
        ];
        for p in well_known {
            let path = Path::new(p);
            if path.is_file() {
                return Some(path.to_path_buf());
            }
        }
        // User-local installs (default for the official portable .zip
        // when extracted to %LOCALAPPDATA%).
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let candidate = Path::new(&local).join("SumatraPDF").join("SumatraPDF.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn print_pdf(path: &std::path::Path, printer_name: &str) -> anyhow::Result<()> {
        // Preferred path: shell out to SumatraPDF. It's the only
        // Windows PDF handler with a reliable scriptable CLI for
        // printing to a specific queue, and it works without Edge /
        // Adobe Reader being installed.
        if let Some(sumatra) = find_sumatra() {
            tracing::info!(?sumatra, ?printer_name, "printing via SumatraPDF");
            let status = std::process::Command::new(&sumatra)
                .arg("-print-to")
                .arg(printer_name)
                // -silent suppresses error dialogs so the user
                // never sees a window flash from SumatraPDF itself.
                .arg("-silent")
                // -exit-when-done so SumatraPDF closes after
                // dispatching the print job (it lingers by default
                // when launched without a UI).
                .arg("-exit-when-done")
                .arg(path)
                .status()
                .with_context(|| format!("could not invoke {}", sumatra.display()))?;
            if status.success() {
                return Ok(());
            }
            tracing::warn!(?status, "SumatraPDF exited non-zero; falling back to printto");
        } else {
            tracing::warn!(
                "SumatraPDF not found — falling back to ShellExecute('printto') which silently no-ops on stock Edge installs"
            );
        }

        // Fallback: ShellExecuteW with the `printto` verb. Works only
        // when an Adobe Reader / Foxit-class PDF handler that
        // implements the verb is the registered default. Stock
        // Win10/11 ships Microsoft Edge as the PDF handler and
        // Edge does NOT implement `printto`, so this fallback is
        // a silent no-op on most untouched installs. Returns a
        // clear error to the caller in that case so the operator
        // sees an actionable message in the agent UI.
        let verb = HSTRING::from("printto");
        let file = HSTRING::from(path.to_string_lossy().as_ref());
        let args_str = format!(r#""{}""#, printer_name);
        let args = HSTRING::from(args_str.as_str());
        unsafe {
            let result = ShellExecuteW(
                None,
                PCWSTR::from_raw(verb.as_ptr()),
                PCWSTR::from_raw(file.as_ptr()),
                PCWSTR::from_raw(args.as_ptr()),
                PCWSTR::null(),
                SW_HIDE,
            );
            // ShellExecute returns an HINSTANCE > 32 on success.
            if (result.0 as isize) <= 32 {
                return Err(anyhow!(
                    "ShellExecute('printto') returned {}. Stock Windows installs use Microsoft Edge as the PDF handler, and Edge doesn't implement the printto verb — install SumatraPDF (https://www.sumatrapdfreader.org) and the agent will pick it up automatically.",
                    result.0 as isize
                ));
            }
        }
        // ShellExecute returning > 32 means the handler accepted the
        // call, but on Edge that "acceptance" is a silent no-op —
        // the call returns success while doing nothing. Surface a
        // warning so the operator can investigate if no card prints.
        if find_sumatra().is_none() {
            tracing::warn!(
                "ShellExecute('printto') returned success, but if Edge is your default PDF handler this was likely a no-op. Install SumatraPDF for reliable printing."
            );
        }
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod unix_printer {
    use anyhow::{Context, anyhow};
    use tokio::process::Command;

    pub fn list_printers() -> anyhow::Result<Vec<String>> {
        // Shell out to `lpstat -p` synchronously — short-running.
        let out = std::process::Command::new("lpstat")
            .arg("-p")
            .output()
            .context("lpstat not available")?;
        if !out.status.success() {
            return Ok(vec![]);
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut printers = Vec::new();
        for line in stdout.lines() {
            // "printer Foo is idle.  enabled since ..."
            if let Some(rest) = line.strip_prefix("printer ") {
                if let Some((name, _)) = rest.split_once(' ') {
                    printers.push(name.to_string());
                }
            }
        }
        Ok(printers)
    }

    pub fn default_printer() -> anyhow::Result<Option<String>> {
        let out = std::process::Command::new("lpstat")
            .arg("-d")
            .output()
            .context("lpstat not available")?;
        if !out.status.success() {
            return Ok(None);
        }
        // "system default destination: Foo"
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if let Some(idx) = line.find(": ") {
                return Ok(Some(line[idx + 2..].trim().to_string()));
            }
        }
        Ok(None)
    }

    pub async fn print_pdf(
        path: &std::path::Path,
        printer_name: &str,
    ) -> anyhow::Result<()> {
        let status = Command::new("lp")
            .arg("-d")
            .arg(printer_name)
            .arg(path)
            .status()
            .await
            .context("could not run `lp`")?;
        if !status.success() {
            return Err(anyhow!("lp exited with {status}"));
        }
        Ok(())
    }
}

/// True if a third-party PDF helper that the agent depends on is
/// installed. Drives the setup checklist in the agent UI.
///
/// • Windows: needs SumatraPDF to bridge PDF → printer queue.
/// • macOS / Linux: CUPS / `lp` handles PDFs natively; always true.
pub fn helper_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_printer::find_sumatra().is_some()
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        true
    }
}

pub fn list_printers() -> anyhow::Result<Vec<String>> {
    #[cfg(target_os = "windows")]
    {
        windows_printer::list_printers()
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        unix_printer::list_printers()
    }
}

pub fn default_printer() -> anyhow::Result<Option<String>> {
    #[cfg(target_os = "windows")]
    {
        windows_printer::default_printer()
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        unix_printer::default_printer()
    }
}

pub async fn print_pdf_bytes(bytes: &[u8], printer_name: &str) -> anyhow::Result<()> {
    let temp = NamedTempFile::with_suffix(".pdf")?;
    let path: PathBuf = temp.path().to_path_buf();
    {
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(bytes).await?;
        file.flush().await?;
    }
    // Keep the temp file alive past the print dispatch — Windows
    // `printto` returns immediately but the underlying handler may
    // still be reading the file when ShellExecute returns. We sleep
    // briefly to let the handler queue the job, then drop the temp.
    let result = dispatch(&path, printer_name).await;
    tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;
    drop(temp);
    result
}

async fn dispatch(path: &std::path::Path, printer_name: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // ShellExecute is sync; run on the blocking pool so we don't
        // block the tokio reactor.
        let path = path.to_path_buf();
        let printer = printer_name.to_string();
        tokio::task::spawn_blocking(move || windows_printer::print_pdf(&path, &printer)).await??;
        Ok(())
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        unix_printer::print_pdf(path, printer_name).await
    }
}
