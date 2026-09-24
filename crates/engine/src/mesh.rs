//! Geometrie: eingebaute Grundformen und Werkzeuge für Low-Poly-Modelle.
//!
//! Jeder Eckpunkt hat eine eigene Farbe. Damit lassen sich Modelle ohne Texturen
//! mehrfarbig gestalten (Stamm braun, Krone grün) – der typische Low-Poly-Stil.

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Grundfarbe in linearem RGB; wird mit der Farbe des Objekts multipliziert.
    pub color: [f32; 3],
    /// Texturkoordinaten (nur bei Meshes mit Textur von Bedeutung).
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, color: Vec3) -> Self {
        Vertex { position: position.into(), normal: normal.into(), color: color.into(), uv: [0.0, 0.0] }
    }
}

/// Geometrie im Arbeitsspeicher. Der Renderer lädt sie bei Bedarf auf die Grafikkarte.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// Textur für das ganze Mesh; ohne Textur zählen nur die Farben.
    pub texture: Option<crate::assets::TextureId>,
    /// Beide Seiten der Dreiecke zeichnen (Blätter, Gras, dünne Flächen).
    pub double_sided: bool,
    /// Pixel mit Textur-Alpha unter 0,5 weglassen (ausgeschnittene Blätter, Zäune).
    pub alpha_cutout: bool,
}

impl MeshData {
    /// Würfel mit Kantenlänge 1, Mittelpunkt im Ursprung.
    pub fn cube() -> Self {
        // (Normale, u, v) mit u × v = Normale, damit die Dreiecke von außen gegen den
        // Uhrzeigersinn laufen (Front-Face = CCW).
        let faces = [
            (Vec3::X, Vec3::Y, Vec3::Z),
            (Vec3::NEG_X, Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::Z, Vec3::X),
            (Vec3::NEG_Y, Vec3::X, Vec3::Z),
            (Vec3::Z, Vec3::X, Vec3::Y),
            (Vec3::NEG_Z, Vec3::Y, Vec3::X),
        ];
        let mut mesh = MeshData::default();
        for (n, u, v) in faces {
            mesh.push_quad(n * 0.5, u * 0.5, v * 0.5, n, Vec3::ONE);
        }
        mesh
    }

    /// Ebene mit Kantenlänge 1 in der XZ-Ebene, Normale nach oben.
    pub fn plane() -> Self {
        let mut mesh = MeshData::default();
        mesh.push_quad(Vec3::ZERO, Vec3::Z * 0.5, Vec3::X * 0.5, Vec3::Y, Vec3::ONE);
        mesh
    }

    /// Ebene aus `cells` × `cells` Feldern, Kantenlänge 1 (für Wasser mit Wellen).
    pub fn grid(cells: u32) -> Self {
        let mut mesh = MeshData::default();
        for z in 0..=cells {
            for x in 0..=cells {
                let p = Vec3::new(x as f32 / cells as f32 - 0.5, 0.0, z as f32 / cells as f32 - 0.5);
                mesh.vertices.push(Vertex::new(p, Vec3::Y, Vec3::ONE));
            }
        }
        let row = cells + 1;
        for z in 0..cells {
            for x in 0..cells {
                let a = z * row + x;
                mesh.indices.extend_from_slice(&[a, a + row, a + 1, a + 1, a + row, a + row + 1]);
            }
        }
        mesh
    }

    /// Kugel mit Durchmesser 1.
    pub fn sphere(segments: u32, rings: u32) -> Self {
        let mut mesh = MeshData::default();
        for ring in 0..=rings {
            let theta = std::f32::consts::PI * ring as f32 / rings as f32;
            for seg in 0..=segments {
                let phi = std::f32::consts::TAU * seg as f32 / segments as f32;
                let n = Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin());
                mesh.vertices.push(Vertex::new(n * 0.5, n, Vec3::ONE));
            }
        }
        let stride = segments + 1;
        for ring in 0..rings {
            for seg in 0..segments {
                let a = ring * stride + seg;
                let b = a + stride;
                mesh.indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
            }
        }
        mesh
    }

    /// Aufrechte Kapsel, `height` ist die Gesamthöhe inklusive Halbkugeln.
    /// Passt zu `physics::Shape::Capsule` mit denselben Maßen.
    pub fn capsule(radius: f32, height: f32, segments: u32, rings_per_cap: u32) -> Self {
        let half_cylinder = (height / 2.0 - radius).max(0.0);
        let mut mesh = MeshData::default();
        // Obere Halbkugel, dann untere; der Äquator kommt doppelt vor, dazwischen
        // entsteht der Zylindermantel.
        let rows = (0..=rings_per_cap)
            .map(|r| (r, half_cylinder))
            .chain((rings_per_cap..=2 * rings_per_cap).map(|r| (r, -half_cylinder)));
        let mut row_count = 0;
        for (ring, offset) in rows {
            let theta = std::f32::consts::PI * ring as f32 / (2 * rings_per_cap) as f32;
            for seg in 0..=segments {
                let phi = std::f32::consts::TAU * seg as f32 / segments as f32;
                let n = Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin());
                mesh.vertices.push(Vertex::new(n * radius + Vec3::Y * offset, n, Vec3::ONE));
            }
            row_count += 1;
        }
        let stride = segments + 1;
        for row in 0..row_count - 1 {
            for seg in 0..segments {
                let a = row * stride + seg;
                let b = a + stride;
                mesh.indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
            }
        }
        mesh
    }

    /// Kegelstumpf entlang +Y von 0 bis `height` (mit `top_radius` 0 ein Kegel).
    pub fn cylinder(bottom_radius: f32, top_radius: f32, height: f32, segments: u32, color: Vec3) -> Self {
        let mut mesh = MeshData::default();
        let ring = |r: f32, y: f32, i: u32| {
            let a = std::f32::consts::TAU * i as f32 / segments as f32;
            Vec3::new(a.cos() * r, y, a.sin() * r)
        };
        for i in 0..segments {
            let (b0, b1) = (ring(bottom_radius, 0.0, i), ring(bottom_radius, 0.0, i + 1));
            let (t0, t1) = (ring(top_radius, height, i), ring(top_radius, height, i + 1));
            // Mantel (von außen gegen den Uhrzeigersinn)
            mesh.push_triangle(b0, t1, b1, color);
            if top_radius > 0.0 {
                mesh.push_triangle(b0, t0, t1, color);
                mesh.push_triangle(Vec3::Y * height, t1, t0, color);
            }
            mesh.push_triangle(Vec3::ZERO, b0, b1, color);
        }
        mesh
    }

    /// Kugelähnlicher Körper aus Dreiecken (Ikosaeder, `subdivisions` mal verfeinert),
    /// Durchmesser 1. Ideal als Grundlage für Felsen und Baumkronen.
    pub fn icosphere(subdivisions: u32, color: Vec3) -> Self {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let mut points: Vec<Vec3> = [
            (-1.0, t, 0.0), (1.0, t, 0.0), (-1.0, -t, 0.0), (1.0, -t, 0.0),
            (0.0, -1.0, t), (0.0, 1.0, t), (0.0, -1.0, -t), (0.0, 1.0, -t),
            (t, 0.0, -1.0), (t, 0.0, 1.0), (-t, 0.0, -1.0), (-t, 0.0, 1.0),
        ]
        .iter()
        .map(|&(x, y, z)| Vec3::new(x, y, z).normalize())
        .collect();
        let mut faces: Vec<[usize; 3]> = vec![
            [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
            [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
            [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
            [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
        ];
        for _ in 0..subdivisions {
            let mut cache = std::collections::HashMap::new();
            let mut midpoint = |a: usize, b: usize, points: &mut Vec<Vec3>| {
                *cache.entry((a.min(b), a.max(b))).or_insert_with(|| {
                    points.push(((points[a] + points[b]) * 0.5).normalize());
                    points.len() - 1
                })
            };
            faces = faces
                .iter()
                .flat_map(|&[a, b, c]| {
                    let ab = midpoint(a, b, &mut points);
                    let bc = midpoint(b, c, &mut points);
                    let ca = midpoint(c, a, &mut points);
                    [[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]
                })
                .collect();
        }
        let mut mesh = MeshData::default();
        for &p in &points {
            mesh.vertices.push(Vertex::new(p * 0.5, p, color));
        }
        mesh.indices = faces.iter().flat_map(|&[a, b, c]| [a as u32, b as u32, c as u32]).collect();
        mesh
    }

    /// Fügt ein Dreieck mit eigener, flacher Normale hinzu (Ecken gegen den Uhrzeigersinn).
    pub fn push_triangle(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Vec3) {
        let normal = (b - a).cross(c - a).normalize_or_zero();
        let base = self.vertices.len() as u32;
        for p in [a, b, c] {
            self.vertices.push(Vertex::new(p, normal, color));
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Hängt ein anderes Mesh an, vorher mit `transform` verschoben/gedreht/skaliert.
    pub fn append(&mut self, other: &MeshData, transform: Mat4) {
        let normal_matrix = Mat3::from_mat4(transform).inverse().transpose();
        let base = self.vertices.len() as u32;
        self.vertices.extend(other.vertices.iter().map(|v| Vertex {
            position: transform.transform_point3(v.position.into()).into(),
            normal: (normal_matrix * Vec3::from(v.normal)).normalize_or_zero().into(),
            color: v.color,
            uv: v.uv,
        }));
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Verschiebt jeden Eckpunkt: `f(position) -> neue Position`. Danach passen die
    /// Normalen nicht mehr – meist folgt [`flat_shaded`](Self::flat_shaded).
    pub fn displace(mut self, f: impl Fn(Vec3) -> Vec3) -> Self {
        for v in &mut self.vertices {
            v.position = f(v.position.into()).into();
        }
        self
    }

    /// Färbt jeden Eckpunkt neu: `f(position, alte Farbe) -> Farbe`.
    pub fn recolor(mut self, f: impl Fn(Vec3, Vec3) -> Vec3) -> Self {
        for v in &mut self.vertices {
            v.color = f(v.position.into(), v.color.into()).into();
        }
        self
    }

    /// Jedes Dreieck bekommt eigene Eckpunkte und eine flache Normale – facettierter
    /// Low-Poly-Look. Die Farbe eines Dreiecks ist der Mittelwert seiner Ecken.
    pub fn flat_shaded(&self) -> Self {
        let mut mesh = MeshData::default();
        for tri in self.indices.chunks_exact(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| self.vertices[i as usize]);
            let color = (Vec3::from(a.color) + Vec3::from(b.color) + Vec3::from(c.color)) / 3.0;
            mesh.push_triangle(a.position.into(), b.position.into(), c.position.into(), color);
        }
        mesh
    }

    /// Hüllkugel (Mittelpunkt, Radius) – für das Aussortieren unsichtbarer Objekte.
    pub fn bounds(&self) -> (Vec3, f32) {
        if self.vertices.is_empty() {
            return (Vec3::ZERO, 0.0);
        }
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &self.vertices {
            min = min.min(v.position.into());
            max = max.max(v.position.into());
        }
        let center = (min + max) * 0.5;
        let radius = self.vertices.iter().map(|v| center.distance(v.position.into())).fold(0.0, f32::max);
        (center, radius)
    }

    /// Vereinfachte Fassung für große Entfernungen: Eckpunkte in Würfeln der Kantenlänge
    /// `cell` werden zusammengelegt, dabei entartete Dreiecke fallen weg. Ergebnis ist flach
    /// schattiert (passt zum Low-Poly-Stil).
    pub fn simplified(&self, cell: f32) -> MeshData {
        use std::collections::HashMap;
        let key = |p: Vec3| {
            let c = (p / cell).floor().as_ivec3();
            (c.x, c.y, c.z)
        };
        // Mittelpunkt aller Eckpunkte je Zelle
        let mut sums: HashMap<(i32, i32, i32), (Vec3, u32)> = HashMap::new();
        for v in &self.vertices {
            let p = Vec3::from(v.position);
            let entry = sums.entry(key(p)).or_insert((Vec3::ZERO, 0));
            entry.0 += p;
            entry.1 += 1;
        }
        let snap = |p: Vec3| {
            let (sum, count) = sums[&key(p)];
            sum / count as f32
        };
        let mut mesh = MeshData { texture: self.texture, double_sided: self.double_sided, alpha_cutout: self.alpha_cutout, ..Default::default() };
        for tri in self.indices.chunks_exact(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| self.vertices[i as usize]);
            let [pa, pb, pc] = [a, b, c].map(|v| snap(v.position.into()));
            if (pb - pa).cross(pc - pa).length_squared() < 1e-10 {
                continue;
            }
            let color = (Vec3::from(a.color) + Vec3::from(b.color) + Vec3::from(c.color)) / 3.0;
            let normal = (pb - pa).cross(pc - pa).normalize();
            let base = mesh.vertices.len() as u32;
            for (p, v) in [(pa, a), (pb, b), (pc, c)] {
                mesh.vertices.push(Vertex { position: p.into(), normal: normal.into(), color: color.into(), uv: v.uv });
            }
            mesh.indices.extend([base, base + 1, base + 2]);
        }
        mesh
    }

    fn push_quad(&mut self, center: Vec3, u: Vec3, v: Vec3, normal: Vec3, color: Vec3) {
        let base = self.vertices.len() as u32;
        for corner in [center - u - v, center + u - v, center + u + v, center - u + v] {
            self.vertices.push(Vertex::new(corner, normal, color));
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vereinfachen_spart_dreiecke_und_behaelt_die_form() {
        let detailed = MeshData::icosphere(3, Vec3::ONE).flat_shaded();
        let coarse = detailed.simplified(0.25);
        let (tris_before, tris_after) = (detailed.indices.len() / 3, coarse.indices.len() / 3);
        assert!(tris_after > 8 && tris_after < tris_before / 3, "{tris_before} → {tris_after}");
        let (center, radius) = coarse.bounds();
        assert!(center.length() < 0.05 && (radius - 0.5).abs() < 0.08, "Form verloren: {center} {radius}");
    }

    /// Jedes Dreieck muss vom Mittelpunkt `center` weg zeigen (sonst wird es weggeschnitten).
    fn assert_outward(name: &str, mesh: &MeshData, center: Vec3) {
        for tri in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| Vec3::from(mesh.vertices[i as usize].position));
            let normal = (b - a).cross(c - a);
            if normal.length_squared() < 1e-12 {
                continue; // entartet (z. B. an den Polen einer Kugel), unsichtbar
            }
            let centroid = (a + b + c) / 3.0;
            assert!(normal.dot(centroid - center) > 0.0, "{name}: Dreieck {a} {b} {c} zeigt nach innen");
        }
    }

    #[test]
    fn alle_grundformen_zeigen_nach_aussen() {
        assert_outward("Würfel", &MeshData::cube(), Vec3::ZERO);
        assert_outward("Kugel", &MeshData::sphere(12, 8), Vec3::ZERO);
        assert_outward("Kapsel", &MeshData::capsule(0.4, 1.8, 12, 4), Vec3::ZERO);
        assert_outward("Ikosphäre", &MeshData::icosphere(2, Vec3::ONE), Vec3::ZERO);
        assert_outward("Zylinder", &MeshData::cylinder(0.5, 0.3, 2.0, 8, Vec3::ONE), Vec3::Y);
        assert_outward("Kegel", &MeshData::cylinder(0.5, 0.0, 2.0, 8, Vec3::ONE), Vec3::Y * 0.5);
        assert_outward("flach", &MeshData::icosphere(1, Vec3::ONE).flat_shaded(), Vec3::ZERO);
    }
}
