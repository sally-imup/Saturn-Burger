#![windows_subsystem = "windows"]

mod icons;
mod java;
mod launcher;
mod loader;
mod manifest;
mod modrinth;
mod profiles;
mod progress;
mod update;

use eframe::egui;
use loader::Loader;
use manifest::Version;
use modrinth::{Hit, Kind};
use profiles::Profile;
use progress::Progress;
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

/// Preset flag JVM. Opsi eksperimental (G1NewSizePercent dll.) butuh UnlockExperimentalVMOptions.
const PRESETS: &[(&str, &str)] = &[
    (
        "Hemat memori",
        "-Xms512M -Xmx2G -XX:+UseG1GC -XX:+UseStringDeduplication -XX:MaxGCPauseMillis=100",
    ),
    (
        "Seimbang",
        "-Xms1G -Xmx4G -XX:+UseG1GC -XX:+UseStringDeduplication -XX:MaxGCPauseMillis=50 -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20",
    ),
    (
        "Performa",
        "-Xms2G -Xmx6G -XX:+UseG1GC -XX:+UseStringDeduplication -XX:MaxGCPauseMillis=50 -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=30 -XX:G1ReservePercent=20 -XX:+DisableExplicitGC",
    ),
];

fn preset_name(args: &str) -> &'static str {
    PRESETS
        .iter()
        .find(|(_, a)| *a == args)
        .map(|(n, _)| *n)
        .unwrap_or("Kustom")
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
    Status(Progress),
    Done(Result<(), String>),
    UpdateAvailable(update::Update),
    Installed(Result<Profile, String>),
    /// Hasil daftar versi loader untuk (loader, versi Minecraft).
    LoaderVersions {
        key: (Loader, String),
        result: Result<Vec<String>, String>,
    },
    Search(Result<Vec<Hit>, String>),
    ModDone(Result<String, String>),
}

/// Aksi yang diminta dari jendela Mod dalam satu frame.
enum ModAction {
    Search,
    Install(Hit),
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
    /// Progres bar 0.0 sampai 1.0 saat sedang mengunduh.
    progress: Option<f32>,
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
    add_loader: Loader,
    /// Versi loader yang tersedia untuk pilihan saat ini (terbaru di depan).
    loader_versions: Vec<String>,
    add_loader_version: String,
    /// (loader, versi Minecraft) yang terakhir diminta daftarnya.
    loader_key: Option<(Loader, String)>,
    loader_error: Option<String>,

    // State jendela "Mod" per instance
    /// Indeks profil yang sedang dibuka jendela Mod-nya.
    mods_for: Option<usize>,
    mod_kind: Kind,
    mod_query: String,
    mod_results: Vec<Hit>,
    mod_busy: bool,
    mod_status: String,

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
            progress: None,
            busy: true,
            show_add: false,
            confirm_remove: None,
            add_filter: Filter::Release,
            add_search: String::new(),
            add_selected: None,
            add_name: String::new(),
            add_isolated: true,
            add_loader: Loader::Vanilla,
            loader_versions: Vec::new(),
            add_loader_version: String::new(),
            loader_key: None,
            loader_error: None,
            mods_for: None,
            mod_kind: Kind::Mod,
            mod_query: String::new(),
            mod_results: Vec::new(),
            mod_busy: false,
            mod_status: String::new(),
            tx,
            rx,
        }
    }

    fn start_install(&mut self, v: Version, name: String, isolated: bool, loader: Loader, loader_version: String) {
        self.busy = true;
        self.status = format!("Memasang {name}...");
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let ptx = tx.clone();
            let mut progress = |p: Progress| {
                let _ = ptx.send(Msg::Status(p));
            };
            let profile = Profile {
                name,
                version_id: v.id.clone(),
                version_kind: v.kind.clone(),
                version_url: v.url.clone(),
                isolated,
                loader,
                loader_version,
            };
            let r = launcher::install_profile(&v, &profile, &mut progress)
                .map(|_| profile)
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Installed(r));
        });
    }

    /// Ambil daftar versi loader untuk Minecraft tertentu di background.
    fn request_loader_versions(&mut self, loader: Loader, mc: String) {
        self.loader_key = Some((loader, mc.clone()));
        self.loader_versions.clear();
        self.add_loader_version.clear();
        self.loader_error = None;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = loader::list_versions(loader, &mc).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::LoaderVersions {
                key: (loader, mc),
                result,
            });
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
            let mut progress = |p: Progress| {
                let _ = progress_tx.send(Msg::Status(p));
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
        self.mods_for = None;
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
                let _ = tx.send(Msg::Status(Progress::text(format!("Gagal update: {e:#}"))));
            }
        });
    }

    /// Buka jendela Mod untuk profil `i` dan langsung cari.
    fn open_mods(&mut self, i: usize) {
        self.mods_for = Some(i);
        self.mod_query.clear();
        self.mod_results.clear();
        self.start_search();
    }

    /// Cari di Modrinth sesuai jenis yang dipilih dan versi profil yang dibuka.
    fn start_search(&mut self) {
        let Some(idx) = self.mods_for else { return };
        let Some(p) = self.profiles.get(idx).cloned() else { return };
        let kind = self.mod_kind;
        if kind == Kind::Mod && p.loader == Loader::Vanilla {
            self.mod_results.clear();
            self.mod_status = "Mod butuh profil dengan loader (Fabric, Quilt, Forge, atau NeoForge)".into();
            return;
        }

        let query = self.mod_query.trim().to_string();
        // Filter loader hanya untuk mod; resource pack, shader, dan modpack punya format sendiri.
        let loader_filter = if kind == Kind::Mod { p.loader.modrinth() } else { None };
        self.mod_busy = true;
        self.mod_status = "Mencari...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let r = modrinth::search(&query, kind, &p.version_id, loader_filter).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Search(r));
        });
    }

    /// Pasang versi terbaru sebuah mod, resource pack, atau shader ke instance yang dibuka.
    fn start_mod_install(&mut self, hit: Hit) {
        let Some(idx) = self.mods_for else { return };
        let Some(p) = self.profiles.get(idx).cloned() else { return };
        let kind = self.mod_kind;
        self.mod_busy = true;
        self.mod_status = format!("Memasang {}...", hit.title);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let ptx = tx.clone();
            let mut progress = |pr: Progress| {
                let _ = ptx.send(Msg::Status(pr));
            };
            let dir = profiles::game_dir(&p);
            let r = modrinth::install_project(
                &hit.project_id,
                kind,
                &p.version_id,
                p.loader.modrinth(),
                &dir,
                &mut progress,
            )
            .map(|files| format!("Terpasang: {}", files.join(", ")))
            .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::ModDone(r));
        });
    }

    /// Pasang modpack sebagai instance baru (memakai loader dari modpack itu sendiri).
    fn start_modpack(&mut self, hit: Hit) {
        let Some(idx) = self.mods_for else { return };
        let Some(mc) = self.profiles.get(idx).map(|p| p.version_id.clone()) else { return };
        self.mod_busy = true;
        self.mod_status = format!("Memasang modpack {}...", hit.title);
        let versions = self.versions.clone();
        let names: Vec<String> = self.profiles.iter().map(|p| p.name.clone()).collect();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let ptx = tx.clone();
            let mut progress = |pr: Progress| {
                let _ = ptx.send(Msg::Status(pr));
            };
            let r = create_modpack(&hit.project_id, &mc, &versions, &names, &mut progress)
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Installed(r));
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
            .max_height(170.0)
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
        ui.label("Loader");
        ui.horizontal_wrapped(|ui| {
            for l in Loader::ALL {
                if ui.selectable_label(self.add_loader == l, l.label()).clicked() {
                    self.add_loader = l;
                }
            }
        });

        // Daftar versi loader diambil sekali per kombinasi (loader, versi Minecraft).
        if self.add_loader != Loader::Vanilla {
            if let Some(i) = self.add_selected {
                let mc = self.versions[i].id.clone();
                let want = (self.add_loader, mc.clone());
                if self.loader_key.as_ref() != Some(&want) {
                    self.request_loader_versions(want.0, mc);
                }
            }

            let mut lv_pick: Option<String> = None;
            ui.horizontal(|ui| {
                ui.label("Versi loader");
                if self.loader_versions.is_empty() {
                    match &self.loader_error {
                        Some(e) => {
                            ui.colored_label(egui::Color32::LIGHT_RED, e.as_str());
                        }
                        None => {
                            let hint = if self.add_selected.is_none() {
                                "Pilih versi Minecraft dulu"
                            } else {
                                "Memuat..."
                            };
                            ui.label(egui::RichText::new(hint).color(MUTED));
                        }
                    }
                } else {
                    egui::ComboBox::from_id_salt("loader_ver")
                        .selected_text(self.add_loader_version.as_str())
                        .show_ui(ui, |ui| {
                            for lv in &self.loader_versions {
                                if ui
                                    .selectable_label(self.add_loader_version == *lv, lv.as_str())
                                    .clicked()
                                {
                                    lv_pick = Some(lv.clone());
                                }
                            }
                        });
                }
            });
            if let Some(lv) = lv_pick {
                self.add_loader_version = lv;
            }
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

        let loader_ok = self.add_loader == Loader::Vanilla || !self.add_loader_version.is_empty();
        ui.horizontal(|ui| {
            let can_install = !self.busy
                && self.add_selected.is_some()
                && !name.is_empty()
                && !name_taken
                && loader_ok;
            if ui.add_enabled(can_install, egui::Button::new("Install")).clicked() {
                if let Some(i) = self.add_selected {
                    let v = self.versions[i].clone();
                    let isolated = self.add_isolated;
                    let loader = self.add_loader;
                    let lv = if loader == Loader::Vanilla {
                        String::new()
                    } else {
                        self.add_loader_version.clone()
                    };
                    self.start_install(v, name.clone(), isolated, loader, lv);
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

    /// Isi jendela "Mod" untuk satu instance. Mengembalikan aksi yang diminta user.
    fn mods_dialog(&mut self, ui: &mut egui::Ui) -> Option<ModAction> {
        let idx = self.mods_for?;
        let p = self.profiles.get(idx)?.clone();
        let mut action: Option<ModAction> = None;

        ui.label(
            egui::RichText::new(format!("Minecraft {}  |  {}", p.version_id, p.loader_label())).color(MUTED),
        );
        ui.horizontal_wrapped(|ui| {
            for k in [Kind::Mod, Kind::Modpack, Kind::ResourcePack, Kind::Shader] {
                if ui.selectable_label(self.mod_kind == k, k.label()).clicked() && self.mod_kind != k {
                    self.mod_kind = k;
                    action = Some(ModAction::Search);
                }
            }
        });

        let blocked = self.mod_kind == Kind::Mod && p.loader == Loader::Vanilla;
        if blocked {
            ui.colored_label(
                MUTED,
                "Mod butuh profil dengan loader. Buat profil baru dengan Fabric, Quilt, Forge, atau NeoForge.",
            );
        }

        let mut search_now = false;
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.mod_query)
                    .hint_text("Cari di Modrinth...")
                    .desired_width(330.0),
            );
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                search_now = true;
            }
            if ui.add_enabled(!self.mod_busy, egui::Button::new("Cari")).clicked() {
                search_now = true;
            }
        });
        if search_now {
            action = Some(ModAction::Search);
        }

        ui.add_space(4.0);
        let busy = self.mod_busy;
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for h in &self.mod_results {
                    egui::Frame::none()
                        .fill(CARD)
                        .rounding(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(h.title.as_str());
                                    ui.label(egui::RichText::new(h.description.as_str()).color(MUTED).small());
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let btn = egui::Button::image_and_text(
                                        icons::img(&self.icons.download, 14.0),
                                        "Install",
                                    );
                                    if ui.add_enabled(!busy && !blocked, btn).clicked() {
                                        action = Some(ModAction::Install(h.clone()));
                                    }
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            });

        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "Versi terbaru untuk Minecraft {} otomatis dipilih.  {}",
                p.version_id, self.mod_status
            ))
            .color(MUTED)
            .small(),
        );
        action
    }
}

/// Unduh modpack (.mrpack), buat instance baru dengan loader-nya, lalu pasang semua file-nya.
fn create_modpack(
    project_id: &str,
    mc: &str,
    versions: &[Version],
    names: &[String],
    progress: &mut dyn FnMut(Progress),
) -> anyhow::Result<Profile> {
    let dir = launcher::data_dir().join("downloads");
    let (path, meta) = modrinth::download_modpack(project_id, mc, &dir, progress)?;
    let v = versions
        .iter()
        .find(|v| v.id == meta.mc)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("versi Minecraft {} tidak ada di daftar", meta.mc))?;

    let mut name = meta.name.clone();
    let mut n = 2;
    while names.contains(&name) {
        name = format!("{} ({n})", meta.name);
        n += 1;
    }

    let profile = Profile {
        name,
        version_id: v.id.clone(),
        version_kind: v.kind.clone(),
        version_url: v.url.clone(),
        isolated: true,
        loader: meta.loader,
        loader_version: meta.loader_version.clone(),
    };
    launcher::install_profile(&v, &profile, progress)?;
    modrinth::extract_modpack(&path, &profiles::game_dir(&profile), progress)?;
    Ok(profile)
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
                Msg::Status(p) => {
                    self.status = p.text;
                    self.progress = p.fraction;
                }
                Msg::Done(Ok(())) => {
                    self.progress = None;
                    self.status = "Game berjalan".into();
                    self.busy = false;
                }
                Msg::Done(Err(e)) => {
                    self.progress = None;
                    self.status = format!("Error: {e}");
                    self.busy = false;
                }
                Msg::UpdateAvailable(u) => self.update = Some(u),
                Msg::Installed(Ok(p)) => {
                    let name = p.name.clone();
                    self.profiles.push(p);
                    let _ = profiles::save(&self.profiles);
                    self.selected = Some(self.profiles.len() - 1);
                    self.progress = None;
                    self.status = "Versi berhasil dipasang".into();
                    self.mod_status = format!("Instance '{name}' siap");
                    self.busy = false;
                    self.mod_busy = false;
                }
                Msg::Installed(Err(e)) => {
                    self.progress = None;
                    self.status = format!("Gagal memasang: {e}");
                    self.busy = false;
                    self.mod_busy = false;
                    self.mod_status = format!("Gagal: {e}");
                }
                Msg::LoaderVersions { key, result } => {
                    if self.loader_key.as_ref() == Some(&key) {
                        match result {
                            Ok(list) if list.is_empty() => {
                                self.loader_versions.clear();
                                self.loader_error = Some("Tidak ada versi loader untuk Minecraft ini".into());
                            }
                            Ok(list) => {
                                self.add_loader_version = list[0].clone();
                                self.loader_versions = list;
                            }
                            Err(e) => {
                                self.loader_error = Some(format!("Gagal memuat versi loader: {e}"));
                            }
                        }
                    }
                }
                Msg::Search(Ok(hits)) => {
                    self.mod_status = if hits.is_empty() {
                        "Tidak ada hasil".into()
                    } else {
                        format!("{} hasil", hits.len())
                    };
                    self.mod_results = hits;
                    self.mod_busy = false;
                }
                Msg::Search(Err(e)) => {
                    self.mod_status = format!("Gagal mencari: {e}");
                    self.mod_busy = false;
                }
                Msg::ModDone(Ok(s)) => {
                    self.mod_status = s;
                    self.mod_busy = false;
                    self.progress = None;
                }
                Msg::ModDone(Err(e)) => {
                    self.mod_status = format!("Gagal: {e}");
                    self.mod_busy = false;
                    self.progress = None;
                }
            }
        }
        if self.busy || self.mod_busy {
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

                            ui.label("Preset");
                            egui::ComboBox::from_id_salt("preset")
                                .selected_text(preset_name(&self.cfg.jvm_args))
                                .show_ui(ui, |ui| {
                                    for (name, args) in PRESETS {
                                        if ui
                                            .selectable_label(self.cfg.jvm_args == *args, *name)
                                            .clicked()
                                        {
                                            self.cfg.jvm_args = args.to_string();
                                        }
                                    }
                                });
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
            if let Some(f) = self.progress {
                ui.add(egui::ProgressBar::new(f).show_percentage().desired_width(f32::INFINITY));
            }
            ui.label(egui::RichText::new(self.status.as_str()).color(MUTED));
            ui.add_space(4.0);
        });

        let mut play_req: Option<usize> = None;
        let mut remove_req: Option<usize> = None;
        let mut mod_req: Option<usize> = None;

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
                                            p.version_id,
                                            p.loader_label(),
                                            badge
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
                                        if ui
                                            .add_enabled(
                                                !self.busy,
                                                egui::Button::image_and_text(
                                                    icons::img(&self.icons.download, 14.0),
                                                    "Mod",
                                                ),
                                            )
                                            .clicked()
                                        {
                                            mod_req = Some(i);
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
        if let Some(i) = mod_req {
            self.selected = Some(i);
            self.open_mods(i);
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
                .fixed_size([440.0, 560.0])
                .show(ctx, |ui| self.add_dialog(ui));
            if !open {
                self.show_add = false;
            }
        }

        // Jendela Mod untuk instance yang dipilih
        if let Some(idx) = self.mods_for {
            match self.profiles.get(idx).map(|p| p.name.clone()) {
                None => self.mods_for = None,
                Some(name) => {
                    let mut open = true;
                    let mut action: Option<ModAction> = None;
                    egui::Window::new(format!("Mod: {name}"))
                        .open(&mut open)
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .fixed_size([480.0, 480.0])
                        .show(ctx, |ui| {
                            action = self.mods_dialog(ui);
                        });
                    if !open {
                        self.mods_for = None;
                    }
                    match action {
                        Some(ModAction::Search) => self.start_search(),
                        Some(ModAction::Install(hit)) => {
                            if self.mod_kind == Kind::Modpack {
                                self.start_modpack(hit);
                            } else {
                                self.start_mod_install(hit);
                            }
                        }
                        None => {}
                    }
                }
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
