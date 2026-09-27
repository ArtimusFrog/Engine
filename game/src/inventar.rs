//! Inventar im Stil eines Fantasy-Rollenspiels: verziertes Fenster am rechten Bildschirmrand
//! (Taste I) mit Reitern, Plätzen und Tooltips, dazu die kleine Übersicht unten rechts.
//!
//! Die Symbole sind in Blender gerenderte Bilder (`art/icons/gegenstaende.py` →
//! `game/assets/icons/<name>.png`). Fehlt eines, wird das gemalte Ersatzsymbol aus `ui.rs` benutzt.

use std::collections::HashMap;

use engine::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind};
use engine::prelude::Image;

use crate::protocol::{Inventory, Item, Tool};
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
pub(crate) const GOLD: Color32 = Color32::from_rgb(214, 172, 92);
pub(crate) const GOLD_LIGHT: Color32 = Color32::from_rgb(244, 214, 142);
pub(crate) const GOLD_DARK: Color32 = Color32::from_rgb(104, 78, 38);
pub(crate) const BRONZE: Color32 = Color32::from_rgb(74, 57, 36);
pub(crate) const LEATHER_TOP: Color32 = Color32::from_rgb(38, 29, 22);
pub(crate) const LEATHER_BOTTOM: Color32 = Color32::from_rgb(17, 13, 10);
pub(crate) const PARCHMENT: Color32 = Color32::from_rgb(222, 206, 172);
pub(crate) const MUTED: Color32 = Color32::from_rgb(150, 136, 112);

/// Welche Gegenstände das Fenster zeigt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    All,
    Resources,
    Loot,
    Waffen,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::All, Tab::Resources, Tab::Loot, Tab::Waffen];

    fn label(self) -> &'static str {
        match self {
            Tab::All => "Alles",
            Tab::Resources => "Rohstoffe",
            Tab::Loot => "Tierbeute",
            Tab::Waffen => "Waffen",
        }
    }

    fn shows(self, item: Item) -> bool {
        match self {
            Tab::All => true,
            Tab::Resources => !item.is_loot(),
            Tab::Loot => item.is_loot(),
            Tab::Waffen => false,
        }
    }
}

/// Symbol der Startwaffe einer Klasse.
fn startwaffe_symbol(class: crate::protocol::CharacterClass) -> &'static str {
    match class {
        crate::protocol::CharacterClass::Zwerg => "schmiedehammer",
        crate::protocol::CharacterClass::Bogenschuetze => "jagdbogen",
        _ => "zauberstab",
    }
}

fn seltenheit_farbe(s: crate::waffen::Seltenheit) -> Color32 {
    let f = s.farbe();
    Color32::from_rgb((f[0] * 255.0) as u8, (f[1] * 255.0) as u8, (f[2] * 255.0) as u8)
}

/// Die gerenderten Symbole (Gegenstände und Werkzeuge) als egui-Texturen, nach Dateinamen:
/// groß (Fenster, Auswahlleiste) und klein (Übersicht).
struct Icons {
    large: HashMap<&'static str, egui::TextureHandle>,
    small: HashMap<&'static str, egui::TextureHandle>,
}

impl Icons {
    fn load(ctx: &egui::Context) -> Icons {
        let mut icons = Icons { large: HashMap::new(), small: HashMap::new() };
        let Some(dir) = crate::asset_files::asset_dir() else { return icons };
        let werkzeuge = [Tool::Pickaxe, Tool::Axe].map(|t| t.icon_file(crate::protocol::CharacterClass::Mage));
        let faehigkeiten = crate::protocol::CharacterClass::ALL.into_iter().flat_map(crate::faehigkeiten::Faehigkeit::der_klasse).map(|f| f.icon_file());
        let waffen = crate::waffen::WAFFEN.iter().map(|w| w.datei).chain(["schmiedehammer", "zauberstab"]);
        let files = Item::ALL.iter().map(|i| i.icon_file()).chain(werkzeuge).chain(faehigkeiten).chain(waffen);
        for file in files {
            let path = dir.join("icons").join(format!("{file}.png"));
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
            icons.large.insert(file, ctx.load_texture(format!("symbol_{file}"), texture(&level), options));
            let small = half(&level);
            icons.small.insert(file, ctx.load_texture(format!("symbol_{file}_klein"), texture(&small), options));
        }
        icons
    }

    /// Malt ein Symbol; `true`, wenn es das Bild gab.
    fn paint_file(&self, painter: &egui::Painter, rect: Rect, file: &str, tint: Color32) -> bool {
        let set = if rect.width() > 40.0 { &self.large } else { &self.small };
        let Some(texture) = set.get(file) else { return false };
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), rect, uv, tint);
        true
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect, item: Item) {
        if !self.paint_file(painter, rect, item.icon_file(), Color32::WHITE) {
            ui::item_icon(painter, rect.shrink(rect.width() * 0.1), item);
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
    /// Welcher Platz der Auswahlleiste seit wann gewählt ist (für den Hinweis zur Fähigkeit)
    auswahl: (usize, f64),
    icons: Option<Icons>,
    tab: Tab,
    /// Waffe, die im Fenster angeklickt wurde (das Spiel holt sie ab und rüstet sie aus)
    pub ausruesten: Option<u8>,
}

impl Default for InventoryUi {
    fn default() -> Self {
        InventoryUi { icons: None, tab: Tab::All, ausruesten: None, auswahl: (usize::MAX, 0.0) }
    }
}

impl InventoryUi {
    fn icons(&mut self, ctx: &egui::Context) -> &Icons {
        self.icons.get_or_insert_with(|| Icons::load(ctx))
    }

    /// Das Inventar-Fenster am rechten Rand. Liefert `true`, wenn es geschlossen werden soll.
    pub fn window(&mut self, ctx: &egui::Context, inventory: &Inventory, owner: &str, class: crate::protocol::CharacterClass) -> bool {
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
                format!("{owner} · {}", class.label()),
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
            let tab_width = (width - MARGIN * 2.0 - 3.0 * 6.0) / 4.0;
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
            // Waffen: Startwaffe und alle erbeuteten; Klick rüstet aus
            let waffen: Vec<u8> = std::iter::once(0).chain(crate::waffen::WAFFEN.iter().filter(|w| inventory.waffen & (1u16 << w.id) != 0).map(|w| w.id)).collect();
            for slot in 0..COLUMNS * ROWS {
                if self.tab != Tab::Waffen {
                    break;
                }
                let (column, row) = (slot % COLUMNS, slot / COLUMNS);
                let slot_rect = Rect::from_min_size(
                    egui::pos2(rect.left() + MARGIN + column as f32 * (SLOT + GAP), grid_top + row as f32 * (SLOT + GAP)),
                    egui::vec2(SLOT, SLOT),
                );
                let waffe = waffen.get(slot).copied();
                let response = ui.interact(slot_rect, egui::Id::new(("inventar_waffe", slot)), egui::Sense::click());
                slot_frame(&painter, slot_rect, response.hovered() && waffe.is_some(), waffe.is_some());
                let Some(id) = waffe else { continue };
                let w = crate::waffen::waffe(id);
                let eigene = w.is_none_or(|w| w.klasse == class);
                let getragen = (id == 0 && crate::waffen::ausgeruestet(inventory.waffe, class).is_none()) || (id != 0 && inventory.waffe == id && eigene);
                if let Some(w) = w {
                    painter.rect_stroke(slot_rect.shrink(1.0), 4.0, Stroke::new(2.0, seltenheit_farbe(w.seltenheit)), StrokeKind::Inside);
                }
                let datei = w.map_or(startwaffe_symbol(class), |w| w.datei);
                let tint = if eigene { Color32::WHITE } else { Color32::from_gray(90) };
                if !icons.paint_file(&painter, slot_rect.shrink(3.0), datei, tint) {
                    let name = w.map_or(crate::waffen::startwaffe(class), |w| w.name);
                    painter.text(slot_rect.center(), Align2::CENTER_CENTER, &name[..1], FontId::proportional(20.0), PARCHMENT);
                }
                if getragen {
                    painter.rect_stroke(slot_rect.expand(2.0), 6.0, Stroke::new(2.5, GOLD_LIGHT), StrokeKind::Outside);
                    painter.text(slot_rect.right_top() + egui::vec2(-4.0, 2.0), Align2::RIGHT_TOP, "✔", FontId::proportional(14.0), GOLD_LIGHT);
                }
                if response.clicked() && eigene && !getragen {
                    self.ausruesten = Some(id);
                }
                response.on_hover_ui(|ui| {
                    ui.set_max_width(280.0);
                    match w {
                        Some(w) => {
                            ui.label(egui::RichText::new(w.name).size(18.0).strong().color(seltenheit_farbe(w.seltenheit)));
                            ui.label(egui::RichText::new(format!("{} · {}", w.seltenheit.label(), match w.klasse {
                                crate::protocol::CharacterClass::Zwerg => "Hammer (Zwerg)",
                                crate::protocol::CharacterClass::Bogenschuetze => "Bogen (Bogenschütze)",
                                _ => "Stab (Magier)",
                            })).size(13.0).color(GOLD));
                            ui.label(egui::RichText::new(crate::waffen::werte_zeile(w)).size(14.0).color(Color32::WHITE));
                            ui.label(egui::RichText::new(w.beschreibung).size(13.0).italics().color(PARCHMENT));
                        }
                        None => {
                            ui.label(egui::RichText::new(crate::waffen::startwaffe(class)).size(18.0).strong().color(Color32::WHITE));
                            ui.label(egui::RichText::new("Startwaffe · keine Boni").size(13.0).color(GOLD));
                        }
                    }
                    let zeile = if getragen { "In der Hand" } else if eigene { "Klick: ausrüsten" } else { "Passt nicht zu deiner Figur" };
                    ui.label(egui::RichText::new(zeile).size(13.0).color(MUTED));
                });
            }
            for slot in 0..COLUMNS * ROWS {
                if self.tab == Tab::Waffen {
                    break;
                }
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

    /// Auswahlleiste unten in der Mitte: Werkzeuge und die drei Fähigkeiten der Figur, der gewählte
    /// Platz leuchtet. `abklingen`: Restzeit der Fähigkeiten in Sekunden (0 = bereit);
    /// `punkte`: je Fähigkeit (gefüllt, von) – arkane Ladungen bzw. Stand der Kombo.
    pub fn hotbar(&mut self, ctx: &egui::Context, selected: usize, class: crate::protocol::CharacterClass, abklingen: [f32; 4], punkte: [Option<(u8, u8)>; 4]) {
        self.icons(ctx);
        let jetzt = ctx.input(|i| i.time);
        if self.auswahl.0 != selected {
            self.auswahl = (selected, jetzt);
        }
        let seit_auswahl = (jetzt - self.auswahl.1) as f32;
        let icons = self.icons.as_ref().expect("Symbole geladen");
        const SLOTS: usize = 8;
        let slot = 50.0;
        let gap = 5.0;
        let width = SLOTS as f32 * slot + (SLOTS - 1) as f32 * gap + 20.0;
        egui::Area::new(egui::Id::new("auswahlleiste")).anchor(Align2::CENTER_BOTTOM, [0.0, -14.0]).interactable(false).show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, slot + 20.0), egui::Sense::hover());
            let painter = ui.painter();
            let shadow = egui::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(130) };
            painter.add(shadow.as_shape(rect, 8));
            painter.rect_filled(rect, 8.0, Color32::from_rgb(10, 8, 6));
            gradient(painter, rect.shrink(2.0), LEATHER_TOP, LEATHER_BOTTOM, false);
            painter.rect_stroke(rect.shrink(1.0), 7.0, Stroke::new(1.5, GOLD), StrokeKind::Inside);
            painter.rect_stroke(rect.shrink(3.5), 5.0, Stroke::new(1.0, GOLD_DARK), StrokeKind::Inside);
            for side in [rect.left_center(), rect.right_center()] {
                diamond(painter, side, 7.0, GOLD, GOLD_DARK);
            }
            for index in 0..SLOTS {
                let slot_rect = Rect::from_min_size(rect.left_top() + egui::vec2(10.0 + index as f32 * (slot + gap), 10.0), egui::vec2(slot, slot));
                let tool = Tool::HOTBAR.get(index).copied();
                let active = index == selected && tool.is_some();
                if active {
                    // Goldener Schein um den gewählten Platz
                    for (grow, alpha) in [(6.0, 30), (3.5, 60)] {
                        painter.rect_stroke(slot_rect.expand(grow), 7.0, Stroke::new(3.0, Color32::from_rgba_unmultiplied(244, 214, 142, alpha)), StrokeKind::Outside);
                    }
                }
                slot_frame(painter, slot_rect, active, tool.is_some());
                // Ultimative Fähigkeit: goldener Doppelrahmen, pulsiert, sobald sie bereit ist
                if let Some(Tool::Faehigkeit(platz)) = tool {
                    if tool.and_then(|t| t.faehigkeit(class)).is_some_and(|f| f.ist_ultimativ()) {
                        let bereit = abklingen[(platz as usize).min(3)] <= 0.0;
                        let puls = if bereit { 0.5 + 0.5 * (jetzt as f32 * 4.0).sin() } else { 0.0 };
                        if bereit {
                            painter.rect_stroke(slot_rect.expand(3.0 + 2.0 * puls), 6.0, Stroke::new(2.5, Color32::from_rgba_unmultiplied(255, 180, 60, (90.0 + 120.0 * puls) as u8)), StrokeKind::Outside);
                        }
                        painter.rect_stroke(slot_rect.shrink(1.0), 4.0, Stroke::new(1.5, Color32::from_rgb(255, 170, 60)), StrokeKind::Inside);
                        for ecke in [slot_rect.left_top(), slot_rect.right_top(), slot_rect.left_bottom(), slot_rect.right_bottom()] {
                            diamond(painter, ecke, 4.0, Color32::from_rgb(255, 190, 80), GOLD_DARK);
                        }
                    }
                }
                if active {
                    painter.rect_stroke(slot_rect, 4.0, Stroke::new(2.0, GOLD_LIGHT), StrokeKind::Inside);
                }
                if let Some(tool) = tool {
                    let tint = if active { Color32::WHITE } else { Color32::from_gray(185) };
                    if !icons.paint_file(painter, slot_rect.shrink(3.0), tool.icon_file(class), tint) {
                        let erster: String = tool.label(class).chars().take(1).collect();
                        painter.text(slot_rect.center(), Align2::CENTER_CENTER, erster, FontId::proportional(20.0), PARCHMENT);
                    }
                    // Abklingzeit: dunkler Schleier von oben und die Sekunden
                    if let Tool::Faehigkeit(platz) = tool {
                        let rest = abklingen[(platz as usize).min(3)];
                        if rest > 0.0 {
                            let gesamt = tool.faehigkeit(class).map_or(1.0, |f| f.abklingen(0) as f32 / 60.0);
                            let mut schleier = slot_rect.shrink(3.0);
                            schleier.set_height(schleier.height() * (rest / gesamt).clamp(0.0, 1.0));
                            painter.rect_filled(schleier, 3.0, Color32::from_black_alpha(165));
                            let text = if rest >= 1.0 { format!("{rest:.0}") } else { format!("{rest:.1}") };
                            painter.text(slot_rect.center() + egui::vec2(1.0, 1.0), Align2::CENTER_CENTER, &text, FontId::proportional(17.0), Color32::BLACK);
                            painter.text(slot_rect.center(), Align2::CENTER_CENTER, &text, FontId::proportional(17.0), Color32::WHITE);
                        }
                    }
                }
                // Punkte unter der Fähigkeit: arkane Ladungen (voll: leuchten) bzw. Kombo
                if let Some(Tool::Faehigkeit(platz)) = tool {
                    if let Some((voll, von)) = punkte[(platz as usize).min(3)] {
                        let farbe = match class {
                            crate::protocol::CharacterClass::Zwerg => Color32::from_rgb(255, 205, 110),
                            crate::protocol::CharacterClass::Bogenschuetze => Color32::from_rgb(170, 230, 130),
                            _ => Color32::from_rgb(175, 140, 255),
                        };
                        // Voll geladen bzw. als Nächstes kommt der Schmetterschlag
                        let fertig = voll + (class == crate::protocol::CharacterClass::Zwerg) as u8 >= von;
                        for k in 0..von {
                            let p = egui::pos2(slot_rect.center().x + (k as f32 - (von - 1) as f32 * 0.5) * 11.0, slot_rect.bottom() + 5.0);
                            let an = k < voll;
                            if an && fertig {
                                painter.circle_filled(p, 6.0, farbe.gamma_multiply(0.35));
                            }
                            painter.circle_filled(p, 3.4, if an { farbe } else { Color32::from_black_alpha(160) });
                            painter.circle_stroke(p, 3.4, Stroke::new(1.0, if an { Color32::WHITE.gamma_multiply(0.8) } else { GOLD_DARK }));
                        }
                    }
                }
                // Tastennummer oben links
                let number = (index + 1).to_string();
                let corner = slot_rect.left_top() + egui::vec2(4.0, 2.0);
                painter.text(corner + egui::vec2(1.0, 1.0), Align2::LEFT_TOP, &number, FontId::proportional(12.0), Color32::BLACK);
                painter.text(corner, Align2::LEFT_TOP, &number, FontId::proportional(12.0), if active { GOLD_LIGHT } else { MUTED });
            }
            // Name des Werkzeugs über der Leiste
            if let Some(tool) = Tool::HOTBAR.get(selected) {
                let above = egui::pos2(rect.center().x, rect.top() - 6.0);
                painter.text(above + egui::vec2(1.0, 1.0), Align2::CENTER_BOTTOM, tool.label(class), FontId::proportional(15.0), Color32::BLACK);
                painter.text(above, Align2::CENTER_BOTTOM, tool.label(class), FontId::proportional(15.0), GOLD_LIGHT);
                // Bei Fähigkeiten: die Werte darunter (klein, über dem Namen)
                if let Some(f) = tool.faehigkeit(class) {
                    let zeile = egui::pos2(rect.center().x, rect.top() - 24.0);
                    painter.text(zeile + egui::vec2(1.0, 1.0), Align2::CENTER_BOTTOM, f.werte_zeile(), FontId::proportional(12.5), Color32::BLACK);
                    painter.text(zeile, Align2::CENTER_BOTTOM, f.werte_zeile(), FontId::proportional(12.5), PARCHMENT);
                    // Nach dem Wählen ein paar Sekunden: was die Fähigkeit besonders macht
                    let sichtbar = (1.0 - (seit_auswahl - 5.0) / 1.0).clamp(0.0, 1.0);
                    if sichtbar > 0.0 {
                        let hinweis = egui::pos2(rect.center().x, rect.top() - 42.0);
                        let galley = painter.layout(f.beschreibung().to_string(), FontId::proportional(13.0), PARCHMENT.gamma_multiply(sichtbar), 520.0);
                        let pos = hinweis - egui::vec2(galley.size().x * 0.5, galley.size().y);
                        painter.rect_filled(Rect::from_min_size(pos, galley.size()).expand(5.0), 5.0, Color32::from_black_alpha((150.0 * sichtbar) as u8));
                        painter.galley(pos, galley, PARCHMENT);
                    }
                }
            }
        });
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
pub(crate) fn gradient(painter: &egui::Painter, rect: Rect, from: Color32, to: Color32, horizontal: bool) {
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
pub(crate) fn frame(painter: &egui::Painter, rect: Rect) {
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
pub(crate) fn diamond(painter: &egui::Painter, center: Pos2, size: f32, fill: Color32, edge: Color32) {
    let points = vec![
        center + egui::vec2(0.0, -size),
        center + egui::vec2(size, 0.0),
        center + egui::vec2(0.0, size),
        center + egui::vec2(-size, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(points, fill, Stroke::new(1.0, edge)));
}

/// Goldene Trennlinie, die zu den Enden ausläuft, mit Raute in der Mitte.
pub(crate) fn divider(painter: &egui::Painter, left: f32, right: f32, y: f32) {
    let middle = (left + right) / 2.0;
    let line = |a: f32, b: f32, from: Color32, to: Color32| gradient(painter, Rect::from_min_max(egui::pos2(a, y - 0.75), egui::pos2(b, y + 0.75)), from, to, true);
    line(left, middle - 8.0, Color32::TRANSPARENT, GOLD);
    line(middle + 8.0, right, GOLD, Color32::TRANSPARENT);
    diamond(painter, egui::pos2(middle, y), 4.5, GOLD_LIGHT, GOLD_DARK);
}

/// Text mit etwas Buchstabenabstand (für Überschriften).
pub(crate) fn spaced_text(painter: &egui::Painter, center: Pos2, text: &str, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::default();
    job.append(text, 0.0, egui::TextFormat { font_id: FontId::proportional(size), color, extra_letter_spacing: 3.0, ..Default::default() });
    let galley = painter.layout_job(job);
    let position = center - galley.size() / 2.0;
    painter.galley(position, galley, color);
}

pub(crate) fn close_button(painter: &egui::Painter, center: Pos2, hovered: bool) {
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
pub(crate) fn key_hint(painter: &egui::Painter, right_center: Pos2, key: &str, label: &str) {
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
