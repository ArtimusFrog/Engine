# Konzept: Vor der Siedlung – Wildnis, Runenfragmente, Runenstein

Bevor ein Spieler am Ende einer Heerstraße siedeln kann, braucht er einen **Runenstein**. Den gibt es
nicht geschenkt: Man kämpft sich durch die Lager der Wildnis, sammelt vier **Runenfragmente**, vereint
sie am **Runenbrunnen** in der Burg und setzt den Stein in den **Schutzstein** eines freien
Siedlungsplatzes. Erst dann lässt sich dort die Dorfhalle bauen. (Wann die Wellen der Festung
beginnen, wird später gesondert geregelt – der Runenstein startet sie nicht.)

Code: `wildnis.rs` (Lager und Bewohner), `faehigkeiten.rs` (Fähigkeiten der Figuren),
`server.rs` (`cast`, `land_hits`, `spieler_schaden`, `wildnis_takt`, `runen`), Modelle in
`art/lib/runen.py` und `art/lib/zwerg.py`.

## Lager der Wildnis

- Neun Lager, fest verteilt (auf allen Rechnern gleich): ebener Boden an Land, weit weg von
  Heerstraßen, Siedlungsplätzen, Startlager, Sehenswürdigkeiten, Burg und Festung – und voneinander.
  Jedes hat Feuer, Zelte, Kisten und seinen Namen auf der Karte.
- **Gefahr steigt zur Inselmitte** (zur Schattenfestung) hin:

| Gefahr | Lager | Bewohner | Anführer |
|---|---|---|---|
| 1 (außen) | Wolfsrudel | 3 Schattenwölfe | großer Wolf |
| 1 | Knochenlager | 2 Skelette, Bogenschütze | großes Skelett |
| 2 | Späherlager | 2 Bogenschützen, Pikenier, Meuchler | Dunkler Ritter |
| 2 | Hexenzirkel | Dunkelmagier, 2 Skelette, Gespenst | Dunkelmagier |
| 3 (innen) | Kriegslager | 2 Ritter, Pikenier, Bogenschütze, Dunkelmagier | Steingolem |

- Leben ×1 / ×1,7 / ×2,6 je Gefahr, Anführer ×2,5. Schlagkraft 45 / 65 / 85 % (Anführer ×1,4).
- **Verhalten:** Wer näher als 15–21 m kommt (oder aus der Ferne auf sie schießt), wird bemerkt –
  dann greift das **ganze Lager** an. Verfolgt wird bis zur Leine (38–50 m vom Lager), danach kehren
  die Bewohner um und heilen sich. Fernkämpfer schießen aus bis zu 16 m.
- Ein leeres Lager wird nach **150 s** neu besetzt, aber nur, wenn niemand näher als 60 m ist.

## Beute

- Jeder Besiegte: Gold (4 / 7 / 11, Anführer ×4).
- **Runenfragment** mit etwas Glück (bekommt, wer zuletzt getroffen hat):

| Gefahr | normaler Bewohner | Anführer |
|---|---|---|
| 1 | 8 % | 40 % |
| 2 | 14 % | 60 % |
| 3 | 20 % | 85 % |

  Ein ganzes Lager bringt im Mittel etwa 0,6 (Wolfsrudel) bis 1,8 (Kriegslager) Fragmente – für
  einen Runenstein räumt man also drei bis sechs Lager. Wer sich an die gefährlichen Lager traut,
  ist schneller.

## Runenbrunnen und Runenstein

- **Runenbrunnen:** der Brunnen in der Mitte des Burghofs, umgeben von vier Runensäulen und einem
  leuchtenden Runenkreis, darüber schwebt ein großer Runenkristall. Mit vier Fragmenten: **E** →
  ein Runenstein.
- **Schutzstein:** am Ende jeder Heerstraße. Mit einem Runenstein: **E** → der Siedlungsplatz gehört
  diesem Spieler (ein Platz je Spieler, ein Spieler je Platz). Über dem Schutzstein schwebt nun der
  leuchtende Runenstein, bis die Dorfhalle steht.
- Die Dorfhalle lässt sich nur auf dem eigenen, aktivierten Siedlungsplatz bauen. Aus älteren
  Spielständen: Wer dort schon eine Dorfhalle hat, dem gehört der Platz.

## Spielfiguren und Fähigkeiten

Zwei Figuren (Hauptmenü oder Einstellungen): **Magier** (100 Leben) und **Zwerg** (150 Leben).
Auswahlleiste: 1 Spitzhacke, 2 Axt, **3–5 die drei Fähigkeiten** der Figur. Linksklick setzt sie
ein (den Standardangriff auf Taste 3 kann man gedrückt halten). Die Abklingzeit steht auf dem Platz.

| Figur | Taste 3 | Taste 4 | Taste 5 |
|---|---|---|---|
| Magier | Arkangeschoss: 22 Schaden, ignoriert Rüstung, 0,7 s | Feuerball: 34 Schaden im Umkreis von 3,8 m, Brand 8/s für 4 s, 5 s | Frostnova: 20 Schaden um sich (7,5 m), verlangsamt um 60 %, 12 s |
| Zwerg | Hammerschlag: 36 Schaden vor sich (3,4 m, Kegel), 0,8 s | Wurfhammer: 28 Schaden, betäubt 1,2 s, 5 s | Erdbeben: 32 Schaden um sich (6,5 m), betäubt 1,6 s, 12 s |

Die Fähigkeiten treffen Tiere, Lagerbewohner und die Truppen der Festung.

## Leben der Spieler

- Lagerbewohner und Truppen der Festung treffen Spieler (die Truppen mit halbem Schaden).
- Nach 8 s ohne Treffer heilt man sich (5 % je Sekunde).
- Wer fällt, erwacht mit vollen Leben an seinem Startpunkt (vor der eigenen Dorfhalle, sonst im
  Startlager). Nichts geht verloren.

## Testen

Admin-Panel (X): „+4 Runenfragmente“, „Lager neu besetzen“. Startoptionen: `--figur zwerg`,
`--demo-lager N` (vor ein Lager), `--demo-angriff N` (dabei Fähigkeit N einsetzen),
`--demo-brunnen` (an den Runenbrunnen, mit vier Fragmenten).

## Später

- Eigene Modelle für die Lager (düstere Zelte, Banner der Festung, Käfige)
- Mehr Figuren, Fähigkeiten aufwerten, Ausrüstung
- Seltene, besonders starke Lager (Weltbosse) mit sicherem Fragment
