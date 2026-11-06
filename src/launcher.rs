use crate::manifest::{self, rules_allow, Artifact, Library, LibDownloads, Rule, Version};
use crate::Config;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct AssetIndex {
    objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize)]
struct AssetObject {
    hash: String,
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Saturn")
}

pub fn load_config() -> Config {
    fs::read_to_string(data_dir().join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(c: &Config) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(s) = serde_json::to_string_pretty(c) {
        let _ = fs::write(data_dir().join("config.json"), s);
    }
}

/// Unduh file hanya jika belum ada (tulis ke .part dulu, lalu rename).
fn download(url: &str, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = reqwest::blocking::get(url)?.error_for_status()?.bytes()?;
    let tmp = PathBuf::from(format!("{}.part", dest.display()));
    fs::write(&tmp, &bytes)?;
    fs::rename(&tmp, dest)?;
    Ok(())
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
fn handle_library(lib: &Library, libs: &Path, natives: &Path) -> Result<Option<PathBuf>> {
    let dl: Option<&LibDownloads> = lib.downloads.as_ref();

    // Format lama: natives dideklarasikan lewat peta classifier.
    if let Some(key) = lib.natives.as_ref().and_then(|m| m.get("windows")) {
        let key = key.replace("${arch}", "64");
        if let Some(art) = dl.and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&key)) {
            let path = libs.join(artifact_path(art, &lib.name));
            download(&art.url, &path)?;
            extract_natives(&path, natives)?;
        }
        return Ok(None);
    }

    // Format 1.19+: native sebagai library terpisah "...:natives-windows".
    if lib.name.contains(":natives-windows") {
        if let Some(art) = dl.and_then(|d| d.artifact.as_ref()) {
            let path = libs.join(artifact_path(art, &lib.name));
            download(&art.url, &path)?;
            extract_natives(&path, natives)?;
        }
        return Ok(None);
    }

    // Library biasa.
    let path = match dl.and_then(|d| d.artifact.as_ref()) {
        Some(art) => {
            let p = libs.join(artifact_path(art, &lib.name));
            download(&art.url, &p)?;
            p
        }
        None => {
            let base = lib.url.as_deref().unwrap_or("https://libraries.minecraft.net/");
            let rel = maven_path(&lib.name);
            let p = libs.join(&rel);
            download(&format!("{base}{rel}"), &p)?;
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

pub fn launch(v: &Version, cfg: &Config, progress: &mut dyn FnMut(String)) -> Result<()> {
    let root = data_dir();
    let libs = root.join("libraries");
    let assets = root.join("assets");
    let natives = root.join("natives").join(&v.id);
    let game_dir = root.join("game");
    fs::create_dir_all(&natives)?;
    fs::create_dir_all(&game_dir)?;

    progress("Mengambil info versi...".into());
    let vj = manifest::fetch_version_json(&v.url)?;

    progress("Mengunduh client...".into());
    let client = root
        .join("versions")
        .join(&vj.id)
        .join(format!("{}.jar", vj.id));
    download(&vj.downloads.client.url, &client)?;

    // Libraries + natives
    let mut cp: Vec<PathBuf> = Vec::new();
    let total = vj.libraries.len();
    for (i, lib) in vj.libraries.iter().enumerate() {
        if !rules_allow(lib.rules.as_deref()) {
            continue;
        }
        progress(format!("Library {}/{}", i + 1, total));
        if let Some(p) = handle_library(lib, &libs, &natives)? {
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

    // Assets (sequential; bisa dibuat paralel nanti)
    progress("Mengunduh asset...".into());
    let idx_path = assets
        .join("indexes")
        .join(format!("{}.json", vj.asset_index.id));
    download(&vj.asset_index.url, &idx_path)?;
    let index: AssetIndex = serde_json::from_str(&fs::read_to_string(&idx_path)?)?;
    let total = index.objects.len();
    for (i, obj) in index.objects.values().enumerate() {
        if i % 200 == 0 {
            progress(format!("Asset {i}/{total}"));
        }
        let h = &obj.hash;
        let url = format!("https://resources.download.minecraft.net/{}/{}", &h[..2], h);
        download(&url, &assets.join("objects").join(&h[..2]).join(h))?;
    }

    // Variabel placeholder argumen
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", cfg.username.clone());
    vars.insert("version_name", vj.id.clone());
    vars.insert("game_directory", game_dir.display().to_string());
    vars.insert("assets_root", assets.display().to_string());
    vars.insert("assets_index_name", vj.asset_index.id.clone());
    vars.insert("auth_uuid", "00000000-0000-0000-0000-000000000000".into());
    vars.insert("auth_access_token", "0".into());
    vars.insert("auth_xuid", "0".into());
    vars.insert("clientid", "0".into());
    vars.insert("user_type", "legacy".into());
    vars.insert("user_properties", "{}".into());
    vars.insert("version_type", v.kind.clone());
    vars.insert("natives_directory", natives.display().to_string());
    vars.insert("launcher_name", "Saturn".into());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    vars.insert("classpath", classpath.clone());

    // JVM args: pengaturan user (dari config) + argumen dari version json
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
            jvm.push(format!("-Djava.library.path={}", natives.display()));
            jvm.push("-cp".into());
            jvm.push(classpath.clone());
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

    progress("Menjalankan Minecraft...".into());
    let mut cmd = Command::new(&cfg.java_path);
    cmd.args(&jvm)
        .arg(&vj.main_class)
        .args(&game)
        .current_dir(&game_dir);
    cmd.spawn()
        .with_context(|| format!("gagal menjalankan '{}' (cek path Java)", cfg.java_path))?;
    Ok(())
}
