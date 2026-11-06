//! Membuat semua aset gambar dari SVG saat build:
//! - ikon aplikasi (assets/saturn.svg): RGBA untuk jendela, ICO untuk installer,
//!   dan ikon yang di-embed ke saturn.exe (Explorer, taskbar, shortcut)
//! - ikon UI (assets/icons/*.svg): RGBA untuk tombol dan label (dipakai icons.rs)

use std::{
    env, fs,
    path::{Path, PathBuf},
};

const APP_ICON_SIZE: u32 = 256;
const UI_ICON_SIZE: u32 = 64;
const UI_ICONS: &[&str] = &["plus", "play", "trash", "check", "download", "close"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/saturn.svg");
    println!("cargo:rerun-if-changed=assets/icons");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // 1) Ikon aplikasi
    let app_rgba = render(&manifest_dir.join("assets/saturn.svg"), APP_ICON_SIZE);
    fs::write(out_dir.join("saturn_icon_256.rgba"), &app_rgba).unwrap();

    // 2) ICO untuk installer
    let icon_dir = manifest_dir.join("target").join("icons");
    fs::create_dir_all(&icon_dir).unwrap();
    let ico_path = icon_dir.join("saturn.ico");
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    let image = ico::IconImage::from_rgba_data(APP_ICON_SIZE, APP_ICON_SIZE, app_rgba.clone());
    dir.add_entry(ico::IconDirEntry::encode(&image).unwrap());
    dir.write(fs::File::create(&ico_path).unwrap()).unwrap();

    // 3) Ikon UI
    for name in UI_ICONS {
        let rgba = render(&manifest_dir.join(format!("assets/icons/{name}.svg")), UI_ICON_SIZE);
        fs::write(out_dir.join(format!("icon_{name}.rgba")), &rgba).unwrap();
    }

    // 4) Ikon dan metadata yang di-embed ke saturn.exe (hanya saat target Windows)
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.set("ProductName", "Saturn");
        res.set("FileDescription", "Saturn Minecraft Launcher");
        res.set("InternalName", "saturn");
        res.set("OriginalFilename", "saturn.exe");
        res.set("CompanyName", "Sally");
        res.set("LegalCopyright", "Copyright (c) 2026 Sally");
        res.compile().expect("gagal embed ikon dan metadata ke exe");
    }
}

/// Render SVG ke RGBA (alpha biasa) berukuran size x size.
fn render(svg_path: &Path, size: u32) -> Vec<u8> {
    let svg = fs::read(svg_path)
        .unwrap_or_else(|e| panic!("gagal membaca {}: {e}", svg_path.display()));
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())
        .unwrap_or_else(|e| panic!("gagal parse {}: {e}", svg_path.display()));

    let s = tree.size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("gagal membuat pixmap");
    let transform = resvg::tiny_skia::Transform::from_scale(
        size as f32 / s.width(),
        size as f32 / s.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny-skia menyimpan alpha premultiplied; egui dan ICO butuh alpha biasa.
    let mut rgba = pixmap.data().to_vec();
    unpremultiply(&mut rgba);
    rgba
}

fn unpremultiply(px: &mut [u8]) {
    for p in px.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}
