use clap::Args;
use colored::Colorize;

use crate::config;

#[derive(Args)]
pub struct WhoamiArgs {}

pub async fn run(_args: WhoamiArgs) {
    let creds = match config::require_credentials() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.red());
            std::process::exit(1);
        }
    };

    // Mask key: show first 8 and last 4 chars
    let masked = if creds.api_key.len() > 12 {
        format!(
            "{}...{}",
            &creds.api_key[..8],
            &creds.api_key[creds.api_key.len() - 4..]
        )
    } else {
        "***".to_string()
    };

    println!("  Key: {}", masked);
    println!("  API: {}", creds.api_url);

    // Check auth status
    eprint!("  Auth: ");
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/v1/whoami", creds.api_url))
        .header("Authorization", format!("Bearer {}", creds.api_key))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            eprintln!("{}", "authenticated".green());
        }
        Ok(r) => {
            eprintln!("{}", format!("invalid ({})", r.status()).red());
        }
        Err(e) => {
            eprintln!("{}", format!("unreachable ({e})").red());
        }
    }
}
