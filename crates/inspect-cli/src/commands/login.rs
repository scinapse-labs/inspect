use clap::Args;
use colored::Colorize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::config;

#[derive(Args)]
pub struct LoginArgs {
    /// API key (skip interactive prompt)
    #[arg(long)]
    pub api_key: Option<String>,

    /// API URL override
    #[arg(long)]
    pub api_url: Option<String>,
}

const AUTH_URL: &str = "https://inspect.ataraxy-labs.com/cli/auth";
const TIMEOUT_SECS: u64 = 120;

pub async fn run(args: LoginArgs) {
    let key = match args.api_key {
        Some(k) => k,
        None => match listen_for_callback().await {
            Some(k) => k,
            None => {
                eprintln!("{}", "error: login timed out or failed".red());
                std::process::exit(1);
            }
        },
    };

    if key.is_empty() {
        eprintln!("{}", "error: no API key provided".red());
        std::process::exit(1);
    }

    if !key.starts_with("insp_") {
        eprintln!(
            "{}",
            "error: invalid key format (expected insp_ prefix)".red()
        );
        std::process::exit(1);
    }

    let api_url = config::resolve_api_url(args.api_url.as_deref());

    // Validate key against API
    eprint!("Validating... ");
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/v1/whoami", api_url))
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            eprintln!("{}", "valid".green());
        }
        Ok(r) => {
            eprintln!("{}", "invalid".red());
            eprintln!(
                "error: API returned {} (check your key)",
                r.status()
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{}", "failed".red());
            eprintln!("error: could not reach API: {e}");
            std::process::exit(1);
        }
    }

    let creds = config::Credentials {
        api_key: key,
        api_url,
    };

    match config::save_credentials(&creds) {
        Ok(()) => println!("{}", "Logged in successfully.".green()),
        Err(e) => {
            eprintln!("{}", format!("error: {e}").red());
            std::process::exit(1);
        }
    }
}

/// Start a local server, open the browser, and wait for the callback with the API key.
async fn listen_for_callback() -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let port = listener.local_addr().ok()?.port();

    let url = format!("{AUTH_URL}?port={port}");
    println!("Opening browser to log in...");
    println!("  {}", url.underline().cyan());
    println!();

    open_browser(&url);

    let key = tokio::time::timeout(
        std::time::Duration::from_secs(TIMEOUT_SECS),
        accept_callback(&listener),
    )
    .await
    .ok()
    .flatten();

    key
}

async fn accept_callback(listener: &TcpListener) -> Option<String> {
    let (mut stream, _) = listener.accept().await.ok()?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.ok()?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse GET /callback?key=insp_... HTTP/1.1
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;

    let key = parse_key_from_path(path);

    let (status, body) = if key.is_some() {
        ("200 OK", SUCCESS_HTML)
    } else {
        ("400 Bad Request", ERROR_HTML)
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{body}"
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;

    key
}

fn parse_key_from_path(path: &str) -> Option<String> {
    let query = path.split('?').nth(1)?;
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("key=") {
            let decoded = urldecode(value);
            if decoded.starts_with("insp_") {
                return Some(decoded);
            }
        }
    }
    None
}

fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().and_then(|c| (c as char).to_digit(16));
            let lo = chars.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(h), Some(l)) = (hi, lo) {
                result.push((h * 16 + l) as u8 as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn();
    }
}

const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>inspect</title>
<style>
  body { font-family: system-ui, sans-serif; background: #0a0a0a; color: #e0e0e0;
         display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
  .card { text-align: center; }
  h1 { font-size: 20px; color: #4ade80; margin-bottom: 8px; }
  p { color: #666; font-size: 14px; }
</style>
</head>
<body><div class="card"><h1>Logged in</h1><p>You can close this tab.</p></div></body>
</html>"#;

const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>inspect</title>
<style>
  body { font-family: system-ui, sans-serif; background: #0a0a0a; color: #e0e0e0;
         display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
  .card { text-align: center; }
  h1 { font-size: 20px; color: #f87171; margin-bottom: 8px; }
  p { color: #666; font-size: 14px; }
</style>
</head>
<body><div class="card"><h1>Login failed</h1><p>Invalid or missing key. Try again.</p></div></body>
</html>"#;
