use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{UVec2, Vec2, Vec3};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

use crate::assets::Assets;
use crate::camera::Camera;
use crate::input::{Input, KeyCode};
use crate::physics::Physics;
use crate::renderer::{RenderStats, Renderer, UiFrame};
use crate::scene::Scene;

/// Das Spiel.
///
/// Ablauf pro Frame: so oft wie nötig `fixed_update` + Physik-Takt (60 Hz), danach
/// einmal `update`, dann `ui` und das Zeichnen.
pub trait Game: 'static {
    /// Einmal beim Start.
    fn init(&mut self, ctx: &mut Context);

    /// In festen Takten von `Physics::FIXED_DT`, direkt vor jedem Physik-Takt.
    /// Gehört hier hin: alles, was die Spielwelt verändert (Bewegung, Treffer, Regeln).
    fn fixed_update(&mut self, _ctx: &mut Context) {}

    /// Einmal pro Bild. Gehört hier hin: Kamera, Effekte.
    fn update(&mut self, ctx: &mut Context);

    /// Einmal pro Bild nach `update`: Menüs und Anzeigen mit egui zeichnen.
    fn ui(&mut self, _ctx: &mut Context, _ui: &egui::Context) {}
}

pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig { title: "Engine JN".into(), width: 1280, height: 720 }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Time {
    /// Sekunden seit dem letzten Frame.
    pub delta: f32,
    /// Sekunden seit dem Start.
    pub elapsed: f32,
    pub frame: u64,
    /// Anzahl der bisherigen Physik-Takte.
    pub tick: u64,
}

/// Licht und Atmosphäre der Welt. Alle Farben in linearem RGB.
#[derive(Clone, Copy, Debug)]
pub struct Environment {
    pub sky_color: Vec3,
    /// Richtung *zur* Sonne.
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sky_ambient: Vec3,
    pub ground_ambient: Vec3,
    pub fog_density: f32,
    /// Halbe Kantenlänge des Bereichs mit Schatten in Metern. Größer = mehr Schatten
    /// sichtbar, aber unschärfer.
    pub shadow_range: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            sky_color: Vec3::new(0.45, 0.65, 0.95),
            sun_direction: Vec3::new(0.6, 0.55, 0.35),
            sun_color: Vec3::new(1.0, 0.95, 0.85),
            sky_ambient: Vec3::new(0.22, 0.28, 0.38),
            ground_ambient: Vec3::new(0.12, 0.1, 0.08),
            fog_density: 0.006,
            shadow_range: 35.0,
        }
    }
}

/// Anzeige-Einstellungen. Änderungen übernimmt die Engine automatisch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Default for Display {
    fn default() -> Self {
        Display { fullscreen: false, vsync: true }
    }
}

/// Messwerte für die Debug-Anzeige (F3).
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub fps: f32,
    pub frame_ms: f32,
    pub render: RenderStats,
}

/// Alles, worauf das Spiel zugreifen kann.
pub struct Context {
    pub scene: Scene,
    pub physics: Physics,
    pub camera: Camera,
    pub input: Input,
    pub assets: Assets,
    pub env: Environment,
    pub time: Time,
    pub display: Display,
    /// Mauszeiger ausblenden und im Fenster festhalten (z. B. zum Umschauen).
    pub cursor_locked: bool,
    /// Kurzer Text für den Fenstertitel (z. B. Verbindungsstatus).
    pub status: String,
    /// Debug-Anzeige (F3) sichtbar?
    pub show_debug: bool,
    /// Zusätzliche Zeilen für die Debug-Anzeige; werden nach jedem Frame geleert.
    pub debug_lines: Vec<String>,
    pub stats: FrameStats,
    /// Name der Grafikkarte und Grafikschnittstelle.
    pub gpu: String,
    window_size: UVec2,
    pixels_per_point: f32,
    exit_requested: bool,
    headless: bool,
}

impl Context {
    fn new() -> Self {
        Context {
            scene: Scene::default(),
            physics: Physics::new(),
            camera: Camera::default(),
            input: Input::default(),
            assets: Assets::new(),
            env: Environment::default(),
            time: Time::default(),
            display: Display::default(),
            cursor_locked: false,
            status: String::new(),
            show_debug: false,
            debug_lines: Vec::new(),
            stats: FrameStats::default(),
            gpu: String::new(),
            window_size: UVec2::ONE,
            pixels_per_point: 1.0,
            exit_requested: false,
            headless: false,
        }
    }

    /// Kontext ohne Fenster, z. B. für Tests. Die Takte treibt man selbst mit
    /// [`fixed_tick`](Self::fixed_tick) an.
    pub fn headless() -> Self {
        Context { headless: true, ..Self::new() }
    }

    /// Läuft das Spiel ohne Fenster (dedizierter Server, Tests)?
    pub fn is_headless(&self) -> bool {
        self.headless
    }

    /// Ein fester Takt: Spiellogik, dann Physik.
    pub fn fixed_tick(&mut self, game: &mut dyn Game) {
        self.physics.begin_tick();
        game.fixed_update(self);
        self.physics.step();
        self.time.tick += 1;
    }

    /// Überträgt die Physik-Positionen in die Szene (sonst macht das die Engine pro Frame).
    pub fn sync_scene(&mut self) {
        self.physics.sync_to_scene(&mut self.scene, 1.0);
    }

    /// Leert Szene und Physik, z. B. beim Wechsel vom Menü ins Spiel. Meshes bleiben geladen.
    pub fn reset_world(&mut self) {
        self.scene = Scene::default();
        self.physics = Physics::new();
    }

    pub fn window_size(&self) -> UVec2 {
        self.window_size
    }

    /// Rechnet einen Punkt der Welt in Bildschirmkoordinaten der UI (egui-Punkte) um.
    /// `None`, wenn der Punkt hinter der Kamera liegt.
    pub fn world_to_screen(&self, point: Vec3) -> Option<Vec2> {
        let size = self.window_size.as_vec2();
        let clip = self.camera.view_projection(size.x / size.y) * point.extend(1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let pixels = Vec2::new((ndc.x * 0.5 + 0.5) * size.x, (0.5 - ndc.y * 0.5) * size.y);
        Some(pixels / self.pixels_per_point)
    }

    /// Beendet das Spiel nach dem aktuellen Frame.
    pub fn exit(&mut self) {
        self.exit_requested = true;
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }
}

fn init_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,wgpu_hal=error,engine=info,game=info"))
        .try_init();
}

/// Startet die Engine mit dem gegebenen Spiel. Kehrt zurück, wenn das Fenster geschlossen wird.
///
/// Kommandozeile:
/// - `--screenshot <datei.png>`: nach einigen Frames ein Bild speichern und beenden
/// - `--frames <n>`: Anzahl Frames vor dem Screenshot (Standard 30)
pub fn run(config: EngineConfig, game: impl Game) {
    init_logging();

    let mut app = App {
        config,
        game: Box::new(game),
        ctx: Context::new(),
        window: None,
        renderer: None,
        ui: None,
        initialized: false,
        applied_cursor_locked: false,
        applied_display: Display::default(),
        last_frame: Instant::now(),
        start: Instant::now(),
        accumulator: 0.0,
        fps_timer: 0.0,
        fps_frames: 0,
        auto_screenshot: AutoScreenshot::from_args(),
    };

    let event_loop = EventLoop::new().expect("Ereignisschleife konnte nicht erstellt werden");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("Engine ist abgestürzt");
}

/// Startet das Spiel ohne Fenster, z. B. als dedizierter Server. Ruft nur `init` und
/// `fixed_update` auf (60-mal pro Sekunde), nie `update` oder `ui`. Läuft, bis `ctx.exit()` kommt.
pub fn run_headless(mut game: impl Game) {
    init_logging();
    let mut ctx = Context::headless();
    game.init(&mut ctx);

    let tick = Duration::from_secs_f32(Physics::FIXED_DT);
    let start = Instant::now();
    let mut next = Instant::now();
    while !ctx.exit_requested {
        ctx.time.delta = Physics::FIXED_DT;
        ctx.time.elapsed = (Instant::now() - start).as_secs_f32();
        ctx.fixed_tick(&mut game);
        ctx.time.frame += 1;

        next += tick;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > Duration::from_secs(1) {
            log::warn!("Server kommt nicht hinterher, überspringe {:.1} s", (now - next).as_secs_f32());
            next = now;
        }
    }
}

struct AutoScreenshot {
    path: PathBuf,
    after_frames: u64,
}

impl AutoScreenshot {
    fn from_args() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1));
        let path = value("--screenshot")?;
        let after_frames = value("--frames").and_then(|n| n.parse().ok()).unwrap_or(30);
        Some(AutoScreenshot { path: path.into(), after_frames })
    }
}

/// egui-Zustand, der zum Fenster gehört.
struct UiState {
    ctx: egui::Context,
    winit: egui_winit::State,
}

struct App {
    config: EngineConfig,
    game: Box<dyn Game>,
    ctx: Context,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    ui: Option<UiState>,
    initialized: bool,
    applied_cursor_locked: bool,
    applied_display: Display,
    last_frame: Instant,
    start: Instant,
    accumulator: f32,
    fps_timer: f32,
    fps_frames: u32,
    auto_screenshot: Option<AutoScreenshot>,
}

impl App {
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(renderer), Some(ui)) = (&self.window, &mut self.renderer, &mut self.ui) else { return };

        let now = Instant::now();
        let frame_time = (now - self.last_frame).as_secs_f32();
        // Begrenzen, damit nach Hängern (Fenster verschieben, Debugger) nichts durch Wände springt.
        self.ctx.time.delta = frame_time.min(0.1);
        self.ctx.time.elapsed = (now - self.start).as_secs_f32();
        self.last_frame = now;

        // Feste Takte nachholen. Höchstens 5 pro Frame, sonst schaukelt sich ein
        // langsamer Rechner immer weiter auf.
        self.accumulator += self.ctx.time.delta;
        let mut steps = 0;
        while self.accumulator >= Physics::FIXED_DT && steps < 5 {
            self.ctx.fixed_tick(self.game.as_mut());
            self.accumulator -= Physics::FIXED_DT;
            steps += 1;
        }
        self.accumulator = self.accumulator.min(Physics::FIXED_DT);
        let alpha = self.accumulator / Physics::FIXED_DT;
        self.ctx.physics.sync_to_scene(&mut self.ctx.scene, alpha);

        if self.ctx.input.key_pressed(KeyCode::F3) {
            self.ctx.show_debug = !self.ctx.show_debug;
        }
        self.game.update(&mut self.ctx);

        // Benutzeroberfläche
        self.ctx.pixels_per_point = egui_winit::pixels_per_point(&ui.ctx, window);
        let raw_input = ui.winit.take_egui_input(window);
        ui.ctx.begin_pass(raw_input);
        self.game.ui(&mut self.ctx, &ui.ctx);
        if self.ctx.show_debug {
            debug_overlay(&self.ctx, &ui.ctx);
        }
        let output = ui.ctx.end_pass();
        ui.winit.handle_platform_output(window, output.platform_output);
        let mut ui_frame = UiFrame {
            primitives: ui.ctx.tessellate(output.shapes, output.pixels_per_point),
            textures_delta: output.textures_delta,
            pixels_per_point: output.pixels_per_point,
        };

        // Wünsche des Spiels ans Fenster übernehmen
        if self.ctx.cursor_locked != self.applied_cursor_locked {
            self.applied_cursor_locked = self.ctx.cursor_locked;
            set_cursor_locked(window, self.applied_cursor_locked);
        }
        if self.ctx.display != self.applied_display {
            if self.ctx.display.fullscreen != self.applied_display.fullscreen {
                window.set_fullscreen(self.ctx.display.fullscreen.then_some(Fullscreen::Borderless(None)));
            }
            if self.ctx.display.vsync != self.applied_display.vsync {
                renderer.set_vsync(self.ctx.display.vsync);
            }
            self.applied_display = self.ctx.display;
        }

        renderer.render(&self.ctx, Some(&ui_frame));

        if self.ctx.input.key_pressed(KeyCode::F12) {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let path = PathBuf::from(format!("screenshots/screenshot-{secs}.png"));
            match renderer.screenshot(&self.ctx, Some(&ui_frame), &path) {
                Ok(()) => log::info!("Screenshot gespeichert: {}", path.display()),
                Err(e) => log::error!("Screenshot fehlgeschlagen: {e}"),
            }
        }
        if let Some(shot) = &self.auto_screenshot {
            if self.ctx.time.frame + 1 >= shot.after_frames {
                match renderer.screenshot(&self.ctx, Some(&ui_frame), &shot.path) {
                    Ok(()) => println!("Screenshot gespeichert: {}", shot.path.display()),
                    Err(e) => eprintln!("Screenshot fehlgeschlagen: {e}"),
                }
                event_loop.exit();
            }
        }

        self.fps_timer += frame_time;
        self.fps_frames += 1;
        if self.fps_timer >= 0.5 {
            let fps = self.fps_frames as f32 / self.fps_timer;
            self.ctx.stats.fps = fps;
            self.ctx.stats.frame_ms = 1000.0 / fps;
            let status = if self.ctx.status.is_empty() { String::new() } else { format!(" – {}", self.ctx.status) };
            window.set_title(&format!("{}{status}", self.config.title));
            self.fps_timer = 0.0;
            self.fps_frames = 0;
        }
        self.ctx.stats.render = renderer.stats();
        // Texturänderungen sind jetzt auf der Grafikkarte angekommen.
        ui_frame.textures_delta.clear();

        self.ctx.input.end_frame();
        self.ctx.debug_lines.clear();
        self.ctx.time.frame += 1;
        if self.ctx.exit_requested {
            event_loop.exit();
        }
    }
}

fn debug_overlay(ctx: &Context, ui: &egui::Context) {
    egui::Area::new(egui::Id::new("engine_debug"))
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .interactable(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(170))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    let s = &ctx.stats;
                    let text = |ui: &mut egui::Ui, t: String| {
                        ui.label(egui::RichText::new(t).monospace().color(egui::Color32::from_gray(230)));
                    };
                    text(ui, format!("{:.0} FPS  ({:.1} ms)", s.fps, s.frame_ms));
                    text(ui, ctx.gpu.clone());
                    text(ui, format!("Objekte: {}  Draw-Calls: {}", s.render.instances, s.render.draw_calls));
                    text(ui, format!("Takt: {}  VSync: {}", ctx.time.tick, if ctx.display.vsync { "an" } else { "aus" }));
                    let p = ctx.camera.position;
                    text(ui, format!("Kamera: {:.1} {:.1} {:.1}", p.x, p.y, p.z));
                    for line in &ctx.debug_lines {
                        text(ui, line.clone());
                    }
                });
        });
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = Arc::new(event_loop.create_window(attributes).expect("Fenster konnte nicht erstellt werden"));
        let size = window.inner_size();
        self.ctx.window_size = UVec2::new(size.width.max(1), size.height.max(1));
        let renderer = pollster::block_on(Renderer::new(window.clone()));
        self.ctx.gpu = renderer.adapter_name().to_string();
        self.renderer = Some(renderer);

        let egui_ctx = egui::Context::default();
        let viewport = egui_ctx.viewport_id();
        let winit_state =
            egui_winit::State::new(egui_ctx.clone(), viewport, &window, Some(window.scale_factor() as f32), None, None);
        self.ui = Some(UiState { ctx: egui_ctx, winit: winit_state });
        self.window = Some(window);

        if !self.initialized {
            self.game.init(&mut self.ctx);
            self.initialized = true;
        }
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Zuerst darf die Benutzeroberfläche reagieren. Braucht sie die Eingabe (Klick auf
        // einen Knopf, Tippen in ein Textfeld), bekommt das Spiel sie nicht.
        let consumed = match (&mut self.ui, &self.window) {
            (Some(ui), Some(window)) if !self.ctx.cursor_locked => ui.winit.on_window_event(window, &event).consumed,
            _ => false,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.ctx.window_size = UVec2::new(size.width.max(1), size.height.max(1));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Loslassen immer weitergeben, sonst bleiben Tasten "hängen".
                if !consumed || event.state == winit::event::ElementState::Released {
                    self.ctx.input.on_key(event.physical_key, event.state, event.repeat);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if !consumed || state == winit::event::ElementState::Released {
                    self.ctx.input.on_mouse_button(button, state);
                }
            }
            WindowEvent::MouseWheel { delta, .. } if !consumed => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 100.0,
                };
                self.ctx.input.on_scroll(steps);
            }
            WindowEvent::Focused(false) => self.ctx.input.release_all(),
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.ctx.input.on_mouse_motion(delta.0, delta.1);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn set_cursor_locked(window: &Window, locked: bool) {
    if locked {
        // "Locked" gibt es unter Windows nicht, dort hält "Confined" den Zeiger im Fenster.
        if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            let _ = window.set_cursor_grab(CursorGrabMode::Confined);
        }
    } else {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
    }
    window.set_cursor_visible(!locked);
}
