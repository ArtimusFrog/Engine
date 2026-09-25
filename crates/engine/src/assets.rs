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

impl Image {
    /// Liest eine PNG-Datei (z. B. Symbole für die Benutzeroberfläche).
    pub fn load_png(path: &std::path::Path) -> Result<Image, String> {
        let image = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?.to_rgba8();
        Ok(Image { width: image.width(), height: image.height(), rgba: image.into_raw() })
    }

    /// Alle Mipmap-Stufen (Stufe 0 = das Bild selbst), für ruhige Texturen in der Ferne.
    ///
    /// Für Ausschnitt-Texturen (Blätter, Gras) zweierlei Besonderheiten:
    /// - durchsichtige Pixel bekommen die Farbe ihrer Nachbarn, sonst entstehen dunkle Ränder
    /// - die Deckkraft jeder Stufe wird so skaliert, dass gleich viel Fläche „stehen bleibt“ –
    ///   sonst lösen sich Baumkronen in der Ferne auf
    pub fn mip_chain(&self) -> Vec<Image> {
        let mut base = self.clone();
        let cutout = base.rgba.chunks_exact(4).any(|p| p[3] < 255);
        if cutout {
            base.bleed_colors(8);
        }
        let coverage = base.coverage(1.0);
        let mut levels = vec![base];
        while levels.last().is_some_and(|l| l.width > 1 || l.height > 1) {
            let mut next = levels.last().unwrap().half();
            if cutout {
                next.keep_coverage(coverage);
            }
            levels.push(next);
        }
        levels
    }

    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y.min(self.height - 1) * self.width + x.min(self.width - 1)) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// Halbe Größe; Farben nach Deckkraft gewichtet und in linearem Licht gemittelt.
    fn half(&self) -> Image {
        let (width, height) = ((self.width / 2).max(1), (self.height / 2).max(1));
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let mut color = [0.0f32; 3];
                let (mut weight, mut alpha) = (0.0f32, 0.0f32);
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = self.pixel(x * 2 + dx, y * 2 + dy);
                    let a = p[3] as f32 / 255.0;
                    let w = a.max(0.001);
                    for c in 0..3 {
                        color[c] += srgb_to_linear(p[c]) * w;
                    }
                    weight += w;
                    alpha += a;
                }
                rgba.extend(color.map(|c| linear_to_srgb(c / weight)));
                rgba.push((alpha / 4.0 * 255.0).round() as u8);
            }
        }
        Image { width, height, rgba }
    }

    /// Anteil der Pixel, die nach Skalierung der Deckkraft mit `scale` noch gezeichnet werden.
    fn coverage(&self, scale: f32) -> f32 {
        let visible = self.rgba.chunks_exact(4).filter(|p| p[3] as f32 * scale >= 127.5).count();
        visible as f32 / (self.rgba.len() / 4) as f32
    }

    fn keep_coverage(&mut self, target: f32) {
        let (mut low, mut high) = (0.0f32, 8.0f32);
        for _ in 0..16 {
            let mid = (low + high) / 2.0;
            if self.coverage(mid) < target {
                low = mid;
            } else {
                high = mid;
            }
        }
        for p in self.rgba.chunks_exact_mut(4) {
            p[3] = (p[3] as f32 * high).ceil().min(255.0) as u8;
        }
    }

    /// Färbt durchsichtige Pixel in `rounds` Schritten mit der Farbe sichtbarer Nachbarn.
    fn bleed_colors(&mut self, rounds: u32) {
        let (w, h) = (self.width as i32, self.height as i32);
        let mut filled: Vec<bool> = self.rgba.chunks_exact(4).map(|p| p[3] > 0).collect();
        for _ in 0..rounds {
            let mut updates = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) as usize;
                    if filled[i] {
                        continue;
                    }
                    let (mut sum, mut count) = ([0u32; 3], 0);
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let n = (ny * w + nx) as usize;
                        if filled[n] {
                            for c in 0..3 {
                                sum[c] += self.rgba[n * 4 + c] as u32;
                            }
                            count += 1;
                        }
                    }
                    if count > 0 {
                        updates.push((i, sum.map(|s| (s / count) as u8)));
                    }
                }
            }
            if updates.is_empty() {
                break;
            }
            for (i, color) in updates {
                self.rgba[i * 4..i * 4 + 3].copy_from_slice(&color);
                filled[i] = true;
            }
        }
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    (c as f32 / 255.0).powf(2.2)
}

fn linear_to_srgb(c: f32) -> u8 {
    (c.max(0.0).powf(1.0 / 2.2) * 255.0).round().min(255.0) as u8
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
    fn mipmaps_behalten_die_blattflaeche() {
        // Schachbrett aus einzelnen Pixeln: 25 % deckend – ohne Ausgleich wäre ab Stufe 1 alles durchsichtig.
        let (w, h) = (64u32, 64u32);
        let rgba = (0..w * h).flat_map(|i| if (i % w) % 2 == 0 && (i / w) % 2 == 0 { [40, 200, 40, 255] } else { [0, 0, 0, 0] }).collect();
        let levels = Image { width: w, height: h, rgba }.mip_chain();
        assert_eq!(levels.len(), 7);
        assert_eq!((levels[6].width, levels[6].height), (1, 1));
        for level in &levels[1..4] {
            let c = level.coverage(1.0);
            assert!(c > 0.15, "Stufe {}x{}: nur {c} deckend", level.width, level.height);
            // Kein Schwarz an den Rändern: Farbe bleibt grün
            assert!(level.rgba[1] > 150 && level.rgba[0] < 90, "{:?}", &level.rgba[..4]);
        }
    }

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
