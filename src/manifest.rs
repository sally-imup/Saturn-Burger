use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<Version>,
}

/// Satu entri di version_manifest_v2.json.
/// `kind` berisi: "release", "snapshot", "old_beta", "old_alpha".
#[derive(Deserialize, Clone)]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    pub main_class: String,
    pub asset_index: AssetIndexRef,
    pub downloads: ClientDownloads,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub arguments: Option<Arguments>,
    pub minecraft_arguments: Option<String>,
    /// Versi Java yang dibutuhkan versi ini (default 8 jika tidak ada).
    pub java_version: Option<JavaVersion>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub major_version: u32,
}

#[derive(Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize)]
pub struct ClientDownloads {
    pub client: Artifact,
}

#[derive(Deserialize)]
pub struct Artifact {
    pub path: Option<String>,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
    /// Format lama (tanpa `downloads`): base URL maven.
    pub url: Option<String>,
}

#[derive(Deserialize)]
pub struct LibDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<Value>,
}

#[derive(Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Arguments {
    pub game: Vec<Value>,
    pub jvm: Vec<Value>,
}

impl Default for Arguments {
    fn default() -> Self {
        Self { game: vec![], jvm: vec![] }
    }
}

pub fn fetch_manifest() -> Result<Vec<Version>> {
    let m: Manifest = reqwest::blocking::get(MANIFEST_URL)?
        .error_for_status()?
        .json()?;
    Ok(m.versions)
}

pub fn fetch_version_json(url: &str) -> Result<VersionJson> {
    Ok(reqwest::blocking::get(url)?.error_for_status()?.json()?)
}

/// Evaluasi rules Mojang untuk Windows. Rule yang butuh `features`
/// (demo mode, custom resolution, dll.) dianggap tidak cocok.
pub fn rules_allow(rules: Option<&[Rule]>) -> bool {
    let Some(rules) = rules else { return true };
    let mut allowed = false;
    for r in rules {
        if r.features.is_some() {
            continue;
        }
        let os_ok = r
            .os
            .as_ref()
            .and_then(|o| o.name.as_deref())
            .map_or(true, |n| n == "windows");
        if os_ok {
            allowed = r.action == "allow";
        }
    }
    allowed
}
