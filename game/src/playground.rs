//! Das Spiel als Ganzes: Menüs, Wechsel zwischen Menü und Runde, Eingabe, Kamera und HUD.

use engine::egui::{self, Align2, Color32, RichText};
use engine::prelude::*;

use crate::protocol::{PlayerInput, DEFAULT_PORT, MAX_NAME_CHARS};
use crate::session::{Mode, Session};
use crate::settings::Settings;
use crate::sounds::Sounds;
use crate::ui;
use crate::world::World;

/// Nach so vielen Sekunden ohne Antwort gibt der Verbindungsaufbau auf.
const CONNECT_TIMEOUT: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    MainMenu,
    Join,
    Settings,
    Connecting,
    Playing,
    Paused,
}

pub struct Playground {
    /// Direkt in eine Runde starten (Kommandozeile), statt ins Hauptmenü.
    start: Option<Mode>,
    settings: Settings,
    session: Option<Session>,
    /// Kulisse hinter dem Hauptmenü.
    menu_world: Option<World>,
    screen: Screen,
    /// Wohin „Zurück“ aus den Einstellungen führt.
    settings_return: Screen,
    join_address: String,
    error: Option<String>,
    connect_started: f32,
    orbit: OrbitController,
    fly: FlyController,
    free_camera: bool,
    // Eingaben, die zwischen zwei Takten fallen, bis zum nächsten Takt merken.
    jump_requested: bool,
    throw_requested: bool,
    harvest_requested: Option<u32>,
    /// Rohstoff unter dem Fadenkreuz, der in Reichweite ist.
    aim: Option<u32>,
    last_harvest: f32,
    /// Nur zum Testen: Figur läuft von allein.
    autopilot: bool,
    themed: bool,
    /// Eigene Adresse im Heimnetz, damit Mitspieler wissen, wohin sie sich verbinden.
    local_ip: Option<std::net::IpAddr>,
    /// Nur zum Testen: Figur hackt regelmäßig in die Luft.
    demo_chop: bool,
    /// Geräusche und Klangkulisse (nur mit Fenster).
    sounds: Option<Sounds>,
}

impl Playground {
    pub fn new(start: Option<Mode>, autopilot: bool) -> Self {
        let settings = Settings::load();
        Playground {
            start,
            join_address: settings.last_address.clone(),
            settings,
            session: None,
            menu_world: None,
            screen: Screen::MainMenu,
            settings_return: Screen::MainMenu,
            error: None,
            connect_started: 0.0,
            orbit: OrbitController::default(),
            fly: FlyController::default(),
            free_camera: false,
            jump_requested: false,
            throw_requested: false,
            harvest_requested: None,
            aim: None,
            last_harvest: 0.0,
            autopilot,
            themed: false,
            local_ip: None,
            demo_chop: false,
            sounds: None,
        }
    }

    fn apply_settings(&mut self, ctx: &mut Context) {
        self.settings.apply(ctx);
        self.orbit.sensitivity = 0.0025 * self.settings.mouse_sensitivity;
        self.orbit.invert_y = self.settings.invert_y;
        self.fly.sensitivity = self.orbit.sensitivity;
    }

    fn start_session(&mut self, ctx: &mut Context, mode: Mode) {
        match Session::start(ctx, mode, &self.settings.hello()) {
            Ok(session) => {
                self.screen = if session.is_connecting() { Screen::Connecting } else { Screen::Playing };
                self.session = Some(session);
                self.menu_world = None;
                self.error = None;
                self.connect_started = ctx.time.elapsed;
                self.local_ip = ui::local_ip();
                self.orbit.distance = 6.0;
                ctx.camera.pitch = -0.35;
                ctx.cursor_locked = self.screen == Screen::Playing && !ctx.is_headless();
            }
            Err(message) => {
                log::error!("{message}");
                self.show_menu(ctx, Some(message));
            }
        }
    }

    /// Beendet die laufende Runde (falls vorhanden) und zeigt das Hauptmenü.
    fn show_menu(&mut self, ctx: &mut Context, error: Option<String>) {
        self.session = None;
        ctx.reset_world();
        let mut world = World::new(ctx);
        // Im Menü immer goldene Abendstimmung.
        world.day.hour = 17.6;
        self.menu_world = Some(world);
        self.screen = Screen::MainMenu;
        self.error = error;
        self.free_camera = false;
        ctx.cursor_locked = false;
    }

    fn build_input(&mut self, ctx: &Context) -> PlayerInput {
        let playing = self.screen == Screen::Playing && !self.free_camera;
        let yaw = ctx.camera.yaw;
        let (forward, right) = (vec2(yaw.sin(), -yaw.cos()), vec2(yaw.cos(), yaw.sin()));
        let mut wish = Vec2::ZERO;
        if playing {
            for (key, dir) in [(KeyCode::KeyW, forward), (KeyCode::KeyS, -forward), (KeyCode::KeyD, right), (KeyCode::KeyA, -right)] {
                if ctx.input.key(key) {
                    wish += dir;
                }
            }
        }
        if self.autopilot {
            wish = forward;
            self.jump_requested |= ctx.time.tick % 120 == 60;
        }
        let input = PlayerInput {
            seq: 0,
            wish: wish.normalize_or_zero(),
            sprint: playing && ctx.input.key(KeyCode::ShiftLeft),
            jump: self.jump_requested,
            throw: self.throw_requested.then(|| ctx.camera.forward()),
            harvest: self.harvest_requested.take(),
        };
        self.jump_requested = false;
        self.throw_requested = false;
        input
    }

    fn handle_game_keys(&mut self, ctx: &mut Context) {
        let escape = ctx.input.key_pressed(KeyCode::Escape);
        match self.screen {
            Screen::Playing => {
                if escape {
                    self.screen = Screen::Paused;
                    ctx.cursor_locked = false;
                    return;
                }
                if !ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) && !self.free_camera {
                    ctx.cursor_locked = true;
                } else if ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) {
                    self.throw_requested = true;
                    if let Some(session) = &mut self.session {
                        session.preview_throw();
                    }
                }
                self.jump_requested |= ctx.input.key_pressed(KeyCode::Space);
                // Rechte Maustaste halten: im Takt der Abklingzeit zuschlagen.
                let cooldown = crate::world::HARVEST_COOLDOWN_TICKS as f32 * Physics::FIXED_DT;
                if let (true, true, Some(id)) = (ctx.cursor_locked, ctx.input.mouse(MouseButton::Right), self.aim) {
                    if ctx.time.elapsed - self.last_harvest >= cooldown {
                        self.last_harvest = ctx.time.elapsed;
                        self.harvest_requested = Some(id);
                        if let Some(session) = &mut self.session {
                            session.preview_harvest(ctx, id);
                        }
                    }
                }
                // F6: eine Stunde vorspulen (nur wer die Welt berechnet)
                if ctx.input.key_pressed(KeyCode::F6) {
                    if let Some(session) = &mut self.session {
                        session.skip_time(1.0);
                    }
                }
                if ctx.input.key_pressed(KeyCode::F1) {
                    self.free_camera = !self.free_camera;
                    ctx.cursor_locked = false;
                }
            }
            Screen::Paused if escape => {
                self.screen = Screen::Playing;
                ctx.cursor_locked = true;
            }
            Screen::Settings if escape => self.close_settings(ctx),
            Screen::Join if escape => self.screen = Screen::MainMenu,
            _ => {}
        }
    }

    fn close_settings(&mut self, ctx: &mut Context) {
        self.settings.name = crate::protocol::clean_name(&self.settings.name);
        self.settings.save();
        self.apply_settings(ctx);
        self.screen = self.settings_return;
    }

    fn update_camera(&mut self, ctx: &mut Context) {
        let Some(session) = &self.session else {
            // Hauptmenü: Kamera kreist langsam um die Insel.
            let t = ctx.time.elapsed * 0.03 + 0.8;
            ctx.camera.position = vec3(t.sin() * 210.0, 70.0, t.cos() * 210.0);
            ctx.camera.look_at(vec3(0.0, 6.0, 0.0));
            return;
        };
        if self.free_camera {
            self.fly.update(ctx);
            return;
        }
        let Some(avatar) = session.local_player().and_then(|id| session.world().players.get(&id)) else { return };
        let Some(entity) = ctx.scene.try_get(avatar.entity) else { return };
        let target = entity.transform.position + Vec3::Y * 0.6;
        let character = avatar.character;
        self.orbit.update(ctx, target, Some(character));
    }

    /// Welcher Rohstoff liegt unter dem Fadenkreuz und ist nah genug?
    fn update_aim(&mut self, ctx: &Context) {
        self.aim = None;
        if self.screen != Screen::Playing || self.free_camera {
            return;
        }
        let Some(session) = &self.session else { return };
        let Some(local) = session.local_player() else { return };
        let Some(avatar) = session.world().players.get(&local) else { return };
        let hit = ctx.physics.raycast(ctx.camera.position, ctx.camera.forward(), 60.0, Some(avatar.character));
        if let Some((Some(entity), _)) = hit {
            self.aim = session.world().resource_at(entity).filter(|&id| session.world().in_reach(ctx, local, id, 0.0));
        }
    }

    fn inventory_hud(&self, egui_ctx: &egui::Context) {
        let Some(session) = &self.session else { return };
        let inventory = session.local_inventory();
        egui::Area::new(egui::Id::new("inventar"))
            .anchor(Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .interactable(false)
            .show(egui_ctx, |ui| {
                egui::Frame::new().fill(Color32::from_black_alpha(160)).corner_radius(8.0).inner_margin(10.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (label, count, color) in [
                            ("Holz", inventory.wood, Color32::from_rgb(150, 98, 52)),
                            ("Stein", inventory.stone, Color32::from_rgb(140, 142, 150)),
                        ] {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
                            if label == "Holz" {
                                ui.painter().rect_filled(rect.shrink2(egui::vec2(1.0, 6.0)), 4.0, color);
                                ui.painter().circle_stroke(
                                    rect.right_center() - egui::vec2(4.0, 0.0),
                                    5.0,
                                    egui::Stroke::new(2.0, Color32::from_rgb(90, 55, 25)),
                                );
                            } else {
                                ui.painter().circle_filled(rect.center(), 11.0, color);
                                ui.painter().circle_filled(rect.center() + egui::vec2(-3.0, -3.0), 4.0, Color32::from_rgb(175, 177, 185));
                            }
                            ui.label(RichText::new(format!("{count}")).size(22.0).strong().color(Color32::WHITE));
                            ui.label(RichText::new(label).size(14.0).color(ui::TEXT.gamma_multiply(0.8)));
                            ui.add_space(10.0);
                        }
                    });
                });
            });
    }

    /// Hinweis unter dem Fadenkreuz, wenn ein Rohstoff anvisiert ist.
    fn aim_hud(&self, egui_ctx: &egui::Context) {
        let (Some(id), Some(session)) = (self.aim, &self.session) else { return };
        let Some(resource) = session.world().resources.get(&id) else { return };
        let center = egui_ctx.content_rect().center();
        let painter = egui_ctx.layer_painter(egui::LayerId::background());
        let action = match resource.spec.kind {
            crate::island::ResourceKind::Wood => "Rechtsklick: Holz hacken",
            crate::island::ResourceKind::Stone => "Rechtsklick: Stein abbauen",
        };
        let big = egui::FontId::proportional(18.0);
        painter.text(center + egui::vec2(0.0, 28.0), Align2::CENTER_TOP, resource.spec.name, big, Color32::WHITE);
        let small = egui::FontId::proportional(14.0);
        painter.text(center + egui::vec2(0.0, 50.0), Align2::CENTER_TOP, action, small, Color32::from_white_alpha(200));
        // Lebensbalken
        let fraction = resource.health as f32 / resource.spec.max_health as f32;
        let bar = egui::Rect::from_center_size(center + egui::vec2(0.0, 76.0), egui::vec2(110.0, 7.0));
        painter.rect_filled(bar, 3.0, Color32::from_black_alpha(150));
        let mut fill = bar;
        fill.set_width(bar.width() * fraction);
        painter.rect_filled(fill, 3.0, ui::ACCENT);
    }

    fn status_text(&self) -> String {
        match self.session.as_ref().map(|s| (s.mode(), s)) {
            None => "Hauptmenü".into(),
            Some((Mode::Offline, _)) => "Einzelspieler".into(),
            Some((Mode::Host { .. } | Mode::Server { .. }, s)) => {
                format!("Host · Port {} · {} Spieler", s.port().unwrap_or(0), s.world().players.len())
            }
            Some((Mode::Join { address }, s)) => match s.ping_ms() {
                Some(ping) => format!("{address} · {ping:.0} ms · {} Spieler", s.world().players.len()),
                None => format!("Verbinde mit {address} …"),
            },
        }
    }

    // ---------- Bildschirme ----------

    fn main_menu(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let mut action = None;
        ui::left_shade(egui_ctx);
        egui::Area::new(egui::Id::new("hauptmenue")).anchor(Align2::LEFT_CENTER, [70.0, 0.0]).show(egui_ctx, |ui| {
            ui.label(RichText::new("ENGINE JN").size(64.0).strong().color(Color32::WHITE));
            ui.label(RichText::new("Fantasy-Insel · Multiplayer").size(18.0).color(ui::TEXT));
            ui.add_space(24.0);
            ui::panel_frame().show(ui, |ui| {
                ui.set_width(300.0);
                if ui::big_button(ui, "Einzelspieler").clicked() {
                    action = Some(Mode::Offline);
                }
                if ui::big_button(ui, "Spiel hosten").clicked() {
                    action = Some(Mode::Host { port: DEFAULT_PORT });
                }
                if ui::big_button(ui, "Beitreten").clicked() {
                    self.screen = Screen::Join;
                }
                if ui::big_button(ui, "Einstellungen").clicked() {
                    self.settings_return = Screen::MainMenu;
                    self.screen = Screen::Settings;
                }
                if ui::big_button(ui, "Beenden").clicked() {
                    ctx.exit();
                }
                if let Some(error) = &self.error {
                    ui.add_space(6.0);
                    ui.label(RichText::new(error).color(ui::ERROR));
                }
            });
            ui.add_space(10.0);
            ui.label(RichText::new(format!("Spielername: {}", self.settings.name)).size(15.0).color(ui::TEXT));
        });
        if let Some(mode) = action {
            self.start_session(ctx, mode);
        }
    }

    fn join_menu(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let mut connect = false;
        ui::dim_background(egui_ctx, 90);
        ui::center_panel(egui_ctx, "beitreten", 420.0, |ui| {
            ui::heading(ui, "Beitreten");
            ui.label(RichText::new("Adresse des Hosts oder Servers").color(ui::MUTED));
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.join_address)
                    .hint_text("z. B. 100.64.1.2 oder spiel.example.com")
                    .desired_width(f32::INFINITY),
            );
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                connect = true;
            }
            ui.label(
                RichText::new(format!("Ohne Angabe wird Port {DEFAULT_PORT} benutzt. Du spielst als „{}“.", self.settings.name))
                    .size(14.0)
                    .color(ui::MUTED),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add_sized([180.0, 42.0], egui::Button::new("Verbinden")).clicked() {
                    connect = true;
                }
                if ui.add_sized([180.0, 42.0], egui::Button::new("Zurück")).clicked() {
                    self.screen = Screen::MainMenu;
                }
            });
        });

        let address = self.join_address.trim().to_string();
        if connect && !address.is_empty() {
            self.settings.last_address = address.clone();
            self.settings.save();
            let address = if address.contains(':') { address } else { format!("{address}:{DEFAULT_PORT}") };
            self.start_session(ctx, Mode::Join { address });
        }
    }

    fn settings_menu(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let mut close = false;
        ui::dim_background(egui_ctx, 120);
        ui::center_panel(egui_ctx, "einstellungen", 470.0, |ui| {
            ui::heading(ui, "Einstellungen");
            egui::Grid::new("einstellungen_raster").num_columns(2).spacing([24.0, 14.0]).show(ui, |ui| {
                let s = &mut self.settings;
                ui.label("Name");
                ui.add(egui::TextEdit::singleline(&mut s.name).char_limit(MAX_NAME_CHARS).desired_width(220.0));
                ui.end_row();

                ui.label("Figur");
                egui::ComboBox::from_id_salt("figur")
                    .selected_text(s.character.label())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for class in crate::protocol::CharacterClass::ALL {
                            ui.selectable_value(&mut s.character, class, class.label());
                        }
                    });
                ui.end_row();

                ui.label("Mausempfindlichkeit");
                ui.add(egui::Slider::new(&mut s.mouse_sensitivity, 0.2..=3.0).fixed_decimals(1).suffix("×"));
                ui.end_row();

                ui.label("Maus-Y umkehren");
                ui.checkbox(&mut s.invert_y, "");
                ui.end_row();

                ui.label("Sichtfeld");
                ui.add(egui::Slider::new(&mut s.fov_degrees, 60.0..=110.0).fixed_decimals(0).suffix("°"));
                ui.end_row();

                ui.label("Vollbild");
                ui.checkbox(&mut s.fullscreen, "");
                ui.end_row();

                ui.label("VSync");
                ui.checkbox(&mut s.vsync, "");
                ui.end_row();

                let percent = |value: f64, _: std::ops::RangeInclusive<usize>| format!("{:.0} %", value * 100.0);
                for (label, value) in [
                    ("Lautstärke", &mut s.volume_master),
                    ("Effekte", &mut s.volume_effects),
                    ("Umgebung", &mut s.volume_ambient),
                    ("Musik", &mut s.volume_music),
                ] {
                    ui.label(label);
                    ui.add(egui::Slider::new(value, 0.0..=1.0).custom_formatter(percent));
                    ui.end_row();
                }
            });
            ui.add_space(10.0);
            if ui::big_button(ui, "Zurück").clicked() {
                close = true;
            }
        });
        // Änderungen sofort sichtbar machen, gespeichert wird beim Schließen.
        self.apply_settings(ctx);
        if close {
            self.close_settings(ctx);
        }
    }

    fn connecting_screen(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let mut cancel = false;
        let address = match self.session.as_ref().map(Session::mode) {
            Some(Mode::Join { address }) => address.clone(),
            _ => String::new(),
        };
        ui::dim_background(egui_ctx, 120);
        ui::center_panel(egui_ctx, "verbinden", 380.0, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(26.0).color(ui::ACCENT));
                ui.label(RichText::new(format!("Verbinde mit {address} …")).size(20.0));
            });
            ui.add_space(8.0);
            if ui::big_button(ui, "Abbrechen").clicked() {
                cancel = true;
            }
        });
        if cancel {
            self.show_menu(ctx, None);
        }
    }

    fn pause_menu(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        ui::dim_background(egui_ctx, 140);
        let online = !matches!(self.session.as_ref().map(Session::mode), Some(Mode::Offline) | None);
        let mut leave = false;
        ui::center_panel(egui_ctx, "pause", 320.0, |ui| {
            ui::heading(ui, "Menü");
            if online {
                ui.label(RichText::new("Das Spiel läuft im Hintergrund weiter.").size(14.0).color(ui::MUTED));
            }
            if ui::big_button(ui, "Weiter").clicked() {
                self.screen = Screen::Playing;
                ctx.cursor_locked = true;
            }
            if ui::big_button(ui, "Einstellungen").clicked() {
                self.settings_return = Screen::Paused;
                self.screen = Screen::Settings;
            }
            if ui::big_button(ui, "Zum Hauptmenü").clicked() {
                leave = true;
            }
            if ui::big_button(ui, "Spiel beenden").clicked() {
                ctx.exit();
            }
        });
        if leave {
            self.show_menu(ctx, None);
        }
    }

    fn hud(&mut self, ctx: &Context, egui_ctx: &egui::Context) {
        let Some(session) = &self.session else { return };
        let local = session.local_player();

        for (&id, avatar) in &session.world().players {
            if Some(id) == local {
                continue;
            }
            if let Some(entity) = ctx.scene.try_get(avatar.entity) {
                ui::name_tag(ctx, egui_ctx, entity.transform.position + Vec3::Y * 1.25, &avatar.name);
            }
        }

        if self.screen != Screen::Playing {
            return;
        }
        ui::time_bar(egui_ctx, session.day());
        if ctx.cursor_locked {
            ui::crosshair(egui_ctx);
        }
        self.aim_hud(egui_ctx);
        self.inventory_hud(egui_ctx);

        // Status oben rechts
        egui::Area::new(egui::Id::new("status"))
            .anchor(Align2::RIGHT_TOP, [-14.0, 14.0])
            .interactable(false)
            .show(egui_ctx, |ui| {
                egui::Frame::new().fill(Color32::from_black_alpha(175)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new(self.status_text()).size(16.0));
                    if let (Mode::Host { port }, Some(ip)) = (session.mode(), self.local_ip) {
                        ui.label(RichText::new(format!("Im Heimnetz: {ip}:{port}")).size(14.0).color(ui::TEXT.gamma_multiply(0.8)));
                    }
                });
            });

        // Spielerliste mit Tab
        if ctx.input.key(KeyCode::Tab) {
            ui::center_panel(egui_ctx, "spielerliste", 340.0, |ui| {
                ui::heading(ui, "Spieler");
                let mut players: Vec<_> = session.world().players.iter().collect();
                players.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()));
                for (&id, avatar) in players {
                    let me = if Some(id) == local { "  (du)" } else { "" };
                    ui.label(RichText::new(format!("{}{me}", avatar.name)).size(19.0));
                }
            });
        }

        let hint = if ctx.cursor_locked || self.free_camera {
            "WASD Laufen · Shift Rennen · Leertaste Springen · Rechtsklick Abbauen · Linksklick Werfen · Tab Spieler · Esc Menü"
        } else {
            "Klicken zum Spielen"
        };
        egui::Area::new(egui::Id::new("hinweis"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -16.0])
            .interactable(false)
            .show(egui_ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                let size = if ctx.cursor_locked { 14.0 } else { 22.0 };
                ui.label(RichText::new(hint).size(size).color(Color32::from_white_alpha(200)));
            });
    }
}

/// Übersetzt die technischen Trennungsgründe des Netzwerks in verständliche Sätze.
fn explain_disconnect(reason: &str) -> String {
    if reason.contains("terminated by server") || reason.contains("terminated by the server") {
        "Der Host hat das Spiel beendet oder dich getrennt.".into()
    } else if reason.contains("request step") {
        "Keine Antwort. Stimmt die Adresse, läuft der Server, und habt ihr dieselbe Spielversion?".into()
    } else if reason.contains("timed out") {
        "Die Verbindung ist abgebrochen (keine Antwort mehr vom Server).".into()
    } else if reason.contains("denied") {
        "Der Server hat die Verbindung abgelehnt – vielleicht ist er voll.".into()
    } else {
        format!("Verbindung verloren ({reason}).")
    }
}

impl Game for Playground {
    fn init(&mut self, ctx: &mut Context) {
        if !ctx.is_headless() {
            self.sounds = Some(Sounds::new(ctx));
        }
        self.apply_settings(ctx);
        // Nur zum Testen: Figur für diesen Start festlegen (`--figur barbar`).
        let args: Vec<String> = std::env::args().collect();
        if let Some(name) = args.iter().position(|a| a == "--figur").and_then(|i| args.get(i + 1)) {
            if let Some(class) = crate::protocol::CharacterClass::ALL.into_iter().find(|c| c.label().eq_ignore_ascii_case(name)) {
                self.settings.character = class;
            }
        }
        self.demo_chop = args.iter().any(|a| a == "--demo-hacken");
        match self.start.take() {
            Some(mode) => self.start_session(ctx, mode),
            None => self.show_menu(ctx, None),
        }
        // Nur für automatische Screenshots: direkt einen bestimmten Bildschirm zeigen.
        let args: Vec<String> = std::env::args().collect();
        if let Some(screen) = args.iter().position(|a| a == "--screen").and_then(|i| args.get(i + 1)) {
            match screen.as_str() {
                "beitreten" => self.screen = Screen::Join,
                "einstellungen" => self.screen = Screen::Settings,
                "pause" => self.screen = Screen::Paused,
                "fehler" => self.error = Some("Der Server antwortet nicht. Stimmt die Adresse, und läuft er?".into()),
                _ => {}
            }
            ctx.show_debug = args.iter().any(|a| a == "--debug");
        }
        if let (Some(hour), Some(session)) = (
            args.iter().position(|a| a == "--uhrzeit").and_then(|i| args.get(i + 1)).and_then(|h| h.parse::<f32>().ok()),
            &mut self.session,
        ) {
            session.world_mut().day.hour = hour.rem_euclid(24.0);
        }
        if args.iter().any(|a| a == "--blick-hoch") {
            ctx.camera.pitch = 0.45;
            ctx.camera.yaw = -1.9;
        }
        // Nur für Screenshots: Blickrichtung in Grad (0 = Norden, negativ = links).
        if let Some(winkel) = args.iter().position(|a| a == "--kamera-winkel").and_then(|i| args.get(i + 1)).and_then(|w| w.parse::<f32>().ok()) {
            ctx.camera.yaw = winkel.to_radians();
            ctx.camera.pitch = -0.12;
            self.orbit.distance = 3.0;
        }
        if args.iter().any(|a| a == "--kamera-vorne") {
            ctx.camera.yaw = std::f32::consts::PI;
            ctx.camera.pitch = -0.15;
            self.orbit.distance = 3.5;
        }
    }

    fn fixed_update(&mut self, ctx: &mut Context) {
        if crate::STOP_REQUESTED.load(std::sync::atomic::Ordering::SeqCst) {
            log::info!("Beende Server, speichere Spielstand …");
            // Die Runde schließen: dabei wird gespeichert.
            self.session = None;
            ctx.exit();
            return;
        }
        let input = self.build_input(ctx);
        let Some(session) = &mut self.session else { return };
        if let Err(reason) = session.fixed_update(ctx, input) {
            log::error!("Verbindung zum Server verloren: {reason}");
            if ctx.is_headless() {
                ctx.exit();
            } else {
                self.show_menu(ctx, Some(explain_disconnect(&reason)));
            }
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        self.handle_game_keys(ctx);

        if self.screen == Screen::Connecting {
            match &self.session {
                Some(session) if !session.is_connecting() => {
                    self.screen = Screen::Playing;
                    ctx.cursor_locked = true;
                }
                Some(_) if ctx.time.elapsed - self.connect_started > CONNECT_TIMEOUT => {
                    self.show_menu(ctx, Some("Der Server antwortet nicht. Stimmt die Adresse, und läuft er?".into()));
                }
                _ => {}
            }
        }

        if let Some(session) = &mut self.session {
            if self.demo_chop && (ctx.time.elapsed % 1.2) < ctx.time.delta {
                if let Some(local) = session.local_player() {
                    session.world_mut().play_action(local, crate::characters::Action::Chop);
                }
            }
            session.world_mut().update_visuals(ctx);
        } else if let Some(world) = &mut self.menu_world {
            world.update_visuals(ctx);
        }
        self.update_camera(ctx);
        if let Some(sounds) = &mut self.sounds {
            if let Some(session) = &mut self.session {
                let inventory = session.local_inventory();
                let items = session.local_player().map(|_| inventory.wood + inventory.stone);
                sounds.update(ctx, session.world_mut(), items);
            } else if let Some(world) = &mut self.menu_world {
                world.sound_events.clear();
                sounds.menu(ctx);
            }
        }
        self.update_aim(ctx);
        ctx.status = self.status_text();
        if let Some(session) = &self.session {
            ctx.debug_lines.push(format!("Spieler: {}  Objekte: {}", session.world().players.len(), session.world().objects.len()));
        }
    }

    fn ui(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        if !self.themed {
            ui::apply_theme(egui_ctx);
            self.themed = true;
        }
        self.hud(ctx, egui_ctx);
        match self.screen {
            Screen::MainMenu => self.main_menu(ctx, egui_ctx),
            Screen::Join => self.join_menu(ctx, egui_ctx),
            Screen::Settings => self.settings_menu(ctx, egui_ctx),
            Screen::Connecting => self.connecting_screen(ctx, egui_ctx),
            Screen::Paused => self.pause_menu(ctx, egui_ctx),
            Screen::Playing => {}
        }
    }
}
