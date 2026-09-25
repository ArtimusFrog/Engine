"""Totholz im Stil der Bäume (Mid-Poly): Baumstümpfe und umgestürzte Stämme.

Gebaut mit dem Baum-Baukasten aus baeume.py – dieselbe Rinde mit Rillen, weiche Normalen, ein
Material für alles. Dazu Schnittflächen mit Jahresringen, gesplitterte Bruchkanten, Moos auf den
Oberseiten und Baumpilze. Z oben, Ursprung am Boden.
"""

import math

from mathutils import Vector

from baeume import FLECK, OBEN, Baum, farbe

RINDE = "#5E4636"
RILLE = "#3E2E22"
HIRNHOLZ_HELL = "#D6AE78"
HIRNHOLZ_DUNKEL = "#A07A4C"
SPLITTER = "#B98E5E"
MOOS = "#557D30"
PILZ = "#D9B98A"


def _jahresringe(baum, mitte, achse, radius, ringe=4, schief=0.0):
    """Schnittfläche mit Jahresringen (Scheibe senkrecht zu `achse`), gefärbt Ring für Ring."""
    achse = achse.normalized()
    hilfe = Vector((0, 0, 1)) if abs(achse.z) < 0.9 else Vector((1, 0, 0))
    u = achse.cross(hilfe).normalized()
    v = achse.cross(u)
    ecken = 16
    hell, dunkel = farbe(HIRNHOLZ_HELL), farbe(HIRNHOLZ_DUNKEL)
    radien = [radius * (k + 1) / ringe for k in range(ringe)]

    def punkt(r, k):
        a = math.tau * k / ecken
        rr = r * (1 + 0.04 * math.sin(a * 3 + r * 7))
        p = mitte + (u * math.cos(a) + v * math.sin(a)) * rr
        return p + achse * (schief * math.cos(a) * rr)

    innen = [mitte + achse * 0.005] * ecken
    for i, r in enumerate(radien):
        aussen = [punkt(r, k) for k in range(ecken)]
        c = hell if i % 2 == 0 else dunkel
        c = c * (1.04 - 0.1 * i / ringe)
        for k in range(ecken):
            j = (k + 1) % ecken
            punkte = [innen[k], aussen[k], aussen[j]] if i == 0 else [innen[k], aussen[k], aussen[j], innen[j]]
            baum.flaeche(punkte, [FLECK] * len(punkte), [c] * len(punkte), [achse] * len(punkte))
        innen = aussen


def _splitter(baum, mitte, achse, radius, anzahl):
    """Gesplittertes Holz an einer Bruchkante: spitze, schräge Holzzacken."""
    r = baum.rng
    achse = achse.normalized()
    hilfe = Vector((0, 0, 1)) if abs(achse.z) < 0.9 else Vector((1, 0, 0))
    u = achse.cross(hilfe).normalized()
    v = achse.cross(u)
    for _ in range(anzahl):
        a = r.uniform(0, math.tau)
        d = r.uniform(0.1, 0.85) * radius
        fuss = mitte + (u * math.cos(a) + v * math.sin(a)) * d
        laenge = r.uniform(0.12, 0.45) * (1.2 - d / radius)
        kipp = (u * math.cos(a) + v * math.sin(a)) * r.uniform(0.0, 0.35)
        spitze = fuss + (achse + kipp).normalized() * laenge
        breite = r.uniform(0.04, 0.09)
        baum.rohr([fuss, fuss.lerp(spitze, 0.5), spitze], [breite, breite * 0.7, 0.01], 4, baum.rinde(SPLITTER), spitze=False, rauh=0.3)


def _weich(a, b, x):
    t = max(0.0, min(1.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)


def _brettwurzeln(baum, radius, anzahl, rinde, rille):
    """Kurze, kräftige Wurzeln, die vom Stammfuß in den Boden laufen."""
    r = baum.rng
    for k in range(anzahl):
        a = math.tau * k / anzahl + r.uniform(-0.25, 0.25)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        start = aussen * radius * 0.7 + Vector((0, 0, radius * 0.55))
        mitte = aussen * radius * 1.25 + Vector((0, 0, radius * 0.22))
        ende = aussen * radius * r.uniform(1.6, 1.95) + Vector((0, 0, -0.05))
        baum.rohr([start, mitte, ende], [radius * 0.42, radius * 0.3, radius * 0.1], 7, rinde, streifen=rille, rauh=0.6)


def _moos(moos, oben=0.55, boden=0.18):
    """Moos als weicher Übergang: auf nach oben gerichteten Flächen und am Boden."""
    def muster(rng, flaeche, c):
        n = flaeche.normal
        z = flaeche.calc_center_median().z
        anteil = max(_weich(0.35, 0.9, n.z) * oben, _weich(0.22, 0.0, z) * boden)
        return c.lerp(moos, anteil * rng.uniform(0.85, 1.0))
    return muster


def _baumpilze(baum, punkte_mit_richtung):
    """Halbrunde Baumpilze (Konsolen), flach am Holz."""
    r = baum.rng
    for p, aussen in punkte_mit_richtung:
        for i in range(r.randint(2, 3)):
            groesse = r.uniform(0.07, 0.13) * (1 - 0.25 * i)
            mitte = p + aussen * groesse * 0.4 + Vector((0, 0, -i * 0.11))
            baum.kugel(mitte, groesse, farbe(PILZ) * r.uniform(0.85, 1.05), stufen=1, platt=0.35, dunkel_unten=0.5)


def _fliegenpilz(baum, fuss):
    r = baum.rng
    hoehe = r.uniform(0.1, 0.16)
    baum.rohr([fuss, fuss + Vector((0, 0, hoehe))], [0.018, 0.015], 6, baum.rinde("#F1EDE4"), spitze=False, rauh=0.2)
    baum.kugel(fuss + Vector((0, 0, hoehe)), r.uniform(0.05, 0.07), farbe("#D8403A"), stufen=1, platt=0.55, dunkel_unten=0.6)


def baumstumpf(seed=13, gebrochen=False):
    """Baumstumpf: ausgestellter Fuß mit Wurzeln, Rinde mit Rillen, oben glatte Schnittfläche
    mit Jahresringen oder (gebrochen) höher und zersplittert; Moos, Baumpilze, Fliegenpilz."""
    baum = Baum("Baumstumpf", seed, hoehe=1.0)
    r = baum.rng
    rinde = baum.rinde(RINDE)
    rille = baum.rinde(RILLE)
    radius = r.uniform(0.38, 0.46)
    hoehe = r.uniform(0.9, 1.3) if gebrochen else r.uniform(0.45, 0.7)
    schritte = 6
    punkte = [Vector((r.uniform(-0.02, 0.02), r.uniform(-0.02, 0.02), hoehe * i / schritte)) for i in range(schritte + 1)]
    # Unten deutlich breiter (Wurzelansatz), oben fast gerade
    radien = [radius * (1.0 + 0.45 * max(0.0, 1 - i / 2.5) ** 2) for i in range(schritte + 1)]
    baum.rohr(punkte, radien, 16, rinde, spitze=False, streifen=rille, rauh=0.7)
    _brettwurzeln(baum, radius, 5, rinde, rille)
    oben = punkte[-1]
    if gebrochen:
        # Bruchkante: Deckel leicht vertieft, darauf Splitter
        _jahresringe(baum, oben - Vector((0, 0, 0.02)), OBEN, radius * 0.93, 3, schief=0.25)
        _splitter(baum, oben, OBEN, radius, 14)
    else:
        # Sägeschnitt: helle Jahresringe, knapp unter dem Rindenrand
        _jahresringe(baum, oben - Vector((0, 0, 0.01)), OBEN, radius * 0.95, 5, schief=0.06)
    # Pilze an der Seite, ein Fliegenpilz daneben
    a = r.uniform(0, math.tau)
    aussen = Vector((math.cos(a), math.sin(a), 0))
    _baumpilze(baum, [(aussen * radius * 1.02 + Vector((0, 0, hoehe * 0.55)), aussen)])
    if r.random() < 0.7:
        b = a + math.pi + r.uniform(-0.6, 0.6)
        _fliegenpilz(baum, Vector((math.cos(b), math.sin(b), 0)) * radius * 1.9)
    return baum.fertig(rinden_muster=_moos(farbe(MOOS), 0.4, 0.55))


def baumstamm(seed=12):
    """Umgestürzter Stamm: leicht gebogen, Rinde mit Rillen, Wurzelteller mit Erde und Wurzeln an
    einem Ende, gesplitterte Bruchstelle am anderen; Aststummel, Moos oben auf, Baumpilze."""
    baum = Baum("Baumstamm", seed, hoehe=1.0)
    r = baum.rng
    rinde = baum.rinde(RINDE)
    rille = baum.rinde(RILLE)
    moos = farbe(MOOS)
    laenge = r.uniform(3.6, 4.6)
    radius = r.uniform(0.3, 0.36)
    schritte = 9
    punkte = []
    for i in range(schritte + 1):
        t = i / schritte
        x = -laenge / 2 + laenge * t
        punkte.append(Vector((x, math.sin(t * math.pi) * 0.18, radius * (1.0 - 0.18 * t) + math.sin(t * math.pi) * 0.04)))
    radien = [radius * (1.15 - 0.3 * i / schritte) for i in range(schritte + 1)]
    baum.rohr(punkte, radien, 16, rinde, spitze=False, streifen=rille, rauh=0.7)
    achse = (punkte[1] - punkte[0]).normalized()
    # Wurzelteller: Erdscheibe mit Wurzeln, die nach außen und hinten ragen
    fuss = punkte[0]
    teller = fuss - achse * 0.06
    _jahresringe(baum, teller, -achse, radien[0] * 0.97, 4, schief=0.05)
    erde = baum.rinde("#4E3A2A")
    for k in range(9):
        w = math.tau * k / 9 + r.uniform(-0.2, 0.2)
        richtung = Vector((0, math.cos(w), math.sin(w)))
        start = fuss + richtung * radien[0] * 0.6
        ende = fuss + richtung * r.uniform(0.8, 1.2) - achse * r.uniform(0.2, 0.5)
        ende.z = max(ende.z, 0.02)
        mitte = start.lerp(ende, 0.5) - achse * 0.1
        baum.rohr([start, mitte, ende], [0.1, 0.07, 0.03], 6, erde if k % 3 == 0 else rinde, streifen=rille, rauh=0.8)
    # Bruchstelle am dünnen Ende
    _jahresringe(baum, punkte[-1], achse, radien[-1] * 0.9, 3, schief=0.3)
    _splitter(baum, punkte[-1], achse, radien[-1], 10)
    # Aststummel
    for i in range(3):
        j = r.randint(2, schritte - 2)
        w = r.uniform(0.3, 2.8)
        richtung = Vector((r.uniform(-0.3, 0.3), math.cos(w), math.sin(w))).normalized()
        start = punkte[j] + richtung * radien[j] * 0.8
        baum.rohr([start, start + richtung * r.uniform(0.25, 0.45)], [radien[j] * 0.28, radien[j] * 0.18], 7, rinde, spitze=False, rauh=0.5)
        _jahresringe(baum, start + richtung * 0.44, richtung, radien[j] * 0.17, 2)
    # Baumpilze an der Seite, Fliegenpilz im Schatten
    j = r.randint(3, schritte - 3)
    seite = Vector((0, -1, 0))
    _baumpilze(baum, [(punkte[j] + seite * radien[j] * 1.02 + Vector((0, 0, 0.05)), seite)])
    _fliegenpilz(baum, punkte[j + 1] + Vector((0, 0.1 + radien[j + 1] * 1.4, -punkte[j + 1].z)))

    return baum.fertig(rinden_muster=_moos(moos, 0.5, 0.0))
