//! 3D-Modelle aus glTF-Dateien (.glb/.gltf) mit Skelett und Animationen.
//!
//! Ein [`Model`] enthält die Knochen-Hierarchie, Meshes, Texturen und Animationen, so
//! wie sie z. B. aus Blender exportiert werden. Ein [`Animator`] spielt Animationen auf
//! einem Modell ab, blendet weich zwischen ihnen über und erzeugt daraus in jedem Frame
//! die verformte Geometrie (CPU-Skinning).

use std::sync::Arc;

use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

use crate::assets::{Image, TextureId};
use crate::mesh::{MeshData, Vertex};

/// Position, Drehung und Größe eines Knotens relativ zu seinem Elternknoten.
#[derive(Clone, Copy, Debug)]
pub struct Trs {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Trs {
    fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    fn blend(&self, other: &Trs, t: f32) -> Trs {
        Trs {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub rest: Trs,
}

/// Ein Stück Geometrie, das an einem Knoten hängt.
#[derive(Clone, Debug)]
struct Part {
    material: usize,
    node: usize,
    skinned: bool,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<[f32; 2]>,
    /// Grundfarbe des Materials mal Vertex-Farbe (linear), je Eckpunkt.
    colors: Vec<[f32; 3]>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Property {
    Translation,
    Rotation,
    Scale,
}

#[derive(Clone, Debug)]
struct Channel {
    node: usize,
    property: Property,
    times: Vec<f32>,
    /// Werte passend zu `times` (Vektoren in xyz, Quaternionen in xyzw).
    values: Vec<Vec4>,
    step: bool,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub duration: f32,
    channels: Vec<Channel>,
}

/// Oberfläche eines Modellteils, wie sie in der glTF-Datei steht.
#[derive(Clone, Copy, Debug)]
pub struct MaterialInfo {
    /// Index in [`Model::images`] für die Grundfarbe.
    pub image: Option<usize>,
    /// Grundfarbe (linear RGBA), wird mit der Textur multipliziert.
    pub base_color: [f32; 4],
    pub alpha_cutout: bool,
    pub double_sided: bool,
}

impl Default for MaterialInfo {
    fn default() -> Self {
        MaterialInfo { image: None, base_color: [1.0; 4], alpha_cutout: false, double_sided: false }
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub nodes: Vec<Node>,
    parts: Vec<Part>,
    /// (Knoten, inverse Bind-Matrix) je Gelenk des Skeletts.
    joints: Vec<(usize, Mat4)>,
    pub clips: Vec<Clip>,
    pub images: Vec<Image>,
    /// Materialien (Index 0 = Standard, danach wie in der Datei).
    pub materials: Vec<MaterialInfo>,
    /// Reihenfolge, in der Knoten ausgewertet werden (Eltern vor Kindern).
    order: Vec<usize>,
}

impl Model {
    /// Lädt eine glTF-Datei aus dem Speicher (.glb mit eingebetteten Daten).
    pub fn from_glb(bytes: &[u8]) -> Result<Model, String> {
        let (document, buffers, images) = gltf::import_slice(bytes).map_err(|e| format!("glTF nicht lesbar: {e}"))?;
        Self::from_import(document, buffers, images)
    }

    /// Lädt eine glTF-Datei von der Festplatte (.gltf mit .bin und Bildern daneben, oder .glb).
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Model, String> {
        let path = path.as_ref();
        let (document, buffers, images) = gltf::import(path).map_err(|e| format!("{} nicht lesbar: {e}", path.display()))?;
        Self::from_import(document, buffers, images)
    }

    fn from_import(document: gltf::Document, buffers: Vec<gltf::buffer::Data>, images: Vec<gltf::image::Data>) -> Result<Model, String> {
        let mut nodes: Vec<Node> = document
            .nodes()
            .map(|n| {
                let (t, r, s) = n.transform().decomposed();
                Node {
                    name: n.name().unwrap_or("").to_string(),
                    parent: None,
                    rest: Trs { translation: Vec3::from(t), rotation: Quat::from_array(r), scale: Vec3::from(s) },
                }
            })
            .collect();
        for node in document.nodes() {
            for child in node.children() {
                nodes[child.index()].parent = Some(node.index());
            }
        }

        // Material 0 ist der Standard für Teile ohne Material.
        let materials: Vec<MaterialInfo> = std::iter::once(MaterialInfo::default())
            .chain(document.materials().map(|m| {
                let pbr = m.pbr_metallic_roughness();
                MaterialInfo {
                    image: pbr.base_color_texture().map(|t| t.texture().source().index()),
                    base_color: pbr.base_color_factor(),
                    alpha_cutout: m.alpha_mode() == gltf::material::AlphaMode::Mask,
                    double_sided: m.double_sided(),
                }
            }))
            .collect();

        let mut parts = Vec::new();
        for node in document.nodes() {
            let Some(mesh) = node.mesh() else { continue };
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|b| Some(&buffers[b.index()]));
                let positions: Vec<Vec3> = reader.read_positions().map(|p| p.map(Vec3::from).collect()).unwrap_or_default();
                let count = positions.len();
                let normals = reader.read_normals().map(|n| n.map(Vec3::from).collect()).unwrap_or_else(|| vec![Vec3::Y; count]);
                let uvs = reader.read_tex_coords(0).map(|t| t.into_f32().collect()).unwrap_or_else(|| vec![[0.0; 2]; count]);
                let joints: Vec<[u16; 4]> = reader.read_joints(0).map(|j| j.into_u16().collect()).unwrap_or_default();
                let weights = reader.read_weights(0).map(|w| w.into_f32().collect()).unwrap_or_default();
                let indices = reader.read_indices().map(|i| i.into_u32().collect()).unwrap_or_else(|| (0..count as u32).collect());
                let material = primitive.material().index().map_or(0, |i| i + 1);
                let base = materials.get(material).map_or([1.0; 4], |m| m.base_color);
                let colors = match reader.read_colors(0) {
                    Some(c) => c.into_rgb_f32().map(|c| [c[0] * base[0], c[1] * base[1], c[2] * base[2]]).collect(),
                    None => vec![[base[0], base[1], base[2]]; count],
                };
                parts.push(Part {
                    material,
                    node: node.index(),
                    skinned: node.skin().is_some() && joints.len() == count,
                    positions,
                    normals,
                    uvs,
                    colors,
                    joints,
                    weights,
                    indices,
                });
            }
        }

        let joints = document
            .skins()
            .next()
            .map(|skin| {
                let inverse: Vec<Mat4> = skin
                    .reader(|b| Some(&buffers[b.index()]))
                    .read_inverse_bind_matrices()
                    .map(|m| m.map(|m| Mat4::from_cols_array_2d(&m)).collect())
                    .unwrap_or_default();
                skin.joints().enumerate().map(|(i, j)| (j.index(), inverse.get(i).copied().unwrap_or(Mat4::IDENTITY))).collect()
            })
            .unwrap_or_default();

        let clips = document
            .animations()
            .map(|animation| {
                let mut duration = 0.0f32;
                let channels = animation
                    .channels()
                    .filter_map(|channel| {
                        let reader = channel.reader(|b| Some(&buffers[b.index()]));
                        let times: Vec<f32> = reader.read_inputs()?.collect();
                        let (property, values) = match reader.read_outputs()? {
                            gltf::animation::util::ReadOutputs::Translations(t) => (Property::Translation, t.map(|v| Vec3::from(v).extend(0.0)).collect()),
                            gltf::animation::util::ReadOutputs::Rotations(r) => (Property::Rotation, r.into_f32().map(Vec4::from).collect()),
                            gltf::animation::util::ReadOutputs::Scales(s) => (Property::Scale, s.map(|v| Vec3::from(v).extend(0.0)).collect()),
                            gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => return None,
                        };
                        let mut values: Vec<Vec4> = values;
                        // Kubische Splines speichern je Schlüssel (Tangente, Wert, Tangente): nur den Wert nehmen.
                        if channel.sampler().interpolation() == gltf::animation::Interpolation::CubicSpline {
                            values = values.chunks(3).map(|c| c[1]).collect();
                        }
                        duration = duration.max(times.last().copied().unwrap_or(0.0));
                        Some(Channel {
                            node: channel.target().node().index(),
                            property,
                            times,
                            values,
                            step: channel.sampler().interpolation() == gltf::animation::Interpolation::Step,
                        })
                    })
                    .collect();
                Clip { name: animation.name().unwrap_or("").to_string(), duration, channels }
            })
            .collect();

        let images = images.into_iter().map(to_rgba).collect();

        // Eltern immer vor ihren Kindern auswerten.
        let mut order = Vec::with_capacity(nodes.len());
        let mut placed = vec![false; nodes.len()];
        while order.len() < nodes.len() {
            for (i, node) in nodes.iter().enumerate() {
                if !placed[i] && node.parent.is_none_or(|p| placed[p]) {
                    placed[i] = true;
                    order.push(i);
                }
            }
        }

        Ok(Model { nodes, parts, joints, clips, images, materials, order })
    }

    pub fn node(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    pub fn clip(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    fn rest_pose(&self) -> Vec<Trs> {
        self.nodes.iter().map(|n| n.rest).collect()
    }

    /// Wendet eine Animation zum Zeitpunkt `time` auf `pose` an.
    fn sample(&self, clip: usize, time: f32, pose: &mut [Trs]) {
        for channel in &self.clips[clip].channels {
            let value = sample_channel(channel, time);
            let trs = &mut pose[channel.node];
            match channel.property {
                Property::Translation => trs.translation = value.truncate(),
                Property::Rotation => trs.rotation = Quat::from_vec4(value).normalize(),
                Property::Scale => trs.scale = value.truncate(),
            }
        }
    }

    fn global_matrices(&self, pose: &[Trs], out: &mut Vec<Mat4>) {
        out.clear();
        out.resize(self.nodes.len(), Mat4::IDENTITY);
        for &i in &self.order {
            let local = pose[i].matrix();
            out[i] = match self.nodes[i].parent {
                Some(parent) => out[parent] * local,
                None => local,
            };
        }
    }

    /// Geometrie eines einzelnen Knotens (z. B. einer Waffe) in dessen Elternraum, zum
    /// Anbringen an einen anderen Knochen oder ein anderes Modell.
    pub fn extract_part(&self, node_name: &str, texture: Option<TextureId>) -> Option<MeshData> {
        let node = self.node(node_name)?;
        let local = self.nodes[node].rest.matrix();
        let mut mesh = MeshData { texture, ..Default::default() };
        for part in self.parts.iter().filter(|p| p.node == node) {
            append_part(&mut mesh, part, |p| local.transform_point3(p), |n| (Mat3::from_mat4(local) * n).normalize_or_zero());
        }
        (!mesh.vertices.is_empty()).then_some(mesh)
    }

    /// Legt die Bilder des Modells als Texturen an (einmal pro `prefix`) und liefert
    /// deren IDs in der Reihenfolge von [`images`](Self::images).
    pub fn register_textures(&self, assets: &mut crate::assets::Assets, prefix: &str) -> Vec<TextureId> {
        self.images.iter().enumerate().map(|(i, image)| assets.named_texture(&format!("{prefix}#{i}"), || image.clone())).collect()
    }

    /// Unbewegliches Modell (Baum, Fels, Gebäude) in Ruhehaltung – ein Mesh je Material,
    /// damit jedes seine eigene Textur, Beidseitigkeit und Ausschnitt-Maske behält.
    /// `textures` stammen aus [`register_textures`](Self::register_textures).
    pub fn static_meshes(&self, textures: &[TextureId]) -> Vec<MeshData> {
        let mut globals = Vec::new();
        self.global_matrices(&self.rest_pose(), &mut globals);
        let joint_matrices: Vec<Mat4> = self.joints.iter().map(|&(node, inverse)| globals[node] * inverse).collect();

        let mut meshes: Vec<(usize, MeshData)> = Vec::new();
        for part in &self.parts {
            let material = self.materials.get(part.material).copied().unwrap_or_default();
            let index = match meshes.iter().position(|(m, _)| *m == part.material) {
                Some(index) => index,
                None => {
                    meshes.push((
                        part.material,
                        MeshData {
                            texture: material.image.and_then(|i| textures.get(i).copied()),
                            double_sided: material.double_sided,
                            alpha_cutout: material.alpha_cutout,
                            ..Default::default()
                        },
                    ));
                    meshes.len() - 1
                }
            };
            let mesh = &mut meshes[index].1;
            let base = mesh.vertices.len();
            if part.skinned {
                // In Ruhehaltung genügt die Matrix des stärksten Gelenks je Eckpunkt.
                let joint = |i: usize| {
                    let (j, w) = (part.joints[i], part.weights[i]);
                    let strongest = (0..4).max_by(|&a, &b| w[a].total_cmp(&w[b])).unwrap_or(0);
                    joint_matrices.get(j[strongest] as usize).copied().unwrap_or(Mat4::IDENTITY)
                };
                append_part(mesh, part, |p| p, |n| n);
                for i in 0..part.positions.len() {
                    let m = joint(i);
                    let v = &mut mesh.vertices[base + i];
                    v.position = m.transform_point3(part.positions[i]).into();
                    v.normal = (Mat3::from_mat4(m) * part.normals[i]).normalize_or_zero().into();
                }
            } else {
                let m = globals[part.node];
                let normal = Mat3::from_mat4(m).inverse().transpose();
                append_part(mesh, part, |p| m.transform_point3(p), |n| (normal * n).normalize_or_zero());
            }
        }
        meshes.into_iter().map(|(_, mesh)| mesh).collect()
    }

    /// Namen aller Knoten mit starrer Geometrie (Anbauteile wie Helm, Waffen, Schilde).
    pub fn attachments(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.parts.iter().filter(|p| !p.skinned).map(|p| self.nodes[p.node].name.as_str()).collect();
        names.dedup();
        names
    }

    /// Höhe des Modells in Ruhehaltung (für die Skalierung auf eine Wunschgröße).
    pub fn rest_height(&self) -> f32 {
        let animator = Animator::new(Arc::new(self.clone()));
        let mesh = animator.skinned_mesh(None);
        let (min, max) = mesh.vertices.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v.position[1]), hi.max(v.position[1])));
        max - min
    }
}

fn to_rgba(image: gltf::image::Data) -> Image {
    use gltf::image::Format;
    let pixels = image.pixels;
    let rgba = match image.format {
        Format::R8G8B8A8 => pixels,
        Format::R8G8B8 => pixels.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        Format::R8G8 => pixels.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        Format::R8 => pixels.iter().flat_map(|&p| [p, p, p, 255]).collect(),
        other => {
            log::warn!("Bildformat {other:?} wird nicht unterstützt, nehme Weiß");
            vec![255; (image.width * image.height * 4) as usize]
        }
    };
    Image { width: image.width, height: image.height, rgba }
}

fn sample_channel(channel: &Channel, time: f32) -> Vec4 {
    let times = &channel.times;
    if times.len() == 1 || time <= times[0] {
        return channel.values[0];
    }
    let last = times.len() - 1;
    if time >= times[last] {
        return channel.values[last];
    }
    let next = times.partition_point(|&t| t <= time);
    let prev = next - 1;
    if channel.step {
        return channel.values[prev];
    }
    let t = (time - times[prev]) / (times[next] - times[prev]);
    let (a, b) = (channel.values[prev], channel.values[next]);
    if channel.property == Property::Rotation {
        Quat::from_vec4(a).slerp(Quat::from_vec4(b), t).into()
    } else {
        a.lerp(b, t)
    }
}

fn append_part(mesh: &mut MeshData, part: &Part, position: impl Fn(Vec3) -> Vec3, normal: impl Fn(Vec3) -> Vec3) {
    let base = mesh.vertices.len() as u32;
    for i in 0..part.positions.len() {
        mesh.vertices.push(Vertex {
            position: position(part.positions[i]).into(),
            normal: normal(part.normals[i]).into(),
            color: part.colors[i],
            uv: part.uvs[i],
        });
    }
    mesh.indices.extend(part.indices.iter().map(|i| i + base));
}

#[derive(Clone, Copy, Debug)]
struct Playback {
    clip: usize,
    time: f32,
    speed: f32,
    looping: bool,
}

/// Spielt Animationen auf einem Modell ab.
pub struct Animator {
    model: Arc<Model>,
    current: Option<Playback>,
    /// Vorherige Animation während des Überblendens: (Wiedergabe, vergangen, Dauer).
    fading: Option<(Playback, f32, f32)>,
    hidden: Vec<bool>,
    pose: Vec<Trs>,
    fade_pose: Vec<Trs>,
    globals: Vec<Mat4>,
}

impl Animator {
    pub fn new(model: Arc<Model>) -> Self {
        let pose = model.rest_pose();
        let mut animator = Animator {
            hidden: vec![false; model.nodes.len()],
            fade_pose: pose.clone(),
            pose,
            globals: Vec::new(),
            current: None,
            fading: None,
            model,
        };
        animator.evaluate();
        animator
    }

    pub fn model(&self) -> &Arc<Model> {
        &self.model
    }

    /// Startet eine Animation. Läuft sie schon (und schleift), passiert nichts.
    /// `fade` = Dauer des Übergangs in Sekunden. Liefert `false`, wenn es sie nicht gibt.
    pub fn play(&mut self, name: &str, looping: bool, fade: f32) -> bool {
        let Some(clip) = self.model.clip(name) else { return false };
        if let Some(current) = &self.current {
            if current.clip == clip && looping && current.looping {
                return true;
            }
        }
        if let Some(previous) = self.current.take() {
            self.fading = (fade > 0.0).then_some((previous, 0.0, fade));
        }
        self.current = Some(Playback { clip, time: 0.0, speed: 1.0, looping });
        true
    }

    /// Abspielgeschwindigkeit der aktuellen Animation (1 = normal).
    pub fn set_speed(&mut self, speed: f32) {
        if let Some(current) = &mut self.current {
            current.speed = speed;
        }
    }

    /// Name der aktuellen Animation.
    pub fn current(&self) -> Option<&str> {
        self.current.map(|c| self.model.clips[c.clip].name.as_str())
    }

    /// Ist eine einmalige Animation zu Ende gespielt?
    pub fn finished(&self) -> bool {
        self.current.is_none_or(|c| !c.looping && c.time >= self.model.clips[c.clip].duration)
    }

    /// Blendet einen Knoten mit allem, was daran hängt, aus oder ein.
    pub fn set_visible(&mut self, node_name: &str, visible: bool) {
        if let Some(node) = self.model.node(node_name) {
            self.hidden[node] = !visible;
        }
    }

    pub fn update(&mut self, dt: f32) {
        let model = self.model.clone();
        let advance = |p: &mut Playback| {
            let duration = model.clips[p.clip].duration.max(1e-4);
            p.time += dt * p.speed;
            if p.looping {
                p.time %= duration;
            } else {
                p.time = p.time.min(duration);
            }
        };
        if let Some(current) = &mut self.current {
            advance(current);
        }
        if let Some((previous, elapsed, duration)) = &mut self.fading {
            advance(previous);
            *elapsed += dt;
            if *elapsed >= *duration {
                self.fading = None;
            }
        }
        self.evaluate();
    }

    fn evaluate(&mut self) {
        let model = &self.model;
        self.pose.clone_from_slice(&model.nodes.iter().map(|n| n.rest).collect::<Vec<_>>());
        if let Some(current) = self.current {
            model.sample(current.clip, current.time, &mut self.pose);
        }
        if let Some((previous, elapsed, duration)) = self.fading {
            for (slot, node) in self.fade_pose.iter_mut().zip(&model.nodes) {
                *slot = node.rest;
            }
            model.sample(previous.clip, previous.time, &mut self.fade_pose);
            let t = (elapsed / duration).clamp(0.0, 1.0);
            for (target, from) in self.pose.iter_mut().zip(&self.fade_pose) {
                *target = from.blend(target, t);
            }
        }
        model.global_matrices(&self.pose, &mut self.globals);
    }

    /// Lage eines Knotens (z. B. eines Hand-Knochens) im Raum des Modells.
    pub fn node_matrix(&self, name: &str) -> Option<Mat4> {
        self.model.node(name).map(|i| self.globals[i])
    }

    fn is_hidden(&self, mut node: usize) -> bool {
        loop {
            if self.hidden[node] {
                return true;
            }
            match self.model.nodes[node].parent {
                Some(parent) => node = parent,
                None => return false,
            }
        }
    }

    /// Verformte Geometrie in der aktuellen Pose (Raum des Modells).
    pub fn skinned_mesh(&self, texture: Option<TextureId>) -> MeshData {
        let joint_matrices: Vec<Mat4> = self.model.joints.iter().map(|&(node, inverse)| self.globals[node] * inverse).collect();
        let mut mesh = MeshData { texture, ..Default::default() };
        for part in &self.model.parts {
            if self.is_hidden(part.node) {
                continue;
            }
            if part.skinned {
                let base = mesh.vertices.len() as u32;
                for i in 0..part.positions.len() {
                    let (j, w) = (part.joints[i], part.weights[i]);
                    let mut m = Mat4::ZERO;
                    for k in 0..4 {
                        if w[k] > 0.0 {
                            m += joint_matrices.get(j[k] as usize).copied().unwrap_or(Mat4::IDENTITY) * w[k];
                        }
                    }
                    mesh.vertices.push(Vertex {
                        position: m.transform_point3(part.positions[i]).into(),
                        normal: (Mat3::from_mat4(m) * part.normals[i]).normalize_or_zero().into(),
                        color: part.colors[i],
                        uv: part.uvs[i],
                    });
                }
                mesh.indices.extend(part.indices.iter().map(|i| i + base));
            } else {
                let m = self.globals[part.node];
                let normal = Mat3::from_mat4(m).inverse().transpose();
                append_part(&mut mesh, part, |p| m.transform_point3(p), |n| (normal * n).normalize_or_zero());
            }
        }
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNIGHT: &[u8] = include_bytes!("../../../game/assets/characters/Knight.glb");
    const BARBARIAN: &[u8] = include_bytes!("../../../game/assets/characters/Barbarian.glb");

    #[test]
    fn ritter_laden_und_animieren() {
        let model = Arc::new(Model::from_glb(KNIGHT).expect("Ritter nicht lesbar"));
        assert!(model.clips.len() >= 70, "nur {} Animationen", model.clips.len());
        assert_eq!(model.images.len(), 1);
        let height = model.rest_height();
        assert!((1.5..3.5).contains(&height), "Höhe {height}");

        let mut animator = Animator::new(model.clone());
        let rest = animator.skinned_mesh(None);
        assert!(rest.vertices.len() > 1000);

        assert!(animator.play("Running_A", true, 0.2));
        assert!(!animator.play("Gibt_es_nicht", true, 0.2));
        animator.update(0.3);
        let running = animator.skinned_mesh(None);
        assert_eq!(running.vertices.len(), rest.vertices.len());
        let moved = rest.vertices.iter().zip(&running.vertices).filter(|(a, b)| Vec3::from(a.position).distance(b.position.into()) > 0.01).count();
        assert!(moved > rest.vertices.len() / 4, "Die Animation bewegt kaum etwas: {moved}");

        assert!(animator.node_matrix("handslot.r").is_some());
        animator.set_visible("Knight_Helmet", false);
        assert!(animator.skinned_mesh(None).vertices.len() < running.vertices.len(), "Helm lässt sich nicht ausblenden");

        assert!(animator.play("1H_Melee_Attack_Chop", false, 0.1));
        assert!(!animator.finished());
        for _ in 0..100 {
            animator.update(0.05);
        }
        assert!(animator.finished());
    }

    #[test]
    fn axt_aus_dem_barbaren_herausloesen() {
        let model = Model::from_glb(BARBARIAN).unwrap();
        let axe = model.extract_part("1H_Axe", None).expect("Keine Axt gefunden");
        assert!(axe.vertices.len() > 20);
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;

    #[test]
    fn gltf_mit_externen_dateien_laden_und_backen() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/assets/characters/axe_1handed.gltf");
        let model = Model::from_file(path).expect("Axt nicht lesbar");
        assert_eq!(model.images.len(), 1, "Textur aus der PNG-Datei fehlt");
        assert!(model.materials.len() >= 2, "Material aus der Datei fehlt");

        let mut assets = crate::assets::Assets::new();
        let textures = model.register_textures(&mut assets, "axt");
        let again = model.register_textures(&mut assets, "axt");
        assert_eq!(textures, again, "Texturen dürfen nicht doppelt angelegt werden");

        let meshes = model.static_meshes(&textures);
        assert!(!meshes.is_empty());
        assert!(meshes.iter().all(|m| m.texture == Some(textures[0])));
        assert!(meshes.iter().map(|m| m.vertices.len()).sum::<usize>() > 20);
    }

    /// So exportiert Blender flach eingefärbte Modelle: Materialfarbe plus Vertex-Farben.
    #[test]
    fn materialfarbe_und_vertexfarben_uebernehmen() {
        let dir = std::env::temp_dir().join(format!("engine_farbtest_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut bin = Vec::new();
        for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.5, 0.25], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]] {
            for x in v {
                bin.extend_from_slice(&x.to_le_bytes());
            }
        }
        std::fs::write(dir.join("dreieck.bin"), &bin).unwrap();
        let json = r#"{
            "asset": {"version": "2.0"},
            "scene": 0, "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "COLOR_0": 1}, "material": 0}]}],
            "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0.5, 1.0, 1.0, 1.0]}, "doubleSided": true}],
            "buffers": [{"uri": "dreieck.bin", "byteLength": 72}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}, {"buffer": 0, "byteOffset": 36, "byteLength": 36}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 1, 0]},
                {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}
            ]
        }"#;
        std::fs::write(dir.join("dreieck.gltf"), json).unwrap();

        let model = Model::from_file(dir.join("dreieck.gltf")).expect("Dreieck nicht lesbar");
        let meshes = model.static_meshes(&[]);
        assert_eq!(meshes.len(), 1);
        assert!(meshes[0].double_sided);
        assert_eq!(meshes[0].vertices[0].color, [0.5, 0.5, 0.25]);
        assert_eq!(meshes[0].vertices[2].color, [0.0, 0.0, 1.0]);
        // Animierte Modelle bekommen dieselben Farben.
        let skinned = Animator::new(Arc::new(model)).skinned_mesh(None);
        assert_eq!(skinned.vertices[1].color, [0.5, 1.0, 1.0]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
