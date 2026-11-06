#![windows_subsystem = "windows"]

mod java;
mod launcher;
mod manifest;
mod update;

use eframe::egui;
use manifest::Version;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};

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
            jvm_args: "-Xmx2G -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:MaxGCPauseMillis=50".into(),
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
}

struct App {
    cfg: Config,
    versions: Vec<Version>,
    filter: Filter,
    selected: Option<usize>,
    status: String,
    busy: bool,
    update: Option<update::Update>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = channel();
        let cfg = launcher::load_config();

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
            cfg,
            versions: Vec::new(),
            filter: Filter::Release,
            selected: None,
            status: "Memuat daftar versi...".into(),
            busy: true,
            update: None,
            tx,
            rx,
        }
    }

    fn play(&mut self) {
        let Some(idx) = self.selected else { return };
        let v = self.versions[idx].clone();
        let cfg = self.cfg.clone();
        launcher::save_config(&cfg);

        self.busy = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let mut progress = |s: String| {
                let _ = progress_tx.send(Msg::Status(s));
            };
            let res = launcher::launch(&v, &cfg, &mut progress).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Done(res));
        });
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Manifest(Ok(v)) => {
                    self.versions = v;
                    self.status = "Pilih versi lalu klik Main".into();
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
            }
        }
        if self.busy {
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading("Saturn");
            if let Some(u) = self.update.clone() {
                ui.horizontal(|ui| {
                    ui.label(format!("Versi baru {} tersedia", u.version));
                    if ui.button("Update").clicked() {
                        self.start_update(u.clone());
                    }
                });
            }
            ui.horizontal(|ui| {
                for f in [Filter::Release, Filter::Snapshot, Filter::Old, Filter::All] {
                    if ui.selectable_label(self.filter == f, f.label()).clicked() {
                        self.filter = f;
                        self.selected = None;
                    }
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(6.0);
            egui::Grid::new("settings").num_columns(2).show(ui, |ui| {
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
            ui.horizontal(|ui| {
                let can_play = !self.busy && self.selected.is_some();
                if ui.add_enabled(can_play, egui::Button::new("Main")).clicked() {
                    self.play();
                }
                ui.label(self.status.as_str());
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let visible: Vec<usize> = self
                .versions
                .iter()
                .enumerate()
                .filter(|(_, v)| self.filter.matches(&v.kind))
                .map(|(i, _)| i)
                .collect();
            ui.label(format!("{} versi", visible.len()));
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in visible {
                    let label = format!("{}   [{}]", self.versions[i].id, self.versions[i].kind);
                    if ui.selectable_label(self.selected == Some(i), label).clicked() {
                        self.selected = Some(i);
                    }
                }
            });
        });
    }
}

/// Ikon jendela, dibuat oleh build.rs dari assets/saturn.svg.
const ICON_RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/saturn_icon_256.rgba"));

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
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
