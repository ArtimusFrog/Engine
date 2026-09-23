//! Die Seite, die das Sagen hat: rechnet die echte Physik und verteilt den Zustand.
//! Läuft beim Host, auf dem dedizierten Server und im Einzelspieler (dann ohne Netzwerk).

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use engine::prelude::*;

use crate::protocol::*;
use crate::world::{World, FIRST_RUNTIME_ID, SPAWN_POINT, THROW_COOLDOWN_TICKS, THROW_SPEED};

/// Alle wie viele Takte ein Snapshot rausgeht (2 = 30 pro Sekunde).
const SNAPSHOT_INTERVAL: u64 = 2;
/// Alle wie viele Takte auch ruhende Objekte mitgeschickt werden.
const FULL_SNAPSHOT_INTERVAL: u64 = 60;
/// Mehr gepufferte Eingaben pro Spieler erhöhen nur die Verzögerung.
const MAX_QUEUED_INPUTS: usize = 12;
/// Wie viele verspätete Eingaben ein Client auf einmal nachholen darf.
const MAX_INPUT_CREDIT: u32 = 8;
/// Geworfene Bälle; die ältesten verschwinden.
const MAX_THROWN: usize = 30;

struct RemoteClient {
    inputs: VecDeque<PlayerInput>,
    last_received: u32,
    last_processed: u32,
    /// Wie viele Eingaben der Client gerade verarbeiten lassen darf (+1 pro Takt).
    credit: u32,
}

pub struct Authority {
    net: Option<NetServer>,
    clients: HashMap<ClientId, RemoteClient>,
    next_object_id: NetId,
    thrown: VecDeque<NetId>,
    send_full_snapshot: bool,
}

impl Authority {
    pub fn new(net: Option<NetServer>) -> Self {
        Authority { net, clients: HashMap::new(), next_object_id: FIRST_RUNTIME_ID, thrown: VecDeque::new(), send_full_snapshot: true }
    }

    pub fn port(&self) -> Option<u16> {
        self.net.as_ref().map(NetServer::port)
    }

    pub fn player_count(&self) -> usize {
        self.clients.len()
    }

    /// Ein Takt. `local_input` ist die Eingabe des Hosts (fehlt beim dedizierten Server).
    pub fn tick(&mut self, ctx: &mut Context, world: &mut World, local_input: Option<PlayerInput>) {
        if self.net.is_some() {
            if ctx.time.tick % SNAPSHOT_INTERVAL == 0 {
                self.send_snapshot(ctx, world);
            }
            self.receive(ctx, world);
        }

        let ids: Vec<ClientId> = self.clients.keys().copied().collect();
        for id in ids {
            // Jede Eingabe wird genau einmal abgespielt – dann rechnet der Server exakt
            // dasselbe wie die Vorhersage des Clients. Kommen Pakete gebündelt an, holt er
            // mehrere pro Takt nach; das Guthaben verhindert, dass ein manipulierter Client
            // durch zusätzliche Eingaben schneller läuft.
            let client = self.clients.get_mut(&id).expect("Client fehlt");
            client.credit = (client.credit + 1).min(MAX_INPUT_CREDIT);
            let mut batch = Vec::new();
            while client.credit > 0 {
                let Some(input) = client.inputs.pop_front() else { break };
                client.credit -= 1;
                client.last_processed = input.seq;
                batch.push(input);
            }
            for input in batch {
                self.play_input(ctx, world, id, &input);
            }
        }
        if let Some(input) = local_input {
            self.play_input(ctx, world, HOST_PLAYER, &input);
        }

        // Wer vom Rand fällt, fängt am Startpunkt neu an.
        for avatar in world.players.values() {
            if ctx.physics.character_position(avatar.character).y < -30.0 {
                ctx.physics.teleport_character(avatar.character, SPAWN_POINT);
            }
        }

        if let Some(net) = &mut self.net {
            net.flush();
        }
    }

    fn play_input(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, input: &PlayerInput) {
        world.apply_input(ctx, player, input);
        if let Some(aim) = input.throw {
            self.throw_ball(ctx, world, player, aim);
        }
    }

    fn throw_ball(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, aim: Vec3) {
        let Some(avatar) = world.players.get_mut(&player) else { return };
        if ctx.time.tick < avatar.last_throw_tick + THROW_COOLDOWN_TICKS {
            return;
        }
        avatar.last_throw_tick = ctx.time.tick;
        let aim = aim.normalize_or(Vec3::NEG_Z);
        let position = ctx.physics.character_position(avatar.character) + Vec3::Y * 0.5 + aim * 0.9;
        let velocity = aim * THROW_SPEED + Vec3::Y * 2.0;

        let id = self.next_object_id;
        self.next_object_id += 1;
        world.spawn_object(ctx, id, ObjectKind::Ball, position, velocity);
        self.broadcast(ServerMessage::Spawn { id, kind: ObjectKind::Ball, position, velocity });

        self.thrown.push_back(id);
        if self.thrown.len() > MAX_THROWN {
            let old = self.thrown.pop_front().expect("Liste ist nicht leer");
            world.remove_object(ctx, old);
            self.broadcast(ServerMessage::Despawn { id: old });
        }
    }

    fn receive(&mut self, ctx: &mut Context, world: &mut World) {
        let Some(net) = &mut self.net else { return };
        let dt = Duration::from_secs_f32(Physics::FIXED_DT);

        for event in net.receive(dt) {
            match event {
                ServerEvent::Connected(id) => {
                    let spawn = SPAWN_POINT + vec3((id % 5) as f32 - 2.0, 0.0, 0.0);
                    let name = clean_name(&String::from_utf8_lossy(&net.hello(id)));
                    world.spawn_player(ctx, id, &name, spawn);
                    // Neuer Spieler: begrüßen und über alles informieren, was schon da ist.
                    let mut intro = vec![ServerMessage::Welcome { player_id: id, tick: ctx.time.tick as u32 }];
                    intro.extend(
                        world.players.iter().filter(|&(&p, _)| p != id).map(|(&p, a)| ServerMessage::PlayerJoined { player_id: p, name: a.name.clone() }),
                    );
                    for (&object_id, object) in &world.objects {
                        if let (Some(kind), Some((position, _))) = (object.kind, ctx.physics.body_pose(object.body)) {
                            intro.push(ServerMessage::Spawn { id: object_id, kind, position, velocity: Vec3::ZERO });
                        }
                    }
                    for message in intro {
                        net.send(id, Channel::Reliable, encode(&message));
                    }
                    for &other in self.clients.keys() {
                        net.send(other, Channel::Reliable, encode(&ServerMessage::PlayerJoined { player_id: id, name: name.clone() }));
                    }
                    self.clients.insert(
                        id,
                        RemoteClient { inputs: VecDeque::new(), last_received: 0, last_processed: 0, credit: 0 },
                    );
                    self.send_full_snapshot = true;
                }
                ServerEvent::Disconnected(id, reason) => {
                    log::info!("Verbindung zu {id} getrennt: {reason}");
                    self.clients.remove(&id);
                    world.remove_player(ctx, id);
                    net.broadcast(Channel::Reliable, encode(&ServerMessage::PlayerLeft { player_id: id }));
                }
            }
        }

        for (&id, client) in &mut self.clients {
            while let Some(bytes) = net.message(id, Channel::Unreliable) {
                let Some(ClientMessage::Inputs(inputs)) = decode(&bytes) else { continue };
                for input in inputs {
                    if input.seq > client.last_received {
                        client.last_received = input.seq;
                        client.inputs.push_back(input);
                    }
                }
            }
            while client.inputs.len() > MAX_QUEUED_INPUTS {
                client.inputs.pop_front();
            }
        }
    }

    fn send_snapshot(&mut self, ctx: &Context, world: &World) {
        let full = self.send_full_snapshot || ctx.time.tick % FULL_SNAPSHOT_INTERVAL == 0;
        self.send_full_snapshot = false;

        let players = world
            .players
            .iter()
            .map(|(&id, avatar)| {
                let state = ctx.physics.character(avatar.character);
                PlayerState {
                    id,
                    position: ctx.physics.character_position(avatar.character),
                    velocity: state.velocity,
                    grounded: state.grounded,
                    facing: avatar.facing,
                    last_input: self.clients.get(&id).map_or(0, |c| c.last_processed),
                }
            })
            .collect();
        let objects = world
            .objects
            .iter()
            .filter(|(_, o)| full || !ctx.physics.is_sleeping(o.body))
            .filter_map(|(&id, o)| {
                let (position, rotation) = ctx.physics.body_pose(o.body)?;
                Some(ObjectState { id, position, rotation })
            })
            .collect();

        let snapshot = ServerMessage::Snapshot(Snapshot { tick: ctx.time.tick as u32, players, objects });
        if let Some(net) = &mut self.net {
            net.broadcast(Channel::Unreliable, encode(&snapshot));
        }
    }

    fn broadcast(&mut self, message: ServerMessage) {
        if let Some(net) = &mut self.net {
            net.broadcast(Channel::Reliable, encode(&message));
        }
    }
}
