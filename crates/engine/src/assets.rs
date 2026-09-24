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
        self.meshes.push(MeshSlot { data: mesh, version: 0 });
        MeshId(self.meshes.len() as u32 - 1)
    }

    /// Ersetzt die Geometrie eines Meshs (z. B. jede Frame bei animierten Figuren).
    pub fn update_mesh(&mut self, id: MeshId, mesh: MeshData) {
        let slot = &mut self.meshes[id.0 as usize];
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

    pub(crate) fn mesh_slots(&self) -> &[MeshSlot] {
        &self.meshes
    }

    pub(crate) fn textures(&self) -> &[Image] {
        &self.textures
    }
}
