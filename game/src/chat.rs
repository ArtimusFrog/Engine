//! Chat für Mitspieler: Verlauf unten links (blendet nach einer Weile aus), Eingabefeld mit
//! Enter, Sprechblasen über den Köpfen der Mitspieler.

use engine::egui::{self, Align2, Color32, FontId, Stroke};
use engine::prelude::*;

use crate::inventar::{GOLD, GOLD_DARK};
use crate::protocol::{PlayerId, MAX_CHAT_CHARS};
use crate::world::{player_color, ChatLine, World};

/// So lange (Sekunden) bleibt eine Zeile sichtbar, wenn der Chat zu ist; danach blendet sie aus.
const VISIBLE: f32 = 12.0;
const FADE: f32 = 2.0;
/// So lange steht eine Nachricht als Sprechblase über dem Kopf.
const BUBBLE: f32 = 6.0;
const SHOWN_LINES: usize = 8;

/// Was im Chat passiert ist (für die Steuerung).
pub enum ChatAction {
    None,
    Send(String),
    Close,
}

#[derive(Default)]
pub struct ChatUi {
    pub open: bool,
    input: String,
    focus: bool,
    lines: Vec<(ChatLine, f32)>,
}

impl ChatUi {
    /// Neue Zeilen aus der Welt übernehmen.
    pub fn collect(&mut self, world: &mut World, now: f32) {
        for line in world.chat_events.drain(..) {
            self.lines.push((line, now));
        }
        if self.lines.len() > 100 {
            self.lines.drain(..self.lines.len() - 100);
        }
    }

    /// Nur zum Testen/für Screenshots: eine Zeile direkt eintragen.
    pub fn push(&mut self, line: ChatLine, now: f32) {
        self.lines.push((line, now));
    }

    /// Nur für Screenshots: Text ins Eingabefeld legen.
    pub fn prefill(&mut self, text: &str) {
        self.input = text.to_string();
    }

    pub fn start_typing(&mut self) {
        self.open = true;
        self.focus = true;
        self.input.clear();
    }

    /// Letzte Nachricht eines Spielers, solange sie noch als Sprechblase zu sehen ist.
    pub fn bubble(&self, player: PlayerId, now: f32) -> Option<&str> {
        self.lines.iter().rev().find(|(line, at)| line.from == Some(player) && now - at < BUBBLE).map(|(line, _)| line.text.as_str())
    }

    pub fn show(&mut self, egui_ctx: &egui::Context, now: f32) -> ChatAction {
        let mut action = ChatAction::None;
        let open = self.open;
        let visible: Vec<(&ChatLine, f32)> = self
            .lines
            .iter()
            .rev()
            .take(if open { 14 } else { SHOWN_LINES })
            .map(|(line, at)| (line, if open { 1.0 } else { (1.0 - (now - at - VISIBLE) / FADE).clamp(0.0, 1.0) }))
            .filter(|(_, alpha)| *alpha > 0.0)
            .collect();
        if visible.is_empty() && !open {
            return action;
        }
        egui::Area::new(egui::Id::new("chat")).anchor(Align2::LEFT_BOTTOM, [16.0, -16.0]).interactable(open).show(egui_ctx, |ui| {
            let frame = if open {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(17, 13, 10, 225))
                    .stroke(Stroke::new(1.0, GOLD_DARK))
                    .corner_radius(8.0)
                    .inner_margin(10.0)
            } else {
                egui::Frame::new().inner_margin(10.0)
            };
            frame.show(ui, |ui| {
                ui.set_width(380.0);
                ui.spacing_mut().item_spacing.y = 3.0;
                for (line, alpha) in visible.iter().rev() {
                    let text = chat_text(line, *alpha);
                    let label = egui::Label::new(text).wrap();
                    if open {
                        ui.add(label);
                    } else {
                        // Geschlossen: jede Zeile auf einem dezenten Schatten, damit man sie überall lesen kann
                        egui::Frame::new()
                            .fill(Color32::from_black_alpha((110.0 * alpha) as u8))
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.add(label);
                            });
                    }
                }
                if open {
                    ui.add_space(4.0);
                    let edit = egui::TextEdit::singleline(&mut self.input)
                        .char_limit(MAX_CHAT_CHARS)
                        .hint_text("Nachricht … (Enter senden, Esc abbrechen)")
                        .desired_width(f32::INFINITY)
                        .font(FontId::proportional(16.0));
                    let response = ui.add(edit);
                    if self.focus {
                        response.request_focus();
                        self.focus = false;
                    }
                    if response.lost_focus() {
                        action = if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.input.trim().is_empty() {
                            ChatAction::Send(std::mem::take(&mut self.input))
                        } else {
                            ChatAction::Close
                        };
                        self.open = false;
                    }
                }
            });
        });
        action
    }
}

/// Eine Zeile: Name in der Farbe des Spielers, Hinweise kursiv in Gold.
fn chat_text(line: &ChatLine, alpha: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let fade = |c: Color32| c.gamma_multiply(alpha);
    let font = FontId::proportional(15.0);
    match line.from {
        Some(id) => {
            let color = srgb(player_color(id)).lerp_to_gamma(Color32::WHITE, 0.35);
            job.append(&format!("{}: ", line.name), 0.0, egui::TextFormat { font_id: font.clone(), color: fade(color), ..Default::default() });
            job.append(&line.text, 0.0, egui::TextFormat { font_id: font, color: fade(Color32::from_rgb(240, 236, 228)), ..Default::default() });
        }
        None => {
            job.append(&line.text, 0.0, egui::TextFormat { font_id: font, color: fade(GOLD), italics: true, ..Default::default() });
        }
    }
    job
}

fn srgb(c: Vec3) -> Color32 {
    let f = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    };
    Color32::from_rgb(f(c.x), f(c.y), f(c.z))
}

/// Sprechblase über einem Kopf (Weltposition), blendet mit der Entfernung aus.
pub fn speech_bubble(ctx: &Context, egui_ctx: &egui::Context, position: Vec3, text: &str) {
    let distance = ctx.camera.position.distance(position);
    let alpha = (1.0 - (distance - 18.0) / 20.0).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let Some(screen) = ctx.world_to_screen(position) else { return };
    let painter = egui_ctx.layer_painter(egui::LayerId::background());
    let galley = painter.layout(text.to_string(), FontId::proportional(15.0), Color32::from_rgb(40, 30, 20).gamma_multiply(alpha), 220.0);
    let size = galley.size() + egui::vec2(16.0, 10.0);
    let bottom = egui::pos2(screen.x, screen.y);
    let rect = egui::Rect::from_min_size(bottom - egui::vec2(size.x / 2.0, size.y + 8.0), size);
    let paper = Color32::from_rgb(246, 238, 220).gamma_multiply(alpha);
    painter.rect_filled(rect, 8.0, paper);
    painter.rect_stroke(rect, 8.0, Stroke::new(1.5, GOLD_DARK.gamma_multiply(alpha)), egui::StrokeKind::Inside);
    // Spitze nach unten
    let tip = vec![bottom - egui::vec2(6.0, 8.5), bottom - egui::vec2(-6.0, 8.5), bottom];
    painter.add(egui::Shape::convex_polygon(tip, paper, Stroke::NONE));
    painter.galley(rect.min + egui::vec2(8.0, 5.0), galley, Color32::BLACK);
}
