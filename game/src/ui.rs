//! Aussehen und wiederverwendbare Bausteine der Benutzeroberfläche.

use engine::egui::{self, Align2, Color32, FontId, RichText, Stroke};
use engine::prelude::*;
use engine::daycycle::DayCycle;

use crate::protocol::Item;

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


const GOLD: Color32 = Color32::from_rgb(214, 178, 96);
const GOLD_DARK: Color32 = Color32::from_rgb(120, 92, 42);

fn darken(color: Color32, factor: f32) -> Color32 {
    let f = |c: u8| (c as f32 * factor).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(color.r()), f(color.g()), f(color.b()))
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Raute als Zierelement.
fn diamond(painter: &egui::Painter, center: egui::Pos2, size: f32, fill: Color32) {
    let points = vec![
        center + egui::vec2(0.0, -size),
        center + egui::vec2(size, 0.0),
        center + egui::vec2(0.0, size),
        center + egui::vec2(-size, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(points, fill, Stroke::new(1.0, GOLD_DARK)));
}

/// Runder Himmelsausschnitt: Farbverlauf der Tageszeit, Hügel, Sonne bzw. Mond auf ihrer
/// Bahn, nachts Sterne.
fn sky_medallion(painter: &egui::Painter, center: egui::Pos2, radius: f32, day: &DayCycle) {
    let horizon = sky_band(day.hour);
    let zenith = if day.is_night() { darken(horizon, 0.45) } else { lerp_color(horizon, Color32::from_rgb(40, 90, 190), 0.55) };

    // Himmel als Fächer mit Farbverlauf von oben nach unten
    let mut sky = egui::Mesh::default();
    sky.colored_vertex(center, lerp_color(zenith, horizon, 0.5));
    let segments = 48;
    for i in 0..=segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let p = center + egui::vec2(a.cos(), a.sin()) * radius;
        let t = (p.y - (center.y - radius)) / (2.0 * radius);
        sky.colored_vertex(p, lerp_color(zenith, horizon, t));
        if i > 0 {
            sky.add_triangle(0, i as u32, i as u32 + 1);
        }
    }
    painter.add(egui::Shape::mesh(sky));

    if day.is_night() {
        for (x, y) in [(-0.45, -0.5), (0.2, -0.62), (0.55, -0.3), (-0.15, -0.2), (-0.62, -0.1), (0.38, -0.7), (0.05, -0.45)] {
            painter.circle_filled(center + egui::vec2(x, y) * radius, 0.9, Color32::from_white_alpha(210));
        }
    }

    // Sonne bzw. Mond auf einem Bogen: links auf, oben am höchsten, rechts unter.
    let (angle, is_sun) = if day.is_night() {
        (((day.hour - 18.0).rem_euclid(24.0)) / 12.0 * std::f32::consts::PI, false)
    } else {
        ((day.hour - 6.0) / 12.0 * std::f32::consts::PI, true)
    };
    let body = center + egui::vec2(-angle.cos() * radius * 0.62, -angle.sin() * radius * 0.62 + radius * 0.18);
    if is_sun {
        painter.circle_filled(body, 9.0, Color32::from_rgba_unmultiplied(255, 190, 70, 60));
        painter.circle_filled(body, 5.5, Color32::from_rgb(255, 214, 90));
    } else {
        painter.circle_filled(body, 8.0, Color32::from_white_alpha(28));
        painter.circle_filled(body, 5.0, Color32::from_rgb(232, 236, 250));
        painter.circle_filled(body + egui::vec2(2.2, -1.6), 4.2, lerp_color(zenith, horizon, 0.3));
    }

    // Hügel-Silhouette, unten vom Kreis begrenzt
    let hill = if day.is_night() {
        Color32::from_rgb(10, 14, 22)
    } else {
        darken(lerp_color(horizon, Color32::from_rgb(30, 60, 30), 0.7), 0.7)
    };
    let mut ground = egui::Mesh::default();
    let columns = 32;
    for i in 0..=columns {
        let x = -radius + 2.0 * radius * i as f32 / columns as f32;
        let bottom = (radius * radius - x * x).max(0.0).sqrt();
        let wave = radius * (0.34 + 0.08 * (x / radius * 5.0).sin() + 0.05 * (x / radius * 11.0).cos());
        let top = wave.min(bottom);
        ground.colored_vertex(center + egui::vec2(x, top), hill);
        ground.colored_vertex(center + egui::vec2(x, bottom), hill);
        if i > 0 {
            let v = (i * 2) as u32;
            ground.add_triangle(v - 2, v - 1, v);
            ground.add_triangle(v - 1, v + 1, v);
        }
    }
    painter.add(egui::Shape::mesh(ground));

    // Rahmen: dunkler Außenrand, Goldring, feiner Innenring, Rauten an den Seiten
    painter.circle_stroke(center, radius + 3.5, Stroke::new(2.0, Color32::from_black_alpha(200)));
    painter.circle_stroke(center, radius + 1.5, Stroke::new(2.5, GOLD));
    painter.circle_stroke(center, radius - 0.5, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 240, 200, 70)));
    for angle in [0.0f32, 90.0, 180.0, 270.0] {
        let a = angle.to_radians();
        diamond(painter, center + egui::vec2(a.cos(), a.sin()) * (radius + 1.5), 3.5, GOLD);
    }
}

/// Zeitleiste im MMORPG-Stil oben in der Mitte: Himmels-Medaillon mit Sonne bzw. Mond,
/// links die Tageszeit mit Tagesfortschritt, rechts Uhrzeit und Tag.
pub fn time_bar(ctx: &egui::Context, day: &DayCycle) {
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("zeitleiste")));
    let screen = ctx.content_rect();
    let center = egui::pos2(screen.center().x, screen.top() + 44.0);
    let (half_width, half_height, chamfer) = (200.0, 20.0, 12.0);

    // Goldene Zierlinien mit Rauten links und rechts
    for side in [-1.0f32, 1.0] {
        let start = center + egui::vec2(side * (half_width + 4.0), 0.0);
        let end = center + egui::vec2(side * (half_width + 46.0), 0.0);
        painter.line_segment([start, end], Stroke::new(1.5, GOLD.gamma_multiply(0.8)));
        diamond(&painter, end, 4.5, GOLD);
        diamond(&painter, start + egui::vec2(side * 14.0, 0.0), 2.5, GOLD_DARK);
    }

    // Banner mit abgeschrägten Ecken, dunkles Glas mit Goldkante
    let banner = |inset: f32| {
        let (w, h, c) = (half_width - inset, half_height - inset, chamfer - inset * 0.6);
        vec![
            center + egui::vec2(-w + c, -h),
            center + egui::vec2(w - c, -h),
            center + egui::vec2(w, -h + c),
            center + egui::vec2(w, h - c),
            center + egui::vec2(w - c, h),
            center + egui::vec2(-w + c, h),
            center + egui::vec2(-w, h - c),
            center + egui::vec2(-w, -h + c),
        ]
    };
    painter.add(egui::Shape::convex_polygon(banner(-3.0), Color32::from_black_alpha(90), Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(banner(0.0), Color32::from_rgba_unmultiplied(12, 14, 22, 240), Stroke::new(1.5, GOLD)));
    painter.add(egui::Shape::closed_line(banner(3.5), Stroke::new(1.0, Color32::from_rgba_unmultiplied(214, 178, 96, 60))));
    // Glanzlinie oben
    let shine_y = -half_height + 5.0;
    painter.line_segment(
        [center + egui::vec2(-half_width + chamfer + 6.0, shine_y), center + egui::vec2(half_width - chamfer - 6.0, shine_y)],
        Stroke::new(1.0, Color32::from_white_alpha(18)),
    );

    // Links: Tageszeit und Fortschritt des Tages
    let left = center.x - half_width + 22.0;
    let text_right = center.x - 40.0;
    let phase = day.phase().label().to_uppercase();
    painter.text(egui::pos2(left, center.y - 6.0), Align2::LEFT_CENTER, phase, FontId::proportional(13.0), GOLD);
    let track = egui::Rect::from_min_max(egui::pos2(left, center.y + 7.0), egui::pos2(text_right, center.y + 9.5));
    painter.rect_filled(track, 1.5, Color32::from_white_alpha(25));
    let mut fill = track;
    fill.set_width(track.width() * day.hour / 24.0);
    painter.rect_filled(fill, 1.5, GOLD.gamma_multiply(0.85));
    diamond(&painter, egui::pos2(fill.right(), track.center().y), 3.5, Color32::from_rgb(255, 230, 160));

    // Rechts: Uhrzeit groß, darunter der Tag
    let right = center.x + half_width - 22.0;
    let clock_color = Color32::from_rgb(248, 244, 232);
    painter.text(egui::pos2(right, center.y - 5.0), Align2::RIGHT_CENTER, day.clock(), FontId::proportional(22.0), clock_color);
    painter.text(egui::pos2(right, center.y + 11.0), Align2::RIGHT_CENTER, format!("TAG {}", day.day), FontId::proportional(11.0), GOLD);

    // Mitte: Himmels-Medaillon, ragt über das Banner hinaus
    sky_medallion(&painter, center, 29.0, day);
}

/// Kleiner Lebensbalken: grün bei voller Gesundheit, über Gelb nach Rot.
pub fn health_bar(painter: &egui::Painter, center: egui::Pos2, width: f32, fraction: f32, alpha: f32) {
    let fraction = fraction.clamp(0.0, 1.0);
    let bar = egui::Rect::from_center_size(center, egui::vec2(width, 6.0));
    painter.rect_filled(bar.expand(1.5), 3.0, Color32::from_black_alpha((190.0 * alpha) as u8));
    let mut fill = bar;
    fill.set_width(bar.width() * fraction);
    let (r, g) = if fraction > 0.5 { ((1.0 - fraction) * 2.0, 1.0) } else { (1.0, fraction * 2.0) };
    let color = Color32::from_rgb((70.0 + r * 185.0) as u8, (60.0 + g * 160.0) as u8, 50);
    painter.rect_filled(fill, 2.0, color.gamma_multiply(alpha));
    // Heller Streifen oben für etwas Glanz
    let mut shine = fill;
    shine.set_height(2.0);
    painter.rect_filled(shine, 1.0, Color32::from_white_alpha((60.0 * alpha) as u8));
}

/// Schmale Lebensleiste in fester Farbe (Truppen rot, Soldaten blau), mit dunklem Rest.
pub fn unit_bar(painter: &egui::Painter, center: egui::Pos2, width: f32, fraction: f32, alpha: f32, color: Color32) {
    let fraction = fraction.clamp(0.0, 1.0);
    let bar = egui::Rect::from_center_size(center, egui::vec2(width, 5.0));
    painter.rect_filled(bar.expand(1.5), 2.5, Color32::from_black_alpha((200.0 * alpha) as u8));
    painter.rect_filled(bar, 2.0, Color32::from_rgb(60, 25, 30).gamma_multiply(alpha * 0.8));
    let mut fill = bar;
    fill.set_width(bar.width() * fraction);
    painter.rect_filled(fill, 2.0, color.gamma_multiply(alpha));
    let mut shine = fill;
    shine.set_height(1.5);
    painter.rect_filled(shine, 1.0, Color32::from_white_alpha((70.0 * alpha) as u8));
}

/// Gemaltes Symbol für einen Gegenstand im Inventar.
pub fn item_icon(painter: &egui::Painter, rect: egui::Rect, item: Item) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 40.0;
    let v = |x: f32, y: f32| c + egui::vec2(x * s, y * s);
    let outline = Stroke::new(1.5 * s, Color32::from_black_alpha(120));
    match item {
        Item::Runenfragment | Item::Runenstein => {
            // Blauer Kristall mit Rune (gemalt, falls das Symbol fehlt)
            let n = if item == Item::Runenstein { 1.0 } else { 0.7 };
            let punkte = vec![v(0.0, -16.0 * n), v(11.0 * n, -4.0), v(7.0 * n, 14.0 * n), v(-7.0 * n, 14.0 * n), v(-11.0 * n, -4.0)];
            painter.add(egui::Shape::convex_polygon(punkte, Color32::from_rgb(70, 130, 220), outline));
            painter.line_segment([v(0.0, -8.0 * n), v(0.0, 8.0 * n)], Stroke::new(2.0 * s, Color32::from_rgb(190, 235, 255)));
            painter.line_segment([v(-5.0 * n, -2.0), v(5.0 * n, 3.0)], Stroke::new(2.0 * s, Color32::from_rgb(190, 235, 255)));
        }
        Item::Heiltrank => {
            // Runde Flasche mit rotem Trank, Hals und Korken
            painter.circle_filled(v(0.0, 5.0), 11.0 * s, Color32::from_rgb(200, 40, 50));
            painter.circle_stroke(v(0.0, 5.0), 11.0 * s, outline);
            painter.rect_filled(egui::Rect::from_center_size(v(0.0, -9.0), egui::vec2(7.0 * s, 8.0 * s)), 1.5 * s, Color32::from_rgb(210, 225, 235));
            painter.rect_filled(egui::Rect::from_center_size(v(0.0, -14.0), egui::vec2(8.0 * s, 4.0 * s)), 1.5 * s, Color32::from_rgb(150, 100, 55));
            painter.circle_filled(v(-4.0, 1.0), 2.5 * s, Color32::from_rgb(255, 170, 170));
        }
        Item::Kristall => {
            // Hoher blauer Kristall mit Glanzkante
            let punkte = vec![v(0.0, -17.0), v(8.0, -8.0), v(7.0, 14.0), v(-7.0, 14.0), v(-8.0, -8.0)];
            painter.add(egui::Shape::convex_polygon(punkte, Color32::from_rgb(60, 156, 255), outline));
            painter.line_segment([v(0.0, -15.0), v(0.0, 12.0)], Stroke::new(2.0 * s, Color32::from_rgb(210, 241, 255)));
        }
        Item::Lehm => {
            // Zwei helle, weiche Lehmblöcke
            for (dx, dy) in [(-4.0, 5.0), (4.0, -4.0)] {
                let block = egui::Rect::from_center_size(v(dx, dy), egui::vec2(20.0 * s, 13.0 * s));
                painter.rect_filled(block, 4.0 * s, Color32::from_rgb(224, 184, 120));
                painter.rect_stroke(block, 4.0 * s, outline, egui::StrokeKind::Inside);
                painter.rect_filled(egui::Rect::from_min_size(block.left_bottom() - egui::vec2(0.0, 4.0 * s), egui::vec2(block.width(), 4.0 * s)), 2.0 * s, Color32::from_rgb(168, 182, 188));
            }
        }
        Item::Gold => {
            // Drei gestapelte Goldmünzen
            for (i, dy) in [9.0f32, 2.0, -5.0].into_iter().enumerate() {
                let mitte = v(-1.0 + i as f32 * 1.5, dy);
                let rand = egui::Rect::from_center_size(mitte, egui::vec2(26.0 * s, 9.0 * s));
                painter.rect_filled(rand.translate(egui::vec2(0.0, 2.5 * s)), 4.5 * s, Color32::from_rgb(170, 115, 25));
                painter.rect_filled(rand, 4.5 * s, Color32::from_rgb(245, 196, 64));
                painter.rect_stroke(rand, 4.5 * s, Stroke::new(1.2 * s, Color32::from_rgb(150, 100, 20)), egui::StrokeKind::Inside);
            }
            painter.circle_filled(v(4.0, -9.0), 2.0 * s, Color32::from_rgb(255, 250, 220));
        }
        Item::Wood => {
            // Zwei gestapelte Holzscheite mit Jahresringen
            for (dy, dx) in [(7.0, -2.0), (-5.0, 3.0)] {
                let log = egui::Rect::from_center_size(v(dx - 2.0, dy), egui::vec2(28.0 * s, 11.0 * s));
                painter.rect_filled(log, 5.0 * s, Color32::from_rgb(130, 82, 42));
                painter.rect_filled(log.shrink2(egui::vec2(3.0 * s, 4.0 * s)).translate(egui::vec2(0.0, -2.0 * s)), 2.0 * s, Color32::from_rgb(160, 104, 56));
                let end = v(dx + 12.0, dy);
                painter.circle_filled(end, 6.0 * s, Color32::from_rgb(222, 184, 128));
                painter.circle_stroke(end, 3.5 * s, Stroke::new(1.2 * s, Color32::from_rgb(170, 120, 70)));
                painter.circle_filled(end, 1.2 * s, Color32::from_rgb(150, 100, 55));
                painter.circle_stroke(end, 6.0 * s, Stroke::new(1.5 * s, Color32::from_rgb(90, 55, 25)));
            }
        }
        Item::Stone | Item::Ore => {
            let points = [v(-13.0, 8.0), v(-11.0, -5.0), v(-2.0, -12.0), v(10.0, -9.0), v(14.0, 2.0), v(9.0, 11.0), v(-6.0, 12.0)];
            painter.add(egui::Shape::convex_polygon(points.to_vec(), Color32::from_rgb(128, 131, 140), outline));
            let top = [v(-11.0, -5.0), v(-2.0, -12.0), v(10.0, -9.0), v(3.0, -3.0), v(-7.0, -1.0)];
            painter.add(egui::Shape::convex_polygon(top.to_vec(), Color32::from_rgb(170, 173, 182), Stroke::NONE));
            painter.line_segment([v(3.0, -3.0), v(9.0, 11.0)], Stroke::new(1.2 * s, Color32::from_rgb(95, 97, 105)));
            if item == Item::Ore {
                for (x, y) in [(-5.0, 3.0), (5.0, -6.0), (8.0, 5.0)] {
                    painter.circle_filled(v(x, y), 2.6 * s, Color32::from_rgb(190, 100, 45));
                }
            }
        }
        Item::Meat => {
            // Keule: schräges, ovales Fleischstück mit herausstehendem Knochen
            let bone = Color32::from_rgb(238, 232, 214);
            painter.line_segment([v(0.0, 0.0), v(11.0, -11.0)], Stroke::new(4.5 * s, bone));
            painter.circle_filled(v(11.0, -14.0), 3.2 * s, bone);
            painter.circle_filled(v(14.0, -11.0), 3.2 * s, bone);
            let oval = |center: egui::Pos2, a: f32, b: f32| -> Vec<egui::Pos2> {
                let (u, w) = (egui::vec2(-0.707, 0.707), egui::vec2(0.707, 0.707));
                (0..24)
                    .map(|k| {
                        let angle = k as f32 / 24.0 * std::f32::consts::TAU;
                        center + u * a * s * angle.cos() + w * b * s * angle.sin()
                    })
                    .collect()
            };
            painter.add(egui::Shape::convex_polygon(oval(v(-5.0, 5.0), 13.0, 9.5), Color32::from_rgb(140, 62, 34), outline));
            painter.add(egui::Shape::convex_polygon(oval(v(-6.0, 3.5), 10.0, 6.5), Color32::from_rgb(182, 92, 50), Stroke::NONE));
            painter.add(egui::Shape::convex_polygon(oval(v(-8.0, 1.0), 4.0, 2.2), Color32::from_rgb(222, 150, 100), Stroke::NONE));
        }
        Item::Pelt => {
            // Ausgebreitetes Fell mit vier Zipfeln
            let fur = Color32::from_rgb(150, 98, 55);
            for (x, y) in [(-12.0, -11.0), (12.0, -11.0), (-12.0, 11.0), (12.0, 11.0)] {
                painter.circle_filled(v(x, y), 4.5 * s, fur);
            }
            let body = [v(-10.0, -12.0), v(0.0, -15.0), v(10.0, -12.0), v(12.0, 0.0), v(10.0, 12.0), v(0.0, 15.0), v(-10.0, 12.0), v(-12.0, 0.0)];
            painter.add(egui::Shape::convex_polygon(body.to_vec(), fur, outline));
            painter.circle_filled(v(0.0, 0.0), 7.0 * s, Color32::from_rgb(190, 140, 90));
            for (x, y) in [(-4.0, -7.0), (5.0, 6.0), (-5.0, 7.0)] {
                painter.line_segment([v(x, y), v(x + 2.5, y - 2.5)], Stroke::new(1.2 * s, Color32::from_rgb(110, 70, 38)));
            }
        }
        Item::Wool => {
            // Wollknäuel mit loser Strähne
            let wool = Color32::from_rgb(236, 230, 214);
            painter.line_segment([v(8.0, 9.0), v(16.0, 15.0)], Stroke::new(2.0 * s, wool));
            painter.circle_filled(v(0.0, 0.0), 13.0 * s, wool);
            painter.circle_stroke(v(0.0, 0.0), 13.0 * s, outline);
            let thread = Stroke::new(1.3 * s, Color32::from_rgb(190, 180, 158));
            for (from, to) in [((-11.0, -5.0), (6.0, 11.0)), ((-7.0, -10.0), (11.0, 6.0)), ((-12.0, 2.0), (-1.0, 12.0)), ((-2.0, -12.0), (12.0, -2.0))] {
                painter.line_segment([v(from.0, from.1), v(to.0, to.1)], thread);
            }
        }
    }
}
