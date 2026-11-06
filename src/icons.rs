//! Ikon UI dari assets/icons/*.svg. build.rs merender SVG ke RGBA saat build.

use eframe::egui;

const SIZE: usize = 64;

#[derive(Clone)]
pub struct Icons {
    pub plus: egui::TextureHandle,
    pub play: egui::TextureHandle,
    pub trash: egui::TextureHandle,
    pub check: egui::TextureHandle,
    pub download: egui::TextureHandle,
    pub close: egui::TextureHandle,
}

fn load(ctx: &egui::Context, name: &str, rgba: &[u8]) -> egui::TextureHandle {
    let img = egui::ColorImage::from_rgba_unmultiplied([SIZE, SIZE], rgba);
    ctx.load_texture(name, img, egui::TextureOptions::LINEAR)
}

impl Icons {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            plus: load(ctx, "plus", include_bytes!(concat!(env!("OUT_DIR"), "/icon_plus.rgba"))),
            play: load(ctx, "play", include_bytes!(concat!(env!("OUT_DIR"), "/icon_play.rgba"))),
            trash: load(ctx, "trash", include_bytes!(concat!(env!("OUT_DIR"), "/icon_trash.rgba"))),
            check: load(ctx, "check", include_bytes!(concat!(env!("OUT_DIR"), "/icon_check.rgba"))),
            download: load(
                ctx,
                "download",
                include_bytes!(concat!(env!("OUT_DIR"), "/icon_download.rgba")),
            ),
            close: load(ctx, "close", include_bytes!(concat!(env!("OUT_DIR"), "/icon_close.rgba"))),
        }
    }
}

/// Gambar ikon dengan ukuran tertentu, untuk Button::image_and_text atau Image::new.
pub fn img(tex: &egui::TextureHandle, size: f32) -> egui::load::SizedTexture {
    egui::load::SizedTexture::new(tex.id(), egui::vec2(size, size))
}
