//! Der Spielplatz: verbindet Welt, Netzwerk-Rolle, Eingabe und Kamera.

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

pub struct Playground {
    mode: Mode,
    world: Option<World>,
    authority: Option<Authority>,
    replica: Option<Replica>,
    orbit: OrbitController,
    fly: FlyController,
    free_camera: bool,
    // Eingaben, die zwischen zwei Takten fallen, bis zum nächsten Takt merken.
    jump_requested: bool,
    throw_requested: bool,
    /// Nur zum Testen: Figur läuft von allein.
    autopilot: bool,
}

impl Playground {
    pub fn new(mode: Mode, autopilot: bool) -> Self {
        Playground {
            mode,
            world: None,
            authority: None,
            replica: None,
            orbit: OrbitController::default(),
            fly: FlyController::default(),
            free_camera: false,
            jump_requested: false,
            throw_requested: false,
            autopilot,
        }
    }

    /// ID der eigenen Figur, sobald es eine gibt.
    pub fn local_player(&self) -> Option<PlayerId> {
        match (&self.mode, &self.replica) {
            (Mode::Server { .. }, _) => None,
            (_, Some(replica)) => replica.local_id(),
            _ => Some(HOST_PLAYER),
        }
    }

    #[cfg(test)]
    pub fn world(&self) -> &World {
        self.world.as_ref().expect("Welt ist noch nicht aufgebaut")
    }

    #[cfg(test)]
    pub fn server_port(&self) -> Option<u16> {
        self.authority.as_ref().and_then(Authority::port)
    }

    #[cfg(test)]
    pub fn corrections(&self) -> u32 {
        self.replica.as_ref().map_or(0, |r| r.corrections)
    }

    fn build_input(&mut self, ctx: &Context) -> PlayerInput {
        let yaw = ctx.camera.yaw;
        let (forward, right) = (vec2(yaw.sin(), -yaw.cos()), vec2(yaw.cos(), yaw.sin()));
        let mut wish = Vec2::ZERO;
        if !self.free_camera {
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
            sprint: ctx.input.key(KeyCode::ShiftLeft),
            jump: self.jump_requested,
            throw: self.throw_requested.then(|| ctx.camera.forward()),
        };
        self.jump_requested = false;
        self.throw_requested = false;
        input
    }

    fn update_status(&self, ctx: &mut Context) {
        ctx.status = match (&self.mode, &self.authority, &self.replica) {
            (Mode::Offline, ..) => "Einzelspieler".into(),
            (_, Some(authority), _) => format!(
                "Host · Port {} · {} Mitspieler",
                authority.port().unwrap_or(0),
                authority.player_count()
            ),
            (Mode::Join { address }, _, Some(replica)) if replica.is_connected() => {
                format!("Verbunden mit {address} · Ping {:.0} ms", replica.ping_ms())
            }
            (Mode::Join { address }, ..) => format!("Verbinde mit {address} …"),
            _ => String::new(),
        };
    }
}

impl Game for Playground {
    fn init(&mut self, ctx: &mut Context) {
        ctx.camera.pitch = -0.35;
        if matches!(self.mode, Mode::Join { .. }) {
            // Beim Client bewegt der Server die Objekte, nicht die eigene Physik.
            ctx.physics.set_replica(true);
        }
        let mut world = World::new(ctx);

        match self.mode.clone() {
            Mode::Offline => {
                self.authority = Some(Authority::new(None));
                world.spawn_player(ctx, HOST_PLAYER, SPAWN_POINT);
            }
            Mode::Host { port } => {
                match NetServer::listen(port, PROTOCOL_ID, 16) {
                    Ok(net) => {
                        log::info!("Host: warte auf Mitspieler an Port {}", net.port());
                        self.authority = Some(Authority::new(Some(net)));
                    }
                    Err(e) => {
                        log::error!("Port {port} ist nicht verfügbar ({e}), spiele allein");
                        self.mode = Mode::Offline;
                        self.authority = Some(Authority::new(None));
                    }
                }
                world.spawn_player(ctx, HOST_PLAYER, SPAWN_POINT);
            }
            Mode::Server { port } => {
                let net = NetServer::listen(port, PROTOCOL_ID, 32)
                    .unwrap_or_else(|e| panic!("Server kann Port {port} nicht öffnen: {e}"));
                log::info!("Server läuft an Port {}", net.port());
                self.authority = Some(Authority::new(Some(net)));
            }
            Mode::Join { address } => match Replica::connect(&address) {
                Ok(replica) => {
                    log::info!("Verbinde mit {address} …");
                    self.replica = Some(replica);
                }
                Err(e) => {
                    log::error!("Verbindung zu {address} nicht möglich: {e}");
                    ctx.exit();
                }
            },
        }
        self.world = Some(world);
    }

    fn fixed_update(&mut self, ctx: &mut Context) {
        let has_local_player = self.local_player().is_some();
        let input = self.build_input(ctx);
        let Some(world) = &mut self.world else { return };

        if let Some(authority) = &mut self.authority {
            authority.tick(ctx, world, has_local_player.then_some(input));
            if ctx.is_headless() && ctx.time.tick % (60 * 30) == 0 {
                log::info!("{} Spieler verbunden", authority.player_count());
            }
        }
        if let Some(replica) = &mut self.replica {
            if let Err(reason) = replica.tick(ctx, world, input) {
                log::error!("Verbindung zum Server verloren: {reason}");
                ctx.exit();
            }
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        // Maus einfangen: Klick ins Fenster. Freigeben: Esc.
        if ctx.input.key_pressed(KeyCode::Escape) {
            ctx.cursor_locked = false;
        } else if !ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) && !self.free_camera {
            ctx.cursor_locked = true;
        } else if ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) {
            self.throw_requested = true;
        }
        self.jump_requested |= ctx.input.key_pressed(KeyCode::Space);

        if ctx.input.key_pressed(KeyCode::F1) {
            self.free_camera = !self.free_camera;
            ctx.cursor_locked = false;
        }
        self.update_status(ctx);

        let local = self.local_player();
        let Some(world) = &self.world else { return };
        world.update_visuals(ctx);

        if self.free_camera {
            self.fly.update(ctx);
            return;
        }
        let Some(avatar) = local.and_then(|id| world.players.get(&id)) else { return };
        let Some(entity) = ctx.scene.try_get(avatar.entity) else { return };
        let target = entity.transform.position + Vec3::Y * 0.6;
        self.orbit.update(ctx, target, Some(avatar.character));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Startet einen Server und einen Client im selben Prozess und lässt beide laufen.
    fn run_pair(ticks: u32, client_autopilot: bool) -> (Playground, Context, Playground, Context) {
        let mut server = Playground::new(Mode::Server { port: 0 }, false);
        let mut server_ctx = Context::headless();
        server.init(&mut server_ctx);
        let port = server.server_port().expect("Server hat keinen Port");

        let mut client = Playground::new(Mode::Join { address: format!("127.0.0.1:{port}") }, client_autopilot);
        let mut client_ctx = Context::headless();
        client.init(&mut client_ctx);

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
    fn client_verbindet_sich_und_bekommt_eine_figur() {
        let (server, _, client, _) = run_pair(120, false);
        let id = client.local_player().expect("Client hat keine Spieler-ID bekommen");
        assert!(server.world().players.contains_key(&id), "Server kennt den Spieler nicht");
        assert!(client.world().players.contains_key(&id), "Client hat keine eigene Figur");
    }

    #[test]
    fn bewegung_kommt_beim_server_an_und_vorhersage_stimmt() {
        // 2 s geradeaus inkl. Sprung, noch bevor die Figur die Kisten erreicht.
        let (server, server_ctx, client, client_ctx) = run_pair(120, true);
        let id = client.local_player().unwrap();
        let on_server = server.world().player_position(&server_ctx, id).unwrap();
        let on_client = client.world().player_position(&client_ctx, id).unwrap();

        assert!(on_server.z < SPAWN_POINT.z - 5.0, "Spieler hat sich auf dem Server nicht bewegt: {on_server}");
        // Der Client ist dem Server um die Netzwerk-Laufzeit voraus.
        assert!(on_server.distance(on_client) < 1.5, "Server {on_server} und Client {on_client} liegen zu weit auseinander");
        assert_eq!(client.corrections(), 0, "Vorhersage weicht vom Server ab");
    }

    #[test]
    fn kisten_bewegen_sich_auf_dem_client_mit() {
        // Autopilot läuft vom Startpunkt geradeaus in die Kistenpyramide.
        let (server, server_ctx, client, client_ctx) = run_pair(420, true);
        let mut moved = 0;
        for (id, object) in &server.world().objects {
            let (on_server, _) = server_ctx.physics.body_pose(object.body).unwrap();
            let (on_client, _) = client_ctx.physics.body_pose(client.world().objects[id].body).unwrap();
            assert!(on_server.distance(on_client) < 1.0, "Objekt {id}: Server {on_server}, Client {on_client}");
            if on_server.distance(Vec3::new(0.0, 0.0, -3.0)) > 2.5 {
                moved += 1;
            }
        }
        assert!(moved > 0, "Keine Kiste wurde umgestoßen");
    }
}
