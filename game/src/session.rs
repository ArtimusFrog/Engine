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
            let spawn = world.spawn;
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
    }

    impl Game for TestGame {
        fn init(&mut self, _ctx: &mut Context) {}

        fn fixed_update(&mut self, ctx: &mut Context) {
            let input = PlayerInput {
                wish: if self.autopilot { vec2(0.0, -1.0) } else { Vec2::ZERO },
                jump: self.autopilot && ctx.time.tick % 120 == 60,
                harvest: self.harvest,
                // Zum Zaubern den Stab nehmen, sonst die Spitzhacke
                tool: if self.cast.is_some() || self.staff { Tool::Staff } else { Tool::Pickaxe },
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
            let server = TestGame { session, autopilot: false, harvest: None, cast: None, staff: false };

            let mut client_ctx = Context::headless();
            let address = format!("127.0.0.1:{port}");
            let session = Session::start(&mut client_ctx, Mode::Join { address }, &hello("Testerin")).unwrap();
            let client = TestGame { session, autopilot: client_autopilot, harvest: None, cast: None, staff: false };
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

        pair.client.harvest = Some(tree);
        pair.run(hits * 30 + 60);

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
