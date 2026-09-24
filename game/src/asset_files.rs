//! Modelle aus `game/assets/` (aus Blender exportiert) statt aus Code.
//!
//! Liegt z. B. `natur/eiche.gltf` oder `natur/eiche_1.gltf`, `natur/eiche_2.gltf` im
//! Asset-Ordner, nimmt die Insel diese Varianten statt der eingebauten Eichen.

use std::path::{Path, PathBuf};

use engine::prelude::*;

/// Der Asset-Ordner: neben der .exe (`assets/`), sonst im Repo (`game/assets/`).
pub fn asset_dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let mut candidates = Vec::new();
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join("assets"));
        // target/debug oder target/release → Repo
        candidates.push(dir.join("..").join("..").join("game").join("assets"));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
    candidates.push(PathBuf::from("game").join("assets"));
    candidates.into_iter().find(|dir| dir.is_dir())
}

/// Gehört die Datei zu den Varianten von `name` (`name.gltf`, `name_3.glb`, …)?
fn is_variant(path: &Path, name: &str) -> bool {
    let is_model = matches!(path.extension().and_then(|e| e.to_str()), Some("gltf" | "glb"));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let numbered = stem.strip_prefix(name).and_then(|rest| rest.strip_prefix('_')).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    is_model && (stem == name || numbered)
}

/// Alle Varianten eines Modells, sortiert: `<ordner>/<name>.gltf` und `<ordner>/<name>_<zahl>.gltf`.
pub fn variants(folder: &str, name: &str) -> Vec<PathBuf> {
    let Some(dir) = asset_dir().map(|d| d.join(folder)) else { return Vec::new() };
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_variant(path, name))
        .collect();
    found.sort();
    found
}

/// Ein unbewegliches Modell als ein einziges Mesh, plus die leuchtenden Teile getrennt
/// (Materialien mit „Emission“ in Blender), damit sie nachts glühen können.
pub struct StaticModel {
    pub mesh: MeshData,
    pub glow: Option<MeshData>,
}

impl StaticModel {
    pub fn load(assets: &mut Assets, path: &Path) -> Result<StaticModel, String> {
        let model = Model::from_file(path)?;
        let textures = model.register_textures(assets, &path.to_string_lossy());
        let mut mesh = MeshData::default();
        let mut glow = MeshData::default();
        for (material, part) in model.static_mesh_groups(&textures) {
            let target = if material.emissive > 0.0 { &mut glow } else { &mut mesh };
            if target.texture.is_some() && part.texture.is_some() && target.texture != part.texture {
                log::warn!("{}: mehr als eine Textur – nur die erste wird benutzt", path.display());
            }
            target.texture = target.texture.or(part.texture);
            target.double_sided |= part.double_sided;
            target.alpha_cutout |= part.alpha_cutout;
            target.append(&part, Mat4::IDENTITY);
        }
        if mesh.vertices.is_empty() && glow.vertices.is_empty() {
            return Err(format!("{} enthält keine Geometrie", path.display()));
        }
        Ok(StaticModel { mesh, glow: (!glow.vertices.is_empty()).then_some(glow) })
    }

    /// Passt das Modell an den Maßstab an, in dem die Insel es aufstellt (Teilen durch
    /// `scale`), und senkt es um `sink` Meter ab, damit es an Hängen nicht schwebt.
    pub fn normalized(mut self, scale: Vec3, sink: f32) -> Self {
        let adjust = |mesh: MeshData| mesh.displace(|p| p / scale - Vec3::Y * sink / scale.y);
        self.mesh = adjust(self.mesh);
        self.glow = self.glow.map(adjust);
        self
    }
}

/// Meshes (und leuchtende Teile) aller Varianten eines Modells; leer, wenn es keine Datei gibt.
/// Jede Datei wird nur einmal pro Programmlauf gelesen.
pub fn load_variants(ctx: &mut Context, folder: &str, name: &str, scale: Vec3, sink: f32) -> Vec<(MeshId, Option<MeshId>)> {
    let mut result = Vec::new();
    for path in variants(folder, name) {
        let key = format!("datei:{}", path.display());
        let glow_key = format!("{key}#leuchten");
        if let Some(mesh) = ctx.assets.find_mesh(&key) {
            result.push((mesh, ctx.assets.find_mesh(&glow_key)));
            continue;
        }
        match StaticModel::load(&mut ctx.assets, &path) {
            Ok(model) => {
                let model = model.normalized(scale, sink);
                let mesh = ctx.assets.named_mesh(&key, || model.mesh);
                let glow = model.glow.map(|g| ctx.assets.named_mesh(&glow_key, || g));
                log::info!("Modell {} geladen", path.display());
                result.push((mesh, glow));
            }
            Err(message) => log::warn!("{message} – nehme das eingebaute Modell"),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varianten_erkennen() {
        for yes in ["eiche.gltf", "eiche_1.gltf", "eiche_12.glb"] {
            assert!(is_variant(Path::new(yes), "eiche"), "{yes}");
        }
        for no in ["eiche_gross.gltf", "eiche_1.bin", "zaubereiche.gltf", "eiche_.gltf"] {
            assert!(!is_variant(Path::new(no), "eiche"), "{no}");
        }
    }
}
