//! Profil (instance) yang sudah terpasang. Disimpan di %APPDATA%\Saturn\profiles.json.

use crate::launcher::data_dir;
use crate::loader::Loader;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub version_id: String,
    pub version_kind: String,
    pub version_url: String,
    /// true = folder game sendiri (mod, save, config terpisah).
    /// false = pakai folder game bersama.
    pub isolated: bool,
    /// Loader mod. Profil lama tanpa field ini dianggap vanilla.
    #[serde(default)]
    pub loader: Loader,
    /// Versi loader, kosong untuk vanilla.
    #[serde(default)]
    pub loader_version: String,
}

impl Profile {
    /// Teks singkat untuk daftar profil: "Vanilla release" atau "Fabric 0.16.9".
    pub fn loader_label(&self) -> String {
        if self.loader == Loader::Vanilla {
            self.version_kind.clone()
        } else {
            format!("{} {}", self.loader.label(), self.loader_version)
        }
    }
}

fn file() -> PathBuf {
    data_dir().join("profiles.json")
}

pub fn load() -> Vec<Profile> {
    fs::read_to_string(file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(list: &[Profile]) -> Result<()> {
    fs::create_dir_all(data_dir())?;
    fs::write(file(), serde_json::to_string_pretty(list)?)?;
    Ok(())
}

/// Folder game untuk sebuah profil.
pub fn game_dir(p: &Profile) -> PathBuf {
    if p.isolated {
        data_dir().join("instances").join(safe_name(&p.name))
    } else {
        data_dir().join("game")
    }
}

/// Hapus folder instance sebuah profil. Hanya untuk profil terisolasi;
/// folder game bersama tidak disentuh.
pub fn delete_files(p: &Profile) -> Result<()> {
    if !p.isolated {
        return Ok(());
    }
    let dir = game_dir(p);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

fn safe_name(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if t.is_empty() {
        "instance".into()
    } else {
        t
    }
}
