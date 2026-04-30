// Printer integration. Three platform-specific paths:
//
// • Windows: enumerate printers via `EnumPrinters` (winspool API),
//   fetch the OS default via `GetDefaultPrinter`, print PDFs by
//   shelling out to `ShellExecuteW` with the `printto` verb. The
//   `printto` verb invokes whatever PDF handler the user has set as
//   default (Edge ships built-in on Win10/11; Adobe Reader / Foxit
//   / SumatraPDF also work). The handler in turn submits the PDF to
//   the chosen printer queue and exits silently.
//
// • macOS / Linux: use the `lp` command (CUPS). Built into macOS,
//   nearly always present on Linux. Same approach as the Phase-1
//   server-side direct-print path we explored earlier — the agent's
//   value on these platforms is mostly the local-HTTP listener +
//   pairing UX rather than the print backend itself.

use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;

#[cfg(target_os = "windows")]
mod windows_printer {
    use super::*;
    use anyhow::{Context, anyhow};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows::Win32::Graphics::Printing::{
        EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_LOCAL, PRINTER_ENUM_CONNECTIONS,
        PRINTER_INFO_2W,
    };
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::{HSTRING, PCWSTR};

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

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

    pub fn print_pdf(path: &std::path::Path, printer_name: &str) -> anyhow::Result<()> {
        // ShellExecuteW with the `printto` verb. Args[0] is the
        // printer name, quoted to handle queues with spaces.
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
                    "ShellExecute returned {} for printto verb",
                    result.0 as isize
                ))
                .context("Windows print dispatch failed");
            }
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
