//! Asset-Betrachter: zeigt Modelle im Licht des Spiels.
//!
//! Zwei Arten:
//! - **Galerie** im Hauptmenü: Liste aller Modelle des Spiels zum Anschauen. Wer aus dem
//!   Projektordner spielt (Entwickler), kann Modelle zusätzlich für die Insel markieren.
//! - **Einzeln** über `game --ansehen <datei>` (auch Blender-Quellen aus `art/modelle/`, die
//!   bei jeder Änderung neu gebaut werden). Schalter für Screenshots: `--animation <name>`,
//!   `--uhrzeit <h>`, `--drehen <grad>`.
//!
//! Modell auf einem Boden mit 1-Meter-Karos, daneben eine 1,8 m große Spielfigur als
//! Größenvergleich. Wird eine Datei neu exportiert, lädt der Betrachter sie von selbst neu.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use engine::egui::{self, RichText};
use engine::mesh::Vertex;
use engine::prelude::*;

use crate::asset_files;
use crate::blender::{self, Build, BuildState};
use crate::characters::Puppet;
use crate::markierungen::{self, Markierungen, PLAETZE};
use crate::protocol::CharacterClass;
use crate::ui;

/// So oft wird geschaut, ob die Datei neu exportiert wurde (Sekunden).
const POLL_INTERVAL: f32 = 0.5;
/// Abstand der Vergleichsfigur zum Modell in Metern.
const REFERENCE_GAP: f32 = 0.8;
/// Kamera-Abstand: so nah und so weit geht es.
const ZOOM_MIN: f32 = 0.3;
const ZOOM_MAX: f32 = 320.0;

/// Was gerade geladen ist.
struct Shown {
    model: Arc<Model>,
    entities: Vec<EntityId>,
    /// Nur bei animierten Modellen: Animator, Mesh und Textur zum Neuverformen.
    animated: Option<(Animator, MeshId, Option<TextureId>)>,
    min: Vec3,
    max: Vec3,
    vertices: usize,
    triangles: usize,
    /// Verstöße gegen die Asset-Regeln (siehe art/README.md).
    problems: Vec<String>,
}

/// Galerie-Zustand: alle Modelle und (für Entwickler) ihre Markierungen.
struct Gallery {
    /// (Gruppe, Anzeigename, Pfad relativ zum Asset-Ordner, voller Pfad)
    assets: Vec<(String, String, String, PathBuf)>,
    marks: Markierungen,
    /// Markieren nur, wenn das Spiel aus dem Projektordner läuft.
    developer: bool,
    back: bool,
}

pub struct Viewer {
    path: PathBuf,
    /// Blender-Skript oder .blend, aus dem `path` gebaut wird (falls angegeben).
    source: Option<PathBuf>,
    source_stamp: Option<SystemTime>,
    blender: Option<PathBuf>,
    build: Option<Build>,
    build_error: Option<String>,
    shown: Option<Shown>,
    error: Option<String>,
    /// Anzahl der Ladevorgänge (für eindeutige Texturnamen).
    generation: u32,
    /// Letzter bekannter Stand der Dateien und ob er sich seit der letzten Prüfung gehalten hat.
    stamp: Option<SystemTime>,
    pending: Option<SystemTime>,
    poll_timer: f32,
    reloaded_at: Option<f32>,
    gallery: Option<Gallery>,

    day: DayCycle,
    speed: f32,
    playing: bool,
    turntable: bool,
    reference: Option<(EntityId, Puppet)>,
    show_reference: bool,

    // Kamera kreist um die Mitte des Modells.
    distance: f32,
    pointer_over_ui: bool,
    themed: bool,
    /// Nur für Screenshots (`--zoom`, `--standbild`): fester Abstand und Animation bis zu
    /// diesem Zeitpunkt (Sekunden) abspielen, dann anhalten.
    fixed_zoom: Option<f32>,
    /// Fester Blickpunkt (`--ziel x,y,z`, Modellkoordinaten), sonst die Mitte des Modells
    fixed_target: Option<Vec3>,
    freeze_at: Option<f32>,
    played: f32,
}

/// Gruppenname für einen Ordner im Asset-Verzeichnis.
fn group_name(folder: &str) -> String {
    match folder {
        "natur" => "Natur".into(),
        "tiere" => "Tiere".into(),
        "figuren" => "Figuren".into(),
        "gebaeude" => "Gebäude".into(),
        "bauwerke" => "Bauwerke".into(),
        "npc" => "Figuren (NPC)".into(),
        "gegenstaende" => "Gegenstände".into(),
        "" => "Sonstiges".into(),
        other => other.to_string(),
    }
}

impl Viewer {
    pub fn new(path: PathBuf) -> Self {
        let path = std::path::absolute(&path).unwrap_or(path);
        let source = blender::is_source(&path).then(|| path.clone());
        let asset = source.as_deref().and_then(blender::asset_for_source);
        let error = (source.is_some() && asset.is_none()).then(|| "Blender-Quellen müssen unter art/modelle/ liegen.".to_string());
        Viewer {
            path: asset.unwrap_or(path),
            blender: source.as_ref().and_then(|_| blender::find_blender()),
            source,
            source_stamp: None,
            build: None,
            build_error: None,
            shown: None,
            error,
            generation: 0,
            stamp: None,
            pending: None,
            poll_timer: 0.0,
            reloaded_at: None,
            gallery: None,
            day: DayCycle { hour: 10.5, ..Default::default() },
            speed: 1.0,
            playing: true,
            turntable: false,
            reference: None,
            show_reference: true,
            distance: 5.0,
            pointer_over_ui: false,
            themed: false,
            fixed_zoom: None,
            fixed_target: None,
            freeze_at: None,
            played: 0.0,
        }
    }

    /// Galerie für das Hauptmenü: alle Modelle im Asset-Ordner.
    pub fn gallery() -> Self {
        let dir = asset_files::asset_dir();
        let mut assets = Vec::new();
        if let Some(dir) = &dir {
            for path in asset_files::all_models(dir) {
                let relative = path.strip_prefix(dir).unwrap_or(&path).components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
                let folder = relative.rsplit_once('/').map_or("", |(folder, _)| folder).to_string();
                let stem = path.file_stem().map_or_else(String::new, |s| s.to_string_lossy().replace('_', " "));
                let mut buchstaben = stem.chars();
                let name: String = buchstaben.next().map(|c| c.to_uppercase().chain(buchstaben).collect()).unwrap_or_default();
                assets.push((group_name(&folder), name, relative, path));
            }
        }
        // Natur und Tiere zuerst, dann der Rest.
        let order = |group: &str| ["Tiere", "Natur", "Figuren"].iter().position(|g| *g == group).unwrap_or(9);
        assets.sort_by(|a, b| order(&a.0).cmp(&order(&b.0)).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        // Entwickler = das Spiel läuft aus dem Projekt (Assets unter …/game/assets, nicht neben der installierten .exe).
        let developer = dir.as_ref().is_some_and(|d| d.components().rev().nth(1).is_some_and(|c| c.as_os_str() == "game"));
        let first = assets.first().map(|a| a.3.clone()).unwrap_or_default();
        let mut viewer = Viewer::new(first);
        viewer.gallery = Some(Gallery { assets, marks: Markierungen::laden(), developer, back: false });
        viewer
    }

    /// Will der Spieler aus der Galerie zurück ins Hauptmenü?
    pub fn wants_back(&self) -> bool {
        self.gallery.as_ref().is_some_and(|g| g.back)
    }

    /// Boden und Kamera anlegen, erstes Modell laden.
    pub fn setup(&mut self, ctx: &mut Context) {
        let floor = ctx.assets.named_mesh("betrachter_boden", || checker_floor(30));
        ctx.scene.spawn(Entity::new("Boden", floor));
        ctx.camera.yaw = 0.5;
        ctx.camera.pitch = -0.3;
        ctx.cursor_locked = false;
        self.stamp = self.files_stamp();
        self.load(ctx);
    }

    /// Anderes Modell zeigen (Galerie).
    fn select(&mut self, ctx: &mut Context, path: PathBuf) {
        if let Some(old) = self.shown.take() {
            for id in old.entities {
                ctx.scene.despawn(id);
            }
        }
        self.path = path;
        self.reloaded_at = None;
        self.error = None;
        self.stamp = self.files_stamp();
        self.pending = None;
        self.load(ctx);
    }

    /// Jüngste Änderungszeit aller Dateien neben dem Modell (.gltf, .bin, Texturen).
    fn files_stamp(&self) -> Option<SystemTime> {
        let dir = self.path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let own = std::fs::metadata(&self.path).and_then(|m| m.modified()).ok()?;
        let newest = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.metadata().ok()?.modified().ok())
            .fold(own, |a, b| a.max(b));
        Some(newest)
    }

    fn load(&mut self, ctx: &mut Context) {
        if !self.path.is_file() {
            return;
        }
        let model = match Model::from_file(&self.path) {
            Ok(model) => Arc::new(model),
            Err(message) => {
                log::error!("{message}");
                self.error = Some(message);
                return;
            }
        };
        let previous_clip = self.shown.as_ref().and_then(|s| s.animated.as_ref()).and_then(|(a, _, _)| a.current().map(str::to_string));
        if let Some(old) = self.shown.take() {
            for id in old.entities {
                ctx.scene.despawn(id);
            }
        }

        self.generation += 1;
        let textures = model.register_textures(&mut ctx.assets);
        let mut entities = Vec::new();
        let meshes: Vec<MeshData>;
        let mut animated = None;
        if model.clips.is_empty() {
            let groups = model.static_mesh_groups(&textures);
            for (info, mesh) in &groups {
                let id = ctx.assets.add_mesh(mesh.clone());
                // Blattkarten (Bäume, Gras) wie auf der Insel: weiches Laub-Licht, leichter Wind;
                // Teile mit Emission leuchten wie im Spiel.
                let material = if info.emissive > 0.0 {
                    Material::Emissive { glow: 1.5 }
                } else if mesh.alpha_cutout {
                    Material::Leaves { sway: 0.02 }
                } else {
                    Material::Standard
                };
                entities.push(ctx.scene.spawn(Entity::new("Modell", id).with_material(material)));
            }
            meshes = groups.into_iter().map(|(_, mesh)| mesh).collect();
        } else {
            // Animierte Modelle werden als ein Mesh verformt (eine Textur, Farben aus den Materialien).
            let texture = textures.first().copied();
            let mut animator = Animator::new(model.clone());
            let wanted = std::env::args().skip_while(|a| a != "--animation").nth(1);
            let clip = [wanted, previous_clip, Some("Idle".to_string())]
                .into_iter()
                .flatten()
                .find(|name| model.clip(name).is_some())
                .unwrap_or_else(|| model.clips[0].name.clone());
            animator.play(&clip, true, 0.0);
            let mesh = animator.skinned_mesh(texture);
            let id = ctx.assets.add_mesh(mesh.clone());
            entities.push(ctx.scene.spawn(Entity::new("Modell", id)));
            meshes = vec![mesh];
            animated = Some((animator, id, texture));
        }

        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in meshes.iter().flat_map(|m| &m.vertices) {
            min = min.min(v.position.into());
            max = max.max(v.position.into());
        }
        if meshes.iter().all(|m| m.vertices.is_empty()) {
            (min, max) = (Vec3::ZERO, Vec3::ZERO);
        }
        let first = self.reloaded_at.is_none();
        self.shown = Some(Shown {
            problems: asset_files::check_model(&self.path, &model),
            model,
            entities,
            animated,
            min,
            max,
            vertices: meshes.iter().map(|m| m.vertices.len()).sum(),
            triangles: meshes.iter().map(|m| m.indices.len() / 3).sum(),
        });
        self.error = None;
        if first {
            let size = (max - min).max_element().max(0.3);
            self.distance = (size * 2.2).clamp(1.0, 260.0);
            self.reloaded_at = Some(-100.0);
        } else {
            self.reloaded_at = Some(ctx.time.elapsed);
        }
        log::info!("{} geladen", self.path.display());
        self.place_reference(ctx);
    }

    /// Startet Blender für die Quelle (falls es eine gibt).
    fn start_build(&mut self) {
        let Some(source) = &self.source else { return };
        self.source_stamp = blender::source_stamp(source);
        let Some(exe) = &self.blender else {
            self.build_error = Some("Blender wurde nicht gefunden. Ist es installiert?".into());
            return;
        };
        match Build::start(exe, source) {
            Ok(build) => {
                self.build = Some(build);
                self.build_error = None;
            }
            Err(message) => self.build_error = Some(message),
        }
    }

    /// Läuft ein Bau, auf sein Ende warten; sonst bei geänderter Quelle einen starten.
    fn poll_build(&mut self, ctx: &mut Context) {
        match self.build.as_mut().map(Build::poll) {
            Some(BuildState::Running) => {}
            Some(BuildState::Done) => {
                self.build = None;
                self.stamp = self.files_stamp();
                self.pending = None;
                self.load(ctx);
            }
            Some(BuildState::Failed(message)) => {
                log::error!("Blender-Bau fehlgeschlagen:\n{message}");
                self.build = None;
                self.build_error = Some(message);
            }
            None => {
                if self.source.as_deref().is_some_and(|s| blender::source_stamp(s) != self.source_stamp) {
                    self.start_build();
                }
            }
        }
    }

    /// Stellt die Vergleichsfigur (1,8 m) rechts neben das Modell.
    fn place_reference(&mut self, ctx: &mut Context) {
        let Some(shown) = &self.shown else { return };
        let position = Vec3::new(shown.max.x.max(0.0) + REFERENCE_GAP + 0.4, 0.9, 0.0);
        match &self.reference {
            Some((root, _)) => ctx.scene.get_mut(*root).transform.position = position,
            None => {
                let root = ctx.scene.spawn(Entity::new("Vergleichsfigur", ctx.assets.cube()).with_transform(Transform::from_position(position)));
                ctx.scene.get_mut(root).visible = false;
                let puppet = Puppet::new(ctx, CharacterClass::Mage, root);
                self.reference = Some((root, puppet));
            }
        }
    }

    fn center(&self) -> Vec3 {
        match &self.shown {
            Some(s) => (s.min + s.max) * 0.5,
            None => Vec3::Y,
        }
    }

    /// Einmal pro Bild: Neuladen, Animation, Kamera, Licht.
    pub fn update(&mut self, ctx: &mut Context) {
        let dt = ctx.time.delta;

        // Neu exportiert? Erst laden, wenn sich die Dateien eine Prüfung lang nicht mehr ändern.
        self.poll_timer -= dt;
        if self.poll_timer <= 0.0 {
            self.poll_timer = POLL_INTERVAL;
            self.poll_build(ctx);
            let now = self.files_stamp();
            if now != self.stamp && self.build.is_none() {
                if self.pending == now {
                    self.stamp = now;
                    self.pending = None;
                    self.load(ctx);
                } else {
                    self.pending = now;
                }
            }
        }

        if let Some(Some((animator, mesh, texture))) = self.shown.as_mut().map(|s| s.animated.as_mut()) {
            animator.set_speed(self.speed);
            if let Some(until) = self.freeze_at {
                let step = (until - self.played).clamp(0.0, dt);
                if step > 0.0 {
                    animator.update(step);
                    self.played += step;
                }
            } else if self.playing {
                animator.update(dt);
            }
            ctx.assets.update_mesh(*mesh, animator.skinned_mesh(*texture));
        }
        if let Some((root, puppet)) = &mut self.reference {
            let position = ctx.scene.get(*root).transform.position;
            puppet.update(ctx, position, 0.0);
            puppet.set_visible(ctx, self.show_reference);
        }

        // Kamera: linke Maustaste ziehen = drehen, Mausrad = zoomen.
        if !self.pointer_over_ui {
            if ctx.input.mouse(MouseButton::Left) {
                let delta = ctx.input.mouse_delta() * 0.005;
                ctx.camera.yaw += delta.x;
                ctx.camera.pitch = (ctx.camera.pitch - delta.y).clamp(-1.45, 0.6);
            }
            self.distance = (self.distance * 0.88f32.powf(ctx.input.scroll())).clamp(ZOOM_MIN, ZOOM_MAX);
        }
        if self.turntable {
            ctx.camera.yaw += dt * 0.4;
        }
        if let Some(zoom) = self.fixed_zoom {
            self.distance = zoom;
        }
        let target = self.fixed_target.unwrap_or_else(|| self.center());
        ctx.camera.position = target - ctx.camera.forward() * self.distance;
        ctx.camera.near = (self.distance * 0.01).clamp(0.01, 0.1);

        self.day.apply(&mut ctx.env);
        let size = self.shown.as_ref().map_or(2.0, |s| (s.max - s.min).max_element());
        ctx.env.shadow_range = (size * 1.5 + 3.0).clamp(6.0, 60.0);
    }

    /// Die Liste aller Modelle (links) – nur in der Galerie.
    fn asset_list(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let Some(gallery) = &mut self.gallery else { return };
        let mut chosen = None;
        let hoehe = egui_ctx.content_rect().height() - 24.0;
        egui::Window::new("Asset-Galerie")
            .title_bar(false)
            .anchor(egui::Align2::LEFT_TOP, [12.0, 12.0])
            .resizable(false)
            .movable(false)
            .frame(ui::panel_frame())
            .show(egui_ctx, |ui| {
            ui.set_width(260.0);
            ui.set_height(hoehe - 40.0);
            if ui.button(RichText::new("Zurück zum Hauptmenü").size(17.0)).clicked() {
                gallery.back = true;
            }
            ui.add_space(6.0);
            ui.label(RichText::new("ASSET-GALERIE").strong().size(20.0).color(ui::ACCENT));
            ui.label(RichText::new(format!("{} Modelle", gallery.assets.len())).color(ui::MUTED).size(13.0));
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut current_group = "";
                for (group, name, relative, path) in &gallery.assets {
                    if group != current_group {
                        ui.add_space(6.0);
                        ui.label(RichText::new(group.to_uppercase()).size(13.0).strong().color(ui::MUTED));
                        current_group = group;
                    }
                    let marked = gallery.marks.platz(relative).map(|p| format!("   [{}]", markierungen::platz_name(p))).unwrap_or_default();
                    let active = *path == self.path;
                    if ui.selectable_label(active, format!("{name}{marked}")).clicked() && !active {
                        chosen = Some(path.clone());
                    }
                }
            });
        });
        if let Some(path) = chosen {
            self.select(ctx, path);
        }
    }

    /// „Auf der Insel verwenden als …“ – nur für Entwickler.
    fn marking(&mut self, ui: &mut egui::Ui) {
        let Some(gallery) = &mut self.gallery else { return };
        if !gallery.developer {
            return;
        }
        let Some((_, _, relative, _)) = gallery.assets.iter().find(|a| a.3 == self.path) else { return };
        let relative = relative.clone();
        ui.separator();
        ui.label(RichText::new("Auf der Insel verwenden als").strong());
        let current = gallery.marks.platz(&relative).map(str::to_string);
        let mut choice = current.clone();
        egui::ComboBox::from_id_salt("markierung")
            .width(230.0)
            .selected_text(current.as_deref().map_or("– nicht verwenden –", markierungen::platz_name))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut choice, None, "– nicht verwenden –");
                for (id, name, folder) in PLAETZE {
                    ui.selectable_value(&mut choice, Some(id.to_string()), format!("{name}  ({folder})"));
                }
            });
        if choice != current {
            gallery.marks.setzen(&relative, choice.as_deref());
            gallery.marks.speichern();
        }
        ui.label(
            RichText::new("Nur auf diesem PC (Entwickler-Test). Wirkt ab der nächsten Runde; mehrere Modelle für einen Platz werden gemischt.")
                .color(ui::MUTED)
                .size(12.0),
        );
    }

    pub fn ui(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        if !self.themed {
            ui::apply_theme(egui_ctx);
            self.themed = true;
        }
        self.asset_list(ctx, egui_ctx);
        let mut reload = false;
        let in_gallery = self.gallery.is_some();
        let window = egui::Window::new(if in_gallery { "Details" } else { "Asset-Betrachter" })
            .anchor(if in_gallery { egui::Align2::RIGHT_TOP } else { egui::Align2::LEFT_TOP }, if in_gallery { [-12.0, 12.0] } else { [12.0, 12.0] })
            .resizable(false)
            .collapsible(true)
            .vscroll(true)
            .max_height(egui_ctx.content_rect().height() - 24.0)
            .default_height(egui_ctx.content_rect().height() - 24.0)
            .frame(ui::panel_frame());
        window.show(egui_ctx, |ui| {
            ui.set_width(290.0);
            ui.spacing_mut().slider_width = 150.0;
            let name = self.path.file_name().map_or_else(|| self.path.display().to_string(), |n| n.to_string_lossy().into_owned());
            ui.label(RichText::new(name).strong().size(16.0).color(ui::ACCENT));
            if let Some(source) = self.source.as_deref().and_then(Path::file_name) {
                ui.label(RichText::new(format!("Quelle: {}", source.to_string_lossy())).color(ui::MUTED));
            }
            if self.build.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Blender baut …").color(ui::ACCENT));
                });
            }
            if let Some(error) = &self.build_error {
                ui.label(RichText::new(error).color(ui::ERROR).monospace().size(11.0));
            }
            if let Some(error) = &self.error {
                ui.label(RichText::new(error).color(ui::ERROR));
            }
            if self.reloaded_at.is_some_and(|t| ctx.time.elapsed - t < 2.0) {
                ui.label(RichText::new("Neu geladen").color(ui::ACCENT));
            }
            // Für Entwickler zuerst: Modell für die Insel markieren.
            self.marking(ui);
            if let Some(shown) = &mut self.shown {
                let size = shown.max - shown.min;
                ui.label(format!("Größe: {:.2} × {:.2} × {:.2} m (B × H × T)", size.x, size.y, size.z));
                ui.label(RichText::new(format!("Unterkante bei {:.2} m", shown.min.y)).color(ui::MUTED));
                ui.label(format!("{} Dreiecke, {} Eckpunkte", shown.triangles, shown.vertices));
                for problem in &shown.problems {
                    ui.label(RichText::new(format!("⚠ {problem}")).color(ui::ERROR));
                }
                ui.label(format!("{} Materialien, {} Texturen", shown.model.materials.len().saturating_sub(1), shown.model.images.len()));

                if let Some((animator, _, _)) = &mut shown.animated {
                    ui.separator();
                    ui.label(RichText::new("Animationen").strong());
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        for clip in &shown.model.clips {
                            let active = animator.current() == Some(clip.name.as_str());
                            let label = format!("{}  ({:.1} s)", clip.name, clip.duration);
                            if ui.selectable_label(active, label).clicked() {
                                animator.play(&clip.name, true, 0.2);
                            }
                        }
                    });
                    if ui.button(if self.playing { "Pause" } else { "Abspielen" }).clicked() {
                        self.playing = !self.playing;
                    }
                    ui.add(egui::Slider::new(&mut self.speed, 0.1..=2.0).text("Tempo"));
                }
            }
            ui.separator();
            // Zoom: Regler (logarithmisch) plus Knöpfe; das Mausrad geht auch.
            ui.horizontal(|ui| {
                if ui.button(RichText::new(" − ").size(18.0)).clicked() {
                    self.distance = (self.distance * 1.25).min(ZOOM_MAX);
                }
                ui.add(egui::Slider::new(&mut self.distance, ZOOM_MIN..=ZOOM_MAX).logarithmic(true).show_value(false).text("Zoom"));
                if ui.button(RichText::new(" + ").size(18.0)).clicked() {
                    self.distance = (self.distance * 0.8).max(ZOOM_MIN);
                }
            });
            ui.add(egui::Slider::new(&mut self.day.hour, 0.0..=24.0).text("Uhrzeit"));
            ui.checkbox(&mut self.turntable, "Drehteller");
            ui.checkbox(&mut self.show_reference, "Vergleichsfigur (1,8 m)");
            if !in_gallery {
                reload = ui.button("Neu laden").clicked();
            }
            ui.label(RichText::new("Linke Maustaste ziehen: drehen · Mausrad: zoomen").color(ui::MUTED).size(12.0));
        });
        self.pointer_over_ui = egui_ctx.is_pointer_over_egui() || egui_ctx.egui_is_using_pointer();
        if reload {
            self.load(ctx);
        }
    }
}

/// Boden mit 1-Meter-Karos, damit Größen sofort ablesbar sind.
fn checker_floor(half: i32) -> MeshData {
    let mut mesh = MeshData::default();
    for z in -half..half {
        for x in -half..half {
            let light = (x + z).rem_euclid(2) == 0;
            let color = if light { [0.36, 0.42, 0.33] } else { [0.3, 0.36, 0.28] };
            let base = mesh.vertices.len() as u32;
            for (dx, dz) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
                mesh.vertices.push(Vertex {
                    position: [(x + dx) as f32, 0.0, (z + dz) as f32],
                    normal: [0.0, 1.0, 0.0],
                    color,
                    uv: [0.0, 0.0],
                });
            }
            mesh.indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    mesh
}

/// Einzelbetrachter als eigenes Programm (`game --ansehen <datei>`).
impl Game for Viewer {
    fn init(&mut self, ctx: &mut Context) {
        self.setup(ctx);
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<f32>().ok());
        if let Some(hour) = value("--uhrzeit") {
            self.day.hour = hour.rem_euclid(24.0);
        }
        if let Some(winkel) = value("--drehen") {
            ctx.camera.yaw = winkel.to_radians();
        }
        if let Some(winkel) = value("--neigung") {
            ctx.camera.pitch = (-winkel).to_radians().clamp(-1.45, 0.6);
        }
        self.fixed_zoom = value("--zoom");
        self.fixed_target = args.iter().position(|a| a == "--ziel").and_then(|i| args.get(i + 1)).and_then(|v| {
            let p: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            (p.len() == 3).then(|| vec3(p[0], p[1], p[2]))
        });
        self.freeze_at = value("--standbild");
        if args.iter().any(|a| a == "--ohne-vergleich") {
            self.show_reference = false;
        }
        // `--dazu <datei> x,y,z`: ein zweites Modell daneben zeigen (z. B. Marktplatz in der Burganlage)
        if let Some(i) = args.iter().position(|a| a == "--dazu") {
            let offset: Vec<f32> = args.get(i + 2).map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect()).unwrap_or_default();
            match (args.get(i + 1).map(|f| Model::from_file(Path::new(f))), offset.len()) {
                (Some(Ok(model)), 3) => {
                    let textures = model.register_textures(&mut ctx.assets);
                    for (info, mesh) in model.static_mesh_groups(&textures) {
                        let id = ctx.assets.add_mesh(mesh);
                        let material = if info.emissive > 0.0 { Material::Emissive { glow: 1.5 } } else { Material::Standard };
                        let at = Transform::from_position(vec3(offset[0], offset[1], offset[2]));
                        ctx.scene.spawn(Entity::new("Dazu", id).with_material(material).with_transform(at));
                    }
                }
                (Some(Err(message)), _) => log::error!("--dazu: {message}"),
                _ => log::error!("--dazu <datei> x,y,z"),
            }
        }
        match &self.source {
            Some(source) if blender::needs_build(source) => self.start_build(),
            Some(source) => self.source_stamp = blender::source_stamp(source),
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        Viewer::update(self, ctx);
    }

    fn ui(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        Viewer::ui(self, ctx, egui_ctx);
    }
}
