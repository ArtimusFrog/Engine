//! Asset-Betrachter: zeigt ein Modell aus Blender im Licht des Spiels.
//!
//! `game --ansehen game/assets/tiere/fuchs.gltf` öffnet das Modell auf einem Boden mit
//! 1-Meter-Karos, daneben eine Spielfigur als Größenvergleich. Wird die Datei neu
//! exportiert, lädt der Betrachter sie von selbst neu.
//!
//! Weitere Schalter (für automatische Screenshots): `--animation <name>`, `--uhrzeit <h>`,
//! `--drehen <grad>`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use engine::egui::{self, RichText};
use engine::mesh::Vertex;
use engine::prelude::*;

use crate::blender::{self, Build, BuildState};
use crate::characters::Puppet;
use crate::protocol::CharacterClass;
use crate::ui;

/// So oft wird geschaut, ob die Datei neu exportiert wurde (Sekunden).
const POLL_INTERVAL: f32 = 0.5;
/// Abstand der Vergleichsfigur zum Modell in Metern.
const REFERENCE_GAP: f32 = 0.8;

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
            day: DayCycle { hour: 10.5, ..Default::default() },
            speed: 1.0,
            playing: true,
            turntable: false,
            reference: None,
            show_reference: true,
            distance: 5.0,
            pointer_over_ui: false,
            themed: false,
        }
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
        if !self.path.exists() {
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
        let textures = model.register_textures(&mut ctx.assets, &format!("ansehen{}", self.generation));
        let mut entities = Vec::new();
        let meshes: Vec<MeshData>;
        let mut animated = None;
        if model.clips.is_empty() {
            meshes = model.static_meshes(&textures);
            for mesh in &meshes {
                let id = ctx.assets.add_mesh(mesh.clone());
                entities.push(ctx.scene.spawn(Entity::new("Modell", id)));
            }
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
        let first = self.shown.is_none() && self.reloaded_at.is_none();
        self.shown = Some(Shown {
            problems: crate::asset_files::check_model(&self.path, &model),
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
            let size = (max - min).max_element().max(0.5);
            self.distance = (size * 2.2).clamp(2.0, 60.0);
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
                let puppet = Puppet::new(ctx, CharacterClass::Knight, root);
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

impl Game for Viewer {
    fn init(&mut self, ctx: &mut Context) {
        let floor = ctx.assets.add_mesh(checker_floor(30));
        ctx.scene.spawn(Entity::new("Boden", floor));
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<f32>().ok());
        if let Some(hour) = value("--uhrzeit") {
            self.day.hour = hour.rem_euclid(24.0);
        }
        ctx.camera.yaw = value("--drehen").map_or(0.5, f32::to_radians);
        ctx.camera.pitch = -0.3;
        self.stamp = self.files_stamp();
        // Erst die vorhandene Fassung zeigen, dann bei Bedarf im Hintergrund neu bauen.
        self.load(ctx);
        match &self.source {
            Some(source) if blender::needs_build(source) => self.start_build(),
            Some(source) => self.source_stamp = blender::source_stamp(source),
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Context) {
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
            if self.playing {
                animator.update(dt);
            }
            ctx.assets.update_mesh(*mesh, animator.skinned_mesh(*texture));
        }
        if let Some((root, puppet)) = &mut self.reference {
            let position = ctx.scene.get(*root).transform.position;
            puppet.update(ctx, position, 0.0);
            puppet.set_visible(ctx, self.show_reference);
        }

        // Kamera: linke Maustaste ziehen = drehen, Mausrad = Abstand.
        if !self.pointer_over_ui {
            if ctx.input.mouse(MouseButton::Left) {
                let delta = ctx.input.mouse_delta() * 0.005;
                ctx.camera.yaw += delta.x;
                ctx.camera.pitch = (ctx.camera.pitch - delta.y).clamp(-1.45, 0.6);
            }
            self.distance = (self.distance * 0.9f32.powf(ctx.input.scroll())).clamp(0.5, 120.0);
        }
        if self.turntable {
            ctx.camera.yaw += dt * 0.4;
        }
        let target = self.center();
        ctx.camera.position = target - ctx.camera.forward() * self.distance;
        ctx.camera.near = (self.distance * 0.01).clamp(0.02, 0.1);

        self.day.apply(&mut ctx.env);
        let size = self.shown.as_ref().map_or(2.0, |s| (s.max - s.min).max_element());
        ctx.env.shadow_range = (size * 1.5 + 3.0).clamp(6.0, 60.0);
    }

    fn ui(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        if !self.themed {
            ui::apply_theme(egui_ctx);
            self.themed = true;
        }
        let mut reload = false;
        egui::Window::new("Asset-Betrachter")
            .anchor(egui::Align2::LEFT_TOP, [12.0, 12.0])
            .resizable(false)
            .collapsible(true)
            .frame(ui::panel_frame())
            .show(egui_ctx, |ui| {
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
                if let Some(shown) = &mut self.shown {
                    let size = shown.max - shown.min;
                    ui.label(format!("Größe: {:.2} × {:.2} × {:.2} m (B × H × T)", size.x, size.y, size.z));
                    ui.label(RichText::new(format!("Unterkante bei {:.2} m", shown.min.y)).color(ui::MUTED));
                    ui.label(format!("{} Dreiecke, {} Eckpunkte", shown.triangles, shown.vertices));
                    for problem in &shown.problems {
                        ui.label(RichText::new(format!("⚠ {problem}")).color(ui::ERROR));
                    }
                    ui.label(format!(
                        "{} Materialien, {} Texturen",
                        shown.model.materials.len().saturating_sub(1),
                        shown.model.images.len()
                    ));

                    if let Some((animator, _, _)) = &mut shown.animated {
                        ui.separator();
                        ui.label(RichText::new("Animationen").strong());
                        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
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
                ui.add(egui::Slider::new(&mut self.day.hour, 0.0..=24.0).text("Uhrzeit"));
                ui.checkbox(&mut self.turntable, "Drehteller");
                ui.checkbox(&mut self.show_reference, "Vergleichsfigur (1,8 m)");
                reload = ui.button("Neu laden").clicked();
                ui.label(RichText::new("Linke Maustaste ziehen: drehen · Mausrad: zoomen").color(ui::MUTED).size(12.0));
            });
        self.pointer_over_ui = egui_ctx.is_pointer_over_egui() || egui_ctx.egui_is_using_pointer();
        if reload {
            self.load(ctx);
        }
    }
}
