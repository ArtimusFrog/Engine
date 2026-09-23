use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use glam::{UVec2, Vec3};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::assets::Assets;
use crate::camera::Camera;
use crate::input::{Input, KeyCode};
use crate::renderer::Renderer;
use crate::scene::Scene;

/// Das Spiel. Die Engine ruft `init` einmal beim Start und `update` einmal pro Frame auf.
pub trait Game: 'static {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context);
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
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            sky_color: Vec3::new(0.45, 0.65, 0.95),
            sun_direction: Vec3::new(0.4, 0.8, 0.3),
            sun_color: Vec3::new(1.0, 0.95, 0.85),
            sky_ambient: Vec3::new(0.22, 0.28, 0.38),
            ground_ambient: Vec3::new(0.12, 0.1, 0.08),
            fog_density: 0.006,
        }
    }
}

/// Alles, worauf das Spiel zugreifen kann.
pub struct Context {
    pub scene: Scene,
    pub camera: Camera,
    pub input: Input,
    pub assets: Assets,
    pub env: Environment,
    pub time: Time,
    /// Mauszeiger ausblenden und im Fenster festhalten (z. B. zum Umschauen).
    pub cursor_locked: bool,
    window_size: UVec2,
    exit_requested: bool,
}

impl Context {
    fn new() -> Self {
        Context {
            scene: Scene::default(),
            camera: Camera::default(),
            input: Input::default(),
            assets: Assets::new(),
            env: Environment::default(),
            time: Time::default(),
            cursor_locked: false,
            window_size: UVec2::ONE,
            exit_requested: false,
        }
    }

    pub fn window_size(&self) -> UVec2 {
        self.window_size
    }

    /// Beendet das Spiel nach dem aktuellen Frame.
    pub fn exit(&mut self) {
        self.exit_requested = true;
    }
}

/// Startet die Engine mit dem gegebenen Spiel. Kehrt zurück, wenn das Fenster geschlossen wird.
///
/// Kommandozeile:
/// - `--screenshot <datei.png>`: nach einigen Frames ein Bild speichern und beenden
/// - `--frames <n>`: Anzahl Frames vor dem Screenshot (Standard 30)
pub fn run(config: EngineConfig, game: impl Game) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,wgpu_hal=error,engine=info,game=info")).init();

    let mut app = App {
        config,
        game: Box::new(game),
        ctx: Context::new(),
        window: None,
        renderer: None,
        initialized: false,
        cursor_locked: false,
        last_frame: Instant::now(),
        start: Instant::now(),
        fps_timer: 0.0,
        fps_frames: 0,
        auto_screenshot: AutoScreenshot::from_args(),
    };

    let event_loop = EventLoop::new().expect("Ereignisschleife konnte nicht erstellt werden");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("Engine ist abgestürzt");
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

struct App {
    config: EngineConfig,
    game: Box<dyn Game>,
    ctx: Context,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    initialized: bool,
    cursor_locked: bool,
    last_frame: Instant,
    start: Instant,
    fps_timer: f32,
    fps_frames: u32,
    auto_screenshot: Option<AutoScreenshot>,
}

impl App {
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else { return };

        let now = Instant::now();
        // Begrenzen, damit nach Hängern (Fenster verschieben, Debugger) nichts durch Wände springt.
        self.ctx.time.delta = (now - self.last_frame).as_secs_f32().min(0.1);
        self.ctx.time.elapsed = (now - self.start).as_secs_f32();
        self.last_frame = now;

        self.game.update(&mut self.ctx);

        if self.ctx.cursor_locked != self.cursor_locked {
            self.cursor_locked = self.ctx.cursor_locked;
            set_cursor_locked(window, self.cursor_locked);
        }

        renderer.render(&self.ctx);

        if self.ctx.input.key_pressed(KeyCode::F12) {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let path = PathBuf::from(format!("screenshots/screenshot-{secs}.png"));
            match renderer.screenshot(&self.ctx, &path) {
                Ok(()) => log::info!("Screenshot gespeichert: {}", path.display()),
                Err(e) => log::error!("Screenshot fehlgeschlagen: {e}"),
            }
        }
        if let Some(shot) = &self.auto_screenshot {
            if self.ctx.time.frame + 1 >= shot.after_frames {
                match renderer.screenshot(&self.ctx, &shot.path) {
                    Ok(()) => println!("Screenshot gespeichert: {}", shot.path.display()),
                    Err(e) => eprintln!("Screenshot fehlgeschlagen: {e}"),
                }
                event_loop.exit();
            }
        }

        self.fps_timer += self.ctx.time.delta;
        self.fps_frames += 1;
        if self.fps_timer >= 0.5 {
            let fps = self.fps_frames as f32 / self.fps_timer;
            window.set_title(&format!("{} – {fps:.0} FPS – {}", self.config.title, renderer.adapter_name()));
            self.fps_timer = 0.0;
            self.fps_frames = 0;
        }

        self.ctx.input.end_frame();
        self.ctx.time.frame += 1;
        if self.ctx.exit_requested {
            event_loop.exit();
        }
    }
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
        self.renderer = Some(pollster::block_on(Renderer::new(window.clone())));
        self.window = Some(window);

        if !self.initialized {
            self.game.init(&mut self.ctx);
            self.initialized = true;
        }
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.ctx.window_size = UVec2::new(size.width.max(1), size.height.max(1));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.ctx.input.on_key(event.physical_key, event.state, event.repeat);
            }
            WindowEvent::MouseInput { state, button, .. } => self.ctx.input.on_mouse_button(button, state),
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
