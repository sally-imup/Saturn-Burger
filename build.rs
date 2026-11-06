//! Membuat ikon Saturn dari assets/saturn.svg:
//! - RGBA mentah untuk ikon jendela (dipakai main.rs lewat include_bytes!)
//! - saturn.ico untuk installer (target/icons/saturn.ico)
//! - ikon yang di-embed ke saturn.exe (Explorer, taskbar, shortcut)

use std::{env, fs, path::PathBuf};

const SIZE: u32 = 256;

fn main() {
    println!("cargo:rerun-if-changed=assets/saturn.svg");
    println!("cargo:rerun-if-changed=build.rs");

    let svg = fs::read("assets/saturn.svg").expect("assets/saturn.svg tidak ditemukan");
    let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())
        .expect("gagal membaca saturn.svg");

    let size = tree.size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(SIZE, SIZE).expect("gagal membuat pixmap");
    let transform = resvg::tiny_skia::Transform::from_scale(
        SIZE as f32 / size.width(),
        SIZE as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny-skia menyimpan alpha premultiplied; ikon butuh alpha biasa.
    let mut rgba = pixmap.data().to_vec();
    unpremultiply(&mut rgba);

    // 1) RGBA mentah untuk ikon jendela
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("saturn_icon_256.rgba"), &rgba).unwrap();

    // 2) ICO untuk installer
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon_dir = manifest_dir.join("target").join("icons");
    fs::create_dir_all(&icon_dir).unwrap();
    let ico_path = icon_dir.join("saturn.ico");

    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    let image = ico::IconImage::from_rgba_data(SIZE, SIZE, rgba.clone());
    dir.add_entry(ico::IconDirEntry::encode(&image).unwrap());
    dir.write(fs::File::create(&ico_path).unwrap()).unwrap();

    // 3) Ikon yang di-embed ke saturn.exe (hanya saat target Windows)
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winres::WindowsResource::new()
            .set_icon(ico_path.to_str().unwrap())
            .compile()
            .expect("gagal embed ikon ke exe");
    }
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
