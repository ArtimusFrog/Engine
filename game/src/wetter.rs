//! Wetter: ziehende Wolken, Regen und Gewitter, danach ein Regenbogen, morgens Nebel in den
//! Tälern, in klaren Nächten manchmal Polarlichter.
//!
//! Das Wetter ergibt sich allein aus Tag und Uhrzeit (die der Server an alle schickt) – so sehen
//! alle Mitspieler dasselbe Wetter, ohne dass etwas zusätzlich übertragen werden muss.

use engine::noise::{hash01, Rng};
use engine::prelude::*;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Wie ein Tag wird (aus der Tagesnummer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DayKind {
    Clear,
    Cloudy,
    /// Regen von `start` bis `end` Uhr, `storm` = mit Gewitter
    Rainy { start: f32, end: f32, storm: bool },
}

pub fn day_kind(day: u32) -> DayKind {
    // Der erste Tag ist immer freundlich
    if day <= 1 {
        return DayKind::Clear;
    }
    let roll = hash01(day as i32, 17, 90_210);
    let at = hash01(day as i32, 23, 90_211);
    if roll < 0.45 {
        DayKind::Clear
    } else if roll < 0.72 {
        DayKind::Cloudy
    } else {
        let start = 9.0 + at * 8.0;
        DayKind::Rainy { start, end: start + 2.0 + at * 2.5, storm: roll > 0.9 }
    }
}

/// Wetter zu einer Uhrzeit (weich ineinander übergehend).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherState {
    pub clouds: f32,
    pub rain: f32,
    pub storm: bool,
    pub rainbow: f32,
    pub aurora: f32,
    pub mist: f32,
}

pub fn weather_at(day: u32, hour: f32) -> WeatherState {
    let mut state = WeatherState { clouds: 0.2, ..Default::default() };
    match day_kind(day) {
        DayKind::Clear => state.clouds = 0.15 + 0.1 * (hour * 0.4).sin().abs(),
        DayKind::Cloudy => state.clouds = 0.55 + 0.15 * (hour * 0.3).sin(),
        DayKind::Rainy { start, end, storm } => {
            // Wolken ziehen eine Stunde vorher auf und lösen sich danach langsam auf
            let gather = smoothstep(start - 1.5, start, hour) * (1.0 - smoothstep(end, end + 2.0, hour));
            state.clouds = 0.3 + 0.7 * gather;
            state.rain = smoothstep(start, start + 0.4, hour) * (1.0 - smoothstep(end - 0.4, end, hour));
            state.storm = storm && state.rain > 0.5;
            // Regenbogen kurz nach dem Regen, solange die Sonne scheint
            let after = hour - end;
            state.rainbow = if (0.0..1.2).contains(&after) && (7.0..19.0).contains(&hour) { (1.0 - after / 1.2) * smoothstep(0.0, 0.2, after) } else { 0.0 };
        }
    }
    // Morgennebel in den Tälern, nach Regen etwas mehr
    state.mist = smoothstep(4.0, 5.5, hour) * (1.0 - smoothstep(8.0, 10.0, hour)) * 0.8;
    // Polarlicht in manchen klaren Nächten (die Nacht gehört zum Abend-Tag)
    let night = hour >= 21.0 || hour < 4.5;
    let night_day = if hour < 12.0 { day.saturating_sub(1) } else { day };
    if night && hash01(night_day as i32, 5, 4_242) > 0.6 && state.clouds < 0.5 {
        let t = if hour >= 21.0 { hour - 21.0 } else { hour + 3.0 };
        state.aurora = smoothstep(0.0, 1.5, t) * (1.0 - smoothstep(6.0, 7.5, t));
    }
    state
}

/// Darstellung: Wetter auf Licht und Himmel anwenden, Regentropfen, Blitze.
pub struct Weather {
    current: WeatherState,
    rng: Rng,
    /// Helligkeit eines gerade zuckenden Blitzes (klingt schnell ab) und Zeit bis zum nächsten.
    flash: f32,
    next_flash: f32,
    /// Donner, der gleich (verzögert) zu hören sein soll: Restzeit, Lautstärke.
    pub thunder: Option<(f32, f32)>,
    /// Nur für Screenshots: dieses Wetter statt des berechneten.
    pub force: Option<WeatherState>,
    /// Die Kamera ist unter Tage (in einem Dungeon): kein Regen, keine Blitze
    pub unter_tage: bool,
}

impl Default for Weather {
    fn default() -> Self {
        Weather { current: WeatherState::default(), rng: Rng::new(0x3E_7732), flash: 0.0, next_flash: 1.0, thunder: None, force: None, unter_tage: false }
    }
}

impl Weather {
    /// Nur für Screenshots: ein Wetter nach Namen erzwingen.
    pub fn force_named(&mut self, name: &str) {
        let base = WeatherState::default();
        self.force = match name {
            "klar" => Some(WeatherState { clouds: 0.15, ..base }),
            "wolken" | "bewoelkt" => Some(WeatherState { clouds: 0.6, ..base }),
            "regen" => Some(WeatherState { clouds: 1.0, rain: 1.0, ..base }),
            "gewitter" => Some(WeatherState { clouds: 1.0, rain: 1.0, storm: true, ..base }),
            "regenbogen" => Some(WeatherState { clouds: 0.45, rain: 0.15, rainbow: 1.0, ..base }),
            "polarlicht" => Some(WeatherState { clouds: 0.1, aurora: 1.0, ..base }),
            "nebel" => Some(WeatherState { clouds: 0.3, mist: 1.0, ..base }),
            _ => None,
        };
    }

    pub fn state(&self) -> WeatherState {
        self.current
    }

    /// Nach `DayCycle::apply` aufrufen: dämpft Sonne und Farben bei Bewölkung, macht Regen und Blitze.
    pub fn apply(&mut self, ctx: &mut Context, day: u32, hour: f32) {
        let dt = ctx.time.delta;
        let target = self.force.unwrap_or_else(|| weather_at(day, hour));
        if self.force.is_some() {
            self.current = target;
        }
        // Weich nachführen (z. B. nach dem Vorspulen der Uhr)
        let k = (dt * 0.5).min(1.0);
        let c = &mut self.current;
        c.clouds += (target.clouds - c.clouds) * k;
        c.rain += (target.rain - c.rain) * k;
        c.rainbow += (target.rainbow - c.rainbow) * k;
        c.aurora += (target.aurora - c.aurora) * k;
        c.mist += (target.mist - c.mist) * k;
        c.storm = target.storm;
        let c = *c;

        let headless = ctx.is_headless();
        let env = &mut ctx.env;
        let overcast = smoothstep(0.35, 0.95, c.clouds);
        env.sun_color *= 1.0 - overcast * 0.65;
        let grey = |v: Vec3, amount: f32| {
            let l = v.dot(vec3(0.2126, 0.7152, 0.0722));
            v.lerp(Vec3::splat(l), amount)
        };
        env.sky_color = grey(env.sky_color, overcast * 0.6) * (1.0 - c.rain * 0.3);
        env.zenith_color = grey(env.zenith_color, overcast * 0.7) * (1.0 - c.rain * 0.35);
        env.sky_ambient = grey(env.sky_ambient, overcast * 0.4) * (1.0 + overcast * 0.2);
        env.fog_density *= 1.0 + c.rain * 2.2;
        env.sky.clouds = c.clouds;
        env.sky.rain = c.rain;
        env.sky.rainbow = c.rainbow;
        env.sky.aurora = c.aurora;
        env.sky.mist = c.mist;
        env.sky.sun_visible *= 1.0 - overcast * 0.9;

        // Blitze bei Gewitter: kurz alles hell, Donner mit Verzögerung
        if c.storm && !headless && !self.unter_tage {
            self.next_flash -= dt;
            if self.next_flash <= 0.0 {
                self.next_flash = self.rng.range(5.0, 14.0);
                self.flash = 1.0;
                let distance = self.rng.range(0.4, 3.0);
                self.thunder = Some((distance, 1.0 - distance / 4.0));
            }
        }
        if self.flash > 0.0 {
            let strength = self.flash * self.flash;
            env.sky_ambient += Vec3::new(0.9, 0.95, 1.2) * strength;
            env.sky_color += Vec3::new(0.6, 0.65, 0.8) * strength;
            env.zenith_color += Vec3::new(0.4, 0.45, 0.6) * strength;
            self.flash = (self.flash - dt * 5.0).max(0.0);
        }

        // Regentropfen rund um die Kamera (schnell fallende, leicht durchsichtige Striche)
        if c.rain > 0.02 && !ctx.is_headless() && !self.unter_tage {
            let camera = ctx.camera.position;
            let drops = (c.rain * 90.0 * dt * 60.0) as u32;
            for _ in 0..drops {
                let offset = vec3(self.rng.range(-14.0, 14.0), self.rng.range(4.0, 10.0), self.rng.range(-14.0, 14.0));
                ctx.particles.burst(Burst {
                    position: camera + offset,
                    count: 1,
                    color: vec3(0.82, 0.87, 0.95),
                    color_variation: 0.05,
                    speed: 1.0,
                    direction: vec3(0.8, -14.0, 0.3),
                    size: 0.03,
                    life: 0.8,
                    gravity: 12.0,
                    glow: 0.0,
                    grow: 0.0,
                    round: false,
                });
            }
        }
    }

    /// Donner, der jetzt zu hören ist (Lautstärke), nachdem der Blitz gezuckt hat.
    pub fn take_thunder(&mut self, dt: f32) -> Option<f32> {
        let (left, volume) = self.thunder.as_mut()?;
        *left -= dt;
        if *left <= 0.0 {
            let volume = *volume;
            self.thunder = None;
            return Some(volume);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wetter_wechselt_ueber_die_tage() {
        let kinds: Vec<DayKind> = (1..60).map(day_kind).collect();
        assert_eq!(kinds[0], DayKind::Clear, "der erste Tag ist klar");
        assert!(kinds.iter().any(|k| matches!(k, DayKind::Rainy { .. })), "nie Regen");
        assert!(kinds.iter().any(|k| *k == DayKind::Clear), "nie klar");
        // An einem Regentag regnet es mitten im Regen, danach gibt es einen Regenbogen
        let (day, start, end) = (1..60)
            .find_map(|d| if let DayKind::Rainy { start, end, .. } = day_kind(d) { Some((d, start, end)) } else { None })
            .unwrap();
        assert!(weather_at(day, (start + end) / 2.0).rain > 0.9);
        assert!(weather_at(day, end + 0.5).rainbow > 0.3 || !(7.0..19.0).contains(&(end + 0.5)));
        assert_eq!(weather_at(day, start - 3.0).rain, 0.0);
    }
}
