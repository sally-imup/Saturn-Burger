//! Unduh dan siapkan runtime Java (JRE) dari Adoptium, sesuai versi yang dibutuhkan Minecraft.

use crate::launcher::data_dir;
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

/// Pastikan JRE dengan major version tertentu tersedia; unduh dan ekstrak jika belum ada.
pub fn ensure_java(major: u32, progress: &mut dyn FnMut(String)) -> Result<PathBuf> {
    let exe = javaw_path(major);
    if exe.exists() {
        return Ok(exe);
    }

    progress(format!("Mengunduh Java {major}..."));
    let bytes = reqwest::blocking::get(adoptium_url(major))?
        .error_for_status()?
        .bytes()?;

    // Ekstrak ke folder staging dulu agar instalasi setengah jadi tidak terpakai.
    let staging = data_dir().join("runtime").join(format!("java-{major}.staging"));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    extract_zip(&bytes, &staging)?;

    // Arsip Adoptium punya satu folder root (mis. jdk-21.0.5+11-jre); pindahkan isinya.
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

    if !exe.exists() {
        return Err(anyhow!("javaw.exe tidak ditemukan setelah ekstrak"));
    }
    Ok(exe)
}

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(io::Cursor::new(bytes))?;
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
