//! Cek rilis terbaru di GitHub dan jalankan installer baru untuk update.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{fs, process::Command};

/// Ganti dengan "owner/repo" repo GitHub kamu.
pub const REPO: &str = "OWNER/saturn";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone)]
pub struct Update {
    pub version: String,
    pub url: String,
}

/// Mengembalikan `Some` jika ada rilis yang lebih baru dari versi yang sedang berjalan.
pub fn check() -> Result<Option<Update>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let rel: Release = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", "Saturn-Launcher")
        .header("Accept", "application/vnd.github+json")
        .send()?
        .error_for_status()?
        .json()?;

    let latest = rel.tag_name.trim_start_matches('v').to_string();
    if !is_newer(&latest, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }

    let asset = rel
        .assets
        .into_iter()
        .find(|a| a.name.ends_with("-Setup.exe"))
        .ok_or_else(|| anyhow!("installer tidak ada di rilis {latest}"))?;

    Ok(Some(Update {
        version: latest,
        url: asset.browser_download_url,
    }))
}

fn parse(v: &str) -> Vec<u32> {
    v.split(|c| c == '.' || c == '-')
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse(latest) > parse(current)
}

/// Unduh installer ke folder temp lalu jalankan. Pemanggil sebaiknya menutup aplikasi setelahnya.
pub fn download_and_run(u: &Update) -> Result<()> {
    let bytes = reqwest::blocking::get(&u.url)?.error_for_status()?.bytes()?;
    let path = std::env::temp_dir().join("Saturn-Setup.exe");
    fs::write(&path, &bytes)?;
    Command::new(&path).spawn()?;
    Ok(())
}
