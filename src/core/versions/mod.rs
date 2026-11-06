//! Minecraft Java version manifest loading, caching, and classification.
//!
//! The public manifest is the Mojang launcher metadata endpoint. Manifest `type`
//! values are release, snapshot, old_beta, and old_alpha; pre-releases and April
//! Fools builds therefore need additional ID-based classification.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const CACHE_FILE: &str = "version_manifest_v2.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VersionCategory {
    Release,
    Snapshot,
    AprilFools,
    PreRelease,
    OldBeta,
    OldAlpha,
    Experimental,
}

impl VersionCategory {
    pub const ALL: [Self; 7] = [
        Self::Release, Self::Snapshot, Self::AprilFools, Self::PreRelease,
        Self::OldBeta, Self::OldAlpha, Self::Experimental,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Release => "Vanilla / Release",
            Self::Snapshot => "Snapshot",
            Self::AprilFools => "April Fools",
            Self::PreRelease => "Pre-release / RC",
            Self::OldBeta => "Old Beta",
            Self::OldAlpha => "Old Alpha",
            Self::Experimental => "Experimental / Custom",
        }
    }

    pub const fn directory(self) -> &'static str {
        match self {
            Self::Release => "vanilla",
            Self::Snapshot => "snapshot",
            Self::AprilFools => "april-fools",
            Self::PreRelease => "pre-release",
            Self::OldBeta => "old-beta",
            Self::OldAlpha => "old-alpha",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionManifest {
    #[serde(default)]
    pub latest: LatestVersions,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LatestVersions {
    pub release: Option<String>,
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    #[serde(rename = "releaseTime", default)]
    pub release_time: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub compliance_level: Option<u8>,
}

impl ManifestVersion {
    pub fn category(&self) -> VersionCategory {
        classify_version(&self.id, &self.version_type)
    }
}

pub fn classify_version(id: &str, version_type: &str) -> VersionCategory {
    classify_version_with_series(id, version_type, None)
}

/// Classifies a version using its detailed version JSON `series_id` when available.
/// Mojang uses values such as `april2024` for modern April Fools releases.
pub fn classify_version_with_series(
    id: &str,
    version_type: &str,
    series_id: Option<&str>,
) -> VersionCategory {
    let lower = id.to_ascii_lowercase();
    if series_id.is_some_and(|series| series.to_ascii_lowercase().starts_with("april"))
        || is_april_fools_id(&lower) {
        return VersionCategory::AprilFools;
    }
    if version_type == "old_beta" {
        return VersionCategory::OldBeta;
    }
    if version_type == "old_alpha" {
        return VersionCategory::OldAlpha;
    }
    if version_type == "release" {
        return VersionCategory::Release;
    }
    if version_type == "snapshot" {
        if lower.contains("-pre-") || lower.contains("-rc-")
            || lower.ends_with("-pre") || lower.ends_with("-rc")
        {
            return VersionCategory::PreRelease;
        }
        return VersionCategory::Snapshot;
    }
    VersionCategory::Experimental
}

fn is_april_fools_id(id: &str) -> bool {
    // Explicit historical IDs are safer than classifying every week-14 snapshot.
    // New joke releases can be added when Mojang publishes their `series_id` metadata.
    matches!(id,
        "20w14infinite" | "23w13a_or_b" | "24w14potato" |
        "25w14craftmine" | "26w14a"
    )
}

#[derive(Debug, Clone, Default)]
pub struct ManifestSnapshot {
    pub manifest: Option<VersionManifest>,
    pub loading: bool,
    pub error: Option<String>,
    pub last_updated: Option<String>,
}

#[derive(Clone)]
pub struct ManifestController {
    state: Arc<Mutex<ManifestSnapshot>>,
    cache_path: PathBuf,
}

impl ManifestController {
    pub fn new(data_directory: &Path) -> Self {
        let cache_path = data_directory.join("metadata").join(CACHE_FILE);
        let cached = fs::read_to_string(&cache_path).ok()
            .and_then(|contents| serde_json::from_str::<VersionManifest>(&contents).ok());
        Self {
            state: Arc::new(Mutex::new(ManifestSnapshot {
                manifest: cached,
                loading: false,
                error: None,
                last_updated: None,
            })),
            cache_path,
        }
    }

    pub fn snapshot(&self) -> ManifestSnapshot {
        self.state.lock().map(|state| state.clone()).unwrap_or_default()
    }

    pub fn refresh(&self) {
        {
            let Ok(mut state) = self.state.lock() else { return; };
            if state.loading { return; }
            state.loading = true;
            state.error = None;
        }
        let state = Arc::clone(&self.state);
        let cache_path = self.cache_path.clone();
        thread::spawn(move || {
            let result = fetch_manifest();
            if let Ok(mut guard) = state.lock() {
                guard.loading = false;
                match result {
                    Ok(manifest) => {
                        if let Some(parent) = cache_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        if let Ok(bytes) = serde_json::to_vec_pretty(&manifest) {
                            let temporary = cache_path.with_extension("json.tmp");
                            let backup = cache_path.with_extension("json.bak");
                            if fs::write(&temporary, bytes).is_ok() {
                                let had_cache = cache_path.exists();
                                let staged = !had_cache || fs::rename(&cache_path, &backup).is_ok();
                                if staged && fs::rename(&temporary, &cache_path).is_err() && had_cache {
                                    let _ = fs::rename(&backup, &cache_path);
                                }
                                if cache_path.exists() && backup.exists() {
                                    let _ = fs::remove_file(&backup);
                                }
                                if temporary.exists() {
                                    let _ = fs::remove_file(&temporary);
                                }
                            }
                        }
                        guard.manifest = Some(manifest);
                        guard.last_updated = Some("Just now".to_owned());
                        guard.error = None;
                    }
                    Err(error) => guard.error = Some(error),
                }
            }
        });
    }
}

fn fetch_manifest() -> Result<VersionManifest, String> {
    let response = reqwest::blocking::Client::builder()
        .user_agent(concat!("SaturnLauncher/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|error| format!("Could not create HTTP client: {error}"))?
        .get(MANIFEST_URL)
        .send()
        .map_err(|error| format!("Could not download Mojang version manifest: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Mojang manifest request failed: {error}"))?;
    let manifest: VersionManifest = response.json()
        .map_err(|error| format!("Mojang returned invalid version manifest JSON: {error}"))?;
    if manifest.versions.is_empty() {
        return Err("Mojang version manifest contains no versions.".to_owned());
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::{classify_version, classify_version_with_series, ManifestVersion, VersionCategory};

    #[test]
    fn classifies_standard_manifest_types() {
        assert_eq!(classify_version("1.21.4", "release"), VersionCategory::Release);
        assert_eq!(classify_version("25w10a", "snapshot"), VersionCategory::Snapshot);
        assert_eq!(classify_version("b1.7.3", "old_beta"), VersionCategory::OldBeta);
        assert_eq!(classify_version("a1.2.6", "old_alpha"), VersionCategory::OldAlpha);
    }

    #[test]
    fn keeps_april_fools_separate_from_snapshots() {
        assert_eq!(classify_version("25w14craftmine", "snapshot"), VersionCategory::AprilFools);
        assert_eq!(classify_version("20w14infinite", "snapshot"), VersionCategory::AprilFools);
        assert_eq!(classify_version("26w14a", "snapshot"), VersionCategory::AprilFools);
    }

    #[test]
    fn series_id_classifies_future_april_fools_versions() {
        assert_eq!(
            classify_version_with_series("27w14a", "snapshot", Some("april2027")),
            VersionCategory::AprilFools
        );
    }

    #[test]
    fn identifies_pre_releases_and_release_candidates() {
        assert_eq!(classify_version("1.21.5-pre-1", "snapshot"), VersionCategory::PreRelease);
        assert_eq!(classify_version("1.21.5-rc-1", "snapshot"), VersionCategory::PreRelease);
    }

    #[test]
    fn unknown_manifest_types_are_experimental() {
        assert_eq!(classify_version("custom-build", "custom"), VersionCategory::Experimental);
    }

    #[test]
    fn deserializes_manifest_version_fields() {
        let json = r#"{"id":"1.21.4","type":"release","url":"https://example.invalid/1.21.4.json","releaseTime":"2024-12-03T00:00:00+00:00","sha1":"abc","complianceLevel":1}"#;
        let version: ManifestVersion = serde_json::from_str(json).unwrap();
        assert_eq!(version.category(), VersionCategory::Release);
        assert_eq!(version.sha1.as_deref(), Some("abc"));
    }
}
