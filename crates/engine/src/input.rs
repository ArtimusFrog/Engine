use std::collections::HashSet;

use glam::Vec2;
use winit::event::ElementState;
use winit::keyboard::PhysicalKey;

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

/// Zustand von Tastatur und Maus für den aktuellen Frame.
#[derive(Default)]
pub struct Input {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    buttons_down: HashSet<MouseButton>,
    buttons_pressed: HashSet<MouseButton>,
    mouse_delta: Vec2,
}

impl Input {
    /// Taste wird gerade gehalten.
    pub fn key(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Taste wurde in diesem Frame gedrückt.
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn mouse(&self, button: MouseButton) -> bool {
        self.buttons_down.contains(&button)
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    /// Mausbewegung seit dem letzten Frame (roh, ohne Mausbeschleunigung).
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub(crate) fn on_key(&mut self, key: PhysicalKey, state: ElementState, repeat: bool) {
        let PhysicalKey::Code(code) = key else { return };
        match state {
            ElementState::Pressed => {
                if !repeat {
                    self.keys_pressed.insert(code);
                }
                self.keys_down.insert(code);
            }
            ElementState::Released => {
                self.keys_down.remove(&code);
            }
        }
    }

    pub(crate) fn on_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.buttons_pressed.insert(button);
                self.buttons_down.insert(button);
            }
            ElementState::Released => {
                self.buttons_down.remove(&button);
            }
        }
    }

    pub(crate) fn on_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.mouse_delta += Vec2::new(dx as f32, dy as f32);
    }

    /// Fenster hat den Fokus verloren: Tasten gälten sonst als dauerhaft gedrückt.
    pub(crate) fn release_all(&mut self) {
        self.keys_down.clear();
        self.buttons_down.clear();
    }

    pub(crate) fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.buttons_pressed.clear();
        self.mouse_delta = Vec2::ZERO;
    }
}
