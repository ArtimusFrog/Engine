# Herkunft der Klänge

Alle Aufnahmen in `game/assets/sounds/` (außer `menue.ogg`) stammen aus frei nutzbaren Paketen unter
**CC0** (gemeinfrei, keine Namensnennung nötig). Sie wurden normalisiert (Spitze 0,9), gekürzt, auf
mono/44,1 kHz gebracht und als 16-Bit-WAV gespeichert. Mehrere Dateien `<name>_1.wav`, `<name>_2.wav` …
sind Varianten; das Spiel wählt beim Abspielen zufällig eine. Fehlt eine Datei, erzeugt das Spiel den
Klang selbst (`game/src/sounds.rs`).

| Klang im Spiel | Quelle | Paket | Lizenz |
|---|---|---|---|
| schwung_1–4, wurf_1–3 | swish-3/4/7/9, swish-1/5/8 | [Swishes Sound Pack](https://opengameart.org/content/swishes-sound-pack) | CC0 |
| hammer_1–3 | impactPlate_heavy_001/003/004 | [Kenney Impact Sounds](https://kenney.nl/assets/impact-sounds) | CC0 |
| treffer_1–4 | impactPunch_heavy_000–003 | Kenney Impact Sounds | CC0 |
| pfeiltreffer_1–3 | impactPunch_medium_000/001/003 | Kenney Impact Sounds | CC0 |
| fangen_1–2 | impactMetal_medium_000/003 | Kenney Impact Sounds | CC0 |
| hacken_2–4, stein_1–3, erz_1–3 | impactWood_heavy, impactMining, impactMetal_light | Kenney Impact Sounds | CC0 |
| eisbruch_2–3 | impactGlass_heavy_000/003 | Kenney Impact Sounds | CC0 |
| hacken_1, klinge_1–2 | chop, knifeSlice, knifeSlice2 | [Kenney RPG Audio](https://kenney.nl/assets/rpg-audio) | CC0 |
| eisbruch_1, frostnova | ice.wav, coldsnap.wav | [Ice spells](https://opengameart.org/content/ice-spells) | CC0 |
| explosion | jm-fx-fireball-01 (Julien Matthey) | [Fireball](https://opengameart.org/content/fireball-1) | CC0 |
| blitz_1–2 | spark.wav, continuousspark.wav (Brian MacIntosh) | [Electricity Sound Effects](https://opengameart.org/content/electricity-sound-effects-0) | CC0 |
| zauber_1–2 | magical_3, magical_1 (JaggedStone) | [Magic Spell SFX](https://opengameart.org/content/magic-spell-sfx) | CC0 |
| donner | sfx100v2_thunder_01 | [100 CC0 SFX #2](https://opengameart.org/content/100-cc0-sfx-2) | CC0 |

Geprüft und nicht verwendet: „Sound Effects Pack 2“ (Retro-Arcade-Klänge, passen nicht zum Stil),
„Spell Sounds Starter Pack“ und „Bow & Arrow Shot“ (CC-BY-SA/GPL). Bogenschüsse bleiben selbst erzeugt.
