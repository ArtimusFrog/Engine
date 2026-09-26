//! Spielfiguren aus Blender: der Magier (`art/modelle/figuren/magier.py` → `figuren/magier.gltf`)
//! und der Zwerg (`zwerg.py` → `figuren/zwerg.gltf`), mit Skelett-Animationen: Idle, Laufen,
//! Rennen, Springen, Hieb, Werfen, Zaubern, Abbauen, Hacken.
//! In der rechten Hand hält die Figur je nach Auswahl die Waffe (Stab bzw. Kriegshammer), die
//! Spitzhacke oder die Axt (starre Anbauteile im Modell, von denen immer nur eines sichtbar ist).
//!
//! Die Figuren haben echte Maße und werden nicht skaliert.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use engine::prelude::*;

use crate::asset_files;
use crate::protocol::{CharacterClass, Tool};

/// Die Kapsel hat ihren Mittelpunkt auf halber Höhe (1,8 m), das Modell steht mit den Füßen im Ursprung.
const FEET_OFFSET: f32 = -0.9;

/// Modelldatei einer Klasse.
fn datei(class: CharacterClass) -> &'static str {
    match class {
        CharacterClass::Zwerg => "zwerg",
        _ => "magier",
    }
}

/// So schnell wandern die Füße in den Animationen nach hinten (m/s, siehe art/lib/figuren.py) –
/// danach richtet sich das Abspieltempo, damit nichts rutscht. Der Zwerg hat kürzere Beine.
fn schritt(class: CharacterClass) -> (f32, f32) {
    match class {
        CharacterClass::Zwerg => (1.15, 2.6),
        _ => (1.7, 3.8),
    }
}

/// Jede Figur wird nur einmal pro Programmlauf eingelesen (fehlt der Zwerg, nimmt er den Magier).
fn model(class: CharacterClass) -> Option<Arc<Model>> {
    static MODELS: OnceLock<Mutex<HashMap<&'static str, Option<Arc<Model>>>>> = OnceLock::new();
    let name = datei(class);
    let geladen = MODELS
        .get_or_init(Default::default)
        .lock()
        .expect("Figurenliste gesperrt")
        .entry(name)
        .or_insert_with(|| {
            let path = asset_files::asset_dir()?.join("figuren").join(format!("{name}.gltf"));
            match Model::from_file(&path) {
                Ok(model) => Some(Arc::new(model)),
                Err(message) => {
                    log::error!("Spielfigur {name} fehlt: {message}");
                    None
                }
            }
        })
        .clone();
    if geladen.is_none() && class != CharacterClass::Mage {
        return model(CharacterClass::Mage);
    }
    geladen
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Chop,
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
    last_position: Option<Vec3>,
    speed: f32,
    acting: bool,
    tool: Option<Tool>,
    schritt: (f32, f32),
}

impl Puppet {
    /// Hängt die Figur an das Objekt der Spielfigur (`root` = Mittelpunkt der Kapsel).
    pub fn new(ctx: &mut Context, class: CharacterClass, root: EntityId) -> Puppet {
        let animator = model(class).map(|model| {
            let mut animator = Animator::new(model);
            animator.play("Idle", true, 0.0);
            animator
        });
        let mesh = match &animator {
            // Ein gemeinsames Mesh je Figur für alle Spieler; die Grafikkarte verformt es je Figur.
            Some(animator) => ctx.assets.named_mesh(&format!("figur_{}_gpu", datei(class)), || animator.model().skinned_gpu_mesh(None)),
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
        if let Some(animator) = &animator {
            figure.joints = animator.palette();
        }
        let figure = ctx.scene.spawn(figure);
        let mut puppet = Puppet { animator, figure, last_position: None, speed: 0.0, acting: false, tool: None, schritt: schritt(class) };
        puppet.set_tool(Tool::default());
        puppet
    }

    /// Zeigt, was in der Hand liegt: Waffe (Stab oder Hammer) bei den Fähigkeiten, sonst das Werkzeug.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool == Some(tool) {
            return;
        }
        self.tool = Some(tool);
        if let Some(animator) = &mut self.animator {
            let waffe = matches!(tool, Tool::Faehigkeit(_));
            animator.set_visible("Stab", waffe);
            animator.set_visible("Hammer", waffe);
            animator.set_visible("Spitzhacke", tool == Tool::Pickaxe);
            animator.set_visible("Axt", tool == Tool::Axe);
        }
    }

    /// Blendet die ganze Figur ein oder aus (z. B. Vergleichsfigur im Asset-Betrachter).
    pub fn set_visible(&self, ctx: &mut Context, visible: bool) {
        ctx.scene.get_mut(self.figure).visible = visible && self.animator.is_some();
    }

    /// Spielt eine einmalige Aktion ab: Holz hacken oder abbauen.
    pub fn act(&mut self, action: Action) {
        match action {
            Action::Chop => self.act_clip("Hacken", 1.3),
            Action::Mine => self.act_clip("Abbauen", 1.25),
        }
    }

    /// Spielt einen Clip einmal ab (fehlt er, eine ähnliche Bewegung).
    pub fn act_clip(&mut self, clip: &str, speed: f32) {
        let Some(animator) = &mut self.animator else { return };
        let fallback = match clip {
            "Hacken" | "Abbauen" => "Hieb",
            _ => "Werfen",
        };
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
        let (walk, run) = self.schritt;

        if self.acting && animator.finished() {
            self.acting = false;
        }
        if !self.acting {
            if airborne {
                animator.play("Springen", true, 0.15);
                animator.set_speed(1.0);
            } else if self.speed > 3.4 {
                animator.play("Rennen", true, 0.2);
                animator.set_speed((self.speed / run).clamp(0.7, 3.0));
            } else if self.speed > 0.4 {
                animator.play("Laufen", true, 0.2);
                animator.set_speed((self.speed / walk).clamp(0.5, 3.0));
            } else {
                animator.play("Idle", true, 0.25);
                animator.set_speed(1.0);
            }
        }
        animator.update(dt);
        if let Some(figure) = ctx.scene.try_get_mut(self.figure) {
            figure.joints = animator.palette();
        }
    }
}
