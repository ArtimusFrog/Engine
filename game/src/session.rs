//! Eine laufende Spielrunde: allein, als Host, als Client oder als reiner Server.

use engine::prelude::*;

use crate::client::Replica;
use crate::characters::Action;
use crate::protocol::{Hello, Inventory, PlayerId, PlayerInput, HOST_PLAYER, PROTOCOL_ID};
use crate::save;
use crate::server::Authority;
use crate::world::World;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Allein spielen, kein Netzwerk.
    Offline,
    /// Selbst spielen und gleichzeitig Server für andere sein.
    Host { port: u16 },
    /// Nur Server, ohne Fenster und ohne eigene Figur.
    Server { port: u16 },
    /// Mit einem Server verbinden.
    Join { address: String },
}

pub struct Session {
    mode: Mode,
    world: World,
    authority: Option<Authority>,
    replica: Option<Replica>,
}

impl Session {
    /// Leert die Welt und startet eine neue Runde.
    pub fn start(ctx: &mut Context, mode: Mode, hello: &Hello) -> Result<Session, String> {
        let save_path = save::default_path(matches!(mode, Mode::Server { .. }));
        Self::start_with_save(ctx, mode, hello, save_path)
    }

    /// Wie `start`, mit festem Speicherort für den Spielstand (`None` = nicht speichern).
    pub fn start_with_save(ctx: &mut Context, mode: Mode, hello: &Hello, save_path: Option<std::path::PathBuf>) -> Result<Session, String> {
        ctx.reset_world();
        let is_client = matches!(mode, Mode::Join { .. });
        // Beim Client bewegt der Server die Objekte, nicht die eigene Physik.
        ctx.physics.set_replica(is_client);

        let (authority, replica) = match &mode {
            Mode::Offline => (Some(Authority::new(None, save_path)), None),
            Mode::Host { port } | Mode::Server { port } => {
                let max_clients = if matches!(mode, Mode::Host { .. }) { 16 } else { 32 };
                let net = NetServer::listen(*port, PROTOCOL_ID, max_clients)
                    .map_err(|e| format!("Port {port} lässt sich nicht öffnen: {e}"))?;
                log::info!("Server läuft an Port {}", net.port());
                (Some(Authority::new(Some(net), save_path)), None)
            }
            Mode::Join { address } => {
                let replica = Replica::connect(address, hello).map_err(|e| format!("{address} ist nicht erreichbar: {e}"))?;
                log::info!("Verbinde mit {address} …");
                (None, Some(replica))
            }
        };

        let mut world = World::new(ctx);
        let mut authority = authority;
        if let Some(authority) = &mut authority {
            authority.restore(ctx, &mut world);
        }
        if matches!(mode, Mode::Offline | Mode::Host { .. }) {
            let spawn = world.startpunkt(&hello.name);
            world.spawn_player(ctx, HOST_PLAYER, &hello.name, hello.class, spawn);
            if let Some(authority) = &mut authority {
                authority.welcome_back(&mut world, HOST_PLAYER);
            }
        }
        Ok(Session { mode, world, authority, replica })
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Eigenes Inventar (auf dem Client so, wie der Server es zuletzt gemeldet hat).
    pub fn local_inventory(&self) -> Inventory {
        self.local_player().and_then(|id| self.world.inventories.get(&id).copied()).unwrap_or_default()
    }

    /// Zeigt einen eigenen Schlag sofort an. Beim Host passiert das ohnehin im selben Takt,
    /// beim Client würde man sonst auf die Antwort des Servers warten.
    pub fn preview_harvest(&mut self, ctx: &mut Context, id: u32) {
        if let Some(replica) = &mut self.replica {
            self.world.preview_hit(ctx, id);
            if let Some(local) = replica.local_id() {
                let action = crate::client::harvest_action(&self.world, id);
                self.world.play_action(local, action);
            }
            replica.note_preview(id, ctx.time.tick);
        }
    }

    /// Zauber-Animation der eigenen Figur sofort zeigen (nur Client, siehe `preview_harvest`).
    pub fn preview_cast(&mut self) {
        if let Some(local) = self.replica.as_ref().and_then(Replica::local_id) {
            self.world.play_action(local, Action::Cast);
        }
    }

    /// Schickt eine Chatnachricht (beim Host/Einzelspieler direkt, sonst über den Server).
    pub fn send_chat(&mut self, ctx: &Context, text: &str) {
        let local = self.local_player();
        if let Some(replica) = &mut self.replica {
            replica.send_chat(text);
        } else if let (Some(authority), Some(local)) = (&mut self.authority, local) {
            authority.chat(ctx, &mut self.world, local, text);
        }
    }

    /// Gebäude errichten (beim Host/Einzelspieler direkt, sonst entscheidet der Server).
    pub fn request_build(&mut self, ctx: &mut Context, kind: crate::bauten::BuildingKind, at: Vec2, yaw: f32) {
        let local = self.local_player();
        if let Some(replica) = &mut self.replica {
            replica.send_build(kind, at, yaw);
        } else if let (Some(authority), Some(local)) = (&mut self.authority, local) {
            if let Err(reason) = authority.build(ctx, &mut self.world, local, kind, at, yaw) {
                self.world.chat_events.push(crate::world::ChatLine::notice(reason));
            }
        }
    }

    /// Turm aufwerten (`upgrade`) bzw. eigenes Gebäude abreißen.
    /// Befehl zur Verteidigung (aufwerten, abreißen, zielen, Welle rufen, Straße wählen).
    pub fn td(&mut self, ctx: &mut Context, befehl: crate::td::TdBefehl) {
        let local = self.local_player();
        if let Some(replica) = &mut self.replica {
            replica.send_td(befehl);
        } else if let (Some(authority), Some(local)) = (&mut self.authority, local) {
            if let Err(reason) = authority.td(ctx, &mut self.world, local, befehl) {
                self.world.chat_events.push(crate::world::ChatLine::notice(reason));
            }
        }
    }

    /// Befehl aus dem Admin-Panel (beim Host/Einzelspieler direkt, sonst an den Server).
    pub fn admin(&mut self, command: crate::protocol::AdminCommand) {
        if let Some(replica) = &mut self.replica {
            replica.send_admin(command);
        } else if let Some(authority) = &mut self.authority {
            authority.admin(&mut self.world, HOST_PLAYER, command);
        }
    }

    /// Stellt die Uhr vor (nur wer die Welt berechnet: Einzelspieler, Host, Server).
    pub fn skip_time(&mut self, hours: f32) -> bool {
        if self.authority.is_none() {
            return false;
        }
        self.world.day.advance(hours * self.world.day.seconds_per_hour);
        true
    }

    pub fn day(&self) -> &DayCycle {
        &self.world.day
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// ID der eigenen Figur, sobald es eine gibt.
    pub fn local_player(&self) -> Option<PlayerId> {
        match (&self.mode, &self.replica) {
            (Mode::Server { .. }, _) => None,
            (_, Some(replica)) => replica.local_id(),
            _ => Some(HOST_PLAYER),
        }
    }

    /// Client, der noch auf die Begrüßung des Servers wartet.
    pub fn is_connecting(&self) -> bool {
        self.replica.as_ref().is_some_and(|r| r.local_id().is_none())
    }

    pub fn port(&self) -> Option<u16> {
        self.authority.as_ref().and_then(Authority::port)
    }

    pub fn ping_ms(&self) -> Option<f64> {
        self.replica.as_ref().filter(|r| r.is_connected()).map(Replica::ping_ms)
    }

    #[cfg(test)]
    pub fn corrections(&self) -> u32 {
        self.replica.as_ref().map_or(0, |r| r.corrections)
    }

    /// Ein Takt. Liefert einen Fehlertext, wenn die Verbindung zum Server weg ist.
    pub fn fixed_update(&mut self, ctx: &mut Context, input: PlayerInput) -> Result<(), String> {
        let has_local_player = self.local_player().is_some();
        if let Some(authority) = &mut self.authority {
            authority.tick(ctx, &mut self.world, has_local_player.then_some(input));
            if ctx.is_headless() && ctx.time.tick % (60 * 60) == 0 {
                log::info!("{} Spieler verbunden", authority.player_count());
            }
        }
        if let Some(replica) = &mut self.replica {
            replica.tick(ctx, &mut self.world, input)?;
        }
        Ok(())
    }
}

/// Beim Verlassen der Runde (Menü, Fenster zu, Server-Stopp) den Stand sichern.
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(authority) = &mut self.authority {
            authority.save(&self.world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::ResourceKind;
    use crate::protocol::{CharacterClass, Tool};

    fn hello(name: &str) -> Hello {
        Hello { name: name.into(), class: CharacterClass::Mage }
    }

    /// Minimales Spiel für Tests: eine Runde, deren Figur optional geradeaus läuft
    /// oder auf einen Rohstoff einschlägt.
    struct TestGame {
        session: Session,
        autopilot: bool,
        harvest: Option<u32>,
        cast: Option<Vec3>,
        /// Stab statt Spitzhacke in der Hand
        staff: bool,
        /// Werkzeug in der Hand, wenn nicht gezaubert wird
        tool: Tool,
    }

    impl Game for TestGame {
        fn init(&mut self, _ctx: &mut Context) {}

        fn fixed_update(&mut self, ctx: &mut Context) {
            let input = PlayerInput {
                wish: if self.autopilot { vec2(0.0, -1.0) } else { Vec2::ZERO },
                jump: self.autopilot && ctx.time.tick % 120 == 60,
                harvest: self.harvest,
                // Zum Zaubern den Stab nehmen, sonst die Spitzhacke
                tool: if self.cast.is_some() || self.staff { Tool::Staff } else { self.tool },
                cast: self.cast.take(),
                ..Default::default()
            };
            self.session.fixed_update(ctx, input).expect("Verbindung verloren");
        }

        fn update(&mut self, _ctx: &mut Context) {}
    }

    /// Server und Client im selben Prozess.
    struct Pair {
        server: TestGame,
        server_ctx: Context,
        client: TestGame,
        client_ctx: Context,
    }

    impl Pair {
        fn start(client_autopilot: bool) -> Pair {
            let mut server_ctx = Context::headless();
            let session = Session::start(&mut server_ctx, Mode::Server { port: 0 }, &hello("Server")).unwrap();
            let port = session.port().expect("Server hat keinen Port");
            let server = TestGame { session, autopilot: false, harvest: None, cast: None, staff: false, tool: Tool::Pickaxe };

            let mut client_ctx = Context::headless();
            let address = format!("127.0.0.1:{port}");
            let session = Session::start(&mut client_ctx, Mode::Join { address }, &hello("Testerin")).unwrap();
            let client = TestGame { session, autopilot: client_autopilot, harvest: None, cast: None, staff: false, tool: Tool::Pickaxe };
            Pair { server, server_ctx, client, client_ctx }
        }

        fn run(&mut self, ticks: u32) {
            for _ in 0..ticks {
                self.server_ctx.fixed_tick(&mut self.server);
                self.client_ctx.fixed_tick(&mut self.client);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            self.server_ctx.sync_scene();
            self.client_ctx.sync_scene();
        }
    }

    #[test]
    fn client_verbindet_sich_mit_namen() {
        let mut pair = Pair::start(false);
        pair.run(120);
        let id = pair.client.session.local_player().expect("Client hat keine Spieler-ID bekommen");
        assert!(!pair.client.session.is_connecting());
        assert_eq!(pair.server.session.world().players[&id].name, "Testerin", "Server kennt den Namen nicht");
        assert_eq!(pair.client.session.world().players[&id].name, "Testerin");
        assert_eq!(pair.server.session.world().players[&id].class, CharacterClass::Mage, "Figur kommt nicht an");
    }

    #[test]
    fn server_und_client_bauen_dieselbe_insel() {
        let mut pair = Pair::start(false);
        pair.run(5);
        let (server, client) = (pair.server.session.world(), pair.client.session.world());
        assert!(server.resources.len() > 200, "Zu wenige Rohstoffe: {}", server.resources.len());
        assert_eq!(server.resources.len(), client.resources.len());
        assert_eq!(server.spawn, client.spawn);
        for (id, resource) in &server.resources {
            assert_eq!(resource.spec.transform.position, client.resources[id].spec.transform.position, "Rohstoff {id}");
        }
    }

    #[test]
    fn bewegung_kommt_beim_server_an_und_vorhersage_stimmt() {
        // 2 s geradeaus inkl. Sprung über die Lichtung am Startpunkt.
        let mut pair = Pair::start(true);
        pair.run(120);
        let id = pair.client.session.local_player().unwrap();
        let spawn = pair.server.session.world().spawn;
        let on_server = pair.server.session.world().player_position(&pair.server_ctx, id).unwrap();
        let on_client = pair.client.session.world().player_position(&pair.client_ctx, id).unwrap();

        let moved = vec2(on_server.x - spawn.x, on_server.z - spawn.z).length();
        assert!(moved > 5.0, "Spieler hat sich auf dem Server kaum bewegt: {moved} m");
        // Der Client ist dem Server um die Netzwerk-Laufzeit voraus.
        assert!(on_server.distance(on_client) < 1.5, "Server {on_server} und Client {on_client} liegen zu weit auseinander");
        assert_eq!(pair.client.session.corrections(), 0, "Vorhersage weicht vom Server ab");
    }

    #[test]
    fn schaf_mit_zaubern_erlegen_im_multiplayer() {
        use crate::animals::AnimalKind;
        let mut pair = Pair::start(false);
        pair.run(60);
        let player = pair.client.session.local_player().unwrap();

        // Das Schaf, das dem Startpunkt am nächsten ist; die Figur 4 m daneben stellen.
        let world = pair.server.session.world();
        let (sheep, animal) = world
            .animals
            .iter()
            .enumerate()
            .filter(|(_, a)| a.kind == AnimalKind::Sheep)
            .min_by(|a, b| a.1.position.distance(world.spawn).total_cmp(&b.1.position.distance(world.spawn)))
            .expect("Kein Schaf auf der Insel");
        let stand = animal.position + vec3(4.0, 0.0, 0.0);
        let stand = vec3(stand.x, world.terrain.height_at(stand.x, stand.z) + 1.0, stand.z);
        let character = world.players[&player].character;
        pair.server_ctx.physics.teleport_character(character, stand);
        pair.run(5);

        let max = AnimalKind::Sheep.max_health();
        let mut ticks = 0;
        while pair.server.session.world().animals[sheep].is_alive() && ticks < 60 * 12 {
            // Immer auf die Stelle zielen, an der das Schaf auf dem Server gerade steht.
            pair.client.cast = Some(pair.server.session.world().animals[sheep].hit_sphere().0);
            pair.run(1);
            ticks += 1;
        }
        pair.run(60);

        assert!(!pair.server.session.world().animals[sheep].is_alive(), "Schaf lebt auf dem Server noch");
        assert_eq!(pair.client.session.world().animals[sheep].health, 0, "Schaf lebt beim Client noch");
        // Drei Treffer mit 0,7 s Abklingzeit dazwischen.
        assert!(ticks >= (max as u32 - 1) * 42, "Zauber zu schnell hintereinander: {ticks} Takte");
        let inventory = pair.client.session.local_inventory();
        assert_eq!((inventory.meat, inventory.wool, inventory.pelt), (2, 3, 0), "Beute im Inventar");
    }

    #[test]
    fn baum_faellen_im_multiplayer() {
        let mut pair = Pair::start(false);
        pair.run(60);
        let player = pair.client.session.local_player().unwrap();

        // Nächsten Baum zum Startpunkt suchen und die Figur auf dem Server daneben stellen.
        let world = pair.server.session.world();
        let (&tree, resource) = world
            .resources
            .iter()
            .filter(|(_, r)| r.spec.kind == ResourceKind::Wood)
            .min_by(|a, b| {
                let da = a.1.spec.transform.position.distance(world.spawn);
                let db = b.1.spec.transform.position.distance(world.spawn);
                da.total_cmp(&db)
            })
            .expect("Kein Baum auf der Insel");
        let base = resource.spec.transform.position;
        let hits = resource.spec.max_health as u32;
        let stand = base + vec3(1.2, 0.0, 0.0);
        let stand = vec3(stand.x, world.terrain.height_at(stand.x, stand.z) + 1.0, stand.z);
        let character = world.players[&player].character;
        pair.server_ctx.physics.teleport_character(character, stand);

        // Mit der Spitzhacke geht es nicht …
        pair.client.harvest = Some(tree);
        pair.run(100);
        assert_eq!(pair.server.session.world().resources[&tree].health, resource_health(&pair, tree), "Baum ohne Axt beschädigt");
        // … mit der Axt schon.
        pair.client.tool = Tool::Axe;
        pair.run(hits * crate::world::HARVEST_COOLDOWN_TICKS as u32 + 90);

        assert!(!pair.server.session.world().resources[&tree].is_present(), "Baum steht auf dem Server noch");
        assert!(!pair.client.session.world().resources[&tree].is_present(), "Baum steht beim Client noch");
        let inventory = pair.client.session.local_inventory();
        assert_eq!(inventory.wood, hits - 1 + 4, "Holz im Inventar");
        assert_eq!(inventory.stone, 0);
    }

    /// Stellt die Figur des Clients neben das nächste Vorkommen dieser Art und liefert dessen ID.
    fn neben_vorkommen(pair: &mut Pair, kind: ResourceKind) -> u32 {
        let player = pair.client.session.local_player().unwrap();
        let world = pair.server.session.world();
        let (&id, resource) = world
            .resources
            .iter()
            .filter(|(_, r)| r.spec.kind == kind)
            .min_by(|a, b| a.1.spec.transform.position.distance(world.spawn).total_cmp(&b.1.spec.transform.position.distance(world.spawn)))
            .expect("Kein Vorkommen auf der Insel");
        let stand = resource.spec.transform.position + vec3(1.6, 0.0, 0.0);
        let stand = vec3(stand.x, world.terrain.height_at(stand.x, stand.z) + 1.0, stand.z);
        let character = world.players[&player].character;
        pair.server_ctx.physics.teleport_character(character, stand);
        id
    }

    #[test]
    fn erz_mit_der_spitzhacke_abbauen() {
        let mut pair = Pair::start(false);
        pair.run(60);
        let ore = neben_vorkommen(&mut pair, ResourceKind::Ore);
        let hits = pair.server.session.world().resources[&ore].spec.max_health as u32;
        pair.client.harvest = Some(ore);
        pair.run(hits * crate::world::MINE_COOLDOWN_TICKS as u32 + 90);

        assert!(!pair.server.session.world().resources[&ore].is_present(), "Erzvorkommen steht auf dem Server noch");
        assert!(!pair.client.session.world().resources[&ore].is_present(), "Erzvorkommen steht beim Client noch");
        let inventory = pair.client.session.local_inventory();
        assert_eq!(inventory.ore, hits - 1 + 4, "Erz im Inventar");
        assert_eq!(inventory.stone, 0);
    }

    #[test]
    fn ohne_spitzhacke_kein_abbau() {
        let mut pair = Pair::start(false);
        pair.run(60);
        let stone = neben_vorkommen(&mut pair, ResourceKind::Stone);
        let full = pair.server.session.world().resources[&stone].spec.max_health;
        // Mit dem Stab in der Hand passiert nichts …
        pair.client.staff = true;
        pair.client.harvest = Some(stone);
        pair.run(150);
        assert_eq!(pair.server.session.world().resources[&stone].health, full, "Abbau ohne Spitzhacke");
        assert_eq!(pair.client.session.local_inventory().stone, 0);
        // … mit der Spitzhacke schon.
        pair.client.staff = false;
        pair.run(80);
        assert!(pair.server.session.world().resources[&stone].health < full, "Spitzhacke baut nicht ab");
        assert!(pair.client.session.local_inventory().stone > 0);
    }

    fn resource_health(pair: &Pair, id: u32) -> u8 {
        pair.server.session.world().resources[&id].spec.max_health
    }

    /// Freier Bauplatz im Radius der Dorfhalle von `owner` (Ringe um die Halle).
    fn platz_in_siedlung(world: &World, kind: crate::bauten::BuildingKind, owner: &str) -> Vec2 {
        let halle = world.dorfhalle_von(owner).expect("keine Dorfhalle");
        let mitte = vec2(halle.position.x, halle.position.z);
        (0..240)
            .map(|i| mitte + Vec2::from_angle((i % 24) as f32 / 24.0 * std::f32::consts::TAU) * (14.0 + (i / 24) as f32 * 1.5))
            .find(|&at| crate::bauten::check_site(world, kind, at, None).is_ok() && crate::bauten::siedlung_pruefen(world, kind, at, owner).is_ok())
            .expect("kein Platz in der Siedlung")
    }

    #[test]
    fn gebaeude_bauen_fertigstellen_und_liefern() {
        use crate::bauten::{BuildingKind, PRODUCTION_SECONDS};
        let mut pair = Pair::start(false);
        pair.run(60);
        let id = pair.client.session.local_player().unwrap();
        // Auf den ersten Siedlungsplatz stellen
        let world = pair.server.session.world();
        let (mitte, _) = world.siedlungsplaetze[0];
        let character = world.players[&id].character;
        let stand = mitte + vec2(0.0, 16.0);
        let y = world.terrain.height_at(stand.x, stand.y) + 1.0;
        pair.server_ctx.physics.teleport_character(character, vec3(stand.x, y, stand.y));
        pair.run(40);
        let bauen = |pair: &mut Pair, kind: BuildingKind, at: Vec2| {
            let mut client_ctx = std::mem::replace(&mut pair.client_ctx, Context::headless());
            pair.client.session.request_build(&mut client_ctx, kind, at, 0.3);
            pair.client_ctx = client_ctx;
            pair.run(30);
        };

        // Ohne Dorfhalle kein Holzfäller, ohne Rohstoffe keine Dorfhalle
        pair.server.session.world_mut().inventories.insert(id, Inventory { wood: 60, stone: 25, ..Default::default() });
        bauen(&mut pair, BuildingKind::Lumberjack, mitte + vec2(0.0, -16.0));
        assert!(pair.server.session.world().buildings.is_empty(), "Holzfäller ohne Dorfhalle");
        pair.server.session.world_mut().inventories.insert(id, Inventory::default());
        bauen(&mut pair, BuildingKind::Dorfhalle, mitte);
        assert!(pair.server.session.world().buildings.is_empty(), "Bau ohne Rohstoffe");

        pair.server.session.world_mut().inventories.insert(id, Inventory { wood: 60, stone: 25, ..Default::default() });
        bauen(&mut pair, BuildingKind::Dorfhalle, mitte);
        assert_eq!(pair.server.session.world().buildings.len(), 1, "Server baut die Dorfhalle nicht");
        assert_eq!(pair.client.session.world().buildings.len(), 1, "Client sieht die Baustelle nicht");
        assert_eq!(pair.client.session.local_inventory().wood, 30, "Holz nicht abgezogen");
        assert_eq!(pair.client.session.local_inventory().stone, 10, "Stein nicht abgezogen");

        // Holzfäller im Radius der Dorfhalle
        let at = platz_in_siedlung(pair.server.session.world(), BuildingKind::Lumberjack, "testerin");
        bauen(&mut pair, BuildingKind::Lumberjack, at);
        assert_eq!(pair.server.session.world().buildings.len(), 2, "Holzfäller im Radius abgelehnt");
        assert_eq!(pair.client.session.local_inventory().wood, 10);

        pair.run((BuildingKind::Lumberjack.build_seconds(1) * 60.0) as u32 + 30);
        assert!(pair.server.session.world().buildings.iter().all(|b| b.finished()), "Server: nicht fertig");
        assert!(pair.client.session.world().buildings.iter().all(|b| b.finished()), "Client: nicht fertig");
        pair.run((PRODUCTION_SECONDS * 60.0) as u32 + 30);
        assert_eq!(pair.client.session.local_inventory().wood, 11, "Holzfäller liefert nicht");
    }

    #[test]
    fn siedlung_regeln_im_einzelspieler() {
        use crate::bauten::{Building, BuildingKind};
        let mut ctx = Context::headless();
        let mut session = Session::start_with_save(&mut ctx, Mode::Offline, &hello("Nils"), None).unwrap();
        let local = session.local_player().unwrap();
        let world = session.world();
        // Die Siedlungsplätze sind eben
        for &(mitte, hoehe) in &world.siedlungsplaetze {
            for k in 0..16 {
                let p = mitte + Vec2::from_angle(k as f32 / 16.0 * std::f32::consts::TAU) * 25.0;
                assert!((world.terrain.height_at(p.x, p.y) - hoehe).abs() < 0.2, "Siedlungsplatz uneben bei {p}");
            }
        }
        let (mitte, _) = world.siedlungsplaetze[1];
        let character = world.players[&local].character;
        let stand = mitte + vec2(16.0, 0.0);
        let y = world.terrain.height_at(stand.x, stand.y) + 1.0;
        ctx.physics.teleport_character(character, vec3(stand.x, y, stand.y));
        let genug = Inventory { wood: 500, stone: 500, ore: 100, gold: 1000, ..Default::default() };
        session.world_mut().inventories.insert(local, genug);

        // Nicht irgendwo: nur auf einem Siedlungsplatz
        session.request_build(&mut ctx, BuildingKind::Dorfhalle, mitte + vec2(30.0, 0.0), 0.0);
        assert!(session.world().buildings.is_empty(), "Dorfhalle abseits des Siedlungsplatzes");
        session.request_build(&mut ctx, BuildingKind::Dorfhalle, mitte, 0.0);
        assert_eq!(session.world().buildings.len(), 1, "Dorfhalle auf dem Siedlungsplatz abgelehnt");
        // Nur eine Dorfhalle je Spieler
        let anderer = session.world().siedlungsplaetze[2].0;
        session.request_build(&mut ctx, BuildingKind::Dorfhalle, anderer, 0.0);
        assert_eq!(session.world().buildings.len(), 1, "zweite Dorfhalle");
        // Die Straße gehört jetzt dieser Siedlung
        assert_eq!(session.world().dorfhalle_auf_platz(1).map(|h| h.owner.as_str()), Some("nils"));

        // Erzmine erst ab dem Rathaus
        let at = platz_in_siedlung(session.world(), BuildingKind::Lumberjack, "nils");
        session.request_build(&mut ctx, BuildingKind::Mine, at, 0.0);
        assert_eq!(session.world().buildings.len(), 1, "Erzmine ohne Rathaus");
        session.world_mut().advance_buildings(60.0);
        let halle = session.world().dorfhalle_von("nils").unwrap().id;
        session.td(&mut ctx, crate::td::TdBefehl::Aufwerten(halle, 0));
        assert_eq!(session.world().dorfhalle_von("nils").unwrap().level, 2, "Dorfhalle wird kein Rathaus");
        let at = platz_in_siedlung(session.world(), BuildingKind::Mine, "nils");
        session.request_build(&mut ctx, BuildingKind::Mine, at, 0.0);
        assert_eq!(session.world().buildings.len(), 2, "Erzmine im Radius des Rathauses abgelehnt");
        // Außerhalb des Radius nicht
        let weit = mitte + (mitte - vec2(0.0, 0.0)).normalize() * -60.0;
        session.request_build(&mut ctx, BuildingKind::Quarry, weit, 0.0);
        assert_eq!(session.world().buildings.len(), 2, "Steinbruch außerhalb des Radius");

        // Ein Platz, auf dem schon jemand siedelt, ist vergeben
        let fremd = session.world().siedlungsplaetze[0];
        let anna = Building {
            id: 900,
            kind: BuildingKind::Dorfhalle,
            position: vec3(fremd.0.x, fremd.1, fremd.0.y),
            yaw: 0.0,
            progress: 1.0,
            owner: "anna".into(),
            produce_in: 0.0,
            level: 1,
            zweig: 0,
            ziel: Default::default(),
        };
        session.world_mut().place_building(&mut ctx, anna);
        assert_eq!(crate::bauten::siedlung_pruefen(session.world(), BuildingKind::Dorfhalle, fremd.0, "bert"), Err("Dieser Siedlungsplatz ist schon vergeben"));
        // Wer eine Dorfhalle hat, fängt dort an
        assert!(vec2(session.world().startpunkt("Nils").x, session.world().startpunkt("Nils").z).distance(mitte) < 20.0);
        // Fällt die Straße, wird die Siedlung an ihrem Ende zerstört (Dorfhalle und Gebäude im Radius)
        let vorher = session.world().buildings.len();
        session.world_mut().heer.gefallene_lanes.push(1);
        session.fixed_update(&mut ctx, crate::protocol::PlayerInput::default()).unwrap();
        assert!(session.world().dorfhalle_von("nils").is_none(), "Dorfhalle steht noch");
        assert_eq!(session.world().buildings.len(), vorher - 2, "Gebäude im Radius stehen noch");
        assert!(session.world().dorfhalle_von("anna").is_some(), "fremde Siedlung mit zerstört");
    }

    #[test]
    fn chat_kommt_bei_allen_an_und_wird_gebremst() {
        let mut pair = Pair::start(false);
        pair.run(60);
        let ctx = Context::headless();
        pair.client.session.send_chat(&ctx, "  Hallo Insel!  ");
        pair.run(20);
        let beim_server: Vec<_> = pair.server.session.world_mut().chat_events.drain(..).filter(|l| l.from.is_some()).collect();
        let beim_client: Vec<_> = pair.client.session.world_mut().chat_events.drain(..).filter(|l| l.from.is_some()).collect();
        assert_eq!(beim_server.len(), 1, "Server hat die Nachricht nicht");
        assert_eq!(beim_server[0].text, "Hallo Insel!");
        assert_eq!(beim_server[0].name, "Testerin");
        assert_eq!(beim_client.len(), 1, "Nachricht kommt beim Client nicht zurück");
        // Flut: höchstens fünf Nachrichten in fünf Sekunden
        for i in 0..10 {
            pair.client.session.send_chat(&ctx, &format!("Nachricht {i}"));
        }
        pair.run(20);
        let angekommen = pair.client.session.world_mut().chat_events.drain(..).filter(|l| l.from.is_some()).count();
        assert_eq!(angekommen, 4, "Tempobremse greift nicht");
        // Leere Nachrichten werden verworfen
        pair.client.session.send_chat(&ctx, "   ");
        pair.run(10);
        assert_eq!(pair.client.session.world_mut().chat_events.drain(..).filter(|l| l.from.is_some()).count(), 0);
    }

    #[test]
    fn abbauen_aus_der_ferne_wird_abgelehnt() {
        let mut pair = Pair::start(false);
        pair.run(60);
        let world = pair.server.session.world();
        let far = world
            .resources
            .iter()
            .max_by(|a, b| {
                let da = a.1.spec.transform.position.distance(world.spawn);
                let db = b.1.spec.transform.position.distance(world.spawn);
                da.total_cmp(&db)
            })
            .map(|(&id, _)| id)
            .unwrap();
        pair.client.harvest = Some(far);
        pair.run(120);
        let resource = &pair.server.session.world().resources[&far];
        assert_eq!(resource.health, resource.spec.max_health, "Server hat einen Schlag aus der Ferne angenommen");
    }


    #[test]
    fn uhrzeit_kommt_beim_client_an() {
        let mut pair = Pair::start(false);
        pair.run(30);
        assert!(!pair.client.session.skip_time(5.0), "Client darf die Zeit nicht verstellen");
        // 12 Stunden ab 8 Uhr: 20 Uhr (noch Tag, gleiche Geschwindigkeit überall).
        assert!(pair.server.session.skip_time(12.0));
        pair.run(90);
        let (server, client) = (pair.server.session.day(), pair.client.session.day());
        assert_eq!(server.day, client.day);
        assert!((server.hour - client.hour).abs() < 0.05, "Server {} / Client {}", server.clock(), client.clock());
        assert!(server.hour > 19.9, "Zeitsprung fehlt: {}", server.clock());
    }

    #[test]
    fn tiere_bewegen_sich_beim_client_mit() {
        let mut pair = Pair::start(false);
        let start: Vec<Vec3> = pair.server.session.world().animals.iter().map(|a| a.position).collect();
        assert!(start.len() > 20, "zu wenige Tiere: {}", start.len());
        pair.run(60 * 12);
        let server = &pair.server.session.world().animals;
        let client = &pair.client.session.world().animals;
        assert_eq!(server.len(), client.len());
        let moved = server.iter().zip(&start).filter(|(a, s)| a.position.distance(**s) > 1.0).count();
        assert!(moved > 3, "Tiere laufen nicht umher: {moved}");
        // Der Client zeigt sie leicht verzögert (Interpolation), aber am selben Ort.
        for (s, c) in server.iter().zip(client) {
            assert!(s.position.distance(c.position) < 1.5, "Tier weicht ab: Server {:?} / Client {:?}", s.position, c.position);
        }
    }


    #[test]
    fn spielstand_ueberlebt_neustart() {
        let path = std::env::temp_dir().join(format!("welt_test_{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let tree = {
            let mut ctx = Context::headless();
            let mut session = Session::start_with_save(&mut ctx, Mode::Offline, &hello("Nils"), Some(path.clone())).unwrap();
            let world = session.world_mut();
            let tree = world.resources.iter().find(|(_, r)| r.spec.kind == ResourceKind::Wood).map(|(&id, _)| id).unwrap();
            world.resource_hit(&mut ctx, tree, 0, false);
            world.inventories.insert(HOST_PLAYER, Inventory { wood: 9, stone: 2, ..Default::default() });
            world.day.hour = 21.5;
            tree
            // Ende des Blocks: Session wird geschlossen und speichert.
        };
        assert!(path.exists(), "kein Spielstand geschrieben");

        let mut ctx = Context::headless();
        let session = Session::start_with_save(&mut ctx, Mode::Offline, &hello("nils"), Some(path.clone())).unwrap();
        let world = session.world();
        assert_eq!(world.inventories.get(&HOST_PLAYER), Some(&Inventory { wood: 9, stone: 2, ..Default::default() }), "Inventar (Name ohne Groß/klein)");
        assert!(!world.resources[&tree].is_present(), "gefällter Baum steht wieder");
        assert!((world.day.hour - 21.5).abs() < 0.01);
        drop(session);
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn belegter_port_liefert_fehlermeldung() {
        let mut ctx = Context::headless();
        let first = Session::start(&mut ctx, Mode::Server { port: 0 }, &hello("A")).unwrap();
        let port = first.port().unwrap();
        let mut ctx2 = Context::headless();
        let error = Session::start(&mut ctx2, Mode::Host { port }, &hello("B")).err().expect("Zweiter Server auf demselben Port");
        assert!(error.contains(&port.to_string()), "{error}");
    }
}
