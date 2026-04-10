use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub api_key: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
}

fn default_api_url() -> String {
    "https://inspect-api.fly.dev".to_string()
}

fn credentials_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("inspect");
    config_dir.join("credentials.json")
}

pub fn load_credentials() -> Option<Credentials> {
    let path = credentials_path();
    let data = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_credentials(creds: &Credentials) -> Result<(), String> {
    let path = credentials_path();
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create config dir: {e}"))?;

    let data = serde_json::to_string_pretty(creds)
        .map_err(|e| format!("Failed to serialize credentials: {e}"))?;
    fs::write(&path, &data).map_err(|e| format!("Failed to write credentials: {e}"))?;

    // Set permissions to 0600 on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms)
            .map_err(|e| format!("Failed to set permissions: {e}"))?;
    }

    Ok(())
}

pub fn remove_credentials() -> Result<(), String> {
    let path = credentials_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to remove credentials: {e}"))?;
    }
    Ok(())
}

pub fn require_credentials() -> Result<Credentials, String> {
    // Check env var first (CI-friendly)
    if let Ok(key) = std::env::var("INSPECT_API_KEY") {
        let url = resolve_api_url(None);
        return Ok(Credentials {
            api_key: key,
            api_url: url,
        });
    }

    load_credentials().ok_or_else(|| {
        "Not logged in. Run `inspect login` or set INSPECT_API_KEY.".to_string()
    })
}

pub fn resolve_api_url(flag: Option<&str>) -> String {
    if let Some(url) = flag {
        return url.to_string();
    }
    if let Ok(url) = std::env::var("INSPECT_API_URL") {
        return url;
    }
    if let Some(creds) = load_credentials() {
        return creds.api_url;
    }
    default_api_url()
}
