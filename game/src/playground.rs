//! Das Spiel als Ganzes: Menüs, Wechsel zwischen Menü und Runde, Eingabe, Kamera und HUD.

use engine::egui::{self, Align2, Color32, RichText};
use engine::prelude::*;

use crate::protocol::{Item, PlayerInput, Tool, DEFAULT_PORT, MAX_NAME_CHARS};
use crate::session::{Mode, Session};
use crate::settings::Settings;
use crate::sounds::Sounds;
use crate::ui;
use crate::world::World;

/// Nach so vielen Sekunden ohne Antwort gibt der Verbindungsaufbau auf.
const CONNECT_TIMEOUT: f32 = 10.0;
/// Bis zu dieser Entfernung (Meter) haben Tiere einen Lebensbalken, verletzte auch weiter.
const HEALTH_BAR_DISTANCE: f32 = 24.0;
const HEALTH_BAR_DISTANCE_HURT: f32 = 45.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    MainMenu,
    /// Asset-Galerie: alle Modelle ansehen (und für Entwickler: für die Insel markieren).
    Gallery,
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
    /// Zielpunkt eines Zaubers, der im nächsten Takt losgeschickt wird.
    cast_requested: Option<Vec3>,
    harvest_requested: Option<u32>,
    /// Rohstoff unter dem Fadenkreuz, der in Reichweite ist.
    aim: Option<u32>,
    /// Tier unter dem Fadenkreuz (in Zauber-Reichweite).
    aim_animal: Option<u16>,
    last_harvest: f32,
    last_cast: f32,
    /// Inventar-Fenster offen (Taste I)?
    inventory_open: bool,
    /// Symbole und Zustand des Inventar-Fensters.
    inventory_ui: crate::inventar::InventoryUi,
    /// Gewählter Platz der Auswahlleiste (Werkzeug in der Hand).
    hotbar_slot: usize,
    /// Chat (Enter) und Übersichtskarte (M)
    chat: crate::chat::ChatUi,
    map_open: bool,
    map_ui: crate::karte::MapUi,
    /// Nur für Screenshots: Karte auf den Startplatz zoomen, sobald die Welt da ist.
    demo_map_near: bool,
    /// Nur zum Testen: Figur steht an einem Vorkommen und baut es ab
    /// (`Some(erz?)` = noch hinstellen, sobald die Figur da ist).
    demo_mine: Option<u32>,
    demo_mine_request: Option<crate::island::ResourceKind>,
    /// Nur zum Testen: neben das nächste Kristallvorkommen stellen und hinschauen.
    demo_crystal: Option<Option<Vec3>>,
    /// Nur zum Testen: vor eine Sehenswürdigkeit stellen (`--demo-ort Name [Abstand]`).
    demo_spot: Option<(String, f32)>,
    demo_yaw_offset: f32,
    /// Nur zum Testen: Figur läuft von allein.
    autopilot: bool,
    themed: bool,
    /// Eigene Adresse im Heimnetz, damit Mitspieler wissen, wohin sie sich verbinden.
    local_ip: Option<std::net::IpAddr>,
    /// Nur zum Testen: Figur hackt regelmäßig in die Luft.
    demo_chop: bool,
    /// Nur zum Testen: Kamera schaut aufs nächste Tier, die Figur zaubert jede Sekunde darauf.
    demo_cast: bool,
    /// Geräusche und Klangkulisse (nur mit Fenster).
    sounds: Option<Sounds>,
    /// Asset-Galerie, solange sie offen ist.
    gallery: Option<crate::viewer::Viewer>,
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
            cast_requested: None,
            harvest_requested: None,
            aim: None,
            aim_animal: None,
            last_harvest: 0.0,
            last_cast: -10.0,
            inventory_open: false,
            inventory_ui: Default::default(),
            hotbar_slot: 0,
            chat: Default::default(),
            map_open: false,
            map_ui: Default::default(),
            demo_map_near: false,
            demo_mine: None,
            demo_mine_request: None,
            demo_crystal: None,
            demo_spot: None,
            demo_yaw_offset: -0.75,
            autopilot,
            themed: false,
            local_ip: None,
            demo_chop: false,
            demo_cast: false,
            sounds: None,
            gallery: None,
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
                self.inventory_open = false;
                self.orbit.distance = 6.0;
                ctx.camera.pitch = -0.35;
                self.refresh_cursor(ctx);
            }
            Err(message) => {
                log::error!("{message}");
                self.show_menu(ctx, Some(message));
            }
        }
    }

    /// Öffnet die Asset-Galerie (ersetzt die Menü-Kulisse durch den Betrachter).
    fn open_gallery(&mut self, ctx: &mut Context) {
        ctx.reset_world();
        self.menu_world = None;
        let mut gallery = crate::viewer::Viewer::gallery();
        gallery.setup(ctx);
        self.gallery = Some(gallery);
        self.screen = Screen::Gallery;
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
        let playing = self.screen == Screen::Playing && !self.free_camera && !self.chat.open;
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
            cast: self.cast_requested.take(),
            harvest: self.harvest_requested.take(),
            tool: self.tool(),
        };
        self.jump_requested = false;
        input
    }

    /// Wohin ein Zauber jetzt fliegen würde: erster Treffer entlang des Fadenkreuzes.
    fn spell_aim(&self, ctx: &Context) -> Option<(Vec3, Option<u16>)> {
        let session = self.session.as_ref()?;
        let local = session.local_player()?;
        let world = session.world();
        let player = world.player_position(ctx, local)?;
        let reach = ctx.camera.position.distance(player) + crate::world::CAST_RANGE;
        let (point, animal) = world.spell_target(ctx, ctx.camera.position, ctx.camera.forward(), reach, Some(local));
        // Ziele hinter der Reichweite des Magiers zählen nicht.
        let animal = animal.filter(|_| point.distance(player) <= crate::world::CAST_RANGE);
        Some((point, animal))
    }

    /// Werkzeug in der Hand (aus der Auswahlleiste).
    fn tool(&self) -> Tool {
        Tool::HOTBAR.get(self.hotbar_slot).copied().unwrap_or_default()
    }

    /// Schlägt auf den anvisierten Rohstoff, sobald die Abklingzeit um ist. Vorkommen gehen
    /// nur mit der Spitzhacke.
    fn harvest_aimed(&mut self, ctx: &mut Context) {
        let tool = self.tool();
        let Some(id) = self.aim else { return };
        let Some(session) = &mut self.session else { return };
        let Some(kind) = session.world().resources.get(&id).map(|r| r.spec.kind) else { return };
        // Bäume nur mit der Axt, Vorkommen nur mit der Spitzhacke
        if tool != kind.tool() {
            return;
        }
        let ticks = if kind.needs_pickaxe() { crate::world::MINE_COOLDOWN_TICKS } else { crate::world::HARVEST_COOLDOWN_TICKS };
        // Etwas Luft, damit der Server den Schlag nicht als zu früh verwirft.
        if ctx.time.elapsed - self.last_harvest < ticks as f32 * Physics::FIXED_DT + 0.03 {
            return;
        }
        self.last_harvest = ctx.time.elapsed;
        self.harvest_requested = Some(id);
        session.preview_harvest(ctx, id);
    }

    fn toggle_inventory(&mut self, ctx: &mut Context) {
        self.inventory_open = !self.inventory_open;
        self.map_open = false;
        self.refresh_cursor(ctx);
    }

    fn toggle_map(&mut self, ctx: &mut Context) {
        self.map_open = !self.map_open;
        self.inventory_open = false;
        self.refresh_cursor(ctx);
    }

    /// Mauszeiger festhalten, solange man spielt und kein Fenster (Inventar, Karte, Chat) offen ist.
    fn refresh_cursor(&self, ctx: &mut Context) {
        ctx.cursor_locked =
            self.screen == Screen::Playing && !self.free_camera && !self.inventory_open && !self.map_open && !self.chat.open && !ctx.is_headless();
    }

    fn handle_game_keys(&mut self, ctx: &mut Context) {
        let escape = ctx.input.key_pressed(KeyCode::Escape);
        match self.screen {
            Screen::Playing => {
                // Beim Schreiben gehören alle Tasten dem Chat (Enter/Esc erledigt das Eingabefeld).
                if self.chat.open {
                    return;
                }
                if escape && self.inventory_open {
                    self.toggle_inventory(ctx);
                    return;
                }
                if escape && self.map_open {
                    self.toggle_map(ctx);
                    return;
                }
                if (ctx.input.key_pressed(KeyCode::Enter) || ctx.input.key_pressed(KeyCode::NumpadEnter)) && !self.free_camera {
                    self.inventory_open = false;
                    self.map_open = false;
                    self.chat.start_typing();
                    self.refresh_cursor(ctx);
                    return;
                }
                if ctx.input.key_pressed(KeyCode::KeyM) && !self.free_camera {
                    self.toggle_map(ctx);
                }
                if escape {
                    self.screen = Screen::Paused;
                    ctx.cursor_locked = false;
                    return;
                }
                if ctx.input.key_pressed(KeyCode::KeyI) && !self.free_camera {
                    self.toggle_inventory(ctx);
                }
                // Auswahlleiste: Tasten 1–8 oder Mausrad
                let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8];
                for (slot, key) in digits.into_iter().enumerate() {
                    if ctx.input.key_pressed(key) && slot < Tool::HOTBAR.len() {
                        self.hotbar_slot = slot;
                    }
                }
                let scroll = ctx.input.scroll();
                if ctx.cursor_locked && scroll.abs() > 0.1 {
                    let count = Tool::HOTBAR.len();
                    self.hotbar_slot = if scroll < 0.0 { (self.hotbar_slot + 1) % count } else { (self.hotbar_slot + count - 1) % count };
                }
                // Linksklick: Werkzeug benutzen – mit dem Stab zaubern, mit der Spitzhacke abbauen (gedrückt halten).
                let cast_cooldown = crate::world::CAST_COOLDOWN_TICKS as f32 * Physics::FIXED_DT + 0.05;
                if !ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) && !self.free_camera && !self.inventory_open && !self.map_open {
                    ctx.cursor_locked = true;
                } else if ctx.cursor_locked && matches!(self.tool(), Tool::Pickaxe | Tool::Axe) && ctx.input.mouse(MouseButton::Left) {
                    self.harvest_aimed(ctx);
                } else if ctx.cursor_locked
                    && self.tool() == Tool::Staff
                    && ctx.input.mouse_pressed(MouseButton::Left)
                    && ctx.time.elapsed - self.last_cast >= cast_cooldown
                {
                    if let Some((target, _)) = self.spell_aim(ctx) {
                        self.last_cast = ctx.time.elapsed;
                        self.cast_requested = Some(target);
                        if let Some(session) = &mut self.session {
                            session.preview_cast();
                        }
                    }
                }
                self.jump_requested |= ctx.input.key_pressed(KeyCode::Space);
                // Rechte Maustaste halten geht auch (mit dem passenden Werkzeug).
                if ctx.cursor_locked && ctx.input.mouse(MouseButton::Right) {
                    self.harvest_aimed(ctx);
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
                self.refresh_cursor(ctx);
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
            let r = crate::island::ISLAND_RADIUS * 1.3;
            ctx.camera.position = vec3(t.sin() * r, 130.0, t.cos() * r);
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
        self.aim_animal = None;
        if self.screen != Screen::Playing || self.free_camera {
            return;
        }
        self.aim_animal = self.spell_aim(ctx).and_then(|(_, animal)| animal);
        let Some(session) = &self.session else { return };
        let Some(local) = session.local_player() else { return };
        let Some(avatar) = session.world().players.get(&local) else { return };
        let hit = ctx.physics.raycast(ctx.camera.position, ctx.camera.forward(), 60.0, Some(avatar.character));
        if let Some((Some(entity), _)) = hit {
            self.aim = session.world().resource_at(entity).filter(|&id| session.world().in_reach(ctx, local, id, 0.0));
        }
    }

    /// Das Inventar-Fenster (Taste I) am rechten Bildschirmrand, siehe `inventar.rs`.
    fn inventory_window(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let Some(session) = &self.session else { return };
        let inventory = session.local_inventory();
        let name = session.local_player().and_then(|id| session.world().players.get(&id)).map(|a| a.name.clone()).unwrap_or_default();
        if self.inventory_ui.window(egui_ctx, &inventory, &name) {
            self.toggle_inventory(ctx);
        }
    }

    /// Kleine Lebensbalken über Tieren in der Nähe (verletzte auch weiter weg), nur im Spiel.
    fn animal_bars_visible(&self, ctx: &Context, egui_ctx: &egui::Context) {
        if self.screen != Screen::Playing {
            return;
        }
        let Some(session) = &self.session else { return };
        let world = session.world();
        let ignore = session.local_player().and_then(|id| world.players.get(&id)).map(|a| a.character);
        let painter = egui_ctx.layer_painter(egui::LayerId::background());
        for (id, animal) in world.animals.iter().enumerate() {
            if !animal.is_alive() {
                continue;
            }
            let max = animal.kind.max_health();
            let hurt = animal.health < max;
            let (height, radius) = animal.kind.hit_sphere();
            let top = animal.shown_position() + Vec3::Y * (height + radius + 0.3);
            let distance = ctx.camera.position.distance(top);
            let limit = if hurt || self.aim_animal == Some(id as u16) { HEALTH_BAR_DISTANCE_HURT } else { HEALTH_BAR_DISTANCE };
            if distance > limit {
                continue;
            }
            let Some(screen) = ctx.world_to_screen(top) else { continue };
            // Hinter Bäumen, Felsen oder Hügeln verborgen?
            let to = top - ctx.camera.position;
            if ctx.physics.raycast(ctx.camera.position, to / distance, distance - 0.3, ignore).is_some() {
                continue;
            }
            let alpha = ((limit - distance) / 5.0).clamp(0.0, 1.0);
            let width = (46.0 * (12.0 / distance.max(6.0)).sqrt()).clamp(26.0, 56.0);
            ui::health_bar(&painter, egui::pos2(screen.x, screen.y), width, animal.health as f32 / max as f32, alpha);
        }
    }

    /// Hinweis unter dem Fadenkreuz, wenn ein Rohstoff oder Tier anvisiert ist.
    fn aim_hud(&self, egui_ctx: &egui::Context) {
        if let (None, Some(id), Some(session)) = (self.aim, self.aim_animal, &self.session) {
            let Some(animal) = session.world().animals.get(id as usize) else { return };
            let center = egui_ctx.content_rect().center();
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            painter.text(center + egui::vec2(0.0, 28.0), Align2::CENTER_TOP, animal.kind.label(), egui::FontId::proportional(18.0), Color32::WHITE);
            let action = "Linksklick: Zauber";
            painter.text(center + egui::vec2(0.0, 50.0), Align2::CENTER_TOP, action, egui::FontId::proportional(14.0), Color32::from_white_alpha(200));
            let fraction = animal.health as f32 / animal.kind.max_health() as f32;
            ui::health_bar(&painter, center + egui::vec2(0.0, 78.0), 110.0, fraction, 1.0);
            return;
        }
        let (Some(id), Some(session)) = (self.aim, &self.session) else { return };
        let Some(resource) = session.world().resources.get(&id) else { return };
        let center = egui_ctx.content_rect().center();
        let painter = egui_ctx.layer_painter(egui::LayerId::background());
        let needed = resource.spec.kind.tool();
        let hint;
        let (action, color) = if self.tool() == needed {
            let verb = if needed == Tool::Axe { "Linksklick: Holz hacken" } else { "Linksklick: Abbauen" };
            (verb, Color32::from_white_alpha(200))
        } else {
            hint = format!("{} nehmen: Taste {}", needed.label(), needed.slot() + 1);
            (hint.as_str(), Color32::from_rgb(255, 170, 90))
        };
        let big = egui::FontId::proportional(18.0);
        painter.text(center + egui::vec2(0.0, 28.0), Align2::CENTER_TOP, resource.spec.name, big, Color32::WHITE);
        let small = egui::FontId::proportional(14.0);
        painter.text(center + egui::vec2(0.0, 50.0), Align2::CENTER_TOP, action, small, color);
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
        let mut open_gallery = false;
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
                if ui::big_button(ui, "Asset-Galerie").clicked() {
                    open_gallery = true;
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
        if open_gallery {
            self.open_gallery(ctx);
        }
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

        // Wegweiser: aus der Nähe steht an jedem Brett, wohin es zeigt
        for &(tip, name) in &session.world().places.signs {
            if ctx.camera.position.distance(tip) < 12.0 {
                ui::name_tag(ctx, egui_ctx, tip, name);
            }
        }
        for (&id, avatar) in &session.world().players {
            if Some(id) == local {
                continue;
            }
            if let Some(entity) = ctx.scene.try_get(avatar.entity) {
                let head = entity.transform.position + Vec3::Y * 1.25;
                ui::name_tag(ctx, egui_ctx, head, &avatar.name);
                if let Some(text) = self.chat.bubble(id, ctx.time.elapsed) {
                    crate::chat::speech_bubble(ctx, egui_ctx, head + Vec3::Y * 0.45, text);
                }
            }
        }

        // Bei offener Karte nur die Karte (keine Leisten darüber)
        if self.screen != Screen::Playing || self.map_open {
            return;
        }
        ui::time_bar(egui_ctx, session.day());
        if ctx.cursor_locked {
            ui::crosshair(egui_ctx);
        }
        self.aim_hud(egui_ctx);
        if !self.inventory_open {
            self.inventory_ui.hud(egui_ctx, &session.local_inventory());
        }
        self.inventory_ui.hotbar(egui_ctx, self.hotbar_slot);

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
            "WASD Laufen · Shift Rennen · Leertaste Springen · 1–3 Werkzeug · Linksklick Benutzen · I Inventar · M Karte · Enter Chat · Esc Menü"
        } else if self.inventory_open || self.map_open || self.chat.open {
            ""
        } else {
            "Klicken zum Spielen"
        };
        // Über der Auswahlleiste
        egui::Area::new(egui::Id::new("hinweis"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, if ctx.cursor_locked { -118.0 } else { -122.0 }])
            .interactable(false)
            .show(egui_ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                let size = if ctx.cursor_locked { 14.0 } else { 22.0 };
                ui.label(RichText::new(hint).size(size).color(Color32::from_white_alpha(200)));
            });
    }
}

/// Nur zum Testen: stellt die eigene Figur neben das nächste Stein- oder Erzvorkommen.
fn demo_place_at_node(ctx: &mut Context, session: &mut Session, wanted: crate::island::ResourceKind) -> Option<u32> {
    let local = session.local_player()?;
    let world = session.world();

    let (&id, resource) = world
        .resources
        .iter()
        .filter(|(_, r)| r.spec.kind == wanted && r.is_present())
        .min_by(|a, b| a.1.spec.transform.position.distance(world.spawn).total_cmp(&b.1.spec.transform.position.distance(world.spawn)))?;
    let base = resource.spec.transform.position;
    let to_spawn = (world.spawn - base).with_y(0.0).normalize_or(Vec3::Z);
    let stand = base + to_spawn * 1.9;
    let stand = vec3(stand.x, world.terrain.height_at(stand.x, stand.z) + 1.0, stand.z);
    let character = world.players.get(&local)?.character;
    ctx.physics.teleport_character(character, stand);
    Some(id)
}

/// Nur zum Testen: stellt die eigene Figur vor das nächste Kristallvorkommen.
fn demo_place_at_crystal(ctx: &mut Context, session: &mut Session) -> Option<Vec3> {
    let local = session.local_player()?;
    let world = session.world();
    let crystal = world.crystals().iter().copied().min_by(|a, b| a.distance(world.spawn).total_cmp(&b.distance(world.spawn)))?;
    let away = (world.spawn - crystal).with_y(0.0).normalize_or(Vec3::Z);
    let stand = crystal + away * 3.0;
    let stand = vec3(stand.x, world.terrain.height_at(stand.x, stand.z) + 1.0, stand.z);
    let character = world.players.get(&local)?.character;
    ctx.physics.teleport_character(character, stand);
    Some(crystal)
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
        self.demo_cast = args.iter().any(|a| a == "--demo-zaubern");
        if self.demo_cast {
            self.hotbar_slot = Tool::Staff.slot();
        }
        // Nur zum Testen: an das nächste Vorkommen stellen und abbauen (`--demo-abbauen [erz|stein]`).
        // Nur für Screenshots: Karte offen bzw. ein paar Chatzeilen
        if let Some(position) = args.iter().position(|a| a == "--demo-karte") {
            self.demo_map_near = args.get(position + 1).is_some_and(|a| a == "nah");
            self.map_open = true;
            self.refresh_cursor(ctx);
        }
        if args.iter().any(|a| a == "--demo-chat") {
            use crate::world::ChatLine;
            let now = ctx.time.elapsed;
            self.chat.push(ChatLine::notice("Mira ist beigetreten".into()), now);
            self.chat.push(ChatLine { from: Some(7), name: "Mira".into(), text: "Hallo! Wo habt ihr das Erz gefunden?".into() }, now);
            self.chat.push(ChatLine { from: Some(0), name: self.settings.name.clone(), text: "Im Gebirge im Norden, schau auf die Karte (M)".into() }, now);
            self.chat.push(ChatLine { from: Some(7), name: "Mira".into(), text: "Danke, bin unterwegs!".into() }, now);
            self.chat.start_typing();
            self.chat.prefill("Treffen wir uns am Bergsee?");
            self.refresh_cursor(ctx);
        }
        if args.iter().any(|a| a == "--demo-kristall") {
            self.demo_crystal = Some(None);
        }
        if let Some(position) = args.iter().position(|a| a == "--demo-ort") {
            let name = args.get(position + 1).cloned().unwrap_or_default().to_lowercase();
            let distance = args.get(position + 2).and_then(|d| d.parse().ok()).unwrap_or(14.0);
            self.demo_spot = Some((name, distance));
            self.demo_yaw_offset = 0.12;
        }
        if let Some(position) = args.iter().position(|a| a == "--demo-abbauen") {
            use crate::island::ResourceKind;
            let kind = match args.get(position + 1).map(String::as_str) {
                Some("stein") => ResourceKind::Stone,
                Some("baum") => ResourceKind::Wood,
                _ => ResourceKind::Ore,
            };
            self.demo_mine_request = Some(kind);
            self.hotbar_slot = kind.tool().slot();
        }
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
                "galerie" => self.open_gallery(ctx),
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
        // Nur für Screenshots: ein bestimmtes Wetter erzwingen (`--wetter regen`)
        if let (Some(name), Some(session)) = (args.iter().position(|a| a == "--wetter").and_then(|i| args.get(i + 1)), &mut self.session) {
            session.world_mut().weather.force_named(name);
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
        // Nur für Screenshots: etwas Beute ins Inventar legen (und es mit `--demo-inventar` öffnen).
        let open_inventory = args.iter().any(|a| a == "--demo-inventar");
        if open_inventory || args.iter().any(|a| a == "--demo-beute") {
            if let Some(session) = &mut self.session {
                if let Some(local) = session.local_player() {
                    let inventory = session.world_mut().inventories.entry(local).or_default();
                    for (item, amount) in [(Item::Wood, 23), (Item::Stone, 11), (Item::Ore, 7), (Item::Meat, 5), (Item::Pelt, 3), (Item::Wool, 6)] {
                        inventory.add_item(item, amount);
                    }
                }
            }
            self.inventory_open = open_inventory;
            ctx.cursor_locked = !open_inventory;
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

        if self.screen == Screen::Gallery {
            let back = ctx.input.key_pressed(KeyCode::Escape) || self.gallery.as_ref().is_some_and(|g| g.wants_back());
            if back {
                self.gallery = None;
                self.show_menu(ctx, None);
            } else if let Some(gallery) = &mut self.gallery {
                gallery.update(ctx);
            }
            return;
        }

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

        // Nur für Screenshots: Kamera auf den nächsten Vogelschwarm richten
        if std::env::args().any(|a| a == "--demo-voegel") {
            if let Some(session) = &self.session {
                let world = session.world();
                // Freie Kamera ein paar Meter neben dem Schwarm, Blick darauf
                if let Some(flock) = world.nearest_flock(ctx, ctx.camera.position) {
                    self.free_camera = true;
                    ctx.camera.position = flock + vec3(-9.0, -2.5, -9.0);
                    ctx.camera.look_at(flock);
                }
                if ctx.time.frame % 60 == 0 {
                    log::info!("Schmetterlinge zu sehen: {}", world.visible_butterflies(ctx));
                }
            }
        }
        if let (Some((name, distance)), Some(session)) = (self.demo_spot.take(), &mut self.session) {
            let world = session.world();
            let target = world.places.labels.iter().find(|(label, _)| label.to_lowercase().contains(&name)).map(|&(_, at)| at);
            if let (Some(at), Some(local)) = (target, session.local_player()) {
                let ground = world.terrain.height_at(at.x, at.y);
                let mut away = (vec2(world.spawn.x, world.spawn.z) - at).normalize_or(Vec2::Y);
                // Den Wasserfall von bachabwärts ansehen (vom Startplatz aus verdecken ihn Hügel)
                // Den See vom Steg aus zeigen (Figur auf dem Steg, Blick über das Wasser)
                if name.contains("see") {
                    if let Some(&(_, base, _)) = world.places.boats.first() {
                        let to_lake = (vec2(base.x, base.z) - at).normalize_or(Vec2::Y);
                        away = -to_lake;
                    }
                }
                if let (true, Some((top, foot))) = (name.contains("wasserfall"), world.places.waterfall) {
                    away = (vec2(foot.x - top.x, foot.z - top.z)).normalize_or(Vec2::Y);
                    away = (away + away.perp() * 0.35).normalize();
                }
                let mut stand = at + away * distance;
                // Nicht im Wasser stehen: notfalls näher heran
                while world.terrain.height_at(stand.x, stand.y) < 0.5 && stand.distance(at) > 3.0 {
                    stand -= away;
                }
                if let Some(character) = world.players.get(&local).map(|a| a.character) {
                    let y = world.terrain.height_at(stand.x, stand.y) + 1.0;
                    ctx.physics.teleport_character(character, vec3(stand.x, y, stand.y));
                }
                self.demo_crystal = Some(Some(vec3(at.x, ground + 1.5, at.y)));
            }
        }
        if let (Some(None), Some(session)) = (self.demo_crystal, &mut self.session) {
            self.demo_crystal = Some(demo_place_at_crystal(ctx, session));
        }
        if let (Some(Some(crystal)), Some(session)) = (self.demo_crystal, &self.session) {
            if let Some(player) = session.local_player().and_then(|p| session.world().player_position(ctx, p)) {
                let to = crystal - player;
                ctx.camera.yaw = to.x.atan2(-to.z) + self.demo_yaw_offset;
                ctx.camera.pitch = if self.demo_yaw_offset > 0.0 { -0.32 } else { -0.12 };
                self.orbit.distance = 5.5;
            }
        }
        if let (Some(kind), None, Some(session)) = (self.demo_mine_request, self.demo_mine, &mut self.session) {
            self.demo_mine = demo_place_at_node(ctx, session, kind);
        }
        if let (Some(id), Some(session)) = (self.demo_mine, &self.session) {
            let world = session.world();
            if let (Some(player), Some(resource)) = (session.local_player().and_then(|p| world.player_position(ctx, p)), world.resources.get(&id)) {
                let to = resource.spec.transform.position - player;
                ctx.camera.yaw = to.x.atan2(-to.z) + 1.0;
                ctx.camera.pitch = -0.28;
                self.orbit.distance = 5.0;
            }
            self.aim = Some(id);
            self.harvest_aimed(ctx);
        }
        if let Some(session) = &mut self.session {
            if self.demo_cast {
                let world = session.world();
                let player = session.local_player().and_then(|id| world.player_position(ctx, id));
                let nearest = player.and_then(|p| {
                    world
                        .animals
                        .iter()
                        .filter(|a| a.is_alive() && a.position.distance(p) < 35.0)
                        .min_by(|a, b| a.position.distance(p).total_cmp(&b.position.distance(p)))
                        .map(|a| (p, a.hit_sphere().0))
                });
                if let Some((player, target)) = nearest {
                    // Kamera hinter die Figur, Blick aufs Tier.
                    let to = target - player;
                    ctx.camera.yaw = to.x.atan2(-to.z) + 0.25;
                    ctx.camera.pitch = -0.2;
                    if ctx.time.elapsed - self.last_cast > 1.0 {
                        self.last_cast = ctx.time.elapsed;
                        self.cast_requested = Some(target);
                        session.preview_cast();
                        log::debug!("Demo-Zauber: Bild {}, {:.2} s", ctx.time.frame, ctx.time.elapsed);
                    }
                }
            }
            if self.demo_chop && (ctx.time.elapsed % 1.2) < ctx.time.delta {
                if let Some(local) = session.local_player() {
                    session.world_mut().play_action(local, crate::characters::Action::Chop);
                }
            }
            session.world_mut().update_visuals(ctx);
            self.chat.collect(session.world_mut(), ctx.time.elapsed);
            if std::mem::take(&mut self.demo_map_near) {
                let spawn = session.world().spawn;
                self.map_ui.focus(vec2(spawn.x, spawn.z), 3.5);
            }
        } else if let Some(world) = &mut self.menu_world {
            world.update_visuals(ctx);
        }
        self.update_camera(ctx);
        if let Some(sounds) = &mut self.sounds {
            if let Some(session) = &mut self.session {
                let inventory = session.local_inventory();
                let items = session.local_player().map(|_| inventory.total());
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
        if !self.inventory_open {
            self.animal_bars_visible(ctx, egui_ctx);
        }
        self.hud(ctx, egui_ctx);
        match self.screen {
            Screen::MainMenu => self.main_menu(ctx, egui_ctx),
            Screen::Gallery => {
                if let Some(gallery) = &mut self.gallery {
                    gallery.ui(ctx, egui_ctx);
                }
            }
            Screen::Join => self.join_menu(ctx, egui_ctx),
            Screen::Settings => self.settings_menu(ctx, egui_ctx),
            Screen::Connecting => self.connecting_screen(ctx, egui_ctx),
            Screen::Paused => self.pause_menu(ctx, egui_ctx),
            Screen::Playing if self.inventory_open => self.inventory_window(ctx, egui_ctx),
            Screen::Playing if self.map_open => {
                if let Some(session) = &self.session {
                    if self.map_ui.show(ctx, egui_ctx, session.world(), session.local_player()) {
                        self.toggle_map(ctx);
                    }
                }
            }
            Screen::Playing => {}
        }
        if self.screen == Screen::Playing && !self.map_open {
            match self.chat.show(egui_ctx, ctx.time.elapsed) {
                crate::chat::ChatAction::Send(text) => {
                    if let Some(session) = &mut self.session {
                        session.send_chat(ctx, &text);
                    }
                    self.refresh_cursor(ctx);
                }
                crate::chat::ChatAction::Close => self.refresh_cursor(ctx),
                crate::chat::ChatAction::None => {}
            }
        }
    }
}
