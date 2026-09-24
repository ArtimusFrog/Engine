use std::collections::HashMap;

use crate::mesh::MeshData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeshId(pub(crate) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureId(pub(crate) u32);

/// Bild im Arbeitsspeicher, 8 Bit RGBA, Farben im sRGB-Farbraum (wie in PNG-Dateien).
#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub(crate) struct MeshSlot {
    pub data: MeshData,
    /// Hüllkugel im Raum des Meshs (Mittelpunkt, Radius).
    pub bounds: (glam::Vec3, f32),
    /// Wird bei jeder Änderung erhöht, damit der Renderer neu hochlädt.
    pub version: u64,
}

/// Verwaltet alle Ressourcen des Spiels. Hängt nicht von der Grafikkarte ab,
/// damit später auch ein Server ohne Fenster dieselben Daten laden kann.
pub struct Assets {
    meshes: Vec<MeshSlot>,
    textures: Vec<Image>,
    named: HashMap<String, MeshId>,
    named_textures: HashMap<String, TextureId>,
    /// Detailstufen je Mesh, nach Entfernung sortiert.
    lods: HashMap<MeshId, Vec<Lod>>,
    cube: MeshId,
    plane: MeshId,
    sphere: MeshId,
}

impl Assets {
    pub(crate) fn new() -> Self {
        let mut assets = Assets {
            meshes: Vec::new(),
            textures: Vec::new(),
            named: HashMap::new(),
            named_textures: HashMap::new(),
            lods: HashMap::new(),
            cube: MeshId(0),
            plane: MeshId(0),
            sphere: MeshId(0),
        };
        assets.cube = assets.add_mesh(MeshData::cube());
        assets.plane = assets.add_mesh(MeshData::plane());
        assets.sphere = assets.add_mesh(MeshData::sphere(32, 16));
        assets
    }

    pub fn add_mesh(&mut self, mesh: MeshData) -> MeshId {
        let bounds = mesh.bounds();
        self.meshes.push(MeshSlot { data: mesh, bounds, version: 0 });
        MeshId(self.meshes.len() as u32 - 1)
    }

    /// Ersetzt die Geometrie eines Meshs (z. B. jede Frame bei animierten Figuren).
    pub fn update_mesh(&mut self, id: MeshId, mesh: MeshData) {
        let slot = &mut self.meshes[id.0 as usize];
        slot.bounds = mesh.bounds();
        slot.data = mesh;
        slot.version += 1;
    }

    pub fn mesh(&self, id: MeshId) -> &MeshData {
        &self.meshes[id.0 as usize].data
    }

    /// Liefert das Mesh mit diesem Namen; beim ersten Mal wird es mit `build` erzeugt.
    /// So entstehen aufwendige Modelle nur einmal, auch wenn die Welt neu aufgebaut wird.
    pub fn named_mesh(&mut self, name: &str, build: impl FnOnce() -> MeshData) -> MeshId {
        if let Some(&id) = self.named.get(name) {
            return id;
        }
        let id = self.add_mesh(build());
        self.named.insert(name.to_string(), id);
        id
    }

    /// Das Mesh mit diesem Namen, falls es schon angelegt wurde.
    pub fn find_mesh(&self, name: &str) -> Option<MeshId> {
        self.named.get(name).copied()
    }

    /// Wie [`named_mesh`](Self::named_mesh), nur für Texturen.
    pub fn named_texture(&mut self, name: &str, build: impl FnOnce() -> Image) -> TextureId {
        if let Some(&id) = self.named_textures.get(name) {
            return id;
        }
        let id = self.add_texture(build());
        self.named_textures.insert(name.to_string(), id);
        id
    }

    pub fn add_texture(&mut self, image: Image) -> TextureId {
        self.textures.push(image);
        TextureId(self.textures.len() as u32 - 1)
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

    /// Legt fest, welches Mesh ab welcher Entfernung zur Kamera statt `mesh` gezeichnet
    /// wird (`None` = gar nicht mehr). Gilt für alle Objekte mit diesem Mesh.
    pub fn set_lods(&mut self, mesh: MeshId, mut levels: Vec<Lod>) {
        levels.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        self.lods.insert(mesh, levels);
    }

    /// Hat dieses Mesh schon Detailstufen?
    pub fn has_lods(&self, mesh: MeshId) -> bool {
        self.lods.contains_key(&mesh)
    }

    /// Das Mesh, das in `distance` Metern Entfernung gezeichnet wird.
    pub fn mesh_at_distance(&self, mesh: MeshId, distance: f32) -> Option<MeshId> {
        match self.lods.get(&mesh).and_then(|levels| levels.iter().rev().find(|l| distance >= l.distance)) {
            Some(level) => level.mesh,
            None => Some(mesh),
        }
    }

    pub(crate) fn mesh_slots(&self) -> &[MeshSlot] {
        &self.meshes
    }

    pub(crate) fn textures(&self) -> &[Image] {
        &self.textures
    }
}

/// Eine Detailstufe: ab `distance` Metern wird `mesh` gezeichnet (`None` = ausgeblendet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lod {
    pub distance: f32,
    pub mesh: Option<MeshId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detailstufen_nach_entfernung() {
        let mut assets = Assets::new();
        let full = assets.add_mesh(MeshData::icosphere(2, glam::Vec3::ONE));
        let coarse = assets.add_mesh(MeshData::icosphere(0, glam::Vec3::ONE));
        assets.set_lods(full, vec![Lod { distance: 120.0, mesh: None }, Lod { distance: 40.0, mesh: Some(coarse) }]);
        assert_eq!(assets.mesh_at_distance(full, 10.0), Some(full));
        assert_eq!(assets.mesh_at_distance(full, 50.0), Some(coarse));
        assert_eq!(assets.mesh_at_distance(full, 500.0), None);
        assert_eq!(assets.mesh_at_distance(coarse, 500.0), Some(coarse));
    }
}
