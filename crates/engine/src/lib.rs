//! Engine JN – eigene Spiele-Engine.
//!
//! Ein Spiel implementiert [`Game`] und wird mit [`run`] gestartet. Alles, was das
//! Spiel pro Frame braucht (Szene, Kamera, Eingabe, Zeit), steckt im [`Context`].

mod app;
pub mod assets;
pub mod camera;
pub mod daycycle;
pub mod input;
pub mod mesh;
pub mod model;
pub mod net;
pub mod noise;
pub mod particles;
pub mod physics;
mod renderer;
pub mod scene;
pub mod storage;
pub mod terrain;

pub use egui;

pub use app::{run, run_headless, Context, Display, EngineConfig, Environment, FrameStats, Game, PointLight, SkyBodies, Time};

/// Alles, was ein Spiel typischerweise braucht, mit einem einzigen `use`.
pub mod prelude {
    pub use crate::app::{run, run_headless, Context, Display, EngineConfig, Environment, Game, PointLight, Time};
    pub use egui;
    pub use crate::assets::{Assets, Image, MeshId, TextureId};
    pub use crate::model::{Animator, Model};
    pub use crate::camera::{Camera, FlyController, OrbitController};
    pub use crate::daycycle::{DayCycle, DayPhase};
    pub use crate::input::{Input, KeyCode, MouseButton};
    pub use crate::mesh::MeshData;
    pub use crate::net::{Channel, ClientId, NetClient, NetServer, ServerEvent};
    pub use crate::physics::{BodyDesc, CharacterId, CharacterSettings, CharacterState, Physics, RigidBodyHandle, Shape};
    pub use crate::scene::{Entity, EntityId, Material, Scene, Transform};
    pub use crate::noise::Rng;
    pub use crate::particles::Burst;
    pub use crate::terrain::Terrain;
    pub use glam::{vec2, vec3, vec4, Mat4, Quat, Vec2, Vec3, Vec4};
}
