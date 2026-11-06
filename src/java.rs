//! Unduh dan siapkan runtime Java (JRE) sesuai versi yang dibutuhkan Minecraft.
//! Sumber: runtime ramping dari release GitHub, lalu fallback ke Adoptium.

use crate::launcher::{bytes_progress, data_dir, download, Expect};
use crate::progress::Progress;
use anyhow::{anyhow, Result};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn adoptium_url(major: u32) -> String {
    format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/windows/x64/jre/hotspot/normal/eclipse")
}

pub fn runtime_dir(major: u32) -> PathBuf {
    data_dir().join("runtime").join(format!("java-{major}"))
}

pub fn javaw_path(major: u32) -> PathBuf {
    runtime_dir(major).join("bin").join("javaw.exe")
}

fn staging_dir(major: u32) -> PathBuf {
    data_dir().join("runtime").join(format!("java-{major}.staging"))
}

/// Pastikan JRE dengan major version tertentu tersedia; unduh dan ekstrak jika belum ada.
pub fn ensure_java(major: u32, progress: &mut dyn FnMut(Progress)) -> Result<PathBuf> {
    let exe = javaw_path(major);
    if exe.exists() {
        return Ok(exe);
    }

    let zip_path = data_dir().join("runtime").join(format!("java-{major}.zip"));
    let slim = format!(
        "https://github.com/{}/releases/latest/download/saturn-jre-{major}-win64.zip",
        crate::update::REPO
    );
    let sources = [slim, adoptium_url(major)];

    let mut last_err = None;
    for url in &sources {
        match fetch_runtime(url, major, &zip_path, progress) {
            Ok(()) => {
                last_err = None;
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    if let Some(e) = last_err {
        return Err(e);
    }

    if !exe.exists() {
        return Err(anyhow!("javaw.exe tidak ditemukan setelah ekstrak"));
    }
    Ok(exe)
}

/// Unduh arsip runtime dari `url`, ekstrak ke staging, lalu pindahkan ke folder runtime.
fn fetch_runtime(url: &str, major: u32, zip_path: &Path, progress: &mut dyn FnMut(Progress)) -> Result<()> {
    // Arsip lama mungkin rusak; selalu mulai dari unduhan baru.
    let _ = fs::remove_file(zip_path);
    let label = format!("Mengunduh Java {major}");
    download(url, zip_path, &Expect::none(), &mut |d, t| {
        progress(bytes_progress(&label, d, t, 0.0, 1.0))
    })?;

    let staging = staging_dir(major);
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;

    let extracted = extract_zip(zip_path, &staging);
    let _ = fs::remove_file(zip_path);
    if let Err(e) = extracted {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    // Arsip punya satu folder root (mis. jdk-21.0.5+11-jre); pindahkan isinya.
    let inner = fs::read_dir(&staging)?
        .next()
        .ok_or_else(|| anyhow!("arsip Java kosong"))??
        .path();

    let target = runtime_dir(major);
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::rename(&inner, &target)?;
    fs::remove_dir_all(&staging)?;
    Ok(())
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(zip_path)?)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let Some(rel) = f.enclosed_name() else { continue };
        let out = dest.join(rel);
        if f.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() {
            fs::create_dir_all(p)?;
        }
        let mut w = fs::File::create(&out)?;
        io::copy(&mut f, &mut w)?;
    }
    Ok(())
}
