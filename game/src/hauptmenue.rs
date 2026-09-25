//! Das Hauptmenü als Titelbild: die Insel bei Nacht (mit Polarlicht), die Kamera kreist hoch
//! darüber; dazu Kinobalken, Vignette, aufsteigende Glutfunken, ein großer goldener Titel und
//! Knöpfe im Gold-Leder-Stil.

use engine::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind};
use engine::noise::Rng;
use engine::prelude::*;

use crate::inventar::{diamond, divider, gradient, spaced_text, GOLD, GOLD_DARK, GOLD_LIGHT, LEATHER_BOTTOM, LEATHER_TOP, PARCHMENT};
use crate::world::World;

/// Uhrzeit im Menü: tiefe Nacht, Mond und Sterne stehen am Himmel.
pub const MENU_HOUR: f32 = 23.2;

struct Ember {
    /// Position in Bildschirm-Anteilen (0..1), Tempo und Größe
    at: Pos2,
    speed: f32,
    size: f32,
    phase: f32,
}

pub struct TitleScreen {
    opened: f32,
    embers: Vec<Ember>,
    rng: Rng,
}

impl TitleScreen {
    /// Menü-Insel vorbereiten: tiefe Nacht mit Polarlicht, klarer Himmel.
    pub fn new(world: &mut World, now: f32) -> TitleScreen {
        world.day.hour = MENU_HOUR;
        world.weather.force_named("polarlicht");
        TitleScreen { opened: now, embers: Vec::new(), rng: Rng::new(0x71_7E) }
    }

    /// Die Kamera kreist langsam hoch über der ganzen Insel.
    pub fn camera(&self, ctx: &mut Context) {
        let t = ctx.time.elapsed * 0.03 + 0.8;
        let r = crate::island::ISLAND_RADIUS * 1.25;
        ctx.camera.position = vec3(t.sin() * r, crate::island::ISLAND_RADIUS * 0.38, t.cos() * r);
        ctx.camera.look_at(vec3(0.0, 6.0, 0.0));
    }

    /// Alles über dem Bild: Vignette, Kinobalken, Glutfunken, Schwarzblende zwischen den Einstellungen.
    pub fn backdrop(&mut self, ctx: &Context, egui_ctx: &egui::Context) {
        let screen = egui_ctx.content_rect();
        let painter = egui_ctx.layer_painter(egui::LayerId::background());
        let now = ctx.time.elapsed;
        let dt = ctx.time.delta;

        // Links abgedunkelt (für Titel und Knöpfe), zu den Rändern hin Vignette
        let left = Rect::from_min_max(screen.min, egui::pos2(screen.min.x + screen.width() * 0.62, screen.max.y));
        gradient(&painter, left, Color32::from_black_alpha(215), Color32::TRANSPARENT, true);
        for (rect, from, to, horizontal) in [
            (Rect::from_min_max(screen.min, egui::pos2(screen.max.x, screen.min.y + 170.0)), Color32::from_black_alpha(150), Color32::TRANSPARENT, false),
            (Rect::from_min_max(egui::pos2(screen.min.x, screen.max.y - 190.0), screen.max), Color32::TRANSPARENT, Color32::from_black_alpha(175), false),
            (Rect::from_min_max(egui::pos2(screen.max.x - 160.0, screen.min.y), screen.max), Color32::TRANSPARENT, Color32::from_black_alpha(110), true),
        ] {
            gradient(&painter, rect, from, to, horizontal);
        }
        // Kinobalken
        let bar = (screen.height() * 0.055).round();
        painter.rect_filled(Rect::from_min_max(screen.min, egui::pos2(screen.max.x, screen.min.y + bar)), 0.0, Color32::BLACK);
        painter.rect_filled(Rect::from_min_max(egui::pos2(screen.min.x, screen.max.y - bar), screen.max), 0.0, Color32::BLACK);

        // Glutfunken: steigen langsam auf, flackern, schwanken
        while self.embers.len() < 46 {
            let at = egui::pos2(self.rng.range(0.0, 1.0), if self.embers.len() < 30 { self.rng.range(0.0, 1.0) } else { 1.05 });
            self.embers.push(Ember { at, speed: self.rng.range(0.012, 0.04), size: self.rng.range(1.0, 2.8), phase: self.rng.range(0.0, 6.28) });
        }
        for ember in &mut self.embers {
            ember.at.y -= ember.speed * dt;
            ember.at.x += (now * 0.7 + ember.phase).sin() * 0.004 * dt;
            let p = egui::pos2(screen.min.x + ember.at.x * screen.width(), screen.min.y + ember.at.y * screen.height());
            let flicker = 0.55 + 0.45 * (now * 3.1 + ember.phase * 2.0).sin();
            let fade = (ember.at.y * 1.4).clamp(0.0, 1.0);
            let alpha = (flicker * fade * 255.0) as u8;
            painter.circle_filled(p, ember.size * 3.0, Color32::from_rgba_unmultiplied(255, 120, 30, alpha / 8));
            painter.circle_filled(p, ember.size, Color32::from_rgba_unmultiplied(255, 190, 90, alpha));
        }
        self.embers.retain(|e| e.at.y > -0.05);

        // Aus dem Schwarz einblenden, wenn das Menü aufgeht
        let fade = 1.0 - ((now - self.opened) / 2.5).clamp(0.0, 1.0);
        if fade > 0.0 {
            painter.rect_filled(screen, 0.0, Color32::from_black_alpha((fade * 255.0) as u8));
        }
    }

    /// Großer Titel mit Zierlinien; blendet beim Öffnen des Menüs langsam ein.
    pub fn title(&self, ui: &mut egui::Ui, now: f32) {
        let appear = ((now - self.opened - 0.4) / 2.0).clamp(0.0, 1.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(560.0, 150.0), Sense::hover());
        let painter = ui.painter();
        let alpha = |c: Color32| c.gamma_multiply(appear);
        let center = egui::pos2(rect.left() + 250.0, rect.top() + 52.0);
        spaced_text(painter, center + egui::vec2(3.0, 4.0), "ENGINE JN", 78.0, alpha(Color32::from_black_alpha(220)));
        spaced_text(painter, center, "ENGINE JN", 78.0, alpha(GOLD_LIGHT));
        spaced_text(painter, center + egui::vec2(0.0, -2.0), "ENGINE JN", 78.0, alpha(Color32::from_rgba_unmultiplied(255, 244, 205, 90)));
        let line = rect.top() + 104.0;
        let (a, b) = (rect.left() + 20.0, rect.left() + 480.0);
        if appear > 0.0 {
            let shrink = (1.0 - appear) * 200.0;
            divider(painter, a + shrink, b - shrink, line);
        }
        spaced_text(painter, egui::pos2(center.x, line + 24.0), "FANTASY-INSEL  ·  MULTIPLAYER", 16.0, alpha(PARCHMENT));
    }
}

/// Knopf im Gold-Leder-Stil: dunkles Leder, Goldkante, beim Überfahren warm leuchtend.
pub fn epic_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 46.0), Sense::click());
    let painter = ui.painter();
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    if hovered {
        painter.rect_filled(rect.expand(6.0), 10.0, Color32::from_rgba_unmultiplied(255, 170, 60, 22));
    }
    painter.rect_filled(rect, 6.0, Color32::from_rgb(8, 6, 5));
    let (top, bottom) = if hovered { (Color32::from_rgb(78, 56, 30), Color32::from_rgb(40, 28, 16)) } else { (LEATHER_TOP, LEATHER_BOTTOM) };
    gradient(painter, rect.shrink(2.0), top, bottom, false);
    painter.rect_stroke(rect, 6.0, Stroke::new(if hovered { 1.8 } else { 1.2 }, if hovered { GOLD_LIGHT } else { GOLD_DARK }), StrokeKind::Inside);
    painter.rect_stroke(rect.shrink(3.5), 4.0, Stroke::new(1.0, Color32::from_black_alpha(120)), StrokeKind::Inside);
    for x in [rect.left() + 16.0, rect.right() - 16.0] {
        diamond(painter, egui::pos2(x, rect.center().y), if hovered { 5.0 } else { 4.0 }, if hovered { GOLD_LIGHT } else { GOLD }, GOLD_DARK);
    }
    let offset = if pressed { egui::vec2(0.0, 1.0) } else { egui::Vec2::ZERO };
    let font = FontId::proportional(20.0);
    painter.text(rect.center() + offset + egui::vec2(1.0, 1.5), Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha(200));
    painter.text(rect.center() + offset, Align2::CENTER_CENTER, text, font, if hovered { GOLD_LIGHT } else { PARCHMENT });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
