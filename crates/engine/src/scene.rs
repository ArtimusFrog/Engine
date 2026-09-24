use glam::{Mat4, Quat, Vec3, Vec4};

use crate::assets::MeshId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Transform { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }
}

impl Transform {
    pub fn from_position(position: Vec3) -> Self {
        Transform { position, ..Default::default() }
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// Wie ein Objekt aussieht, zusätzlich zu Farbe und Form.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Material {
    /// Normal beleuchtet, mit Schatten.
    #[default]
    Standard,
    /// Wasseroberfläche mit Wellen und Glanz (am besten mit `MeshData::grid`).
    Water,
    /// Blätter und Gras, die sich im Wind wiegen. `sway` = Stärke (≈ 0.05–0.3).
    Foliage { sway: f32 },
    /// Laub aus Blattkarten mit weichen (kugelförmigen) Normalen aus Blender: sanftes Licht,
    /// scheint gegen die Sonne durch, wiegt sich im Wind wie `Foliage`.
    Leaves { sway: f32 },
    /// Leuchtet von selbst (Kristalle, magische Pilze). `glow` = Helligkeit (≈ 0.5–3).
    Emissive { glow: f32 },
}

impl Material {
    /// Kodierung für den Shader: (Art, Parameter).
    pub(crate) fn shader_params(self) -> [f32; 4] {
        match self {
            Material::Standard => [0.0, 0.0, 0.0, 0.0],
            Material::Water => [1.0, 0.0, 0.0, 0.0],
            Material::Foliage { sway } => [2.0, sway, 0.0, 0.0],
            Material::Leaves { sway } => [2.0, sway, 1.0, 0.0],
            Material::Emissive { glow } => [3.0, glow, 0.0, 0.0],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub name: String,
    pub transform: Transform,
    pub mesh: MeshId,
    /// Grundfarbe in linearem RGB, Alpha derzeit ungenutzt.
    pub color: Vec4,
    pub visible: bool,
    pub material: Material,
    /// Hängt das Objekt an ein anderes: `transform` ist dann relativ zum Elternobjekt.
    pub parent: Option<EntityId>,
}

impl Entity {
    pub fn new(name: impl Into<String>, mesh: MeshId) -> Self {
        Entity { name: name.into(), transform: Transform::default(), mesh, color: Vec4::ONE, visible: true, material: Material::Standard, parent: None }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }

    pub fn with_material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    pub fn with_parent(mut self, parent: EntityId) -> Self {
        self.parent = Some(parent);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(usize);

impl EntityId {
    pub(crate) fn index(self) -> usize {
        self.0
    }

    pub(crate) fn from_index(index: usize) -> Self {
        EntityId(index)
    }
}

/// Alle Objekte der Spielwelt.
///
/// Gelöschte Objekte hinterlassen eine Lücke, damit die IDs der anderen gültig bleiben.
#[derive(Default)]
pub struct Scene {
    entities: Vec<Option<Entity>>,
    count: usize,
}

impl Scene {
    pub fn spawn(&mut self, entity: Entity) -> EntityId {
        self.entities.push(Some(entity));
        self.count += 1;
        EntityId(self.entities.len() - 1)
    }

    /// Löscht ein Objekt samt allen Objekten, die daran hängen.
    pub fn despawn(&mut self, id: EntityId) {
        if self.entities.get_mut(id.0).and_then(Option::take).is_none() {
            return;
        }
        self.count -= 1;
        let children: Vec<_> = self
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.as_ref().is_some_and(|e| e.parent == Some(id)))
            .map(|(i, _)| EntityId(i))
            .collect();
        for child in children {
            self.despawn(child);
        }
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.get(id.0).is_some_and(Option::is_some)
    }

    /// Panics, wenn das Objekt gelöscht wurde; siehe [`try_get`](Self::try_get).
    pub fn get(&self, id: EntityId) -> &Entity {
        self.try_get(id).expect("Objekt wurde gelöscht")
    }

    pub fn get_mut(&mut self, id: EntityId) -> &mut Entity {
        self.try_get_mut(id).expect("Objekt wurde gelöscht")
    }

    pub fn try_get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(id.0)?.as_ref()
    }

    pub fn try_get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(id.0)?.as_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter().flatten()
    }

    /// Transformation in Weltkoordinaten, inklusive aller Elternobjekte.
    pub fn world_matrix(&self, id: EntityId) -> Mat4 {
        let Some(entity) = self.try_get(id) else { return Mat4::IDENTITY };
        match entity.parent {
            Some(parent) => self.world_matrix(parent) * entity.transform.matrix(),
            None => entity.transform.matrix(),
        }
    }

    /// Alle Objekte zusammen mit ihrer Weltmatrix.
    pub fn iter_world(&self) -> impl Iterator<Item = (&Entity, Mat4)> {
        self.entities
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|e| (e, self.world_matrix(EntityId(i)))))
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
