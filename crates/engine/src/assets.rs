use crate::mesh::MeshData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeshId(pub(crate) u32);

/// Verwaltet alle Ressourcen des Spiels. Hängt nicht von der Grafikkarte ab,
/// damit später auch ein Server ohne Fenster dieselben Daten laden kann.
pub struct Assets {
    meshes: Vec<MeshData>,
    cube: MeshId,
    plane: MeshId,
    sphere: MeshId,
}

impl Assets {
    pub(crate) fn new() -> Self {
        let mut assets = Assets { meshes: Vec::new(), cube: MeshId(0), plane: MeshId(0), sphere: MeshId(0) };
        assets.cube = assets.add_mesh(MeshData::cube());
        assets.plane = assets.add_mesh(MeshData::plane());
        assets.sphere = assets.add_mesh(MeshData::sphere(32, 16));
        assets
    }

    pub fn add_mesh(&mut self, mesh: MeshData) -> MeshId {
        self.meshes.push(mesh);
        MeshId(self.meshes.len() as u32 - 1)
    }

    pub fn cube(&self) -> MeshId {
        self.cube
    }

    pub fn plane(&self) -> MeshId {
        self.plane
    }

    pub fn sphere(&self) -> MeshId {
        self.sphere
    }

    pub(crate) fn meshes(&self) -> &[MeshData] {
        &self.meshes
    }
}
