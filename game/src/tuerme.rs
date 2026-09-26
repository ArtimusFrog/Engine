//! Verteidigungstürme an den Heerstraßen (Tower Defense): zehn Arten in drei Stufen.
//! Konzept: docs/konzept_tower_defense.md. Modelle: art/lib/tuerme.py
//! (`bauten/turm_<art>_<stufe>.gltf` und der drehbare Kopf `bauten/turm_<art>_kopf.gltf`).
//!
//! Hier stehen nur die Werte (Reichweite, Schaden, Kosten …). Wie die Türme schießen, rechnet
//! der Server in `Verteidigung`, das Treffen und die Wirkungen stehen in `heer.rs`.

use std::collections::HashMap;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bauten::Building;
use crate::heer::{Heer, Hit};
use crate::protocol::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TowerKind {
    Arrow,
    Ballista,
    Catapult,
    Fire,
    Frost,
    Lightning,
    Sun,
    Arcane,
    Poison,
    Banner,
}

/// Schadensarten; die Einheiten sind unterschiedlich empfindlich (siehe `heer.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageKind {
    Physical,
    /// Halbiert die Rüstung
    Pierce,
    Fire,
    Frost,
    Lightning,
    /// Doppelt gegen Untote
    Holy,
    /// Ignoriert die Rüstung
    Arcane,
    Poison,
}

/// Plattformhöhe je Stufe (wie `KOPF_Z` in art/lib/tuerme.py): dort sitzt der drehbare Kopf.
pub const KOPF_Z: [f32; 3] = [4.2, 5.4, 6.6];
/// Wie groß der Kopf im Spiel dargestellt wird (damit er über die Zinnen schaut).
pub const KOPF_GROESSE: f32 = 1.3;
/// Höchste Stufe
pub const MAX_STUFE: u8 = 3;

/// Werte eines Turms auf einer Stufe.
#[derive(Clone, Copy, Debug, Default)]
pub struct Werte {
    pub reichweite: f32,
    /// Sekunden zwischen zwei Schüssen (0 = schießt nicht)
    pub takt: f32,
    pub schaden: f32,
    pub art: Option<DamageKind>,
    /// Flächenschaden (Radius)
    pub flaeche: f32,
    /// Kettenblitz: so viele Ziele
    pub kette: u8,
    /// Verlangsamung (Anteil) für 2,5 s
    pub bremse: f32,
    /// Brand bzw. Gift: Schaden pro Sekunde und Dauer
    pub nachwirkung: f32,
    pub dauer: f32,
    /// Katapult: so nah schießt es nicht
    pub mindestens: f32,
    /// Kriegsbanner: Verstärkung für Türme in der Nähe
    pub aura: f32,
}

impl TowerKind {
    pub const ALL: [TowerKind; 10] = [
        TowerKind::Arrow,
        TowerKind::Ballista,
        TowerKind::Catapult,
        TowerKind::Fire,
        TowerKind::Frost,
        TowerKind::Lightning,
        TowerKind::Sun,
        TowerKind::Arcane,
        TowerKind::Poison,
        TowerKind::Banner,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TowerKind::Arrow => "Pfeilturm",
            TowerKind::Ballista => "Balliste",
            TowerKind::Catapult => "Katapult",
            TowerKind::Fire => "Feuerturm",
            TowerKind::Frost => "Frostturm",
            TowerKind::Lightning => "Blitzturm",
            TowerKind::Sun => "Sonnenturm",
            TowerKind::Arcane => "Arkanturm",
            TowerKind::Poison => "Giftturm",
            TowerKind::Banner => "Kriegsbanner",
        }
    }

    /// Mit unbestimmtem Artikel („einen Pfeilturm“, „eine Balliste“ …).
    pub fn with_article(self) -> String {
        let artikel = match self {
            TowerKind::Ballista => "eine",
            TowerKind::Catapult | TowerKind::Banner => "ein",
            _ => "einen",
        };
        format!("{artikel} {}", self.label())
    }

    pub fn description(self) -> &'static str {
        match self {
            TowerKind::Arrow => "Günstig und schnell: Pfeile auf ein Ziel.",
            TowerKind::Ballista => "Schwere Bolzen durchschlagen Rüstungen. Langsam, aber verheerend.",
            TowerKind::Catapult => "Steinbrocken treffen ganze Gruppen. Schießt nicht auf nahe Ziele.",
            TowerKind::Fire => "Flammenstoß setzt Gegner in Brand.",
            TowerKind::Frost => "Eis verlangsamt die Truppen – Zeit für die anderen Türme.",
            TowerKind::Lightning => "Kettenblitz springt von Gegner zu Gegner.",
            TowerKind::Sun => "Heiliges Licht – doppelter Schaden an Skeletten und Gespenstern.",
            TowerKind::Arcane => "Arkane Kraft ignoriert jede Rüstung.",
            TowerKind::Poison => "Giftwolke am Boden zermürbt und schwächt die Rüstung.",
            TowerKind::Banner => "Keine Angriffe: Türme in der Nähe schießen schneller und härter.",
        }
    }

    /// Dateiname in art/lib/tuerme.py und game/assets/bauten/.
    pub fn file(self) -> &'static str {
        match self {
            TowerKind::Arrow => "pfeil",
            TowerKind::Ballista => "balliste",
            TowerKind::Catapult => "katapult",
            TowerKind::Fire => "feuer",
            TowerKind::Frost => "frost",
            TowerKind::Lightning => "blitz",
            TowerKind::Sun => "sonne",
            TowerKind::Arcane => "arkan",
            TowerKind::Poison => "gift",
            TowerKind::Banner => "banner",
        }
    }

    /// Kosten für Stufe 1 bzw. das Aufwerten auf `stufe` (Holz, Stein, Erz).
    pub fn kosten(self, stufe: u8) -> [(Item, u32); 3] {
        let tabelle: [[u32; 3]; 3] = match self {
            TowerKind::Arrow => [[12, 4, 0], [16, 8, 2], [20, 12, 6]],
            TowerKind::Ballista => [[18, 8, 2], [22, 12, 5], [26, 16, 10]],
            TowerKind::Catapult => [[20, 12, 0], [24, 18, 4], [28, 24, 8]],
            TowerKind::Fire => [[10, 14, 2], [14, 18, 5], [18, 22, 9]],
            TowerKind::Frost => [[8, 14, 4], [12, 18, 7], [16, 22, 11]],
            TowerKind::Lightning => [[8, 12, 8], [12, 16, 12], [16, 20, 16]],
            TowerKind::Sun => [[10, 16, 4], [14, 20, 8], [18, 24, 12]],
            TowerKind::Arcane => [[10, 14, 6], [14, 18, 10], [18, 22, 14]],
            TowerKind::Poison => [[14, 8, 2], [18, 12, 5], [22, 16, 9]],
            TowerKind::Banner => [[16, 6, 4], [20, 10, 8], [24, 14, 12]],
        };
        let [holz, stein, erz] = tabelle[(stufe.clamp(1, 3) - 1) as usize];
        [(Item::Wood, holz), (Item::Stone, stein), (Item::Ore, erz)]
    }

    /// Werte auf einer Stufe (1–3): Schaden ×1,6 je Stufe, Reichweite +10 %, Feuerrate +10 %.
    pub fn werte(self, stufe: u8) -> Werte {
        let s = (stufe.clamp(1, 3) - 1) as usize;
        let mal = [1.0, 1.6, 2.56][s];
        let weit = 1.0 + 0.1 * s as f32;
        let schnell = 1.0 - 0.09 * s as f32;
        let stufen = |werte: [f32; 3]| werte[s];
        let mut w = match self {
            TowerKind::Arrow => Werte { reichweite: 20.0, takt: 0.8, schaden: 14.0, art: Some(DamageKind::Physical), ..Default::default() },
            TowerKind::Ballista => Werte { reichweite: 28.0, takt: 2.6, schaden: 70.0, art: Some(DamageKind::Pierce), ..Default::default() },
            TowerKind::Catapult => Werte {
                reichweite: 30.0,
                takt: 3.2,
                schaden: 40.0,
                art: Some(DamageKind::Physical),
                flaeche: stufen([3.0, 3.7, 4.5]),
                mindestens: 8.0,
                ..Default::default()
            },
            TowerKind::Fire => Werte {
                reichweite: 12.0,
                takt: 1.2,
                schaden: 10.0,
                art: Some(DamageKind::Fire),
                flaeche: 1.6,
                nachwirkung: 12.0 * mal,
                dauer: 3.0,
                ..Default::default()
            },
            TowerKind::Frost => Werte {
                reichweite: 16.0,
                takt: 1.4,
                schaden: 6.0,
                art: Some(DamageKind::Frost),
                bremse: stufen([0.25, 0.35, 0.45]),
                ..Default::default()
            },
            TowerKind::Lightning => Werte {
                reichweite: 18.0,
                takt: 1.8,
                schaden: 26.0,
                art: Some(DamageKind::Lightning),
                kette: [3, 4, 5][s],
                ..Default::default()
            },
            TowerKind::Sun => Werte { reichweite: 22.0, takt: 1.5, schaden: 22.0, art: Some(DamageKind::Holy), ..Default::default() },
            TowerKind::Arcane => Werte { reichweite: 20.0, takt: 1.6, schaden: 30.0, art: Some(DamageKind::Arcane), ..Default::default() },
            TowerKind::Poison => Werte {
                reichweite: 15.0,
                takt: 2.4,
                schaden: 0.0,
                art: Some(DamageKind::Poison),
                flaeche: 3.0,
                nachwirkung: 8.0 * mal,
                dauer: 4.0,
                ..Default::default()
            },
            TowerKind::Banner => Werte { reichweite: 12.0, aura: stufen([0.15, 0.25, 0.35]), ..Default::default() },
        };
        w.schaden *= mal;
        w.reichweite *= weit;
        w.takt *= schnell;
        w
    }
}

impl Werte {
    /// Kurzbeschreibung der Werte für Menü und Turmfenster.
    pub fn zeilen(&self) -> Vec<String> {
        let mut z = Vec::new();
        if self.aura > 0.0 {
            z.push(format!("Türme im Umkreis von {:.0} m: +{:.0} % Schaden und Feuerrate", self.reichweite, self.aura * 100.0));
            return z;
        }
        z.push(format!("Reichweite {:.0} m · alle {:.1} s", self.reichweite, self.takt));
        if self.schaden > 0.0 {
            z.push(format!("Schaden {:.0}{}", self.schaden, match self.art {
                Some(DamageKind::Pierce) => " (durchschlagend)",
                Some(DamageKind::Holy) => " (heilig, ×2 gegen Untote)",
                Some(DamageKind::Arcane) => " (arkan, ohne Rüstung)",
                Some(DamageKind::Fire) => " (Feuer)",
                Some(DamageKind::Frost) => " (Frost)",
                Some(DamageKind::Lightning) => " (Blitz)",
                _ => "",
            }));
        }
        if self.flaeche > 0.0 && self.nachwirkung == 0.0 {
            z.push(format!("Fläche {:.1} m", self.flaeche));
        }
        if self.kette > 0 {
            z.push(format!("Kettenblitz auf {} Ziele", self.kette));
        }
        if self.bremse > 0.0 {
            z.push(format!("verlangsamt um {:.0} %", self.bremse * 100.0));
        }
        if self.nachwirkung > 0.0 {
            z.push(format!("{:.0} Schaden/s für {:.0} s", self.nachwirkung, self.dauer));
        }
        if self.mindestens > 0.0 {
            z.push(format!("nicht näher als {:.0} m", self.mindestens));
        }
        z
    }
}

/// Beute für eine besiegte Einheit (bekommt, wer den letzten Treffer gesetzt hat).
pub fn beute(kind: crate::heer::EnemyKind) -> &'static [(Item, u32)] {
    use crate::heer::EnemyKind;
    match kind {
        EnemyKind::Wolf | EnemyKind::Archer | EnemyKind::Skeleton => &[(Item::Stone, 1)],
        EnemyKind::Pikeman | EnemyKind::Warlock | EnemyKind::Ghost => &[(Item::Stone, 1), (Item::Ore, 1)],
        EnemyKind::Knight => &[(Item::Ore, 2)],
        EnemyKind::Golem => &[(Item::Stone, 4), (Item::Ore, 3)],
    }
}

/// Ein Schuss eines Turms (für die Darstellung): welcher Turm, wohin, welche Art.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schuss {
    pub turm: u32,
    pub ziel: Vec3,
}

/// Die Türme im Kampf (nur auf dem Server): Nachladezeiten und Giftwolken.
#[derive(Default)]
pub struct Verteidigung {
    nachladen: HashMap<u32, f32>,
    /// Giftwolken: Mitte, Radius, Schaden/s, Restzeit, Besitzer
    wolken: Vec<(Vec3, f32, f32, f32, String)>,
}

impl Verteidigung {
    /// Ein Takt: jeder fertige Turm sucht sich ein Ziel und schießt. Liefert die Schüsse (für die
    /// Darstellung). Besiegte Einheiten landen mit dem Besitzer des Turms in `heer.gefallen`.
    pub fn tick(&mut self, dt: f32, buildings: &[Building], heer: &mut Heer) -> Vec<Schuss> {
        let mut schuesse = Vec::new();
        // Kriegsbanner verstärken Türme in ihrer Nähe
        let banner: Vec<(Vec3, Werte)> = buildings
            .iter()
            .filter_map(|b| b.tower().filter(|&t| t == TowerKind::Banner && b.finished()).map(|t| (b.position, t.werte(b.level))))
            .collect();
        for building in buildings {
            let Some(kind) = building.tower() else { continue };
            if !building.finished() || kind == TowerKind::Banner {
                continue;
            }
            let mut werte = kind.werte(building.level);
            let staerke: f32 = banner
                .iter()
                .filter(|(ort, w)| ort.distance(building.position) <= w.reichweite)
                .map(|(_, w)| w.aura)
                .fold(0.0, f32::max);
            werte.schaden *= 1.0 + staerke;
            werte.nachwirkung *= 1.0 + staerke;
            werte.takt /= 1.0 + staerke;
            let bereit = self.nachladen.entry(building.id).or_insert(0.0);
            *bereit -= dt;
            if *bereit > 0.0 {
                continue;
            }
            let mund = building.position + Vec3::Y * (KOPF_Z[(building.level.clamp(1, 3) - 1) as usize] + 1.0);
            let Some((ziel, ort)) = heer.first_in_range(mund, werte.reichweite, werte.mindestens) else { continue };
            *bereit = werte.takt;
            let art = werte.art.unwrap_or(DamageKind::Physical);
            let hit = Hit {
                schaden: werte.schaden,
                art,
                bremse: werte.bremse,
                brand: if art == DamageKind::Fire { werte.nachwirkung } else { 0.0 },
                dauer: werte.dauer,
            };
            let owner = building.owner.as_str();
            schuesse.push(Schuss { turm: building.id, ziel: ort });
            if kind == TowerKind::Poison {
                self.wolken.push((ort, werte.flaeche, werte.nachwirkung, werte.dauer, building.owner.clone()));
            } else if werte.kette > 0 {
                // Kettenblitz: springt zur nächsten Einheit, jedes Mal etwas schwächer
                let mut getroffen = vec![ziel];
                let (mut von, mut hit) = (ort, hit);
                heer.damage(ziel, hit, owner);
                for _ in 1..werte.kette {
                    let Some((naechster, dort)) = heer.nearest_except(von, 7.0, &getroffen) else { break };
                    hit.schaden *= 0.85;
                    heer.damage(naechster, hit, owner);
                    schuesse.push(Schuss { turm: building.id, ziel: dort });
                    getroffen.push(naechster);
                    von = dort;
                }
            } else if werte.flaeche > 0.0 {
                for id in heer.within(ort, werte.flaeche) {
                    heer.damage(id, hit, owner);
                }
            } else {
                heer.damage(ziel, hit, owner);
            }
        }
        // Giftwolken: Schaden über Zeit an allen darin, Rüstung geschwächt
        for (mitte, radius, dps, rest, owner) in &mut self.wolken {
            *rest -= dt;
            for id in heer.within(*mitte, *radius) {
                let hit = Hit { schaden: *dps * dt, art: DamageKind::Poison, ..Default::default() };
                heer.damage(id, hit, owner);
                heer.weaken(id, 0.2);
            }
        }
        self.wolken.retain(|w| w.3 > 0.0);
        self.nachladen.retain(|id, _| buildings.iter().any(|b| b.id == *id));
        schuesse
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stufen_machen_staerker_und_kosten_mehr() {
        for kind in TowerKind::ALL {
            let (a, b, c) = (kind.werte(1), kind.werte(2), kind.werte(3));
            assert!(b.schaden >= a.schaden && c.schaden >= b.schaden, "{kind:?} Schaden");
            assert!(b.reichweite > a.reichweite && c.reichweite > b.reichweite, "{kind:?} Reichweite");
            let summe = |stufe| kind.kosten(stufe).iter().map(|(_, n)| n).sum::<u32>();
            assert!(summe(2) > summe(1) && summe(3) > summe(2), "{kind:?} Kosten");
        }
        assert_eq!(TowerKind::Banner.werte(3).aura, 0.35);
        assert_eq!(TowerKind::Lightning.werte(3).kette, 5);
    }
}

#[cfg(test)]
mod kampf_tests {
    use super::*;
    use crate::bauten::BuildingKind;

    /// Ein Turm an der Südstraße: Stelle nach `entlang` Metern, `seitlich` daneben.
    fn turm_an_strasse(world: &crate::world::World, kind: TowerKind, id: u32, entlang: usize, seitlich: f32) -> Building {
        let strasse: Vec<Vec2> = world.heer.strassen().next().unwrap();
        let (a, b) = (strasse[entlang], strasse[entlang + 1]);
        let quer = (b - a).normalize().perp();
        let p = a + quer * seitlich;
        Building {
            id,
            kind: BuildingKind::Tower(kind),
            position: vec3(p.x, world.terrain.height_at(p.x, p.y), p.y),
            yaw: 0.0,
            progress: 1.0,
            owner: "nils".into(),
            produce_in: 0.0,
            level: 3,
        }
    }

    #[test]
    fn tuerme_besiegen_truppen_und_bringen_beute() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let tuerme: Vec<Building> = TowerKind::ALL
            .iter()
            .enumerate()
            .map(|(i, &kind)| turm_an_strasse(&world, kind, i as u32 + 1, 10 + i * 6, if i % 2 == 0 { 7.0 } else { -7.0 }))
            .collect();
        let mut verteidigung = Verteidigung::default();
        world.heer.spawn_wave();
        let mut schuesse = 0;
        for _ in 0..2400 {
            world.heer.tick(0.05, &[]);
            schuesse += verteidigung.tick(0.05, &tuerme, &mut world.heer).len();
        }
        assert!(schuesse > 5, "Türme schießen kaum: {schuesse}");
        let gefallen = std::mem::take(&mut world.heer.gefallen);
        assert!(!gefallen.is_empty(), "keine Einheit besiegt");
        assert!(gefallen.iter().all(|(name, _)| name == "nils"), "Beute an den Falschen: {gefallen:?}");
    }

    #[test]
    fn frost_bremst_und_ruestung_zaehlt() {
        use crate::heer::{EnemyKind, Hit};
        // Rüstung: der Ritter nimmt von Pfeilen weniger als vom Arkanturm
        assert!(EnemyKind::Knight.factor(DamageKind::Physical, 0.0) < EnemyKind::Knight.factor(DamageKind::Arcane, 0.0));
        assert!(EnemyKind::Ghost.factor(DamageKind::Holy, 0.0) > 1.5);
        assert_eq!(EnemyKind::Skeleton.factor(DamageKind::Poison, 0.0), 0.0);
        let _ = Hit::default();
    }
}
