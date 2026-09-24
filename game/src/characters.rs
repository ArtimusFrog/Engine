//! Spielfiguren: KayKit-Abenteurer (CC0) mit Skelett-Animationen.
//!
//! Die Modelle stecken direkt in der .exe, damit das Spiel ohne Zusatzdateien läuft.

use std::sync::{Arc, OnceLock};

use engine::prelude::*;

use crate::protocol::CharacterClass;

/// Höhe der Figur in Metern (passt zur Kapsel der Physik).
const FIGURE_HEIGHT: f32 = 1.8;
/// Die Kapsel hat ihren Mittelpunkt auf halber Höhe, das Modell steht mit den Füßen im Ursprung.
const FEET_OFFSET: f32 = -0.9;

struct Loaded {
    model: Arc<Model>,
    scale: f32,
}

fn bytes(class: CharacterClass) -> &'static [u8] {
    match class {
        CharacterClass::Knight => include_bytes!("../assets/characters/Knight.glb"),
        CharacterClass::Barbarian => include_bytes!("../assets/characters/Barbarian.glb"),
        CharacterClass::Mage => include_bytes!("../assets/characters/Mage.glb"),
        CharacterClass::Rogue => include_bytes!("../assets/characters/Rogue.glb"),
    }
}

/// Jedes Modell wird nur einmal pro Programmlauf eingelesen.
fn loaded(class: CharacterClass) -> &'static Loaded {
    static CACHE: [OnceLock<Loaded>; 4] = [OnceLock::new(), OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let index = CharacterClass::ALL.iter().position(|&c| c == class).unwrap_or(0);
    CACHE[index].get_or_init(|| {
        let model = Model::from_glb(bytes(class)).unwrap_or_else(|e| panic!("Figur {} ist beschädigt: {e}", class.label()));
        let scale = FIGURE_HEIGHT / model.rest_height();
        Loaded { model: Arc::new(model), scale }
    })
}

fn texture(ctx: &mut Context, class: CharacterClass) -> TextureId {
    let model = &loaded(class).model;
    ctx.assets.named_texture(&format!("figur_{class:?}"), || model.images[0].clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Chop,
    Throw,
}

/// Die sichtbare, animierte Figur eines Spielers.
pub struct Puppet {
    animator: Animator,
    texture: TextureId,
    mesh: MeshId,
    axe: EntityId,
    last_position: Option<Vec3>,
    speed: f32,
    acting: bool,
    /// Wie lange die Axt noch in der Hand bleibt.
    axe_timer: f32,
}

impl Puppet {
    /// Hängt die Figur an das Objekt der Spielfigur (`root` = Mittelpunkt der Kapsel).
    pub fn new(ctx: &mut Context, class: CharacterClass, root: EntityId) -> Puppet {
        let loaded = loaded(class);
        let texture = texture(ctx, class);
        let mut animator = Animator::new(loaded.model.clone());
        // Waffen und Schilde weg, Helm/Hut und Umhang bleiben.
        for name in loaded.model.attachments() {
            let keep = ["Helmet", "Hat", "Cape"].iter().any(|k| name.contains(k));
            animator.set_visible(name, keep);
        }
        animator.play("Idle", true, 0.0);
        let mesh = ctx.assets.add_mesh(animator.skinned_mesh(Some(texture)));

        let figure = ctx.scene.spawn(
            Entity::new("Figur", mesh).with_parent(root).with_transform(
                Transform::from_position(Vec3::Y * FEET_OFFSET)
                    // KayKit-Figuren schauen nach +Z, unsere Spielfiguren nach -Z.
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                    .with_scale(Vec3::splat(loaded.scale)),
            ),
        );
        let barbarian_texture = self::texture(ctx, CharacterClass::Barbarian);
        let axe_mesh = ctx.assets.named_mesh("axt", || {
            loaded_axe(barbarian_texture)
        });
        let mut axe = Entity::new("Axt", axe_mesh).with_parent(figure);
        axe.visible = false;
        let axe = ctx.scene.spawn(axe);

        Puppet { animator, texture, mesh, axe, last_position: None, speed: 0.0, acting: false, axe_timer: 0.0 }
    }

    /// Spielt eine einmalige Aktion ab (Hacken, Werfen).
    pub fn act(&mut self, action: Action) {
        let (clip, speed) = match action {
            Action::Chop => ("1H_Melee_Attack_Chop", 1.5),
            Action::Throw => ("Throw", 1.4),
        };
        self.animator.play(clip, false, 0.08);
        self.animator.set_speed(speed);
        self.acting = true;
        if action == Action::Chop {
            self.axe_timer = 1.6;
        }
    }

    /// Einmal pro Bild: Animation passend zur Bewegung wählen und die Figur neu verformen.
    /// `position` ist die dargestellte Position der Kapsel, `ground` die Bodenhöhe darunter.
    pub fn update(&mut self, ctx: &mut Context, position: Vec3, ground: f32) {
        let dt = ctx.time.delta.max(1e-4);
        let last = self.last_position.replace(position).unwrap_or(position);
        let horizontal = vec2(position.x - last.x, position.z - last.z).length() / dt;
        // Geglättet, sonst flackert die Animation bei ungleichmäßigen Bildraten.
        self.speed += (horizontal.min(20.0) - self.speed) * (dt * 10.0).min(1.0);
        let airborne = position.y + FEET_OFFSET - ground > 0.35;

        if self.acting && self.animator.finished() {
            self.acting = false;
        }
        if !self.acting {
            if airborne {
                self.animator.play("Jump_Idle", true, 0.15);
                self.animator.set_speed(1.0);
            } else if self.speed > 3.4 {
                self.animator.play("Running_A", true, 0.2);
                self.animator.set_speed((self.speed / 5.5).clamp(0.7, 1.7));
            } else if self.speed > 0.4 {
                self.animator.play("Walking_A", true, 0.2);
                self.animator.set_speed((self.speed / 2.2).clamp(0.5, 1.6));
            } else {
                self.animator.play("Idle", true, 0.25);
                self.animator.set_speed(1.0);
            }
        }
        self.animator.update(dt);
        ctx.assets.update_mesh(self.mesh, self.animator.skinned_mesh(Some(self.texture)));

        // Axt folgt der rechten Hand.
        self.axe_timer -= dt;
        if let (Some(hand), Some(axe)) = (self.animator.node_matrix("handslot.r"), ctx.scene.try_get_mut(self.axe)) {
            let (scale, rotation, translation) = hand.to_scale_rotation_translation();
            axe.transform = Transform { position: translation, rotation, scale };
            axe.visible = self.axe_timer > 0.0;
        }
    }
}

fn loaded_axe(texture: TextureId) -> MeshData {
    loaded(CharacterClass::Barbarian)
        .model
        .extract_part("1H_Axe", Some(texture))
        .unwrap_or_else(|| MeshData::cube())
}
