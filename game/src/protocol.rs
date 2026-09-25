//! Was zwischen Server und Clients über das Netzwerk geht.

use glam::{Quat, Vec2, Vec3};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 7777;

/// Bei jeder inkompatiblen Änderung an diesen Nachrichten hochzählen. Server und Client
/// mit unterschiedlicher ID können sich nicht verbinden.
pub const PROTOCOL_ID: u64 = 0x4A4E_0000_0000_000B;

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
    /// Ein Spieler wirkt einen Zauber: ein Geschoss fliegt von `origin` nach `target`.
    /// `hit`: trifft es dort ein Tier (dann folgt `AnimalHit`)?
    SpellCast { by: PlayerId, origin: Vec3, target: Vec3, hit: bool },
    /// Ein Tier wurde getroffen und hat noch `health` Leben (0 = erlegt).
    AnimalHit { id: u16, health: u8, by: PlayerId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Die letzten paar Eingaben (älteste zuerst). Mehrfach senden gleicht Paketverlust aus.
    Inputs(Vec<PlayerInput>),
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
}

/// Alles, was im Inventar liegen kann.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Wood,
    Stone,
    Meat,
    Pelt,
    Wool,
}

impl Item {
    pub const ALL: [Item; 5] = [Item::Wood, Item::Stone, Item::Meat, Item::Pelt, Item::Wool];

    pub fn label(self) -> &'static str {
        match self {
            Item::Wood => "Holz",
            Item::Stone => "Stein",
            Item::Meat => "Fleisch",
            Item::Pelt => "Fell",
            Item::Wool => "Wolle",
        }
    }

    /// Dateiname des Symbols in `game/assets/icons/` (gerendert von `art/icons/gegenstaende.py`).
    pub fn icon_file(self) -> &'static str {
        match self {
            Item::Wood => "holz",
            Item::Stone => "stein",
            Item::Meat => "fleisch",
            Item::Pelt => "fell",
            Item::Wool => "wolle",
        }
    }

    /// Rohstoffe aus der Natur oder Beute von Tieren.
    pub fn is_loot(self) -> bool {
        matches!(self, Item::Meat | Item::Pelt | Item::Wool)
    }

    /// Art des Gegenstands (Zeile unter dem Namen im Tooltip).
    pub fn kind_line(self) -> &'static str {
        match self {
            Item::Wood | Item::Stone => "Rohstoff · Baumaterial",
            Item::Meat => "Tierbeute · Nahrung",
            Item::Pelt | Item::Wool => "Tierbeute · Handwerksmaterial",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Item::Wood => "Von Bäumen geschlagen. Brennt gut und lässt sich verbauen.",
            Item::Stone => "Aus Felsen gebrochen. Hart und schwer.",
            Item::Meat => "Rohes Fleisch von erlegten Tieren.",
            Item::Pelt => "Warmes Fell von Hase, Fuchs, Reh, Wolf oder Bär.",
            Item::Wool => "Weiche Schafwolle.",
        }
    }
}

impl Inventory {
    pub fn add(&mut self, kind: crate::island::ResourceKind, amount: u32) {
        match kind {
            crate::island::ResourceKind::Wood => self.wood += amount,
            crate::island::ResourceKind::Stone => self.stone += amount,
        }
    }

    pub fn count(&self, item: Item) -> u32 {
        match item {
            Item::Wood => self.wood,
            Item::Stone => self.stone,
            Item::Meat => self.meat,
            Item::Pelt => self.pelt,
            Item::Wool => self.wool,
        }
    }

    pub fn add_item(&mut self, item: Item, amount: u32) {
        let slot = match item {
            Item::Wood => &mut self.wood,
            Item::Stone => &mut self.stone,
            Item::Meat => &mut self.meat,
            Item::Pelt => &mut self.pelt,
            Item::Wool => &mut self.wool,
        };
        *slot += amount;
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
    // Ritter, Barbar und Schurkin gibt es zurzeit nicht (nur noch für alte Spielstände lesbar).
    Knight,
    Barbarian,
    #[default]
    Mage,
    Rogue,
}

impl CharacterClass {
    /// Wählbare Figuren.
    pub const ALL: [CharacterClass; 1] = [CharacterClass::Mage];

    pub fn label(self) -> &'static str {
        match self {
            CharacterClass::Knight => "Ritter",
            CharacterClass::Barbarian => "Barbar",
            CharacterClass::Mage => "Magier",
            CharacterClass::Rogue => "Schurkin",
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
