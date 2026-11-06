//! Pencarian dan unduhan dari Modrinth: mod, modpack, resource pack, dan shader.
//! Mod otomatis memakai versi terbaru yang cocok. Jenis lain bisa dipilih versinya.

use crate::launcher::{bytes_progress, download, Expect};
use crate::loader::Loader;
use crate::progress::Progress;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Component, Path, PathBuf},
};

const API: &str = "https://api.modrinth.com/v2";
const UA: &str = "Saturn/0.1.0 (launcher Minecraft)";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mod,
    Modpack,
    ResourcePack,
    Shader,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Mod => "Mod",
            Kind::Modpack => "Modpack",
            Kind::ResourcePack => "Resource pack",
            Kind::Shader => "Shader",
        }
    }

    fn project_type(self) -> &'static str {
        match self {
            Kind::Mod => "mod",
            Kind::Modpack => "modpack",
            Kind::ResourcePack => "resourcepack",
            Kind::Shader => "shader",
        }
    }

    /// Folder di dalam instance tempat file kind ini disimpan.
    pub fn folder(self) -> &'static str {
        match self {
            Kind::Mod => "mods",
            Kind::Modpack => "",
            Kind::ResourcePack => "resourcepacks",
            Kind::Shader => "shaderpacks",
        }
    }
}

/// Satu hasil pencarian.
#[derive(Clone, Deserialize)]
pub struct Hit {
    pub project_id: String,
    pub title: String,
    pub description: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<Hit>,
}

#[derive(Clone, Deserialize)]
struct Hashes {
    sha1: Option<String>,
}

#[derive(Clone, Deserialize)]
struct VFile {
    url: String,
    filename: String,
    size: u64,
    #[serde(default)]
    primary: bool,
    hashes: Hashes,
}

#[derive(Clone, Deserialize)]
struct VDep {
    project_id: Option<String>,
    dependency_type: String,
}

/// Satu versi sebuah project di Modrinth.
#[derive(Clone, Deserialize)]
pub struct VersionEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_number: String,
    version_type: String,
    #[serde(default)]
    game_versions: Vec<String>,
    files: Vec<VFile>,
    #[serde(default)]
    dependencies: Vec<VDep>,
}

impl VersionEntry {
    /// Nama yang ditampilkan ke user. Hanya nama versi, tanpa info loader.
    pub fn label(&self) -> String {
        if self.name.trim().is_empty() {
            self.version_number.clone()
        } else {
            self.name.clone()
        }
    }
}

fn http() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder().user_agent(UA).build()?)
}

fn get_json<T: serde::de::DeserializeOwned>(url: &str, query: &[(&str, String)]) -> Result<T> {
    Ok(http()?.get(url).query(query).send()?.error_for_status()?.json()?)
}

/// Cari project. `loader` dipakai untuk filter kategori (hanya untuk mod).
pub fn search(query: &str, kind: Kind, mc: &str, loader: Option<&str>) -> Result<Vec<Hit>> {
    let mut facets = vec![
        format!("[\"project_type:{}\"]", kind.project_type()),
        format!("[\"versions:{mc}\"]"),
    ];
    if let Some(l) = loader {
        facets.push(format!("[\"categories:{l}\"]"));
    }
    let r: SearchResponse = get_json(
        &format!("{API}/search"),
        &[
            ("query", query.to_string()),
            ("facets", format!("[{}]", facets.join(","))),
            ("limit", "25".to_string()),
        ],
    )?;
    Ok(r.hits)
}

/// Semua versi project untuk Minecraft `mc` (dan loader kalau diberikan), dari yang terbaru.
pub fn list_versions(project_id: &str, mc: &str, loader: Option<&str>) -> Result<Vec<VersionEntry>> {
    let mut q = vec![("game_versions", format!("[\"{mc}\"]"))];
    if let Some(l) = loader {
        q.push(("loaders", format!("[\"{l}\"]")));
    }
    get_json(&format!("{API}/project/{project_id}/version"), &q)
}

/// Versi terbaru (utamakan status release) untuk Minecraft dan loader tertentu.
fn latest_version(project_id: &str, mc: &str, loader: Option<&str>) -> Result<VersionEntry> {
    let mut list = list_versions(project_id, mc, loader)?;
    if list.is_empty() {
        bail!(
            "tidak ada versi untuk Minecraft {mc}{}",
            loader.map(|l| format!(" ({l})")).unwrap_or_default()
        );
    }
    let idx = list.iter().position(|v| v.version_type == "release").unwrap_or(0);
    Ok(list.swap_remove(idx))
}

fn primary_file(v: &VersionEntry) -> Result<&VFile> {
    v.files
        .iter()
        .find(|f| f.primary)
        .or(v.files.first())
        .ok_or_else(|| anyhow!("versi ini tidak punya file"))
}

/// Unduh satu file ke `dir` dengan verifikasi SHA-1 dan ukuran dari Modrinth.
fn save_file(f: &VFile, dir: &Path, progress: &mut dyn FnMut(Progress)) -> Result<PathBuf> {
    let name = Path::new(&f.filename)
        .file_name()
        .ok_or_else(|| anyhow!("nama file tidak valid: {}", f.filename))?;
    fs::create_dir_all(dir)?;
    let dest = dir.join(name);
    let label = f.filename.clone();
    download(
        &f.url,
        &dest,
        &Expect {
            sha1: f.hashes.sha1.clone(),
            size: Some(f.size),
        },
        &mut |d, t| progress(bytes_progress(&label, d, t, 0.0, 1.0)),
    )?;
    Ok(dest)
}

/// Pasang versi terbaru sebuah mod ke folder game, beserta dependensi wajibnya.
/// Mengembalikan nama file yang terpasang.
pub fn install_project(
    project_id: &str,
    mc: &str,
    loader: Option<&str>,
    game_dir: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<Vec<String>> {
    let dir = game_dir.join(Kind::Mod.folder());
    let mut queue = vec![project_id.to_string()];
    let mut seen = HashSet::new();
    let mut installed = Vec::new();

    while let Some(pid) = queue.pop() {
        if !seen.insert(pid.clone()) {
            continue;
        }
        progress(Progress::text(format!("Mencari versi {pid}...")));
        let v = latest_version(&pid, mc, loader)?;
        queue.extend(
            v.dependencies
                .iter()
                .filter(|d| d.dependency_type == "required")
                .filter_map(|d| d.project_id.clone()),
        );
        let file = primary_file(&v)?;
        let path = save_file(file, &dir, progress)?;
        installed.push(
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
    }
    Ok(installed)
}

/// Pasang satu versi resource pack atau shader yang dipilih user. Mengembalikan nama file.
pub fn install_file(
    entry: &VersionEntry,
    kind: Kind,
    game_dir: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<String> {
    let path = save_file(primary_file(entry)?, &game_dir.join(kind.folder()), progress)?;
    Ok(path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default())
}

/// Info utama dari sebuah modpack (.mrpack).
pub struct PackMeta {
    pub name: String,
    pub mc: String,
    pub loader: Loader,
    pub loader_version: String,
}

#[derive(Deserialize)]
struct Index {
    name: String,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default)]
    files: Vec<IndexFile>,
}

#[derive(Deserialize)]
struct IndexFile {
    path: String,
    hashes: Hashes,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
    env: Option<Env>,
}

#[derive(Deserialize)]
struct Env {
    client: Option<String>,
}

fn read_index(path: &Path) -> Result<Index> {
    let mut zip = zip::ZipArchive::new(fs::File::open(path)?)?;
    let entry = zip
        .by_name("modrinth.index.json")
        .context("modrinth.index.json tidak ada, file ini bukan .mrpack")?;
    Ok(serde_json::from_reader(entry)?)
}

/// Unduh file .mrpack dari versi modpack yang dipilih, lalu baca info-nya.
pub fn download_pack(entry: &VersionEntry, dir: &Path, progress: &mut dyn FnMut(Progress)) -> Result<(PathBuf, PackMeta)> {
    let path = save_file(primary_file(entry)?, dir, progress)?;
    let index = read_index(&path)?;

    let mut loader = Loader::Vanilla;
    let mut loader_version = String::new();
    for (key, l) in [
        ("fabric-loader", Loader::Fabric),
        ("quilt-loader", Loader::Quilt),
        ("neoforge", Loader::NeoForge),
        ("forge", Loader::Forge),
    ] {
        if let Some(ver) = index.dependencies.get(key) {
            loader = l;
            loader_version = ver.clone();
            break;
        }
    }
    let pack_mc = index
        .dependencies
        .get("minecraft")
        .cloned()
        .or_else(|| entry.game_versions.first().cloned())
        .unwrap_or_default();

    Ok((
        path,
        PackMeta {
            name: index.name,
            mc: pack_mc,
            loader,
            loader_version,
        },
    ))
}

/// Pasang semua file dari modpack ke folder game, lalu salin folder overrides.
pub fn extract_modpack(mrpack: &Path, game_dir: &Path, progress: &mut dyn FnMut(Progress)) -> Result<()> {
    let index = read_index(mrpack)?;
    fs::create_dir_all(game_dir)?;

    let total = index.files.len().max(1);
    for (i, f) in index.files.iter().enumerate() {
        // File yang tidak dipakai di sisi client dilewati.
        if f.env.as_ref().and_then(|e| e.client.as_deref()) == Some("unsupported") {
            continue;
        }
        if !Path::new(&f.path).components().all(|c| matches!(c, Component::Normal(_))) {
            bail!("path tidak aman di dalam modpack: {}", f.path);
        }
        let url = f
            .downloads
            .first()
            .ok_or_else(|| anyhow!("tidak ada URL untuk {}", f.path))?;
        let lo = i as f32 / total as f32;
        let hi = (i + 1) as f32 / total as f32;
        let label = format!("Modpack {}/{}", i + 1, total);
        download(
            url,
            &game_dir.join(&f.path),
            &Expect {
                sha1: f.hashes.sha1.clone(),
                size: f.file_size,
            },
            &mut |d, t| progress(bytes_progress(&label, d, t, lo, hi)),
        )?;
    }

    extract_overrides(mrpack, game_dir)
}

/// Salin isi folder `overrides/` dan `client-overrides/` ke folder game.
fn extract_overrides(mrpack: &Path, game_dir: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(mrpack)?)?;
    for i in 0..zip.len() {
        let mut e = zip.by_index(i)?;
        let Some(name) = e.enclosed_name() else { continue };
        let rel = ["overrides", "client-overrides"]
            .iter()
            .find_map(|p| name.strip_prefix(p).ok())
            .map(|r| r.to_path_buf());
        let Some(rel) = rel else { continue };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = game_dir.join(&rel);
        if e.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() {
            fs::create_dir_all(p)?;
        }
        io::copy(&mut e, &mut fs::File::create(&out)?)?;
    }
    Ok(())
}
