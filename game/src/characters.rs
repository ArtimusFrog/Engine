//! Spielfigur: der Magier aus Blender (`art/modelle/figuren/magier.py` → `figuren/magier.gltf`)
//! mit Skelett-Animationen: Idle, Laufen, Rennen, Springen, Hieb, Werfen, Zaubern, Abbauen.
//! In der rechten Hand hält er je nach Werkzeug den Stab oder die Spitzhacke (zwei starre
//! Anbauteile im Modell, von denen immer nur eines sichtbar ist).
//!
//! Die Figur hat echte Maße (etwa 1,85 m, mit Hut mehr) und wird nicht skaliert.

use std::sync::{Arc, OnceLock};

use engine::prelude::*;

use crate::asset_files;
use crate::protocol::{CharacterClass, Tool};

/// Die Kapsel hat ihren Mittelpunkt auf halber Höhe (1,8 m), das Modell steht mit den Füßen im Ursprung.
const FEET_OFFSET: f32 = -0.9;
/// So schnell wandern die Füße in den Animationen nach hinten (m/s, siehe art/lib/figuren.py) –
/// danach richtet sich das Abspieltempo, damit nichts rutscht.
const WALK_SPEED: f32 = 1.7;
const RUN_SPEED: f32 = 3.8;

/// Der Magier wird nur einmal pro Programmlauf eingelesen.
fn model() -> Option<Arc<Model>> {
    static MODEL: OnceLock<Option<Arc<Model>>> = OnceLock::new();
    MODEL
        .get_or_init(|| {
            let path = asset_files::asset_dir()?.join("figuren").join("magier.gltf");
            match Model::from_file(&path) {
                Ok(model) => Some(Arc::new(model)),
                Err(message) => {
                    log::error!("Spielfigur fehlt: {message}");
                    None
                }
            }
        })
        .clone()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Chop,
    Cast,
    /// Mit der Spitzhacke auf ein Vorkommen schlagen
    Mine,
}

/// Wie lange (Sekunden) nach dem Beginn von „Abbauen“ die Hacke auftrifft (Bild 16 von 30, Tempo 1,25).
pub const MINE_STRIKE: f32 = 16.0 / 30.0 / 1.25;
/// Wie lange nach dem Beginn von „Hacken“ die Axt den Stamm trifft (Bild 11 von 24, Tempo 1,3).
pub const CHOP_STRIKE: f32 = 11.0 / 30.0 / 1.3;

/// Die sichtbare, animierte Figur eines Spielers.
pub struct Puppet {
    /// Fehlt die Modelldatei, bleibt nur die (sichtbare) Kapsel.
    animator: Option<Animator>,
    figure: EntityId,
    mesh: MeshId,
    last_position: Option<Vec3>,
    speed: f32,
    acting: bool,
    tool: Option<Tool>,
}

impl Puppet {
    /// Hängt die Figur an das Objekt der Spielfigur (`root` = Mittelpunkt der Kapsel).
    /// Alle Klassen sehen im Moment gleich aus: der Magier.
    pub fn new(ctx: &mut Context, _class: CharacterClass, root: EntityId) -> Puppet {
        let animator = model().map(|model| {
            let mut animator = Animator::new(model);
            animator.play("Idle", true, 0.0);
            animator
        });
        let mesh = match &animator {
            Some(animator) => ctx.assets.add_mesh(animator.skinned_mesh(None)),
            None => {
                ctx.scene.get_mut(root).visible = true;
                ctx.assets.cube()
            }
        };
        let mut figure = Entity::new("Figur", mesh).with_parent(root).with_transform(
            Transform::from_position(Vec3::Y * FEET_OFFSET)
                // Blender-Modelle schauen nach +Z, unsere Spielfiguren nach -Z.
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        );
        figure.visible = animator.is_some();
        let figure = ctx.scene.spawn(figure);
        let mut puppet = Puppet { animator, figure, mesh, last_position: None, speed: 0.0, acting: false, tool: None };
        puppet.set_tool(Tool::default());
        puppet
    }

    /// Zeigt das Werkzeug in der Hand: Stab oder Spitzhacke.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool == Some(tool) {
            return;
        }
        self.tool = Some(tool);
        if let Some(animator) = &mut self.animator {
            animator.set_visible("Stab", tool == Tool::Staff);
            animator.set_visible("Spitzhacke", tool == Tool::Pickaxe);
            animator.set_visible("Axt", tool == Tool::Axe);
        }
    }

    /// Blendet die ganze Figur ein oder aus (z. B. Vergleichsfigur im Asset-Betrachter).
    pub fn set_visible(&self, ctx: &mut Context, visible: bool) {
        ctx.scene.get_mut(self.figure).visible = visible && self.animator.is_some();
    }

    /// Spielt eine einmalige Aktion ab: Hieb mit dem Stab (Holz hacken) oder Zaubern.
    pub fn act(&mut self, action: Action) {
        let Some(animator) = &mut self.animator else { return };
        let (clip, fallback, speed) = match action {
            Action::Chop => ("Hacken", "Hieb", 1.3),
            Action::Cast => ("Zaubern", "Werfen", 1.35),
            Action::Mine => ("Abbauen", "Hieb", 1.25),
        };
        // Ältere Modelle ohne die Animation: eine ähnliche Bewegung nehmen.
        if !animator.play(clip, false, 0.08) {
            animator.play(fallback, false, 0.08);
        }
        animator.set_speed(speed);
        self.acting = true;
    }

    /// Einmal pro Bild: Animation passend zur Bewegung wählen und die Figur neu verformen.
    /// `position` ist die dargestellte Position der Kapsel, `ground` die Bodenhöhe darunter.
    pub fn update(&mut self, ctx: &mut Context, position: Vec3, ground: f32) {
        let Some(animator) = &mut self.animator else { return };
        let dt = ctx.time.delta.max(1e-4);
        let last = self.last_position.replace(position).unwrap_or(position);
        let horizontal = vec2(position.x - last.x, position.z - last.z).length() / dt;
        // Geglättet, sonst flackert die Animation bei ungleichmäßigen Bildraten.
        self.speed += (horizontal.min(20.0) - self.speed) * (dt * 10.0).min(1.0);
        let airborne = position.y + FEET_OFFSET - ground > 0.35;

        if self.acting && animator.finished() {
            self.acting = false;
        }
        if !self.acting {
            if airborne {
                animator.play("Springen", true, 0.15);
                animator.set_speed(1.0);
            } else if self.speed > 3.4 {
                animator.play("Rennen", true, 0.2);
                animator.set_speed((self.speed / RUN_SPEED).clamp(0.7, 2.2));
            } else if self.speed > 0.4 {
                animator.play("Laufen", true, 0.2);
                animator.set_speed((self.speed / WALK_SPEED).clamp(0.5, 2.0));
            } else {
                animator.play("Idle", true, 0.25);
                animator.set_speed(1.0);
            }
        }
        animator.update(dt);
        ctx.assets.update_mesh(self.mesh, animator.skinned_mesh(None));
    }
}
