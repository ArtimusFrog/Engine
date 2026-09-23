use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// Geometrie im Arbeitsspeicher. Der Renderer lädt sie bei Bedarf auf die Grafikkarte.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
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
            mesh.push_quad(n * 0.5, u * 0.5, v * 0.5, n);
        }
        mesh
    }

    /// Ebene mit Kantenlänge 1 in der XZ-Ebene, Normale nach oben.
    pub fn plane() -> Self {
        let mut mesh = MeshData::default();
        mesh.push_quad(Vec3::ZERO, Vec3::Z * 0.5, Vec3::X * 0.5, Vec3::Y);
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
                mesh.vertices.push(Vertex { position: (n * 0.5).into(), normal: n.into() });
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

    fn push_quad(&mut self, center: Vec3, u: Vec3, v: Vec3, normal: Vec3) {
        let base = self.vertices.len() as u32;
        for corner in [center - u - v, center + u - v, center + u + v, center - u + v] {
            self.vertices.push(Vertex { position: corner.into(), normal: normal.into() });
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}
