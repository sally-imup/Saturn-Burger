use crate::java;
use crate::manifest::{self, rules_allow, Artifact, LibDownloads, Library, Rule, Version, VersionJson};
use crate::profiles::{self, Profile};
use crate::progress::Progress;
use crate::Config;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    fs, io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

/// File di atas ambang ini dianggap "besar" dan progresnya ditampilkan per byte.
const BIG_FILE: u64 = 1_000_000;
const CHUNK: usize = 64 * 1024;
const ATTEMPTS: u32 = 3;

#[derive(Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
    size: Option<u64>,
}

/// Hasil memasang sebuah versi.
pub struct Installed {
    pub vj: VersionJson,
    pub classpath: String,
    pub natives: PathBuf,
    pub assets: PathBuf,
}

/// Checksum yang diharapkan untuk sebuah file. Kosong berarti tidak ada verifikasi.
pub struct Expect {
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl Expect {
    pub fn none() -> Self {
        Self { sha1: None, size: None }
    }
}

fn expect_art(a: &Artifact) -> Expect {
    Expect {
        sha1: a.sha1.clone(),
        size: a.size,
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Saturn")
}

pub fn load_config() -> Config {
    let mut cfg: Config = fs::read_to_string(data_dir().join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Buang opsi eksperimental yang sudah tersimpan di config lama dan bikin JVM gagal start.
    cfg.jvm_args = cfg
        .jvm_args
        .split_whitespace()
        .filter(|a| !a.starts_with("-XX:G1NewSizePercent"))
        .collect::<Vec<_>>()
        .join(" ");
    cfg
}

pub fn save_config(c: &Config) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(s) = serde_json::to_string_pretty(c) {
        let _ = fs::write(data_dir().join("config.json"), s);
    }
}

/// Ukuran file dalam satuan yang mudah dibaca.
pub fn fmt_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.2} GB", b / (KB * KB * KB))
    }
}

fn ratio(done: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (done as f32 / total as f32).clamp(0.0, 1.0)
    }
}

/// Teks "dulu / total (persen)", atau hanya ukuran jika total tidak diketahui.
fn size_text(done: u64, total: u64) -> String {
    if total > 0 {
        format!("{} / {} ({:.0}%)", fmt_size(done), fmt_size(total), ratio(done, total) * 100.0)
    } else {
        fmt_size(done)
    }
}

/// Progres untuk satu file: `lo..hi` adalah rentang fraksi bar utama yang dipakai file ini.
pub fn bytes_progress(label: &str, done: u64, total: u64, lo: f32, hi: f32) -> Progress {
    let text = format!("{label} — {}", size_text(done, total));
    if total > 0 {
        Progress::at(text, lo + (hi - lo) * ratio(done, total))
    } else {
        Progress::text(text)
    }
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Cek apakah file sudah utuh: ukuran dan SHA-1 harus cocok jika diketahui.
pub fn verify(path: &Path, expect: &Expect) -> Result<bool> {
    if let Some(sz) = expect.size {
        if fs::metadata(path)?.len() != sz {
            return Ok(false);
        }
    }
    if let Some(want) = &expect.sha1 {
        let mut f = io::BufReader::new(fs::File::open(path)?);
        let mut hasher = Sha1::new();
        let mut buf = vec![0u8; CHUNK];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        return Ok(hex(&hasher.finalize()).eq_ignore_ascii_case(want));
    }
    Ok(true)
}

/// Satu kali percobaan unduh: tulis ke .part sambil menghitung SHA-1, lalu verifikasi.
fn try_download(url: &str, dest: &Path, expect: &Expect, on_bytes: &mut dyn FnMut(u64, u64)) -> Result<()> {
    let mut resp = reqwest::blocking::get(url)?.error_for_status()?;
    let total = expect.size.or(resp.content_length()).unwrap_or(0);

    let tmp = PathBuf::from(format!("{}.part", dest.display()));
    let mut out = io::BufWriter::new(fs::File::create(&tmp)?);
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; CHUNK];
    let mut done: u64 = 0;

    on_bytes(0, total);
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        done += n as u64;
        on_bytes(done, total);
    }
    out.flush()?;
    drop(out);

    if let Some(sz) = expect.size {
        if done != sz {
            bail!("ukuran file tidak cocok (dapat {done} byte, seharusnya {sz})");
        }
    }
    if let Some(want) = &expect.sha1 {
        let got = hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(want) {
            bail!("checksum SHA-1 tidak cocok (file rusak saat diunduh)");
        }
    }
    fs::rename(&tmp, dest)?;
    Ok(())
}

/// Pastikan file ada dan utuh. File yang sudah ada tapi rusak dihapus lalu diunduh ulang.
/// Unduhan dicoba sampai 3 kali sebelum menyerah.
pub fn download(url: &str, dest: &Path, expect: &Expect, on_bytes: &mut dyn FnMut(u64, u64)) -> Result<()> {
    if dest.exists() {
        if verify(dest, expect)? {
            return Ok(());
        }
        fs::remove_file(dest).with_context(|| format!("gagal menghapus file rusak {}", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=ATTEMPTS {
        match try_download(url, dest, expect, on_bytes) {
            Ok(()) => return Ok(()),
            Err(e) => {
                let _ = fs::remove_file(format!("{}.part", dest.display()));
                last_err = Some(e);
                if attempt < ATTEMPTS {
                    thread::sleep(Duration::from_secs(attempt as u64));
                }
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| anyhow!("unduhan gagal"))
        .context(format!("gagal mengunduh {url} setelah {ATTEMPTS} percobaan")))
}

/// Path maven dari "group:artifact:version[:classifier]".
fn maven_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return name.to_string();
    }
    let group = parts[0].replace('.', "/");
    let (artifact, version) = (parts[1], parts[2]);
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.jar")
}

fn artifact_path(art: &Artifact, name: &str) -> String {
    art.path.clone().unwrap_or_else(|| maven_path(name))
}

fn extract_natives(jar: &Path, dest: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(jar)?)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let Some(rel) = f.enclosed_name() else { continue };
        if f.is_dir() || rel.starts_with("META-INF") {
            continue;
        }
        let out = dest.join(rel);
        if let Some(p) = out.parent() {
            fs::create_dir_all(p)?;
        }
        let mut w = fs::File::create(&out)?;
        io::copy(&mut f, &mut w)?;
    }
    Ok(())
}

/// Unduh satu library. Mengembalikan path jar jika library masuk classpath,
/// None jika library ini hanya berisi native (sudah diekstrak).
fn handle_library(lib: &Library, libs: &Path, natives: &Path, on_bytes: &mut dyn FnMut(u64, u64)) -> Result<Option<PathBuf>> {
    let dl: Option<&LibDownloads> = lib.downloads.as_ref();

    // Format lama: natives dideklarasikan lewat peta classifier.
    if let Some(key) = lib.natives.as_ref().and_then(|m| m.get("windows")) {
        let key = key.replace("${arch}", "64");
        if let Some(art) = dl.and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&key)) {
            let path = libs.join(artifact_path(art, &lib.name));
            download(&art.url, &path, &expect_art(art), on_bytes)?;
            extract_natives(&path, natives)?;
        }
        return Ok(None);
    }

    // Format 1.19+: native sebagai library terpisah "...:natives-windows".
    if lib.name.contains(":natives-windows") {
        if let Some(art) = dl.and_then(|d| d.artifact.as_ref()) {
            let path = libs.join(artifact_path(art, &lib.name));
            download(&art.url, &path, &expect_art(art), on_bytes)?;
            extract_natives(&path, natives)?;
        }
        return Ok(None);
    }

    // Library biasa.
    let path = match dl.and_then(|d| d.artifact.as_ref()) {
        Some(art) => {
            let p = libs.join(artifact_path(art, &lib.name));
            download(&art.url, &p, &expect_art(art), on_bytes)?;
            p
        }
        None => {
            let base = lib.url.as_deref().unwrap_or("https://libraries.minecraft.net/");
            let rel = maven_path(&lib.name);
            let p = libs.join(&rel);
            download(&format!("{base}{rel}"), &p, &Expect::none(), on_bytes)?;
            p
        }
    };
    Ok(Some(path))
}

fn subst(s: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out
}

/// Ubah daftar argumen modern (string atau objek dengan rules) menjadi Vec<String>.
fn collect_args(list: &[Value], vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in list {
        match item {
            Value::String(s) => out.push(subst(s, vars)),
            Value::Object(_) => {
                let rules: Option<Vec<Rule>> = item
                    .get("rules")
                    .and_then(|r| serde_json::from_value(r.clone()).ok());
                if !rules_allow(rules.as_deref()) {
                    continue;
                }
                match item.get("value") {
                    Some(Value::String(s)) => out.push(subst(s, vars)),
                    Some(Value::Array(a)) => {
                        for x in a {
                            if let Some(s) = x.as_str() {
                                out.push(subst(s, vars));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

/// Pasang semua file versi ini (client, library, natives, asset). Aman dijalankan ulang:
/// file yang sudah utuh dilewati, file yang rusak diunduh ulang.
/// Pembagian progres: client 0–10%, library 10–50%, asset 50–100%.
pub fn install(v: &Version, progress: &mut dyn FnMut(Progress)) -> Result<Installed> {
    let root = data_dir();
    let libs = root.join("libraries");
    let assets = root.join("assets");
    let natives = root.join("natives").join(&v.id);
    fs::create_dir_all(&natives)?;

    progress(Progress::text("Mengambil info versi..."));
    let vj = manifest::fetch_version_json(&v.url)?;

    // Client
    let client = root
        .join("versions")
        .join(&vj.id)
        .join(format!("{}.jar", vj.id));
    download(
        &vj.downloads.client.url,
        &client,
        &expect_art(&vj.downloads.client),
        &mut |d, t| progress(bytes_progress("Mengunduh client", d, t, 0.0, 0.10)),
    )?;

    // Library
    let active: Vec<&Library> = vj
        .libraries
        .iter()
        .filter(|l| rules_allow(l.rules.as_deref()))
        .collect();
    let total = active.len().max(1);
    let mut cp: Vec<PathBuf> = Vec::new();
    for (i, lib) in active.iter().enumerate() {
        let lo = 0.10 + 0.40 * (i as f32 / total as f32);
        let hi = 0.10 + 0.40 * ((i + 1) as f32 / total as f32);
        let label = format!("Library {}/{}", i + 1, total);
        let entry = handle_library(lib, &libs, &natives, &mut |d, t| {
            progress(bytes_progress(&label, d, t, lo, hi))
        })?;
        if let Some(p) = entry {
            cp.push(p);
        }
    }
    cp.push(client);
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = cp
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(sep);

    // Asset index
    let idx_path = assets
        .join("indexes")
        .join(format!("{}.json", vj.asset_index.id));
    download(
        &vj.asset_index.url,
        &idx_path,
        &Expect {
            sha1: vj.asset_index.sha1.clone(),
            size: vj.asset_index.size,
        },
        &mut |d, t| progress(bytes_progress("Mengunduh daftar asset", d, t, 0.40, 0.50)),
    )?;
    let index: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;

    // Asset objects: nama file = SHA-1 isinya, jadi bisa langsung diverifikasi.
    let total = index.objects.len().max(1);
    for (i, obj) in index.objects.values().enumerate() {
        let lo = 0.50 + 0.50 * (i as f32 / total as f32);
        let hi = 0.50 + 0.50 * ((i + 1) as f32 / total as f32);
        if i % 50 == 0 {
            progress(Progress::at(format!("Memeriksa asset {i}/{total}"), lo));
        }
        let h = &obj.hash;
        let url = format!("https://resources.download.minecraft.net/{}/{}", &h[..2], h);
        let path = assets.join("objects").join(&h[..2]).join(h);
        let expect = Expect {
            sha1: Some(h.clone()),
            size: obj.size,
        };
        let label = format!("Asset {}/{}", i + 1, total);
        download(&url, &path, &expect, &mut |d, t| {
            // File kecil terlalu cepat berganti; progres per byte hanya untuk file besar.
            if t >= BIG_FILE {
                progress(bytes_progress(&label, d, t, lo, hi));
            }
        })?;
    }
    progress(Progress::at(format!("Asset {total}/{total} siap"), 1.0));

    Ok(Installed {
        vj,
        classpath,
        natives,
        assets,
    })
}

/// Jalankan sebuah profil. Pasang file yang belum ada, lalu start JVM.
pub fn launch(p: &Profile, cfg: &Config, progress: &mut dyn FnMut(Progress)) -> Result<()> {
    let v = Version {
        id: p.version_id.clone(),
        kind: p.version_kind.clone(),
        url: p.version_url.clone(),
    };
    let game_dir = profiles::game_dir(p);
    fs::create_dir_all(&game_dir)?;

    let inst = install(&v, progress)?;
    let vj = &inst.vj;

    // Variabel placeholder argumen
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", cfg.username.clone());
    vars.insert("version_name", vj.id.clone());
    vars.insert("game_directory", game_dir.display().to_string());
    vars.insert("assets_root", inst.assets.display().to_string());
    vars.insert("assets_index_name", vj.asset_index.id.clone());
    vars.insert("auth_uuid", "00000000-0000-0000-0000-000000000000".into());
    vars.insert("auth_access_token", "0".into());
    vars.insert("auth_xuid", "0".into());
    vars.insert("clientid", "0".into());
    vars.insert("user_type", "legacy".into());
    vars.insert("user_properties", "{}".into());
    vars.insert("version_type", v.kind.clone());
    vars.insert("natives_directory", inst.natives.display().to_string());
    vars.insert("launcher_name", "Saturn".into());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    vars.insert("classpath", inst.classpath.clone());

    // JVM args: pengaturan user + argumen dari version json
    let user_jvm: Vec<String> = cfg.jvm_args.split_whitespace().map(String::from).collect();
    let (jvm, game): (Vec<String>, Vec<String>) = match &vj.arguments {
        Some(a) => {
            let mut jvm = user_jvm;
            jvm.extend(collect_args(&a.jvm, &vars));
            (jvm, collect_args(&a.game, &vars))
        }
        None => {
            // Format lama (sebelum 1.13)
            let mut jvm = user_jvm;
            jvm.push(format!("-Djava.library.path={}", inst.natives.display()));
            jvm.push("-cp".into());
            jvm.push(inst.classpath.clone());
            let game = vj
                .minecraft_arguments
                .as_deref()
                .unwrap_or("")
                .split_whitespace()
                .map(|s| subst(s, &vars))
                .collect::<Vec<String>>();
            (jvm, game)
        }
    };

    // Pilih Java: otomatis (unduh sesuai versi) atau path manual dari pengaturan.
    let major = vj.java_version.as_ref().map(|j| j.major_version).unwrap_or(8);
    let java_exe = if cfg.java_path.trim().is_empty() || cfg.java_path.trim().eq_ignore_ascii_case("auto") {
        java::ensure_java(major, progress)?.display().to_string()
    } else {
        cfg.java_path.clone()
    };

    progress(Progress::text("Menjalankan Minecraft..."));
    let mut cmd = Command::new(&java_exe);
    cmd.args(&jvm)
        .arg(&vj.main_class)
        .args(&game)
        .current_dir(&game_dir);
    cmd.spawn()
        .with_context(|| format!("gagal menjalankan '{}' (cek path Java)", java_exe))?;
    Ok(())
}
