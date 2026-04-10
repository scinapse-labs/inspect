use clap::Args;
use colored::Colorize;
use std::io::{self, Write};

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

pub async fn run(args: LoginArgs) {
    let key = match args.api_key {
        Some(k) => k,
        None => {
            println!("To get an API key, visit:");
            println!(
                "  {}",
                "https://inspect.ataraxy-labs.com/dashboard/keys"
                    .underline()
                    .cyan()
            );
            println!();

            // Best-effort open browser
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg("https://inspect.ataraxy-labs.com/dashboard/keys")
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg("https://inspect.ataraxy-labs.com/dashboard/keys")
                    .spawn();
            }

            eprint!("Paste your API key: ");
            io::stderr().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        }
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
