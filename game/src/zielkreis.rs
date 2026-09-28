//! Zielkreis am Boden: zeigt, wo eine Fähigkeit mit Flächenschaden einschlagen wird (bzw. wie weit
//! sie um die Figur reicht). Der Kreis besteht aus kurzen, leuchtenden Stücken, die jedes für
//! sich auf dem Gelände liegen – so folgt er Hängen und Mulden. Innen dreht sich ein
//! gestrichelter Ring, in der Mitte sitzt eine Raute.

use engine::prelude::*;

/// Stücke des äußeren und des inneren Rings.
const AUSSEN: usize = 44;
const INNEN: usize = 18;

pub struct Zielkreis {
    mesh: Option<MeshId>,
    teile: Vec<EntityId>,
    sichtbar: bool,
}

impl Default for Zielkreis {
    fn default() -> Self {
        Zielkreis { mesh: None, teile: Vec::new(), sichtbar: false }
    }
}

/// Ein flaches Stück: 1 m lang (entlang X, um die Mitte), 1 m breit (entlang Z, nach außen).
fn stueck() -> MeshData {
    let mut mesh = MeshData::default();
    let (a, b, c, d) = (vec3(-0.5, 0.0, -0.5), vec3(0.5, 0.0, -0.5), vec3(0.5, 0.0, 0.5), vec3(-0.5, 0.0, 0.5));
    mesh.push_triangle(a, c, b, Vec3::ONE);
    mesh.push_triangle(a, d, c, Vec3::ONE);
    mesh.double_sided = true;
    mesh
}

impl Zielkreis {
    fn anlegen(&mut self, ctx: &mut Context) {
        if !self.teile.is_empty() {
            return;
        }
        let mesh = *self.mesh.get_or_insert_with(|| ctx.assets.add_mesh(stueck()));
        for _ in 0..(AUSSEN + INNEN + 2) {
            let mut e = Entity::new("Zielkreis", mesh).with_material(Material::Emissive { glow: 1.0 });
            e.casts_shadow = false;
            e.visible = false;
            self.teile.push(ctx.scene.spawn(e));
        }
    }

    /// Kreis um `mitte` (am Boden) mit `radius` zeigen. `bereit`: die Fähigkeit ist einsatzbereit
    /// (sonst blasser); `farbe` RGB.
    pub fn zeigen(&mut self, ctx: &mut Context, mitte: Vec3, radius: f32, farbe: Vec3, bereit: bool, hoehe: &dyn Fn(Vec2) -> f32) {
        self.anlegen(ctx);
        self.sichtbar = true;
        let t = ctx.time.elapsed;
        let puls = 0.85 + 0.15 * (t * 4.0).sin();
        let staerke = if bereit { puls } else { 0.35 };
        let farbe = (farbe * staerke).extend(1.0);
        // Auf dem Gelände: nie unter den Mittelpunkt (auf Brücken, in der Burg), leicht darüber
        let boden = |p: Vec2| hoehe(p).max(mitte.y - 1.0).min(mitte.y + 2.5) + 0.12;
        let mut stellen: Vec<(Vec3, f32, Vec3)> = Vec::with_capacity(AUSSEN + INNEN + 2);
        let umfang = std::f32::consts::TAU * radius / AUSSEN as f32;
        for k in 0..AUSSEN {
            let w = std::f32::consts::TAU * (k as f32 + 0.5) / AUSSEN as f32;
            let p = vec2(mitte.x + w.cos() * radius, mitte.z + w.sin() * radius);
            stellen.push((vec3(p.x, boden(p), p.y), -w, vec3(umfang * 1.04, 1.0, 0.38)));
        }
        // Innen ein gestrichelter Ring, der sich langsam dreht
        let r_innen = radius * 0.55;
        let drehung = t * 0.8;
        for k in 0..INNEN {
            let w = std::f32::consts::TAU * (k as f32 + 0.5) / INNEN as f32 + drehung;
            let p = vec2(mitte.x + w.cos() * r_innen, mitte.z + w.sin() * r_innen);
            let laenge = std::f32::consts::TAU * r_innen / INNEN as f32 * 0.55;
            stellen.push((vec3(p.x, boden(p), p.y), -w, vec3(laenge, 1.0, 0.16)));
        }
        // Mitte: zwei gekreuzte Rauten
        let m = vec2(mitte.x, mitte.z);
        for k in 0..2 {
            stellen.push((vec3(m.x, boden(m), m.y), std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::FRAC_PI_2 - drehung, vec3(0.5, 1.0, 0.12)));
        }
        for (&entity, (ort, winkel, groesse)) in self.teile.iter().zip(stellen) {
            if let Some(e) = ctx.scene.try_get_mut(entity) {
                e.visible = true;
                e.color = farbe;
                // Das Stück liegt quer zum Radius: X entlang des Kreises
                e.transform = Transform::from_position(ort).with_rotation(Quat::from_rotation_y(winkel + std::f32::consts::FRAC_PI_2)).with_scale(groesse);
            }
        }
    }

    pub fn verstecken(&mut self, ctx: &mut Context) {
        if !self.sichtbar {
            return;
        }
        self.sichtbar = false;
        for &entity in &self.teile {
            if let Some(e) = ctx.scene.try_get_mut(entity) {
                e.visible = false;
            }
        }
    }
}
