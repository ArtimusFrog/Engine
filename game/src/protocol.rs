//! Was zwischen Server und Clients über das Netzwerk geht.

use glam::{Quat, Vec2, Vec3};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 7777;

/// Bei jeder inkompatiblen Änderung an diesen Nachrichten hochzählen. Server und Client
/// mit unterschiedlicher ID können sich nicht verbinden.
pub const PROTOCOL_ID: u64 = 0x4A4E_0000_0000_0025;

pub type PlayerId = u64;
pub type NetId = u32;

/// Spieler-ID des Hosts (spielt selbst auf dem Server-Rechner).
pub const HOST_PLAYER: PlayerId = 0;

/// Eingaben eines Spielers für einen Takt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    /// Fortlaufende Nummer, damit der Server bestätigen kann, was er verarbeitet hat.
    pub seq: u32,
    /// Gewünschte Laufrichtung in der XZ-Ebene, Länge 0..1.
    pub wish: Vec2,
    pub sprint: bool,
    pub jump: bool,
    /// Zielpunkt, falls in diesem Takt ein Zauber gewirkt wird.
    pub cast: Option<Vec3>,
    /// ID eines Rohstoffs, auf den in diesem Takt geschlagen wird.
    pub harvest: Option<u32>,
    /// Werkzeug in der Hand (Auswahlleiste).
    pub tool: Tool,
    /// Admin: frei fliegen, durch Wände (`rise` = hoch/runter, -1..1)
    pub noclip: bool,
    pub rise: f32,
}

/// Wetter, das das Admin-Panel erzwingen kann (0 = automatisch nach Tag und Uhrzeit).
pub const WETTER: [&str; 8] = ["automatisch", "klar", "wolken", "regen", "gewitter", "regenbogen", "polarlicht", "nebel"];

/// Befehle aus dem Admin-Panel (Taste X).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AdminCommand {
    /// Wetter nach `WETTER` (0 = automatisch)
    Weather(u8),
    /// Truppen aus der Schattenfestung an/aus
    Waves(bool),
    /// Sofort eine Welle
    WaveNow,
    /// Alle Truppen entfernen
    ClearEnemies,
    /// Wellen zurück auf Welle 1, volle Leben
    ResetWaves,
    /// Schwierigkeit (setzt die Wellen zurück)
    Schwierigkeit(crate::td::Schwierigkeit),
    /// Nach Welle 30 weiter (Endlosmodus) oder Schluss mit Sieg
    Endlos(bool),
    /// Gold für den, der den Befehl gibt (zum Testen)
    Gold(u32),
    /// Zu einer Welle springen (die nächste ist dann diese + 1)
    SpringeZuWelle(u32),
    /// Vier Runenfragmente für den, der den Befehl gibt (zum Testen)
    Runenfragmente,
    /// Alle Lager der Wildnis sofort wieder besetzen
    LagerNeu,
}

/// Nur dieser Spieler darf das Admin-Panel (Taste X) benutzen.
pub const ADMIN_NAME: &str = "nilsl";

/// Darf ein Spieler mit diesem Namen Admin-Befehle geben?
pub fn ist_admin(name: &str) -> bool {
    crate::save::player_key(name) == ADMIN_NAME
}

/// Runen: Fragmente am Runenbrunnen der Burg zu einem Runenstein vereinen, einen Runenstein in
/// den Schutzstein eines Siedlungsplatzes setzen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunenBefehl {
    Schmieden,
    Einsetzen(u8),
}

/// So viele Runenfragmente ergeben einen Runenstein.
pub const FRAGMENTE_JE_STEIN: u32 = 4;

/// Was in der Auswahlleiste liegt: zwei Werkzeuge und die drei Fähigkeiten der Figur.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    /// Baut Stein- und Erzvorkommen ab.
    #[default]
    Pickaxe,
    /// Fällt Bäume.
    Axe,
    /// Fähigkeit 0–3 der Klasse (3 = ultimativ, siehe `faehigkeiten.rs`)
    Faehigkeit(u8),
}

impl Tool {
    /// Belegung der Auswahlleiste (Platz 1, 2, …).
    pub const HOTBAR: [Tool; 6] = [Tool::Pickaxe, Tool::Axe, Tool::Faehigkeit(0), Tool::Faehigkeit(1), Tool::Faehigkeit(2), Tool::Faehigkeit(3)];
    /// Der Standardangriff (Taste 3)
    pub const ANGRIFF: Tool = Tool::Faehigkeit(0);

    /// Platz in der Auswahlleiste (0 = Taste 1).
    pub fn slot(self) -> usize {
        Tool::HOTBAR.iter().position(|&t| t == self).unwrap_or(0)
    }

    /// Die Fähigkeit in der Hand (für die Klasse der Figur).
    pub fn faehigkeit(self, class: CharacterClass) -> Option<crate::faehigkeiten::Faehigkeit> {
        match self {
            Tool::Faehigkeit(platz) => Some(crate::faehigkeiten::Faehigkeit::von(class, platz)),
            _ => None,
        }
    }

    pub fn label(self, class: CharacterClass) -> &'static str {
        match self {
            Tool::Pickaxe => "Spitzhacke",
            Tool::Axe => "Axt",
            Tool::Faehigkeit(_) => self.faehigkeit(class).map_or("", |f| f.label()),
        }
    }

    /// Symbol in `game/assets/icons/`.
    pub fn icon_file(self, class: CharacterClass) -> &'static str {
        match self {
            Tool::Pickaxe => "spitzhacke",
            Tool::Axe => "axt",
            Tool::Faehigkeit(_) => self.faehigkeit(class).map_or("", |f| f.icon_file()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectKind {
    Ball,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub facing: f32,
    /// Letzte Eingabe dieses Spielers, die im Zustand schon enthalten ist.
    pub last_input: u32,
    pub tool: Tool,
    /// Lebenspunkte (die höchsten stehen in `CharacterClass::max_leben`)
    pub leben: u16,
    /// Waffe in der Hand (0 = Startwaffe, sonst ID aus `waffen.rs`)
    pub waffe: u8,
    /// Arkane Ladungen (Magier)
    pub ladung: u8,
    /// Getragene Rüstung (Kopf, Brust, Füße)
    pub ruestung: [u8; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectState {
    pub id: NetId,
    pub position: Vec3,
    pub rotation: Quat,
}

/// Zustand der Welt zu einem Server-Takt. Ruhende Objekte fehlen meist, um Bandbreite
/// zu sparen; der Client behält dann ihre letzte bekannte Lage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub tick: u32,
    pub players: Vec<PlayerState>,
    pub objects: Vec<ObjectState>,
    /// Uhrzeit (Stunden) und Tag – der Server bestimmt die Tageszeit für alle.
    pub hour: f32,
    pub day: u32,
    /// Tiere, die sich bewegt haben (alle paar Sekunden alle).
    pub animals: Vec<crate::animals::AnimalState>,
    /// Alle Truppen der Festung, die gerade unterwegs sind
    pub enemies: Vec<crate::heer::EnemyState>,
    /// Angriffe der Truppen seit dem letzten Schnappschuss: (Art, von, Ziel)
    pub strikes: Vec<(crate::heer::EnemyKind, Vec3, Vec3)>,
    /// Erzwungenes Wetter (Index in `WETTER`)
    pub weather: u8,
    /// Schüsse der Türme seit dem letzten Schnappschuss
    pub shots: Vec<crate::tuerme::Schuss>,
    /// Stand der Verteidigung: Wellen, Leben, Vorschau, Soldaten …
    pub td: crate::td::TdStand,
    /// Ereignisse seit dem letzten Schnappschuss (Bossfähigkeiten, Explosionen …)
    pub ereignisse: Vec<crate::td::Ereignis>,
    /// Arbeiter der Rohstoffgebäude
    pub arbeiter: Vec<crate::arbeiter::ArbeiterState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { player_id: PlayerId, tick: u32 },
    PlayerJoined { player_id: PlayerId, name: String, class: CharacterClass },
    PlayerLeft { player_id: PlayerId },
    /// `by`: wer das Objekt geworfen hat (für die Wurf-Animation).
    Spawn { id: NetId, kind: ObjectKind, position: Vec3, velocity: Vec3, by: Option<PlayerId> },
    Despawn { id: NetId },
    Snapshot(Snapshot),
    /// Ein Rohstoff wurde getroffen und hat noch `health` Schläge übrig.
    ResourceHit { id: u32, health: u8, by: PlayerId },
    ResourceGone { id: u32, by: PlayerId },
    ResourceBack { id: u32 },
    /// Für neue Spieler: welche Rohstoffe fehlen oder beschädigt sind.
    ResourceStates { gone: Vec<u32>, damaged: Vec<(u32, u8)> },
    /// Das eigene Inventar hat sich geändert.
    Inventory(Inventory),
    /// Ein Spieler setzt eine Fähigkeit ein: Geschosse fliegen von `origin` nach `target`;
    /// bei Nahkampf und Wirkungen um sich ist `target` die Mitte. `hit`: trifft sie etwas?
    /// `stufe`: Kombo des Hammerschlags (0–2) bzw. 1 = Arkanlanze; `kette`: wo der Wurfhammer
    /// nach dem ersten Treffer noch aufschlägt.
    SpellCast { by: PlayerId, origin: Vec3, target: Vec3, hit: bool, art: crate::faehigkeiten::Faehigkeit, stufe: u8, kette: Vec<Vec3> },
    /// Ein Spieler wurde getroffen (roter Rand, Klang)
    SpielerGetroffen { player: PlayerId, schaden: u16 },
    /// Ein Spieler ist gefallen und steht an seinem Startpunkt wieder auf
    SpielerGefallen { player: PlayerId, von: String },
    /// Beute liegt jetzt am Boden (beim Beitreten: alles, was gerade liegt)
    Beute(Vec<crate::beute::Bodenbeute>),
    /// Beute wurde aufgehoben oder ist verschwunden
    BeuteWeg(u32),
    /// Ein Tier wurde getroffen und hat noch `health` Leben (0 = erlegt).
    AnimalHit { id: u16, health: u8, by: PlayerId },
    /// Chatnachricht eines Spielers (vom Server geprüft).
    Chat { from: PlayerId, name: String, text: String },
    /// Ein Spieler hat ein Gebäude in Auftrag gegeben – es entsteht jetzt bei allen.
    BuildingPlaced(crate::bauten::Building),
    /// Für neue Spieler: alle Gebäude mit ihrem Baufortschritt.
    Buildings(Vec<crate::bauten::Building>),
    /// Der eigene Bauauftrag wurde abgelehnt (Grund zum Anzeigen).
    BuildRefused(String),
    /// Ein Gebäude hat sich geändert (Turm aufgewertet)
    BuildingChanged(crate::bauten::Building),
    /// Ein Gebäude wurde abgerissen
    BuildingRemoved(u32),
    /// Meldung an alle (neue Welle, Durchbruch, Niederlage)
    Notice(String),
    /// Auswertung einer überstandenen Welle
    Bericht(crate::td::WellenBericht),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Die letzten paar Eingaben (älteste zuerst). Mehrfach senden gleicht Paketverlust aus.
    Inputs(Vec<PlayerInput>),
    /// Chatnachricht an alle.
    Chat(String),
    /// Gebäude errichten: Art, Mitte (x, z) und Drehung.
    Build { kind: crate::bauten::BuildingKind, at: Vec2, yaw: f32 },
    /// Befehl aus dem Admin-Panel
    Admin(AdminCommand),
    /// Verteidigung: Turm aufwerten, abreißen, Zielmodus, Welle rufen, Straße wählen
    Td(crate::td::TdBefehl),
    /// Runenstein schmieden oder in einen Schutzstein setzen
    Runen(RunenBefehl),
    /// Beute vom Boden aufheben (E)
    Aufheben(u32),
    /// Waffe ausrüsten (0 = Startwaffe)
    Ausruesten(u8),
    /// Rüstung anlegen: Platz (0 Kopf, 1 Brust, 2 Füße) und Teil (0 = ablegen)
    RuestungAnlegen(u8, u8),
    /// Durch einen Durchgang eines Dungeons gehen (Index in `World::durchgaenge`)
    Durchgang(u16),
    /// Mit dem Händler auf dem Marktplatz handeln
    Handel(crate::handel::HandelBefehl),
    /// Einen Heiltrank trinken (Taste Q)
    Trinken,
}

/// So viele Heiltränke bekommt jeder Spieler, wenn er die Welt zum ersten Mal betritt.
pub const START_HEILTRAENKE: u32 = 100;
/// Ein Heiltrank heilt sofort diesen Anteil der Lebenspunkte …
pub const TRANK_HEILUNG: f32 = 0.4;
/// … und danach kann man so lange keinen trinken (Takte, 60 je Sekunde).
pub const TRANK_ABKLINGEN: u64 = 3 * 60;

/// Was im Inventar angelegt werden soll (Klick oder auf einen Ausrüstungsplatz gezogen).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anlegen {
    /// Waffe (0 = Startwaffe)
    Waffe(u8),
    /// Platz (0 Kopf, 1 Brust, 2 Füße) und Teil (0 = ablegen)
    Ruestung(u8, u8),
}

pub fn encode<T: Serialize>(message: &T) -> Vec<u8> {
    postcard::to_allocvec(message).expect("Nachricht lässt sich nicht kodieren")
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    match postcard::from_bytes(bytes) {
        Ok(message) => Some(message),
        Err(e) => {
            log::warn!("Ungültige Nachricht verworfen: {e}");
            None
        }
    }
}

pub const MAX_NAME_CHARS: usize = 16;
pub const MAX_CHAT_CHARS: usize = 160;

/// Macht aus einer Eingabe eine gültige Chatnachricht (gekürzt, ohne Steuerzeichen), leer = keine.
pub fn clean_chat(text: &str) -> Option<String> {
    let text: String = text.chars().filter(|c| !c.is_control()).take(MAX_CHAT_CHARS).collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Macht aus einer Eingabe einen gültigen Spielernamen (gekürzt, ohne Steuerzeichen).
pub fn clean_name(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
    let name = name.trim();
    if name.is_empty() { "Spieler".to_string() } else { name.to_string() }
}

/// Gesammelte Rohstoffe und Beute eines Spielers.
/// Neue Felder mit `serde(default)`, damit alte Spielstände lesbar bleiben.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    pub wood: u32,
    pub stone: u32,
    #[serde(default)]
    pub meat: u32,
    #[serde(default)]
    pub pelt: u32,
    #[serde(default)]
    pub wool: u32,
    #[serde(default)]
    pub ore: u32,
    /// Gold: Kopfgeld, Wellenbonus – die Währung für Türme und Fallen
    #[serde(default)]
    pub gold: u32,
    /// Runenfragmente (Beute aus den Lagern der Wildnis) und fertige Runensteine
    #[serde(default)]
    pub runenfragmente: u32,
    #[serde(default)]
    pub runensteine: u32,
    /// Erbeutete Waffen (Bit n = Waffe n aus `waffen.rs`) und die ausgerüstete (0 = Startwaffe)
    #[serde(default)]
    pub waffen: u32,
    #[serde(default)]
    pub waffe: u8,
    /// Erbeutete Rüstung (Bit n = Teil n aus `ruestung.rs`) und die getragene (Kopf, Brust, Füße; 0 = nichts)
    #[serde(default)]
    pub ruestungen: u32,
    #[serde(default)]
    pub ruestung: [u8; 3],
    /// Lehm (Lehmgrube, Lehmvorkommen) und magische Kristalle (Kristallturm)
    #[serde(default)]
    pub lehm: u32,
    #[serde(default)]
    pub kristalle: u32,
    /// Heiltränke (Taste Q)
    #[serde(default)]
    pub heiltraenke: u32,
}

/// Alles, was im Inventar liegen kann.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Gold,
    Wood,
    Stone,
    Ore,
    Meat,
    Pelt,
    Wool,
    Runenfragment,
    Runenstein,
    Lehm,
    Kristall,
    Heiltrank,
}

impl Item {
    pub const ALL: [Item; 12] = [Item::Gold, Item::Heiltrank, Item::Runenstein, Item::Runenfragment, Item::Kristall, Item::Wood, Item::Stone, Item::Lehm, Item::Ore, Item::Meat, Item::Pelt, Item::Wool];

    pub fn label(self) -> &'static str {
        match self {
            Item::Gold => "Gold",
            Item::Wood => "Holz",
            Item::Stone => "Stein",
            Item::Ore => "Eisenerz",
            Item::Meat => "Fleisch",
            Item::Pelt => "Fell",
            Item::Wool => "Wolle",
            Item::Runenfragment => "Runenfragment",
            Item::Runenstein => "Runenstein",
            Item::Lehm => "Lehm",
            Item::Kristall => "Kristall",
            Item::Heiltrank => "Heiltrank",
        }
    }

    /// Dateiname des Symbols in `game/assets/icons/` (gerendert von `art/icons/gegenstaende.py`).
    pub fn icon_file(self) -> &'static str {
        match self {
            Item::Gold => "gold",
            Item::Wood => "holz",
            Item::Stone => "stein",
            Item::Ore => "erz",
            Item::Meat => "fleisch",
            Item::Pelt => "fell",
            Item::Wool => "wolle",
            Item::Runenfragment => "runenfragment",
            Item::Runenstein => "runenstein",
            Item::Lehm => "lehm",
            Item::Kristall => "kristall",
            Item::Heiltrank => "heiltrank",
        }
    }

    /// Rohstoffe aus der Natur oder Beute von Tieren.
    pub fn is_loot(self) -> bool {
        matches!(self, Item::Meat | Item::Pelt | Item::Wool)
    }

    /// Art des Gegenstands (Zeile unter dem Namen im Tooltip).
    pub fn kind_line(self) -> &'static str {
        match self {
            Item::Gold => "Währung · für Türme und Fallen",
            Item::Wood | Item::Stone => "Rohstoff · Baumaterial",
            Item::Ore => "Rohstoff · Metall",
            Item::Meat => "Tierbeute · Nahrung",
            Item::Pelt | Item::Wool => "Tierbeute · Handwerksmaterial",
            Item::Runenfragment => "Magie · Beute aus der Wildnis",
            Item::Runenstein => "Magie · öffnet einen Siedlungsplatz",
            Item::Lehm => "Rohstoff · Baumaterial",
            Item::Kristall => "Magie · Rohstoff für Zaubertürme",
            Item::Heiltrank => "Trank · Q: trinken",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Item::Gold => "Kopfgeld für besiegte Truppen der Schattenfestung und Lohn für überstandene Wellen.",
            Item::Wood => "Von Bäumen geschlagen. Brennt gut und lässt sich verbauen.",
            Item::Stone => "Mit der Spitzhacke aus Steinvorkommen gebrochen. Hart und schwer.",
            Item::Ore => "Rostrotes Eisenerz aus dunklen Erzvorkommen. Lässt sich zu Eisen schmelzen.",
            Item::Meat => "Rohes Fleisch von erlegten Tieren.",
            Item::Pelt => "Warmes Fell von Hase, Fuchs, Reh, Wolf oder Bär.",
            Item::Wool => "Weiche Schafwolle.",
            Item::Runenfragment => "Splitter eines alten Schutzsteins, erbeutet in den Lagern der Wildnis. Der Runenbrunnen in der Burg vereint vier davon zu einem Runenstein.",
            Item::Runenstein => "Setze ihn in den Schutzstein am Ende einer Heerstraße (E): Dann gehört dir der Siedlungsplatz und du kannst deine Dorfhalle bauen.",
            Item::Lehm => "Heller, feuchter Lehm aus Lehmvorkommen in Niederungen und an der Küste. Für Rathaus, Burgfried, Kristallturm und Straßen.",
            Item::Kristall => "Ein magischer Kristall, den die Kristallmagier eines Kristallturms aus den leuchtenden Vorkommen lösen. Stärkt die Zaubertürme.",
            Item::Heiltrank => "Roter Heiltrank. Mit Q getrunken heilt er sofort 40 % deiner Lebenspunkte – von selbst heilen Wunden nicht.",
        }
    }
}

impl Inventory {
    pub fn add(&mut self, kind: crate::island::ResourceKind, amount: u32) {
        self.add_item(kind.item(), amount);
    }

    pub fn count(&self, item: Item) -> u32 {
        match item {
            Item::Gold => self.gold,
            Item::Wood => self.wood,
            Item::Stone => self.stone,
            Item::Ore => self.ore,
            Item::Meat => self.meat,
            Item::Pelt => self.pelt,
            Item::Wool => self.wool,
            Item::Runenfragment => self.runenfragmente,
            Item::Runenstein => self.runensteine,
            Item::Lehm => self.lehm,
            Item::Kristall => self.kristalle,
            Item::Heiltrank => self.heiltraenke,
        }
    }

    pub fn add_item(&mut self, item: Item, amount: u32) {
        let slot = match item {
            Item::Gold => &mut self.gold,
            Item::Wood => &mut self.wood,
            Item::Stone => &mut self.stone,
            Item::Ore => &mut self.ore,
            Item::Meat => &mut self.meat,
            Item::Pelt => &mut self.pelt,
            Item::Wool => &mut self.wool,
            Item::Runenfragment => &mut self.runenfragmente,
            Item::Runenstein => &mut self.runensteine,
            Item::Lehm => &mut self.lehm,
            Item::Kristall => &mut self.kristalle,
            Item::Heiltrank => &mut self.heiltraenke,
        };
        *slot += amount;
    }

    /// Nimmt `amount` Stück heraus, wenn so viele da sind.
    pub fn remove_item(&mut self, item: Item, amount: u32) -> bool {
        let slot = match item {
            Item::Gold => &mut self.gold,
            Item::Wood => &mut self.wood,
            Item::Stone => &mut self.stone,
            Item::Ore => &mut self.ore,
            Item::Meat => &mut self.meat,
            Item::Pelt => &mut self.pelt,
            Item::Wool => &mut self.wool,
            Item::Runenfragment => &mut self.runenfragmente,
            Item::Runenstein => &mut self.runensteine,
            Item::Lehm => &mut self.lehm,
            Item::Kristall => &mut self.kristalle,
            Item::Heiltrank => &mut self.heiltraenke,
        };
        if *slot < amount {
            return false;
        }
        *slot -= amount;
        true
    }

    /// Alle Gegenstände, die mindestens einmal da sind.
    pub fn items(&self) -> impl Iterator<Item = (Item, u32)> + '_ {
        Item::ALL.into_iter().map(|item| (item, self.count(item))).filter(|&(_, n)| n > 0)
    }

    pub fn total(&self) -> u32 {
        Item::ALL.into_iter().map(|item| self.count(item)).sum()
    }
}

/// Welche Figur ein Spieler spielt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacterClass {
    // Ritter und Barbar gibt es zurzeit nicht (nur noch für alte Spielstände lesbar).
    Knight,
    Barbarian,
    #[default]
    Mage,
    Rogue,
    Zwerg,
    Bogenschuetze,
}

impl CharacterClass {
    /// Wählbare Figuren.
    pub const ALL: [CharacterClass; 4] = [CharacterClass::Mage, CharacterClass::Zwerg, CharacterClass::Bogenschuetze, CharacterClass::Rogue];

    pub fn label(self) -> &'static str {
        match self {
            CharacterClass::Knight => "Ritter",
            CharacterClass::Barbarian => "Barbar",
            CharacterClass::Mage => "Magier",
            CharacterClass::Rogue => "Schurke",
            CharacterClass::Zwerg => "Zwerg",
            CharacterClass::Bogenschuetze => "Bogenschütze",
        }
    }

    /// Lebenspunkte bei voller Gesundheit.
    pub fn max_leben(self) -> u16 {
        match self {
            CharacterClass::Zwerg => 150,
            CharacterClass::Bogenschuetze => 110,
            CharacterClass::Rogue => 120,
            _ => 100,
        }
    }

    /// Kurze Beschreibung für die Figurenwahl.
    pub fn beschreibung(self) -> &'static str {
        match self {
            CharacterClass::Zwerg => "Zäh und stark im Nahkampf: Hammerschlag, Wurfhammer, Erdbeben – und der Ahnenhammer. 150 Leben.",
            CharacterClass::Bogenschuetze => "Flink und treffsicher auf große Entfernung: Pfeilschuss, Salve, Explosivpfeil – und der Pfeilregen. 110 Leben.",
            CharacterClass::Rogue => "Schnell und gerissen mit der Runenklinge: Klingenhieb, Wurfdolche, Rauchbombe – und die Schattenklingen. 120 Leben.",
            _ => "Kämpft aus der Ferne mit Magie: Arkangeschoss, Feuerball, Frostnova – und der Meteorsturm. 100 Leben.",
        }
    }
}

/// Was der Client beim Verbinden mitschickt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hello {
    pub name: String,
    pub class: CharacterClass,
}

impl Hello {
    /// Liest die Begrüßung; alte oder kaputte Daten werden als bloßer Name gedeutet.
    pub fn parse(bytes: &[u8]) -> Hello {
        postcard::from_bytes(bytes)
            .map(|hello: Hello| Hello { name: clean_name(&hello.name), ..hello })
            .unwrap_or_else(|_| Hello { name: clean_name(&String::from_utf8_lossy(bytes)), class: CharacterClass::default() })
    }
}
