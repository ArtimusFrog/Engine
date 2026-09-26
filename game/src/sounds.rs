//! Alle Geräusche des Spiels und die Klangkulisse (Wind, Wellen, Vögel, Musik).
//!
//! Jeder Klang kann durch eine Aufnahme ersetzt werden: `game/assets/sounds/<name>.ogg`
//! (oder .wav/.flac). Solange es keine gibt, erzeugt das Spiel ihn selbst (Synthese).

use engine::prelude::*;
use engine::audio::synth::{envelope, normalize, make_loopable, render, sine, LowPass, Noise};

use crate::asset_files;
use crate::island::ResourceKind;
use crate::world::{SoundEvent, World};

pub struct Sounds {
    chop: SoundId,
    stone: SoundId,
    ore: SoundId,
    tree_falls: SoundId,
    rock_breaks: SoundId,
    crackle: SoundId,
    thunder: SoundId,
    rain: LoopId,
    /// Musik für die Nacht und den Zauberwald (die Tagesmusik ist `music`)
    music_night: LoopId,
    music_magic: LoopId,
    /// Aktuelle Anteile der drei Musikstücke (weich überblendet)
    music_mix: [f32; 3],
    gull: SoundId,
    howl: SoundId,
    /// Musik im Hauptmenü (game/assets/sounds/menue.ogg), wird im Hintergrund geladen
    menu_music: Option<LoopId>,
    menu_loading: Option<std::sync::mpsc::Receiver<Result<DecodedSound, String>>>,
    /// Wie laut die Menümusik gerade ist (weiches Ein- und Ausblenden)
    menu_volume: f32,
    /// Zeit bis zum nächsten Möwenruf bzw. Wolfsgeheul
    next_gull: f32,
    next_howl: f32,
    cast: SoundId,
    impact: SoundId,
    pickup: SoundId,
    wind: LoopId,
    music: LoopId,
    rng: Rng,
    last_items: Option<u32>,
}

/// Startet das Laden der Menümusik in einem eigenen Thread (sie ist ein ganzes Musikstück).
fn start_menu_music() -> Option<std::sync::mpsc::Receiver<Result<DecodedSound, String>>> {
    let path = asset_files::asset_dir().map(|dir| dir.join("sounds").join("menue.ogg")).filter(|p| p.is_file())?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(Audio::decode_file(&path));
    });
    Some(receiver)
}

/// Aufnahme aus `game/assets/sounds/`, sonst der selbst erzeugte Klang.
fn sound(audio: &mut Audio, name: &str, build: impl FnOnce() -> SoundBuffer) -> SoundId {
    let file = asset_files::asset_dir().and_then(|dir| {
        ["ogg", "wav", "flac"].iter().map(|ext| dir.join("sounds").join(format!("{name}.{ext}"))).find(|p| p.is_file())
    });
    if let Some(path) = file {
        match audio.load_file(name, &path) {
            Ok(id) => return id,
            Err(message) => log::warn!("{message} – nehme den erzeugten Klang"),
        }
    }
    audio.add_sound(name, || {
        let mut buffer = build();
        normalize(&mut buffer, 0.9);
        buffer
    })
}

impl Sounds {
    pub fn new(ctx: &mut Context) -> Sounds {
        let a = &mut ctx.audio;
        let wind = sound(a, "wind", wind);
        let music = sound(a, "musik", music);
        Sounds {
            chop: sound(a, "hacken", chop),
            stone: sound(a, "stein", stone),
            ore: sound(a, "erz", ore),
            tree_falls: sound(a, "baum_faellt", tree_falls),
            rock_breaks: sound(a, "fels_bricht", rock_breaks),
            crackle: sound(a, "knistern", crackle),
            thunder: sound(a, "donner", thunder),
            music_night: {
                let s = sound(a, "musik_nacht", music_night);
                a.start_loop(s, Bus::Music)
            },
            music_magic: {
                let s = sound(a, "musik_zauberwald", music_magic);
                a.start_loop(s, Bus::Music)
            },
            music_mix: [1.0, 0.0, 0.0],
            menu_music: None,
            menu_loading: start_menu_music(),
            menu_volume: 0.0,
            gull: sound(a, "moewe", gull),
            howl: sound(a, "wolfsgeheul", howl),
            next_gull: 4.0,
            next_howl: 20.0,
            rain: {
                let rain = sound(a, "regen", rain);
                a.start_loop(rain, Bus::Ambient)
            },
            cast: sound(a, "zauber", cast),
            impact: sound(a, "treffer", impact),
            pickup: sound(a, "einsammeln", pickup),
            wind: a.start_loop(wind, Bus::Ambient),
            music: a.start_loop(music, Bus::Music),
            rng: Rng::new(99),
            last_items: None,
        }
    }

    /// Menümusik übernehmen, sobald sie im Hintergrund fertig geladen ist, und ihre Lautstärke
    /// weich zum Ziel führen.
    fn menu_music(&mut self, ctx: &mut Context, target: f32) {
        if let Some(receiver) = &self.menu_loading {
            match receiver.try_recv() {
                Ok(Ok(decoded)) => {
                    let id = ctx.audio.add_decoded("menue", decoded);
                    self.menu_music = Some(ctx.audio.start_loop(id, Bus::Music));
                    self.menu_loading = None;
                }
                Ok(Err(message)) => {
                    log::warn!("Menümusik: {message}");
                    self.menu_loading = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.menu_loading = None,
            }
        }
        let speed = if target > self.menu_volume { 0.35 } else { 0.8 };
        self.menu_volume += (target - self.menu_volume).clamp(-speed * ctx.time.delta, speed * ctx.time.delta);
        if let Some(id) = self.menu_music {
            ctx.audio.set_loop(id, self.menu_volume, None, 1.0);
        }
    }

    /// Läuft die eigene Menümusik (dann schweigt die erzeugte Musik im Menü)?
    fn has_menu_music(&self) -> bool {
        self.menu_music.is_some() || self.menu_loading.is_some()
    }

    /// Einmal pro Bild: Ereignisse der Welt abspielen und die Kulisse anpassen.
    /// `items` = Anzahl der Rohstoffe im eigenen Inventar (für das Einsammel-Geräusch).
    pub fn update(&mut self, ctx: &mut Context, world: &mut World, items: Option<u32>) {
        for event in std::mem::take(&mut world.sound_events) {
            let pitch = self.rng.range(0.9, 1.1);
            match event {
                SoundEvent::Hit { kind, at, finished } => {
                    let (hit, done) = match kind {
                        ResourceKind::Wood => (self.chop, self.tree_falls),
                        ResourceKind::Stone => (self.stone, self.rock_breaks),
                        ResourceKind::Ore => (self.ore, self.rock_breaks),
                    };
                    ctx.audio.play(hit, Play { at: Some(at + Vec3::Y), pitch, range: 45.0, ..Default::default() });
                    if finished {
                        ctx.audio.play(done, Play { at: Some(at + Vec3::Y), volume: 0.9, range: 70.0, ..Default::default() });
                    }
                }
                SoundEvent::Thunder { volume } => {
                    ctx.audio.play(self.thunder, Play { volume: volume.clamp(0.2, 1.0), pitch: self.rng.range(0.85, 1.1), ..Default::default() });
                }
                SoundEvent::Built { at, done } => {
                    if done {
                        ctx.audio.play(self.pickup, Play { at: Some(at + Vec3::Y * 2.0), volume: 0.9, pitch: 0.7, range: 60.0, ..Default::default() });
                        ctx.audio.play(self.rock_breaks, Play { at: Some(at), volume: 0.5, pitch: 1.2, range: 60.0, ..Default::default() });
                    } else {
                        let pitch = self.rng.range(0.72, 0.95);
                        ctx.audio.play(self.stone, Play { at: Some(at), volume: 0.45, pitch, range: 40.0, ..Default::default() });
                    }
                }
                SoundEvent::Crackle { at } => {
                    let volume = self.rng.range(0.25, 0.5);
                    ctx.audio.play(self.crackle, Play { at: Some(at), volume, pitch: self.rng.range(0.8, 1.3), range: 22.0, ..Default::default() });
                }
                SoundEvent::Cast { player } => {
                    let at = world.player_position(ctx, player);
                    ctx.audio.play(self.cast, Play { at, volume: 0.6, pitch, range: 35.0, ..Default::default() });
                }
                SoundEvent::Impact { at, animal, killed } => {
                    // In ein Tier: kräftig; erlegt: tiefer; sonst leises Verpuffen.
                    let (volume, pitch) = match (animal, killed) {
                        (true, true) => (0.9, pitch * 0.75),
                        (true, false) => (0.7, pitch),
                        (false, _) => (0.35, pitch * 1.25),
                    };
                    ctx.audio.play(self.impact, Play { at: Some(at), volume, pitch, range: 40.0, ..Default::default() });
                }
            }
        }

        // Mehr im Inventar als vorher: kurzes, helles „Pling“.
        if let (Some(before), Some(now)) = (self.last_items, items) {
            if now > before {
                ctx.audio.play(self.pickup, Play { volume: 0.35, pitch: self.rng.range(0.97, 1.05), ..Default::default() });
            }
        }
        self.last_items = items;

        self.menu_music(ctx, 0.0);
        self.ambience(ctx, world);
    }

    /// Wind je nach Höhe, Regen, Möwen, Wölfe, Musik (kein Meeresrauschen mehr).
    fn ambience(&mut self, ctx: &mut Context, world: &World) {
        let camera = ctx.camera.position;
        let terrain = &world.terrain;
        let daylight = (world.day.sun_direction().y * 3.0 + 0.3).clamp(0.0, 1.0);

        // Meer in der Nähe (für die Möwen): im Inland liegt das Gelände nie unter dem Meeresspiegel
        let mut shore_at: Option<Vec3> = None;
        'suche: for ring in [10.0, 25.0, 45.0] {
            for i in 0..12 {
                let p = vec2(camera.x, camera.z) + Vec2::from_angle(i as f32 / 12.0 * std::f32::consts::TAU) * ring;
                if terrain.height_at(p.x, p.y) < 0.0 {
                    shore_at = Some(vec3(p.x, 0.0, p.y));
                    break 'suche;
                }
            }
        }

        // Wind: am Boden kaum hörbar, erst hoch oben (Gebirge, Türme) deutlicher
        let ueber_boden = (camera.y - terrain.height_at(camera.x, camera.z).max(0.0)).max(0.0);
        let wind = 0.05 + ((ueber_boden - 4.0) / 60.0).clamp(0.0, 0.15) + ((camera.y - 28.0) / 80.0).clamp(0.0, 0.25);
        ctx.audio.set_loop(self.wind, wind, None, 1.0);
        let rain = world.weather.state().rain;
        ctx.audio.set_loop(self.rain, rain * 0.7, None, 1.0);

        // Möwen rufen tagsüber an der Küste
        let dt = ctx.time.delta;
        self.next_gull -= dt;
        if self.next_gull <= 0.0 {
            self.next_gull = self.rng.range(4.0, 11.0);
            if let (Some(at), true) = (shore_at, daylight > 0.3 && rain < 0.5) {
                let pos = at + vec3(self.rng.range(-15.0, 15.0), self.rng.range(8.0, 14.0), self.rng.range(-15.0, 15.0));
                ctx.audio.play(self.gull, Play { at: Some(pos), volume: 0.45, pitch: self.rng.range(0.9, 1.15), range: 90.0, ..Default::default() });
            }
        }
        // Wölfe heulen nachts aus den Bergen (vom nächsten Wolf aus)
        self.next_howl -= dt;
        if self.next_howl <= 0.0 {
            self.next_howl = self.rng.range(25.0, 60.0);
            if daylight < 0.2 {
                let wolf = world
                    .animals
                    .iter()
                    .filter(|a| a.kind == crate::animals::AnimalKind::Wolf && a.is_alive())
                    .map(|a| a.position)
                    .min_by(|a, b| a.distance(camera).total_cmp(&b.distance(camera)));
                if let Some(at) = wolf.filter(|w| w.distance(camera) < 180.0) {
                    ctx.audio.play(self.howl, Play { at: Some(at + Vec3::Y), volume: 0.8, pitch: self.rng.range(0.9, 1.1), range: 200.0, ..Default::default() });
                }
            }
        }

        // Musik: Tag, Nacht, Zauberwald – weich überblenden
        let magic = crate::island::is_enchanted(vec2(camera.x, camera.z)) as u8 as f32;
        let target = if magic > 0.5 { [0.0, 0.0, 1.0] } else if daylight < 0.3 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        for (mix, goal) in self.music_mix.iter_mut().zip(target) {
            *mix += (goal - *mix) * (dt * 0.25).min(1.0);
        }
        ctx.audio.set_loop(self.music, 0.35 * self.music_mix[0], None, 1.0);
        ctx.audio.set_loop(self.music_night, 0.35 * self.music_mix[1], None, 1.0);
        ctx.audio.set_loop(self.music_magic, 0.35 * self.music_mix[2], None, 1.0);
    }

    /// Im Menü: die Menümusik und etwas Wind (ohne eigene Menümusik die Tagesmusik).
    pub fn menu(&mut self, ctx: &mut Context) {
        self.menu_music(ctx, 0.75);
        ctx.audio.set_loop(self.wind, 0.15, None, 1.0);
        for quiet in [self.rain, self.music_night, self.music_magic] {
            ctx.audio.set_loop(quiet, 0.0, None, 1.0);
        }
        ctx.audio.set_loop(self.music, if self.has_menu_music() { 0.0 } else { 0.5 }, None, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Klänge (Synthese). Die Zahlen sind Schätzwerte – nach Gehör nachstellen.
// ---------------------------------------------------------------------------

/// Axt in Holz: dumpfer Schlag mit kurzem Splittern.
fn chop() -> SoundBuffer {
    let mut noise = Noise::new(1);
    let mut lp = LowPass::default();
    render(0.35, |t| {
        let thump = sine(t, 95.0 - t * 120.0) * envelope(t, 0.002, 0.07) * 0.9;
        let crack = lp.next(noise.next(), 0.35) * envelope(t, 0.001, 0.035) * 1.2;
        let wood = sine(t, 420.0) * envelope(t, 0.001, 0.05) * 0.25;
        thump + crack + wood
    })
}

/// Hacke auf Stein: heller Klick mit metallischem Nachklang.
fn stone() -> SoundBuffer {
    let mut noise = Noise::new(2);
    let mut lp = LowPass::default();
    render(0.5, |t| {
        let n = noise.next();
        let click = (n - lp.next(n, 0.2)) * envelope(t, 0.0005, 0.02) * 1.1;
        let ring = (sine(t, 1870.0) * 0.5 + sine(t, 2710.0) * 0.3 + sine(t, 3920.0) * 0.15) * envelope(t, 0.001, 0.12) * 0.45;
        let body = sine(t, 180.0) * envelope(t, 0.001, 0.05) * 0.4;
        click + ring + body
    })
}

/// Spitzhacke auf Erz: harter Schlag mit hellem, metallischem Klingen.
fn ore() -> SoundBuffer {
    let mut noise = Noise::new(13);
    let mut lp = LowPass::default();
    render(0.8, |t| {
        let n = noise.next();
        let click = (n - lp.next(n, 0.15)) * envelope(t, 0.0005, 0.018) * 1.2;
        let ring = (sine(t, 2350.0) * 0.45 + sine(t, 3180.0) * 0.3 + sine(t, 4410.0) * 0.2 + sine(t, 1210.0) * 0.25) * envelope(t, 0.001, 0.22) * 0.5;
        let body = sine(t, 150.0) * envelope(t, 0.001, 0.05) * 0.45;
        click + ring + body
    })
}

/// Baum fällt: Knarzen, Rauschen der Krone, dumpfer Aufprall.
fn tree_falls() -> SoundBuffer {
    let mut noise = Noise::new(3);
    let mut leaves = LowPass::default();
    let mut ground = LowPass::default();
    render(2.2, |t| {
        let creak = if t < 0.9 { sine(t, 70.0 + (t * 37.0).sin() * 15.0) * (t * 23.0).sin().abs() * 0.35 * (t / 0.9) } else { 0.0 };
        let swoosh = leaves.next(noise.next(), 0.25) * (((t - 0.5) / 0.9).clamp(0.0, 1.0) * std::f32::consts::PI).sin() * 0.5;
        let hit = t - 1.35;
        let impact = if hit > 0.0 { (sine(hit, 55.0) * 0.9 + ground.next(noise.next(), 0.08) * 1.5) * envelope(hit, 0.005, 0.25) } else { 0.0 };
        creak + swoosh + impact
    })
}

/// Fels zerbricht: Rumpeln und Geröll.
fn rock_breaks() -> SoundBuffer {
    let mut noise = Noise::new(4);
    let mut lp = LowPass::default();
    let mut pebbles = Noise::new(5);
    render(1.2, |t| {
        let rumble = lp.next(noise.next(), 0.05) * envelope(t, 0.01, 0.3) * 2.0;
        let grit = if pebbles.next() > 0.93 { pebbles.next() * envelope(t, 0.05, 0.4) * 0.6 } else { 0.0 };
        rumble + grit + sine(t, 60.0) * envelope(t, 0.005, 0.15) * 0.6
    })
}

/// Möwe: zwei, drei schrille, abfallende Rufe.
fn gull() -> SoundBuffer {
    let mut noise = Noise::new(42);
    render(1.1, |t| {
        let mut s = 0.0;
        for k in 0..3 {
            let local = t - k as f32 * 0.3;
            if (0.0..0.26).contains(&local) {
                let pitch = 1450.0 - local * 1800.0 + (local * 40.0).sin() * 60.0;
                let tone = sine(local, pitch) * 0.6 + sine(local, pitch * 2.0) * 0.25 + noise.next() * 0.08;
                s += tone * envelope(local, 0.02, 0.12) * (1.0 - k as f32 * 0.2);
            }
        }
        s
    })
}

/// Wolfsgeheul: langer, steigender und dann fallender Ton mit leichtem Zittern.
fn howl() -> SoundBuffer {
    render(3.6, |t| {
        let shape = if t < 0.8 { t / 0.8 } else { 1.0 - ((t - 0.8) / 2.8).powf(1.5) * 0.4 };
        let pitch = 330.0 + 180.0 * shape + (t * 5.5).sin() * 6.0;
        let tone = sine(t, pitch) * 0.7 + sine(t, pitch * 2.0) * 0.18 + sine(t, pitch * 3.0) * 0.06;
        let swell = (t / 0.4).min(1.0) * (1.0 - (t - 2.9).max(0.0) / 0.7).max(0.0);
        tone * swell * 0.6
    })
}

/// Musik in der Nacht: ruhige Mollakkorde (Dm – Am – B – C), wenige, weiche Töne.
fn music_night() -> SoundBuffer {
    const CHORDS: [[f32; 3]; 4] = [[146.83, 174.61, 220.00], [110.00, 130.81, 164.81], [116.54, 146.83, 174.61], [130.81, 164.81, 196.00]];
    const SCALE: [f32; 5] = [587.33, 698.46, 783.99, 880.00, 1046.50];
    pad_music(CHORDS, SCALE, 8.0, 0.45, 12)
}

/// Musik im Zauberwald: schwebende Akkorde mit Glöckchen (E-Lydisch).
fn music_magic() -> SoundBuffer {
    const CHORDS: [[f32; 3]; 4] = [[164.81, 207.65, 246.94], [185.00, 233.08, 277.18], [164.81, 207.65, 246.94], [138.59, 164.81, 207.65]];
    const SCALE: [f32; 5] = [1318.51, 1479.98, 1661.22, 1864.66, 1975.53];
    pad_music(CHORDS, SCALE, 7.0, 0.7, 13)
}

/// Gemeinsamer Aufbau: weiche Flächenakkorde und darüber einzelne Töne aus einer Tonleiter.
fn pad_music(chords: [[f32; 3]; 4], scale: [f32; 5], chord_len: f32, density: f32, seed: u64) -> SoundBuffer {
    let total = chord_len * chords.len() as f32;
    let mut rng = Rng::new(seed);
    let mut notes: Vec<(f32, f32)> = Vec::new();
    let mut beat = 0.0;
    while beat < total - 2.0 {
        if rng.chance(density) {
            notes.push((beat + rng.range(0.0, 0.3), scale[(rng.next_u32() % 5) as usize]));
        }
        beat += 1.25;
    }
    let mut buffer = render(total, |t| {
        let index = ((t / chord_len) as usize).min(chords.len() - 1);
        let local = t - index as f32 * chord_len;
        let swell = (local / 2.0).min(1.0) * ((chord_len - local) / 2.0).min(1.0);
        let mut pad = 0.0;
        for &f in &chords[index] {
            pad += sine(t, f) * 0.6 + sine(t, f * 1.004) * 0.35 + sine(t, f * 2.0) * 0.08;
        }
        let mut melody = 0.0;
        for &(start, f) in &notes {
            let local = t - start;
            if (0.0..3.0).contains(&local) {
                melody += (sine(local, f) + sine(local, f * 3.0) * 0.08) * envelope(local, 0.01, 0.8);
            }
        }
        pad * swell * 0.07 + melody * 0.09
    });
    make_loopable(&mut buffer, 1.5);
    buffer
}

/// Donner: tiefes, rollendes Grollen mit einem Krachen am Anfang.
fn thunder() -> SoundBuffer {
    let mut noise = Noise::new(31);
    let mut deep = LowPass::default();
    let mut deeper = LowPass::default();
    render(4.5, |t| {
        let n = noise.next();
        let crack = n * envelope(t, 0.005, 0.15) * 0.6;
        let rumble = deeper.next(deep.next(n, 0.05), 0.08) * 6.0;
        let roll = 0.55 + 0.45 * (t * 2.3).sin() * (t * 0.9 + 1.0).sin();
        crack + rumble * envelope(t, 0.12, 1.6) * roll
    })
}

/// Regen: gleichmäßiges, helles Rauschen mit einzelnen Tropfen (Schleife).
fn rain() -> SoundBuffer {
    let mut noise = Noise::new(32);
    let mut lp = LowPass::default();
    let mut drops = Noise::new(33);
    let mut buffer = render(6.0, |_| {
        let n = noise.next();
        let hiss = (n - lp.next(n, 0.3)) * 0.35 + lp.next(n, 0.3) * 0.15;
        let drop = if drops.next() > 0.9985 { drops.next() * 0.8 } else { 0.0 };
        hiss + drop
    });
    make_loopable(&mut buffer, 1.0);
    buffer
}

/// Lagerfeuer: ein paar kurze, trockene Knackser mit leisem Rauschen dazwischen.
fn crackle() -> SoundBuffer {
    let mut noise = Noise::new(21);
    let mut lp = LowPass::default();
    let mut pops = Noise::new(22);
    let knacks: Vec<(f32, f32)> = (0..5).map(|i| (0.04 + i as f32 * 0.09 + pops.next().abs() * 0.05, 0.4 + pops.next().abs() * 0.6)).collect();
    render(0.55, |t| {
        let n = noise.next();
        let rauschen = lp.next(n, 0.08) * 0.12 * envelope(t, 0.05, 0.4);
        let knack: f32 = knacks.iter().map(|&(at, laut)| if t >= at { (n - lp.next(n, 0.5) * 0.0) * envelope(t - at, 0.0005, 0.008) * laut } else { 0.0 }).sum();
        rauschen + knack
    })
}

/// Zauber: schimmernder, aufsteigender Klang mit einem Luftzug, wenn das Geschoss losfliegt.
fn cast() -> SoundBuffer {
    let mut noise = Noise::new(6);
    let mut lp = LowPass::default();
    render(0.75, |t| {
        let rise = 320.0 + t * 520.0;
        let tone = sine(t, rise) * 0.5 + sine(t, rise * 1.5) * 0.3 + sine(t, rise * 2.02) * 0.2;
        let shimmer = tone * (1.0 + 0.35 * sine(t, 23.0)) * envelope(t, 0.18, 0.28) * 0.55;
        let launch = t - 0.2;
        let whoosh = if launch > 0.0 { lp.next(noise.next(), 0.08 + (launch * 3.0).min(0.4)) * envelope(launch, 0.03, 0.18) * 1.1 } else { 0.0 };
        shimmer + whoosh
    })
}

/// Treffer eines Zaubers: heller Knall mit funkelndem Nachklang.
fn impact() -> SoundBuffer {
    let mut noise = Noise::new(12);
    let mut lp = LowPass::default();
    render(0.6, |t| {
        let n = noise.next();
        let pop = lp.next(n, 0.25) * envelope(t, 0.001, 0.05) * 1.3;
        let thump = sine(t, 140.0 - t * 90.0) * envelope(t, 0.002, 0.08) * 0.8;
        let sparkle = (sine(t, 1760.0) * 0.4 + sine(t, 2637.0) * 0.3 + sine(t, 3520.0) * 0.2) * (1.0 + sine(t, 31.0)) * 0.5 * envelope(t, 0.005, 0.16) * 0.5;
        pop + thump + sparkle
    })
}

/// Einsammeln: zwei helle Töne.
fn pickup() -> SoundBuffer {
    render(0.3, |t| {
        let first = sine(t, 880.0) * envelope(t, 0.003, 0.06);
        let second = if t > 0.07 { sine(t, 1318.5) * envelope(t - 0.07, 0.003, 0.09) } else { 0.0 };
        (first + second) * 0.4
    })
}

/// Wind: tiefes Rauschen, das langsam an- und abschwillt (Schleife).
fn wind() -> SoundBuffer {
    let mut noise = Noise::new(7);
    let mut lp = LowPass::default();
    let mut buffer = render(12.0, |t| {
        let gust = 0.55 + 0.3 * (t * 0.52).sin() + 0.15 * (t * 1.31).sin();
        lp.next(noise.next(), 0.015 + gust * 0.02) * gust * 3.0
    });
    make_loopable(&mut buffer, 1.5);
    buffer
}

/// Ruhige Musik: weiche Akkorde (Flächen) und eine gezupfte Pentatonik-Melodie.
fn music() -> SoundBuffer {
    // Akkordfolge in D-Dur: D – Hm – G – A, je 6 Sekunden.
    const CHORDS: [[f32; 3]; 4] = [[146.83, 185.00, 220.00], [123.47, 146.83, 185.00], [98.00, 123.47, 146.83], [110.00, 138.59, 164.81]];
    const SCALE: [f32; 5] = [587.33, 659.25, 739.99, 880.00, 987.77];
    let chord_len = 6.0;
    let total = chord_len * CHORDS.len() as f32;
    let mut rng = Rng::new(11);
    // Auf etwa drei von vier Schlägen ein Ton aus der Pentatonik.
    let mut notes: Vec<(f32, f32)> = Vec::new();
    for beat in 0..24 {
        if rng.chance(0.75) {
            notes.push((beat as f32 + rng.range(0.0, 0.2), SCALE[(rng.next_u32() % 5) as usize]));
        }
    }
    let mut buffer = render(total, |t| {
        let index = ((t / chord_len) as usize).min(CHORDS.len() - 1);
        let local = t - index as f32 * chord_len;
        // Weich ein- und ausblenden, damit die Akkorde ineinander fließen.
        let swell = (local / 1.5).min(1.0) * ((chord_len - local) / 1.5).min(1.0);
        let mut pad = 0.0;
        for &f in &CHORDS[index] {
            pad += sine(t, f) * 0.6 + sine(t, f * 2.0) * 0.15 + sine(t, f * 1.003) * 0.3;
        }
        let mut melody = 0.0;
        for &(start, f) in &notes {
            let local = t - start;
            if (0.0..2.0).contains(&local) {
                melody += (sine(local, f) + sine(local, f * 2.0) * 0.2) * envelope(local, 0.005, 0.45);
            }
        }
        pad * swell * 0.07 + melody * 0.12
    });
    make_loopable(&mut buffer, 1.0);
    buffer
}

/// Schreibt alle erzeugten Klänge als WAV-Dateien (zum Anhören und Vergleichen):
/// `game --klaenge-exportieren <ordner>`.
pub fn export_all(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let all: [(&str, fn() -> SoundBuffer); 17] = [
        ("hacken", chop),
        ("stein", stone),
        ("erz", ore),
        ("knistern", crackle),
        ("donner", thunder),
        ("regen", rain),
        ("moewe", gull),
        ("wolfsgeheul", howl),
        ("musik_nacht", music_night),
        ("musik_zauberwald", music_magic),
        ("baum_faellt", tree_falls),
        ("fels_bricht", rock_breaks),
        ("zauber", cast),
        ("treffer", impact),
        ("einsammeln", pickup),
        ("wind", wind),
        ("musik", music),
    ];
    std::fs::create_dir_all(dir)?;
    let mut written = Vec::new();
    for (name, build) in all {
        let mut buffer = build();
        normalize(&mut buffer, 0.9);
        let path = dir.join(format!("{name}.wav"));
        std::fs::write(&path, wav_mono_16bit(&buffer))?;
        written.push(path);
    }
    Ok(written)
}

/// Minimale WAV-Datei: Mono, 16 Bit.
fn wav_mono_16bit(buffer: &SoundBuffer) -> Vec<u8> {
    let samples: Vec<i16> = buffer.frames.iter().map(|f| (f[0].clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect();
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // Mono
    out.extend_from_slice(&buffer.sample_rate.to_le_bytes());
    out.extend_from_slice(&(buffer.sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn klaenge_sind_hoerbar_und_uebersteuern_nicht() {
        let all: [(&str, SoundBuffer); 6] = [
            ("hacken", chop()),
            ("stein", stone()),
            ("baum_faellt", tree_falls()),
            ("fels_bricht", rock_breaks()),
            ("wind", wind()),
            ("musik", music()),
        ];
        for (name, mut buffer) in all {
            assert!(!buffer.frames.is_empty(), "{name} ist leer");
            assert!(buffer.frames.iter().all(|f| f[0].is_finite()), "{name} enthält ungültige Werte");
            normalize(&mut buffer, 0.9);
            let peak = buffer.frames.iter().map(|f| f[0].abs()).fold(0.0, f32::max);
            let rms = (buffer.frames.iter().map(|f| f[0] * f[0]).sum::<f32>() / buffer.frames.len() as f32).sqrt();
            assert!((peak - 0.9).abs() < 0.01, "{name} ist stumm");
            // Sonst ist es nur ein einzelnes Knacken statt eines Klangs.
            assert!(rms > 0.01, "{name} ist fast stumm: {rms}");
        }
        assert_eq!(engine::audio::synth::SAMPLE_RATE, chop().sample_rate);
    }
}

