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

#[derive(Clone, Debug)]
pub struct Entity {
    pub name: String,
    pub transform: Transform,
    pub mesh: MeshId,
    /// Grundfarbe in linearem RGB, Alpha derzeit ungenutzt.
    pub color: Vec4,
    pub visible: bool,
    /// Hängt das Objekt an ein anderes: `transform` ist dann relativ zum Elternobjekt.
    pub parent: Option<EntityId>,
}

impl Entity {
    pub fn new(name: impl Into<String>, mesh: MeshId) -> Self {
        Entity { name: name.into(), transform: Transform::default(), mesh, color: Vec4::ONE, visible: true, parent: None }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
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
#[derive(Default)]
pub struct Scene {
    entities: Vec<Entity>,
}

impl Scene {
    pub fn spawn(&mut self, entity: Entity) -> EntityId {
        self.entities.push(entity);
        EntityId(self.entities.len() - 1)
    }

    pub fn get(&self, id: EntityId) -> &Entity {
        &self.entities[id.0]
    }

    pub fn get_mut(&mut self, id: EntityId) -> &mut Entity {
        &mut self.entities[id.0]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    /// Transformation in Weltkoordinaten, inklusive aller Elternobjekte.
    pub fn world_matrix(&self, id: EntityId) -> Mat4 {
        let entity = &self.entities[id.0];
        match entity.parent {
            Some(parent) => self.world_matrix(parent) * entity.transform.matrix(),
            None => entity.transform.matrix(),
        }
    }

    /// Alle Objekte zusammen mit ihrer Weltmatrix.
    pub fn iter_world(&self) -> impl Iterator<Item = (&Entity, Mat4)> {
        self.entities.iter().enumerate().map(|(i, e)| (e, self.world_matrix(EntityId(i))))
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
