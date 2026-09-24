//! Der Launcher: das Programm, das Spieler herunterladen und starten.
//!
//! - Erster Start (Windows): installiert sich selbst, legt Verknüpfungen an.
//! - Jeder Start: sucht nach Updates, lädt nur Geänderte, repariert Beschädigtes,
//!   aktualisiert sich bei Bedarf selbst und startet dann das Spiel.
//!
//! Testschalter: `--quelle <ordner|adresse>` (Update-Quelle), `--ordner <pfad>` (Installationsort;
//! dann ohne Verknüpfungen und Registry-Einträge).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use launcher::config::{game_dir, install_dir, update_location, GAME_NAME, PUBLIC_KEY, WEBSITE};
use launcher::{apply_update, fetch_manifest, replace_file, sha256_file, sha256_hex, source_for, Manifest, Progress};

const BACKGROUND: Color32 = Color32::from_rgb(12, 14, 19);
const CARD: Color32 = Color32::from_rgb(22, 26, 34);
const GOLD: Color32 = Color32::from_rgb(214, 178, 96);
const GOLD_BRIGHT: Color32 = Color32::from_rgb(240, 208, 128);
const GOLD_DARK: Color32 = Color32::from_rgb(120, 92, 42);
const TEXT: Color32 = Color32::from_rgb(235, 238, 245);
const MUTED: Color32 = Color32::from_rgb(150, 158, 175);
const ERROR: Color32 = Color32::from_rgb(255, 120, 105);

const WINDOW: Vec2 = Vec2::new(960.0, 600.0);
const BANNER_HEIGHT: f32 = 330.0;

/// Was der Launcher gerade tut.
#[derive(Clone, Debug)]
enum Phase {
    /// Noch nicht installiert (nur Windows): Willkommensbildschirm.
    Welcome,
    /// Deinstallation bestätigen (aus „Apps & Features“ aufgerufen).
    Uninstall,
    Checking,
    Downloading(Progress),
    /// Der Launcher selbst wird ersetzt und neu gestartet.
    UpdatingLauncher,
    Ready,
    /// Kein Update-Server erreichbar – installierte Version ist spielbar.
    Offline(String),
    Failed(String),
}

struct Shared {
    phase: Phase,
    /// Neueste Version vom Server (oder installierte, wenn offline).
    manifest: Option<Manifest>,
    worker_running: bool,
}

struct LauncherApp {
    shared: Arc<Mutex<Shared>>,
    banner: egui::TextureHandle,
    desktop_shortcut: bool,
    // Nur für automatische Tests (`--screenshot`).
    frames: u32,
    screenshot_requested: bool,
}

fn installed_manifest_path() -> PathBuf {
    install_dir().join("installiert.json")
}

fn load_installed() -> Option<Manifest> {
    serde_json::from_slice(&std::fs::read(installed_manifest_path()).ok()?).ok()
}

/// Im Hintergrund: Update-Liste holen, Launcher und Spiel auf den neuesten Stand bringen.
fn update_worker(shared: Arc<Mutex<Shared>>, ctx: egui::Context) {
    let set = |phase: Phase| {
        shared.lock().expect("Zustand").phase = phase;
        ctx.request_repaint();
    };
    set(Phase::Checking);
    let installed = load_installed();
    let source = source_for(&update_location());

    let manifest = match fetch_manifest(source.as_ref(), PUBLIC_KEY.trim()) {
        Ok(manifest) => manifest,
        Err(message) => {
            log::warn!("Update-Prüfung fehlgeschlagen: {message}");
            let playable = installed.as_ref().is_some_and(|m| game_dir().join(&m.executable).exists());
            let mut state = shared.lock().expect("Zustand");
            state.manifest = installed;
            state.phase = if playable { Phase::Offline(message) } else { Phase::Failed(message) };
            state.worker_running = false;
            ctx.request_repaint();
            return;
        }
    };
    shared.lock().expect("Zustand").manifest = Some(manifest.clone());

    // Neuer Launcher? Dann erst sich selbst ersetzen und neu starten.
    if let (Some(entry), Ok(current)) = (&manifest.launcher, std::env::current_exe()) {
        let outdated = sha256_file(&current).is_ok_and(|hash| hash != entry.sha256);
        if outdated && install::is_installed_copy() {
            set(Phase::UpdatingLauncher);
            let result = source.fetch(&format!("dateien/{}", entry.sha256), &mut |_| {}).and_then(|data| {
                if sha256_hex(&data) != entry.sha256 {
                    return Err("Neuer Launcher ist beschädigt angekommen".into());
                }
                let fresh = current.with_extension("neu");
                std::fs::write(&fresh, &data).map_err(|e| e.to_string())?;
                install::make_executable(&fresh);
                replace_file(&fresh, &current)
            });
            match result {
                Ok(()) => {
                    install::restart(&current);
                    std::process::exit(0);
                }
                // Kein Beinbruch: dann eben mit dem alten Launcher weiter.
                Err(message) => log::warn!("Launcher-Update fehlgeschlagen: {message}"),
            }
        }
    }

    let result = apply_update(source.as_ref(), &manifest, &game_dir(), &mut |progress| {
        if progress.files_total > 0 {
            shared.lock().expect("Zustand").phase = Phase::Downloading(progress.clone());
            ctx.request_repaint();
        }
    });
    let mut state = shared.lock().expect("Zustand");
    state.phase = match result {
        Ok(()) => {
            if let Ok(json) = serde_json::to_vec_pretty(&manifest) {
                let _ = std::fs::write(installed_manifest_path(), json);
            }
            Phase::Ready
        }
        Err(message) => Phase::Failed(message),
    };
    state.worker_running = false;
    ctx.request_repaint();
}

impl LauncherApp {
    fn new(cc: &eframe::CreationContext<'_>, first_phase: Phase) -> Self {
        let image = image::load_from_memory(include_bytes!("../assets/banner.png")).expect("Banner eingebettet").to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let banner = cc.egui_ctx.load_texture("banner", egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()), egui::TextureOptions::LINEAR);
        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = egui::Visuals::dark();
            style.visuals.override_text_color = Some(TEXT);
            style.visuals.selection.bg_fill = GOLD_DARK;
            style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        });
        let mut app = LauncherApp {
            shared: Arc::new(Mutex::new(Shared { phase: first_phase.clone(), manifest: load_installed(), worker_running: false })),
            banner,
            desktop_shortcut: true,
            frames: 0,
            screenshot_requested: false,
        };
        if matches!(first_phase, Phase::Checking) {
            app.start_worker(&cc.egui_ctx);
        }
        app
    }

    fn start_worker(&mut self, ctx: &egui::Context) {
        let mut state = self.shared.lock().expect("Zustand");
        if state.worker_running {
            return;
        }
        state.worker_running = true;
        drop(state);
        let (shared, ctx) = (self.shared.clone(), ctx.clone());
        std::thread::spawn(move || update_worker(shared, ctx));
    }

    fn play(&self, ctx: &egui::Context) {
        let Some(manifest) = self.shared.lock().expect("Zustand").manifest.clone() else { return };
        let exe = game_dir().join(&manifest.executable);
        // Unter macOS/Linux muss das Spiel ausführbar sein (Downloads sind es nicht automatisch).
        install::make_executable(&exe);
        match std::process::Command::new(&exe).current_dir(game_dir()).spawn() {
            Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(e) => self.shared.lock().expect("Zustand").phase = Phase::Failed(format!("Spiel ließ sich nicht starten: {e}")),
        }
    }

    // ---------- Zeichnen ----------

    fn banner(&self, ui: &mut egui::Ui) {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(WINDOW.x, BANNER_HEIGHT));
        let painter = ui.painter();
        painter.image(self.banner.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        // Unten weich in den dunklen Hintergrund übergehen, links etwas abdunkeln für den Titel.
        let mut mesh = egui::Mesh::default();
        let fade_top = rect.bottom() - 150.0;
        let clear = Color32::from_rgba_unmultiplied(12, 14, 19, 0);
        mesh.colored_vertex(Pos2::new(rect.left(), fade_top), clear);
        mesh.colored_vertex(Pos2::new(rect.right(), fade_top), clear);
        mesh.colored_vertex(Pos2::new(rect.right(), rect.bottom()), BACKGROUND);
        mesh.colored_vertex(Pos2::new(rect.left(), rect.bottom()), BACKGROUND);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        let shade = Color32::from_rgba_unmultiplied(12, 14, 19, 150);
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(rect.left_top(), shade);
        mesh.colored_vertex(Pos2::new(rect.left() + 520.0, rect.top()), clear);
        mesh.colored_vertex(Pos2::new(rect.left() + 520.0, rect.bottom()), clear);
        mesh.colored_vertex(rect.left_bottom(), shade);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
        painter.add(mesh);

        // Titel mit Schatten und goldener Linie
        let title_pos = Pos2::new(44.0, 200.0);
        let font = FontId::proportional(54.0);
        painter.text(title_pos + egui::vec2(2.0, 3.0), egui::Align2::LEFT_BOTTOM, GAME_NAME.to_uppercase(), font.clone(), Color32::from_black_alpha(170));
        let title = painter.text(title_pos, egui::Align2::LEFT_BOTTOM, GAME_NAME.to_uppercase(), font, GOLD_BRIGHT);
        let line_y = title.bottom() + 10.0;
        painter.line_segment([Pos2::new(46.0, line_y), Pos2::new(46.0 + title.width(), line_y)], Stroke::new(1.5, GOLD));
        diamond(painter, Pos2::new(46.0 + title.width() + 10.0, line_y), 4.0, GOLD);
        painter.text(Pos2::new(46.0, line_y + 14.0), egui::Align2::LEFT_TOP, "Fantasy-Insel · Multiplayer · Baue, sammle, entdecke", FontId::proportional(17.0), TEXT);
    }

    fn news(&self, ui: &mut egui::Ui, manifest: Option<&Manifest>) {
        ui.label(RichText::new("NEUIGKEITEN").size(13.0).color(GOLD).strong());
        ui.add_space(2.0);
        match manifest {
            Some(m) => {
                ui.label(RichText::new(format!("Version {}  ·  {}", m.version, german_date(&m.date))).size(18.0).strong());
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    if m.news.is_empty() {
                        ui.label(RichText::new("Fehlerbehebungen und Verbesserungen.").color(MUTED));
                    }
                    for line in &m.news {
                        ui.horizontal_wrapped(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 16.0), egui::Sense::hover());
                            diamond(ui.painter(), rect.center() + egui::vec2(0.0, 1.0), 4.0, GOLD);
                            ui.label(RichText::new(line).size(15.5));
                        });
                    }
                });
            }
            None => {
                ui.label(RichText::new("Wird geladen …").color(MUTED));
            }
        }
    }

    fn status(&mut self, ui: &mut egui::Ui, phase: &Phase) {
        let (headline, detail, fraction): (String, String, Option<f32>) = match phase {
            Phase::Checking => ("Suche nach Updates …".into(), String::new(), None),
            Phase::Downloading(p) => (
                format!("Lade Update · Datei {} von {}", (p.files_done + 1).min(p.files_total), p.files_total),
                format!("{} von {}", megabytes(p.bytes_done), megabytes(p.bytes_total)),
                Some(if p.bytes_total > 0 { p.bytes_done as f32 / p.bytes_total as f32 } else { 0.0 }),
            ),
            Phase::UpdatingLauncher => ("Launcher wird aktualisiert …".into(), "Startet gleich neu".into(), None),
            Phase::Ready => ("Bereit zum Spielen".into(), "Alles ist auf dem neuesten Stand.".into(), Some(1.0)),
            Phase::Offline(_) => ("Keine Verbindung zum Update-Server".into(), "Du kannst die installierte Version spielen.".into(), None),
            Phase::Failed(message) => ("Das hat nicht geklappt".into(), message.clone(), None),
            Phase::Welcome | Phase::Uninstall => (String::new(), String::new(), None),
        };
        let error = matches!(phase, Phase::Failed(_));
        ui.label(RichText::new(headline).size(19.0).strong().color(if error { ERROR } else { TEXT }));
        let detail = ui.label(RichText::new(detail).size(14.0).color(MUTED));
        if let Phase::Offline(reason) = phase {
            detail.on_hover_text(reason);
        }
        ui.add_space(4.0);
        progress_bar(ui, fraction, ui.ctx().input(|i| i.time));
        ui.add_space(12.0);

        let playable = matches!(phase, Phase::Ready | Phase::Offline(_));
        ui.horizontal(|ui| {
            if gold_button(ui, "SPIELEN", playable, egui::vec2(230.0, 54.0)).clicked() {
                self.play(ui.ctx());
            }
            if matches!(phase, Phase::Failed(_) | Phase::Offline(_)) && ui.add_sized([120.0, 54.0], egui::Button::new(RichText::new("Erneut").size(17.0))).clicked() {
                self.start_worker(ui.ctx());
            }
        });
    }

    fn footer(&mut self, ui: &mut egui::Ui, installed: Option<&Manifest>) {
        ui.horizontal(|ui| {
            let text = match installed {
                Some(m) => format!("Installiert: Version {}", m.version),
                None => "Noch nicht installiert".into(),
            };
            ui.label(RichText::new(text).size(13.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if link(ui, "Webseite").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(WEBSITE));
                }
                if link(ui, "Spielordner").clicked() {
                    install::open_folder(&install_dir());
                }
                if link(ui, "Dateien prüfen").clicked() {
                    self.start_worker(ui.ctx());
                }
            });
        });
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("WILLKOMMEN").size(13.0).color(GOLD).strong());
        ui.label(RichText::new(format!("{GAME_NAME} wird eingerichtet")).size(22.0).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Installationsort: {}", install_dir().display())).size(14.0).color(MUTED));
        ui.label(RichText::new("Das Spiel wird danach automatisch heruntergeladen und immer aktuell gehalten.").size(14.0).color(MUTED));
        ui.add_space(8.0);
        ui.checkbox(&mut self.desktop_shortcut, RichText::new("Verknüpfung auf dem Desktop").size(15.0));
        ui.add_space(10.0);
        if ui.horizontal(|ui| gold_button(ui, "INSTALLIEREN", true, egui::vec2(260.0, 54.0))).inner.clicked() {
            match install::install(self.desktop_shortcut) {
                Ok(()) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                Err(message) => self.shared.lock().expect("Zustand").phase = Phase::Failed(format!("Installation fehlgeschlagen: {message}")),
            }
        }
    }

    fn uninstall(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("DEINSTALLIEREN").size(13.0).color(GOLD).strong());
        ui.label(RichText::new(format!("{GAME_NAME} von diesem Computer entfernen?")).size(22.0).strong());
        ui.label(RichText::new("Einstellungen und Spielstände bleiben erhalten.").size(14.0).color(MUTED));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if gold_button(ui, "ENTFERNEN", true, egui::vec2(220.0, 50.0)).clicked() {
                install::uninstall();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.add_sized([140.0, 50.0], egui::Button::new(RichText::new("Abbrechen").size(17.0))).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (phase, manifest) = {
            let state = self.shared.lock().expect("Zustand");
            (state.phase.clone(), state.manifest.clone())
        };
        let installed = load_installed();
        ui.painter().rect_filled(ui.max_rect(), 0.0, BACKGROUND);
        self.banner(ui);

        let content = Rect::from_min_max(Pos2::new(40.0, BANNER_HEIGHT - 6.0), Pos2::new(WINDOW.x - 40.0, WINDOW.y - 44.0));
        let mut body = ui.new_child(egui::UiBuilder::new().max_rect(content));
        match phase {
            Phase::Welcome => self.welcome(&mut body),
            Phase::Uninstall => self.uninstall(&mut body),
            _ => {
                body.columns(2, |columns| {
                    self.news(&mut columns[0], manifest.as_ref());
                    card(&mut columns[1], |ui| self.status(ui, &phase));
                });
            }
        }

        let footer = Rect::from_min_max(Pos2::new(40.0, WINDOW.y - 38.0), Pos2::new(WINDOW.x - 40.0, WINDOW.y - 10.0));
        ui.painter().line_segment([Pos2::new(40.0, footer.top() - 4.0), Pos2::new(WINDOW.x - 40.0, footer.top() - 4.0)], Stroke::new(1.0, Color32::from_white_alpha(18)));
        if !matches!(phase, Phase::Welcome | Phase::Uninstall) {
            let mut bottom = ui.new_child(egui::UiBuilder::new().max_rect(footer));
            self.footer(&mut bottom, installed.as_ref());
        }


        // Testschalter: `--auto-installieren` klickt „Installieren“, `--screenshot <datei>`
        // speichert ein Bild, sobald der Launcher zur Ruhe gekommen ist, und beendet ihn.
        let args: Vec<String> = std::env::args().collect();
        if matches!(phase, Phase::Welcome) && args.iter().any(|a| a == "--auto-installieren") {
            if let Err(message) = install::install(self.desktop_shortcut) {
                self.shared.lock().expect("Zustand").phase = Phase::Failed(message);
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if let Some(path) = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)) {
            self.frames += 1;
            let settled = !matches!(phase, Phase::Checking | Phase::Downloading(_) | Phase::UpdatingLauncher);
            if settled && self.frames > 20 && !self.screenshot_requested {
                self.screenshot_requested = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let shot = ui.ctx().input(|i| i.raw.events.iter().find_map(|e| if let egui::Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None }));
            if let Some(image) = shot {
                let [w, h] = image.size;
                let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buffer) = image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
                    match buffer.save(path) {
                        Ok(()) => println!("Screenshot gespeichert: {path}"),
                        Err(e) => eprintln!("Screenshot fehlgeschlagen: {e}"),
                    }
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ui.ctx().request_repaint();
        }
        if matches!(phase, Phase::Checking | Phase::Downloading(_) | Phase::UpdatingLauncher) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}

// ---------- Bausteine ----------

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, GOLD_DARK))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

fn gold_button(ui: &mut egui::Ui, text: &str, enabled: bool, size: Vec2) -> egui::Response {
    let fill = if enabled { GOLD } else { Color32::from_rgb(60, 58, 52) };
    let color = if enabled { Color32::from_rgb(28, 22, 10) } else { MUTED };
    let button = egui::Button::new(RichText::new(text).size(22.0).strong().color(color))
        .fill(fill)
        .stroke(Stroke::new(1.5, if enabled { GOLD_BRIGHT } else { Color32::TRANSPARENT }))
        .corner_radius(CornerRadius::same(8))
        .min_size(size);
    ui.add_enabled(enabled, button)
}

fn link(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Label::new(RichText::new(text).size(13.0).color(GOLD)).sense(egui::Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Goldener Balken; ohne Wert läuft ein Lichtschimmer hin und her (unbestimmt).
fn progress_bar(ui: &mut egui::Ui, fraction: Option<f32>, time: f64) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 10.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(5), Color32::from_rgb(34, 38, 48));
    match fraction {
        Some(f) => {
            let filled = Rect::from_min_size(rect.min, egui::vec2(rect.width() * f.clamp(0.0, 1.0), rect.height()));
            painter.rect_filled(filled, CornerRadius::same(5), GOLD);
        }
        None => {
            let t = ((time * 0.8).sin() * 0.5 + 0.5) as f32;
            let width = rect.width() * 0.25;
            let x = rect.left() + (rect.width() - width) * t;
            painter.rect_filled(Rect::from_min_size(Pos2::new(x, rect.top()), egui::vec2(width, rect.height())), CornerRadius::same(5), GOLD_DARK);
        }
    }
}

fn diamond(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let points = vec![center + egui::vec2(0.0, -size), center + egui::vec2(size, 0.0), center + egui::vec2(0.0, size), center + egui::vec2(-size, 0.0)];
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

/// "2026-09-24" → "24.09.2026"
fn german_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if let [y, m, d] = parts[..] { format!("{d}.{m}.{y}") } else { iso.to_string() }
}

/// Fenstersymbol: Ausschnitt der Insel aus dem Banner.
fn icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/banner.png")).expect("Banner eingebettet").to_rgba8();
    let cropped = image::imageops::crop_imm(&image, 380, 60, 200, 200).to_image();
    let small = image::imageops::resize(&cropped, 64, 64, image::imageops::FilterType::Triangle);
    egui::IconData { width: 64, height: 64, rgba: small.into_raw() }
}

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,launcher=info")).init();
    install::cleanup_after_update();
    let args: Vec<String> = std::env::args().collect();
    let first_phase = if args.iter().any(|a| a == "--deinstallieren") {
        Phase::Uninstall
    } else if install::needs_install() {
        Phase::Welcome
    } else {
        Phase::Checking
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(GAME_NAME)
            .with_inner_size(WINDOW)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(icon()),
        ..Default::default()
    };
    eframe::run_native(GAME_NAME, options, Box::new(move |cc| Ok(Box::new(LauncherApp::new(cc, first_phase)))))
}
