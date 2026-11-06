#![windows_subsystem = "windows"]

mod icons;
mod java;
mod launcher;
mod manifest;
mod profiles;
mod update;

use eframe::egui;
use manifest::Version;
use profiles::Profile;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x7C, 0x6C, 0xF0);
const CARD: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x21, 0x2D);
const CARD_SELECTED: egui::Color32 = egui::Color32::from_rgb(0x2D, 0x28, 0x56);
const MUTED: egui::Color32 = egui::Color32::from_rgb(0x9A, 0xA0, 0xB4);

/// Ikon jendela, dibuat oleh build.rs dari assets/saturn.svg.
const ICON_RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/saturn_icon_256.rgba"));

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub username: String,
    /// "auto" = unduh Java yang sesuai otomatis; atau isi path ke javaw.exe sendiri.
    pub java_path: String,
    pub jvm_args: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: "Player".into(),
            java_path: "auto".into(),
            // Hanya flag standar. Opsi eksperimental (mis. G1NewSizePercent) membuat JVM gagal start.
            jvm_args: "-Xmx2G -XX:+UseG1GC -XX:MaxGCPauseMillis=50".into(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Filter {
    Release,
    Snapshot,
    Old,
    All,
}

impl Filter {
    fn label(self) -> &'static str {
        match self {
            Filter::Release => "Release",
            Filter::Snapshot => "Snapshot",
            Filter::Old => "Old",
            Filter::All => "Semua",
        }
    }

    fn matches(self, kind: &str) -> bool {
        match self {
            Filter::Release => kind == "release",
            Filter::Snapshot => kind == "snapshot",
            Filter::Old => kind == "old_beta" || kind == "old_alpha",
            Filter::All => true,
        }
    }
}

enum Msg {
    Manifest(Result<Vec<Version>, String>),
    Status(String),
    Done(Result<(), String>),
    UpdateAvailable(update::Update),
    Installed(Result<Profile, String>),
}

struct App {
    icons: icons::Icons,
    cfg: Config,
    /// Profil yang sudah terpasang (ditampilkan di daftar utama).
    profiles: Vec<Profile>,
    /// Indeks profil yang dipilih.
    selected: Option<usize>,
    /// Semua versi dari manifest Mojang (dipakai di popup Tambah).
    versions: Vec<Version>,
    update: Option<update::Update>,
    status: String,
    busy: bool,

    // State popup "Tambah versi"
    show_add: bool,
    /// Indeks profil yang menunggu konfirmasi hapus.
    confirm_remove: Option<usize>,
    add_filter: Filter,
    add_search: String,
    add_selected: Option<usize>,
    add_name: String,
    add_isolated: bool,

    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    fn new(icons: icons::Icons) -> Self {
        let (tx, rx) = channel();
        let cfg = launcher::load_config();
        let profiles = profiles::load();
        let selected = if profiles.is_empty() { None } else { Some(0) };

        let t = tx.clone();
        std::thread::spawn(move || {
            let r = manifest::fetch_manifest().map_err(|e| format!("{e:#}"));
            let _ = t.send(Msg::Manifest(r));
        });

        // Cek update di background; kalau gagal (offline, dll.) diam saja.
        let t = tx.clone();
        std::thread::spawn(move || {
            if let Ok(Some(u)) = update::check() {
                let _ = t.send(Msg::UpdateAvailable(u));
            }
        });

        Self {
            icons,
            cfg,
            profiles,
            selected,
            versions: Vec::new(),
            update: None,
            status: "Memuat daftar versi...".into(),
            busy: true,
            show_add: false,
            confirm_remove: None,
            add_filter: Filter::Release,
            add_search: String::new(),
            add_selected: None,
            add_name: String::new(),
            add_isolated: true,
            tx,
            rx,
        }
    }

    fn start_install(&mut self, v: Version, name: String, isolated: bool) {
        self.busy = true;
        self.status = format!("Memasang {name}...");
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let ptx = tx.clone();
            let mut progress = |s: String| {
                let _ = ptx.send(Msg::Status(s));
            };
            let profile = Profile {
                name,
                version_id: v.id.clone(),
                version_kind: v.kind.clone(),
                version_url: v.url.clone(),
                isolated,
            };
            let r = launcher::install(&v, &mut progress)
                .map(|_| profile)
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Installed(r));
        });
    }

    fn play(&mut self, idx: usize) {
        let p = self.profiles[idx].clone();
        let cfg = self.cfg.clone();
        launcher::save_config(&cfg);

        self.busy = true;
        self.status = format!("Menjalankan {}...", p.name);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let mut progress = |s: String| {
                let _ = progress_tx.send(Msg::Status(s));
            };
            let res = launcher::launch(&p, &cfg, &mut progress).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Done(res));
        });
    }

    /// Hapus profil dari daftar beserta folder instance-nya (jika terisolasi).
    fn remove(&mut self, idx: usize) {
        let p = self.profiles.remove(idx);
        let _ = profiles::save(&self.profiles);
        self.selected = None;
        match profiles::delete_files(&p) {
            Ok(()) if p.isolated => {
                self.status = format!("Profil '{}' dan instance-nya dihapus", p.name);
            }
            Ok(()) => {
                self.status = format!("Profil '{}' dihapus (folder game bersama tetap ada)", p.name);
            }
            Err(e) => {
                self.status = format!("Profil dihapus, tapi gagal menghapus folder: {e:#}");
            }
        }
    }

    fn start_update(&mut self, u: update::Update) {
        self.status = format!("Mengunduh Saturn {}...", u.version);
        let tx = self.tx.clone();
        std::thread::spawn(move || match update::download_and_run(&u) {
            // Installer mengambil alih; tutup launcher supaya file exe bisa diganti.
            Ok(()) => std::process::exit(0),
            Err(e) => {
                let _ = tx.send(Msg::Status(format!("Gagal update: {e:#}")));
            }
        });
    }

    /// Isi popup "Tambah versi". Dirender di dalam jendela ini, bukan jendela baru.
    fn add_dialog(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for f in [Filter::Release, Filter::Snapshot, Filter::Old, Filter::All] {
                if ui.selectable_label(self.add_filter == f, f.label()).clicked() {
                    self.add_filter = f;
                }
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.add_search)
                .hint_text("Cari versi...")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);

        let query = self.add_search.trim().to_lowercase();
        let filtered: Vec<usize> = self
            .versions
            .iter()
            .enumerate()
            .filter(|(_, v)| self.add_filter.matches(&v.kind) && v.id.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();

        let mut picked: Option<usize> = None;
        egui::ScrollArea::vertical()
            .max_height(240.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for i in filtered {
                    let v = &self.versions[i];
                    let installed = self.profiles.iter().any(|p| p.version_id == v.id);
                    let label = format!("{}   [{}]", v.id, v.kind);
                    ui.horizontal(|ui| {
                        if installed {
                            ui.add(egui::Image::new(icons::img(&self.icons.check, 14.0)));
                        }
                        if ui.selectable_label(self.add_selected == Some(i), label).clicked() {
                            picked = Some(i);
                        }
                    });
                }
            });
        if let Some(i) = picked {
            self.add_selected = Some(i);
            self.add_name = self.versions[i].id.clone();
        }

        ui.separator();
        ui.label("Nama profil");
        ui.text_edit_singleline(&mut self.add_name);
        ui.checkbox(
            &mut self.add_isolated,
            "Isolated instance (mod, save, dan config terpisah)",
        );

        let name = self.add_name.trim().to_string();
        let name_taken = self.profiles.iter().any(|p| p.name == name);
        if name_taken {
            ui.colored_label(egui::Color32::LIGHT_RED, "Nama profil sudah dipakai");
        }
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            let can_install =
                !self.busy && self.add_selected.is_some() && !name.is_empty() && !name_taken;
            if ui.add_enabled(can_install, egui::Button::new("Install")).clicked() {
                if let Some(i) = self.add_selected {
                    let v = self.versions[i].clone();
                    let isolated = self.add_isolated;
                    self.start_install(v, name.clone(), isolated);
                    self.show_add = false;
                }
            }
            if ui
                .add(egui::Button::image_and_text(icons::img(&self.icons.close, 14.0), "Batal"))
                .clicked()
            {
                self.show_add = false;
            }
        });
    }
}

fn apply_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(0x12, 0x14, 0x1C);
    v.window_fill = egui::Color32::from_rgb(0x18, 0x1B, 0x25);
    v.extreme_bg_color = egui::Color32::from_rgb(0x0C, 0x0E, 0x14);
    v.selection.bg_fill = ACCENT;
    v.hyperlink_color = ACCENT;
    v.window_rounding = egui::Rounding::same(12.0);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.rounding = egui::Rounding::same(8.0);
    }
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x2A, 0x2E, 0x3D);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(0x2A, 0x2E, 0x3D);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    ctx.set_style(style);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Manifest(Ok(v)) => {
                    self.versions = v;
                    self.status = if self.profiles.is_empty() {
                        "Klik + Tambah untuk memasang versi".into()
                    } else {
                        "Siap".into()
                    };
                    self.busy = false;
                }
                Msg::Manifest(Err(e)) => {
                    self.status = format!("Gagal memuat manifest: {e}");
                    self.busy = false;
                }
                Msg::Status(s) => self.status = s,
                Msg::Done(Ok(())) => {
                    self.status = "Game berjalan".into();
                    self.busy = false;
                }
                Msg::Done(Err(e)) => {
                    self.status = format!("Error: {e}");
                    self.busy = false;
                }
                Msg::UpdateAvailable(u) => self.update = Some(u),
                Msg::Installed(Ok(p)) => {
                    self.profiles.push(p);
                    let _ = profiles::save(&self.profiles);
                    self.selected = Some(self.profiles.len() - 1);
                    self.status = "Versi berhasil dipasang".into();
                    self.busy = false;
                }
                Msg::Installed(Err(e)) => {
                    self.status = format!("Gagal memasang: {e}");
                    self.busy = false;
                }
            }
        }
        if self.busy {
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("Saturn");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tambah = egui::Button::image_and_text(icons::img(&self.icons.plus, 14.0), "Tambah");
                    if ui.add_enabled(!self.busy, tambah).clicked() {
                        self.show_add = true;
                        self.add_selected = None;
                    }
                });
            });
            if let Some(u) = self.update.clone() {
                ui.horizontal(|ui| {
                    ui.colored_label(ACCENT, format!("Versi baru {} tersedia", u.version));
                    let update_btn =
                        egui::Button::image_and_text(icons::img(&self.icons.download, 14.0), "Update");
                    if ui.add(update_btn).clicked() {
                        self.start_update(u);
                    }
                });
            }
            ui.add_space(8.0);
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(4.0);
            egui::CollapsingHeader::new("Pengaturan")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("settings")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Username");
                            ui.text_edit_singleline(&mut self.cfg.username);
                            ui.end_row();

                            ui.label("Java");
                            ui.text_edit_singleline(&mut self.cfg.java_path);
                            ui.end_row();

                            ui.label("JVM args");
                            ui.text_edit_singleline(&mut self.cfg.jvm_args);
                            ui.end_row();
                        });
                    ui.label(
                        egui::RichText::new("Java: auto = unduh otomatis sesuai versi, atau isi path javaw.exe")
                            .color(MUTED)
                            .small(),
                    );
                });
            ui.label(egui::RichText::new(self.status.as_str()).color(MUTED));
            ui.add_space(4.0);
        });

        let mut play_req: Option<usize> = None;
        let mut remove_req: Option<usize> = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.profiles.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading("Belum ada versi");
                    ui.label(
                        egui::RichText::new("Klik + Tambah untuk memasang versi Minecraft")
                            .color(MUTED),
                    );
                });
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, p) in self.profiles.iter().enumerate() {
                    let fill = if self.selected == Some(i) { CARD_SELECTED } else { CARD };
                    egui::Frame::none()
                        .fill(fill)
                        .rounding(10.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(p.name.as_str());
                                    let badge = if p.isolated { "Terisolasi" } else { "Bersama" };
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Minecraft {}  |  {}  |  {}",
                                            p.version_id, p.version_kind, badge
                                        ))
                                        .color(MUTED)
                                        .small(),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let hapus = egui::Button::image_and_text(
                                            icons::img(&self.icons.trash, 14.0),
                                            "Hapus",
                                        );
                                        if ui.add(hapus).clicked() {
                                            remove_req = Some(i);
                                        }
                                        if ui
                                            .add_enabled(
                                                !self.busy,
                                                egui::Button::image_and_text(
                                                    icons::img(&self.icons.play, 14.0),
                                                    "Main",
                                                ),
                                            )
                                            .clicked()
                                        {
                                            play_req = Some(i);
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        });

        if let Some(i) = play_req {
            self.selected = Some(i);
            self.play(i);
        }
        if let Some(i) = remove_req {
            self.confirm_remove = Some(i);
        }

        // Konfirmasi hapus profil
        if let Some(i) = self.confirm_remove {
            if i >= self.profiles.len() {
                self.confirm_remove = None;
            } else {
                let name = self.profiles[i].name.clone();
                let isolated = self.profiles[i].isolated;
                let mut open = true;
                let mut confirmed = false;
                let mut cancelled = false;
                egui::Window::new("Hapus profil")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(format!("Hapus profil '{name}'?"));
                        if isolated {
                            ui.colored_label(
                                egui::Color32::LIGHT_RED,
                                "Folder instance (mod, save, config) ikut dihapus permanen.",
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Folder game bersama tidak ikut dihapus.")
                                    .color(MUTED),
                            );
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::image_and_text(
                                    icons::img(&self.icons.trash, 14.0),
                                    egui::RichText::new("Hapus").color(egui::Color32::LIGHT_RED),
                                ))
                                .clicked()
                            {
                                confirmed = true;
                            }
                            if ui.button("Batal").clicked() {
                                cancelled = true;
                            }
                        });
                    });
                if confirmed {
                    self.remove(i);
                    self.confirm_remove = None;
                } else if cancelled || !open {
                    self.confirm_remove = None;
                }
            }
        }

        if self.show_add {
            let mut open = true;
            egui::Window::new("Tambah versi")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([440.0, 470.0])
                .show(ctx, |ui| self.add_dialog(ui));
            if !open {
                self.show_add = false;
            }
        }
    }
}

fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: ICON_RGBA.to_vec(),
        width: 256,
        height: 256,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 680.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Saturn",
        options,
        Box::new(|cc| {
            apply_style(&cc.egui_ctx);
            let icons = icons::Icons::new(&cc.egui_ctx);
            Ok(Box::new(App::new(icons)))
        }),
    )
}
