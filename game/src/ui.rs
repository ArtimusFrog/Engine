//! Aussehen und wiederverwendbare Bausteine der Benutzeroberfläche.

use engine::egui::{self, Align2, Color32, FontId, RichText, Stroke};
use engine::prelude::*;

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
