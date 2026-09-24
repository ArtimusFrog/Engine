//! Aussehen und wiederverwendbare Bausteine der Benutzeroberfläche.

use engine::egui::{self, Align2, Color32, FontId, RichText, Stroke};
use engine::prelude::*;
use engine::daycycle::DayCycle;

pub const ACCENT: Color32 = Color32::from_rgb(255, 150, 40);
pub const TEXT: Color32 = Color32::from_rgb(235, 238, 245);
pub const MUTED: Color32 = Color32::from_rgb(150, 158, 175);
pub const ERROR: Color32 = Color32::from_rgb(255, 110, 100);
const PANEL: Color32 = Color32::from_rgba_premultiplied(14, 17, 23, 240);

/// Dunkles Spiel-Design mit großer, gut lesbarer Schrift.
pub fn apply_theme(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        use egui::TextStyle;
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(30.0)),
            (TextStyle::Body, FontId::proportional(18.0)),
            (TextStyle::Button, FontId::proportional(20.0)),
            (TextStyle::Small, FontId::proportional(14.0)),
            (TextStyle::Monospace, FontId::monospace(15.0)),
        ]
        .into();
        style.spacing.button_padding = egui::vec2(18.0, 9.0);
        style.spacing.item_spacing = egui::vec2(10.0, 12.0);
        style.spacing.slider_width = 220.0;
        style.spacing.interact_size.y = 30.0;

        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = ACCENT.gamma_multiply(0.7);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.extreme_bg_color = Color32::from_rgb(8, 10, 14);
        for (widget, fill) in [
            (&mut v.widgets.inactive, Color32::from_rgb(38, 44, 56)),
            (&mut v.widgets.hovered, Color32::from_rgb(58, 66, 82)),
            (&mut v.widgets.active, Color32::from_rgb(78, 88, 108)),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.corner_radius = egui::CornerRadius::same(6);
        }
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_white_alpha(55));
        v.widgets.inactive.fg_stroke = Stroke::new(2.0, TEXT);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        v.widgets.hovered.fg_stroke = Stroke::new(2.0, Color32::WHITE);
        v.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
        v.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
        style.spacing.icon_width = 22.0;
        style.spacing.icon_width_inner = 12.0;
    });
}

/// Dunkler Verlauf vom linken Bildschirmrand, damit Text auf hellem Hintergrund lesbar bleibt.
pub fn left_shade(ctx: &egui::Context) {
    let screen = ctx.content_rect();
    let rect = egui::Rect::from_min_max(screen.min, egui::pos2(screen.min.x + screen.width() * 0.6, screen.max.y));
    let dark = Color32::from_black_alpha(190);
    let clear = Color32::TRANSPARENT;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), dark);
    mesh.colored_vertex(rect.right_top(), clear);
    mesh.colored_vertex(rect.right_bottom(), clear);
    mesh.colored_vertex(rect.left_bottom(), dark);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ctx.layer_painter(egui::LayerId::background()).add(egui::Shape::mesh(mesh));
}

/// Halbtransparentes Panel in der Bildschirmmitte.
pub fn center_panel(ctx: &egui::Context, id: &str, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(egui::Id::new(id)).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        panel_frame().show(ui, |ui| {
            ui.set_width(width);
            add(ui);
        });
    });
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(12.0)
        .inner_margin(26.0)
        .stroke(Stroke::new(1.0, Color32::from_white_alpha(22)))
        .shadow(egui::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(110) })
}

/// Breiter Menüknopf.
pub fn big_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let width = ui.available_width();
    ui.add_sized([width, 46.0], egui::Button::new(RichText::new(text).size(21.0)))
}

/// Legt den ganzen Bildschirm leicht abgedunkelt hinter Menüs.
pub fn dim_background(ctx: &egui::Context, alpha: u8) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(ctx.content_rect(), 0.0, Color32::from_black_alpha(alpha));
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(30.0).strong().color(Color32::WHITE));
    ui.add_space(4.0);
}

/// Namensschild über einer Figur, blendet mit der Entfernung aus.
pub fn name_tag(ctx: &Context, ui: &egui::Context, position: Vec3, name: &str) {
    let distance = ctx.camera.position.distance(position);
    let alpha = (1.0 - (distance - 15.0) / 25.0).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let Some(screen) = ctx.world_to_screen(position) else { return };
    let pos = egui::pos2(screen.x, screen.y);
    let painter = ui.layer_painter(egui::LayerId::background());
    let font = FontId::proportional(17.0);
    let galley = painter.layout_no_wrap(name.to_string(), font.clone(), Color32::WHITE);
    let rect = Align2::CENTER_BOTTOM.anchor_size(pos, galley.size()).expand2(egui::vec2(8.0, 3.0));
    painter.rect_filled(rect, 5.0, Color32::from_black_alpha((140.0 * alpha) as u8));
    painter.text(pos, Align2::CENTER_BOTTOM, name, font, Color32::WHITE.gamma_multiply(alpha));
}

/// Kleines Fadenkreuz in der Bildschirmmitte.
pub fn crosshair(ui: &egui::Context) {
    let painter = ui.layer_painter(egui::LayerId::background());
    let center = ui.content_rect().center();
    painter.circle_filled(center, 2.5, Color32::from_white_alpha(230));
    painter.circle_stroke(center, 9.0, Stroke::new(1.5, Color32::from_white_alpha(120)));
}

/// IP-Adresse dieses Rechners im lokalen Netz (ohne Pakete zu verschicken).
pub fn local_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

/// Farbe der Zeitleiste zu einer Uhrzeit (Stunden) – Nacht, Morgenrot, Tag, Abendrot.
fn sky_band(hour: f32) -> Color32 {
    const STOPS: [(f32, [u8; 3]); 10] = [
        (0.0, [18, 22, 52]),
        (4.5, [22, 26, 62]),
        (6.0, [235, 125, 85]),
        (7.5, [125, 185, 240]),
        (12.0, [150, 205, 255]),
        (17.0, [125, 185, 240]),
        (19.0, [245, 120, 65]),
        (20.5, [70, 55, 115]),
        (22.0, [25, 28, 62]),
        (24.0, [18, 22, 52]),
    ];
    let next = STOPS.iter().position(|&(h, _)| h >= hour).unwrap_or(STOPS.len() - 1).max(1);
    let ((h0, a), (h1, b)) = (STOPS[next - 1], STOPS[next]);
    let t = ((hour - h0) / (h1 - h0).max(1e-3)).clamp(0.0, 1.0);
    let mix = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8;
    Color32::from_rgb(mix(0), mix(1), mix(2))
}

/// Zeitleiste oben in der Mitte: Tagesverlauf, Sonne bzw. Mond an der aktuellen Uhrzeit,
/// dazu Uhr, Tageszeit und Tageszähler.
pub fn time_bar(ctx: &egui::Context, day: &DayCycle) {
    egui::Area::new(egui::Id::new("zeitleiste"))
        .anchor(Align2::CENTER_TOP, [0.0, 10.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(155))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(16, 8))
                .show(ui, |ui| {
                    let width = 340.0;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 46.0), egui::Sense::hover());
                    let painter = ui.painter();
                    let muted = TEXT.gamma_multiply(0.85);
                    let top = rect.top() + 9.0;
                    painter.text(egui::pos2(rect.left(), top), Align2::LEFT_CENTER, day.phase().label(), FontId::proportional(14.0), muted);
                    painter.text(egui::pos2(rect.center().x, top), Align2::CENTER_CENTER, day.clock(), FontId::proportional(21.0), Color32::WHITE);
                    painter.text(egui::pos2(rect.right(), top), Align2::RIGHT_CENTER, format!("Tag {}", day.day), FontId::proportional(14.0), muted);

                    // Farbverlauf über 24 Stunden
                    let bar = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.top() + 27.0), egui::vec2(width, 10.0));
                    let x_at = |hour: f32| bar.left() + hour / 24.0 * width;
                    let mut mesh = egui::Mesh::default();
                    let steps = 48;
                    for i in 0..=steps {
                        let hour = i as f32 / steps as f32 * 24.0;
                        let color = sky_band(hour);
                        mesh.colored_vertex(egui::pos2(x_at(hour), bar.top()), color);
                        mesh.colored_vertex(egui::pos2(x_at(hour), bar.bottom()), color);
                        if i > 0 {
                            let v = (i * 2) as u32;
                            mesh.add_triangle(v - 2, v - 1, v);
                            mesh.add_triangle(v - 1, v + 1, v);
                        }
                    }
                    painter.add(egui::Shape::mesh(mesh));
                    painter.rect_stroke(bar, 3.0, Stroke::new(1.0, Color32::from_white_alpha(45)), egui::StrokeKind::Outside);
                    for hour in [6.0, 12.0, 18.0] {
                        let x = x_at(hour);
                        painter.line_segment([egui::pos2(x, bar.bottom() + 2.0), egui::pos2(x, bar.bottom() + 6.0)], Stroke::new(1.0, Color32::from_white_alpha(90)));
                    }

                    // Sonne oder Mond an der aktuellen Uhrzeit
                    let center = egui::pos2(x_at(day.hour), bar.center().y);
                    if day.is_night() {
                        painter.circle_filled(center, 12.0, Color32::from_white_alpha(22));
                        painter.circle_filled(center, 7.5, Color32::from_rgb(228, 232, 248));
                        // Sichel: ein Teil wird mit der Nachtfarbe überdeckt
                        painter.circle_filled(center + egui::vec2(3.2, -2.2), 6.2, sky_band(day.hour));
                    } else {
                        painter.circle_filled(center, 13.0, Color32::from_rgba_unmultiplied(255, 200, 80, 55));
                        painter.circle_filled(center, 7.5, Color32::from_rgb(255, 208, 72));
                        painter.circle_stroke(center, 7.5, Stroke::new(1.5, Color32::from_rgb(255, 240, 180)));
                    }
                });
        });
}
