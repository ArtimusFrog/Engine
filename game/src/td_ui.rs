//! Oberfläche der Tower Defense: Wellenleiste mit Vorschau, Bossleisten, Schadenszahlen,
//! Auswertung nach jeder Welle, das Verteidigungsfenster (T) und das Turmfenster (E).

use egui::{Align2, Color32, RichText};
use engine::prelude::*;

use crate::bauten::Building;
use crate::protocol::{Inventory, Item};
use crate::td::{WellenBericht, Zielmodus, ZIEL_WELLE};
use crate::tuerme::{TowerKind, MAX_STUFE};
use crate::ui;
use crate::world::World;

const GRUEN: Color32 = Color32::from_rgb(140, 225, 130);
const ROT: Color32 = Color32::from_rgb(240, 110, 95);
const GELB: Color32 = Color32::from_rgb(235, 200, 90);
const VIOLETT: Color32 = Color32::from_rgb(210, 150, 255);
const GOLD: Color32 = Color32::from_rgb(255, 205, 80);

/// Was im Turmfenster gewählt wurde.
pub enum TurmAktion {
    /// Aufwerten (auf Stufe 3 mit Richtung 1 = A, 2 = B)
    Aufwerten(u8),
    Abreissen,
    Zielen(Zielmodus),
}

/// Was im Verteidigungsfenster gewählt wurde.
pub enum TdAktion {
    Rufen,
    Strasse(u8),
}

fn leben_farbe(leben: u32, max: u32) -> Color32 {
    let anteil = leben as f32 / max.max(1) as f32;
    if anteil > 0.5 {
        GRUEN
    } else if anteil > 0.25 {
        GELB
    } else {
        ROT
    }
}

/// Kosten als farbige Zeile (grün = reicht, rot = fehlt).
pub fn kosten_zeile(ui: &mut egui::Ui, inventory: &Inventory, cost: &[(Item, u32)], size: f32) {
    ui.horizontal_wrapped(|ui| {
        for &(item, amount) in cost {
            let color = if inventory.count(item) >= amount { if item == Item::Gold { GOLD } else { GRUEN } } else { ROT };
            ui.label(RichText::new(format!("{amount} {}", item.label())).size(size).color(color));
        }
    });
}

/// Gold, das es jetzt fürs frühe Rufen gäbe.
pub fn rufen_bonus(world: &World) -> u32 {
    (world.td.naechste * 0.8).round() as u32
}

/// Die Vorschau als kurzer Text: „6 Ritter · 4 Wolf · Boss: Steingolem“.
fn vorschau_text(world: &World) -> String {
    let td = &world.td;
    let mut teile: Vec<String> = td.vorschau.iter().map(|(kind, n)| format!("{n} {}", kind.kurz())).collect();
    if let Some(boss) = td.vorschau_boss {
        teile.push(format!("Boss: {}", boss.label()));
    }
    teile.join(" · ")
}

/// Wellenleiste oben unter der Uhrzeit, darunter die Bosse und die Auswertung der letzten Welle.
pub fn wellen_hud(egui_ctx: &egui::Context, world: &World, ich: &str, bericht: Option<(&WellenBericht, f32)>) {
    let td = &world.td;
    if !td.sichtbar() {
        return;
    }
    egui::Area::new(egui::Id::new("wellen")).anchor(Align2::CENTER_TOP, [0.0, 92.0]).interactable(false).show(egui_ctx, |ui| {
        egui::Frame::new().fill(Color32::from_black_alpha(165)).corner_radius(6.0).inner_margin(egui::Margin::symmetric(14, 6)).show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let ziel = if td.endlos { String::new() } else { format!("/{ZIEL_WELLE}") };
                    ui.label(RichText::new(format!("Welle {}{ziel}", td.welle)).size(16.0).strong().color(VIOLETT));
                    if td.sieg && !td.aktiv {
                        ui.label(RichText::new("·  SIEG – die Insel ist gerettet!").size(15.0).strong().color(GOLD));
                    } else if td.aktiv {
                        ui.label(RichText::new(format!("·  nächste in {:.0} s", td.naechste)).size(15.0));
                    } else {
                        ui.label(RichText::new("·  pausiert").size(15.0).color(ui::MUTED));
                    }
                    // Leben je Straße: die eigene groß, die anderen klein daneben
                    let eigene = td.lane_von(ich);
                    if let Some(lane) = eigene {
                        let leben = td.leben.get(lane).copied().unwrap_or(0);
                        ui.label(RichText::new(format!("·  Straße {}: {}/{} Leben", td.strassen[lane].0, leben, td.max_leben)).size(15.0).strong().color(leben_farbe(leben, td.max_leben)));
                    }
                    for (lane, (name, _)) in td.strassen.iter().enumerate() {
                        if Some(lane) == eigene {
                            continue;
                        }
                        let leben = td.leben.get(lane).copied().unwrap_or(0);
                        ui.label(RichText::new(format!("{} {}", name, leben)).size(13.0).color(leben_farbe(leben, td.max_leben).gamma_multiply(0.85)));
                    }
                });
                if td.aktiv && !(td.sieg && !td.endlos) {
                    let farbe = if td.vorschau_boss.is_some() { Color32::from_rgb(255, 150, 120) } else { Color32::from_white_alpha(190) };
                    ui.label(RichText::new(format!("Als Nächstes: {}", vorschau_text(world))).size(13.0).color(farbe));
                    if td.naechste >= 3.0 {
                        ui.label(RichText::new(format!("N: jetzt rufen (+{} Gold für alle) · T: Verteidigung", rufen_bonus(world))).size(12.0).color(Color32::from_white_alpha(150)));
                    }
                }
            });
        });
        // Bosse im Blick: je Art eine Leiste (der Schwächste zählt), dazu wie viele es sind
        let mut bosse: Vec<(&str, u8, usize)> = Vec::new();
        for (name, health) in world.sichtbare_bosse() {
            match bosse.iter_mut().find(|b| b.0 == name) {
                Some(b) => {
                    b.1 = b.1.min(health);
                    b.2 += 1;
                }
                None => bosse.push((name, health, 1)),
            }
        }
        for (name, health, anzahl) in bosse.iter().take(3) {
            let name = if *anzahl > 1 { format!("{name} ×{anzahl}") } else { name.to_string() };
            ui.add_space(4.0);
            egui::Frame::new().fill(Color32::from_black_alpha(150)).corner_radius(5.0).inner_margin(egui::Margin::symmetric(10, 4)).show(ui, |ui| {
                ui.set_width(300.0);
                ui.label(RichText::new(format!("Boss: {name}")).size(14.0).strong().color(Color32::from_rgb(255, 140, 110)));
                let (rect, _) = ui.allocate_exact_size(egui::vec2(280.0, 9.0), egui::Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(rect, 3.0, Color32::from_black_alpha(160));
                let mut fill = rect;
                fill.set_width(rect.width() * *health as f32 / 100.0);
                painter.rect_filled(fill, 3.0, Color32::from_rgb(210, 60, 60));
            });
        }
    });
    // Auswertung der letzten Welle: einige Sekunden rechts oben
    if let Some((bericht, alter)) = bericht {
        if alter < 9.0 {
            let alpha = ((9.0 - alter) / 1.0).clamp(0.0, 1.0);
            egui::Area::new(egui::Id::new("wellenbericht")).anchor(Align2::RIGHT_TOP, [-14.0, 110.0]).interactable(false).show(egui_ctx, |ui| {
                egui::Frame::new().fill(Color32::from_black_alpha((175.0 * alpha) as u8)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                    ui.set_width(270.0);
                    let weiss = Color32::from_white_alpha((235.0 * alpha) as u8);
                    ui.label(RichText::new(format!("Welle {} überstanden", bericht.welle)).size(17.0).strong().color(GOLD.gamma_multiply(alpha)));
                    ui.label(RichText::new(format!("{} besiegt · {} durchgebrochen", bericht.besiegt, bericht.durchgebrochen)).size(14.0).color(weiss));
                    ui.label(RichText::new(format!("+{} Gold für alle", bericht.gold)).size(14.0).color(GOLD.gamma_multiply(alpha)));
                    if let Some((turm, owner, kills)) = &bericht.bester_turm {
                        ui.label(RichText::new(format!("Bester Turm: {turm} von {owner} ({kills} besiegt)")).size(13.0).color(weiss));
                    }
                    for (i, (name, schaden)) in bericht.schaden.iter().take(4).enumerate() {
                        ui.label(RichText::new(format!("{}. {name} – {schaden} Schaden", i + 1)).size(13.0).color(weiss));
                    }
                });
            });
        }
    }
}

/// Schadenszahlen über den getroffenen Einheiten.
pub fn schadenszahlen(ctx: &Context, egui_ctx: &egui::Context, world: &World) {
    let painter = egui_ctx.layer_painter(egui::LayerId::background());
    for zahl in world.schadenszahlen() {
        let weite = ctx.camera.position.distance(zahl.ort);
        if weite > 70.0 {
            continue;
        }
        let Some(p) = ctx.world_to_screen(zahl.ort) else { continue };
        let alpha = (1.0 - (zahl.alter / 1.1).powi(2)).clamp(0.0, 1.0);
        let groesse = if zahl.gross { 24.0 } else { 17.0 } * (14.0 / weite.max(8.0)).sqrt().clamp(0.6, 1.3);
        let farbe = if zahl.gross { Color32::from_rgb(255, 170, 60) } else { Color32::from_rgb(255, 240, 200) };
        let pos = egui::pos2(p.x, p.y);
        let font = egui::FontId::proportional(groesse);
        painter.text(pos + egui::vec2(1.5, 1.5), Align2::CENTER_CENTER, zahl.wert.to_string(), font.clone(), Color32::from_black_alpha((180.0 * alpha) as u8));
        painter.text(pos, Align2::CENTER_CENTER, zahl.wert.to_string(), font, farbe.gamma_multiply(alpha));
    }
}

/// Verteidigungsfenster (T): nächste Welle mit Eigenschaften der Einheiten, früh rufen, Straßen
/// verteidigen, wer wie viel beiträgt, die letzte Auswertung. Liefert die Wünsche und ob es zugeht.
pub fn td_fenster(egui_ctx: &egui::Context, world: &World, ich: &str) -> (Vec<TdAktion>, bool) {
    let td = &world.td;
    let mut aktionen = Vec::new();
    let mut zu = false;
    ui::center_panel(egui_ctx, "verteidigung", 720.0, |ui| {
        ui::heading(ui, "Verteidigung der Insel");
        let ziel = if td.endlos { "Endlosmodus".to_string() } else { format!("Ziel: Welle {ZIEL_WELLE} überstehen") };
        ui.label(
            RichText::new(format!("Welle {} · {} · Schwierigkeit {} · {} Leben je Straße", td.welle, ziel, td.schwierigkeit.label(), td.max_leben))
                .size(15.0)
                .color(ui::TEXT),
        );
        ui.label(RichText::new("Jede Straße hat ihre eigenen Leben. Fällt eine, wird die Siedlung an ihrem Ende zerstört.").size(13.0).color(ui::MUTED));
        if td.sieg {
            ui.label(RichText::new("Welle 30 überstanden – die Insel ist gerettet!").size(16.0).strong().color(GOLD));
        }
        let hoehe = (egui_ctx.content_rect().height() - 260.0).max(240.0);
        egui::ScrollArea::vertical().max_height(hoehe).show(ui, |ui| {
            ui.add_space(8.0);
            let titel = if td.aktiv { format!("Nächste Welle ({}) in {:.0} s", td.welle + 1, td.naechste) } else { format!("Nächste Welle ({})", td.welle + 1) };
            ui.label(RichText::new(titel).size(17.0).strong().color(ui::ACCENT));
            if let Some(boss) = td.vorschau_boss {
                ui.label(RichText::new(format!("BOSSWELLE: {} – {}", boss.label(), boss_text(boss))).size(14.0).color(Color32::from_rgb(255, 150, 120)));
            }
            for (kind, n) in &td.vorschau {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("{n}× {}", kind.label())).size(14.5).strong());
                    if let Some(text) = kind.eigenschaft() {
                        ui.label(RichText::new(format!("– {text}")).size(13.0).color(ui::MUTED));
                    }
                });
            }
            let kann = td.aktiv && td.naechste >= 3.0 && !(td.sieg && !td.endlos);
            let text = if kann { format!("Welle jetzt rufen (+{} Gold für alle) – Taste N", rufen_bonus(world)) } else { "Welle jetzt rufen".to_string() };
            if ui.add_enabled(kann, egui::Button::new(RichText::new(text).size(16.0)).min_size(egui::vec2(ui.available_width(), 32.0))).clicked() {
                aktionen.push(TdAktion::Rufen);
            }

            ui.add_space(10.0);
            ui.label(RichText::new("Heerstraßen").size(17.0).strong().color(ui::ACCENT));
            ui.label(RichText::new("Teilt euch die Straßen auf: wer eine verteidigt, steht hier und in der Wellenleiste.").size(13.0).color(ui::MUTED));
            ui.columns(td.strassen.len().max(1), |spalten| {
                for (i, (spalte, (name, wer))) in spalten.iter_mut().zip(&td.strassen).enumerate() {
                    egui::Frame::new().fill(Color32::from_black_alpha(90)).corner_radius(6.0).inner_margin(8.0).show(spalte, |ui| {
                        ui.label(RichText::new(format!("Straße {name}")).size(15.0).strong());
                        let leben = td.leben.get(i).copied().unwrap_or(0);
                        ui.label(RichText::new(format!("{leben}/{} Leben", td.max_leben)).size(14.0).strong().color(leben_farbe(leben, td.max_leben)));
                        let meins = wer.as_deref() == Some(ich);
                        ui.label(RichText::new(wer.clone().unwrap_or_else(|| "frei".into())).size(13.0).color(if meins { GRUEN } else { ui::MUTED }));
                        if meins {
                            if ui.button(RichText::new("Freigeben").size(13.0)).clicked() {
                                aktionen.push(TdAktion::Strasse(255));
                            }
                        } else if ui.button(RichText::new("Verteidigen").size(13.0)).clicked() {
                            aktionen.push(TdAktion::Strasse(i as u8));
                        }
                    });
                }
            });

            ui.add_space(10.0);
            ui.label(RichText::new("Beitrag seit Welle 1").size(17.0).strong().color(ui::ACCENT));
            if world.td_beitrag.is_empty() {
                ui.label(RichText::new("Noch niemand hat Schaden gemacht.").size(13.0).color(ui::MUTED));
            }
            for (i, (name, schaden, kills)) in world.td_beitrag.iter().take(8).enumerate() {
                let farbe = if name == ich { GRUEN } else { ui::TEXT };
                ui.label(RichText::new(format!("{}. {name} – {schaden} Schaden, {kills} besiegt", i + 1)).size(14.0).color(farbe));
            }
            if let Some(bericht) = world.berichte.last() {
                ui.add_space(10.0);
                ui.label(RichText::new(format!("Letzte Welle ({})", bericht.welle)).size(17.0).strong().color(ui::ACCENT));
                ui.label(RichText::new(format!("{} besiegt · {} durchgebrochen · +{} Gold", bericht.besiegt, bericht.durchgebrochen, bericht.gold)).size(14.0));
                if let Some((turm, owner, kills)) = &bericht.bester_turm {
                    ui.label(RichText::new(format!("Bester Turm: {turm} von {owner} ({kills} besiegt)")).size(14.0));
                }
            }
        });
        ui.add_space(8.0);
        if ui::big_button(ui, "Schließen (T)").clicked() {
            zu = true;
        }
    });
    (aktionen, zu)
}

fn boss_text(boss: crate::heer::EnemyKind) -> &'static str {
    use crate::heer::EnemyKind::*;
    match boss {
        Golem => "stampft alle 9 s und legt Türme im Umkreis von 14 m lahm, zerfällt in drei Golems",
        Knight => "gerät bei halbem Leben in Wut: doppelte Schlagkraft, unaufhaltsam",
        Warlock => "heilt stark und ruft alle 12 s drei Skelette",
        Ghost => "wird alle 13 s für 3 s unverwundbar",
        _ => "",
    }
}

/// Zielt dieser Turm überhaupt (Banner, Kaserne, Schatzkammer, Runenstampfer, Frostfeld nicht)?
fn zielt(building: &Building) -> bool {
    match building.tower() {
        Some(TowerKind::Banner | TowerKind::Barracks | TowerKind::Treasury | TowerKind::Rune) | None => false,
        Some(TowerKind::Frost) => building.aktiver_zweig() != 2,
        _ => true,
    }
}

/// Turmfenster (E): Werte, Statistik, Zielmodus, nächste Stufe bzw. die zwei Richtungen, Abreißen.
pub fn turm_fenster(egui_ctx: &egui::Context, world: &World, building: &Building, inventory: &Inventory, me: &str) -> (Option<TurmAktion>, bool) {
    let mut aktion = None;
    let mut zu = false;
    let breite = if building.tower().is_some() && building.level + 1 == MAX_STUFE { 760.0 } else { 540.0 };
    ui::center_panel(egui_ctx, "turmfenster", breite, |ui| {
        let titel = match building.tower() {
            Some(t) if building.aktiver_zweig() > 0 => format!("{} · Stufe {} · {}", building.kind.label(), building.level, t.zweig(building.zweig).0),
            Some(_) => format!("{} · Stufe {}", building.kind.label(), building.level),
            None if building.kind == crate::bauten::BuildingKind::Dorfhalle => format!("{} · Stufe {}", building.kind.stufen_name(building.level), building.level),
            None => building.kind.label().to_string(),
        };
        ui::heading(ui, &titel);
        ui.label(RichText::new(format!("Gebaut von {}", if building.owner.is_empty() { "?" } else { &building.owner })).size(13.0).color(ui::MUTED));
        if !building.finished() {
            ui.label(RichText::new(format!("Wird gebaut … {:.0} %", building.progress * 100.0)).size(15.0).color(ui::ACCENT));
        }
        ui.add_space(6.0);
        ui.label(RichText::new(building.kind.description()).size(14.0));
        if let Some(tower) = building.tower() {
            if building.aktiver_zweig() > 0 {
                let (name, text) = tower.zweig(building.zweig);
                ui.label(RichText::new(format!("{name}: {text}")).size(14.0).color(VIOLETT));
            }
            ui.add_space(6.0);
            for zeile in building.kind_werte().zeilen() {
                ui.label(RichText::new(zeile).size(15.0));
            }
            if let Some(&(kills, schaden)) = world.td_stats.get(&building.id) {
                ui.label(RichText::new(format!("Bisher: {kills} besiegt · {schaden} Schaden")).size(14.0).color(GOLD));
            }
            if zielt(building) {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Ziel:").size(14.0));
                    for modus in Zielmodus::ALL {
                        if ui.selectable_label(building.ziel == modus, RichText::new(modus.label()).size(14.0)).on_hover_text(modus.beschreibung()).clicked() {
                            aktion = Some(TurmAktion::Zielen(modus));
                        }
                    }
                });
            }
            ui.add_space(10.0);
            let naechste = building.level + 1;
            if building.level < MAX_STUFE {
                let cost = building.kind.upgrade_cost(naechste);
                let kann = building.finished() && crate::bauten::can_pay(inventory, &cost);
                if naechste < MAX_STUFE {
                    ui.label(RichText::new(format!("Stufe {naechste}")).size(16.0).strong().color(ui::ACCENT));
                    for zeile in tower.werte(naechste, 0).zeilen() {
                        ui.label(RichText::new(zeile).size(14.0).color(GRUEN));
                    }
                    kosten_zeile(ui, inventory, &cost, 14.0);
                    if ui.add_enabled(kann, egui::Button::new(RichText::new(format!("Auf Stufe {naechste} aufwerten")).size(17.0)).min_size(egui::vec2(ui.available_width(), 34.0))).clicked() {
                        aktion = Some(TurmAktion::Aufwerten(0));
                    }
                } else {
                    ui.label(RichText::new("Stufe 3: Richtung wählen").size(16.0).strong().color(ui::ACCENT));
                    kosten_zeile(ui, inventory, &cost, 14.0);
                    ui.columns(2, |spalten| {
                        for (z, spalte) in [1u8, 2].into_iter().zip(spalten.iter_mut()) {
                            egui::Frame::new().fill(Color32::from_black_alpha(90)).corner_radius(8.0).inner_margin(10.0).show(spalte, |ui| {
                                let (name, text) = tower.zweig(z);
                                ui.label(RichText::new(format!("{} {name}", if z == 1 { "A:" } else { "B:" })).size(17.0).strong().color(VIOLETT));
                                ui.label(RichText::new(text).size(13.5));
                                ui.add_space(4.0);
                                for zeile in tower.werte(3, z).zeilen() {
                                    ui.label(RichText::new(zeile).size(12.5).color(GRUEN));
                                }
                                ui.add_space(4.0);
                                if ui.add_enabled(kann, egui::Button::new(RichText::new(format!("{name} wählen")).size(15.0)).min_size(egui::vec2(ui.available_width(), 30.0))).clicked() {
                                    aktion = Some(TurmAktion::Aufwerten(z));
                                }
                            });
                        }
                    });
                }
            } else {
                ui.label(RichText::new("Höchste Stufe erreicht").size(15.0).color(ui::ACCENT));
            }
        } else if building.kind == crate::bauten::BuildingKind::Dorfhalle {
            use crate::bauten::{bauradius, dorfhalle_freischaltung, dorfhalle_gold};
            ui.add_space(6.0);
            ui.label(RichText::new(format!("Bauradius {:.0} m (R zeigt ihn)", bauradius(building.level))).size(15.0));
            ui.label(RichText::new(format!("Schaltet frei: {}", dorfhalle_freischaltung(building.level))).size(15.0));
            ui.label(RichText::new(format!("+{} Gold je überstandener Welle · Gebäude im Radius arbeiten 10 % schneller", dorfhalle_gold(building.level))).size(14.0).color(GOLD));
            ui.label(RichText::new("Hier fängst du an, wenn du ins Spiel kommst oder von der Insel fällst.").size(13.0).color(ui::MUTED));
            ui.add_space(10.0);
            if building.level < MAX_STUFE {
                let naechste = building.level + 1;
                ui.label(RichText::new(format!("Ausbau zum {}", building.kind.stufen_name(naechste))).size(16.0).strong().color(ui::ACCENT));
                ui.label(RichText::new(format!("Bauradius {:.0} m · schaltet frei: {} · +{} Gold je Welle", bauradius(naechste), dorfhalle_freischaltung(naechste), dorfhalle_gold(naechste))).size(14.0).color(GRUEN));
                let cost = building.kind.upgrade_cost(naechste);
                kosten_zeile(ui, inventory, &cost, 14.0);
                let kann = building.finished() && crate::bauten::can_pay(inventory, &cost) && building.owner == me;
                let text = if building.owner == me { format!("Zum {} ausbauen", building.kind.stufen_name(naechste)) } else { "Nur der Besitzer baut aus".to_string() };
                if ui.add_enabled(kann, egui::Button::new(RichText::new(text).size(17.0)).min_size(egui::vec2(ui.available_width(), 34.0))).clicked() {
                    aktion = Some(TurmAktion::Aufwerten(0));
                }
            } else {
                ui.label(RichText::new("Voll ausgebaut").size(15.0).color(ui::ACCENT));
            }
        } else if let Some(item) = building.kind.produces() {
            ui.label(RichText::new(format!("Liefert 1 {} alle {:.0} s", item.label(), crate::bauten::PRODUCTION_SECONDS)).size(15.0));
        } else if let Some(&(_, lp)) = world.td.barrikaden.iter().find(|(id, _)| *id == building.id) {
            ui.label(RichText::new(format!("Zustand: {lp} %")).size(15.0).color(leben_farbe(lp as u32, 100)));
        }
        ui.add_space(8.0);
        if building.owner == me {
            let zurueck: Vec<String> = building.paid().into_iter().filter(|&(_, n)| n / 2 > 0).map(|(item, n)| format!("{} {}", n / 2, item.label())).collect();
            if ui.button(RichText::new(format!("Abreißen (zurück: {})", zurueck.join(", "))).size(14.0).color(Color32::from_rgb(240, 140, 120))).clicked() {
                aktion = Some(TurmAktion::Abreissen);
            }
        }
        ui.add_space(8.0);
        if ui::big_button(ui, "Schließen (E)").clicked() {
            zu = true;
        }
    });
    (aktion, zu)
}
