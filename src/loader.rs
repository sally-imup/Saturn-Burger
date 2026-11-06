//! Mod loader: Fabric, Quilt, Forge, dan NeoForge.
//! Daftar versi diambil dari API masing-masing. Profil loader disimpan di folder versions
//! seperti versi biasa, lalu digabung dengan versi vanilla saat launch.

use crate::java;
use crate::launcher::{bytes_progress, data_dir, download, Expect};
use crate::manifest::{Arguments, Library};
use crate::progress::Progress;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, process::Command};

const UA: &str = "Saturn/0.1.0 (launcher Minecraft)";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl Loader {
    pub const ALL: [Loader; 5] = [
        Loader::Vanilla,
        Loader::Fabric,
        Loader::Quilt,
        Loader::Forge,
        Loader::NeoForge,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Loader::Vanilla => "Vanilla",
            Loader::Fabric => "Fabric",
            Loader::Quilt => "Quilt",
            Loader::Forge => "Forge",
            Loader::NeoForge => "NeoForge",
        }
    }

    /// Nama loader di filter Modrinth (None untuk vanilla).
    pub fn modrinth(self) -> Option<&'static str> {
        match self {
            Loader::Vanilla => None,
            Loader::Fabric => Some("fabric"),
            Loader::Quilt => Some("quilt"),
            Loader::Forge => Some("forge"),
            Loader::NeoForge => Some("neoforge"),
        }
    }
}

fn http() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder().user_agent(UA).build()?)
}

fn meta_base(loader: Loader) -> &'static str {
    match loader {
        Loader::Quilt => "https://meta.quiltmc.org/v3",
        _ => "https://meta.fabricmc.net/v2",
    }
}

/// Satu entri daftar versi Fabric/Quilt. Fabric membungkus versi di dalam `loader`,
/// Quilt langsung di `version`.
#[derive(Deserialize)]
struct MetaEntry {
    #[serde(default)]
    loader: Option<MetaVersion>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Deserialize)]
struct MetaVersion {
    version: String,
}

/// Daftar versi loader yang cocok dengan Minecraft `mc`, dari yang terbaru.
pub fn list_versions(loader: Loader, mc: &str) -> Result<Vec<String>> {
    let client = http()?;
    match loader {
        Loader::Vanilla => Ok(Vec::new()),
        Loader::Fabric | Loader::Quilt => {
            let list: Vec<MetaEntry> = client
                .get(format!("{}/versions/loader/{mc}", meta_base(loader)))
                .send()?
                .error_for_status()?
                .json()?;
            Ok(list
                .into_iter()
                .filter_map(|e| e.version.or(e.loader.map(|l| l.version)))
                .collect())
        }
        Loader::Forge => {
            let all = maven_versions(
                &client,
                "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml",
            )?;
            let prefix = format!("{mc}-");
            Ok(all
                .into_iter()
                .filter_map(|v| v.strip_prefix(&prefix).map(String::from))
                .rev()
                .collect())
        }
        Loader::NeoForge => {
            let all = maven_versions(
                &client,
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
            )?;
            let prefix = neoforge_prefix(mc);
            Ok(all.into_iter().filter(|v| v.starts_with(&prefix)).rev().collect())
        }
    }
}

/// NeoForge memakai "MINOR.PATCH.build", contoh 1.21.1 -> "21.1.".
fn neoforge_prefix(mc: &str) -> String {
    let parts: Vec<&str> = mc.split('.').collect();
    let minor = parts.get(1).copied().unwrap_or("0");
    let patch = parts.get(2).copied().unwrap_or("0");
    format!("{minor}.{patch}.")
}

/// Ambil semua <version> dari maven-metadata.xml (urutan lama ke baru).
fn maven_versions(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<String>> {
    let xml = client.get(url).send()?.error_for_status()?.text()?;
    let mut out = Vec::new();
    let mut rest = xml.as_str();
    while let Some(start) = rest.find("<version>") {
        let after = &rest[start + "<version>".len()..];
        let Some(end) = after.find("</version>") else { break };
        out.push(after[..end].trim().to_string());
        rest = &after[end + "</version>".len()..];
    }
    Ok(out)
}

/// Nama id profil seperti yang dipakai di folder versions.
pub fn profile_id(loader: Loader, mc: &str, lv: &str) -> String {
    match loader {
        Loader::Vanilla => mc.to_string(),
        Loader::Fabric => format!("fabric-loader-{lv}-{mc}"),
        Loader::Quilt => format!("quilt-loader-{lv}-{mc}"),
        Loader::Forge => format!("{mc}-forge-{lv}"),
        Loader::NeoForge => format!("neoforge-{lv}"),
    }
}

/// Bagian dari version JSON loader yang dibutuhkan launcher.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderProfile {
    pub id: String,
    pub main_class: String,
    pub arguments: Option<Arguments>,
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
}

/// Pastikan profil loader sudah terpasang, lalu kembalikan isinya.
/// Fabric/Quilt cukup mengunduh JSON-nya. Forge/NeoForge menjalankan installer resminya.
pub fn ensure(
    loader: Loader,
    mc: &str,
    lv: &str,
    java_major: u32,
    progress: &mut dyn FnMut(Progress),
) -> Result<LoaderProfile> {
    if loader == Loader::Vanilla {
        bail!("vanilla tidak punya profil loader");
    }
    let id = profile_id(loader, mc, lv);
    let json_path = data_dir().join("versions").join(&id).join(format!("{id}.json"));

    if !json_path.exists() {
        match loader {
            Loader::Fabric | Loader::Quilt => {
                progress(Progress::text(format!("Mengunduh profil {}...", loader.label())));
                let url = format!("{}/versions/loader/{mc}/{lv}/profile/json", meta_base(loader));
                let text = http()?.get(&url).send()?.error_for_status()?.text()?;
                if let Some(parent) = json_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&json_path, text)?;
            }
            _ => run_installer(loader, mc, lv, java_major, progress)?,
        }
        if !json_path.exists() {
            bail!("profil {} tidak ditemukan setelah pemasangan", loader.label());
        }
    }

    let text = fs::read_to_string(&json_path)?;
    serde_json::from_str(&text).with_context(|| format!("profil {} rusak: {}", loader.label(), json_path.display()))
}

/// Jalankan installer resmi Forge/NeoForge untuk folder Saturn.
fn run_installer(
    loader: Loader,
    mc: &str,
    lv: &str,
    java_major: u32,
    progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    let root = data_dir();
    fs::create_dir_all(&root)?;
    // Installer membaca launcher_profiles.json; isi kosong sudah cukup.
    let lp = root.join("launcher_profiles.json");
    if !lp.exists() {
        fs::write(&lp, r#"{"profiles":{}}"#)?;
    }

    let (file, url) = match loader {
        Loader::Forge => (
            format!("forge-{mc}-{lv}-installer.jar"),
            format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{lv}/forge-{mc}-{lv}-installer.jar"),
        ),
        Loader::NeoForge => (
            format!("neoforge-{lv}-installer.jar"),
            format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{lv}/neoforge-{lv}-installer.jar"),
        ),
        _ => bail!("installer hanya untuk Forge dan NeoForge"),
    };
    let jar = root.join("installers").join(file);
    let label = format!("Mengunduh installer {}", loader.label());
    download(&url, &jar, &Expect::none(), &mut |d, t| {
        progress(bytes_progress(&label, d, t, 0.0, 1.0))
    })?;

    let java = java::ensure_java(java_major, progress)?;
    progress(Progress::text(format!(
        "Menjalankan installer {} (bisa beberapa menit)...",
        loader.label()
    )));
    let out = Command::new(&java)
        .arg("-jar")
        .arg(&jar)
        .arg("--installClient")
        .arg(&root)
        .current_dir(&root)
        .output()
        .with_context(|| format!("gagal menjalankan installer {}", loader.label()))?;

    if !out.status.success() {
        let log = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let tail: String = log.chars().rev().take(600).collect::<Vec<_>>().into_iter().rev().collect();
        bail!("installer {} gagal:\n{}", loader.label(), tail.trim());
    }
    Ok(())
}
