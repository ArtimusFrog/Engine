//! Landschaft aus einem Höhenraster.

use glam::{Vec2, Vec3};

use crate::mesh::MeshData;

/// Quadratisches Höhenraster. Jede Zelle besteht aus zwei Dreiecken; Höhenabfragen
/// folgen exakt diesen Dreiecken, damit Objekte genau auf dem sichtbaren Boden stehen.
#[derive(Clone, Debug)]
pub struct Terrain {
    /// Welt-Position (x, z) der Ecke mit den kleinsten Koordinaten.
    origin: Vec2,
    cell_size: f32,
    /// Eckpunkte pro Seite (= Zellen + 1).
    points: usize,
    heights: Vec<f32>,
}

impl Terrain {
    /// Erzeugt ein Raster der Kantenlänge `size` um `center` mit `cells` × `cells` Zellen.
    /// `height(x, z)` liefert die Höhe an jedem Eckpunkt.
    pub fn generate(center: Vec2, size: f32, cells: usize, height: impl Fn(Vec2) -> f32) -> Self {
        let points = cells + 1;
        let cell_size = size / cells as f32;
        let origin = center - Vec2::splat(size / 2.0);
        let mut heights = Vec::with_capacity(points * points);
        for z in 0..points {
            for x in 0..points {
                heights.push(height(origin + Vec2::new(x as f32, z as f32) * cell_size));
            }
        }
        Terrain { origin, cell_size, points, heights }
    }

    pub fn size(&self) -> f32 {
        self.cell_size * (self.points - 1) as f32
    }

    pub fn center(&self) -> Vec2 {
        self.origin + Vec2::splat(self.size() / 2.0)
    }

    fn point(&self, x: usize, z: usize) -> Vec3 {
        let p = self.origin + Vec2::new(x as f32, z as f32) * self.cell_size;
        Vec3::new(p.x, self.heights[z * self.points + x], p.y)
    }

    /// Das Dreieck unter (x, z) – außerhalb wird auf den Rand begrenzt.
    fn triangle_at(&self, x: f32, z: f32) -> ([Vec3; 3], Vec2) {
        let local = (Vec2::new(x, z) - self.origin) / self.cell_size;
        let max = (self.points - 2) as f32;
        let cell = local.floor().clamp(Vec2::ZERO, Vec2::splat(max));
        let (cx, cz) = (cell.x as usize, cell.y as usize);
        let f = (local - cell).clamp(Vec2::ZERO, Vec2::ONE);
        let a = self.point(cx, cz);
        let b = self.point(cx + 1, cz);
        let c = self.point(cx, cz + 1);
        let d = self.point(cx + 1, cz + 1);
        // Diagonale von b nach c, wie in `mesh`.
        if f.x + f.y <= 1.0 { ([a, c, b], f) } else { ([b, c, d], f) }
    }

    /// Bodenhöhe an (x, z).
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let ([a, b, c], _) = self.triangle_at(x, z);
        // Baryzentrische Interpolation in der XZ-Ebene.
        let p = Vec2::new(x, z);
        let (a2, b2, c2) = (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z), Vec2::new(c.x, c.z));
        let v0 = b2 - a2;
        let v1 = c2 - a2;
        let v2 = p - a2;
        let denom = v0.perp_dot(v1);
        let v = v2.perp_dot(v1) / denom;
        let w = v0.perp_dot(v2) / denom;
        a.y + (b.y - a.y) * v + (c.y - a.y) * w
    }

    /// Oberflächen-Normale an (x, z).
    pub fn normal_at(&self, x: f32, z: f32) -> Vec3 {
        let ([a, b, c], _) = self.triangle_at(x, z);
        (b - a).cross(c - a).normalize()
    }

    /// Facettiertes Mesh. `color(mitte, normale)` bestimmt die Farbe jedes Dreiecks.
    pub fn mesh(&self, color: impl Fn(Vec3, Vec3) -> Vec3) -> MeshData {
        let mut mesh = MeshData::default();
        let cells = self.points - 1;
        mesh.vertices.reserve(cells * cells * 6);
        mesh.indices.reserve(cells * cells * 6);
        for z in 0..cells {
            for x in 0..cells {
                let a = self.point(x, z);
                let b = self.point(x + 1, z);
                let c = self.point(x, z + 1);
                let d = self.point(x + 1, z + 1);
                for [p, q, r] in [[a, c, b], [b, c, d]] {
                    let normal = (q - p).cross(r - p).normalize();
                    mesh.push_triangle(p, q, r, color((p + q + r) / 3.0, normal));
                }
            }
        }
        mesh
    }

    /// Facettiertes Mesh mit einer Textur über die ganze Fläche (u, v = 0..1 von Ecke zu Ecke).
    /// `tint(mitte, normale)` färbt jedes Dreieck zusätzlich (z. B. leichte Helligkeitsunterschiede).
    pub fn mesh_textured(&self, texture: crate::assets::TextureId, tint: impl Fn(Vec3, Vec3) -> Vec3) -> MeshData {
        let mut mesh = self.mesh(tint);
        let size = self.size();
        for vertex in &mut mesh.vertices {
            vertex.uv = [(vertex.position[0] - self.origin.x) / size, (vertex.position[2] - self.origin.y) / size];
        }
        mesh.texture = Some(texture);
        mesh
    }

    /// Eckpunkte und Dreiecke für die Kollision (siehe `Physics::add_static_mesh`).
    pub fn collision_mesh(&self) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let vertices = (0..self.points).flat_map(|z| (0..self.points).map(move |x| (x, z))).map(|(x, z)| self.point(x, z)).collect();
        let n = self.points as u32;
        let mut triangles = Vec::new();
        for z in 0..n - 1 {
            for x in 0..n - 1 {
                let a = z * n + x;
                let (b, c, d) = (a + 1, a + n, a + n + 1);
                triangles.push([a, c, b]);
                triangles.push([b, c, d]);
            }
        }
        (vertices, triangles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hoehe_folgt_den_dreiecken() {
        let terrain = Terrain::generate(Vec2::ZERO, 10.0, 10, |p| p.x * 0.5 + (p.y * 3.0).floor());
        // Auf Eckpunkten exakt die erzeugte Höhe
        assert!((terrain.height_at(-5.0, -5.0) - (-2.5 - 15.0)).abs() < 1e-4);
        assert!((terrain.height_at(2.0, 3.0) - (1.0 + 9.0)).abs() < 1e-4);
        // Normale einer schiefen Ebene zeigt schräg nach oben
        let flat = Terrain::generate(Vec2::ZERO, 10.0, 10, |p| p.x);
        let n = flat.normal_at(0.3, 0.7);
        assert!((n - Vec3::new(-1.0, 1.0, 0.0).normalize()).length() < 1e-4, "{n}");
        assert!((flat.height_at(1.25, -3.3) - 1.25).abs() < 1e-4);
    }
}
