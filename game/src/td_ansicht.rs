//! Darstellung der Ereignisse der Verteidigung (nur mit Fenster): Bossfähigkeiten, Explosionen,
//! Bodenwellen, Windstöße, Brand-, Gift-, Frost- und Runenfelder am Boden.

use engine::prelude::*;

use crate::td::Ereignis;
use crate::world::SoundEvent;

/// Arten der Felder am Boden (in `World::felder`).
const BRAND: u8 = 0;
const GIFT: u8 = 1;
const FROST: u8 = 2;
/// Waldschrat: der Boden bricht auf (Erdkrumen springen hoch)
const WURZEL: u8 = 3;

fn funken(ctx: &mut Context, at: Vec3, count: u32, color: Vec3, speed: f32, size: f32, life: f32, gravity: f32, glow: f32) {
    ctx.particles.burst(Burst {
        position: at,
        count,
        color,
        color_variation: 0.25,
        speed,
        direction: Vec3::Y * 0.6,
        size,
        life,
        gravity,
        glow,
        grow: 0.0,
        round: true,
    });
}

/// Ring aus Funken am Boden (Bodenwelle, Stampfen, Frostfeld).
fn ring(ctx: &mut Context, mitte: Vec3, radius: f32, color: Vec3, glow: f32, hoch: f32) {
    let n = (radius * 5.0).clamp(16.0, 90.0) as usize;
    for k in 0..n {
        let w = std::f32::consts::TAU * k as f32 / n as f32;
        let p = mitte + vec3(w.cos() * radius, 0.25, w.sin() * radius);
        ctx.particles.burst(Burst {
            position: p,
            count: 1,
            color,
            color_variation: 0.15,
            speed: hoch,
            direction: Vec3::Y,
            size: 0.16,
            life: 0.7,
            gravity: 2.0,
            glow,
            grow: 0.5,
            round: true,
        });
    }
}

/// Ein Ereignis zeigen (einmalig); Felder bleiben eine Weile liegen.
pub fn ereignis(ctx: &mut Context, sounds: &mut Vec<SoundEvent>, felder: &mut Vec<(Vec3, f32, f32, u8)>, ereignis: Ereignis) {
    match ereignis {
        Ereignis::Heilung(at) => {
            funken(ctx, at + Vec3::Y * 1.2, 30, vec3(0.4, 1.0, 0.45), 2.0, 0.12, 1.1, -1.5, 3.0);
            ring(ctx, at, 4.0, vec3(0.4, 1.0, 0.5), 2.0, 1.0);
        }
        Ereignis::Stampfen(at, radius) => {
            for r in [radius * 0.35, radius * 0.7, radius] {
                ring(ctx, at, r, vec3(0.75, 0.55, 1.0), 3.0, 3.0);
            }
            funken(ctx, at + Vec3::Y * 0.3, 60, vec3(0.55, 0.5, 0.5), 7.0, 0.3, 1.2, 9.0, 0.0);
            sounds.push(SoundEvent::Thunder { volume: 0.5 });
        }
        Ereignis::Beschwoerung(at) => {
            ring(ctx, at, 3.0, vec3(0.7, 0.3, 1.0), 5.0, 2.0);
            funken(ctx, at + Vec3::Y * 0.5, 50, vec3(0.6, 0.25, 1.0), 3.0, 0.15, 1.4, -2.0, 5.0);
        }
        Ereignis::Unverwundbar(at) => funken(ctx, at, 60, vec3(0.9, 0.95, 1.0), 3.5, 0.14, 1.2, -1.0, 6.0),
        Ereignis::Wut(at) => {
            funken(ctx, at, 70, vec3(1.0, 0.25, 0.15), 5.0, 0.16, 1.0, 1.0, 5.0);
            sounds.push(SoundEvent::Impact { at, animal: false, killed: true });
        }
        Ereignis::Spaltung(at) => {
            funken(ctx, at, 70, vec3(0.5, 0.47, 0.5), 6.0, 0.28, 1.1, 10.0, 0.0);
            funken(ctx, at, 30, vec3(0.85, 0.5, 1.0), 4.0, 0.1, 0.8, 2.0, 5.0);
            sounds.push(SoundEvent::Impact { at, animal: false, killed: true });
        }
        Ereignis::Explosion(at, radius) => {
            funken(ctx, at, 80, vec3(1.0, 0.55, 0.15), 3.0 + radius * 2.0, 0.22, 0.7, 4.0, 6.0);
            funken(ctx, at, 30, vec3(0.45, 1.0, 0.3), 4.0, 0.18, 0.9, 2.0, 3.0);
            ctx.particles.burst(Burst {
                position: at,
                count: 14,
                color: vec3(0.3, 0.28, 0.26),
                color_variation: 0.1,
                speed: 1.5,
                direction: Vec3::Y,
                size: 0.7,
                life: 1.6,
                gravity: -0.4,
                glow: 0.0,
                grow: 2.0,
                round: true,
            });
            sounds.push(SoundEvent::Impact { at, animal: false, killed: true });
        }
        Ereignis::Puls(at, radius) => {
            for r in [radius * 0.5, radius] {
                ring(ctx, at, r, vec3(0.55, 0.8, 1.0), 4.0, 2.5);
            }
            funken(ctx, at + Vec3::Y * 0.2, 40, vec3(0.5, 0.45, 0.4), 5.0, 0.25, 0.9, 10.0, 0.0);
            sounds.push(SoundEvent::Impact { at, animal: false, killed: false });
        }
        Ereignis::Einfrieren(at) => funken(ctx, at, 40, vec3(0.7, 0.92, 1.0), 2.5, 0.14, 1.0, 3.0, 4.0),
        Ereignis::Windstoss(at) => {
            funken(ctx, at, 40, vec3(0.9, 0.95, 1.0), 6.0, 0.2, 0.6, 0.0, 1.2);
            ctx.particles.burst(Burst {
                position: at,
                count: 10,
                color: vec3(0.8, 0.78, 0.72),
                color_variation: 0.1,
                speed: 4.0,
                direction: Vec3::Y * 0.3,
                size: 0.6,
                life: 0.8,
                gravity: 0.0,
                glow: 0.0,
                grow: 2.5,
                round: true,
            });
        }
        Ereignis::Markiert(at) => funken(ctx, at + Vec3::Y * 1.5, 8, vec3(1.0, 0.3, 0.2), 1.0, 0.12, 0.8, 0.0, 6.0),
        Ereignis::Zerstoert(at) => {
            funken(ctx, at, 60, vec3(0.6, 0.42, 0.25), 6.0, 0.25, 1.4, 12.0, 0.0);
            sounds.push(SoundEvent::Built { at, done: false });
        }
        Ereignis::Brandfeld(at, radius, dauer) => felder.push((at, radius, dauer, BRAND)),
        Ereignis::Giftwolke(at, radius, dauer) => felder.push((at, radius, dauer, GIFT)),
        Ereignis::Felswurf(von, nach) => {
            // Der Felsbrocken fliegt im Bogen und zerschellt
            let weite = von.distance(nach);
            for i in 0..=24 {
                let t = i as f32 / 24.0;
                let p = von.lerp(nach, t) + Vec3::Y * (t * (1.0 - t) * weite * 0.5);
                funken(ctx, p, 2, vec3(0.5, 0.46, 0.42), 0.2, 0.28, 0.25 + t * 0.35, 0.0, 0.0);
            }
            funken(ctx, nach, 50, vec3(0.55, 0.5, 0.45), 6.0, 0.3, 1.1, 10.0, 0.0);
            sounds.push(SoundEvent::Impact { at: nach, animal: false, killed: true });
        }
        Ereignis::Frostnova(at, radius) => {
            for r in [radius * 0.3, radius * 0.65, radius] {
                ring(ctx, at, r, vec3(0.6, 0.85, 1.0), 4.0, 2.0);
            }
            funken(ctx, at + Vec3::Y * 2.0, 70, vec3(0.75, 0.92, 1.0), 6.0, 0.14, 1.3, 1.5, 4.0);
            sounds.push(SoundEvent::Thunder { volume: 0.3 });
        }
        Ereignis::Netz(at, radius) => {
            ring(ctx, at, radius, vec3(0.92, 0.92, 0.88), 1.2, 1.0);
            ring(ctx, at, radius * 0.5, vec3(0.92, 0.92, 0.88), 1.2, 1.0);
            funken(ctx, at + Vec3::Y * 3.0, 60, vec3(0.9, 0.9, 0.86), 7.0, 0.1, 1.2, 3.0, 1.0);
        }
        Ereignis::Brut(at) => funken(ctx, at + Vec3::Y * 0.5, 40, vec3(0.3, 0.45, 0.2), 3.0, 0.16, 0.9, 6.0, 0.8),
        Ereignis::Wiedergeburt(at) => {
            ring(ctx, at, 5.0, vec3(1.0, 0.45, 0.1), 6.0, 4.0);
            funken(ctx, at, 120, vec3(1.0, 0.5, 0.12), 7.0, 0.25, 1.4, -2.0, 6.0);
            sounds.push(SoundEvent::Thunder { volume: 0.5 });
        }
        Ereignis::Flammenatem(maul, richtung) => {
            let r = vec3(richtung.x, -0.15, richtung.y).normalize_or(Vec3::Z);
            for i in 0..6 {
                ctx.particles.burst(Burst {
                    position: maul + r * (i as f32 * 1.5),
                    count: 30,
                    color: vec3(1.0, 0.45 + i as f32 * 0.05, 0.1),
                    color_variation: 0.3,
                    speed: 14.0,
                    direction: r * 3.0,
                    size: 0.45 + i as f32 * 0.1,
                    life: 0.9,
                    gravity: -1.5,
                    glow: 5.0,
                    grow: 1.8,
                    round: true,
                });
            }
            sounds.push(SoundEvent::Thunder { volume: 0.35 });
        }
        Ereignis::Auffliegen(at) => {
            ring(ctx, at - Vec3::Y * 2.0, 8.0, vec3(0.7, 0.66, 0.6), 0.0, 3.0);
            funken(ctx, at - Vec3::Y * 2.0, 60, vec3(0.7, 0.65, 0.58), 8.0, 0.5, 1.2, 0.5, 0.0);
        }
        Ereignis::Sporenexplosion(at) => {
            // Greller Blitz, rote und weiße Hutfetzen, eine dichte gelbgrüne Sporenwolke
            ctx.particles.burst_glow(Burst {
                position: at,
                count: 40,
                color: vec3(1.0, 0.8, 0.35),
                color_variation: 0.2,
                speed: 7.0,
                direction: Vec3::ZERO,
                size: 0.28,
                life: 0.45,
                gravity: 0.0,
                glow: 6.0,
                grow: 1.5,
                round: true,
            });
            for (farbe, n) in [(vec3(0.85, 0.12, 0.08), 26), (vec3(0.96, 0.94, 0.88), 14)] {
                ctx.particles.burst(Burst {
                    position: at + Vec3::Y * 0.3,
                    count: n,
                    color: farbe,
                    color_variation: 0.1,
                    speed: 7.5,
                    direction: Vec3::Y * 0.8,
                    size: 0.12,
                    life: 1.1,
                    gravity: 9.0,
                    glow: 0.0,
                    grow: 0.0,
                    round: false,
                });
            }
            // Leuchtender, durchscheinender Sporennebel und einzelne helle Sporen, die langsam sinken
            ctx.particles.burst_glow(Burst {
                position: at,
                count: 45,
                color: vec3(0.32, 0.42, 0.1),
                color_variation: 0.3,
                speed: 2.8,
                direction: Vec3::ZERO,
                size: 0.3,
                life: 1.8,
                gravity: -0.2,
                glow: 1.2,
                grow: 2.2,
                round: true,
            });
            ctx.particles.burst_glow(Burst {
                position: at + Vec3::Y * 0.4,
                count: 60,
                color: vec3(0.9, 1.0, 0.45),
                color_variation: 0.2,
                speed: 3.5,
                direction: Vec3::Y * 0.4,
                size: 0.05,
                life: 2.4,
                gravity: 0.6,
                glow: 3.0,
                grow: 0.0,
                round: true,
            });
            ring(ctx, at - Vec3::Y * 0.5, crate::wildnis::SPOREN_RADIUS, vec3(0.8, 0.85, 0.4), 1.5, 0.5);
            sounds.push(SoundEvent::Impact { at, animal: false, killed: true });
            sounds.push(SoundEvent::Thunder { volume: 0.25 });
        }
        Ereignis::Ansturm(at) => {
            // Staubwolke vom Scharren
            ctx.particles.burst(Burst {
                position: at + Vec3::Y * 0.2,
                count: 30,
                color: vec3(0.55, 0.45, 0.32),
                color_variation: 0.15,
                speed: 2.5,
                direction: Vec3::Y * 0.4,
                size: 0.25,
                life: 1.2,
                gravity: -0.2,
                glow: 0.0,
                grow: 2.0,
                round: true,
            });
            sounds.push(SoundEvent::Impact { at, animal: true, killed: false });
        }
        Ereignis::Wurzelwarnung(at) => {
            // Der Boden bricht auf: grüner Warnring und bröckelnde Erde
            ring(ctx, at + Vec3::Y * 0.05, crate::wildnis::WURZEL_RADIUS, vec3(0.45, 0.8, 0.2), 2.0, 0.2);
            funken(ctx, at, 24, vec3(0.4, 0.3, 0.2), 1.5, 0.12, crate::wildnis::WURZEL_WARNUNG, 3.0, 0.0);
            felder.push((at, crate::wildnis::WURZEL_RADIUS, crate::wildnis::WURZEL_WARNUNG, WURZEL));
        }
        Ereignis::Wurzeln(at) => {
            // Wurzeln schießen hervor: Erdbrocken, Holzsplitter, Blätter
            for (farbe, n, tempo) in [(vec3(0.3, 0.22, 0.14), 40, 8.0), (vec3(0.42, 0.3, 0.18), 30, 10.0), (vec3(0.35, 0.6, 0.18), 20, 5.0)] {
                ctx.particles.burst(Burst {
                    position: at + Vec3::Y * 0.2,
                    count: n,
                    color: farbe,
                    color_variation: 0.2,
                    speed: tempo,
                    direction: Vec3::Y,
                    size: 0.14,
                    life: 1.1,
                    gravity: 12.0,
                    glow: 0.0,
                    grow: 0.0,
                    round: false,
                });
            }
            ring(ctx, at, crate::wildnis::WURZEL_RADIUS, vec3(0.4, 0.3, 0.2), 0.0, 1.5);
            sounds.push(SoundEvent::Impact { at, animal: false, killed: true });
        }
        Ereignis::Frostfeld(at, radius) => {
            ring(ctx, at, radius, vec3(0.6, 0.85, 1.0), 2.5, 0.6);
            felder.push((at, radius, 1.4, FROST));
        }
    }
}

/// Felder am Boden: Flammen, Giftschwaden, Eiskristalle – ein paar Teilchen je Bild.
pub fn felder(ctx: &mut Context, felder: &mut Vec<(Vec3, f32, f32, u8)>, rng: &mut Rng) {
    let dt = ctx.time.delta;
    for (mitte, radius, rest, art) in felder.iter_mut() {
        *rest -= dt;
        if mitte.distance(ctx.camera.position) > 150.0 {
            continue;
        }
        let dichte = (*radius * *radius * 1.2 * dt).min(3.0);
        let anzahl = dichte.floor() as u32 + rng.chance(dichte.fract()) as u32;
        for _ in 0..anzahl {
            let w = rng.range(0.0, std::f32::consts::TAU);
            let r = rng.range(0.0, *radius).sqrt() * radius.sqrt();
            let p = *mitte + vec3(w.cos() * r, 0.15, w.sin() * r);
            let (color, size, life, gravity, glow, grow) = match *art {
                BRAND => (vec3(1.0, 0.5, 0.12), 0.18, 0.7, -2.5, 4.0, 0.0),
                GIFT => (vec3(0.35, 0.85, 0.25), 0.45, 1.4, -0.2, 0.6, 1.5),
                WURZEL => (vec3(0.38, 0.27, 0.16), 0.09, 0.5, 9.0, 0.0, 0.0),
                _ => (vec3(0.7, 0.9, 1.0), 0.1, 0.9, -0.5, 2.5, 0.0),
            };
            ctx.particles.burst(Burst {
                position: p,
                count: 1,
                color,
                color_variation: 0.2,
                speed: 0.4,
                direction: Vec3::Y,
                size,
                life,
                gravity,
                glow,
                grow,
                round: true,
            });
        }
    }
    felder.retain(|f| f.2 > 0.0);
}
