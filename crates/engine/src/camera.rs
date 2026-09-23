use glam::camera::rh;
use glam::{Mat4, Vec3};

use crate::app::Context;
use crate::input::{KeyCode, MouseButton};

pub struct Camera {
    pub position: Vec3,
    /// Drehung um die Hochachse in Radiant, 0 = Blick nach -Z.
    pub yaw: f32,
    /// Neigung in Radiant, positiv = nach oben.
    pub pitch: f32,
    /// Vertikales Sichtfeld in Radiant.
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { position: Vec3::new(0.0, 2.0, 8.0), yaw: 0.0, pitch: 0.0, fov_y: 70f32.to_radians(), near: 0.1, far: 1000.0 }
    }
}

impl Camera {
    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos())
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn look_at(&mut self, target: Vec3) {
        let dir = (target - self.position).normalize();
        self.pitch = dir.y.asin();
        self.yaw = dir.x.atan2(-dir.z);
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let view = rh::view::look_to_mat4(self.position, self.forward(), Vec3::Y);
        // wgpu nutzt die DirectX-Konvention: Tiefe 0..1, Y zeigt nach oben.
        let proj = rh::proj::directx::perspective(self.fov_y, aspect, self.near, self.far);
        proj * view
    }
}

/// Freie Kamera wie im Unreal-Viewport: rechte Maustaste halten, mit WASD/QE fliegen.
pub struct FlyController {
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for FlyController {
    fn default() -> Self {
        FlyController { speed: 8.0, sensitivity: 0.0025 }
    }
}

impl FlyController {
    pub fn update(&mut self, ctx: &mut Context) {
        let looking = ctx.input.mouse(MouseButton::Right);
        ctx.cursor_locked = looking;
        if !looking {
            return;
        }

        let delta = ctx.input.mouse_delta() * self.sensitivity;
        let cam = &mut ctx.camera;
        cam.yaw += delta.x;
        cam.pitch = (cam.pitch - delta.y).clamp(-1.55, 1.55);

        let (forward, right) = (cam.forward(), cam.right());
        let keys = &ctx.input;
        let mut dir = Vec3::ZERO;
        for (key, step) in [
            (KeyCode::KeyW, forward),
            (KeyCode::KeyS, -forward),
            (KeyCode::KeyD, right),
            (KeyCode::KeyA, -right),
            (KeyCode::KeyE, Vec3::Y),
            (KeyCode::KeyQ, -Vec3::Y),
        ] {
            if keys.key(key) {
                dir += step;
            }
        }
        let boost = if keys.key(KeyCode::ShiftLeft) { 4.0 } else { 1.0 };
        cam.position += dir.normalize_or_zero() * self.speed * boost * ctx.time.delta;
    }
}
