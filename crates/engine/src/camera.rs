use glam::camera::rh;
use glam::{Mat4, Vec2, Vec3};

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
        Camera { position: Vec3::new(0.0, 2.0, 8.0), yaw: 0.0, pitch: 0.0, fov_y: 70f32.to_radians(), near: 0.1, far: 2200.0 }
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

/// Third-Person-Kamera, die um ein Ziel (meist die Spielfigur) kreist.
/// Maus dreht die Kamera, solange der Zeiger gefangen ist; das Mausrad zoomt.
pub struct OrbitController {
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub sensitivity: f32,
    /// Die Kamera bleibt vor Wänden stehen, statt hindurchzusehen.
    pub avoid_walls: bool,
    /// Maus nach oben = nach unten schauen (wie im Flugzeug).
    pub invert_y: bool,
    /// Schulterblick: um so viele Meter nach rechts und oben versetzt kreist die Kamera – die
    /// Bildmitte (Fadenkreuz) liegt dann neben der Figur statt auf ihr.
    pub schulter: Vec2,
}

impl Default for OrbitController {
    fn default() -> Self {
        OrbitController { distance: 6.0, min_distance: 2.0, max_distance: 20.0, sensitivity: 0.0025, avoid_walls: true, invert_y: false, schulter: Vec2::ZERO }
    }
}

impl OrbitController {
    /// `ignore` ist die Spielfigur selbst, damit die Kamera nicht an ihr hängen bleibt.
    pub fn update(&mut self, ctx: &mut Context, target: Vec3, ignore: Option<crate::physics::CharacterId>) {
        if ctx.cursor_locked {
            let delta = ctx.input.mouse_delta() * self.sensitivity;
            ctx.camera.yaw += delta.x;
            let dy = if self.invert_y { -delta.y } else { delta.y };
            ctx.camera.pitch = (ctx.camera.pitch - dy).clamp(-1.4, 1.2);
        }
        self.distance = (self.distance * 0.9f32.powf(ctx.input.scroll())).clamp(self.min_distance, self.max_distance);

        let back = -ctx.camera.forward();
        // Drehpunkt neben der Figur (Schulterblick), aber nicht in eine Wand hinein
        let mut pivot = target;
        if self.schulter != Vec2::ZERO {
            let versatz = ctx.camera.right() * self.schulter.x + Vec3::Y * self.schulter.y;
            let laenge = versatz.length();
            let weit = if self.avoid_walls { ctx.physics.raycast(target, versatz / laenge, laenge + 0.3, ignore).map_or(laenge, |(_, d)| (d - 0.3).max(0.0)) } else { laenge };
            pivot = target + versatz / laenge * weit;
        }
        let mut distance = self.distance;
        if self.avoid_walls {
            if let Some((_, hit)) = ctx.physics.raycast(pivot, back, distance, ignore) {
                distance = (hit - 0.2).max(0.3);
            }
        }
        ctx.camera.position = pivot + back * distance;
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
