//! Tag-Nacht-Zyklus: Sonnen- und Mondbahn, Licht, Himmelsfarben und Sterne je Uhrzeit.
//!
//! [`DayCycle::apply`] schreibt alles in die [`Environment`](crate::Environment). Die
//! Uhrzeit selbst verwaltet das Spiel (im Multiplayer bestimmt sie der Server).

use glam::Vec3;

use crate::app::Environment;

/// Tageszeit in Stunden (0 = Mitternacht, 12 = Mittag).
#[derive(Clone, Copy, Debug)]
pub struct DayCycle {
    pub hour: f32,
    /// Vergangene Tage seit Spielbeginn (beginnt bei 1).
    pub day: u32,
    /// Echte Sekunden für eine Spielstunde am Tag.
    pub seconds_per_hour: f32,
    /// Wie viel schneller die Nacht vergeht (21 bis 5 Uhr).
    pub night_speedup: f32,
    /// Grundwert des Nebels; morgens wird es dunstiger.
    pub fog_density: f32,
}

impl Default for DayCycle {
    fn default() -> Self {
        DayCycle { hour: 8.0, day: 1, seconds_per_hour: 60.0, night_speedup: 2.0, fog_density: 0.0018 }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Grobe Tagesabschnitte für Anzeige und Spiellogik.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayPhase {
    Night,
    Dawn,
    Morning,
    Noon,
    Afternoon,
    Dusk,
    Evening,
}

impl DayPhase {
    pub fn label(self) -> &'static str {
        match self {
            DayPhase::Night => "Nacht",
            DayPhase::Dawn => "Morgendämmerung",
            DayPhase::Morning => "Morgen",
            DayPhase::Noon => "Mittag",
            DayPhase::Afternoon => "Nachmittag",
            DayPhase::Dusk => "Abenddämmerung",
            DayPhase::Evening => "Abend",
        }
    }
}

impl DayCycle {
    pub fn is_night(&self) -> bool {
        !(5.0..21.0).contains(&self.hour)
    }

    /// Lässt die Zeit laufen. `dt` in echten Sekunden.
    pub fn advance(&mut self, dt: f32) {
        let speed = if self.is_night() { self.night_speedup } else { 1.0 };
        self.hour += dt * speed / self.seconds_per_hour.max(0.01);
        while self.hour >= 24.0 {
            self.hour -= 24.0;
            self.day += 1;
        }
    }

    /// Übernimmt eine Uhrzeit vom Server. Kleine Abweichungen werden weich angeglichen,
    /// damit Licht und Schatten nicht springen.
    pub fn sync(&mut self, hour: f32, day: u32) {
        let (mine, theirs) = (self.day as f32 * 24.0 + self.hour, day as f32 * 24.0 + hour);
        let diff = theirs - mine;
        if diff.abs() > 0.25 {
            self.hour = hour;
            self.day = day;
        } else {
            self.hour += diff * 0.1;
            if self.hour >= 24.0 {
                self.hour -= 24.0;
                self.day += 1;
            } else if self.hour < 0.0 {
                self.hour += 24.0;
                self.day = self.day.saturating_sub(1).max(1);
            }
        }
    }

    pub fn phase(&self) -> DayPhase {
        match self.hour {
            h if !(4.5..21.5).contains(&h) => DayPhase::Night,
            h if h < 7.0 => DayPhase::Dawn,
            h if h < 11.0 => DayPhase::Morning,
            h if h < 14.0 => DayPhase::Noon,
            h if h < 18.0 => DayPhase::Afternoon,
            h if h < 20.0 => DayPhase::Dusk,
            _ => DayPhase::Evening,
        }
    }

    /// Uhrzeit als „14:05“.
    pub fn clock(&self) -> String {
        let minutes = (self.hour * 60.0) as u32;
        format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
    }

    /// Richtung zur Sonne. Aufgang im Osten (+X) um 6 Uhr, höchster Stand im Süden (+Z)
    /// um 12 Uhr, Untergang im Westen um 18 Uhr.
    pub fn sun_direction(&self) -> Vec3 {
        let angle = (self.hour - 6.0) / 12.0 * std::f32::consts::PI;
        Vec3::new(angle.cos(), angle.sin() * 0.85, 0.35 + angle.sin() * 0.15).normalize()
    }

    /// Richtung zum Mond (steht etwa gegenüber der Sonne, etwas versetzt).
    pub fn moon_direction(&self) -> Vec3 {
        let angle = (self.hour - 18.5) / 12.0 * std::f32::consts::PI;
        Vec3::new(angle.cos(), angle.sin() * 0.8, -0.3).normalize()
    }

    /// Schreibt Licht, Himmel und Nebel der aktuellen Uhrzeit in die Umgebung.
    pub fn apply(&self, env: &mut Environment) {
        let sun = self.sun_direction();
        let moon = self.moon_direction();
        let elevation = sun.y;

        // Gewichte der Stimmungen: Nacht, Dämmerung (goldene Stunde), Tag.
        let day = smoothstep(0.05, 0.45, elevation);
        let night = 1.0 - smoothstep(-0.3, -0.02, elevation);
        let twilight = (1.0 - day - night).max(0.0);
        let mix3 = |n: Vec3, t: Vec3, d: Vec3| n * night + t * twilight + d * day;

        env.sky_color = mix3(Vec3::new(0.025, 0.035, 0.08), Vec3::new(0.95, 0.5, 0.3), Vec3::new(0.42, 0.62, 0.95));
        env.zenith_color = mix3(Vec3::new(0.004, 0.008, 0.028), Vec3::new(0.12, 0.13, 0.36), Vec3::new(0.05, 0.2, 0.72));
        env.sky_ambient = mix3(Vec3::new(0.025, 0.035, 0.075), Vec3::new(0.24, 0.2, 0.28), Vec3::new(0.23, 0.29, 0.42));
        env.ground_ambient = mix3(Vec3::new(0.01, 0.012, 0.02), Vec3::new(0.12, 0.08, 0.06), Vec3::new(0.13, 0.11, 0.07));

        // Hauptlicht: tagsüber die Sonne, nachts der Mond. Am Übergang sind beide fast
        // dunkel, deshalb springen die Schatten beim Wechsel nicht sichtbar.
        let sun_strength = smoothstep(-0.03, 0.18, elevation);
        let moon_strength = smoothstep(-0.03, 0.25, moon.y) * (1.0 - smoothstep(-0.12, 0.02, elevation));
        let warm = Vec3::new(1.45, 0.62, 0.28);
        let noon = Vec3::new(1.38, 1.17, 0.9);
        if sun_strength >= moon_strength || elevation > 0.0 {
            env.sun_direction = sun;
            env.sun_color = warm.lerp(noon, smoothstep(0.08, 0.5, elevation)) * sun_strength;
        } else {
            env.sun_direction = moon;
            env.sun_color = Vec3::new(0.2, 0.27, 0.5) * 0.75 * moon_strength;
        }

        env.sky.sun_direction = sun;
        env.sky.sun_visible = smoothstep(-0.08, 0.02, elevation);
        env.sky.moon_direction = moon;
        env.sky.moon_visible = smoothstep(-0.05, 0.05, moon.y) * (1.0 - day * 0.8);
        env.sky.stars = night;
        env.sky.glow = twilight;

        // Morgens dunstig, nachts etwas klarer; nachts wird heller belichtet, damit man
        // im Mondlicht noch etwas erkennt.
        let morning_mist = smoothstep(4.5, 6.5, self.hour) * (1.0 - smoothstep(7.0, 10.0, self.hour));
        env.fog_density = self.fog_density * (1.0 + morning_mist * 1.8) * (1.0 - night * 0.3);
        env.exposure = 1.0 + night * 0.4 + twilight * 0.1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sonne_steht_mittags_oben_und_nachts_unten() {
        let at = |hour| DayCycle { hour, ..Default::default() };
        assert!(at(12.0).sun_direction().y > 0.7);
        assert!(at(0.0).sun_direction().y < -0.7);
        assert!(at(6.2).sun_direction().x > 0.9, "Aufgang im Osten");
        assert!(at(0.5).moon_direction().y > 0.5, "Mond nachts oben");
        assert_eq!(at(14.08).clock(), "14:04");
        assert_eq!(at(23.0).phase(), DayPhase::Night);
    }

    #[test]
    fn zeit_laeuft_und_zaehlt_tage() {
        let mut cycle = DayCycle { hour: 23.0, day: 1, seconds_per_hour: 10.0, night_speedup: 2.0, ..Default::default() };
        cycle.advance(10.0); // Nachts doppelt so schnell: zwei Stunden
        assert_eq!(cycle.day, 2);
        assert!((cycle.hour - 1.0).abs() < 1e-4);
    }

    #[test]
    fn licht_wechselt_ohne_sprung() {
        // Rund um den Sonnenuntergang darf die Lichtstärke nur sanft abnehmen.
        let mut env = Environment::default();
        let mut previous = None;
        for step in 0..200 {
            let cycle = DayCycle { hour: 17.0 + step as f32 * 0.02, ..Default::default() };
            cycle.apply(&mut env);
            let strength = env.sun_color.length();
            if let Some(prev) = previous {
                assert!((strength - prev as f32).abs() < 0.08, "Lichtsprung um {}", cycle.clock());
            }
            previous = Some(strength);
        }
    }
}
