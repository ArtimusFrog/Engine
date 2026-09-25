//! Übersichtskarte (Taste M), fast bildschirmfüllend: die gezeichnete Insel (siehe
//! `island::map_image`) mit Ortsnamen, Startplatz, Kristallvorkommen, Mitspielern und dem eigenen
//! Standort als goldenem Pfeil. Mausrad zoomt (dorthin, wo die Maus steht), Ziehen verschiebt,
//! Doppelklick zeigt wieder die ganze Insel. Rechts eine Seitenleiste mit Spielern, Orten und Legende.

use engine::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};
use engine::prelude::*;

use crate::inventar::{close_button, diamond, divider, frame, key_hint, spaced_text, GOLD, GOLD_DARK, GOLD_LIGHT, MUTED, PARCHMENT};
use crate::island::{landmarks, MAP_EXTENT};
use crate::protocol::PlayerId;
use crate::world::{player_color, World};

const MARGIN: f32 = 18.0;
const HEADER: f32 = 50.0;
const SIDEBAR: f32 = 250.0;
const MAX_ZOOM: f32 = 6.0;
const INK: Color32 = Color32::from_rgb(58, 38, 20);
const CRYSTAL: Color32 = Color32::from_rgb(110, 205, 255);
const CRYSTAL_EDGE: Color32 = Color32::from_rgb(20, 50, 90);

pub struct MapUi {
    texture: Option<egui::TextureHandle>,
    /// Welt-Punkt (x, z) in der Mitte der Ansicht und Zoom (1 = ganze Insel).
    center: Vec2,
    zoom: f32,
}

impl Default for MapUi {
    fn default() -> Self {
        MapUi { texture: None, center: Vec2::ZERO, zoom: 1.0 }
    }
}

impl MapUi {
    /// Ansicht auf einen Punkt richten (z. B. für Screenshots).
    pub fn focus(&mut self, center: Vec2, zoom: f32) {
        self.center = center;
        self.zoom = zoom.clamp(1.0, MAX_ZOOM);
    }

    /// Zeigt das Kartenfenster. Liefert `true`, wenn es geschlossen werden soll.
    pub fn show(&mut self, ctx: &Context, egui_ctx: &egui::Context, world: &World, local: Option<PlayerId>) -> bool {
        let Some(image) = &world.map else { return false };
        let texture = self
            .texture
            .get_or_insert_with(|| {
                let color = egui::ColorImage::from_rgba_unmultiplied([image.width as usize, image.height as usize], &image.rgba);
                egui_ctx.load_texture("uebersichtskarte", color, egui::TextureOptions::LINEAR)
            })
            .clone();
        let screen = egui_ctx.content_rect();
        let window = screen.shrink2(egui::vec2(28.0, 22.0));
        let mut close = false;
        let local_position = local.and_then(|id| world.player_position(ctx, id)).map(|p| vec2(p.x, p.z));

        egui::Area::new(egui::Id::new("karte")).fixed_pos(window.min).show(egui_ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(window.size(), egui::Sense::hover());
            let painter = ui.painter().clone();
            frame(&painter, rect);
            let title = egui::pos2(rect.center().x, rect.top() + MARGIN + 12.0);
            spaced_text(&painter, title + egui::vec2(0.0, 1.5), "KARTE DER INSEL", 24.0, Color32::from_black_alpha(200));
            spaced_text(&painter, title, "KARTE DER INSEL", 24.0, GOLD_LIGHT);
            divider(&painter, rect.left() + MARGIN, rect.right() - MARGIN, rect.top() + HEADER - 4.0);
            let close_center = egui::pos2(rect.right() - MARGIN - 8.0, rect.top() + MARGIN + 10.0);
            let response = ui.interact(Rect::from_center_size(close_center, egui::vec2(24.0, 24.0)), egui::Id::new("karte_schliessen"), egui::Sense::click());
            close_button(&painter, close_center, response.hovered());
            if response.on_hover_text("Schließen (M oder Esc)").clicked() {
                close = true;
            }

            // Aufteilung: Karte links, Seitenleiste rechts (bei schmalen Fenstern nur die Karte)
            let body = Rect::from_min_max(egui::pos2(rect.left() + MARGIN, rect.top() + HEADER + 6.0), egui::pos2(rect.right() - MARGIN, rect.bottom() - MARGIN));
            let with_sidebar = body.width() > body.height() * 0.9 + SIDEBAR;
            let view = if with_sidebar { Rect::from_min_max(body.min, egui::pos2(body.right() - SIDEBAR - 14.0, body.bottom())) } else { body };

            // ---------- Zoomen und Verschieben ----------
            let fit = view.width().min(view.height());
            let scale = |zoom: f32| fit / (2.0 * MAP_EXTENT) * zoom;
            let interaction = ui.interact(view, egui::Id::new("karte_ansicht"), egui::Sense::click_and_drag());
            if interaction.dragged() {
                let delta = interaction.drag_delta();
                self.center -= vec2(delta.x, delta.y) / scale(self.zoom);
            }
            if interaction.double_clicked() {
                self.center = Vec2::ZERO;
                self.zoom = 1.0;
            }
            if interaction.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll.abs() > 0.0 {
                    if let Some(pointer) = interaction.hover_pos() {
                        // Der Punkt unter der Maus bleibt beim Zoomen, wo er ist.
                        let offset = vec2(pointer.x - view.center().x, pointer.y - view.center().y);
                        let before = self.center + offset / scale(self.zoom);
                        self.zoom = (self.zoom * (scroll * 0.004).exp()).clamp(1.0, MAX_ZOOM);
                        self.center = before - offset / scale(self.zoom);
                    }
                }
            }
            self.center = self.center.clamp(Vec2::splat(-MAP_EXTENT), Vec2::splat(MAP_EXTENT));
            let s = scale(self.zoom);
            let (center, view_center) = (self.center, view.center());
            let to_screen = move |p: Vec2| view_center + egui::vec2(p.x - center.x, p.y - center.y) * s;

            // ---------- Karte ----------
            // Außerhalb des Bildes in der Randfarbe der Karte weitermalen (keine sichtbare Kante)
            let edge = Color32::from_rgb(image.rgba[0], image.rgba[1], image.rgba[2]);
            painter.rect_filled(view, 2.0, edge);
            let clip = painter.with_clip_rect(view);
            let image_rect = Rect::from_min_max(to_screen(Vec2::splat(-MAP_EXTENT)), to_screen(Vec2::splat(MAP_EXTENT)));
            clip.image(texture.id(), image_rect, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);

            let crystal_size = 3.5 + self.zoom.sqrt() * 1.5;
            for crystal in world.crystals() {
                diamond(&clip, to_screen(vec2(crystal.x, crystal.z)), crystal_size, CRYSTAL, CRYSTAL_EDGE);
            }
            for (name, place) in landmarks() {
                label(&clip, to_screen(place), name, 16.0 + self.zoom.sqrt() * 2.0, INK);
            }
            let start = to_screen(vec2(world.spawn.x, world.spawn.z));
            clip.circle_filled(start, 7.0, GOLD);
            clip.circle_stroke(start, 7.0, Stroke::new(2.0, INK));
            clip.circle_filled(start, 2.5, INK);
            label(&clip, start + egui::vec2(0.0, 17.0), "Startplatz", 14.0, INK);

            let now = ctx.time.elapsed;
            let mut players: Vec<(PlayerId, String, Vec2)> = Vec::new();
            for (&id, avatar) in &world.players {
                let Some(position) = world.player_position(ctx, id) else { continue };
                players.push((id, avatar.name.clone(), vec2(position.x, position.z)));
                if Some(id) == local {
                    continue;
                }
                let at = to_screen(vec2(position.x, position.z));
                clip.circle_filled(at, 6.5, srgb(player_color(id)));
                clip.circle_stroke(at, 6.5, Stroke::new(2.0, Color32::WHITE));
                label(&clip, at + egui::vec2(0.0, -16.0), &avatar.name, 14.0, INK);
            }
            if let (Some(position), Some(facing)) = (local_position, local.and_then(|id| world.players.get(&id)).map(|a| a.facing)) {
                let at = to_screen(position);
                let pulse = (now * 2.5).sin() * 0.5 + 0.5;
                clip.circle_stroke(at, 13.0 + pulse * 6.0, Stroke::new(2.0, Color32::from_rgba_unmultiplied(244, 214, 142, (150.0 * (1.0 - pulse)) as u8)));
                arrow(&clip, at, facing, 1.25);
            }
            compass(&clip, view.right_top() + egui::vec2(-40.0, 44.0));
            // Maßstab unten links
            scale_bar(&clip, view.left_bottom() + egui::vec2(20.0, -22.0), s);
            painter.rect_stroke(view.expand(1.0), 2.0, Stroke::new(2.0, GOLD), egui::StrokeKind::Outside);
            painter.rect_stroke(view.expand(4.0), 3.0, Stroke::new(1.0, GOLD_DARK), egui::StrokeKind::Outside);

            // ---------- Seitenleiste ----------
            if with_sidebar {
                let side = Rect::from_min_max(egui::pos2(view.right() + 14.0, body.top()), body.max);
                ui.scope_builder(egui::UiBuilder::new().max_rect(side), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let heading = |ui: &mut egui::Ui, text: &str| {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(text).size(15.0).strong().color(GOLD_LIGHT));
                    };
                    let button = |ui: &mut egui::Ui, text: &str| ui.add_sized([side.width(), 30.0], egui::Button::new(egui::RichText::new(text).size(15.0)));
                    if button(ui, "Auf mich zentrieren").clicked() {
                        if let Some(position) = local_position {
                            self.center = position;
                            self.zoom = self.zoom.max(3.0);
                        }
                    }
                    if button(ui, "Ganze Insel").clicked() {
                        self.center = Vec2::ZERO;
                        self.zoom = 1.0;
                    }

                    heading(ui, &format!("Spieler ({})", players.len()));
                    players.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
                    // Eintrag mit gemaltem Zeichen davor; ein Klick zeigt ihn auf der Karte.
                    let entry = |ui: &mut egui::Ui, draw: &dyn Fn(&egui::Painter, Pos2), text: egui::RichText| -> bool {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                            draw(ui.painter(), rect.center());
                            ui.add(egui::Label::new(text).sense(egui::Sense::click())).on_hover_text("Auf der Karte zeigen").clicked()
                        })
                        .inner
                    };
                    for (id, name, position) in &players {
                        let me = Some(*id) == local;
                        let color = if me { GOLD } else { srgb(player_color(*id)) };
                        let text = egui::RichText::new(if me { format!("{name}  (du)") } else { name.clone() }).size(15.0).color(color);
                        if entry(ui, &|p, at| { p.circle_filled(at, 5.0, color); }, text) {
                            self.center = *position;
                            self.zoom = self.zoom.max(3.0);
                        }
                    }

                    heading(ui, "Orte");
                    for (name, place) in landmarks().into_iter().chain([("Startplatz", vec2(world.spawn.x, world.spawn.z))]) {
                        let text = egui::RichText::new(name).size(15.0).color(PARCHMENT);
                        if entry(ui, &|p, at| diamond(p, at, 4.5, GOLD, GOLD_DARK), text) {
                            self.center = place;
                            self.zoom = self.zoom.max(2.5);
                        }
                    }

                    heading(ui, "Legende");
                    let row = |ui: &mut egui::Ui, draw: &dyn Fn(&egui::Painter, Pos2), text: &str| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 20.0), egui::Sense::hover());
                            draw(ui.painter(), rect.center());
                            ui.label(egui::RichText::new(text).size(14.0).color(MUTED));
                        });
                    };
                    row(ui, &|p, at| arrow(p, at, 0.0, 1.0), "Du (mit Blickrichtung)");
                    row(ui, &|p, at| {
                        p.circle_filled(at, 5.5, Color32::from_rgb(90, 160, 230));
                        p.circle_stroke(at, 5.5, Stroke::new(1.5, Color32::WHITE));
                    }, "Mitspieler");
                    row(ui, &|p, at| diamond(p, at, 5.0, CRYSTAL, CRYSTAL_EDGE), "Magische Kristalle");
                    row(ui, &|p, at| {
                        p.circle_filled(at, 5.5, GOLD);
                        p.circle_stroke(at, 5.5, Stroke::new(1.5, INK));
                    }, "Startplatz");
                    row(ui, &|p, at| {
                        p.line_segment([at - egui::vec2(8.0, 0.0), at + egui::vec2(8.0, 0.0)], Stroke::new(3.0, Color32::from_rgb(120, 75, 35)));
                    }, "Trampelpfad");
                    row(ui, &|p, at| {
                        p.circle_filled(at, 3.0, Color32::from_rgb(28, 64, 26));
                    }, "Baum");

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Mausrad: zoomen\nZiehen: verschieben\nDoppelklick: ganze Insel").size(13.0).color(MUTED));
                });
                key_hint(&painter, egui::pos2(side.right(), side.bottom() - 12.0), "M", "Schließen");
            }
        });
        close
    }
}

/// Lineare Farbe → egui-Farbe (sRGB).
fn srgb(c: Vec3) -> Color32 {
    let f = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    };
    Color32::from_rgb(f(c.x), f(c.y), f(c.z))
}

/// Beschriftung mit heller Kontur (gut lesbar auf Pergament).
fn label(painter: &egui::Painter, at: Pos2, text: &str, size: f32, color: Color32) {
    let font = FontId::proportional(size);
    for offset in [egui::vec2(1.0, 0.0), egui::vec2(-1.0, 0.0), egui::vec2(0.0, 1.0), egui::vec2(0.0, -1.0)] {
        painter.text(at + offset, Align2::CENTER_CENTER, text, font.clone(), Color32::from_rgba_unmultiplied(245, 232, 200, 200));
    }
    painter.text(at, Align2::CENTER_CENTER, text, font, color);
}

/// Goldener Pfeil in Blickrichtung (`facing` 0 = Norden/oben).
fn arrow(painter: &egui::Painter, at: Pos2, facing: f32, size: f32) {
    let forward = egui::vec2(facing.sin(), -facing.cos()) * size;
    let side = egui::vec2(-forward.y, forward.x);
    let points = vec![at + forward * 10.0, at - forward * 6.0 + side * 7.0, at - forward * 2.5, at - forward * 6.0 - side * 7.0];
    painter.add(egui::Shape::convex_polygon(vec![points[0], points[1], points[2]], GOLD_LIGHT, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(vec![points[0], points[2], points[3]], GOLD, Stroke::NONE));
    painter.add(egui::Shape::closed_line(points, Stroke::new(1.5, INK)));
}

/// Kleine Kompassrose mit N oben.
fn compass(painter: &egui::Painter, center: Pos2) {
    painter.circle_filled(center, 24.0, Color32::from_rgba_unmultiplied(245, 232, 200, 180));
    painter.circle_stroke(center, 24.0, Stroke::new(1.5, INK));
    for (dir, long) in [(egui::vec2(0.0, -1.0), true), (egui::vec2(0.0, 1.0), true), (egui::vec2(1.0, 0.0), false), (egui::vec2(-1.0, 0.0), false)] {
        let side = egui::vec2(-dir.y, dir.x);
        let tip = center + dir * if long { 19.0 } else { 13.0 };
        let fill = if dir.y < 0.0 { Color32::from_rgb(170, 40, 30) } else { INK };
        painter.add(egui::Shape::convex_polygon(vec![tip, center + side * 4.5, center - side * 4.5], fill, Stroke::NONE));
    }
    painter.text(center + egui::vec2(0.0, -34.0), Align2::CENTER_CENTER, "N", FontId::proportional(15.0), INK);
}

/// Maßstabsleiste mit einer runden Meterzahl (`pixels_per_meter` = aktueller Maßstab).
fn scale_bar(painter: &egui::Painter, left_bottom: Pos2, pixels_per_meter: f32) {
    let meters = [10.0f32, 25.0, 50.0, 100.0, 200.0].into_iter().find(|m| m * pixels_per_meter >= 70.0).unwrap_or(200.0);
    let width = meters * pixels_per_meter;
    let y = left_bottom.y;
    let back = Rect::from_min_max(egui::pos2(left_bottom.x - 8.0, y - 26.0), egui::pos2(left_bottom.x + width + 8.0, y + 8.0));
    painter.rect_filled(back, 4.0, Color32::from_rgba_unmultiplied(245, 232, 200, 170));
    painter.line_segment([egui::pos2(left_bottom.x, y), egui::pos2(left_bottom.x + width, y)], Stroke::new(3.0, INK));
    for x in [left_bottom.x, left_bottom.x + width] {
        painter.line_segment([egui::pos2(x, y - 6.0), egui::pos2(x, y + 3.0)], Stroke::new(2.0, INK));
    }
    painter.text(egui::pos2(left_bottom.x + width / 2.0, y - 14.0), Align2::CENTER_CENTER, format!("{meters:.0} m"), FontId::proportional(13.0), INK);
}
