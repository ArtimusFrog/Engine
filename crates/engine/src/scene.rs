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
}

impl Entity {
    pub fn new(name: impl Into<String>, mesh: MeshId) -> Self {
        Entity { name: name.into(), transform: Transform::default(), mesh, color: Vec4::ONE, visible: true }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(usize);

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

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
