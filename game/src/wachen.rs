//! Schlosswache: zwei Ritter patrouillieren vor dem Portal des Schlosses (Burg Grünfels).
//!
//! Reine Kulisse (keine Kollision). Wo die Ritter gerade sind, folgt aus der Spieluhr, die der
//! Server an alle verteilt – so sehen alle Spieler sie an derselben Stelle, ohne dass etwas
//! übertragen werden muss. Modell: `game/assets/npc/ritter.gltf` (Blender: art/lib/ritter.py).

use std::sync::Arc;

use engine::prelude::*;

use crate::asset_files;
use crate::island::{burg_welt, BURG_HOEHE};

/// Schrittgeschwindigkeit (m/s) und wie lange sie an den Enden stehen bleiben (s).
const TEMPO: f32 = 1.25;
const PAUSE: f32 = 5.0;
/// Bis zu dieser Entfernung zur Kamera wird die Animation berechnet.
const ANIMATION_DISTANCE: f32 = 90.0;

/// Eine Patrouillenstrecke vor dem Portal (Blender-Koordinaten der Burganlage: x, y) und der
/// Zeitversatz, damit die beiden Ritter gegenläufig gehen und sich in der Mitte begegnen.
const STRECKEN: [((f32, f32), (f32, f32), f32); 2] = [((-6.5, -25.0), (6.5, -25.0), 0.0), ((6.5, -27.2), (-6.5, -27.2), 0.0)];

struct Ritter {
    entity: EntityId,
    animator: Animator,
    mesh: MeshId,
    texture: Option<TextureId>,
    von: Vec2,
    bis: Vec2,
    versatz: f32,
}

pub struct Wachen {
    ritter: Vec<Ritter>,
}

/// Blender-Koordinaten der Burganlage (x, y) → Welt (x, z).
fn welt(x: f32, y: f32) -> Vec2 {
    burg_welt(vec2(x, -y))
}

/// Vergangene Echtzeit laut Spieluhr (Sekunden): tagsüber 60 s je Stunde, nachts doppelt so
/// schnell – so laufen die Ritter immer gleich schnell.
fn uhr_sekunden(day: &DayCycle) -> f64 {
    let tag = day.seconds_per_hour as f64;
    let nacht = tag / day.night_speedup.max(0.01) as f64;
    let je_tag = 16.0 * tag + 8.0 * nacht;
    let h = day.hour as f64;
    let heute = if h < 5.0 {
        h * nacht
    } else if h < 21.0 {
        5.0 * nacht + (h - 5.0) * tag
    } else {
        5.0 * nacht + 16.0 * tag + (h - 21.0) * nacht
    };
    day.day as f64 * je_tag + heute
}

/// Lage auf der Strecke: Ort, Blickrichtung (Welt x, z) und ob er gerade geht.
fn patrouille(von: Vec2, bis: Vec2, t: f32) -> (Vec2, Vec2, bool) {
    let laenge = von.distance(bis);
    let gehen = laenge / TEMPO;
    let runde = 2.0 * (gehen + PAUSE);
    let u = t.rem_euclid(runde);
    let hin = (bis - von).normalize_or(Vec2::X);
    // Umdrehen in der ersten Sekunde der Pause
    let drehen = |richtung: Vec2, s: f32| {
        let k = (s / 1.2).clamp(0.0, 1.0);
        let k = k * k * (3.0 - 2.0 * k);
        (richtung * (1.0 - 2.0 * k) + richtung.perp() * (2.0 * k * (1.0 - k)) * 2.0).normalize_or(-richtung)
    };
    if u < gehen {
        (von.lerp(bis, u / gehen), hin, true)
    } else if u < gehen + PAUSE {
        (bis, drehen(hin, u - gehen), false)
    } else if u < 2.0 * gehen + PAUSE {
        (bis.lerp(von, (u - gehen - PAUSE) / gehen), -hin, true)
    } else {
        (von, drehen(-hin, u - 2.0 * gehen - PAUSE), false)
    }
}

impl Wachen {
    /// Lädt das Modell und stellt die Ritter auf (nur mit Fenster; `None`, wenn das Modell fehlt).
    pub fn new(ctx: &mut Context) -> Option<Wachen> {
        let path = asset_files::variants("npc", "ritter").into_iter().next()?;
        let model = match Model::from_file(&path) {
            Ok(model) => Arc::new(model),
            Err(message) => {
                log::warn!("Ritter: {message}");
                return None;
            }
        };
        let textures = model.register_textures(&mut ctx.assets);
        let texture = textures.first().copied();
        let ritter = STRECKEN
            .iter()
            .enumerate()
            .map(|(i, &((x0, y0), (x1, y1), versatz))| {
                let mut animator = Animator::new(model.clone());
                animator.play("Idle", true, 0.0);
                let mesh = ctx.assets.add_mesh(animator.skinned_mesh(texture));
                let (von, bis) = (welt(x0, y0), welt(x1, y1));
                let start = vec3(von.x, BURG_HOEHE + 0.08, von.y);
                let entity = ctx.scene.spawn(Entity::new(format!("Ritter {}", i + 1), mesh).with_transform(Transform::from_position(start)));
                Ritter { entity, animator, mesh, texture, von, bis, versatz }
            })
            .collect();
        Some(Wachen { ritter })
    }

    pub fn update(&mut self, ctx: &mut Context, day: &DayCycle) {
        let t = uhr_sekunden(day);
        let dt = ctx.time.delta;
        for ritter in &mut self.ritter {
            // Rest der Runde in f64 rechnen, erst dann auf f32 (große Uhrzeiten verlieren sonst Genauigkeit)
            let laenge = ritter.von.distance(ritter.bis);
            let runde = 2.0 * (laenge / TEMPO + PAUSE) as f64;
            let lokal = ((t + ritter.versatz as f64).rem_euclid(runde)) as f32;
            let (ort, blick, geht) = patrouille(ritter.von, ritter.bis, lokal);
            let position = vec3(ort.x, BURG_HOEHE + 0.08, ort.y);
            let near = position.distance(ctx.camera.position) < ANIMATION_DISTANCE;
            if near {
                let clip = if geht { "Laufen" } else { "Idle" };
                if ritter.animator.current() != Some(clip) {
                    ritter.animator.play(clip, true, 0.25);
                }
                ritter.animator.set_speed(if geht { 1.05 } else { 1.0 });
                ritter.animator.update(dt);
                ctx.assets.update_mesh(ritter.mesh, ritter.animator.skinned_mesh(ritter.texture));
            }
            if let Some(entity) = ctx.scene.try_get_mut(ritter.entity) {
                entity.transform.position = position;
                // Das Modell schaut nach +Z
                entity.transform.rotation = Quat::from_rotation_y(blick.x.atan2(blick.y));
            }
        }
    }
}
