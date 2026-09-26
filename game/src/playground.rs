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
    /// Kamerafahrten, Titel und Effekte des Hauptmenüs.
    title: Option<crate::hauptmenue::TitleScreen>,
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
    /// Baumenü offen (Taste B)?
    build_menu_open: bool,
    /// Reiter im Baumenü: 0 = Gebäude, 1 = Türme
    build_tab: u8,
    /// Gebäude unter dem Fadenkreuz und das offene Turmfenster (Taste E)
    aim_building: Option<u32>,
    building_window: Option<u32>,
    /// Admin-Panel offen (Taste X) und frei fliegen (Noclip)
    admin_open: bool,
    noclip: bool,
    /// Einheit der Festung unter dem Fadenkreuz (Art, Lebenspunkte)
    aim_enemy: Option<(crate::heer::EnemyKind, u8, u16)>,
    /// Verteidigungsfenster (T) offen
    td_open: bool,
    /// Wie viele Auswertungen schon da waren und seit wann die neueste gezeigt wird
    bericht_seit: (usize, f32),
    /// Nur zum Testen: Fallen und Kaserne an die Südstraße (`--demo-fallen`), Turmfenster öffnen
    demo_fallen: bool,
    demo_turmfenster: Option<u8>,
    /// Gebäude, das gerade platziert wird, und die eigene Drehung dazu (Q/E, Mausrad)
    build_mode: Option<(crate::bauten::BuildingKind, f32)>,
    /// Bauplatz unter dem Fadenkreuz: Mitte, Drehung und ob (bzw. warum nicht) gebaut werden kann
    build_site: Option<(Vec2, f32, Result<f32, &'static str>)>,
    /// Vorschaubilder der Gebäude (art/icons/bauten.py), beim ersten Öffnen geladen
    build_icons: Option<std::collections::HashMap<crate::bauten::BuildingKind, egui::TextureHandle>>,
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
    /// Nur zum Testen: Gebäude in der Nähe aufstellen (`--demo-bau <art> [fortschritt]`, ohne
    /// Fortschritt: Vorschau beim Platzieren) bzw. das Baumenü öffnen (`--demo-baumenue`).
    demo_bau: Option<(String, Option<f32>)>,
    demo_build_menu: bool,
    /// Nur zum Testen: Truppen der Festung sofort losschicken (`--demo-truppen`)
    demo_troops: bool,
    /// Nur zum Testen: Figur auf die Südstraße stellen, den Truppen entgegen (`--demo-kampf`)
    demo_fight: bool,
    /// Nur zum Testen: alle zehn Türme an die Südstraße stellen (`--demo-tuerme`)
    demo_towers: bool,
    /// Nur zum Testen: Wellen gleich bei dieser Nummer beginnen lassen (`--demo-welle N`)
    demo_wave: Option<u32>,
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
            title: None,
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
            build_menu_open: false,
            build_tab: 0,
            aim_building: None,
            building_window: None,
            admin_open: false,
            noclip: false,
            aim_enemy: None,
            td_open: false,
            bericht_seit: (0, 0.0),
            demo_fallen: false,
            demo_turmfenster: None,
            build_mode: None,
            build_site: None,
            build_icons: None,
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
            demo_bau: None,
            demo_build_menu: false,
            demo_troops: false,
            demo_fight: false,
            demo_towers: false,
            demo_wave: None,
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
                self.build_menu_open = false;
                self.build_mode = None;
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
        // Im Menü: die Insel bei Nacht, von oben
        self.title = Some(crate::hauptmenue::TitleScreen::new(&mut world, ctx.time.elapsed));
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
            jump: self.jump_requested && !self.noclip,
            cast: self.cast_requested.take(),
            harvest: self.harvest_requested.take(),
            tool: self.tool(),
            noclip: self.noclip,
            // Noclip: Leertaste hoch, Strg runter
            rise: if playing && self.noclip {
                ctx.input.key(KeyCode::Space) as i32 as f32 - ctx.input.key(KeyCode::ControlLeft) as i32 as f32
            } else {
                0.0
            },
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
        self.build_menu_open = false;
        self.build_mode = None;
        self.refresh_cursor(ctx);
    }

    fn toggle_build_menu(&mut self, ctx: &mut Context) {
        self.build_menu_open = !self.build_menu_open;
        self.inventory_open = false;
        self.map_open = false;
        self.build_mode = None;
        self.refresh_cursor(ctx);
    }

    /// Platzieren beginnen (aus dem Baumenü).
    fn start_placing(&mut self, ctx: &mut Context, kind: crate::bauten::BuildingKind) {
        self.build_menu_open = false;
        self.build_mode = Some((kind, 0.0));
        self.refresh_cursor(ctx);
    }

    /// Vorschau des Gebäudes, das gerade platziert wird: dort, wo das Fadenkreuz den Boden trifft,
    /// die Vorderseite zur eigenen Figur gedreht.
    fn update_build_preview(&mut self, ctx: &mut Context) {
        let Some(session) = &mut self.session else { return };
        let Some((kind, turn)) = self.build_mode.filter(|_| self.screen == Screen::Playing) else {
            if self.build_site.take().is_some() || self.build_mode.is_none() {
                session.world_mut().set_build_preview(ctx, None);
            }
            return;
        };
        let local = session.local_player();
        let world = session.world();
        let Some(player) = local.and_then(|id| world.player_position(ctx, id)) else { return };
        let character = local.and_then(|id| world.players.get(&id)).map(|a| a.character);
        let forward = ctx.camera.forward();
        let hit = ctx.physics.raycast(ctx.camera.position, forward, 70.0, character).map(|(_, d)| ctx.camera.position + forward * d);
        let flat_forward = vec2(forward.x, forward.z).normalize_or(Vec2::Y);
        let own = vec2(player.x, player.z);
        let mut at = hit.map(|p| vec2(p.x, p.z)).unwrap_or(own + flat_forward * 14.0);
        // Nicht auf die eigene Figur bauen
        let nearest = kind.radius() + 1.5;
        if at.distance(own) < nearest {
            at = own + (at - own).normalize_or(flat_forward) * nearest;
        }
        let toward = own - at;
        let mut yaw = toward.x.atan2(toward.y) + turn;
        // Fallen rasten mitten auf der Straße ein, quer zu ihr
        if let Some((platz, quer)) = kind.falle().and_then(|_| crate::bauten::falle_platz(world, at)) {
            at = platz;
            yaw = quer;
        }
        let mut check = crate::bauten::check_site(world, kind, at, Some(player));
        if check.is_ok() && !crate::bauten::affordable(&session.local_inventory(), kind) {
            check = Err("Nicht genug Rohstoffe");
        }
        let ground = check.unwrap_or_else(|_| world.terrain.height_at(at.x, at.y));
        session.world_mut().set_build_preview(ctx, Some((kind, vec3(at.x, ground, at.y), yaw, check.is_ok())));
        self.build_site = Some((at, yaw, check));
    }

    /// Admin-Panel (Taste X): Noclip, Wetter, Truppen der Schattenfestung.
    fn admin_panel(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        use crate::protocol::{AdminCommand, WETTER};
        let Some(session) = &mut self.session else { return };
        let (weather, waves, count, welle, leben, schwierigkeit, endlos) = {
            let world = session.world();
            let td = &world.td;
            (world.weather_choice, td.aktiv, world.feinde.len(), td.welle, td.leben, td.schwierigkeit, td.endlos)
        };
        let mut commands = Vec::new();
        let mut close = false;
        let mut noclip = self.noclip;
        ui::center_panel(egui_ctx, "admin", 470.0, |ui| {
            ui::heading(ui, "Admin");
            ui.checkbox(&mut noclip, RichText::new("Noclip – frei fliegen, durch Wände (Leertaste hoch, Strg runter)").size(16.0));
            ui.add_space(10.0);
            ui.label(RichText::new("Wetter").size(16.0).strong().color(ui::ACCENT));
            ui.horizontal_wrapped(|ui| {
                for (i, name) in WETTER.iter().enumerate() {
                    let mut label = name.to_string();
                    if let Some(first) = label.get_mut(0..1) {
                        first.make_ascii_uppercase();
                    }
                    if ui.selectable_label(weather == i as u8, RichText::new(label).size(15.0)).clicked() {
                        commands.push(AdminCommand::Weather(i as u8));
                    }
                }
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Truppen der Schattenfestung").size(16.0).strong().color(ui::ACCENT));
            ui.label(RichText::new(format!("{count} Einheiten unterwegs · {}", if waves { "Spawn läuft" } else { "Spawn gestoppt" })).size(14.0));
            ui.label(RichText::new(format!("Welle {} · Leben der Insel {}/{}", welle, leben, schwierigkeit.leben())).size(14.0));
            ui.horizontal(|ui| {
                let text = if waves { "Spawn stoppen" } else { "Spawn starten" };
                if ui.button(RichText::new(text).size(16.0)).clicked() {
                    commands.push(AdminCommand::Waves(!waves));
                }
                if ui.button(RichText::new("Welle jetzt").size(16.0)).clicked() {
                    commands.push(AdminCommand::WaveNow);
                }
                if ui.button(RichText::new("Alle entfernen").size(16.0)).clicked() {
                    commands.push(AdminCommand::ClearEnemies);
                }
                if ui.button(RichText::new("Zurück auf Welle 1").size(16.0)).clicked() {
                    commands.push(AdminCommand::ResetWaves);
                }
            });
            ui.label(
                RichText::new(format!("Alle {:.0} s eine Welle: auf jeder der vier Straßen eine Gruppe. Wer das Straßenende erreicht, kostet die Insel Leben.", crate::heer::WAVE_SECONDS))
                    .size(13.0)
                    .color(ui::TEXT.gamma_multiply(0.7)),
            );
            ui.add_space(8.0);
            ui.label(RichText::new("Tower Defense").size(16.0).strong().color(ui::ACCENT));
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Schwierigkeit:").size(15.0));
                for s in crate::td::Schwierigkeit::ALL {
                    if ui.selectable_label(schwierigkeit == s, RichText::new(format!("{} ({} Leben)", s.label(), s.leben())).size(14.0)).clicked() && schwierigkeit != s {
                        commands.push(AdminCommand::Schwierigkeit(s));
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                let mut e = endlos;
                if ui.checkbox(&mut e, RichText::new("Endlosmodus (nach Welle 30 weiter)").size(15.0)).changed() {
                    commands.push(AdminCommand::Endlos(e));
                }
                if ui.button(RichText::new("+500 Gold").size(15.0)).clicked() {
                    commands.push(AdminCommand::Gold(500));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Springen zu Welle:").size(15.0));
                for w in [5u32, 10, 15, 20, 25, 30] {
                    if ui.button(RichText::new(w.to_string()).size(14.0)).clicked() {
                        commands.push(AdminCommand::SpringeZuWelle(w - 1));
                    }
                }
            });
            ui.add_space(10.0);
            if ui::big_button(ui, "Schließen (X)").clicked() {
                close = true;
            }
        });
        for command in commands {
            session.admin(command);
        }
        self.noclip = noclip;
        if close {
            self.admin_open = false;
            self.refresh_cursor(ctx);
        }
    }

    /// Baumenü: die drei Gebäude mit Beschreibung, Kosten und Ertrag.
    fn build_menu(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        use crate::bauten::{BuildingKind, PRODUCTION_SECONDS};
        use crate::tuerme::TowerKind;
        let inventory = self.session.as_ref().map(|s| s.local_inventory()).unwrap_or_default();
        let icons = self.build_icons.get_or_insert_with(|| load_build_icons(egui_ctx)).clone();
        let mut chosen = None;
        let mut close = false;
        let mut tab = self.build_tab;
        let kosten_zeile = |ui: &mut egui::Ui, cost: &[(crate::protocol::Item, u32)]| {
            ui.horizontal_wrapped(|ui| {
                for &(item, amount) in cost {
                    let color = if inventory.count(item) >= amount { Color32::from_rgb(140, 225, 130) } else { Color32::from_rgb(240, 120, 100) };
                    ui.label(RichText::new(format!("{amount} {}", item.label())).size(14.0).color(color));
                }
            });
        };
        ui::center_panel(egui_ctx, "baumenue", 940.0, |ui| {
            ui::heading(ui, "Bauen");
            ui.horizontal(|ui| {
                for (i, name) in ["Gebäude", "Türme", "Fallen"].iter().enumerate() {
                    if ui.selectable_label(tab == i as u8, RichText::new(*name).size(18.0)).clicked() {
                        tab = i as u8;
                    }
                }
            });
            ui.add_space(6.0);
            if tab == 2 {
                ui.label(
                    RichText::new("Fallen stehen direkt auf einer Heerstraße. Die Barrikade hält Gruppen auf, bis sie zerschlagen ist.")
                        .size(15.0)
                        .color(ui::TEXT.gamma_multiply(0.8)),
                );
                ui.add_space(8.0);
                ui.columns(crate::tuerme::FallenArt::ALL.len(), |columns| {
                    for (column, falle) in columns.iter_mut().zip(crate::tuerme::FallenArt::ALL) {
                        let kind = BuildingKind::Falle(falle);
                        let affordable = crate::bauten::affordable(&inventory, kind);
                        egui::Frame::new().fill(Color32::from_black_alpha(90)).corner_radius(8.0).inner_margin(12.0).show(column, |ui| {
                            ui.set_min_height(210.0);
                            ui.label(RichText::new(kind.label()).size(20.0).strong().color(ui::ACCENT));
                            if let Some(icon) = icons.get(&kind) {
                                let width = ui.available_width();
                                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(width, width * 0.5)));
                            }
                            ui.label(RichText::new(kind.description()).size(14.0));
                            ui.add_space(6.0);
                            kosten_zeile(ui, &kind.cost());
                            ui.add_space(8.0);
                            let label = if affordable { "Bauen" } else { "Zu wenig Gold/Rohstoffe" };
                            if ui.add_enabled(affordable, egui::Button::new(RichText::new(label).size(17.0)).min_size(egui::vec2(ui.available_width(), 34.0))).clicked() {
                                chosen = Some(kind);
                            }
                        });
                    }
                });
            } else if tab == 0 {
                ui.label(RichText::new("Fertige Gebäude liefern dir regelmäßig Rohstoffe. Nicht auf die Heerstraßen bauen.").size(15.0).color(ui::TEXT.gamma_multiply(0.8)));
                ui.add_space(8.0);
                ui.columns(BuildingKind::ALL.len(), |columns| {
                    for (column, kind) in columns.iter_mut().zip(BuildingKind::ALL) {
                        let affordable = crate::bauten::affordable(&inventory, kind);
                        egui::Frame::new().fill(Color32::from_black_alpha(90)).corner_radius(8.0).inner_margin(12.0).show(column, |ui| {
                            ui.set_min_height(230.0);
                            ui.label(RichText::new(kind.label()).size(22.0).strong().color(ui::ACCENT));
                            if let Some(icon) = icons.get(&kind) {
                                let width = ui.available_width();
                                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(width, width * 0.5)));
                            }
                            ui.label(RichText::new(kind.description()).size(14.0));
                            ui.add_space(6.0);
                            kosten_zeile(ui, &kind.cost());
                            let liefert = kind.produces().map(|i| i.label()).unwrap_or("-");
                            ui.label(
                                RichText::new(format!("Liefert 1 {liefert} alle {:.0} s · Bauzeit {:.0} s", PRODUCTION_SECONDS, kind.build_seconds(1)))
                                    .size(13.0)
                                    .color(ui::TEXT.gamma_multiply(0.75)),
                            );
                            ui.add_space(8.0);
                            let label = if affordable { "Bauen" } else { "Zu wenig Rohstoffe" };
                            if ui.add_enabled(affordable, egui::Button::new(RichText::new(label).size(17.0)).min_size(egui::vec2(ui.available_width(), 34.0))).clicked() {
                                chosen = Some(kind);
                            }
                        });
                    }
                });
            } else {
                ui.label(
                    RichText::new("Türme nur direkt an einer Heerstraße (4–14 m daneben). Mit E auf einen Turm: Ziel wählen, aufwerten, auf Stufe 3 eine von zwei Richtungen.")
                        .size(15.0)
                        .color(ui::TEXT.gamma_multiply(0.8)),
                );
                ui.add_space(8.0);
                let hoehe = (egui_ctx.content_rect().height() - 250.0).max(200.0);
                egui::ScrollArea::vertical().max_height(hoehe).show(ui, |ui| {
                for reihe in TowerKind::ALL.chunks(5) {
                    ui.columns(5, |columns| {
                        for (column, &tower) in columns.iter_mut().zip(reihe) {
                            let kind = BuildingKind::Tower(tower);
                            let affordable = crate::bauten::affordable(&inventory, kind);
                            egui::Frame::new().fill(Color32::from_black_alpha(90)).corner_radius(8.0).inner_margin(9.0).show(column, |ui| {
                                ui.set_min_height(210.0);
                                ui.label(RichText::new(tower.label()).size(17.0).strong().color(ui::ACCENT));
                                if let Some(icon) = icons.get(&kind) {
                                    let width = ui.available_width();
                                    ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(width, width * 0.5)));
                                }
                                ui.label(RichText::new(tower.description()).size(12.5));
                                ui.add_space(4.0);
                                for zeile in tower.werte(1, 0).zeilen() {
                                    ui.label(RichText::new(zeile).size(12.0).color(ui::TEXT.gamma_multiply(0.8)));
                                }
                                ui.add_space(4.0);
                                kosten_zeile(ui, &kind.cost());
                                if ui.add_enabled(affordable, egui::Button::new(RichText::new("Bauen").size(15.0)).min_size(egui::vec2(ui.available_width(), 28.0))).clicked() {
                                    chosen = Some(kind);
                                }
                            });
                        }
                    });
                    ui.add_space(6.0);
                }
                });
            }
            ui.add_space(8.0);
            if ui::big_button(ui, "Schließen").clicked() {
                close = true;
            }
        });
        self.build_tab = tab;
        if let Some(kind) = chosen {
            self.start_placing(ctx, kind);
        } else if close {
            self.toggle_build_menu(ctx);
        }
    }

    /// Turmfenster (E): Werte, nächste Stufe mit Kosten, Aufwerten und Abreißen.
    fn building_panel(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let Some(id) = self.building_window else { return };
        let Some(session) = &mut self.session else { return };
        let inventory = session.local_inventory();
        let me = session.local_player().and_then(|p| session.world().players.get(&p)).map(|a| crate::save::player_key(&a.name)).unwrap_or_default();
        let Some(building) = session.world().buildings.iter().find(|b| b.id == id).cloned() else {
            self.building_window = None;
            self.refresh_cursor(ctx);
            return;
        };
        let (aktion, mut close) = crate::td_ui::turm_fenster(egui_ctx, session.world(), &building, &inventory, &me);
        if let Some(aktion) = aktion {
            use crate::td::TdBefehl;
            let befehl = match aktion {
                crate::td_ui::TurmAktion::Aufwerten(zweig) => TdBefehl::Aufwerten(id, zweig),
                crate::td_ui::TurmAktion::Abreissen => {
                    close = true;
                    TdBefehl::Abreissen(id)
                }
                crate::td_ui::TurmAktion::Zielen(modus) => TdBefehl::Zielen(id, modus),
            };
            session.td(ctx, befehl);
        }
        if close {
            self.building_window = None;
            self.refresh_cursor(ctx);
        }
    }

    /// Verteidigungsfenster (T): nächste Welle, früh rufen, Straßen, Beitrag.
    fn td_panel(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let Some(session) = &mut self.session else { return };
        let me = session.local_player().and_then(|p| session.world().players.get(&p)).map(|a| crate::save::player_key(&a.name)).unwrap_or_default();
        let (aktionen, close) = crate::td_ui::td_fenster(egui_ctx, session.world(), &me);
        for aktion in aktionen {
            let befehl = match aktion {
                crate::td_ui::TdAktion::Rufen => crate::td::TdBefehl::WelleRufen,
                crate::td_ui::TdAktion::Strasse(i) => crate::td::TdBefehl::Strasse(i),
            };
            session.td(ctx, befehl);
        }
        if close {
            self.td_open = false;
            self.refresh_cursor(ctx);
        }
    }

    fn toggle_map(&mut self, ctx: &mut Context) {
        self.map_open = !self.map_open;
        self.inventory_open = false;
        self.refresh_cursor(ctx);
    }

    /// Mauszeiger festhalten, solange man spielt und kein Fenster (Inventar, Karte, Chat) offen ist.
    fn refresh_cursor(&self, ctx: &mut Context) {
        ctx.cursor_locked = self.screen == Screen::Playing
            && !self.free_camera
            && !self.inventory_open
            && !self.build_menu_open
            && !self.admin_open
            && self.building_window.is_none()
            && !self.td_open
            && !self.map_open
            && !self.chat.open
            && !ctx.is_headless();
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
                // E: Turmfenster für das anvisierte Gebäude (Esc/E schließt)
                if (escape && self.building_window.is_some()) || (ctx.input.key_pressed(KeyCode::KeyE) && self.build_mode.is_none()) {
                    self.building_window = if self.building_window.is_some() { None } else { self.aim_building };
                    if self.building_window.is_some() || escape || ctx.input.key_pressed(KeyCode::KeyE) {
                        self.refresh_cursor(ctx);
                        return;
                    }
                }
                // X: Admin-Panel (Noclip, Wetter, Truppen der Festung)
                if (escape && self.admin_open) || (ctx.input.key_pressed(KeyCode::KeyX) && !self.free_camera) {
                    self.admin_open = !self.admin_open;
                    self.build_menu_open = false;
                    self.inventory_open = false;
                    self.build_mode = None;
                    self.refresh_cursor(ctx);
                    return;
                }
                // T: Verteidigungsfenster
                if (escape && self.td_open) || (ctx.input.key_pressed(KeyCode::KeyT) && !self.free_camera) {
                    self.td_open = !self.td_open;
                    self.build_menu_open = false;
                    self.inventory_open = false;
                    self.build_mode = None;
                    self.refresh_cursor(ctx);
                    return;
                }
                // N: die nächste Welle sofort rufen
                if ctx.input.key_pressed(KeyCode::KeyN) && !self.free_camera {
                    if let Some(session) = &mut self.session {
                        if session.world().td.aktiv {
                            session.td(ctx, crate::td::TdBefehl::WelleRufen);
                        }
                    }
                }
                // B: Baumenü (bzw. Platzieren abbrechen)
                if (escape || ctx.input.key_pressed(KeyCode::KeyB)) && self.build_mode.is_some() {
                    self.build_mode = None;
                    self.refresh_cursor(ctx);
                    return;
                }
                if (escape && self.build_menu_open) || (ctx.input.key_pressed(KeyCode::KeyB) && !self.free_camera) {
                    self.toggle_build_menu(ctx);
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
                // Platzieren: Q/E oder Mausrad drehen, Linksklick bauen, Rechtsklick abbrechen
                if let Some((kind, turn)) = self.build_mode {
                    let step = 15f32.to_radians();
                    let scroll = ctx.input.scroll();
                    let mut turn = turn;
                    if ctx.input.key_pressed(KeyCode::KeyQ) || scroll > 0.1 {
                        turn -= step;
                    }
                    if ctx.input.key_pressed(KeyCode::KeyE) || scroll < -0.1 {
                        turn += step;
                    }
                    self.build_mode = Some((kind, turn));
                    if !ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) {
                        ctx.cursor_locked = true;
                    } else if ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) {
                        if let (Some((at, yaw, Ok(_))), Some(session)) = (self.build_site, &mut self.session) {
                            session.request_build(ctx, kind, at, yaw);
                            self.build_mode = None;
                        }
                    } else if ctx.input.mouse_pressed(MouseButton::Right) {
                        self.build_mode = None;
                    }
                    self.jump_requested |= ctx.input.key_pressed(KeyCode::Space);
                    return;
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
            // Hauptmenü: Kamerafahrten um die Schattenfestung und andere Orte
            if let Some(title) = &self.title {
                if let Some(world) = &self.menu_world {
                    title.camera(ctx, world);
                }
            }
            return;
        };
        if self.free_camera {
            self.fly.update(ctx);
            return;
        }
        // Nur für Screenshots: feste Kamera `--kamera x,y,z,gier,neigung` (Grad)
        if let Some(werte) = std::env::args().skip_while(|a| a != "--kamera").nth(1) {
            let v: Vec<f32> = werte.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if v.len() == 5 {
                ctx.camera.position = vec3(v[0], v[1], v[2]);
                ctx.camera.yaw = v[3].to_radians();
                ctx.camera.pitch = v[4].to_radians();
                return;
            }
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
        self.aim_enemy = None;
        if self.tool() == Tool::Staff {
            if let Some(session) = &self.session {
                self.aim_enemy = session
                    .world()
                    .aimed_enemy(ctx.camera.position, ctx.camera.forward(), crate::world::CAST_RANGE + 8.0)
                    .map(|(kind, health, _, flags)| (kind, health, flags));
            }
        }
        let Some(session) = &self.session else { return };
        let Some(local) = session.local_player() else { return };
        let Some(avatar) = session.world().players.get(&local) else { return };
        let hit = ctx.physics.raycast(ctx.camera.position, ctx.camera.forward(), 60.0, Some(avatar.character));
        self.aim_building = match hit {
            Some((Some(entity), distance)) if distance < 45.0 => session.world().building_at(entity),
            _ => None,
        };
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

    /// Kleine Lebensleisten über den Truppen der Festung (rot) und den Soldaten der Kasernen (blau):
    /// volle nur aus der Nähe, verletzte weiter weg, verdeckte gar nicht.
    fn unit_bars_visible(&self, ctx: &Context, egui_ctx: &egui::Context) {
        if self.screen != Screen::Playing {
            return;
        }
        let Some(session) = &self.session else { return };
        let world = session.world();
        let ignore = session.local_player().and_then(|id| world.players.get(&id)).map(|a| a.character);
        let painter = egui_ctx.layer_painter(egui::LayerId::background());
        for (oben, health, freund, boss) in world.lebensleisten() {
            let verletzt = health < 100;
            // Soldaten nur, wenn sie verletzt sind
            if freund && !verletzt {
                continue;
            }
            let distance = ctx.camera.position.distance(oben);
            let limit = if verletzt || boss { 55.0 } else { 30.0 };
            if distance > limit {
                continue;
            }
            let Some(screen) = ctx.world_to_screen(oben) else { continue };
            // Hinter Bäumen, Felsen oder Hügeln verborgen?
            let to = oben - ctx.camera.position;
            if ctx.physics.raycast(ctx.camera.position, to / distance, distance - 0.5, ignore).is_some() {
                continue;
            }
            let alpha = ((limit - distance) / 6.0).clamp(0.0, 1.0);
            let breite = (if boss { 70.0 } else { 40.0 } * (12.0 / distance.max(6.0)).sqrt()).clamp(22.0, 80.0);
            let farbe = if freund { Color32::from_rgb(90, 150, 255) } else if boss { Color32::from_rgb(235, 70, 60) } else { Color32::from_rgb(215, 60, 70) };
            ui::unit_bar(&painter, egui::pos2(screen.x, screen.y), breite, health as f32 / 100.0, alpha, farbe);
        }
    }

    /// Hinweis unter dem Fadenkreuz, wenn ein Rohstoff oder Tier anvisiert ist.
    fn aim_hud(&self, egui_ctx: &egui::Context) {
        // Gebäude im Visier: Name, Stufe und „E“
        if let (None, None, Some(id), Some(session)) = (self.build_mode, self.aim_enemy, self.aim_building, &self.session) {
            if let Some(building) = session.world().buildings.iter().find(|b| b.id == id) {
                let center = egui_ctx.content_rect().center();
                let painter = egui_ctx.layer_painter(egui::LayerId::background());
                let name = if building.tower().is_some() { format!("{} · Stufe {}", building.kind.label(), building.level) } else { building.kind.label().to_string() };
                painter.text(center + egui::vec2(0.0, 28.0), Align2::CENTER_TOP, name, egui::FontId::proportional(18.0), Color32::WHITE);
                let hinweis = if building.tower().is_some() { "E: Turm verwalten" } else { "E: Gebäude verwalten" };
                painter.text(center + egui::vec2(0.0, 50.0), Align2::CENTER_TOP, hinweis, egui::FontId::proportional(14.0), Color32::from_white_alpha(200));
                return;
            }
        }
        // Einheit der Festung im Visier: Name und Lebensleiste
        if let (None, Some((kind, health, flags))) = (self.build_mode, self.aim_enemy) {
            use crate::heer::zustand::*;
            let center = egui_ctx.content_rect().center();
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            painter.text(center + egui::vec2(0.0, 28.0), Align2::CENTER_TOP, kind.label(), egui::FontId::proportional(18.0), Color32::from_rgb(230, 170, 255));
            let mut zustaende = Vec::new();
            for (bit, name) in [
                (FLIEGT, "fliegt"),
                (GETARNT, "getarnt"),
                (SCHILD, "geschützt"),
                (MARKIERT, "markiert"),
                (BETAEUBT, "betäubt"),
                (BRENNT, "brennt"),
                (VERGIFTET, "vergiftet"),
                (VERLANGSAMT, "verlangsamt"),
                (UNVERWUNDBAR, "unverwundbar"),
                (WUT, "in Wut"),
                (GETEERT, "geteert"),
            ] {
                if flags & bit != 0 {
                    zustaende.push(name);
                }
            }
            let zeile = if zustaende.is_empty() { "Linksklick: Zauber".to_string() } else { zustaende.join(" · ") };
            painter.text(center + egui::vec2(0.0, 50.0), Align2::CENTER_TOP, zeile, egui::FontId::proportional(14.0), Color32::from_white_alpha(200));
            ui::health_bar(&painter, center + egui::vec2(0.0, 78.0), 110.0, health as f32 / 100.0, 1.0);
            if let Some(text) = kind.eigenschaft() {
                painter.text(center + egui::vec2(0.0, 90.0), Align2::CENTER_TOP, text, egui::FontId::proportional(13.0), Color32::from_white_alpha(160));
            }
            return;
        }
        // Beim Platzieren: was gebaut wird und ob es hier geht
        if let (Some((kind, _)), Some((_, _, check))) = (self.build_mode, self.build_site) {
            let center = egui_ctx.content_rect().center();
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            painter.text(center + egui::vec2(0.0, 28.0), Align2::CENTER_TOP, kind.label(), egui::FontId::proportional(20.0), Color32::WHITE);
            let (text, color) = match check {
                Ok(_) => ("Linksklick: hier bauen", Color32::from_rgb(140, 230, 130)),
                Err(reason) => (reason, Color32::from_rgb(255, 130, 110)),
            };
            painter.text(center + egui::vec2(0.0, 54.0), Align2::CENTER_TOP, text, egui::FontId::proportional(16.0), color);
            return;
        }
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
        use crate::hauptmenue::epic_button;
        let mut action = None;
        let mut open_gallery = false;
        let now = ctx.time.elapsed;
        if let Some(title) = &mut self.title {
            title.backdrop(ctx, egui_ctx);
        }
        egui::Area::new(egui::Id::new("hauptmenue")).anchor(Align2::LEFT_CENTER, [80.0, -10.0]).show(egui_ctx, |ui| {
            if let Some(title) = &self.title {
                title.title(ui, now);
            }
            ui.add_space(22.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            let width = 320.0;
            if epic_button(ui, "Einzelspieler", width).clicked() {
                action = Some(Mode::Offline);
            }
            if epic_button(ui, "Spiel hosten", width).clicked() {
                action = Some(Mode::Host { port: DEFAULT_PORT });
            }
            if epic_button(ui, "Beitreten", width).clicked() {
                self.screen = Screen::Join;
            }
            if epic_button(ui, "Einstellungen", width).clicked() {
                self.settings_return = Screen::MainMenu;
                self.screen = Screen::Settings;
            }
            if epic_button(ui, "Asset-Galerie", width).clicked() {
                open_gallery = true;
            }
            if epic_button(ui, "Beenden", width).clicked() {
                ctx.exit();
            }
            if let Some(error) = &self.error {
                ui.add_space(6.0);
                ui.label(RichText::new(error).color(ui::ERROR));
            }
            ui.add_space(12.0);
            ui.label(RichText::new(format!("Spielername: {}", self.settings.name)).size(15.0).color(crate::inventar::MUTED));
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

                ui.label("Schadenszahlen");
                ui.checkbox(&mut s.schadenszahlen, "");
                ui.end_row();

                ui.label("Auflösung (3D)");
                let scale_label = |p: u8| if p == 0 { "Automatisch".to_string() } else { format!("{p} %") };
                egui::ComboBox::from_id_salt("aufloesung").selected_text(scale_label(s.render_scale)).show_ui(ui, |ui| {
                    for p in [0u8, 100, 85, 70, 50] {
                        ui.selectable_value(&mut s.render_scale, p, scale_label(p));
                    }
                });
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
        // Bei offenen Fenstern keine Leisten darüber (sie würden Titel und Text verdecken)
        let fenster = self.inventory_open || self.build_menu_open || self.admin_open || self.building_window.is_some() || self.td_open;
        if !fenster {
            ui::time_bar(egui_ctx, session.day());
        }
        // Tower Defense: Wellenleiste, Bosse, Auswertung, Schadenszahlen
        let world = session.world();
        if world.berichte.len() != self.bericht_seit.0 {
            self.bericht_seit = (world.berichte.len(), ctx.time.elapsed);
        }
        let me = local.and_then(|p| world.players.get(&p)).map(|a| crate::save::player_key(&a.name)).unwrap_or_default();
        let bericht = world.berichte.last().map(|b| (b, ctx.time.elapsed - self.bericht_seit.1));
        if !fenster {
            crate::td_ui::wellen_hud(egui_ctx, world, &me, bericht);
        }
        if self.settings.schadenszahlen {
            crate::td_ui::schadenszahlen(ctx, egui_ctx, world);
        }
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
                    // Bildrate: grün ab 50, gelb ab 30, sonst rot
                    let fps = ctx.stats.fps;
                    let farbe = if fps >= 50.0 {
                        Color32::from_rgb(120, 220, 120)
                    } else if fps >= 30.0 {
                        Color32::from_rgb(235, 200, 90)
                    } else {
                        Color32::from_rgb(235, 100, 90)
                    };
                    let scale = ctx.stats.render_scale;
                    let text = if scale < 0.99 { format!("{fps:.0} FPS · 3D {:.0} %", scale * 100.0) } else { format!("{fps:.0} FPS") };
                    ui.label(RichText::new(text).size(14.0).strong().color(farbe));
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

        let hint = if self.build_mode.is_some() {
            "Linksklick Bauen · Q/E oder Mausrad Drehen · Rechtsklick oder B Abbrechen"
        } else if ctx.cursor_locked || self.free_camera {
            "WASD Laufen · Shift Rennen · Leertaste Springen · 1–3 Werkzeug · Linksklick Benutzen · B Bauen · T Verteidigung · I Inventar · M Karte · Enter Chat · Esc Menü"
        } else if self.inventory_open || self.build_menu_open || self.map_open || self.chat.open || self.td_open || self.admin_open || self.building_window.is_some() {
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
            self.chat.prefill("Treffen wir uns an der Festung?");
            self.refresh_cursor(ctx);
        }
        if let Some(position) = args.iter().position(|a| a == "--demo-bau") {
            let name = args.get(position + 1).cloned().unwrap_or_default().to_lowercase();
            self.demo_bau = Some((name, args.get(position + 2).and_then(|p| p.parse().ok())));
        }
        self.demo_build_menu = args.iter().any(|a| a == "--demo-baumenue");
        self.demo_troops = args.iter().any(|a| a == "--demo-truppen" || a == "--demo-kampf");
        self.admin_open = args.iter().any(|a| a == "--demo-admin");
        self.demo_fight = args.iter().any(|a| a == "--demo-kampf");
        self.demo_towers = args.iter().any(|a| a == "--demo-tuerme");
        self.demo_fallen = args.iter().any(|a| a == "--demo-fallen");
        self.td_open = args.iter().any(|a| a == "--demo-td");
        if let Some(position) = args.iter().position(|a| a == "--demo-turmfenster") {
            self.demo_turmfenster = Some(args.get(position + 1).and_then(|n| n.parse().ok()).unwrap_or(0));
            self.demo_towers = true;
        }
        if let Some(position) = args.iter().position(|a| a == "--demo-welle") {
            self.demo_wave = args.get(position + 1).and_then(|n| n.parse().ok());
            self.demo_troops = true;
        }
        if args.iter().any(|a| a == "tuerme") {
            self.build_tab = 1;
        }
        if args.iter().any(|a| a == "fallen") {
            self.build_tab = 2;
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
        self.update_build_preview(ctx);

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
            let spawn = vec2(world.spawn.x, world.spawn.z);
            let target = if name == "laterne" {
                world.places.lanterns.iter().map(|l| vec2(l.x, l.z)).min_by(|a, b| a.distance(spawn).total_cmp(&b.distance(spawn)))
            } else {
                world.places.labels.iter().find(|(label, _)| label.to_lowercase().contains(&name)).map(|&(_, at)| at)
            };
            if let (Some(at), Some(local)) = (target, session.local_player()) {
                let ground = world.terrain.height_at(at.x, at.y);
                let mut away = (vec2(world.spawn.x, world.spawn.z) - at).normalize_or(Vec2::Y);
                // Den See vom Steg aus zeigen (Figur auf dem Steg, Blick über das Wasser)
                if name.contains("see") {
                    if let Some(&(_, base, _)) = world.places.boats.first() {
                        let to_lake = (vec2(base.x, base.z) - at).normalize_or(Vec2::Y);
                        away = -to_lake;
                    }
                }
                let mut stand = at + away * distance;
                // Nicht im Wasser stehen: notfalls näher heran
                while world.terrain.height_at(stand.x, stand.y) < 0.5 && stand.distance(at) > 3.0 {
                    stand -= away;
                }
                if let Some(character) = world.players.get(&local).map(|a| a.character) {
                    // Auf die feste Fläche knapp über dem Gelände stellen (Rampen, Burgböden) –
                    // nicht aufs Dach von Hallen –, sonst aufs Gelände
                    let top = world.terrain.height_at(stand.x, stand.y) + 12.0;
                    let y = ctx
                        .physics
                        .raycast(vec3(stand.x, top, stand.y), Vec3::NEG_Y, 40.0, Some(character))
                        .map(|(_, d)| top - d + 1.0)
                        .unwrap_or(world.terrain.height_at(stand.x, stand.y) + 1.0);
                    ctx.physics.teleport_character(character, vec3(stand.x, y, stand.y));
                }
                self.demo_crystal = Some(Some(vec3(at.x, ground + 1.5, at.y)));
            }
        }
        if let (Some((name, progress)), Some(session)) = (self.demo_bau.take(), &mut self.session) {
            use crate::bauten::{Building, BuildingKind};
            let kind = BuildingKind::ALL.into_iter().find(|k| k.file_name().starts_with(&name)).unwrap_or(BuildingKind::Lumberjack);
            if let Some(local) = session.local_player() {
                session.world_mut().inventories.insert(local, crate::protocol::Inventory { wood: 200, stone: 200, ore: 20, ..Default::default() });
            }
            let world = session.world();
            let spawn = vec2(world.spawn.x, world.spawn.z);
            // Freien Platz nahe beim Start suchen
            let site = (0..480).find_map(|i| {
                let at = spawn + Vec2::from_angle((i % 24) as f32 / 24.0 * std::f32::consts::TAU) * (45.0 + (i / 24) as f32 * 8.0);
                crate::bauten::check_site(world, kind, at, None).ok().map(|y| (at, y))
            });
            if let (Some((at, y)), Some(local)) = (site, session.local_player()) {
                let away = (spawn - at).normalize_or(Vec2::Y);
                let stand = at + away * 17.0 + away.perp() * 4.0;
                let character = world.players[&local].character;
                let ground = world.terrain.height_at(stand.x, stand.y);
                ctx.physics.teleport_character(character, vec3(stand.x, ground + 1.0, stand.y));
                match progress {
                    Some(progress) => {
                        let building = Building { id: 1, kind, position: vec3(at.x, y, at.y), yaw: away.x.atan2(away.y) + 0.5, progress, owner: "demo".into(), produce_in: 40.0, level: 1, zweig: 0, ziel: Default::default() };
                        session.world_mut().place_building(ctx, building);
                    }
                    None => self.build_mode = Some((kind, 0.5)),
                }
                self.demo_crystal = Some(Some(vec3(at.x, y + 1.0, at.y)));
                self.demo_yaw_offset = 0.0;
            }
        }
        if let (true, Some(session)) = (self.demo_troops, &mut self.session) {
            self.demo_troops = false;
            if let Some(welle) = self.demo_wave.take() {
                session.admin(crate::protocol::AdminCommand::SpringeZuWelle(welle.saturating_sub(1)));
            }
            session.admin(crate::protocol::AdminCommand::Waves(true));
        }
        if let (true, Some(session)) = (self.demo_towers, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_towers = false;
                let strasse: Vec<Vec2> = session.world().heer.strassen().next().unwrap_or_default();
                for (i, kind) in crate::tuerme::TowerKind::ALL.into_iter().enumerate() {
                    let k = 8 + i * 3;
                    let (a, b) = (strasse[k], strasse[k + 1]);
                    let p = a + (b - a).normalize().perp() * if i % 2 == 0 { 8.0 } else { -8.0 };
                    let y = session.world().terrain.height_at(p.x, p.y);
                    let building = crate::bauten::Building {
                        id: 500 + i as u32,
                        kind: crate::bauten::BuildingKind::Tower(kind),
                        position: vec3(p.x, y, p.y),
                        yaw: 0.0,
                        progress: 1.0,
                        owner: "demo".into(),
                        produce_in: 0.0,
                        level: 1 + (i % 3) as u8,
                        zweig: 1 + (i / 3 % 2) as u8,
                        ziel: Default::default(),
                    };
                    session.world_mut().place_building(ctx, building);
                }
                session.admin(crate::protocol::AdminCommand::Waves(true));
                let world = session.world();
                let (a, b) = (strasse[26], strasse[27]);
                let stand = a + (b - a).normalize().perp() * 30.0;
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, vec3(stand.x, world.terrain.height_at(stand.x, stand.y) + 1.0, stand.y));
                let ziel = strasse[26];
                self.demo_crystal = Some(Some(vec3(ziel.x, world.terrain.height_at(ziel.x, ziel.y) + 3.0, ziel.y)));
                self.demo_yaw_offset = 0.0;
                if let Some(i) = self.demo_turmfenster.take() {
                    self.building_window = Some(500 + i as u32);
                    session.world_mut().inventories.entry(local).or_default().gold += 1000;
                }
            }
        }
        // Fallen, Barrikade und Kaserne an der Südstraße, Truppen kommen
        if let (true, Some(session)) = (self.demo_fallen, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_fallen = false;
                use crate::bauten::{Building, BuildingKind};
                use crate::tuerme::{FallenArt, TowerKind};
                let strasse: Vec<Vec2> = session.world().heer.strassen().next().unwrap_or_default();
                let teile = [
                    (BuildingKind::Falle(FallenArt::Stacheln), 14, 0.0, 1, 0),
                    (BuildingKind::Falle(FallenArt::Teer), 17, 0.0, 1, 0),
                    (BuildingKind::Falle(FallenArt::Barrikade), 22, 0.0, 1, 0),
                    (BuildingKind::Tower(TowerKind::Barracks), 20, 8.0, 3, 2),
                    (BuildingKind::Tower(TowerKind::Frost), 16, -8.0, 3, 2),
                    (BuildingKind::Tower(TowerKind::Scout), 12, 8.0, 2, 0),
                ];
                for (i, (kind, k, seite, level, zweig)) in teile.into_iter().enumerate() {
                    let (a, b) = (strasse[k], strasse[k + 1]);
                    let dir = (b - a).normalize();
                    let p = a + dir.perp() * seite;
                    let y = session.world().terrain.height_at(p.x, p.y);
                    let building = Building {
                        id: 700 + i as u32,
                        kind,
                        position: vec3(p.x, y, p.y),
                        yaw: dir.x.atan2(dir.y),
                        progress: 1.0,
                        owner: "demo".into(),
                        produce_in: 0.0,
                        level,
                        zweig,
                        ziel: Default::default(),
                    };
                    session.world_mut().place_building(ctx, building);
                }
                session.admin(crate::protocol::AdminCommand::Waves(true));
                let world = session.world();
                let (a, b) = (strasse[19], strasse[20]);
                let stand = a + (b - a).normalize().perp() * -22.0;
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, vec3(stand.x, world.terrain.height_at(stand.x, stand.y) + 1.0, stand.y));
                let ziel = strasse[18];
                self.demo_crystal = Some(Some(vec3(ziel.x, world.terrain.height_at(ziel.x, ziel.y) + 1.0, ziel.y)));
                self.demo_yaw_offset = 0.0;
            }
        }
        if let (true, Some(session)) = (self.demo_fight, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_fight = false;
                let world = session.world();
                let stand = vec2(0.0, 105.0);
                let y = world.terrain.height_at(stand.x, stand.y) + 1.0;
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, vec3(stand.x, y, stand.y));
                self.hotbar_slot = Tool::Staff.slot();
                self.demo_crystal = Some(Some(vec3(0.0, crate::island::festung_hoehe() + 6.0, 40.0)));
                self.demo_yaw_offset = 0.0;
            }
        }
        if let (true, Some(session)) = (self.demo_build_menu, &mut self.session) {
            if let Some(local) = session.local_player() {
                session.world_mut().inventories.insert(local, crate::protocol::Inventory { wood: 27, stone: 14, ore: 3, gold: 180, ..Default::default() });
                self.demo_build_menu = false;
                self.build_menu_open = true;
                self.refresh_cursor(ctx);
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
            crate::messung::messen("welt", || session.world_mut().update_visuals(ctx));
            self.chat.collect(session.world_mut(), ctx.time.elapsed);
            if std::mem::take(&mut self.demo_map_near) {
                let spawn = session.world().spawn;
                self.map_ui.focus(vec2(spawn.x, spawn.z), 3.5);
            }
        } else if let Some(world) = &mut self.menu_world {
            world.update_visuals(ctx);
        }
        crate::messung::messen("kamera", || self.update_camera(ctx));
        let _klang = std::time::Instant::now();
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
        crate::messung::eintragen("klang", _klang);
        crate::messung::messen("zielen", || self.update_aim(ctx));
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
            self.unit_bars_visible(ctx, egui_ctx);
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
            Screen::Playing if self.build_menu_open => self.build_menu(ctx, egui_ctx),
            Screen::Playing if self.admin_open => self.admin_panel(ctx, egui_ctx),
            Screen::Playing if self.building_window.is_some() => self.building_panel(ctx, egui_ctx),
            Screen::Playing if self.td_open => self.td_panel(ctx, egui_ctx),
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

/// Vorschaubilder für das Baumenü (`game/assets/icons/bau_<name>.png`); fehlende fallen weg.
fn load_build_icons(ctx: &egui::Context) -> std::collections::HashMap<crate::bauten::BuildingKind, egui::TextureHandle> {
    let mut icons = std::collections::HashMap::new();
    let Some(dir) = crate::asset_files::asset_dir() else { return icons };
    let alle = crate::bauten::BuildingKind::ALL
        .into_iter()
        .chain(crate::tuerme::TowerKind::ALL.into_iter().map(crate::bauten::BuildingKind::Tower))
        .chain(crate::tuerme::FallenArt::ALL.into_iter().map(crate::bauten::BuildingKind::Falle));
    for kind in alle {
        let path = dir.join("icons").join(format!("bau_{}.png", kind.file_name()));
        let mut image = match Image::load_png(&path) {
            Ok(image) => image,
            Err(message) => {
                log::warn!("Vorschaubild fehlt: {message}");
                continue;
            }
        };
        // egui erwartet vormultiplizierte Farben
        for pixel in image.rgba.chunks_exact_mut(4) {
            let alpha = pixel[3] as f32 / 255.0;
            for c in &mut pixel[..3] {
                *c = (*c as f32 * alpha).round() as u8;
            }
        }
        let color = egui::ColorImage::from_rgba_premultiplied([image.width as usize, image.height as usize], &image.rgba);
        icons.insert(kind, ctx.load_texture(format!("bau_{}", kind.file_name()), color, egui::TextureOptions::LINEAR));
    }
    icons
}
