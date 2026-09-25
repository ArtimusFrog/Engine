//! Inventar im Stil eines Fantasy-Rollenspiels: verziertes Fenster am rechten Bildschirmrand
//! (Taste I) mit Reitern, Plätzen und Tooltips, dazu die kleine Übersicht unten rechts.
//!
//! Die Symbole sind in Blender gerenderte Bilder (`art/icons/gegenstaende.py` →
//! `game/assets/icons/<name>.png`). Fehlt eines, wird das gemalte Ersatzsymbol aus `ui.rs` benutzt.

use std::collections::HashMap;

use engine::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind};
use engine::prelude::Image;

use crate::protocol::{Inventory, Item};
use crate::ui;

const COLUMNS: usize = 6;
const ROWS: usize = 6;
const SLOT: f32 = 52.0;
const GAP: f32 = 6.0;
const MARGIN: f32 = 16.0;
const HEADER: f32 = 62.0;
const TABS: f32 = 30.0;
const FOOTER: f32 = 46.0;

// Farben: dunkles Leder, Bronze und Gold
const GOLD: Color32 = Color32::from_rgb(214, 172, 92);
const GOLD_LIGHT: Color32 = Color32::from_rgb(244, 214, 142);
const GOLD_DARK: Color32 = Color32::from_rgb(104, 78, 38);
const BRONZE: Color32 = Color32::from_rgb(74, 57, 36);
const LEATHER_TOP: Color32 = Color32::from_rgb(38, 29, 22);
const LEATHER_BOTTOM: Color32 = Color32::from_rgb(17, 13, 10);
const PARCHMENT: Color32 = Color32::from_rgb(222, 206, 172);
const MUTED: Color32 = Color32::from_rgb(150, 136, 112);

/// Welche Gegenstände das Fenster zeigt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    All,
    Resources,
    Loot,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::All, Tab::Resources, Tab::Loot];

    fn label(self) -> &'static str {
        match self {
            Tab::All => "Alles",
            Tab::Resources => "Rohstoffe",
            Tab::Loot => "Tierbeute",
        }
    }

    fn shows(self, item: Item) -> bool {
        match self {
            Tab::All => true,
            Tab::Resources => !item.is_loot(),
            Tab::Loot => item.is_loot(),
        }
    }
}

/// Die gerenderten Symbole als egui-Texturen: groß (Fenster) und klein (Übersicht).
struct Icons {
    large: HashMap<Item, egui::TextureHandle>,
    small: HashMap<Item, egui::TextureHandle>,
}

impl Icons {
    fn load(ctx: &egui::Context) -> Icons {
        let mut icons = Icons { large: HashMap::new(), small: HashMap::new() };
        let Some(dir) = crate::asset_files::asset_dir() else { return icons };
        for item in Item::ALL {
            let path = dir.join("icons").join(format!("{}.png", item.icon_file()));
            let image = match Image::load_png(&path) {
                Ok(image) => image,
                Err(message) => {
                    log::warn!("Symbol fehlt: {message} – nehme das gemalte");
                    continue;
                }
            };
            // Verkleinert vorberechnen, sonst flimmern die Kanten (egui hat keine Mipmaps).
            // egui erwartet vormultiplizierte Farben.
            let mut level = image;
            for pixel in level.rgba.chunks_exact_mut(4) {
                let alpha = pixel[3] as f32 / 255.0;
                for c in &mut pixel[..3] {
                    *c = (*c as f32 * alpha).round() as u8;
                }
            }
            while level.width > 128 {
                level = half(&level);
            }
            let options = egui::TextureOptions::LINEAR;
            let texture = |level: &Image| egui::ColorImage::from_rgba_premultiplied([level.width as usize, level.height as usize], &level.rgba);
            icons.large.insert(item, ctx.load_texture(format!("symbol_{}", item.icon_file()), texture(&level), options));
            let small = half(&level);
            icons.small.insert(item, ctx.load_texture(format!("symbol_{}_klein", item.icon_file()), texture(&small), options));
        }
        icons
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect, item: Item) {
        let set = if rect.width() > 40.0 { &self.large } else { &self.small };
        match set.get(&item) {
            Some(texture) => {
                let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                painter.image(texture.id(), rect, uv, Color32::WHITE);
            }
            None => ui::item_icon(painter, rect.shrink(rect.width() * 0.1), item),
        }
    }
}

/// Halbe Größe (Mittelwert aus je 2 × 2 vormultiplizierten Pixeln).
fn half(image: &Image) -> Image {
    let (width, height) = ((image.width / 2).max(1), (image.height / 2).max(1));
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0.0f32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = (((y * 2 + dy).min(image.height - 1) * image.width + (x * 2 + dx).min(image.width - 1)) * 4) as usize;
                for (c, value) in sum.iter_mut().enumerate() {
                    *value += image.rgba[i + c] as f32;
                }
            }
            rgba.extend(sum.iter().map(|v| (v / 4.0).round().min(255.0) as u8));
        }
    }
    Image { width, height, rgba }
}

pub struct InventoryUi {
    icons: Option<Icons>,
    tab: Tab,
}

impl Default for InventoryUi {
    fn default() -> Self {
        InventoryUi { icons: None, tab: Tab::All }
    }
}

impl InventoryUi {
    fn icons(&mut self, ctx: &egui::Context) -> &Icons {
        self.icons.get_or_insert_with(|| Icons::load(ctx))
    }

    /// Das Inventar-Fenster am rechten Rand. Liefert `true`, wenn es geschlossen werden soll.
    pub fn window(&mut self, ctx: &egui::Context, inventory: &Inventory, owner: &str) -> bool {
        self.icons(ctx);
        let width = MARGIN * 2.0 + COLUMNS as f32 * SLOT + (COLUMNS - 1) as f32 * GAP;
        let grid_height = ROWS as f32 * SLOT + (ROWS - 1) as f32 * GAP;
        let height = MARGIN + HEADER + TABS + 12.0 + grid_height + FOOTER + MARGIN * 0.5;
        let mut close = false;

        egui::Area::new(egui::Id::new("inventar_fenster")).anchor(Align2::RIGHT_CENTER, [-22.0, 0.0]).show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            let painter = ui.painter().clone();
            frame(&painter, rect);

            // ---------- Kopf: Titel, Besitzer, Schließen ----------
            let title_center = egui::pos2(rect.center().x, rect.top() + MARGIN + 16.0);
            spaced_text(&painter, title_center + egui::vec2(0.0, 1.5), "INVENTAR", 23.0, Color32::from_black_alpha(200));
            spaced_text(&painter, title_center, "INVENTAR", 23.0, GOLD_LIGHT);
            painter.text(
                title_center + egui::vec2(0.0, 20.0),
                Align2::CENTER_CENTER,
                format!("{owner} · Magier"),
                FontId::proportional(13.0),
                MUTED,
            );
            divider(&painter, rect.left() + MARGIN, rect.right() - MARGIN, rect.top() + MARGIN + HEADER - 8.0);

            let close_center = egui::pos2(rect.right() - MARGIN - 8.0, rect.top() + MARGIN + 10.0);
            let close_rect = Rect::from_center_size(close_center, egui::vec2(24.0, 24.0));
            let response = ui.interact(close_rect, egui::Id::new("inventar_schliessen"), egui::Sense::click());
            close_button(&painter, close_center, response.hovered());
            if response.on_hover_text("Schließen (I oder Esc)").clicked() {
                close = true;
            }

            // ---------- Reiter ----------
            let tabs_top = rect.top() + MARGIN + HEADER;
            let tab_width = (width - MARGIN * 2.0 - 2.0 * 6.0) / 3.0;
            for (index, tab) in Tab::ALL.into_iter().enumerate() {
                let tab_rect = Rect::from_min_size(
                    egui::pos2(rect.left() + MARGIN + index as f32 * (tab_width + 6.0), tabs_top),
                    egui::vec2(tab_width, TABS - 4.0),
                );
                let response = ui.interact(tab_rect, egui::Id::new(("inventar_reiter", index)), egui::Sense::click());
                tab_button(&painter, tab_rect, tab.label(), self.tab == tab, response.hovered());
                if response.clicked() {
                    self.tab = tab;
                }
            }

            // ---------- Plätze ----------
            let items: Vec<(Item, u32)> = inventory.items().filter(|&(item, _)| self.tab.shows(item)).collect();
            let grid_top = tabs_top + TABS + 10.0;
            let icons = self.icons.as_ref().expect("Symbole geladen");
            for slot in 0..COLUMNS * ROWS {
                let (column, row) = (slot % COLUMNS, slot / COLUMNS);
                let slot_rect = Rect::from_min_size(
                    egui::pos2(rect.left() + MARGIN + column as f32 * (SLOT + GAP), grid_top + row as f32 * (SLOT + GAP)),
                    egui::vec2(SLOT, SLOT),
                );
                let filled = items.get(slot).copied();
                let response = ui.interact(slot_rect, egui::Id::new(("inventar_platz", slot)), egui::Sense::hover());
                let hovered = response.hovered() && filled.is_some();
                slot_frame(&painter, slot_rect, hovered, filled.is_some());
                if let Some((item, count)) = filled {
                    icons.paint(&painter, slot_rect.shrink(1.5), item);
                    count_label(&painter, slot_rect.right_bottom() - egui::vec2(4.0, 2.0), count, 14.0);
                    response.on_hover_ui(|ui| tooltip(ui, icons, item, count));
                }
            }

            // ---------- Fuß: belegte Plätze und Hinweis ----------
            let footer_top = grid_top + grid_height + 8.0;
            divider(&painter, rect.left() + MARGIN, rect.right() - MARGIN, footer_top);
            let used = inventory.items().count();
            let total = COLUMNS * ROWS;
            let bar = Rect::from_min_size(egui::pos2(rect.left() + MARGIN, footer_top + 20.0), egui::vec2(110.0, 8.0));
            painter.text(bar.left_top() - egui::vec2(0.0, 3.0), Align2::LEFT_BOTTOM, format!("Plätze {used} / {total}"), FontId::proportional(13.0), PARCHMENT);
            painter.rect_filled(bar, 3.0, Color32::from_rgb(12, 9, 7));
            let mut fill = bar.shrink(1.0);
            fill.set_width(fill.width() * used as f32 / total as f32);
            gradient(&painter, fill, GOLD_LIGHT, GOLD_DARK, true);
            painter.rect_stroke(bar, 3.0, Stroke::new(1.0, BRONZE), StrokeKind::Outside);
            key_hint(&painter, egui::pos2(rect.right() - MARGIN, footer_top + 22.0), "I", "Schließen");
        });
        close
    }

    /// Kleine Übersicht unten rechts, solange das Fenster zu ist.
    pub fn hud(&mut self, ctx: &egui::Context, inventory: &Inventory) {
        self.icons(ctx);
        let icons = self.icons.as_ref().expect("Symbole geladen");
        let shown: Vec<Item> = Item::ALL.into_iter().filter(|&item| matches!(item, Item::Wood | Item::Stone) || inventory.count(item) > 0).collect();
        let slot = 40.0;
        let width = shown.len() as f32 * (slot + 4.0) - 4.0 + 16.0 + 58.0;
        // Über der Tastenhilfe am unteren Rand, damit sich nichts überlappt.
        egui::Area::new(egui::Id::new("inventar")).anchor(Align2::RIGHT_BOTTOM, [-16.0, -40.0]).interactable(false).show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, slot + 16.0), egui::Sense::hover());
            let painter = ui.painter();
            let shadow = egui::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(120) };
            painter.add(shadow.as_shape(rect, 7));
            painter.rect_filled(rect, 7.0, Color32::from_rgb(10, 8, 6));
            gradient(painter, rect.shrink(2.0), LEATHER_TOP, LEATHER_BOTTOM, false);
            painter.rect_stroke(rect.shrink(1.0), 6.0, Stroke::new(1.5, GOLD), StrokeKind::Inside);
            painter.rect_stroke(rect.shrink(3.5), 4.0, Stroke::new(1.0, GOLD_DARK), StrokeKind::Inside);
            for (index, &item) in shown.iter().enumerate() {
                let slot_rect = Rect::from_min_size(rect.left_top() + egui::vec2(8.0 + index as f32 * (slot + 4.0), 8.0), egui::vec2(slot, slot));
                slot_frame(painter, slot_rect, false, true);
                icons.paint(painter, slot_rect.shrink(3.0), item);
                count_label(painter, slot_rect.right_bottom() - egui::vec2(3.0, 1.0), inventory.count(item), 13.0);
            }
            key_hint(painter, egui::pos2(rect.right() - 8.0, rect.center().y), "I", "");
        });
    }
}

// ---------------------------------------------------------------------------
// Malen
// ---------------------------------------------------------------------------

/// Senkrechter (oder waagerechter) Farbverlauf in einem Rechteck.
fn gradient(painter: &egui::Painter, rect: Rect, from: Color32, to: Color32, horizontal: bool) {
    let mut mesh = egui::Mesh::default();
    let (a, b, c, d) = if horizontal { (from, to, to, from) } else { (from, from, to, to) };
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), c);
    mesh.colored_vertex(rect.left_bottom(), d);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Rahmen des Fensters: Schatten, Lederfläche, doppelte Goldkante, Eckbeschläge.
fn frame(painter: &egui::Painter, rect: Rect) {
    let shadow = egui::Shadow { offset: [0, 10], blur: 30, spread: 2, color: Color32::from_black_alpha(150) };
    painter.add(shadow.as_shape(rect, 10));
    painter.rect_filled(rect, 10.0, Color32::from_rgb(10, 8, 6));
    painter.rect_filled(rect.shrink(2.0), 9.0, BRONZE);
    let inner = rect.shrink(6.0);
    gradient(painter, inner, LEATHER_TOP, LEATHER_BOTTOM, false);
    // Leichte Vignette zu den Rändern
    for (side, width) in [(0, 26.0), (1, 26.0)] {
        let band = if side == 0 {
            Rect::from_min_max(inner.left_top(), egui::pos2(inner.left() + width, inner.bottom()))
        } else {
            Rect::from_min_max(egui::pos2(inner.right() - width, inner.top()), inner.right_bottom())
        };
        let (from, to) = if side == 0 { (Color32::from_black_alpha(70), Color32::TRANSPARENT) } else { (Color32::TRANSPARENT, Color32::from_black_alpha(70)) };
        gradient(painter, band, from, to, true);
    }
    painter.rect_stroke(rect.shrink(3.0), 8.0, Stroke::new(2.0, GOLD), StrokeKind::Inside);
    painter.rect_stroke(rect.shrink(6.5), 5.0, Stroke::new(1.0, GOLD_DARK), StrokeKind::Inside);
    for corner in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
        let inward = egui::vec2(if corner.x < rect.center().x { 1.0 } else { -1.0 }, if corner.y < rect.center().y { 1.0 } else { -1.0 });
        let center = corner + inward * 9.0;
        // Winkelbeschlag
        painter.line_segment([center + egui::vec2(inward.x * 4.0, 0.0), center + egui::vec2(inward.x * 26.0, 0.0)], Stroke::new(2.5, GOLD));
        painter.line_segment([center + egui::vec2(0.0, inward.y * 4.0), center + egui::vec2(0.0, inward.y * 26.0)], Stroke::new(2.5, GOLD));
        diamond(painter, center, 8.0, GOLD, GOLD_DARK);
        painter.circle_filled(center, 2.2, Color32::from_rgb(120, 30, 24));
    }
}

/// Raute mit Rand (Eckbeschläge, Trennlinien).
fn diamond(painter: &egui::Painter, center: Pos2, size: f32, fill: Color32, edge: Color32) {
    let points = vec![
        center + egui::vec2(0.0, -size),
        center + egui::vec2(size, 0.0),
        center + egui::vec2(0.0, size),
        center + egui::vec2(-size, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(points, fill, Stroke::new(1.0, edge)));
}

/// Goldene Trennlinie, die zu den Enden ausläuft, mit Raute in der Mitte.
fn divider(painter: &egui::Painter, left: f32, right: f32, y: f32) {
    let middle = (left + right) / 2.0;
    let line = |a: f32, b: f32, from: Color32, to: Color32| gradient(painter, Rect::from_min_max(egui::pos2(a, y - 0.75), egui::pos2(b, y + 0.75)), from, to, true);
    line(left, middle - 8.0, Color32::TRANSPARENT, GOLD);
    line(middle + 8.0, right, GOLD, Color32::TRANSPARENT);
    diamond(painter, egui::pos2(middle, y), 4.5, GOLD_LIGHT, GOLD_DARK);
}

/// Text mit etwas Buchstabenabstand (für Überschriften).
fn spaced_text(painter: &egui::Painter, center: Pos2, text: &str, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::default();
    job.append(text, 0.0, egui::TextFormat { font_id: FontId::proportional(size), color, extra_letter_spacing: 3.0, ..Default::default() });
    let galley = painter.layout_job(job);
    let position = center - galley.size() / 2.0;
    painter.galley(position, galley, color);
}

fn close_button(painter: &egui::Painter, center: Pos2, hovered: bool) {
    let fill = if hovered { Color32::from_rgb(140, 40, 30) } else { Color32::from_rgb(70, 24, 18) };
    painter.circle_filled(center, 11.0, fill);
    painter.circle_stroke(center, 11.0, Stroke::new(1.5, if hovered { GOLD_LIGHT } else { GOLD }));
    let stroke = Stroke::new(2.0, if hovered { Color32::WHITE } else { PARCHMENT });
    painter.line_segment([center + egui::vec2(-4.5, -4.5), center + egui::vec2(4.5, 4.5)], stroke);
    painter.line_segment([center + egui::vec2(4.5, -4.5), center + egui::vec2(-4.5, 4.5)], stroke);
}

fn tab_button(painter: &egui::Painter, rect: Rect, label: &str, selected: bool, hovered: bool) {
    let rounding = CornerRadius { nw: 6, ne: 6, sw: 2, se: 2 };
    if selected {
        painter.rect_filled(rect, rounding, Color32::from_rgb(92, 66, 34));
        gradient(painter, rect.shrink(2.0), Color32::from_rgb(120, 88, 44), Color32::from_rgb(58, 40, 22), false);
        painter.rect_stroke(rect, rounding, Stroke::new(1.5, GOLD), StrokeKind::Inside);
    } else {
        painter.rect_filled(rect, rounding, if hovered { Color32::from_rgb(44, 34, 26) } else { Color32::from_rgb(26, 20, 15) });
        painter.rect_stroke(rect, rounding, Stroke::new(1.0, if hovered { GOLD } else { BRONZE }), StrokeKind::Inside);
    }
    let color = if selected { GOLD_LIGHT } else if hovered { PARCHMENT } else { MUTED };
    painter.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(14.0), color);
}

/// Eingelassener Platz: dunkler Grund, Schatten oben links, Licht unten rechts.
fn slot_frame(painter: &egui::Painter, rect: Rect, hovered: bool, filled: bool) {
    painter.rect_filled(rect, 4.0, Color32::from_rgb(8, 6, 5));
    let inner = rect.shrink(1.0);
    let (top, bottom) = if filled { (Color32::from_rgb(30, 24, 19), Color32::from_rgb(48, 38, 28)) } else { (Color32::from_rgb(16, 12, 10), Color32::from_rgb(26, 20, 16)) };
    gradient(painter, inner, top, bottom, false);
    painter.line_segment([inner.left_top(), inner.right_top()], Stroke::new(1.0, Color32::from_black_alpha(160)));
    painter.line_segment([inner.left_top(), inner.left_bottom()], Stroke::new(1.0, Color32::from_black_alpha(160)));
    painter.line_segment([inner.left_bottom(), inner.right_bottom()], Stroke::new(1.0, Color32::from_white_alpha(14)));
    painter.line_segment([inner.right_top(), inner.right_bottom()], Stroke::new(1.0, Color32::from_white_alpha(14)));
    if hovered {
        painter.rect_stroke(rect.expand(2.0), 5.0, Stroke::new(3.0, Color32::from_rgba_unmultiplied(244, 214, 142, 60)), StrokeKind::Outside);
        painter.rect_stroke(rect, 4.0, Stroke::new(1.5, GOLD_LIGHT), StrokeKind::Inside);
    } else {
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0, if filled { Color32::from_rgb(98, 76, 46) } else { BRONZE }), StrokeKind::Inside);
    }
}

/// Anzahl unten rechts im Platz, mit dunkler Kontur.
fn count_label(painter: &egui::Painter, corner: Pos2, count: u32, size: f32) {
    if count <= 1 {
        return;
    }
    let text = count.to_string();
    let font = FontId::proportional(size);
    for offset in [egui::vec2(1.0, 1.0), egui::vec2(-1.0, 1.0), egui::vec2(1.0, -1.0), egui::vec2(-1.0, -1.0), egui::vec2(0.0, 1.5)] {
        painter.text(corner + offset, Align2::RIGHT_BOTTOM, &text, font.clone(), Color32::BLACK);
    }
    painter.text(corner, Align2::RIGHT_BOTTOM, &text, font, Color32::WHITE);
}

/// Tastenkappe mit Beschriftung, rechtsbündig.
fn key_hint(painter: &egui::Painter, right_center: Pos2, key: &str, label: &str) {
    let mut x = right_center.x;
    if !label.is_empty() {
        let galley = painter.layout_no_wrap(label.to_string(), FontId::proportional(13.0), MUTED);
        x -= galley.size().x;
        painter.galley(egui::pos2(x, right_center.y - galley.size().y / 2.0), galley, MUTED);
        x -= 6.0;
    }
    let cap = Rect::from_min_max(egui::pos2(x - 20.0, right_center.y - 10.0), egui::pos2(x, right_center.y + 10.0));
    painter.rect_filled(cap, 4.0, Color32::from_rgb(40, 31, 22));
    painter.rect_stroke(cap, 4.0, Stroke::new(1.0, GOLD), StrokeKind::Inside);
    painter.line_segment([cap.left_bottom() + egui::vec2(3.0, -1.5), cap.right_bottom() + egui::vec2(-3.0, -1.5)], Stroke::new(1.0, Color32::from_black_alpha(140)));
    painter.text(cap.center(), Align2::CENTER_CENTER, key, FontId::proportional(13.0), GOLD_LIGHT);
}

/// Tooltip wie im Rollenspiel: großes Symbol, Name, Art, Beschreibung, Anzahl.
fn tooltip(ui: &mut egui::Ui, icons: &Icons, item: Item, count: u32) {
    ui.set_max_width(260.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
        slot_frame(ui.painter(), rect, false, true);
        icons.paint(ui.painter(), rect.shrink(3.0), item);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(egui::RichText::new(item.label()).size(18.0).strong().color(Color32::WHITE));
            ui.label(egui::RichText::new(item.kind_line()).size(13.0).color(GOLD));
            ui.label(egui::RichText::new("Gewöhnlich").size(12.0).color(MUTED));
        });
    });
    ui.add_space(2.0);
    ui.label(egui::RichText::new(item.description()).size(14.0).italics().color(PARCHMENT));
    ui.label(egui::RichText::new(format!("Anzahl: {count}")).size(13.0).color(MUTED));
}
