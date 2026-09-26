//! Leistungsmessung für Entwickler: mit der Umgebungsvariablen `LEISTUNG=1` gestartet, summiert
//! `messen("name", …)` die Zeit je Abschnitt und schreibt alle drei Sekunden eine Übersicht ins Log.
//! Ohne die Variable kostet ein Aufruf nur eine Abfrage.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn aktiv() -> bool {
    static AKTIV: OnceLock<bool> = OnceLock::new();
    *AKTIV.get_or_init(|| std::env::var_os("LEISTUNG").is_some())
}

struct Stand {
    summen: BTreeMap<&'static str, (f64, u32)>,
    seit: Instant,
}

fn stand() -> &'static Mutex<Stand> {
    static STAND: OnceLock<Mutex<Stand>> = OnceLock::new();
    STAND.get_or_init(|| Mutex::new(Stand { summen: BTreeMap::new(), seit: Instant::now() }))
}

/// Führt `f` aus und merkt sich die Dauer unter `name`.
pub fn messen<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    if !aktiv() {
        return f();
    }
    let start = Instant::now();
    let ergebnis = f();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    if let Ok(mut s) = stand().lock() {
        let eintrag = s.summen.entry(name).or_insert((0.0, 0));
        eintrag.0 += ms;
        eintrag.1 += 1;
        if s.seit.elapsed().as_secs_f32() > 3.0 {
            let zeilen: Vec<String> = s.summen.iter().map(|(n, (summe, anzahl))| format!("{n} {:.2} ms", summe / *anzahl as f64)).collect();
            log::info!("Leistung: {}", zeilen.join(" | "));
            s.summen.clear();
            s.seit = Instant::now();
        }
    }
    ergebnis
}

/// Trägt die Zeit seit `start` unter `name` ein (für Abschnitte, die sich schlecht einklammern lassen).
pub fn eintragen(name: &'static str, start: Instant) {
    if aktiv() {
        messen(name, || ());
        if let Ok(mut s) = stand().lock() {
            if let Some(eintrag) = s.summen.get_mut(name) {
                eintrag.0 += start.elapsed().as_secs_f64() * 1000.0;
            }
        }
    }
}
