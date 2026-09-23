//! Eine laufende Spielrunde: allein, als Host, als Client oder als reiner Server.

use engine::prelude::*;

use crate::client::Replica;
use crate::protocol::{PlayerId, PlayerInput, HOST_PLAYER, PROTOCOL_ID};
use crate::server::Authority;
use crate::world::{World, SPAWN_POINT};

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
    pub fn start(ctx: &mut Context, mode: Mode, name: &str) -> Result<Session, String> {
        ctx.reset_world();
        let is_client = matches!(mode, Mode::Join { .. });
        // Beim Client bewegt der Server die Objekte, nicht die eigene Physik.
        ctx.physics.set_replica(is_client);

        let (authority, replica) = match &mode {
            Mode::Offline => (Some(Authority::new(None)), None),
            Mode::Host { port } | Mode::Server { port } => {
                let max_clients = if matches!(mode, Mode::Host { .. }) { 16 } else { 32 };
                let net = NetServer::listen(*port, PROTOCOL_ID, max_clients)
                    .map_err(|e| format!("Port {port} lässt sich nicht öffnen: {e}"))?;
                log::info!("Server läuft an Port {}", net.port());
                (Some(Authority::new(Some(net))), None)
            }
            Mode::Join { address } => {
                let replica = Replica::connect(address, name).map_err(|e| format!("{address} ist nicht erreichbar: {e}"))?;
                log::info!("Verbinde mit {address} …");
                (None, Some(replica))
            }
        };

        let mut world = World::new(ctx);
        if matches!(mode, Mode::Offline | Mode::Host { .. }) {
            world.spawn_player(ctx, HOST_PLAYER, name, SPAWN_POINT);
        }
        Ok(Session { mode, world, authority, replica })
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimales Spiel für Tests: eine Runde, deren Figur optional geradeaus läuft.
    struct TestGame {
        session: Session,
        autopilot: bool,
    }

    impl Game for TestGame {
        fn init(&mut self, _ctx: &mut Context) {}

        fn fixed_update(&mut self, ctx: &mut Context) {
            let input = PlayerInput {
                wish: if self.autopilot { vec2(0.0, -1.0) } else { Vec2::ZERO },
                jump: self.autopilot && ctx.time.tick % 120 == 60,
                ..Default::default()
            };
            self.session.fixed_update(ctx, input).expect("Verbindung verloren");
        }

        fn update(&mut self, _ctx: &mut Context) {}
    }

    /// Startet einen Server und einen Client im selben Prozess und lässt beide laufen.
    fn run_pair(ticks: u32, client_autopilot: bool) -> (TestGame, Context, TestGame, Context) {
        let mut server_ctx = Context::headless();
        let session = Session::start(&mut server_ctx, Mode::Server { port: 0 }, "Server").unwrap();
        let port = session.port().expect("Server hat keinen Port");
        let mut server = TestGame { session, autopilot: false };

        let mut client_ctx = Context::headless();
        let session = Session::start(&mut client_ctx, Mode::Join { address: format!("127.0.0.1:{port}") }, "Testerin").unwrap();
        let mut client = TestGame { session, autopilot: client_autopilot };

        for _ in 0..ticks {
            server_ctx.fixed_tick(&mut server);
            client_ctx.fixed_tick(&mut client);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        server_ctx.sync_scene();
        client_ctx.sync_scene();
        (server, server_ctx, client, client_ctx)
    }

    #[test]
    fn client_verbindet_sich_mit_namen() {
        let (server, _, client, _) = run_pair(120, false);
        let id = client.session.local_player().expect("Client hat keine Spieler-ID bekommen");
        assert!(!client.session.is_connecting());
        assert_eq!(server.session.world().players[&id].name, "Testerin", "Server kennt den Namen nicht");
        assert_eq!(client.session.world().players[&id].name, "Testerin");
    }

    #[test]
    fn bewegung_kommt_beim_server_an_und_vorhersage_stimmt() {
        // 2 s geradeaus inkl. Sprung, noch bevor die Figur die Kisten erreicht.
        let (server, server_ctx, client, client_ctx) = run_pair(120, true);
        let id = client.session.local_player().unwrap();
        let on_server = server.session.world().player_position(&server_ctx, id).unwrap();
        let on_client = client.session.world().player_position(&client_ctx, id).unwrap();

        assert!(on_server.z < SPAWN_POINT.z - 5.0, "Spieler hat sich auf dem Server nicht bewegt: {on_server}");
        // Der Client ist dem Server um die Netzwerk-Laufzeit voraus.
        assert!(on_server.distance(on_client) < 1.5, "Server {on_server} und Client {on_client} liegen zu weit auseinander");
        assert_eq!(client.session.corrections(), 0, "Vorhersage weicht vom Server ab");
    }

    #[test]
    fn kisten_bewegen_sich_auf_dem_client_mit() {
        // Autopilot läuft vom Startpunkt geradeaus in die Kistenpyramide.
        let (server, server_ctx, client, client_ctx) = run_pair(420, true);
        let mut moved = 0;
        for (id, object) in &server.session.world().objects {
            let (on_server, _) = server_ctx.physics.body_pose(object.body).unwrap();
            let (on_client, _) = client_ctx.physics.body_pose(client.session.world().objects[id].body).unwrap();
            assert!(on_server.distance(on_client) < 1.0, "Objekt {id}: Server {on_server}, Client {on_client}");
            if on_server.distance(Vec3::new(0.0, 0.0, -3.0)) > 2.5 {
                moved += 1;
            }
        }
        assert!(moved > 0, "Keine Kiste wurde umgestoßen");
    }

    #[test]
    fn belegter_port_liefert_fehlermeldung() {
        let mut ctx = Context::headless();
        let first = Session::start(&mut ctx, Mode::Server { port: 0 }, "A").unwrap();
        let port = first.port().unwrap();
        let mut ctx2 = Context::headless();
        let error = Session::start(&mut ctx2, Mode::Host { port }, "B").err().expect("Zweiter Server auf demselben Port");
        assert!(error.contains(&port.to_string()), "{error}");
    }
}
