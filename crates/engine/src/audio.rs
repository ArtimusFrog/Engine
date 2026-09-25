//! Ton: Geräusche im Raum, Umgebungsschleifen und Musik.
//!
//! - [`Audio::play`] spielt ein Geräusch einmal ab – mit Position wird es mit der Entfernung
//!   leiser und kommt aus der richtigen Richtung (links/rechts).
//! - [`Audio::start_loop`] startet eine Endlosschleife (Wind, Wellen, Musik), deren
//!   Lautstärke und Position man jedes Bild ändern kann.
//! - Drei Regler: Effekte, Umgebung, Musik – plus Gesamtlautstärke.
//!
//! Ohne Soundkarte (Server, Tests) ist alles stumm, die Aufrufe schaden aber nicht.
//! Klänge kommen aus Dateien (.ogg, .wav, .flac) oder aus [`synth`] (selbst erzeugt).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use glam::Vec3;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Panning, Tween};

/// Ab dieser Entfernung (Meter) wird ein Geräusch leiser.
const REFERENCE_DISTANCE: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(u32);

/// Ein dekodierter Klang, der noch nicht eingetragen ist (siehe [`Audio::decode_file`]).
pub struct DecodedSound(StaticSoundData);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoopId(u32);

/// Mischpult-Kanal mit eigenem Lautstärkeregler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bus {
    Effects,
    Ambient,
    Music,
}

/// Wie ein Geräusch abgespielt wird.
#[derive(Clone, Copy, Debug)]
pub struct Play {
    /// Ort in der Welt; `None` = direkt im Kopf (Menüklicks, eigene Schritte).
    pub at: Option<Vec3>,
    /// 1 = unverändert.
    pub volume: f32,
    /// Tonhöhe/Tempo, 1 = unverändert (kleine Zufallsschwankungen klingen natürlicher).
    pub pitch: f32,
    /// Bis zu dieser Entfernung (Meter) hörbar.
    pub range: f32,
    pub bus: Bus,
}

impl Default for Play {
    fn default() -> Self {
        Play { at: None, volume: 1.0, pitch: 1.0, range: 50.0, bus: Bus::Effects }
    }
}

/// Rohe Tondaten (Stereo, -1..1).
#[derive(Clone, Debug, Default)]
pub struct SoundBuffer {
    pub sample_rate: u32,
    pub frames: Vec<[f32; 2]>,
}

struct Mixer {
    manager: AudioManager<DefaultBackend>,
    tracks: HashMap<Bus, TrackHandle>,
}

struct LoopSlot {
    handle: StaticSoundHandle,
    at: Option<Vec3>,
    volume: f32,
    range: f32,
}

pub struct Audio {
    mixer: Option<Mixer>,
    sounds: Vec<StaticSoundData>,
    named: HashMap<String, SoundId>,
    loops: HashMap<u32, LoopSlot>,
    next_loop: u32,
    listener: Vec3,
    listener_right: Vec3,
    volumes: HashMap<Bus, f32>,
    master: f32,
}

/// Lautstärke 0..1 in Dezibel für kira.
fn decibels(linear: f32) -> Decibels {
    if linear <= 0.001 { Decibels::SILENCE } else { Decibels((20.0 * linear.log10()).max(-60.0)) }
}

impl Audio {
    /// Stumm (ohne Soundkarte).
    pub(crate) fn disabled() -> Audio {
        Audio {
            mixer: None,
            sounds: Vec::new(),
            named: HashMap::new(),
            loops: HashMap::new(),
            next_loop: 0,
            listener: Vec3::ZERO,
            listener_right: Vec3::X,
            volumes: HashMap::new(),
            master: 1.0,
        }
    }

    /// Öffnet die Soundkarte; klappt das nicht, bleibt das Spiel stumm.
    pub(crate) fn open() -> Audio {
        let mut audio = Audio::disabled();
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(mut manager) => {
                let mut tracks = HashMap::new();
                for bus in [Bus::Effects, Bus::Ambient, Bus::Music] {
                    match manager.add_sub_track(TrackBuilder::new()) {
                        Ok(track) => {
                            tracks.insert(bus, track);
                        }
                        Err(e) => log::warn!("Tonspur {bus:?} fehlt: {e}"),
                    }
                }
                audio.mixer = Some(Mixer { manager, tracks });
            }
            Err(e) => log::warn!("Keine Tonausgabe ({e}) – das Spiel bleibt stumm"),
        }
        audio
    }

    pub fn is_enabled(&self) -> bool {
        self.mixer.is_some()
    }

    /// Legt einen Klang aus Rohdaten an (ein Name wird nur einmal angelegt).
    pub fn add_sound(&mut self, name: &str, build: impl FnOnce() -> SoundBuffer) -> SoundId {
        if let Some(&id) = self.named.get(name) {
            return id;
        }
        let buffer = build();
        let frames: Arc<[Frame]> = buffer.frames.iter().map(|&[left, right]| Frame { left, right }).collect();
        let data = StaticSoundData { sample_rate: buffer.sample_rate, frames, settings: StaticSoundSettings::default(), slice: None };
        self.insert(name, data)
    }

    /// Dekodiert eine Datei (.ogg, .wav, .flac), ohne sie schon einzutragen – das geht auch in
    /// einem anderen Thread (z. B. lange Musikstücke, damit der Start nicht wartet).
    pub fn decode_file(path: &Path) -> Result<DecodedSound, String> {
        StaticSoundData::from_file(path).map(DecodedSound).map_err(|e| format!("{} nicht lesbar: {e}", path.display()))
    }

    /// Trägt einen vorher dekodierten Klang ein.
    pub fn add_decoded(&mut self, name: &str, sound: DecodedSound) -> SoundId {
        if let Some(&id) = self.named.get(name) {
            return id;
        }
        self.insert(name, sound.0)
    }

    /// Lädt einen Klang aus einer Datei (.ogg, .wav, .flac).
    pub fn load_file(&mut self, name: &str, path: &Path) -> Result<SoundId, String> {
        if let Some(&id) = self.named.get(name) {
            return Ok(id);
        }
        let data = StaticSoundData::from_file(path).map_err(|e| format!("{} nicht lesbar: {e}", path.display()))?;
        Ok(self.insert(name, data))
    }

    fn insert(&mut self, name: &str, data: StaticSoundData) -> SoundId {
        let id = SoundId(self.sounds.len() as u32);
        self.sounds.push(data);
        self.named.insert(name.to_string(), id);
        id
    }

    pub fn sound(&self, name: &str) -> Option<SoundId> {
        self.named.get(name).copied()
    }

    /// Lautstärke eines Kanals (0..1).
    pub fn set_volume(&mut self, bus: Bus, volume: f32) {
        self.volumes.insert(bus, volume.clamp(0.0, 1.0));
        if let Some(track) = self.mixer.as_mut().and_then(|m| m.tracks.get_mut(&bus)) {
            track.set_volume(decibels(volume), Tween::default());
        }
    }

    /// Gesamtlautstärke (0..1).
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master = volume.clamp(0.0, 1.0);
        if let Some(mixer) = &mut self.mixer {
            mixer.manager.main_track().set_volume(decibels(self.master), Tween::default());
        }
    }

    /// Lautstärke (0..1) und Links/Rechts (-1..1) eines Geräuschs an `at` für den Zuhörer.
    fn spatial(&self, at: Option<Vec3>, range: f32) -> (f32, f32) {
        let Some(at) = at else { return (1.0, 0.0) };
        let offset = at - self.listener;
        let distance = offset.length();
        if distance >= range {
            return (0.0, 0.0);
        }
        // Natürliches Abklingen, zum Rand der Reichweite hin sanft auf null.
        let falloff = (REFERENCE_DISTANCE / distance.max(REFERENCE_DISTANCE)) * (1.0 - distance / range).min(1.0);
        let pan = if distance > 0.5 { offset.normalize().dot(self.listener_right) * 0.75 } else { 0.0 };
        (falloff.clamp(0.0, 1.0), pan)
    }

    /// Spielt einen Klang einmal ab.
    pub fn play(&mut self, sound: SoundId, play: Play) {
        let (gain, pan) = self.spatial(play.at, play.range);
        let gain = gain * play.volume;
        if gain < 0.01 {
            return;
        }
        let Some(data) = self.sounds.get(sound.0 as usize) else { return };
        let Some(mixer) = &mut self.mixer else { return };
        let data = data.with_settings(StaticSoundSettings::new().volume(decibels(gain)).panning(Panning(pan)).playback_rate(play.pitch as f64));
        let result = match mixer.tracks.get_mut(&play.bus) {
            Some(track) => track.play(data).map(|_| ()),
            None => mixer.manager.play(data).map(|_| ()),
        };
        if let Err(e) = result {
            log::debug!("Klang nicht abspielbar: {e}");
        }
    }

    /// Ist die Schleife eingetragen (also hörbar, sobald sie lauter gestellt wird)?
    pub fn has_loop(&self, id: LoopId) -> bool {
        self.loops.contains_key(&id.0)
    }

    /// Startet eine Endlosschleife (anfangs stumm, Lautstärke mit [`set_loop`](Self::set_loop)).
    pub fn start_loop(&mut self, sound: SoundId, bus: Bus) -> LoopId {
        let id = LoopId(self.next_loop);
        self.next_loop += 1;
        let (Some(data), Some(mixer)) = (self.sounds.get(sound.0 as usize), &mut self.mixer) else { return id };
        let data = data.loop_region(..).volume(Decibels::SILENCE);
        let handle = match mixer.tracks.get_mut(&bus) {
            Some(track) => track.play(data),
            None => mixer.manager.play(data),
        };
        match handle {
            Ok(handle) => {
                self.loops.insert(id.0, LoopSlot { handle, at: None, volume: 0.0, range: 80.0 });
            }
            Err(e) => log::debug!("Schleife nicht abspielbar: {e}"),
        }
        id
    }

    /// Lautstärke (0..1) und Ort einer Schleife; wird weich angepasst.
    pub fn set_loop(&mut self, id: LoopId, volume: f32, at: Option<Vec3>, range: f32) {
        if let Some(slot) = self.loops.get_mut(&id.0) {
            slot.volume = volume;
            slot.at = at;
            slot.range = range;
        }
    }

    pub fn stop_loop(&mut self, id: LoopId) {
        if let Some(mut slot) = self.loops.remove(&id.0) {
            slot.handle.stop(Tween { duration: Duration::from_millis(400), ..Default::default() });
        }
    }

    /// Einmal pro Bild (macht die Engine): Zuhörer an die Kamera, Schleifen nachführen.
    pub(crate) fn update(&mut self, listener: Vec3, right: Vec3) {
        self.listener = listener;
        self.listener_right = right;
        let targets: Vec<(u32, f32, f32)> = self
            .loops
            .iter()
            .map(|(&id, slot)| {
                let (gain, pan) = self.spatial(slot.at, slot.range);
                (id, gain * slot.volume, pan)
            })
            .collect();
        let tween = Tween { duration: Duration::from_millis(120), ..Default::default() };
        for (id, gain, pan) in targets {
            if let Some(slot) = self.loops.get_mut(&id) {
                slot.handle.set_volume(decibels(gain), tween);
                slot.handle.set_panning(Panning(pan), tween);
            }
        }
    }
}

/// Bausteine, um Klänge selbst zu erzeugen, solange es keine Aufnahmen gibt.
pub mod synth {
    use super::SoundBuffer;
    use crate::noise::Rng;

    pub const SAMPLE_RATE: u32 = 44_100;

    /// Mono-Klang aus einer Funktion der Zeit (Sekunden) → Stereo-Puffer.
    pub fn render(seconds: f32, mut f: impl FnMut(f32) -> f32) -> SoundBuffer {
        let count = (seconds * SAMPLE_RATE as f32) as usize;
        let frames = (0..count)
            .map(|i| {
                let s = f(i as f32 / SAMPLE_RATE as f32);
                [s, s]
            })
            .collect();
        SoundBuffer { sample_rate: SAMPLE_RATE, frames }
    }

    /// Bringt den lautesten Ausschlag auf `peak` (z. B. 0.9) – alle Klänge gleich laut,
    /// ohne Übersteuern. Wie laut sie im Spiel sind, regelt dann [`Play::volume`](super::Play).
    pub fn normalize(buffer: &mut SoundBuffer, peak: f32) {
        let loudest = buffer.frames.iter().flat_map(|f| [f[0].abs(), f[1].abs()]).fold(0.0, f32::max);
        if loudest > 1e-6 {
            let gain = peak / loudest;
            for frame in &mut buffer.frames {
                frame[0] *= gain;
                frame[1] *= gain;
            }
        }
    }

    /// Weißes Rauschen mit festem Startwert.
    pub struct Noise(Rng);

    impl Noise {
        pub fn new(seed: u64) -> Self {
            Noise(Rng::new(seed))
        }

        pub fn next(&mut self) -> f32 {
            self.0.next_f32() * 2.0 - 1.0
        }
    }

    /// Einfacher Tiefpass: `amount` 0..1, klein = dumpfer.
    #[derive(Default)]
    pub struct LowPass(f32);

    impl LowPass {
        pub fn next(&mut self, input: f32, amount: f32) -> f32 {
            self.0 += (input - self.0) * amount.clamp(0.0, 1.0);
            self.0
        }
    }

    /// Hüllkurve: schnell rein (`attack` s), dann exponentiell abklingen (`decay` s).
    pub fn envelope(t: f32, attack: f32, decay: f32) -> f32 {
        if t < attack { t / attack } else { (-(t - attack) / decay).exp() }
    }

    /// Sinus mit Frequenz `hz` zur Zeit `t`.
    pub fn sine(t: f32, hz: f32) -> f32 {
        (t * hz * std::f32::consts::TAU).sin()
    }

    /// Macht das Ende weich mit dem Anfang überlappend, damit eine Schleife nicht knackt.
    pub fn make_loopable(buffer: &mut SoundBuffer, fade_seconds: f32) {
        let fade = ((fade_seconds * buffer.sample_rate as f32) as usize).min(buffer.frames.len() / 2);
        let len = buffer.frames.len();
        for i in 0..fade {
            let w = i as f32 / fade as f32;
            let tail = buffer.frames[len - fade + i];
            let head = &mut buffer.frames[i];
            head[0] = head[0] * w + tail[0] * (1.0 - w);
            head[1] = head[1] * w + tail[1] * (1.0 - w);
        }
        buffer.frames.truncate(len - fade);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raumklang_leiser_mit_entfernung_und_richtig_verteilt() {
        let mut audio = Audio::disabled();
        audio.update(Vec3::ZERO, Vec3::X);
        let (near, _) = audio.spatial(Some(Vec3::new(0.0, 0.0, -2.0)), 50.0);
        let (far, _) = audio.spatial(Some(Vec3::new(0.0, 0.0, -30.0)), 50.0);
        let (gone, _) = audio.spatial(Some(Vec3::new(0.0, 0.0, -60.0)), 50.0);
        assert!(near > far && far > 0.0 && gone == 0.0, "{near} {far} {gone}");
        let (_, right) = audio.spatial(Some(Vec3::new(10.0, 0.0, 0.0)), 50.0);
        let (_, left) = audio.spatial(Some(Vec3::new(-10.0, 0.0, 0.0)), 50.0);
        assert!(right > 0.5 && left < -0.5);
    }

    #[test]
    fn stumm_ohne_soundkarte_und_klaenge_werden_nur_einmal_angelegt() {
        let mut audio = Audio::disabled();
        let a = audio.add_sound("klick", || synth::render(0.1, |t| synth::sine(t, 440.0)));
        let b = audio.add_sound("klick", || panic!("darf nicht erneut gebaut werden"));
        assert_eq!(a, b);
        audio.play(a, Play::default());
        let l = audio.start_loop(a, Bus::Ambient);
        audio.set_loop(l, 1.0, None, 10.0);
        audio.update(Vec3::ZERO, Vec3::X);
    }

    #[test]
    fn schleife_ohne_knacksen() {
        let mut buffer = synth::render(1.0, |t| synth::sine(t, 3.0));
        let before = buffer.frames.len();
        synth::make_loopable(&mut buffer, 0.1);
        assert!(buffer.frames.len() < before);
        let (first, last) = (buffer.frames[0][0], buffer.frames[buffer.frames.len() - 1][0]);
        assert!((first - last).abs() < 0.05, "Sprung an der Nahtstelle: {first} → {last}");
    }
}
