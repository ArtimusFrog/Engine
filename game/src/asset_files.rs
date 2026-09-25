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
    // Sauberer absoluter Pfad (ohne „..“), damit Pfadvergleiche stimmen.
    candidates.into_iter().find(|dir| dir.is_dir()).map(|dir| std::path::absolute(&dir).unwrap_or(dir))
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
        let textures = model.register_textures(assets);
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
    // In der Asset-Galerie markierte Modelle haben Vorrang (Entwickler-Test).
    let marked = crate::markierungen::fuer_platz(name);
    let paths = if marked.is_empty() { variants(folder, name) } else { marked };
    for path in paths {
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

/// Regeln für Assets je Ordner (siehe art/README.md). Andere Ordner werden nicht geprüft.
struct Rules {
    max_triangles: usize,
    required_clips: &'static [&'static str],
}

fn rules_for(path: &Path) -> Option<Rules> {
    let folder = path.parent()?.file_name()?.to_str()?;
    match folder {
        "natur" => Some(Rules { max_triangles: 5000, required_clips: &[] }),
        "tiere" => Some(Rules { max_triangles: 3000, required_clips: &["Idle", "Laufen", "Rennen"] }),
        "figuren" => Some(Rules { max_triangles: 50000, required_clips: &["Idle", "Laufen", "Rennen", "Springen", "Hieb", "Werfen", "Zaubern", "Abbauen", "Hacken"] }),
        "gebaeude" | "gegenstaende" => Some(Rules { max_triangles: 4000, required_clips: &[] }),
        // Große Bauwerke (Burg): ein Wahrzeichen, dafür viele Details
        "bauwerke" => Some(Rules { max_triangles: 320_000, required_clips: &[] }),
        _ => None,
    }
}

/// Prüft ein geladenes Modell gegen die Regeln seines Ordners. Leer = alles in Ordnung.
pub fn check_model(path: &Path, model: &Model) -> Vec<String> {
    let Some(rules) = rules_for(path) else { return Vec::new() };
    let mut problems = Vec::new();
    let meshes = if model.clips.is_empty() {
        model.static_meshes(&[])
    } else {
        vec![Animator::new(std::sync::Arc::new(model.clone())).skinned_mesh(None)]
    };
    let triangles: usize = meshes.iter().map(|m| m.indices.len() / 3).sum();
    if triangles == 0 {
        problems.push("enthält keine Geometrie".into());
    }
    if triangles > rules.max_triangles {
        problems.push(format!("{triangles} Dreiecke, erlaubt sind {}", rules.max_triangles));
    }
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in meshes.iter().flat_map(|m| &m.vertices) {
        min = min.min(v.position.into());
        max = max.max(v.position.into());
    }
    if triangles > 0 {
        if min.y.abs() > 0.05 {
            problems.push(format!("Ursprung nicht am Boden (Unterkante bei {:.2} m)", min.y));
        }
        let center = (min + max) * 0.5;
        let size = (max - min).max_element();
        if vec2(center.x, center.z).length() > size * 0.5 + 0.1 {
            problems.push(format!("Ursprung nicht unter dem Modell (Mitte bei {:.2}, {:.2})", center.x, center.z));
        }
    }
    for clip in rules.required_clips {
        if model.clip(clip).is_none() {
            problems.push(format!("Animation „{clip}“ fehlt"));
        }
    }
    problems
}

/// Alle Modelldateien unter einem Ordner (rekursiv).
pub fn all_models(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(all_models(&path));
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("gltf" | "glb")) {
            found.push(path);
        }
    }
    found.sort();
    found
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

    /// Jedes Asset im Repo lässt sich laden und hält die Regeln ein.
    #[test]
    fn alle_assets_halten_die_regeln() {
        let dir = asset_dir().expect("Asset-Ordner fehlt");
        let mut problems = Vec::new();
        for path in all_models(&dir) {
            match Model::from_file(&path) {
                Ok(model) => problems.extend(check_model(&path, &model).into_iter().map(|p| format!("{}: {p}", path.display()))),
                Err(message) => problems.push(message),
            }
        }
        assert!(problems.is_empty(), "Asset-Regeln verletzt:\n{}", problems.join("\n"));
    }

    #[test]
    fn regeln_greifen() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../crates/engine/testdaten/axe_1handed.gltf");
        let model = Model::from_file(path).unwrap();
        assert!(check_model(Path::new(path), &model).is_empty(), "Testdaten werden nicht geprüft");
        let problems = check_model(Path::new("assets/tiere/axt.gltf"), &model);
        assert!(problems.iter().any(|p| p.contains("Boden")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("Idle")), "{problems:?}");
    }
}
