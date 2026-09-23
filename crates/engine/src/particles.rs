//! Einfache Partikel: kleine, fliegende Würfel für Splitter, Funken, Blätter, Staub.
//! Rein optisch – sie haben keine Physik-Körper und werden nicht übers Netz geschickt.

use glam::{Mat4, Quat, Vec3, Vec4};

use crate::noise::Rng;

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Vec4,
    pub size: f32,
    pub life: f32,
    pub max_life: f32,
    pub gravity: f32,
    pub spin: Vec3,
    pub rotation: Quat,
    /// Leuchtet selbst (z. B. Funken, Magie).
    pub glow: f32,
}

/// Beschreibung für einen Schwall Partikel.
#[derive(Clone, Copy, Debug)]
pub struct Burst {
    pub position: Vec3,
    pub count: u32,
    pub color: Vec3,
    /// Zufällige Abweichung der Farbe (0 = alle gleich).
    pub color_variation: f32,
    pub speed: f32,
    /// Grundrichtung (wird zufällig gestreut); `Vec3::ZERO` = in alle Richtungen.
    pub direction: Vec3,
    pub size: f32,
    pub life: f32,
    pub gravity: f32,
    pub glow: f32,
}

impl Default for Burst {
    fn default() -> Self {
        Burst {
            position: Vec3::ZERO,
            count: 12,
            color: Vec3::ONE,
            color_variation: 0.15,
            speed: 4.0,
            direction: Vec3::Y,
            size: 0.12,
            life: 0.9,
            gravity: 12.0,
            glow: 0.0,
        }
    }
}

pub struct Particles {
    list: Vec<Particle>,
    rng: Rng,
}

impl Default for Particles {
    fn default() -> Self {
        Particles { list: Vec::new(), rng: Rng::new(0xA11CE) }
    }
}

impl Particles {
    /// Obergrenze, damit ein Effekt-Gewitter die Bildrate nicht ruiniert.
    const MAX: usize = 4000;

    pub fn burst(&mut self, burst: Burst) {
        for _ in 0..burst.count {
            if self.list.len() >= Self::MAX {
                self.list.swap_remove(0);
            }
            let r = &mut self.rng;
            let random_dir = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0)).normalize_or(Vec3::Y);
            let dir = (burst.direction + random_dir * 0.8).normalize_or(random_dir);
            let shade = 1.0 + r.range(-burst.color_variation, burst.color_variation);
            let life = burst.life * r.range(0.6, 1.2);
            self.list.push(Particle {
                position: burst.position,
                velocity: dir * burst.speed * r.range(0.5, 1.2),
                color: (burst.color * shade).extend(1.0),
                size: burst.size * r.range(0.6, 1.3),
                life,
                max_life: life,
                gravity: burst.gravity,
                spin: Vec3::new(r.range(-9.0, 9.0), r.range(-9.0, 9.0), r.range(-9.0, 9.0)),
                rotation: Quat::IDENTITY,
                glow: burst.glow,
            });
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        for p in &mut self.list {
            p.life -= dt;
            p.velocity.y -= p.gravity * dt;
            p.velocity *= 1.0 - (1.5 * dt).min(1.0);
            p.position += p.velocity * dt;
            p.rotation = (Quat::from_scaled_axis(p.spin * dt) * p.rotation).normalize();
        }
        self.list.retain(|p| p.life > 0.0);
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// (Modellmatrix, Farbe, Leuchtkraft) jedes Partikels; schrumpft zum Lebensende.
    pub(crate) fn instances(&self) -> impl Iterator<Item = (Mat4, Vec4, f32)> + '_ {
        self.list.iter().map(|p| {
            let scale = p.size * (p.life / p.max_life).sqrt();
            (Mat4::from_scale_rotation_translation(Vec3::splat(scale), p.rotation, p.position), p.color, p.glow)
        })
    }
}
