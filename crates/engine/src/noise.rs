//! Deterministisches Rauschen und Zufall für prozedurale Welten.
//!
//! Nur Ganzzahl-Hashing und Grundrechenarten: Auf jedem Rechner und Betriebssystem
//! entsteht aus demselben Startwert exakt dieselbe Welt – wichtig für Multiplayer,
//! weil Server und Clients die Insel unabhängig voneinander erzeugen.

use glam::Vec2;

/// Mischt Ganzzahlen zu einem gut verteilten Zufallswert.
pub fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343) ^ (y as u32).wrapping_mul(0xD816_3841) ^ seed.wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    h
}

/// Zufallswert 0..1 für eine Gitterzelle.
pub fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn grad(ix: i32, iy: i32, seed: u32, dx: f32, dy: f32) -> f32 {
    // Acht feste Richtungen statt Winkelfunktionen – bleibt plattformunabhängig.
    match hash(ix, iy, seed) & 7 {
        0 => dx + dy,
        1 => dx - dy,
        2 => -dx + dy,
        3 => -dx - dy,
        4 => dx,
        5 => -dx,
        6 => dy,
        _ => -dy,
    }
}

/// Gradientenrauschen (Perlin-artig), Ergebnis etwa −1..1.
pub fn perlin(p: Vec2, seed: u32) -> f32 {
    let (fx, fy) = (p.x.floor(), p.y.floor());
    let (ix, iy) = (fx as i32, fy as i32);
    let (dx, dy) = (p.x - fx, p.y - fy);
    let (u, v) = (smooth(dx), smooth(dy));
    let a = grad(ix, iy, seed, dx, dy);
    let b = grad(ix + 1, iy, seed, dx - 1.0, dy);
    let c = grad(ix, iy + 1, seed, dx, dy - 1.0);
    let d = grad(ix + 1, iy + 1, seed, dx - 1.0, dy - 1.0);
    let top = a + (b - a) * u;
    let bottom = c + (d - c) * u;
    (top + (bottom - top) * v) * 0.7
}

/// Mehrere Rausch-Lagen übereinander (grobe Form + feine Details), etwa −1..1.
pub fn fbm(p: Vec2, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amplitude, mut frequency, mut total) = (0.0, 1.0, 1.0, 0.0);
    for octave in 0..octaves {
        sum += perlin(p * frequency, seed.wrapping_add(octave * 101)) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / total
}

/// Grat-Rauschen für Gebirge: scharfe Kämme, Ergebnis 0..1.
pub fn ridged(p: Vec2, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amplitude, mut frequency, mut total) = (0.0, 1.0, 1.0, 0.0);
    for octave in 0..octaves {
        let n = 1.0 - perlin(p * frequency, seed.wrapping_add(octave * 131)).abs() * 1.4;
        sum += n.max(0.0).powi(2) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / total
}

/// Kleiner deterministischer Zufallsgenerator (für Platzierung, Varianten).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u32(&mut self) -> u32 {
        // SplitMix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    /// Zufallswert 0..1.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    pub fn chance(&mut self, probability: f32) -> bool {
        self.next_f32() < probability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rauschen_ist_deterministisch_und_im_bereich() {
        for i in 0..2000 {
            let p = Vec2::new(i as f32 * 0.37, i as f32 * 0.11 - 50.0);
            let n = fbm(p, 5, 7);
            assert_eq!(n, fbm(p, 5, 7));
            assert!((-1.0..=1.0).contains(&n), "fbm außerhalb: {n}");
            assert!((0.0..=1.0).contains(&ridged(p, 4, 3)));
        }
        assert_ne!(fbm(Vec2::new(1.3, 2.7), 4, 1), fbm(Vec2::new(1.3, 2.7), 4, 2), "Startwert muss etwas ändern");
    }
}
