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
        CharacterClass::Bogenschuetze => "bogenschuetze",
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

/// Lage eines Knotens in Ruhehaltung (im Raum des Modells).
fn ruhe_matrix(model: &Model, knoten: usize) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    let mut node = Some(knoten);
    while let Some(n) = node {
        let r = &model.nodes[n].rest;
        m = Mat4::from_scale_rotation_translation(r.scale, r.rotation, r.translation) * m;
        node = model.nodes[n].parent;
    }
    m
}

/// Spitze einer Waffe im Raum ihrer Hand: beim Stab das obere Ende, beim Hammer (der mit dem
/// Kopf nach unten hängt) der Kopf, beim Bogen (`None`) die Mitte – dort liegt der Pfeil auf.
fn waffen_spitze_lokal(model: &Model, knoten: &str, hand: &str, oben: Option<bool>) -> Option<Vec3> {
    let mesh = model.extract_part(knoten, None)?;
    let Some(oben) = oben else {
        let summe: Vec3 = mesh.vertices.iter().map(|v| Vec3::from(v.position)).sum();
        return Some(summe / mesh.vertices.len().max(1) as f32);
    };
    let hand = ruhe_matrix(model, model.node(hand)?);
    let hoehe = |v: &engine::mesh::Vertex| hand.transform_point3(Vec3::from(v.position)).y;
    let beste = if oben {
        mesh.vertices.iter().max_by(|a, b| hoehe(a).total_cmp(&hoehe(b)))?
    } else {
        mesh.vertices.iter().min_by(|a, b| hoehe(a).total_cmp(&hoehe(b)))?
    };
    // Beim Hammer die Mitte des Kopfes: alles in der Nähe des tiefsten Punkts mitteln
    let ziel = Vec3::from(beste.position);
    let nah: Vec<Vec3> = mesh.vertices.iter().map(|v| Vec3::from(v.position)).filter(|p| p.distance(ziel) < if oben { 0.05 } else { 0.14 }).collect();
    Some(nah.iter().sum::<Vec3>() / nah.len().max(1) as f32)
}

/// Die Waffe als eigenes Mesh (für den geworfenen Hammer): Mitte im Ursprung, Stiel entlang Y.
pub fn waffen_flugmesh(ctx: &mut Context, class: CharacterClass, knoten: &str) -> Option<MeshId> {
    let model = model(class)?;
    let name = format!("flug_{}_{knoten}", datei(class));
    Some(ctx.assets.named_mesh(&name, || {
        let mut mesh = model.extract_part(knoten, None).unwrap_or_default();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &mesh.vertices {
            lo = lo.min(v.position.into());
            hi = hi.max(v.position.into());
        }
        let mitte = (lo + hi) * 0.5;
        let ausdehnung = hi - lo;
        let lang = if ausdehnung.x >= ausdehnung.y && ausdehnung.x >= ausdehnung.z {
            Vec3::X
        } else if ausdehnung.z >= ausdehnung.y {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let mut drehung = Quat::from_rotation_arc(lang, Vec3::Y);
        let schwerpunkt: Vec3 = mesh.vertices.iter().map(|v| drehung * (Vec3::from(v.position) - mitte)).sum::<Vec3>() / mesh.vertices.len().max(1) as f32;
        if schwerpunkt.y > 0.0 {
            drehung = Quat::from_rotation_x(std::f32::consts::PI) * drehung;
        }
        for v in &mut mesh.vertices {
            v.position = (drehung * (Vec3::from(v.position) - mitte)).into();
            v.normal = (drehung * Vec3::from(v.normal)).into();
        }
        mesh
    }))
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
    /// Anbauteil der ausgerüsteten Waffe (None = Startwaffe „Stab“ bzw. „Hammer“)
    waffe: Option<&'static str>,
    class: CharacterClass,
    /// Spitze der Waffe im Raum der rechten Hand (Stab oben, Hammerkopf)
    spitze: Option<Vec3>,
    /// Die Waffe fliegt gerade (Wurfhammer): die Hand ist leer
    hand_leer: bool,
    /// Treffer-Stopp: so lange steht die Animation fast still (Sekunden)
    stopp: f32,
    /// Anteil der Fähigkeits-Animation an den Beinen (1 im Stand, 0 beim Laufen)
    beine: f32,
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
        let mut figure = Entity::new("Figur", mesh).with_parent(root).with_material(Material::Figur { rim: 1.0 }).with_transform(
            Transform::from_position(Vec3::Y * FEET_OFFSET)
                // Blender-Modelle schauen nach +Z, unsere Spielfiguren nach -Z.
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        );
        figure.visible = animator.is_some();
        if let Some(animator) = &animator {
            figure.joints = animator.palette();
        }
        let figure = ctx.scene.spawn(figure);
        let mut animator = animator;
        if let Some(animator) = &mut animator {
            // Fähigkeiten beim Laufen nur auf dem Oberkörper (Bauch aufwärts)
            animator.set_upper_body("Bauch");
        }
        let mut puppet = Puppet {
            animator,
            figure,
            last_position: None,
            speed: 0.0,
            acting: false,
            tool: None,
            schritt: schritt(class),
            waffe: None,
            class,
            spitze: None,
            hand_leer: false,
            stopp: 0.0,
            beine: 1.0,
        };
        puppet.set_tool(Tool::default());
        puppet.spitze_berechnen();
        puppet
    }

    /// Zeigt, was in der Hand liegt: die Waffe bei den Fähigkeiten, sonst das Werkzeug.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool == Some(tool) {
            return;
        }
        self.tool = Some(tool);
        self.zeigen();
    }

    /// Welche Waffe die Figur trägt (Anbauteil im Modell, None = Startwaffe).
    pub fn set_waffe(&mut self, waffe: Option<&'static str>) {
        if self.waffe == waffe {
            return;
        }
        self.waffe = waffe;
        self.zeigen();
        self.spitze_berechnen();
    }

    /// Knoten der Waffe in der Hand (Startwaffe „Stab“ bzw. „Hammer“).
    pub fn waffen_knoten(&self) -> &'static str {
        self.waffe.unwrap_or(match self.class {
            CharacterClass::Zwerg => "Hammer",
            CharacterClass::Bogenschuetze => "Bogen",
            _ => "Stab",
        })
    }

    /// Die Hand, die die Waffe hält (der Bogen liegt in der linken).
    fn waffen_hand(&self) -> &'static str {
        if self.class == CharacterClass::Bogenschuetze { "Hand.L" } else { "Hand.R" }
    }

    fn spitze_berechnen(&mut self) {
        let oben = match self.class {
            CharacterClass::Zwerg => Some(false),
            CharacterClass::Bogenschuetze => None,
            _ => Some(true),
        };
        let (knoten, hand) = (self.waffen_knoten(), self.waffen_hand());
        self.spitze = self.animator.as_ref().and_then(|a| waffen_spitze_lokal(a.model(), knoten, hand, oben));
    }

    /// Die Waffe fliegt (Wurfhammer): Hand leer bzw. wieder gefangen.
    pub fn set_hand_leer(&mut self, leer: bool) {
        if self.hand_leer != leer {
            self.hand_leer = leer;
            self.zeigen();
        }
    }

    /// Treffer-Stopp: die Animation steht kurz fast still (schwere Schläge wirken wuchtiger).
    pub fn stopp(&mut self, dauer: f32) {
        self.stopp = self.stopp.max(dauer);
    }

    /// Lage eines Knochens in der Welt (z. B. `Hand.L`).
    pub fn knochen_welt(&self, ctx: &Context, name: &str) -> Option<Vec3> {
        let animator = self.animator.as_ref()?;
        let m = ctx.scene.world_matrix(self.figure) * animator.node_matrix(name)?;
        Some(m.w_axis.truncate())
    }

    /// Spitze der Waffe in der Welt (Stab oben, Hammerkopf).
    pub fn waffen_spitze(&self, ctx: &Context) -> Option<Vec3> {
        let animator = self.animator.as_ref()?;
        let m = ctx.scene.world_matrix(self.figure) * animator.node_matrix(self.waffen_hand())?;
        Some(m.transform_point3(self.spitze?))
    }

    /// Spielt eine Fähigkeit als zweite Ebene: im Stand mit dem ganzen Körper, beim Laufen nur
    /// auf dem Oberkörper (die Beine laufen weiter).
    pub fn act_faehigkeit(&mut self, clip: &str, speed: f32) {
        let Some(animator) = &mut self.animator else { return };
        if !animator.play_overlay(clip, speed, 0.06, 0.22) {
            let ersatz = match self.class {
                CharacterClass::Zwerg => "Hieb",
                CharacterClass::Bogenschuetze => "Werfen",
                _ => "Zaubern",
            };
            animator.play_overlay(ersatz, speed * 1.3, 0.06, 0.22);
        }
    }

    fn zeigen(&mut self) {
        let Some(tool) = self.tool else { return };
        let Some(animator) = &mut self.animator else { return };
        let kampf = matches!(tool, Tool::Faehigkeit(_)) && !self.hand_leer;
        for start in ["Stab", "Hammer", "Bogen"] {
            animator.set_visible(start, kampf && self.waffe.is_none());
        }
        for w in &crate::waffen::WAFFEN {
            animator.set_visible(w.datei, kampf && self.waffe == Some(w.datei));
        }
        animator.set_visible("Spitzhacke", tool == Tool::Pickaxe);
        animator.set_visible("Axt", tool == Tool::Axe);
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
        // Fähigkeiten: im Stand ganzer Körper, beim Laufen und Springen nur der Oberkörper
        let ziel = if self.speed < 0.8 && !airborne { 1.0 } else { 0.0 };
        self.beine += (ziel - self.beine) * (dt * 8.0).min(1.0);
        animator.set_overlay_legs(self.beine);
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
        // Treffer-Stopp: kurz fast stehen bleiben
        let anim_dt = if self.stopp > 0.0 {
            self.stopp -= dt;
            dt * 0.05
        } else {
            dt
        };
        animator.update(anim_dt);
        if let Some(figure) = ctx.scene.try_get_mut(self.figure) {
            figure.joints = animator.palette();
        }
    }
}
