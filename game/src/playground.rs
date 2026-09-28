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
    /// Ladebildschirm: die Welt entsteht (danach Spielen bzw. Verbinden)
    Laden,
    Connecting,
    Playing,
    Paused,
}

/// Tipps auf dem Ladebildschirm.
const LADE_TIPPS: [&str; 10] = [
    "Vier Runenfragmente aus den Lagern der Wildnis ergeben am Runenbrunnen der Burg einen Runenstein.",
    "Gegner lassen ihre Beute fallen – mit E hebst du sie auf, bevor sie nach fünf Minuten verschwindet.",
    "Waffen gibt es in vier Seltenheiten. Die Farbe der Lichtsäule verrät, wie wertvoll sie ist.",
    "Je näher ein Lager an der Schattenfestung liegt, desto gefährlicher – und desto besser die Beute.",
    "Nach acht Sekunden ohne Treffer heilst du dich von selbst.",
    "Die Frostnova des Magiers bremst Gegner stark – ideal, um Abstand zu gewinnen.",
    "Das Erdbeben des Zwergs betäubt alles um ihn herum.",
    "R zeigt den Bauradius deiner Siedlung und freie Siedlungsplätze.",
    "Mit T öffnest du das Verteidigungsfenster und rufst Wellen früher.",
    "Im Inventar (I) rüstest du unter „Waffen“ erbeutete Stäbe und Hämmer aus.",
];

pub struct Playground {
    /// Direkt in eine Runde starten (Kommandozeile), statt ins Hauptmenü.
    start: Option<Mode>,
    settings: Settings,
    session: Option<Session>,
    /// Ladebildschirm: welche Runde startet und wie viele Bilder er schon zu sehen war
    laden: Option<(Mode, u32)>,
    /// Tipp auf dem Ladebildschirm
    lade_tipp: usize,
    /// Serverbrowser: laufende Suche nach offenen Spielen
    suche: Option<crate::status::Suche>,
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
    /// Wann jede der drei Fähigkeiten wieder bereit ist (Spielzeit in Sekunden)
    bereit_ab: [f32; 4],
    /// Hammerschlag: Takt und Stufe des letzten Schlags (die eigene Figur zeigt die Kombo sofort)
    kombo: Option<(u64, u8)>,
    /// Kamerawackeln, das im letzten Bild auf Gier und Neigung lag (wird wieder abgezogen)
    wackel_versatz: (f32, f32),
    /// Wann die eigene Figur zuletzt getroffen wurde und wann sie gefallen ist (durch wen)
    getroffen_um: f32,
    gefallen: Option<(f32, String)>,
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
    /// Handelsfenster des Händlers (E auf dem Marktplatz) offen
    handel_offen: bool,
    /// Siedlungsradien anzeigen (R)
    radius_an: bool,
    /// Wie viele Auswertungen schon da waren und seit wann die neueste gezeigt wird
    bericht_seit: (usize, f32),
    /// Nur zum Testen: Fallen und Kaserne an die Südstraße (`--demo-fallen`), Turmfenster öffnen
    demo_fallen: bool,
    demo_turmfenster: Option<u8>,
    /// Nur zum Testen: Siedlung am Ende der Südstraße (`--demo-siedlung [Stufe] [Platz]`)
    demo_siedlung: Option<(u8, usize)>,
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
    /// Nur für Screenshots: `--demo-arbeiter <n> [gier]` folgt dem n-ten Arbeiter aus der Nähe
    demo_arbeiter: Option<(usize, f32)>,
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
    /// Nur zum Testen: vor ein Lager der Wildnis (`--demo-lager N`) bzw. an den Runenbrunnen (`--demo-brunnen`)
    demo_lager: Option<usize>,
    /// Nur für Screenshots: `--demo-pilzling [bilder]` – nach so vielen Bildern zündet der nächste Pilzling
    demo_zuenden: Option<u32>,
    /// Nur für Screenshots: `--demo-dungeon d e [raum]` (e = -1: vor dem Eingang)
    demo_dungeon: Option<(usize, i32, Option<usize>)>,
    /// Nur zum Testen: dabei regelmäßig die Fähigkeit N (0–2) aufs Lager einsetzen (`--demo-angriff N`)
    demo_angriff: Option<u8>,
    demo_brunnen: bool,
    /// Nur für Screenshots: `--demo-haendler [fenster]` – vor den Händler stellen (und handeln)
    demo_haendler: Option<bool>,
    /// Nur zum Testen: Beute vor die Figur legen (`--demo-beute`)
    demo_beute: bool,
    /// Nur zum Testen: alle zehn Türme an die Südstraße stellen (`--demo-tuerme`)
    demo_towers: bool,
    /// Nur für Screenshots: `--demo-turmschuss` lässt alle Türme im Takt auf die Straße feuern (reine Optik)
    demo_turmschuss: Option<(f32, Vec<(u32, Vec3)>)>,
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
            laden: None,
            lade_tipp: 0,
            suche: None,
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
            bereit_ab: [0.0; 4],
            kombo: None,
            wackel_versatz: (0.0, 0.0),
            getroffen_um: -10.0,
            gefallen: None,
            inventory_open: false,
            build_menu_open: false,
            build_tab: 0,
            aim_building: None,
            building_window: None,
            admin_open: false,
            noclip: false,
            aim_enemy: None,
            td_open: false,
            handel_offen: false,
            radius_an: false,
            bericht_seit: (0, 0.0),
            demo_fallen: false,
            demo_turmfenster: None,
            demo_siedlung: None,
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
            demo_arbeiter: None,
            demo_spot: None,
            demo_bau: None,
            demo_build_menu: false,
            demo_troops: false,
            demo_fight: false,
            demo_lager: None,
            demo_zuenden: None,
            demo_dungeon: None,
            demo_angriff: None,
            demo_brunnen: false,
            demo_haendler: None,
            demo_beute: false,
            demo_towers: false,
            demo_turmschuss: None,
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

    /// Die Klasse der eigenen Figur.
    fn klasse(&self) -> crate::protocol::CharacterClass {
        self.session
            .as_ref()
            .and_then(|s| s.local_player().and_then(|id| s.world().players.get(&id)))
            .map_or(self.settings.character, |a| a.class)
    }

    /// Setzt die Fähigkeit auf Platz `platz` ein, sobald sie bereit ist (Ziel: das Fadenkreuz).
    fn faehigkeit_nutzen(&mut self, ctx: &Context, platz: u8) {
        let fach = (platz as usize).min(3);
        if ctx.time.elapsed < self.bereit_ab[fach] {
            return;
        }
        let art = crate::faehigkeiten::Faehigkeit::von(self.klasse(), platz);
        let Some((target, _)) = self.spell_aim(ctx) else { return };
        let waffe = self.session.as_ref().map_or(0, |s| s.local_inventory().waffe);
        let ruestung = self.session.as_ref().map_or([0; 3], |s| s.local_inventory().ruestung);
        let (_, _, f_abklingen, _) = crate::waffen::faktoren(waffe, ruestung, self.klasse(), art);
        // Stufe wie beim Server: Kombo des Hammerschlags, Arkanlanze mit voller Ladung
        let stufe = self.naechste_stufe(ctx, art);
        if art.ist_kombo() {
            self.kombo = Some((Self::takt(ctx), stufe));
        }
        let abklingen = (art.abklingen(stufe) as f32 * f_abklingen).round();
        self.bereit_ab[fach] = ctx.time.elapsed + abklingen * Physics::FIXED_DT + 0.05;
        self.last_cast = ctx.time.elapsed;
        self.cast_requested = Some(target);
        if let Some(session) = &mut self.session {
            session.preview_cast(art, stufe);
        }
    }

    /// Spielzeit in Takten (für die Kombo, wie beim Server).
    fn takt(ctx: &Context) -> u64 {
        (ctx.time.elapsed / Physics::FIXED_DT) as u64
    }

    /// Arkane Ladungen der eigenen Figur.
    fn ladung(&self) -> u8 {
        self.session.as_ref().and_then(|s| s.local_player().and_then(|id| s.world().players.get(&id))).map_or(0, |a| a.ladung)
    }

    /// Welche Stufe die Fähigkeit jetzt hätte (Kombo, Arkanlanze).
    fn naechste_stufe(&self, ctx: &Context, art: crate::faehigkeiten::Faehigkeit) -> u8 {
        use crate::faehigkeiten::{kombo_stufe_von, Faehigkeit, LADUNG_MAX};
        match art {
            Faehigkeit::Hammerschlag | Faehigkeit::Klingenhieb => kombo_stufe_von(art, self.kombo, Self::takt(ctx)),
            Faehigkeit::Arkangeschoss if self.ladung() >= LADUNG_MAX => 1,
            _ => 0,
        }
    }

    /// Punkte unter den Fähigkeiten: arkane Ladungen bzw. wie weit die Kombo ist.
    fn faehigkeits_punkte(&self, ctx: &Context) -> [Option<(u8, u8)>; 4] {
        use crate::faehigkeiten::{Faehigkeit, LADUNG_MAX};
        let mut punkte = [None; 4];
        match Faehigkeit::von(self.klasse(), 0) {
            Faehigkeit::Arkangeschoss => punkte[0] = Some((self.ladung(), LADUNG_MAX)),
            art @ (Faehigkeit::Hammerschlag | Faehigkeit::Klingenhieb) => {
                let weiter = self.naechste_stufe(ctx, art);
                punkte[0] = Some((weiter, 3));
            }
            _ => {}
        }
        punkte
    }

    /// Die Beute, die E hier aufheben würde.
    fn beute_hier(&self, ctx: &Context) -> Option<crate::beute::Bodenbeute> {
        let session = self.session.as_ref()?;
        let p = session.world().player_position(ctx, session.local_player()?)?;
        session.world().beute_bei(p - Vec3::Y * 0.9).copied()
    }

    /// Der Durchgang eines Dungeons, vor dem die eigene Figur steht (Index, Text).
    fn durchgang_hier(&self, ctx: &Context) -> Option<(u16, String)> {
        let session = self.session.as_ref()?;
        let p = session.world().player_position(ctx, session.local_player()?)?;
        session.world().durchgang_bei(p - Vec3::Y * 0.9).map(|(i, d)| (i, d.text.clone()))
    }

    /// Steht die eigene Figur beim Händler?
    fn beim_haendler(&self, ctx: &Context) -> bool {
        let Some(session) = self.session.as_ref() else { return false };
        let Some(local) = session.local_player() else { return false };
        session.world().player_position(ctx, local).is_some_and(|p| crate::handel::in_reichweite(p - Vec3::Y * 0.9))
    }

    /// Das Handelsfenster des Händlers (links), siehe `inventar.rs`. Schließt sich, wenn man weggeht.
    fn handel_fenster(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        if !self.beim_haendler(ctx) {
            self.handel_offen = false;
            self.refresh_cursor(ctx);
            return;
        }
        let Some(session) = &self.session else { return };
        let inventory = session.local_inventory();
        let tag = session.world().day.day;
        let class = self.klasse();
        let (befehl, close) = self.inventory_ui.haendler_fenster(egui_ctx, &inventory, class, tag);
        if let (Some(befehl), Some(session)) = (befehl, &mut self.session) {
            session.handel(ctx, befehl);
        }
        if close {
            self.handel_offen = false;
            self.refresh_cursor(ctx);
        }
    }

    /// Was E hier mit Runen tun würde: am Runenbrunnen schmieden oder in einen Schutzstein einsetzen.
    fn runen_hier(&self, ctx: &Context) -> Option<crate::protocol::RunenBefehl> {
        let session = self.session.as_ref()?;
        let world = session.world();
        let p = world.player_position(ctx, session.local_player()?)?;
        let nah = |q: Vec3| vec2(p.x, p.z).distance(vec2(q.x, q.z)) < crate::world::RUNEN_REICHWEITE + 3.0;
        if nah(world.runenbrunnen) {
            return Some(crate::protocol::RunenBefehl::Schmieden);
        }
        world
            .schutzsteine
            .iter()
            .enumerate()
            .find(|&(i, &stein)| nah(stein) && world.dorfhalle_auf_platz(i).is_none())
            .map(|(i, _)| crate::protocol::RunenBefehl::Einsetzen(i as u8))
    }

    /// Schlägt auf den anvisierten Rohstoff, sobald die Abklingzeit um ist. Vorkommen gehen
    /// nur mit der Spitzhacke.
    fn harvest_aimed(&mut self, ctx: &mut Context) {
        let tool = self.tool();
        let Some(id) = self.aim else { return };
        let Some(session) = &mut self.session else { return };
        let Some(kind) = session.world().resources.get(&id).map(|r| r.spec.kind) else { return };
        // Bäume nur mit der Axt, Vorkommen nur mit der Spitzhacke; Kristalle gar nicht von Hand
        if tool != kind.tool() || !kind.von_hand() {
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
        self.handel_offen = false;
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
        let me = local.and_then(|p| world.players.get(&p)).map(|a| crate::save::player_key(&a.name)).unwrap_or_default();
        let mut check = crate::bauten::check_site(world, kind, at, Some(player));
        if check.is_ok() {
            if let Err(grund) = crate::bauten::siedlung_pruefen(world, kind, at, &me) {
                check = Err(grund);
            }
        }
        if check.is_ok() && !crate::bauten::affordable(&session.local_inventory(), kind) {
            check = Err("Nicht genug Rohstoffe");
        }
        let ground = check.unwrap_or_else(|_| world.terrain.height_at(at.x, at.y));
        session.world_mut().set_build_preview(ctx, Some((kind, vec3(at.x, ground, at.y), yaw, check.is_ok())));
        self.build_site = Some((at, yaw, check));
    }

    /// Leuchtende Ringe am Boden: Bauradius der Dorfhallen (eigene golden, fremde hell) und freie
    /// Siedlungsplätze (grün) – mit R oder automatisch beim Platzieren.
    fn update_radien(&mut self, ctx: &mut Context) {
        use crate::bauten::BuildingKind;
        let Some(session) = &mut self.session else { return };
        let world = session.world();
        let me = session.local_player().and_then(|p| world.players.get(&p)).map(|a| crate::save::player_key(&a.name)).unwrap_or_default();
        let bau = self.build_mode.map(|(k, _)| k).filter(|_| self.screen == Screen::Playing);
        let mut wuensche = Vec::new();
        if self.screen == Screen::Playing {
            let hallen = self.radius_an || matches!(bau, Some(BuildingKind::Lumberjack | BuildingKind::Quarry | BuildingKind::Mine));
            if hallen {
                for halle in world.buildings.iter().filter(|b| b.kind == BuildingKind::Dorfhalle) {
                    let farbe = if halle.owner == me { vec4(1.25, 0.8, 0.18, 1.0) } else { vec4(0.75, 0.85, 1.1, 1.0) };
                    wuensche.push((halle.id as u64, halle.position, crate::bauten::bauradius(halle.level), farbe));
                }
            }
            if self.radius_an || bau == Some(BuildingKind::Dorfhalle) {
                for (i, &(mitte, hoehe)) in world.siedlungsplaetze.iter().enumerate() {
                    if world.dorfhalle_auf_platz(i).is_none() {
                        wuensche.push((1_000_000 + i as u64, vec3(mitte.x, hoehe, mitte.y), 26.0, vec4(0.3, 1.15, 0.35, 1.0)));
                    }
                }
            }
        }
        session.world_mut().zeige_radien(ctx, &wuensche);
    }

    /// Admin-Panel (Taste X): Noclip, Wetter, Truppen der Schattenfestung.
    fn admin_panel(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        use crate::protocol::{AdminCommand, WETTER};
        let Some(session) = &mut self.session else { return };
        let strassen: Vec<String> = session.world().td.strassen.iter().map(|s| s.0.clone()).collect();
        let (weather, waves, count, welle, leben, schwierigkeit, endlos) = {
            let world = session.world();
            let td = &world.td;
            (world.weather_choice, td.aktiv, world.feinde.len(), td.welle, td.leben.clone(), td.schwierigkeit, td.endlos)
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
            let leben: Vec<String> = leben.iter().zip(&strassen).map(|(l, s)| format!("{s} {l}")).collect();
            ui.label(RichText::new(format!("Welle {} · Leben je Straße (von {}): {}", welle, schwierigkeit.leben(), leben.join(" · "))).size(14.0));
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
                if ui.button(RichText::new("+4 Runenfragmente").size(15.0)).clicked() {
                    commands.push(AdminCommand::Runenfragmente);
                }
                if ui.button(RichText::new("Lager neu besetzen").size(15.0)).clicked() {
                    commands.push(AdminCommand::LagerNeu);
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
        use crate::bauten::BuildingKind;
        use crate::tuerme::TowerKind;
        let inventory = self.session.as_ref().map(|s| s.local_inventory()).unwrap_or_default();
        let icons = self.build_icons.get_or_insert_with(|| load_build_icons(egui_ctx)).clone();
        // Stufe der eigenen Dorfhalle (keine = noch keine Siedlung)
        let halle_stufe = self.session.as_ref().and_then(|s| {
            let me = s.local_player().and_then(|p| s.world().players.get(&p)).map(|a| crate::save::player_key(&a.name))?;
            s.world().dorfhalle_von(&me).map(|h| h.level)
        });
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
                let hinweis = match halle_stufe {
                    None => "Zuerst deine Dorfhalle auf einem freien Siedlungsplatz am Ende einer Heerstraße bauen. R zeigt die Plätze.",
                    Some(_) => "Wirtschaftsgebäude nur im Radius deiner Dorfhalle (R). Fertige Gebäude liefern dir regelmäßig Rohstoffe.",
                };
                ui.label(RichText::new(hinweis).size(15.0).color(ui::TEXT.gamma_multiply(0.8)));
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
                            let zeile = match kind.produces() {
                                Some(item) => format!("{} Arbeiter bringen je {} {} · Bauzeit {:.0} s", crate::arbeiter::JE_GEBAEUDE, crate::arbeiter::LADUNG, item.label(), kind.build_seconds(1)),
                                None => format!("Bauradius {:.0} m · Bauzeit {:.0} s · ausbaubar zu Rathaus und Burgfried", crate::bauten::bauradius(1), kind.build_seconds(1)),
                            };
                            ui.label(RichText::new(zeile).size(13.0).color(ui::TEXT.gamma_multiply(0.75)));
                            ui.add_space(8.0);
                            // Voraussetzungen der Siedlung
                            let sperre = match (kind, halle_stufe) {
                                (BuildingKind::Dorfhalle, Some(_)) => Some("Schon gebaut"),
                                (BuildingKind::Dorfhalle, None) => None,
                                (_, None) => Some("Erst Dorfhalle bauen"),
                                (BuildingKind::Mine, Some(1)) => Some("Braucht ein Rathaus"),
                                _ => None,
                            };
                            let label = sperre.unwrap_or(if affordable { "Bauen" } else { "Zu wenig Rohstoffe" });
                            if ui.add_enabled(affordable && sperre.is_none(), egui::Button::new(RichText::new(label).size(17.0)).min_size(egui::vec2(ui.available_width(), 34.0))).clicked() {
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
            && !self.handel_offen
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
                // E beim Händler: Handelsfenster auf und zu (Esc schließt auch)
                if self.handel_offen && (escape || ctx.input.key_pressed(KeyCode::KeyE)) {
                    self.handel_offen = false;
                    self.refresh_cursor(ctx);
                    return;
                }
                if ctx.input.key_pressed(KeyCode::KeyE) && self.build_mode.is_none() && self.building_window.is_none() && self.beute_hier(ctx).is_none() && self.beim_haendler(ctx) {
                    self.handel_offen = true;
                    self.inventory_open = false;
                    self.map_open = false;
                    self.refresh_cursor(ctx);
                    return;
                }
                // E: Beute vom Boden aufheben (hat Vorrang)
                if ctx.input.key_pressed(KeyCode::KeyE) && self.build_mode.is_none() && self.building_window.is_none() {
                    if let Some(id) = self.beute_hier(ctx).map(|b| b.id) {
                        if let Some(session) = &mut self.session {
                            session.aufheben(ctx, id);
                        }
                        return;
                    }
                }
                // E an einem Dungeon: hinein, Treppe hinauf/hinab, hinaus
                if ctx.input.key_pressed(KeyCode::KeyE) && self.build_mode.is_none() && self.building_window.is_none() {
                    if let Some((index, _)) = self.durchgang_hier(ctx) {
                        if let Some(session) = &mut self.session {
                            session.durchgang(ctx, index);
                        }
                        return;
                    }
                }
                // E am Runenbrunnen oder an einem Schutzstein: Runenstein schmieden bzw. einsetzen
                if ctx.input.key_pressed(KeyCode::KeyE) && self.build_mode.is_none() && self.building_window.is_none() && self.aim_building.is_none() {
                    if let Some(befehl) = self.runen_hier(ctx) {
                        if let Some(session) = &mut self.session {
                            session.runen(ctx, befehl);
                        }
                        return;
                    }
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
                // R: Radien der Siedlungen und freie Siedlungsplätze zeigen
                if ctx.input.key_pressed(KeyCode::KeyR) && !self.free_camera {
                    self.radius_an = !self.radius_an;
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
                // Linksklick: Werkzeug benutzen – abbauen (gedrückt halten) oder die Fähigkeit einsetzen
                // (den Standardangriff auf Taste 3 kann man gedrückt halten, die anderen je Klick).
                if !ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) && !self.free_camera && !self.inventory_open && !self.map_open {
                    ctx.cursor_locked = true;
                } else if ctx.cursor_locked && matches!(self.tool(), Tool::Pickaxe | Tool::Axe) && ctx.input.mouse(MouseButton::Left) {
                    self.harvest_aimed(ctx);
                } else if let (true, Tool::Faehigkeit(platz)) = (ctx.cursor_locked, self.tool()) {
                    let klick = if platz == 0 { ctx.input.mouse(MouseButton::Left) } else { ctx.input.mouse_pressed(MouseButton::Left) };
                    if klick {
                        self.faehigkeit_nutzen(ctx, platz);
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
        let wackeln = session.world().wackeln();
        // Das Wackeln des letzten Bildes wieder abziehen (Gier und Neigung sammeln sich sonst an)
        ctx.camera.yaw -= self.wackel_versatz.0;
        ctx.camera.pitch -= self.wackel_versatz.1;
        self.orbit.update(ctx, target, Some(character));
        // Kamerawackeln nach Einschlägen in der Nähe (stark gedämpft, weich rauschend)
        let w = wackeln * wackeln;
        let t = ctx.time.elapsed;
        let rauschen = |f: f32, p: f32| (t * f + p).sin() * 0.6 + (t * f * 2.3 + p * 1.7).sin() * 0.4;
        ctx.camera.position += vec3(rauschen(31.0, 0.0), rauschen(27.0, 1.3), rauschen(29.0, 2.1)) * 0.2 * w;
        self.wackel_versatz = (rauschen(23.0, 4.0) * 0.012 * w, rauschen(25.0, 5.0) * 0.012 * w);
        ctx.camera.yaw += self.wackel_versatz.0;
        ctx.camera.pitch += self.wackel_versatz.1;
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
        if matches!(self.tool(), Tool::Faehigkeit(_)) {
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
        let class = self.klasse();
        if self.inventory_ui.window(egui_ctx, &inventory, &name, class) {
            self.toggle_inventory(ctx);
        }
        if let Some(was) = self.inventory_ui.anlegen.take() {
            if let Some(session) = &mut self.session {
                session.anlegen(was);
            }
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
                let name = if building.tower().is_some() || building.kind == crate::bauten::BuildingKind::Dorfhalle {
                    format!("{} · Stufe {}", building.kind.stufen_name(building.level), building.level)
                } else {
                    building.kind.label().to_string()
                };
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
            let zeile = if zustaende.is_empty() { format!("Linksklick: {}", self.tool().label(self.klasse())) } else { zustaende.join(" · ") };
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
            let action = &if matches!(self.tool(), Tool::Faehigkeit(_)) { format!("Linksklick: {}", self.tool().label(self.klasse())) } else { "Tasten 3–5: Fähigkeiten".to_string() };
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
        let (action, color) = if !resource.spec.kind.von_hand() {
            ("Nur die Magier eines Kristallturms lösen diese Kristalle", Color32::from_rgb(150, 205, 255))
        } else if self.tool() == needed {
            let verb = if needed == Tool::Axe { "Linksklick: Holz hacken" } else { "Linksklick: Abbauen" };
            (verb, Color32::from_white_alpha(200))
        } else {
            hint = format!("{} nehmen: Taste {}", needed.label(self.klasse()), needed.slot() + 1);
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
            // Figurenwahl: Magier oder Zwerg
            ui.horizontal(|ui| {
                ui.label(RichText::new("Figur:").size(18.0).color(crate::inventar::MUTED));
                for class in crate::protocol::CharacterClass::ALL {
                    let gewaehlt = self.settings.character == class;
                    let text = RichText::new(class.label()).size(20.0).strong();
                    if ui.selectable_label(gewaehlt, text).clicked() && !gewaehlt {
                        self.settings.character = class;
                        self.settings.save();
                    }
                }
            });
            ui.label(RichText::new(self.settings.character.beschreibung()).size(14.0).color(crate::inventar::MUTED));
            ui.add_space(6.0);
            if epic_button(ui, "Einzelspieler", width).clicked() {
                action = Some(Mode::Offline);
            }
            if epic_button(ui, "Spiel hosten", width).clicked() {
                action = Some(Mode::Host { port: DEFAULT_PORT });
            }
            if epic_button(ui, "Beitreten", width).clicked() {
                self.screen = Screen::Join;
                self.suche = Some(crate::status::Suche::starten(&self.settings.zuletzt));
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
            self.laden_starten(mode);
        }
    }

    /// Zeigt den Ladebildschirm; die Runde startet, sobald er zu sehen ist.
    fn laden_starten(&mut self, mode: Mode) {
        self.lade_tipp = (self.start_zeit_tipp() as usize) % LADE_TIPPS.len();
        self.laden = Some((mode, 0));
        self.screen = Screen::Laden;
    }

    fn start_zeit_tipp(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
    }

    /// Ladebildschirm (beim Starten und beim Verbinden): Titel, Fortschritt, ein Tipp.
    fn lade_bildschirm(&mut self, ctx: &mut Context, egui_ctx: &egui::Context) {
        let verbinden = self.screen == Screen::Connecting;
        let adresse = match self.session.as_ref().map(Session::mode) {
            Some(Mode::Join { address }) => address.clone(),
            _ => String::new(),
        };
        let mut abbrechen = false;
        let jetzt = ctx.time.elapsed;
        egui::Area::new(egui::Id::new("ladebildschirm")).fixed_pos(egui::pos2(0.0, 0.0)).order(egui::Order::Foreground).show(egui_ctx, |ui| {
            let rect = egui_ctx.content_rect();
            ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            let painter = ui.painter();
            // Hintergrund: dunkles Leder mit warmem Schein in der Mitte
            painter.rect_filled(rect, 0.0, Color32::from_rgb(14, 11, 9));
            for i in 0..12 {
                let r = rect.width().max(rect.height()) * (0.12 + i as f32 * 0.07);
                painter.circle_filled(rect.center() - egui::vec2(0.0, 40.0), r, Color32::from_rgba_unmultiplied(120, 80, 30, 6));
            }
            let mitte = rect.center();
            let titel = mitte - egui::vec2(0.0, 150.0);
            crate::inventar::spaced_text(painter, titel + egui::vec2(3.0, 4.0), "ENGINE JN", 72.0, Color32::from_black_alpha(220));
            crate::inventar::spaced_text(painter, titel, "ENGINE JN", 72.0, crate::inventar::GOLD_LIGHT);
            crate::inventar::divider(painter, mitte.x - 220.0, mitte.x + 220.0, titel.y + 52.0);
            crate::inventar::spaced_text(painter, titel + egui::vec2(0.0, 76.0), "FANTASY-INSEL  ·  MULTIPLAYER", 15.0, crate::inventar::PARCHMENT);
            // Fortschritt: ein wandernder Lichtstreif auf einem goldgerahmten Balken
            let balken = egui::Rect::from_center_size(mitte + egui::vec2(0.0, 40.0), egui::vec2(460.0, 14.0));
            painter.rect_filled(balken.expand(3.0), 6.0, Color32::from_rgb(8, 6, 5));
            painter.rect_stroke(balken.expand(3.0), 6.0, egui::Stroke::new(1.2, crate::inventar::GOLD_DARK), egui::StrokeKind::Inside);
            let phase = (jetzt * 0.45).fract();
            let breite = balken.width() * 0.3;
            let links = balken.left() - breite + (balken.width() + breite) * phase;
            let streif = egui::Rect::from_min_max(egui::pos2(links.max(balken.left()), balken.top()), egui::pos2((links + breite).min(balken.right()), balken.bottom()));
            if streif.width() > 0.0 {
                painter.rect_filled(streif, 4.0, crate::inventar::GOLD);
            }
            let status = if verbinden { format!("Verbinde mit {adresse} …") } else { "Die Insel erwacht – Wälder wachsen, Lager werden aufgeschlagen …".to_string() };
            painter.text(balken.center() + egui::vec2(0.0, -30.0), Align2::CENTER_CENTER, status, egui::FontId::proportional(19.0), Color32::WHITE);
            let tipp = format!("Tipp: {}", LADE_TIPPS[self.lade_tipp % LADE_TIPPS.len()]);
            painter.text(mitte + egui::vec2(0.0, 110.0), Align2::CENTER_CENTER, tipp, egui::FontId::proportional(16.0), crate::inventar::MUTED);
            if verbinden {
                let knopf = egui::Rect::from_center_size(mitte + egui::vec2(0.0, 170.0), egui::vec2(220.0, 44.0));
                let mut kind = ui.new_child(egui::UiBuilder::new().max_rect(knopf));
                if crate::hauptmenue::epic_button(&mut kind, "Abbrechen", 220.0).clicked() {
                    abbrechen = true;
                }
            }
        });
        if abbrechen {
            self.show_menu(ctx, None);
        }
    }

    /// Serverbrowser: offizielle Server, Spiele im Heimnetz und die zuletzt benutzten – mit
    /// Spielerzahl und Ping. Unten kann man weiterhin eine Adresse eintippen.
    fn server_browser(&mut self, _ctx: &mut Context, egui_ctx: &egui::Context) {
        let mut beitreten: Option<String> = None;
        let Some(suche) = &mut self.suche else {
            self.suche = Some(crate::status::Suche::starten(&self.settings.zuletzt));
            return;
        };
        suche.abholen();
        if suche.alter() > std::time::Duration::from_secs(6) {
            suche.aktualisieren();
        }
        let mut aktualisieren = false;
        let mut zurueck = false;
        ui::dim_background(egui_ctx, 110);
        ui::center_panel(egui_ctx, "serverbrowser", 640.0, |ui| {
            ui::heading(ui, "Spiel beitreten");
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Du spielst als „{}“ ({}).", self.settings.name, self.settings.character.label())).size(14.0).color(ui::MUTED));
                if suche.sucht() {
                    ui.add(egui::Spinner::new().size(16.0).color(ui::ACCENT));
                    ui.label(RichText::new("Suche …").size(14.0).color(ui::MUTED));
                }
            });
            ui.add_space(6.0);
            let mut eintraege: Vec<&crate::status::Eintrag> = suche.eintraege.values().collect();
            eintraege.sort_by(|a, b| (a.herkunft, a.status.is_none(), &a.name).cmp(&(b.herkunft, b.status.is_none(), &b.name)));
            egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                if eintraege.is_empty() {
                    ui.label(RichText::new("Noch keine Spiele gefunden. Wer im selben Netz „Spiel hosten“ wählt, erscheint hier von selbst.").size(15.0).color(ui::MUTED));
                }
                let mut gruppe = None;
                for e in eintraege {
                    if gruppe != Some(e.herkunft) {
                        gruppe = Some(e.herkunft);
                        ui.add_space(6.0);
                        ui.label(RichText::new(e.herkunft.label()).size(15.0).strong().color(ui::ACCENT));
                    }
                    egui::Frame::new().fill(Color32::from_black_alpha(90)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&e.name).size(18.0).strong());
                                let zeile = match &e.status {
                                    Some(s) if !crate::status::Suche::passt(s) => "andere Spielversion – bitte aktualisieren".to_string(),
                                    Some(s) => {
                                        let welle = if s.welle > 0 { format!(" · Welle {}", s.welle) } else { String::new() };
                                        format!("{} · {}/{} Spieler{welle}", e.adresse, s.spieler, s.max)
                                    }
                                    None => format!("{} · keine Antwort", e.adresse),
                                };
                                ui.label(RichText::new(zeile).size(13.0).color(ui::MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let passt = e.status.as_ref().is_none_or(crate::status::Suche::passt);
                                if ui.add_enabled(passt, egui::Button::new(RichText::new("Beitreten").size(16.0)).min_size(egui::vec2(120.0, 36.0))).clicked() {
                                    beitreten = Some(e.adresse.clone());
                                }
                                if let Some(ping) = e.ping_ms {
                                    let farbe = if ping < 80 { Color32::from_rgb(120, 220, 120) } else if ping < 160 { Color32::from_rgb(235, 200, 90) } else { Color32::from_rgb(235, 100, 90) };
                                    ui.label(RichText::new(format!("{ping} ms")).size(14.0).color(farbe));
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                }
            });
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("Direkt verbinden").size(15.0).strong().color(ui::ACCENT));
            ui.horizontal(|ui| {
                let feld = ui.add(egui::TextEdit::singleline(&mut self.join_address).hint_text("IP oder Name, z. B. 100.64.1.2").desired_width(360.0));
                let enter = feld.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.add_sized([140.0, 30.0], egui::Button::new("Verbinden")).clicked() || enter) && !self.join_address.trim().is_empty() {
                    beitreten = Some(self.join_address.trim().to_string());
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add_sized([180.0, 40.0], egui::Button::new("Aktualisieren")).clicked() {
                    aktualisieren = true;
                }
                if ui.add_sized([180.0, 40.0], egui::Button::new("Zurück")).clicked() {
                    zurueck = true;
                }
            });
        });
        if aktualisieren {
            if let Some(suche) = &mut self.suche {
                suche.aktualisieren();
            }
        }
        if zurueck {
            self.suche = None;
            self.screen = Screen::MainMenu;
        }
        if let Some(adresse) = beitreten {
            let adresse = if adresse.contains(':') { adresse } else { format!("{adresse}:{DEFAULT_PORT}") };
            self.settings.last_address = adresse.clone();
            self.settings.zuletzt.retain(|a| *a != adresse);
            self.settings.zuletzt.insert(0, adresse.clone());
            self.settings.zuletzt.truncate(6);
            self.settings.save();
            self.suche = None;
            self.laden_starten(Mode::Join { address: adresse });
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
                ui.label("");
                ui.label(RichText::new(s.character.beschreibung()).size(13.0).color(ui::TEXT.gamma_multiply(0.7)));
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

    /// Lebensbalken unten links, roter Rand bei Treffern, Meldung nach dem Fallen, Hinweis an
    /// Runenbrunnen und Schutzstein.
    fn lebens_hud(&self, ctx: &Context, egui_ctx: &egui::Context) {
        let Some(session) = &self.session else { return };
        let world = session.world();
        let Some(local) = session.local_player() else { return };
        let Some(avatar) = world.players.get(&local) else { return };
        let (leben, max) = (avatar.leben.max(0.0), avatar.max_leben());
        let anteil = (leben / max).clamp(0.0, 1.0);
        egui::Area::new(egui::Id::new("leben")).anchor(Align2::LEFT_BOTTOM, [16.0, -16.0]).interactable(false).show(egui_ctx, |ui| {
            egui::Frame::new().fill(Color32::from_black_alpha(170)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.label(RichText::new(format!("{} · {}", avatar.name, avatar.class.label())).size(14.0).color(ui::TEXT.gamma_multiply(0.85)));
                let (rect, _) = ui.allocate_exact_size(egui::vec2(230.0, 18.0), egui::Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(rect, 4.0, Color32::from_rgb(40, 14, 14));
                let mut fill = rect;
                fill.set_width(rect.width() * anteil);
                let farbe = if anteil > 0.5 { Color32::from_rgb(200, 45, 45) } else if anteil > 0.25 { Color32::from_rgb(225, 120, 40) } else { Color32::from_rgb(240, 60, 40) };
                painter.rect_filled(fill, 4.0, farbe);
                let mut glanz = fill;
                glanz.set_height(5.0);
                painter.rect_filled(glanz, 3.0, Color32::from_white_alpha(45));
                painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, Color32::from_rgb(120, 90, 60)), egui::StrokeKind::Outside);
                let text = format!("{:.0} / {:.0}", leben.ceil(), max);
                painter.text(rect.center() + egui::vec2(1.0, 1.0), Align2::CENTER_CENTER, &text, egui::FontId::proportional(13.0), Color32::BLACK);
                painter.text(rect.center(), Align2::CENTER_CENTER, &text, egui::FontId::proportional(13.0), Color32::WHITE);
            });
        });
        // Roter Rand nach einem Treffer
        let seit = ctx.time.elapsed - self.getroffen_um;
        if seit < 0.6 {
            let alpha = ((1.0 - seit / 0.6) * 110.0) as u8;
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            let screen = egui_ctx.content_rect();
            for i in 0..6 {
                let rand = screen.shrink(i as f32 * 9.0);
                painter.rect_stroke(rand, 0.0, egui::Stroke::new(9.0, Color32::from_rgba_unmultiplied(170, 10, 10, alpha / (i + 1))), egui::StrokeKind::Inside);
            }
        }
        // Nach dem Fallen
        if let Some((um, von)) = &self.gefallen {
            if ctx.time.elapsed - um < 4.0 {
                let mitte = egui_ctx.content_rect().center() - egui::vec2(0.0, 120.0);
                let painter = egui_ctx.layer_painter(egui::LayerId::background());
                painter.text(mitte + egui::vec2(2.0, 2.0), Align2::CENTER_CENTER, "Du wurdest besiegt", egui::FontId::proportional(38.0), Color32::BLACK);
                painter.text(mitte, Align2::CENTER_CENTER, "Du wurdest besiegt", egui::FontId::proportional(38.0), Color32::from_rgb(230, 70, 60));
                let zeile = format!("von: {von} · Du erwachst an deinem Startpunkt");
                painter.text(mitte + egui::vec2(0.0, 36.0), Align2::CENTER_CENTER, zeile, egui::FontId::proportional(17.0), Color32::from_white_alpha(220));
            }
        }
        // Beute in Reichweite: E zum Aufheben
        if let Some(b) = self.beute_hier(ctx) {
            let mitte = egui_ctx.content_rect().center() + egui::vec2(0.0, 92.0);
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            let f = b.fund.farbe();
            let farbe = Color32::from_rgb((f[0] * 255.0) as u8, (f[1] * 255.0) as u8, (f[2] * 255.0) as u8);
            let zeile = match b.fund {
                crate::beute::Fund::Waffe(id) => crate::waffen::waffe(id).map_or(String::new(), |w| format!("E: {} aufheben ({}) · {}", w.name, w.seltenheit.label(), w.werte_zeilen().join(" · "))),
                crate::beute::Fund::Ruestung(id) => crate::ruestung::ruestung(id).map_or(String::new(), |r| format!("E: {} aufheben ({}, {}) · {}", r.name, r.seltenheit.label(), r.platz.label(), r.werte_zeilen().join(" · "))),
                _ => format!("E: {} aufheben", b.fund.name()),
            };
            painter.text(mitte + egui::vec2(1.0, 1.0), Align2::CENTER_CENTER, &zeile, egui::FontId::proportional(18.0), Color32::BLACK);
            painter.text(mitte, Align2::CENTER_CENTER, &zeile, egui::FontId::proportional(18.0), farbe);
        }
        // Beim Händler
        if !self.handel_offen && self.beute_hier(ctx).is_none() && self.beim_haendler(ctx) {
            let mitte = egui_ctx.content_rect().center() + egui::vec2(0.0, 92.0);
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            let zeile = "Händler · E: Handeln";
            painter.text(mitte + egui::vec2(1.0, 1.0), Align2::CENTER_CENTER, zeile, egui::FontId::proportional(20.0), Color32::BLACK);
            painter.text(mitte, Align2::CENTER_CENTER, zeile, egui::FontId::proportional(20.0), Color32::from_rgb(255, 214, 140));
        }
        // Vor einem Durchgang eines Dungeons
        if let Some((_, text)) = self.durchgang_hier(ctx).filter(|_| self.beute_hier(ctx).is_none()) {
            let mitte = egui_ctx.content_rect().center() + egui::vec2(0.0, 92.0);
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            let zeile = format!("E: {text}");
            painter.text(mitte + egui::vec2(1.0, 1.0), Align2::CENTER_CENTER, &zeile, egui::FontId::proportional(20.0), Color32::BLACK);
            painter.text(mitte, Align2::CENTER_CENTER, &zeile, egui::FontId::proportional(20.0), Color32::from_rgb(255, 214, 140));
        }
        // Am Runenbrunnen oder an einem Schutzstein: was E hier tut
        if self.build_mode.is_none() && self.aim_building.is_none() {
            if let Some(befehl) = self.runen_hier(ctx) {
                let inventar = session.local_inventory();
                let me = crate::save::player_key(&avatar.name);
                let (zeile, bereit) = match befehl {
                    crate::protocol::RunenBefehl::Schmieden => {
                        let n = inventar.runenfragmente;
                        let noetig = crate::protocol::FRAGMENTE_JE_STEIN;
                        (format!("Runenbrunnen · E: Runenstein schmieden ({n}/{noetig} Runenfragmente)"), n >= noetig)
                    }
                    crate::protocol::RunenBefehl::Einsetzen(platz) => match world.td.runen.get(platz as usize).cloned().flatten() {
                        Some(besitzer) if besitzer == me => ("Dein Siedlungsplatz · B: Dorfhalle bauen".to_string(), true),
                        Some(besitzer) => (format!("Siedlungsplatz von {besitzer}"), false),
                        None => (format!("Schutzstein · E: Runenstein einsetzen ({} Runenstein)", inventar.runensteine), inventar.runensteine > 0),
                    },
                };
                let mitte = egui_ctx.content_rect().center() + egui::vec2(0.0, 120.0);
                let painter = egui_ctx.layer_painter(egui::LayerId::background());
                let farbe = if bereit { Color32::from_rgb(150, 215, 255) } else { Color32::from_white_alpha(200) };
                painter.text(mitte + egui::vec2(1.0, 1.0), Align2::CENTER_CENTER, &zeile, egui::FontId::proportional(18.0), Color32::BLACK);
                painter.text(mitte, Align2::CENTER_CENTER, &zeile, egui::FontId::proportional(18.0), farbe);
            }
        }
    }

    fn hud(&mut self, ctx: &Context, egui_ctx: &egui::Context) {
        // Treffer und Gefallene der eigenen Figur abholen
        if let Some(session) = &mut self.session {
            let local = session.local_player();
            let world = session.world_mut();
            for (player, _) in std::mem::take(&mut world.treffer) {
                if Some(player) == local {
                    self.getroffen_um = ctx.time.elapsed;
                }
            }
            for (player, von) in std::mem::take(&mut world.gefallen) {
                if Some(player) == local {
                    self.gefallen = Some((ctx.time.elapsed, von));
                }
            }
        }
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
                // Lebensbalken unter dem Namen
                let abstand = ctx.camera.position.distance(head);
                if abstand < 40.0 {
                    if let Some(screen) = ctx.world_to_screen(head) {
                        let painter = egui_ctx.layer_painter(egui::LayerId::background());
                        let alpha = (1.0 - (abstand - 15.0) / 25.0).clamp(0.0, 1.0);
                        ui::health_bar(&painter, egui::pos2(screen.x, screen.y + 7.0), 64.0, avatar.leben / avatar.max_leben(), alpha);
                    }
                }
                if let Some(text) = self.chat.bubble(id, ctx.time.elapsed) {
                    crate::chat::speech_bubble(ctx, egui_ctx, head + Vec3::Y * 0.45, text);
                }
            }
        }

        // Bei offener Karte nur die Karte (keine Leisten darüber)
        if self.screen != Screen::Playing || self.map_open {
            return;
        }
        // Eigener Lebensbalken über der Figur, sobald sie verletzt ist
        if let Some(avatar) = local.and_then(|id| session.world().players.get(&id)) {
            if avatar.leben < avatar.max_leben() - 0.5 {
                if let Some(entity) = ctx.scene.try_get(avatar.entity) {
                    if let Some(screen) = ctx.world_to_screen(entity.transform.position + Vec3::Y * 1.3) {
                        let painter = egui_ctx.layer_painter(egui::LayerId::background());
                        ui::health_bar(&painter, egui::pos2(screen.x, screen.y), 64.0, avatar.leben / avatar.max_leben(), 1.0);
                    }
                }
            }
        }
        // Beute am Boden: Name in der Farbe der Seltenheit (aus der Nähe)
        for b in session.world().beute.values() {
            let oben = b.ort + Vec3::Y * 0.9;
            let abstand = ctx.camera.position.distance(oben);
            if abstand > 18.0 {
                continue;
            }
            let Some(screen) = ctx.world_to_screen(oben) else { continue };
            let f = b.fund.farbe();
            let farbe = Color32::from_rgb((f[0] * 255.0) as u8, (f[1] * 255.0) as u8, (f[2] * 255.0) as u8);
            let alpha = (1.0 - (abstand - 10.0) / 8.0).clamp(0.0, 1.0);
            let painter = egui_ctx.layer_painter(egui::LayerId::background());
            let pos = egui::pos2(screen.x, screen.y);
            let text = b.fund.name();
            painter.text(pos + egui::vec2(1.0, 1.0), Align2::CENTER_BOTTOM, &text, egui::FontId::proportional(15.0), Color32::from_black_alpha((220.0 * alpha) as u8));
            painter.text(pos, Align2::CENTER_BOTTOM, &text, egui::FontId::proportional(15.0), farbe.gamma_multiply(alpha));
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
        let jetzt = ctx.time.elapsed;
        let abklingen = self.bereit_ab.map(|t| (t - jetzt).max(0.0));
        let punkte = self.faehigkeits_punkte(ctx);
        self.inventory_ui.hotbar(egui_ctx, self.hotbar_slot, self.klasse(), abklingen, punkte);
        self.lebens_hud(ctx, egui_ctx);

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
            "WASD Laufen · Shift Rennen · Leertaste Springen · 1–2 Werkzeug · 3–5 Fähigkeiten · Linksklick Benutzen · E Benutzen · B Bauen · R Siedlung · T Verteidigung · I Inventar · M Karte · Esc Menü"
        } else if self.inventory_open || self.build_menu_open || self.map_open || self.chat.open || self.td_open || self.admin_open || self.building_window.is_some() {
            ""
        } else {
            "Klicken zum Spielen"
        };
        // Über der Auswahlleiste
        egui::Area::new(egui::Id::new("hinweis"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, if ctx.cursor_locked { -132.0 } else { -136.0 }])
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
            let name = match name.to_lowercase().as_str() {
                "bogenschuetze" | "bogen" | "archer" => "Bogenschütze".to_string(),
                "schurke" | "rogue" => "Schurke".to_string(),
                _ => name.clone(),
            };
            if let Some(class) = crate::protocol::CharacterClass::ALL.into_iter().find(|c| c.label().eq_ignore_ascii_case(&name)) {
                self.settings.character = class;
            }
        }
        self.demo_chop = args.iter().any(|a| a == "--demo-hacken");
        self.demo_cast = args.iter().any(|a| a == "--demo-zaubern");
        if self.demo_cast {
            self.hotbar_slot = Tool::ANGRIFF.slot();
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
        if let Some(i) = args.iter().position(|a| a == "--demo-lager") {
            self.demo_lager = Some(args.get(i + 1).and_then(|n| n.parse().ok()).unwrap_or(0));
        }
        if args.iter().any(|a| a == "--demo-pilzling") {
            // Der Pilzkreis wird gesucht, sobald die Welt steht
            self.demo_lager = Some(usize::MAX);
            self.demo_zuenden = Some(args.iter().position(|a| a == "--demo-pilzling").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(40));
        }
        if let Some(i) = args.iter().position(|a| a == "--demo-dungeon") {
            let zahl = |k: usize| args.get(i + k).and_then(|n| n.parse::<i32>().ok());
            self.demo_dungeon = Some((zahl(1).unwrap_or(0).max(0) as usize, zahl(2).unwrap_or(0), zahl(3).map(|r| r.max(0) as usize)));
        }
        self.demo_brunnen = args.iter().any(|a| a == "--demo-brunnen");
        if let Some(i) = args.iter().position(|a| a == "--demo-haendler") {
            self.demo_haendler = Some(args.get(i + 1).is_some_and(|a| a == "fenster"));
        }
        self.demo_beute = args.iter().any(|a| a == "--demo-beute");

        if let Some(i) = args.iter().position(|a| a == "--demo-angriff") {
            self.demo_angriff = Some(args.get(i + 1).and_then(|n| n.parse().ok()).unwrap_or(0));
        }
        self.demo_towers = args.iter().any(|a| a == "--demo-tuerme");
        self.demo_fallen = args.iter().any(|a| a == "--demo-fallen");
        if let Some(position) = args.iter().position(|a| a == "--demo-siedlung") {
            let stufe = args.get(position + 1).and_then(|n| n.parse().ok()).unwrap_or(1);
            let platz = args.get(position + 2).and_then(|n| n.parse().ok()).unwrap_or(0);
            self.demo_siedlung = Some((stufe, platz));
            self.radius_an = true;
        }
        self.td_open = args.iter().any(|a| a == "--demo-td");
        if let Some(position) = args.iter().position(|a| a == "--demo-arbeiter") {
            let n = args.get(position + 1).and_then(|n| n.parse().ok()).unwrap_or(0);
            let gier: f32 = args.get(position + 2).and_then(|n| n.parse().ok()).unwrap_or(70.0);
            self.demo_arbeiter = Some((n, gier));
        }
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
        // Nur für Screenshots: Serverbrowser bzw. Ladebildschirm gleich zeigen
        if args.iter().any(|a| a == "--demo-browser") {
            self.screen = Screen::Join;
        }
        if args.iter().any(|a| a == "--demo-laden") {
            self.screen = Screen::Laden;
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
                    // Nur für Screenshots: `--demo-ausruestung [platz]` – alles erbeutet, einiges angelegt, Reiter
                    // Ausrüstung offen, Tooltip des Platzes sichtbar
                    if let Some(i) = args.iter().position(|a| a == "--demo-ausruestung") {
                        inventory.waffen = u32::MAX;
                        inventory.ruestungen = u32::MAX;
                        inventory.ruestung = [2, 4, 0];
                        inventory.waffe = 4;
                        let platz = args.get(i + 1).and_then(|n| n.parse().ok()).unwrap_or(3);
                        self.inventory_ui.demo(platz);
                    }
                    // Nur für Screenshots: `--demo-ruestung a,b,c` legt diese Teile an (Dateinamen)
                    if let Some(namen) = args.iter().position(|a| a == "--demo-ruestung").and_then(|i| args.get(i + 1)) {
                        for name in namen.split(',') {
                            if let Some(r) = crate::ruestung::RUESTUNGEN.iter().find(|r| r.datei == name) {
                                inventory.ruestungen |= 1 << r.id;
                                inventory.ruestung[r.platz.index()] = r.id;
                            }
                        }
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
        self.update_radien(ctx);
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

        if self.screen == Screen::Laden {
            if let Some((mode, bilder)) = &mut self.laden {
                *bilder += 1;
                if *bilder >= 3 {
                    let mode = mode.clone();
                    self.laden = None;
                    self.start_session(ctx, mode);
                }
            }
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
                session.world_mut().inventories.insert(local, crate::protocol::Inventory { wood: 200, stone: 200, ore: 20, lehm: 50, ..Default::default() });
            }
            let world = session.world();
            let spawn = vec2(world.spawn.x, world.spawn.z);
            // Rohstoffgebäude neben das nächste passende Vorkommen, sonst nahe beim Start
            let vorkommen = crate::arbeiter::rohstoff(kind).and_then(|art| {
                world
                    .resources
                    .values()
                    .filter(|r| r.spec.kind == art)
                    .map(|r| vec2(r.spec.transform.position.x, r.spec.transform.position.z))
                    .filter(|p| crate::bauten::check_site(world, kind, *p + vec2(30.0, 0.0), None).is_ok() || p.distance(spawn) > 60.0)
                    .min_by(|a, b| a.distance(spawn).total_cmp(&b.distance(spawn)))
            });
            let (mitte, anfang) = vorkommen.map_or((spawn, 45.0), |p| (p, 18.0));
            // Freien Platz suchen
            let site = (0..480).find_map(|i| {
                let at = mitte + Vec2::from_angle((i % 24) as f32 / 24.0 * std::f32::consts::TAU) * (anfang + (i / 24) as f32 * 8.0);
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
                    if std::env::args().any(|a| a == "--demo-turmschuss") {
                        let ziel = strasse[k];
                        let schuesse = &mut self.demo_turmschuss.get_or_insert((0.0, Vec::new())).1;
                        schuesse.push((500 + i as u32, vec3(ziel.x, session.world().terrain.height_at(ziel.x, ziel.y) + 0.9, ziel.y)));
                    }
                }
                session.admin(crate::protocol::AdminCommand::Waves(true));
                let world = session.world();
                // Auf der Straße stehen und sie entlang zu den Türmen schauen
                let stand = strasse[22];
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, vec3(stand.x, world.terrain.height_at(stand.x, stand.y) + 1.0, stand.y));
                let ziel = strasse[30];
                self.demo_crystal = Some(Some(vec3(ziel.x, world.terrain.height_at(ziel.x, ziel.y) + 3.0, ziel.y)));
                self.demo_yaw_offset = 0.0;
                if let Some(i) = self.demo_turmfenster.take() {
                    self.building_window = Some(500 + i as u32);
                    session.world_mut().inventories.entry(local).or_default().gold += 1000;
                }
            }
        }
        // Siedlung: Dorfhalle (Stufe) auf einem Siedlungsplatz, Holzfäller und Steinbruch im Radius
        if let (Some((stufe, platz)), Some(session)) = (self.demo_siedlung, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_siedlung = None;
                use crate::bauten::{Building, BuildingKind};
                let me = crate::save::player_key(&session.world().players[&local].name);
                let (mitte, hoehe) = session.world().siedlungsplaetze[platz.min(3)];
                let strasse: Vec<Vec2> = session.world().heer.strassen().nth(platz.min(3)).unwrap_or_default();
                let ende = *strasse.last().unwrap_or(&mitte);
                let blick = (ende - mitte).normalize_or(Vec2::Y);
                let halle = Building {
                    id: 800,
                    kind: BuildingKind::Dorfhalle,
                    position: vec3(mitte.x, hoehe, mitte.y),
                    yaw: blick.x.atan2(blick.y),
                    progress: 1.0,
                    owner: me.clone(),
                    produce_in: 40.0,
                    level: stufe.clamp(1, 3),
                    zweig: 0,
                    ziel: Default::default(),
                };
                session.world_mut().place_building(ctx, halle);
                for (i, kind) in [BuildingKind::Lumberjack, BuildingKind::Quarry, BuildingKind::Mine].into_iter().enumerate() {
                    let world = session.world();
                    let seite = [blick.perp(), -blick.perp(), -blick][i];
                    let at = (0..40).map(|k| mitte + seite * 19.0 - blick * (k as f32 * 0.8)).find(|&at| crate::bauten::check_site(world, kind, at, None).is_ok());
                    if let Some(at) = at {
                        let y = crate::bauten::check_site(world, kind, at, None).unwrap_or(hoehe);
                        let b = Building { id: 801 + i as u32, kind, position: vec3(at.x, y, at.y), yaw: (mitte - at).x.atan2((mitte - at).y), progress: 1.0, owner: me.clone(), produce_in: 40.0, level: 1, zweig: 0, ziel: Default::default() };
                        session.world_mut().place_building(ctx, b);
                    }
                }
                let world = session.world();
                let stand = mitte + blick * 24.0;
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, vec3(stand.x, world.terrain.height_at(stand.x, stand.y) + 1.0, stand.y));
                // Mit --demo-angriff zielt die Figur auf den freien Platz vor der Halle, sonst auf die Halle
                let ziel = if self.demo_angriff.is_some() { mitte + blick * 13.0 } else { mitte };
                self.demo_crystal = Some(Some(vec3(ziel.x, hoehe + if self.demo_angriff.is_some() { 0.6 } else { 3.0 }, ziel.y)));
                self.demo_yaw_offset = 0.0;
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
                    if std::env::args().any(|a| a == "--demo-turmschuss") {
                        let ziel = strasse[k];
                        let schuesse = &mut self.demo_turmschuss.get_or_insert((0.0, Vec::new())).1;
                        schuesse.push((500 + i as u32, vec3(ziel.x, session.world().terrain.height_at(ziel.x, ziel.y) + 0.9, ziel.y)));
                    }
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
        if let (Some((d, e, raum)), Some(session)) = (self.demo_dungeon, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_dungeon = None;
                let world = session.world();
                let character = world.players[&local].character;
                if let Some(dungeon) = world.dungeons.get(d) {
                    let (stand, ziel) = match dungeon.ebenen.get(e.max(0) as usize).filter(|_| e >= 0) {
                        None => {
                            let vor = dungeon.eingang + Quat::from_rotation_y(dungeon.eingang_yaw) * vec3(0.0, 0.0, 14.0);
                            (vec3(vor.x, world.terrain.height_at(vor.x, vor.z) + 1.0, vor.z), dungeon.eingang + Vec3::Y * 3.0)
                        }
                        Some(ebene) => match raum.and_then(|k| ebene.raeume.get(k)) {
                            Some(r) => {
                                let (m, h) = (r.mitte(), r.max.y - r.min.y);
                                let stand = m - vec2(0.0, 0.12 * h);
                                (vec3(stand.x, crate::dungeon::BODEN_Y + 1.2, stand.y), vec3(m.x, crate::dungeon::BODEN_Y + 1.2, m.y + 0.45 * h))
                            }
                            None => {
                                let m = ebene.raeume[0].mitte();
                                (ebene.ankunft_oben(), vec3(m.x, crate::dungeon::BODEN_Y + 1.2, m.y))
                            }
                        },
                    };
                    ctx.physics.teleport_character(character, stand);
                    self.demo_crystal = Some(Some(ziel));
                    self.demo_yaw_offset = 0.0;
                }
            }
        }
        if let (Some(bilder), Some(session)) = (&mut self.demo_zuenden, &mut self.session) {
            if *bilder == 0 {
                if let Some(local) = session.local_player() {
                    if let Some(p) = session.world().player_position(ctx, local) {
                        session.world_mut().wildnis.zuenden(p, local);
                    }
                }
                self.demo_zuenden = None;
            } else {
                *bilder -= 1;
            }
        }
        if let (Some(index), Some(session)) = (self.demo_lager, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_lager = None;
                let world = session.world();
                // Pilzling-Demo: den Pilzkreis auf die offene Wiese vor dem Startplatz holen
                let index = if index == usize::MAX {
                    let index = world.wildnis.lager.iter().position(|l| l.art().name == "Pilzkreis").unwrap_or(0);
                    let wiese = vec2(world.spawn.x, world.spawn.z) + vec2(10.0, 6.0);
                    let world = session.world_mut();
                    let terrain = world.terrain.clone();
                    world.wildnis.verlegen(index, wiese, &|p| terrain.height_at(p.x, p.y));
                    index
                } else {
                    index
                };
                let world = session.world();
                if let Some(lager) = world.wildnis.lager.get(index.min(world.wildnis.lager.len().saturating_sub(1))) {
                    let mitte = lager.mitte;
                    let weg = (mitte - vec2(world.spawn.x, world.spawn.z)).normalize_or(Vec2::X);
                    let stand = mitte - weg * if self.demo_angriff.is_some() || self.demo_zuenden.is_some() { 9.0 } else { 24.0 };
                    let y = world.terrain.height_at(stand.x, stand.y) + 1.0;
                    let character = world.players[&local].character;
                    ctx.physics.teleport_character(character, vec3(stand.x, y, stand.y));
                    self.hotbar_slot = Tool::ANGRIFF.slot();
                    self.demo_crystal = Some(Some(vec3(mitte.x, lager.hoehe + 1.0, mitte.y)));
                    self.demo_yaw_offset = 0.0;
                }
            }
        }
        if let (Some((uhr, schuesse)), Some(session)) = (&mut self.demo_turmschuss, &mut self.session) {
            *uhr -= ctx.time.delta;
            if *uhr <= 0.0 {
                *uhr = 1.1;
                let world = session.world_mut();
                for &(id, ziel) in schuesse.iter() {
                    world.tower_shots.push(crate::tuerme::Schuss { turm: id, ziel });
                }
            }
        }
        if let (Some(platz), Some(Some(ziel))) = (self.demo_angriff, self.demo_crystal) {
            self.hotbar_slot = Tool::Faehigkeit(platz).slot();
            // Der Standardangriff im Takt der Kombo, die anderen gemächlich
            let takt = if platz == 0 { 0.62 } else { 1.6 };
            if ctx.time.elapsed - self.last_cast > takt {
                let art = crate::faehigkeiten::Faehigkeit::von(self.klasse(), platz);
                let stufe = self.naechste_stufe(ctx, art);
                if art.ist_kombo() {
                    self.kombo = Some((Self::takt(ctx), stufe));
                }
                self.last_cast = ctx.time.elapsed;
                self.cast_requested = Some(ziel);
                if let Some(session) = &mut self.session {
                    session.preview_cast(art, stufe);
                }
            }
        }
        if let (true, Some(session)) = (self.demo_beute, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_beute = false;
                use crate::beute::{Bodenbeute, Fund};
                let world = session.world_mut();
                let class = world.players[&local].class;
                let start = world.spawn;
                let mut funde: Vec<Fund> = crate::waffen::WAFFEN.iter().filter(|w| w.klasse == class).map(|w| Fund::Waffe(w.id)).collect();
                funde.extend(crate::ruestung::RUESTUNGEN.iter().filter(|r| r.klasse == class).map(|r| Fund::Ruestung(r.id)));
                funde.extend([Fund::Gegenstand(crate::protocol::Item::Gold, 16), Fund::Gegenstand(crate::protocol::Item::Runenfragment, 1), Fund::Gegenstand(crate::protocol::Item::Meat, 2)]);
                let vorne = vec3(0.0, 0.0, -1.0);
                for (i, fund) in funde.into_iter().enumerate() {
                    let quer = (i as f32 - 3.5) * 1.3;
                    let p = start + vorne * (4.0 + (i % 2) as f32 * 1.5) + vec3(quer, 0.0, 0.0);
                    let ort = vec3(p.x, world.terrain.height_at(p.x, p.z), p.z);
                    world.beute.insert(9000 + i as u32, Bodenbeute { id: 9000 + i as u32, ort, fund });
                }
                self.demo_crystal = Some(Some(start + vorne * 5.0));
                self.demo_yaw_offset = 0.0;
            }
        }
        if let (true, Some(session)) = (self.demo_brunnen, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_brunnen = false;
                let world = session.world();
                let brunnen = world.runenbrunnen;
                let stand = brunnen + crate::island::burg_drehung() * vec3(0.0, 1.2, 14.0);
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, stand);
                self.demo_crystal = Some(Some(brunnen + Vec3::Y * 3.0));
                self.demo_yaw_offset = 0.0;
                session.world_mut().inventories.entry(local).or_default().runenfragmente = 4;
            }
        }
        let class = self.klasse();
        if let (Some(fenster), Some(session)) = (self.demo_haendler, &mut self.session) {
            if let Some(local) = session.local_player() {
                self.demo_haendler = None;
                let ort = crate::handel::ort();
                let vorne = crate::island::burg_welt(vec2(-47.25, 37.9)) - vec2(ort.x, ort.z);
                let vorne = vec3(vorne.x, 0.0, vorne.y).normalize();
                let stand = ort + vorne * 1.6 + vec3(-vorne.z, 0.0, vorne.x) * 3.2 + Vec3::Y * 1.2;
                let character = session.world().players[&local].character;
                ctx.physics.teleport_character(character, stand);
                self.demo_crystal = Some(Some(ort + Vec3::Y * 1.3));
                self.demo_yaw_offset = 0.0;
                let inventory = session.world_mut().inventories.entry(local).or_default();
                inventory.gold = 640;
                for (item, n) in [(Item::Wood, 46), (Item::Stone, 18), (Item::Pelt, 7), (Item::Wool, 5), (Item::Runenfragment, 2)] {
                    inventory.add_item(item, n);
                }
                for w in crate::waffen::WAFFEN.iter().filter(|w| w.klasse == class).take(3) {
                    inventory.waffen |= 1 << w.id;
                }
                for r in crate::ruestung::RUESTUNGEN.iter().filter(|r| r.klasse == class).take(2) {
                    inventory.ruestungen |= 1 << r.id;
                }
                self.handel_offen = fenster;
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
                self.hotbar_slot = Tool::ANGRIFF.slot();
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
                // Nur für Screenshots: `--demo-kamera gier,neigung,abstand` (Grad, Grad, Meter)
                if let Some(werte) = std::env::args().skip_while(|a| a != "--demo-kamera").nth(1) {
                    let v: Vec<f32> = werte.split(',').filter_map(|t| t.trim().parse().ok()).collect();
                    if v.len() == 3 {
                        ctx.camera.yaw = to.x.atan2(-to.z) + v[0].to_radians();
                        ctx.camera.pitch = v[1].to_radians();
                        self.orbit.distance = v[2];
                        self.orbit.avoid_walls = false;
                    }
                }
            }
        }
        if let (Some((n, gier)), Some(session)) = (self.demo_arbeiter, &self.session) {
            let world = session.world();
            if let (Some(a), Some(local)) = (world.arbeiter.get(n), session.local_player()) {
                // Spieler seitlich neben den Arbeiter stellen, Kamera auf ihn richten
                let seite = Vec2::from_angle(a.blick + gier.to_radians()) * 4.2;
                let stand = vec2(a.position.x + seite.x, a.position.z + seite.y);
                let character = world.players[&local].character;
                ctx.physics.teleport_character(character, vec3(stand.x, world.terrain.height_at(stand.x, stand.y) + 0.2, stand.y));
                let to = a.position - vec3(stand.x, a.position.y, stand.y);
                ctx.camera.yaw = to.x.atan2(-to.z) + 0.42;
                ctx.camera.pitch = -0.1;
                self.orbit.distance = 2.2;
                self.orbit.avoid_walls = false;
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
                        session.preview_cast(crate::faehigkeiten::Faehigkeit::von(crate::protocol::CharacterClass::Mage, 0), 0);
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
            Screen::Join => self.server_browser(ctx, egui_ctx),
            Screen::Laden => self.lade_bildschirm(ctx, egui_ctx),
            Screen::Settings => self.settings_menu(ctx, egui_ctx),
            Screen::Connecting => self.lade_bildschirm(ctx, egui_ctx),
            Screen::Paused => self.pause_menu(ctx, egui_ctx),
            Screen::Playing if self.handel_offen => self.handel_fenster(ctx, egui_ctx),
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
