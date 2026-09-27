//! Serverbrowser: offene Spiele finden, ohne die Adresse eintippen zu müssen.
//!
//! - Jeder Server (Host und dedizierter Server) beantwortet auf `Spielport + 1` (UDP) kurze
//!   Statusanfragen: Name, Spieler, Version, Welle.
//! - Der Browser fragt per Rundruf im Heimnetz (255.255.255.255) und gezielt bei den bekannten
//!   Servern: den offiziellen aus `server.json` auf der Webseite und den zuletzt benutzten.
//! - Antwortet ein Server nicht (z. B. Port gesperrt), steht er trotzdem in der Liste und man kann
//!   ihm beitreten – nur ohne Spielerzahl und Ping.

use std::collections::BTreeMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::protocol::{DEFAULT_PORT, PROTOCOL_ID};

const ANFRAGE: &[u8] = b"ENGINE-JN-STATUS?";
/// Liste der offiziellen Server auf der Webseite.
const SERVERLISTE: &str = "https://buddysagainstbets.com/server.json";

/// Was ein Server über sich sagt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub name: String,
    pub spieler: u16,
    pub max: u16,
    pub protokoll: u64,
    /// Spielport (die Statusanfrage ging an Port + 1)
    pub port: u16,
    pub welle: u32,
}

/// Beantwortet Statusanfragen (beim Server, nicht blockierend).
pub struct StatusAntwort {
    socket: UdpSocket,
}

impl StatusAntwort {
    pub fn neu(spielport: u16) -> Option<StatusAntwort> {
        let socket = match UdpSocket::bind(("0.0.0.0", spielport.wrapping_add(1))) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Statusport {} nicht verfügbar: {e}", spielport.wrapping_add(1));
                return None;
            }
        };
        socket.set_nonblocking(true).ok()?;
        log::info!("Statusanfragen an Port {}", spielport.wrapping_add(1));
        Some(StatusAntwort { socket })
    }

    /// Einmal pro Takt: alle wartenden Anfragen beantworten.
    pub fn beantworten(&self, status: impl Fn() -> Status) {
        let mut puffer = [0u8; 64];
        let mut antwort = None;
        for _ in 0..16 {
            let Ok((n, von)) = self.socket.recv_from(&mut puffer) else { break };
            if &puffer[..n] != ANFRAGE {
                continue;
            }
            let bytes = antwort.get_or_insert_with(|| serde_json::to_vec(&status()).unwrap_or_default());
            let _ = self.socket.send_to(bytes, von);
        }
    }
}

/// Ein Eintrag im Serverbrowser.
#[derive(Clone, Debug)]
pub struct Eintrag {
    /// Adresse zum Beitreten (Host:Spielport)
    pub adresse: String,
    pub name: String,
    pub herkunft: Herkunft,
    pub status: Option<Status>,
    pub ping_ms: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Herkunft {
    Offiziell,
    Heimnetz,
    Zuletzt,
}

impl Herkunft {
    pub fn label(self) -> &'static str {
        match self {
            Herkunft::Offiziell => "Offizieller Server",
            Herkunft::Heimnetz => "Im Heimnetz",
            Herkunft::Zuletzt => "Zuletzt verbunden",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Offiziell {
    name: String,
    adresse: String,
}

/// Sucht Server und sammelt die Antworten (im Hintergrund, die Oberfläche fragt jedes Bild).
pub struct Suche {
    socket: Option<UdpSocket>,
    gesendet: BTreeMap<SocketAddr, Instant>,
    pub eintraege: BTreeMap<String, Eintrag>,
    offizielle: Arc<Mutex<Option<Vec<Offiziell>>>>,
    offizielle_uebernommen: bool,
    zuletzt: Vec<String>,
    gestartet: Instant,
}

/// "host" oder "host:port" → Adresse zum Beitreten und Adresse der Statusanfrage.
fn aufloesen(adresse: &str) -> Option<(String, SocketAddr)> {
    let voll = if adresse.contains(':') { adresse.to_string() } else { format!("{adresse}:{DEFAULT_PORT}") };
    let spiel = voll.to_socket_addrs().ok()?.find(|a| a.is_ipv4())?;
    let status = SocketAddr::new(spiel.ip(), spiel.port().wrapping_add(1));
    Some((voll, status))
}

impl Suche {
    /// Startet die Suche: Liste der offiziellen Server laden, im Heimnetz rufen, bekannte fragen.
    pub fn starten(zuletzt: &[String]) -> Suche {
        let offizielle = Arc::new(Mutex::new(None));
        let ziel = offizielle.clone();
        std::thread::spawn(move || {
            let liste = ureq::get(SERVERLISTE)
                .config()
                .timeout_global(Some(Duration::from_secs(5)))
                .build()
                .call()
                .ok()
                .and_then(|mut antwort| antwort.body_mut().read_to_string().ok())
                .and_then(|text| serde_json::from_str::<Vec<Offiziell>>(&text).ok())
                .unwrap_or_default();
            if let Ok(mut z) = ziel.lock() {
                *z = Some(liste);
            }
        });
        let socket = UdpSocket::bind(("0.0.0.0", 0)).ok().filter(|s| s.set_nonblocking(true).is_ok() && s.set_broadcast(true).is_ok());
        let mut suche = Suche {
            socket,
            gesendet: BTreeMap::new(),
            eintraege: BTreeMap::new(),
            offizielle,
            offizielle_uebernommen: false,
            zuletzt: zuletzt.iter().filter(|a| !a.trim().is_empty()).cloned().collect(),
            gestartet: Instant::now(),
        };
        suche.aktualisieren();
        suche
    }

    /// Fragt alle erneut (Knopf „Aktualisieren“, und alle paar Sekunden).
    pub fn aktualisieren(&mut self) {
        for eintrag in self.eintraege.values_mut() {
            eintrag.ping_ms = None;
        }
        let rundruf = SocketAddr::from(([255, 255, 255, 255], DEFAULT_PORT + 1));
        self.fragen(rundruf);
        let bekannte: Vec<String> = self.eintraege.keys().cloned().chain(self.zuletzt.clone()).collect();
        for adresse in bekannte {
            if let Some((voll, status)) = aufloesen(&adresse) {
                if !self.eintraege.contains_key(&voll) {
                    let name = voll.clone();
                    self.eintraege.insert(voll.clone(), Eintrag { adresse: voll, name, herkunft: Herkunft::Zuletzt, status: None, ping_ms: None });
                }
                self.fragen(status);
            }
        }
        self.gestartet = Instant::now();
    }

    fn fragen(&mut self, ziel: SocketAddr) {
        if let Some(socket) = &self.socket {
            if socket.send_to(ANFRAGE, ziel).is_ok() {
                self.gesendet.insert(ziel, Instant::now());
            }
        }
    }

    /// Einmal pro Bild: Antworten einsammeln, die offizielle Liste übernehmen.
    pub fn abholen(&mut self) {
        if !self.offizielle_uebernommen {
            let liste = self.offizielle.lock().ok().and_then(|mut l| l.take());
            if let Some(liste) = liste {
                self.offizielle_uebernommen = true;
                for s in liste {
                    if let Some((voll, status)) = aufloesen(&s.adresse) {
                        let alt = self.eintraege.remove(&voll);
                        self.eintraege.insert(voll.clone(), Eintrag {
                            adresse: voll,
                            name: s.name,
                            herkunft: Herkunft::Offiziell,
                            status: alt.as_ref().and_then(|e| e.status.clone()),
                            ping_ms: alt.and_then(|e| e.ping_ms),
                        });
                        self.fragen(status);
                    }
                }
            }
        }
        let Some(socket) = &self.socket else { return };
        let mut puffer = [0u8; 1024];
        while let Ok((n, von)) = socket.recv_from(&mut puffer) {
            let Ok(status) = serde_json::from_slice::<Status>(&puffer[..n]) else { continue };
            let gefragt = self.gesendet.get(&von).or_else(|| self.gesendet.get(&SocketAddr::from(([255, 255, 255, 255], von.port()))));
            let ping = gefragt.map(|t| t.elapsed().as_millis() as u32);
            let adresse = format!("{}:{}", von.ip(), status.port);
            // Bekannt unter einem Namen (offizieller Server, zuletzt benutzt)? Dann dort eintragen.
            let schluessel = self
                .eintraege
                .iter()
                .find(|(k, _)| aufloesen(k).is_some_and(|(_, s)| s == von))
                .map(|(k, _)| k.clone())
                .unwrap_or(adresse.clone());
            let eintrag = self.eintraege.entry(schluessel).or_insert_with(|| Eintrag {
                adresse: adresse.clone(),
                name: status.name.clone(),
                herkunft: Herkunft::Heimnetz,
                status: None,
                ping_ms: None,
            });
            if eintrag.herkunft != Herkunft::Offiziell {
                eintrag.name = status.name.clone();
            }
            eintrag.status = Some(status);
            eintrag.ping_ms = ping;
        }
    }

    /// Läuft die Suche noch (die ersten zwei Sekunden)?
    pub fn sucht(&self) -> bool {
        self.gestartet.elapsed() < Duration::from_secs(2) || !self.offizielle_uebernommen
    }

    /// Seit wann die letzte Runde läuft (für das automatische Aktualisieren).
    pub fn alter(&self) -> Duration {
        self.gestartet.elapsed()
    }

    /// Passt ein Server zu dieser Spielversion?
    pub fn passt(status: &Status) -> bool {
        status.protokoll == PROTOCOL_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_antwortet_und_browser_findet_ihn() {
        // Einen freien Spielport suchen, dessen Statusport auch frei ist
        let port = (41000..41100).find(|p| UdpSocket::bind(("0.0.0.0", p + 1)).is_ok()).unwrap();
        let antwort = StatusAntwort::neu(port).expect("Statusport");
        let adresse = format!("127.0.0.1:{port}");
        let mut suche = Suche::starten(std::slice::from_ref(&adresse));
        let status = || Status { name: "Testinsel".into(), spieler: 2, max: 8, protokoll: PROTOCOL_ID, port, welle: 3 };
        for _ in 0..100 {
            antwort.beantworten(status);
            suche.abholen();
            if suche.eintraege.get(&adresse).is_some_and(|e| e.status.is_some()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let e = &suche.eintraege[&adresse];
        let s = e.status.as_ref().expect("keine Antwort");
        assert_eq!((s.name.as_str(), s.spieler, s.welle), ("Testinsel", 2, 3));
        assert!(Suche::passt(s));
        assert!(e.ping_ms.is_some());
    }
}
