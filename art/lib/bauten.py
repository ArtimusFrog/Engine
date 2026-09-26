"""Gebäude, die Spieler selbst errichten (Baumenü mit B): Holzfäller, Steinbruch und Erzmine.

Mid-Poly wie das Startlager (Bausteine aus `lager.py`), hell und freundlich. Der Ursprung liegt
auf dem Boden in der Mitte, die Vorderseite (Tür, Eingang) zeigt nach −Y (im Spiel nach +Z).
Ein Sockel reicht `SOCKEL` Meter unter den Boden, damit die Gebäude auch an leichten Hängen
nicht schweben. Das Spiel lässt die Gebäude beim Bauen von unten nach oben entstehen –
deshalb ist alles aus einzelnen, übereinander liegenden Teilen gebaut.
"""

import math
import random

import bmesh
from mathutils import Matrix, Vector

from lager import (brett_bm, einfarbig, holzfarbe, leucht_material, objekt, setzen, stamm_bm, stammfarbe,
                   stein_bm)
from vorkommen import _brocken_bm, _material, _setzen, _splitter, _steinfarbe, farbe
from werkstatt import vereinen

SOCKEL = 0.8


def _fertig(name, teile, leuchtend=None, leucht=("#FFB050", 2.2)):
    """Wie `lager.fertig`, aber der Ursprung bleibt auf Bodenhöhe (der Sockel ragt darunter)."""
    obj = vereinen(name, teile)
    _material(obj)
    if leuchtend:
        glut = vereinen(name + "Licht", leuchtend)
        glut.data.materials.clear()
        glut.data.materials.append(leucht_material(name + "Leuchten", *leucht))
        for poly in glut.data.polygons:
            poly.material_index = 0
        obj = vereinen(name, [obj, glut])
    dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"MODELL {name}: {dreiecke} Dreiecke")
    return obj


def _brett(teile, name, farbe_von, masse, ort, drehung=(0, 0, 0), fase=0.012):
    bm = brett_bm(*masse, fase=fase)
    setzen(bm, (0, 0, 0), drehung)
    setzen(bm, ort)
    teile.append(objekt(name, bm, farbe_von))


def _stamm(teile, name, farbe_von, radius, laenge, ort, drehung=(0, 0, 0), ecken=10, seed=1, knorrig=0.05):
    """Stamm mit der Mitte bei `ort` (lager.stamm_bm beginnt am Ursprung)."""
    bm = stamm_bm(radius, laenge, ecken=ecken, seed=seed, knorrig=knorrig)
    setzen(bm, (-laenge / 2, 0, 0))
    setzen(bm, (0, 0, 0), drehung)
    setzen(bm, ort)
    teile.append(objekt(name, bm, farbe_von))


def _platte(teile, name, farbe_von, radius_x, radius_y, oben, zufall, ecken=18):
    """Unregelmäßige, flache Bodenplatte (Kies, festgetretene Erde) bis unter den Boden."""
    bm = bmesh.new()
    unten_ring, oben_ring = [], []
    for k in range(ecken):
        w = math.tau * k / ecken
        r = zufall.uniform(0.9, 1.05)
        x, y = math.cos(w) * radius_x * r, math.sin(w) * radius_y * r
        oben_ring.append(bm.verts.new((x, y, oben)))
        unten_ring.append(bm.verts.new((x * 1.08, y * 1.08, -SOCKEL)))
    for k in range(ecken):
        j = (k + 1) % ecken
        bm.faces.new((unten_ring[k], unten_ring[j], oben_ring[j], oben_ring[k]))
    bm.faces.new(oben_ring)
    bm.faces.layers.int.new("fase")
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    teile.append(objekt(name, bm, farbe_von))


def _satteldach(teile, farbe_von, breite, tiefe, traufe, first, ueberstand, zufall, reihen=7, name="Schindeln"):
    """Satteldach mit First entlang X: Schindelreihen als leicht versetzte Bretter."""
    halb = tiefe / 2 + ueberstand
    neigung = math.atan2(first - traufe, tiefe / 2)
    laenge_hang = math.hypot(halb, (first - traufe) * halb / (tiefe / 2))
    for seite in (-1, 1):
        for r in range(reihen):
            t = (r + 0.5) / reihen
            # von der Traufe (außen) zum First
            y = seite * (halb - t * halb)
            z = traufe - (ueberstand * math.tan(neigung)) + t * (first - traufe + ueberstand * math.tan(neigung))
            stuecke = 3
            for s in range(stuecke):
                b = (breite + 2 * ueberstand) / stuecke
                x = -(breite / 2 + ueberstand) + b * (s + 0.5) + zufall.uniform(-0.05, 0.05)
                _brett(teile, name, farbe_von, (b * 1.01, laenge_hang / reihen * 1.35, 0.07),
                       (x, y, z + 0.05), (seite * -math.degrees(neigung), 0, zufall.uniform(-0.6, 0.6)), fase=0.015)
    # Firstbalken
    _brett(teile, "First", farbe_von, (breite + 2 * ueberstand + 0.1, 0.28, 0.2), (0, 0, first + 0.12), fase=0.03)


def _giebel(teile, farbe_von, breite, x, traufe, first, bretter=7):
    """Giebeldreieck aus senkrechten Brettern bei `x` (Wand quer zur X-Achse)."""
    for i in range(bretter):
        t = (i + 0.5) / bretter
        y = -breite / 2 + t * breite
        hoehe = (first - traufe) * (1 - abs(y) / (breite / 2))
        if hoehe < 0.1:
            continue
        _brett(teile, "Giebel", farbe_von, (0.08, breite / bretter * 0.98, hoehe), (x, y, traufe + hoehe / 2), fase=0.01)


def _laterne(teile, licht, zufall, ort, hoehe=2.1):
    eisen = einfarbig("#2E2B2A", 0.05, zufall)
    x, y, z = ort
    _brett(teile, "Pfahl", holzfarbe(zufall, "#6E4A2A", "#4E321A"), (0.12, 0.12, hoehe), (x, y, z + hoehe / 2), fase=0.02)
    _brett(teile, "Arm", eisen, (0.5, 0.05, 0.05), (x + 0.22, y, z + hoehe - 0.1), fase=0.0)
    _brett(teile, "Laternendach", eisen, (0.26, 0.26, 0.06), (x + 0.42, y, z + hoehe - 0.2), fase=0.01)
    _brett(teile, "Laternenboden", eisen, (0.22, 0.22, 0.04), (x + 0.42, y, z + hoehe - 0.55), fase=0.0)
    bm = brett_bm(0.16, 0.16, 0.28, fase=0.0)
    setzen(bm, (x + 0.42, y, z + hoehe - 0.38))
    licht.append(objekt("Flamme", bm, einfarbig("#FFD890", 0.02, zufall)))


def _schild(teile, zufall, ort, drehung_z, bild):
    """Hängeschild an einem Galgen mit gemaltem Zeichen (`bild`: Liste von Brettern in Schildkoordinaten)."""
    holz = holzfarbe(zufall, "#7A5230", "#553820")
    x, y, z = ort
    rot = Matrix.Rotation(math.radians(drehung_z), 4, "Z")

    def p(dx, dy, dz):
        v = rot @ Vector((dx, dy, dz))
        return (x + v.x, y + v.y, z + v.z)
    _brett(teile, "Schildpfahl", holz, (0.14, 0.14, 2.6), p(0, 0, 1.3), (0, 0, drehung_z), fase=0.02)
    _brett(teile, "Schildarm", holz, (0.9, 0.1, 0.1), p(0.4, 0, 2.45), (0, 0, drehung_z), fase=0.01)
    for dx in (0.15, 0.65):
        _brett(teile, "Kette", einfarbig("#3A3634", 0.05, zufall), (0.02, 0.02, 0.22), p(dx, 0, 2.29), (0, 0, drehung_z), fase=0.0)
    _brett(teile, "Schild", einfarbig("#E9D9B0", 0.03, zufall), (0.8, 0.06, 0.5), p(0.4, 0, 1.93), (0, 0, drehung_z), fase=0.015)
    _brett(teile, "Schildrand", holz, (0.86, 0.05, 0.56), p(0.4, 0.012, 1.93), (0, 0, drehung_z), fase=0.01)
    for (bx, bz, bw, bh, winkel, hexfarbe) in bild:
        _brett(teile, "Zeichen", einfarbig(hexfarbe, 0.02, zufall), (bw, 0.02, bh), p(0.4 + bx, -0.045, 1.93 + bz),
               (0, winkel, drehung_z), fase=0.0)


# ---------------------------------------------------------------------------
# Holzfäller: Blockhütte mit Brennholzschuppen, Hackklotz, Sägebock und Stammstapel
# ---------------------------------------------------------------------------
def holzfaeller(seed=61):
    zufall = random.Random(seed)
    teile, licht = [], []
    B, T = 6.2, 4.6          # Hütte (x, y)
    boden = 0.45             # Oberkante Sockel
    lage_h, lagen = 0.34, 8
    traufe = boden + lage_h * lagen + 0.05
    first = traufe + 2.0

    stein = _steinfarbe(zufall, farbe("#6B665E"), farbe("#948D82"), farbe("#BDB5A8"), farbe("#D6CFC2"))
    erde = einfarbig("#8C7A58", 0.06, zufall)
    _platte(teile, "Hof", erde, 7.6, 6.4, 0.04, zufall)
    # Sockel aus Bruchsteinen
    _brett(teile, "Sockel", stein, (B + 0.7, T + 0.7, boden + SOCKEL), (0, 0, (boden - SOCKEL) / 2), fase=0.06)
    for i in range(26):
        w = math.tau * i / 26
        x = math.cos(w) * (B / 2 + 0.36) * 1.02
        y = math.sin(w) * (T / 2 + 0.36) * 1.02
        x = max(-(B / 2 + 0.36), min(B / 2 + 0.36, x * 1.3))
        y = max(-(T / 2 + 0.36), min(T / 2 + 0.36, y * 1.3))
        bm = stein_bm(zufall, zufall.uniform(0.2, 0.28), flach=0.7)
        setzen(bm, (x, y, zufall.uniform(0.1, 0.3)), (0, 0, zufall.uniform(0, 360)))
        teile.append(objekt("Sockelstein", bm, stein))

    # Blockwände: Längswände (entlang X) und Querwände abwechselnd um eine halbe Lage versetzt
    rinde = stammfarbe(zufall, "#9A6436", "#6A4222", "#E6BC80", "#B8864E")
    r = lage_h * 0.56
    tuer = (-0.65, 0.65, 6)          # Türöffnung vorne: x von … bis, Lagen darunter
    fenster_seite = (-0.7, 0.7, 3, 6)  # Fenster in den Querwänden: y von … bis, Lagen von … bis
    fenster_vorne = (1.4, 2.4, 3, 6)   # Fenster vorne rechts
    for lage in range(lagen):
        z = boden + r + lage * lage_h
        for seite in (-1, 1):
            y = seite * T / 2
            lucken = []
            if seite == -1 and lage < tuer[2]:
                lucken.append((tuer[0], tuer[1]))
            if seite == -1 and fenster_vorne[2] <= lage < fenster_vorne[3]:
                lucken.append((fenster_vorne[0], fenster_vorne[1]))
            stuecke, start = [], -B / 2 - 0.35
            for a, b in sorted(lucken):
                stuecke.append((start, a))
                start = b
            stuecke.append((start, B / 2 + 0.35))
            for a, b in stuecke:
                if b - a > 0.2:
                    _stamm(teile, "Blockbalken", rinde, r * zufall.uniform(0.95, 1.05), b - a, ((a + b) / 2, y, z), seed=seed + lage * 13 + int(a * 10))
        z2 = z + lage_h / 2
        if lage == lagen - 1:
            continue
        for seite in (-1, 1):
            x = seite * B / 2
            stuecke = [(-T / 2 - 0.35, T / 2 + 0.35)]
            if fenster_seite[2] <= lage < fenster_seite[3]:
                stuecke = [(-T / 2 - 0.35, fenster_seite[0]), (fenster_seite[1], T / 2 + 0.35)]
            for a, b in stuecke:
                _stamm(teile, "Blockbalken", rinde, r * zufall.uniform(0.95, 1.05), b - a, (x, (a + b) / 2, z2), (0, 0, 90), seed=seed + lage * 17 + int(a * 10) + 5)
    # Innenboden (durch Tür und Fenster sichtbar)
    _brett(teile, "Dielen", holzfarbe(zufall, "#A87A4A", "#7E5630"), (B - 0.2, T - 0.2, 0.06), (0, 0, boden + 0.03), fase=0.0)

    # Tür aus Brettern, einen Spalt offen, mit Eisenbeschlägen
    tuerholz = holzfarbe(zufall, "#7C4E2A", "#5A3618")
    eisen = einfarbig("#35312E", 0.05, zufall)
    tuer_h = lage_h * tuer[2] - 0.05
    for i in range(5):
        _brett(teile, "Türbrett", tuerholz, (0.25, 0.07, tuer_h), (tuer[0] + 0.14 + i * 0.26, -T / 2 - 0.02, boden + tuer_h / 2), (0, 0, 0), fase=0.01)
    for dz in (0.35, tuer_h - 0.35):
        _brett(teile, "Beschlag", eisen, (1.2, 0.03, 0.08), (0, -T / 2 - 0.07, boden + dz), fase=0.0)
    _brett(teile, "Türsturz", tuerholz, (1.6, 0.3, 0.22), (0, -T / 2, boden + tuer_h + 0.12), fase=0.03)
    # Fensterrahmen, Scheiben (warmes Licht von innen) und bunte Fensterläden
    laden = einfarbig("#3E7A55", 0.04, zufall)
    rahmen = holzfarbe(zufall, "#C49A62", "#96703E")
    fz0 = boden + lage_h * fenster_vorne[2]
    fz1 = boden + lage_h * fenster_vorne[3]
    for (cx, cy, quer) in (((fenster_vorne[0] + fenster_vorne[1]) / 2, -T / 2, False), (-B / 2, 0.0, True), (B / 2, 0.0, True)):
        breite = 1.0 if not quer else fenster_seite[1] - fenster_seite[0]
        dreh = (0, 0, 90) if quer else (0, 0, 0)
        _brett(teile, "Fensterbank", rahmen, (breite + 0.3, 0.3, 0.08), (cx, cy, fz0), dreh, fase=0.01)
        _brett(teile, "Fenstersturz", rahmen, (breite + 0.3, 0.3, 0.1), (cx, cy, fz1), dreh, fase=0.01)
        bm = brett_bm(breite, 0.04, fz1 - fz0 - 0.05, fase=0.0)
        setzen(bm, (0, 0, 0), dreh)
        setzen(bm, (cx, cy, (fz0 + fz1) / 2))
        licht.append(objekt("Scheibe", bm, einfarbig("#FFC878", 0.02, zufall)))
        _brett(teile, "Sprosse", rahmen, (0.05, 0.06, fz1 - fz0), (cx, cy + (0.03 if quer else -0.03), (fz0 + fz1) / 2), dreh, fase=0.0)
        for s in (-1, 1):
            # Läden aufgeklappt an der Wand
            if quer:
                ort = (cx + math.copysign(0.24, cx), cy + s * (breite / 2 + 0.3), (fz0 + fz1) / 2)
            else:
                ort = (cx + s * (breite / 2 + 0.3), cy - 0.24, (fz0 + fz1) / 2)
            _brett(teile, "Laden", laden, (0.5, 0.05, fz1 - fz0 - 0.08), ort, dreh, fase=0.012)

    # Dach: Giebel, Schindeln, Schornstein
    giebelholz = holzfarbe(zufall, "#B07A44", "#86582E")
    for x in (-B / 2, B / 2):
        _giebel(teile, giebelholz, T + 0.3, x, traufe - 0.05, first - 0.05)
    schindel = holzfarbe(zufall, "#B4523A", "#8A3A28", maserung=9.0)
    _satteldach(teile, schindel, B, T, traufe, first, 0.55, zufall, reihen=7)
    for i in range(9):
        z = boden + 0.2 + i * 0.62
        for dx, dy in ((0, 0), (0.28, 0.22)):
            bm = brett_bm(0.55 + zufall.uniform(-0.05, 0.05), 0.5, 0.3, fase=0.03)
            setzen(bm, (B / 2 - 1.2 + dx * (i % 2), 0.9 + dy * (i % 2), z + (0.3 if dx else 0)), (0, 0, zufall.uniform(-6, 6)))
            teile.append(objekt("Kaminstein", bm, stein))
    _brett(teile, "Kaminkern", stein, (0.72, 0.62, 5.7 - boden), (B / 2 - 1.05, 1.0, (5.7 + boden) / 2), fase=0.03)
    _brett(teile, "Kaminkrone", stein, (0.9, 0.8, 0.18), (B / 2 - 1.05, 1.0, 5.75), fase=0.03)

    # Veranda vorne: Dielen und zwei Pfosten mit Vordach über der Tür
    diele = holzfarbe(zufall, "#B8864E", "#8E6334")
    for i in range(6):
        _brett(teile, "Verandadiele", diele, (3.0, 0.24, 0.07), (-0.6, -T / 2 - 0.4 - i * 0.25, boden - 0.05), (0, 0, zufall.uniform(-0.5, 0.5)), fase=0.01)
    for x in (-1.9, 0.7):
        _brett(teile, "Verandapfosten", giebelholz, (0.16, 0.16, 2.2), (x, -T / 2 - 1.6, boden + 1.1), fase=0.02)
    for i in range(4):
        _brett(teile, "Vordach", schindel, (3.1, 0.42, 0.06), (-0.6, -T / 2 - 0.2 - i * 0.4, boden + 2.55 - i * 0.12), (-17, 0, 0), fase=0.012)
    _laterne(teile, licht, zufall, (1.25, -T / 2 - 1.7, 0.0), hoehe=2.0)

    # Brennholzschuppen rechts an der Hütte: Pultdach auf Pfosten, darunter gespaltene Scheite
    sx0, sx1 = B / 2 + 0.35, B / 2 + 2.3
    for x in (sx1 - 0.1,):
        for y in (-T / 2 + 0.2, T / 2 - 0.2):
            _brett(teile, "Schuppenpfosten", giebelholz, (0.14, 0.14, 2.3), (x, y, 1.15), fase=0.02)
    for i in range(6):
        y = -T / 2 - 0.1 + i * (T + 0.2) / 5.5
        _brett(teile, "Pultdach", schindel, (2.3, (T + 0.3) / 5.2, 0.06), ((sx0 + sx1) / 2 + 0.1, y + 0.4, 2.5), (0, 12, 0), fase=0.012)
    scheit = stammfarbe(zufall, "#8A5A30", "#5A3A1C", "#E8C48C", "#C49A62")
    for lage in range(6):
        for i in range(9):
            y = -T / 2 + 0.35 + i * 0.48 + (0.24 if lage % 2 else 0)
            if y > T / 2 - 0.2:
                continue
            _stamm(teile, "Scheit", scheit, 0.11 * zufall.uniform(0.9, 1.1), 1.3, ((sx0 + sx1) / 2 - 0.1, y, 0.13 + lage * 0.2), (0, 0, zufall.uniform(-4, 4)), ecken=6, seed=seed + lage * 31 + i)

    # Hof: großer Stammstapel, Sägebock mit Stamm, Hackklotz mit Axt, Schild
    stamm = stammfarbe(zufall, "#7A5230", "#4E3218", "#E0B070", "#B08048")
    for lage, (anzahl, versatz) in enumerate(((4, 0.0), (3, 0.5), (2, 1.0))):
        for i in range(anzahl):
            _stamm(teile, "Stamm", stamm, 0.28, 3.4 + zufall.uniform(-0.2, 0.2),
                   (-B / 2 - 1.9 + (i + versatz * 0.5) * 0.58 - 0.9, -1.2, 0.28 + lage * 0.5), (0, 0, 90 + zufall.uniform(-3, 3)), ecken=11, seed=seed + 200 + lage * 7 + i)
    for y in (-2.9, 0.5):
        _brett(teile, "Keil", giebelholz, (2.2, 0.14, 0.14), (-B / 2 - 2.4, y, 0.07), fase=0.02)
    # Sägebock (zwei X-Böcke) mit aufgelegtem Stamm und Säge
    bock = holzfarbe(zufall, "#9C6E3C", "#724C26")
    for y in (-3.4, -2.4):
        for w in (35, -35):
            _brett(teile, "Bockbein", bock, (0.1, 0.1, 1.2), (2.9, y, 0.5), (w, 0, 0), fase=0.01)
    _stamm(teile, "Sägestamm", stamm, 0.22, 2.2, (2.9, -2.9, 0.95), (0, 0, 90), ecken=10, seed=seed + 300)
    _brett(teile, "Sägeblatt", einfarbig("#B8BEC6", 0.03, zufall), (0.03, 1.1, 0.16), (2.9, -2.1, 1.15), (0, 0, 8), fase=0.0)
    _brett(teile, "Sägegriff", bock, (0.06, 0.1, 0.3), (2.93, -1.52, 1.2), (0, 0, 8), fase=0.01)
    bm = stamm_bm(0.34, 0.55, ecken=14, seed=seed + 400)
    setzen(bm, (0, 0, 0), (0, -90, 0))
    setzen(bm, (1.6, -4.3, 0))
    teile.append(objekt("Hackklotz", bm, stamm))
    _brett(teile, "Axtstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.75, 0.05, 0.05), (1.82, -4.3, 0.84), (0, -38, 0), fase=0.008)
    _brett(teile, "Axtkopf", einfarbig("#8E959E", 0.04, zufall), (0.16, 0.035, 0.22), (1.55, -4.3, 0.62), (0, -38, 0), fase=0.006)
    for i in range(7):
        _stamm(teile, "Spaltscheit", scheit, 0.09, 0.45, (1.0 + zufall.uniform(-0.6, 0.6), -4.6 + zufall.uniform(-0.5, 0.4), 0.09), (0, 0, zufall.uniform(0, 360)), ecken=6, seed=seed + 500 + i)
    # Schild: gekreuzte Äxte
    _schild(teile, zufall, (-3.6, -4.9, 0.0), 0, [
        (-0.05, 0.0, 0.5, 0.05, 40, "#7A4A26"), (0.05, 0.0, 0.5, 0.05, -40, "#7A4A26"),
        (-0.2, 0.15, 0.14, 0.12, 40, "#5C636C"), (0.2, 0.15, 0.14, 0.12, -40, "#5C636C"),
    ])
    return _fertig("Holzfaeller", teile, licht)


# ---------------------------------------------------------------------------
# Steinbruch: Felswand mit ausgeschnittenen Blöcken, Holzkran, Blockstapel, Werkstattdach
# ---------------------------------------------------------------------------
def steinbruch(seed=62):
    zufall = random.Random(seed)
    teile, licht = [], []
    fels = _steinfarbe(zufall, farbe("#6E6A63"), farbe("#9A948A"), farbe("#C6BFB2"), farbe("#DDD6C8"))
    block = _steinfarbe(zufall, farbe("#9E978B"), farbe("#BDB6A9"), farbe("#DAD3C5"), farbe("#EDE7DA"), schicht=6.0)
    kies = einfarbig("#A7A092", 0.07, zufall)
    _platte(teile, "Kiesplatz", kies, 8.2, 7.0, 0.05, zufall)

    # Felswand hinten (+Y): große Brocken, vorne senkrecht „abgeschnitten“
    for i, (x, y, rad, hoch) in enumerate(((-4.2, 3.6, 2.2, 1.5), (-1.3, 4.3, 2.6, 1.8), (1.9, 4.0, 2.4, 1.6), (4.6, 3.3, 1.9, 1.3), (0.2, 5.4, 2.3, 1.9))):
        bm = _brocken_bm(zufall, rad, (1.1, 0.9, hoch), 11, fase=0.03)
        # senkrechte Abbaukante nach vorne
        bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=Vector((0, -rad * 0.35, 0)), plane_no=Vector((0, -1, 0)), clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
        bmesh.ops.triangulate(bm, faces=bm.faces[:])
        _setzen(bm, (x, y, rad * hoch * 0.55 - 0.6), zufall.uniform(-0.2, 0.2))
        teile.append(objekt(f"Fels{i}", bm, fels))
    # Stufen im Fels: halb herausgelöste Blöcke
    for i in range(7):
        x = -4.5 + i * 1.5 + zufall.uniform(-0.2, 0.2)
        stufe = i % 3
        _brett(teile, "Rohblock", block, (1.2, 0.9, 0.8), (x, 2.3 + stufe * 0.55, 0.4 + stufe * 0.7), (0, 0, zufall.uniform(-4, 4)), fase=0.05)
    # Keile und Meißelspuren (dunkle Schlitze) an der Kante
    eisen = einfarbig("#3C3936", 0.05, zufall)
    for i in range(5):
        _brett(teile, "Keil", eisen, (0.08, 0.12, 0.2), (-2.4 + i * 0.45, 1.84, 1.35), (0, 0, 0), fase=0.0)

    # Holzkran: Mast, Ausleger, Strebe, Seiltrommel mit Kurbel, Seil mit hängendem Block
    holz = holzfarbe(zufall, "#A6773F", "#7C552A")
    dunkel = holzfarbe(zufall, "#7A5230", "#553820")
    mx, my = 2.6, 0.2
    for dx, dy in ((-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)):
        _brett(teile, "Kranfuß", dunkel, (0.3, 0.3, 0.3), (mx + dx * 1.6, my + dy * 1.6, 0.15), fase=0.03)
    for dx, dy in ((-0.8, 0), (0.8, 0), (0, -0.8), (0, 0.8)):
        _brett(teile, "Stütze", holz, (0.14, 0.14, 1.9), (mx + dx * 0.6, my + dy * 0.6, 0.85), (dy * 30, -dx * 30, 0), fase=0.02)
    _stamm(teile, "Mast", stammfarbe(zufall, "#8A5A30", "#5E3D22", "#E0B878", "#B08650"), 0.2, 5.4, (mx, my, 2.7), (0, -90, 0), ecken=10, seed=seed + 1)
    _stamm(teile, "Ausleger", stammfarbe(zufall, "#8A5A30", "#5E3D22", "#E0B878", "#B08650"), 0.15, 4.6, (mx - 1.7, my - 0.6, 5.05), (0, -8, -160), ecken=10, seed=seed + 2)
    _brett(teile, "Strebe", holz, (0.12, 0.12, 2.6), (mx - 0.9, my - 0.35, 4.1), (-10, 50, 20), fase=0.02)
    _stamm(teile, "Trommel", holz, 0.28, 0.9, (mx + 0.55, my, 1.3), (0, 0, 90), ecken=12, seed=seed + 3)
    for s in (-1, 1):
        _brett(teile, "Trommelbock", dunkel, (0.12, 0.12, 1.4), (mx + 0.55, my + s * 0.52, 0.7), fase=0.01)
    _brett(teile, "Kurbel", eisen, (0.05, 0.05, 0.5), (mx + 0.55, my + 0.62, 1.5), (30, 0, 0), fase=0.0)
    seil = einfarbig("#C8B58A", 0.03, zufall)
    sx, sy = mx - 3.8, my - 1.3
    _brett(teile, "Seil", seil, (0.035, 0.035, 2.8), (sx, sy, 3.85), fase=0.0)
    _brett(teile, "Seilschlinge", seil, (0.9, 0.9, 0.035), (sx, sy, 2.46), fase=0.0)
    _brett(teile, "Hängeblock", block, (0.8, 0.8, 0.8), (sx, sy, 2.05), (0, 0, 20), fase=0.05)
    _brett(teile, "Rolle", eisen, (0.1, 0.22, 0.22), (sx, sy, 5.2), fase=0.0)

    # Gestapelte, fertige Quader auf Paletten
    for (px, py, reihen) in ((-3.8, -1.9, 3), (-1.9, -2.6, 2), (-4.4, -4.1, 2)):
        for i in range(3):
            _brett(teile, "Palette", dunkel, (1.9, 0.18, 0.12), (px, py - 0.55 + i * 0.55, 0.06), fase=0.01)
        for lage in range(reihen):
            for i in range(2):
                for j in range(2):
                    if lage == reihen - 1 and (i + j) % 2 and reihen > 2:
                        continue
                    _brett(teile, "Quader", block, (0.82, 0.55, 0.5), (px - 0.43 + i * 0.86, py - 0.29 + j * 0.58, 0.37 + lage * 0.52), (0, 0, zufall.uniform(-2, 2)), fase=0.04)

    # Werkstatt: offenes Pultdach auf vier Pfosten, Werkbank mit Hammer und Meißel
    wx, wy = 3.4, -3.0
    for dx in (-1.5, 1.5):
        for dy, h in ((-1.1, 2.4), (1.1, 2.8)):
            _brett(teile, "Pfosten", holz, (0.16, 0.16, h), (wx + dx, wy + dy, h / 2), fase=0.02)
    dach = holzfarbe(zufall, "#4F7FA0", "#3A6480", maserung=7.0)
    for i in range(6):
        _brett(teile, "Dachbrett", dach, (3.6, 0.5, 0.06), (wx, wy - 1.35 + i * 0.52, 2.45 + i * 0.09), (10, 0, 0), fase=0.012)
    _brett(teile, "Werkbank", holz, (2.2, 0.8, 0.12), (wx, wy + 0.5, 0.95), fase=0.02)
    for dx in (-0.95, 0.95):
        for dy in (0.2, 0.8):
            _brett(teile, "Bankbein", dunkel, (0.1, 0.1, 0.9), (wx + dx, wy + dy, 0.45), fase=0.01)
    _brett(teile, "Werkstück", block, (0.6, 0.45, 0.4), (wx - 0.3, wy + 0.5, 1.21), (0, 0, 12), fase=0.04)
    _brett(teile, "Hammerstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.45, 0.04, 0.04), (wx + 0.5, wy + 0.35, 1.04), (0, 0, 30), fase=0.0)
    _brett(teile, "Hammerkopf", eisen, (0.08, 0.16, 0.08), (wx + 0.3, wy + 0.24, 1.06), (0, 0, 30), fase=0.0)
    _brett(teile, "Meißel", eisen, (0.22, 0.03, 0.03), (wx + 0.8, wy + 0.7, 1.03), (0, 0, -20), fase=0.0)
    _laterne(teile, licht, zufall, (wx - 1.5, wy - 1.4, 0.0), hoehe=2.2)

    # Schubkarre mit Bruchsteinen
    kx, ky = 0.2, -3.6
    _brett(teile, "Karrenboden", dunkel, (1.1, 0.7, 0.08), (kx, ky, 0.55), (0, -8, 20), fase=0.01)
    for s in (-1, 1):
        _brett(teile, "Karrenwand", holz, (1.1, 0.06, 0.3), (kx, ky + s * 0.33, 0.7), (0, -8, 20), fase=0.01)
        _brett(teile, "Holm", holz, (1.2, 0.06, 0.06), (kx - 0.9, ky + s * 0.28 - 0.3, 0.55), (0, -12, 20), fase=0.0)
    bm = stamm_bm(0.26, 0.08, ecken=12, seed=seed + 9)
    setzen(bm, (-0.04, 0, 0), (0, 0, 110))
    setzen(bm, (kx + 0.62, ky + 0.22, 0.26))
    teile.append(objekt("Rad", bm, dunkel))
    for i in range(6):
        bm = stein_bm(zufall, zufall.uniform(0.12, 0.18))
        setzen(bm, (kx + zufall.uniform(-0.35, 0.35), ky + zufall.uniform(-0.2, 0.2), 0.75))
        teile.append(objekt("Bruchstein", bm, fels))
    teile += _splitter(zufall, fels, 26, 6.5, (0.08, 0.2))
    _schild(teile, zufall, (-6.0, -3.2, 0.0), 0, [
        (0.0, 0.05, 0.36, 0.22, 0, "#8E8A82"), (-0.1, 0.05, 0.04, 0.22, 0, "#6A665F"),
        (0.2, -0.1, 0.35, 0.05, 45, "#6E4A2A"), (0.08, 0.02, 0.16, 0.1, 45, "#4A4642"),
    ])
    return _fertig("Steinbruch", teile, licht)


# ---------------------------------------------------------------------------
# Erzmine: Felshügel mit Stolleneingang (Holzrahmen, Vordach), Schienen, Lore voller Erz
# ---------------------------------------------------------------------------
def erzmine(seed=63):
    zufall = random.Random(seed)
    teile, licht = [], []
    fels = _steinfarbe(zufall, farbe("#5A544E"), farbe("#7E766C"), farbe("#A89E90"), farbe("#C4BAAC"),
                       moos=farbe("#6F9A48"), moos_rauschen=lambda p: 0.5 + 0.5 * math.sin(p.x * 1.3 + p.y * 0.7))
    erz = _steinfarbe(zufall, farbe("#6A3322"), farbe("#9A4A2C"), farbe("#C8683A"), farbe("#E08E5A"))
    erde = einfarbig("#80705A", 0.06, zufall)
    _platte(teile, "Vorplatz", erde, 7.6, 6.8, 0.05, zufall)

    # Hügel aus Felsbrocken hinter dem Eingang, mit Erzadern
    for i, (x, y, rad, hoch) in enumerate(((0.0, 3.6, 3.2, 1.25), (-3.4, 2.8, 2.3, 1.0), (3.3, 3.0, 2.4, 1.05), (-1.8, 5.2, 2.6, 1.2),
                                           (2.2, 5.4, 2.4, 1.1), (-4.8, 4.8, 1.8, 0.9), (4.9, 4.6, 1.7, 0.85))):
        bm = _brocken_bm(zufall, rad, (1.05, 0.95, hoch), 12, fase=0.03)
        _setzen(bm, (x, y, rad * hoch * 0.5 - 0.7), zufall.uniform(0, math.tau))
        teile.append(objekt(f"Hügel{i}", bm, fels))
    for i in range(10):
        w = zufall.uniform(-1.2, 1.2)
        bm = _brocken_bm(zufall, zufall.uniform(0.3, 0.5), (1.3, 0.7, 0.8), 7, fase=0.04)
        _setzen(bm, (math.sin(w) * 2.9, 3.6 - math.cos(w) * 2.3, zufall.uniform(1.2, 2.8)), zufall.uniform(0, math.tau))
        teile.append(objekt("Erzader", bm, erz))

    # Stolleneingang: dunkle Öffnung, Holzrahmen mit Streben, kleines Schindeldach
    _brett(teile, "Stollen", einfarbig("#140F0C", 0.02, zufall), (2.3, 1.6, 2.6), (0, 1.55, 1.25), fase=0.0)
    holz = stammfarbe(zufall, "#7C5230", "#523418", "#D8A868", "#A87C48")
    for x in (-1.25, 1.25):
        _stamm(teile, "Stempel", holz, 0.17, 2.9, (x, 0.75, 1.45), (0, -90, 0), ecken=9, seed=seed + int(x * 10))
        _stamm(teile, "Strebe", holz, 0.1, 1.2, (x * 0.8, 0.72, 2.55), (0, -90 + (40 if x > 0 else -40), 0), ecken=7, seed=seed + 7)
    _stamm(teile, "Kappe", holz, 0.19, 3.3, (0, 0.75, 3.0), (0, 0, 0), ecken=9, seed=seed + 20)
    for i in range(4):
        _stamm(teile, "Tragbalken", holz, 0.11, 2.6, (-1.2 + i * 0.8, 1.2, 3.25), (0, 0, 90), ecken=7, seed=seed + 30 + i)
    schindel = holzfarbe(zufall, "#3F7F66", "#2E6050", maserung=9.0)
    for i in range(5):
        _brett(teile, "Vordach", schindel, (3.6, 0.42, 0.07), (0, 0.9 - i * 0.38, 3.5 - i * 0.14), (-20, 0, 0), fase=0.012)
    _brett(teile, "Stollenschild", einfarbig("#E9D9B0", 0.03, zufall), (1.1, 0.06, 0.34), (0, 0.5, 3.0), fase=0.015)
    for i in range(3):
        _brett(teile, "Schrift", einfarbig("#5A3A20", 0.03, zufall), (0.22, 0.02, 0.12), (-0.33 + i * 0.33, 0.46, 3.0), fase=0.0)
    for x in (-1.6, 1.6):
        _laterne(teile, licht, zufall, (x, 0.1, 0.0), hoehe=2.3)

    # Schienen aus dem Stollen heraus, mit Schwellen
    eisen = einfarbig("#4A4A4E", 0.04, zufall)
    for s in (-0.42, 0.42):
        _brett(teile, "Schiene", eisen, (0.07, 6.2, 0.09), (s, -1.4, 0.2), fase=0.0)
    schwelle = holzfarbe(zufall, "#6E4A2A", "#4E321A")
    for i in range(12):
        _brett(teile, "Schwelle", schwelle, (1.3, 0.22, 0.12), (0, 1.5 - i * 0.52, 0.1), (0, 0, zufall.uniform(-3, 3)), fase=0.01)
    # Lore voller Erz
    ly = -2.2
    lore = einfarbig("#6B5E52", 0.04, zufall)
    _brett(teile, "Loreboden", lore, (1.0, 1.3, 0.1), (0, ly, 0.52), fase=0.01)
    for s in (-1, 1):
        _brett(teile, "Lorewand", lore, (0.08, 1.4, 0.6), (s * 0.55, ly, 0.82), (0, s * 10, 0), fase=0.01)
        _brett(teile, "Lorestirn", lore, (1.2, 0.08, 0.6), (0, ly + s * 0.68, 0.82), (s * -10, 0, 0), fase=0.01)
        for dy in (-0.4, 0.4):
            bm = stamm_bm(0.17, 0.08, ecken=10, seed=seed + 50)
            setzen(bm, (-0.04, 0, 0))
            setzen(bm, (s * 0.44, ly + dy, 0.4))
            teile.append(objekt("Lorenrad", bm, eisen))
    for dy in (-0.62, 0.62):
        _brett(teile, "Loreband", eisen, (1.25, 0.04, 0.06), (0, ly + dy * 1.1, 1.05), fase=0.0)
    for i in range(9):
        bm = _brocken_bm(zufall, zufall.uniform(0.18, 0.26), (1.1, 1.0, 0.8), 6, fase=0.04)
        _setzen(bm, (zufall.uniform(-0.3, 0.3), ly + zufall.uniform(-0.45, 0.45), 1.02 + zufall.uniform(0, 0.12)), zufall.uniform(0, math.tau))
        teile.append(objekt("Erz", bm, erz))
    # Funkelnde Kristalle im Erz (leuchten leicht)
    for i in range(4):
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.06, radius2=0.0, depth=0.25)
        setzen(bm, (zufall.uniform(-0.3, 0.3), ly + zufall.uniform(-0.4, 0.4), 1.2), (zufall.uniform(-25, 25), zufall.uniform(-25, 25), 0))
        licht.append(objekt("Kristall", bm, einfarbig("#9FE0FF", 0.05, zufall)))

    # Erzhaufen, Kisten, Fass, Spitzhacke und Schild
    for i in range(12):
        w = zufall.uniform(0, math.tau)
        d = zufall.uniform(0.0, 0.9)
        bm = _brocken_bm(zufall, zufall.uniform(0.22, 0.34), (1.1, 1.0, 0.8), 6, fase=0.04)
        _setzen(bm, (3.0 + math.cos(w) * d, -1.6 + math.sin(w) * d, 0.18 + (0.9 - d) * 0.35), zufall.uniform(0, math.tau))
        teile.append(objekt("Erzhaufen", bm, erz))
    kiste = holzfarbe(zufall, "#A87A44", "#7E5630")
    for (x, y, z, g) in ((-3.0, -1.2, 0.35, 0.7), (-3.2, -2.1, 0.3, 0.6), (-3.05, -1.55, 0.97, 0.55)):
        _brett(teile, "Kiste", kiste, (g, g, g), (x, y, z), (0, 0, zufall.uniform(-10, 10)), fase=0.03)
        _brett(teile, "Kistenband", eisen, (g + 0.02, 0.06, g + 0.02), (x, y, z), (0, 0, 0), fase=0.0)
    bm = stamm_bm(0.34, 0.95, ecken=14, radius_ende=0.34, knorrig=0.0, seed=seed + 60)
    setzen(bm, (0, 0, 0), (0, -90, 0))
    setzen(bm, (-2.2, -3.2, 0.0))
    teile.append(objekt("Fass", bm, holzfarbe(zufall, "#9A6B3F", "#6E4826")))
    _brett(teile, "Hackenstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.05, 0.05, 0.9), (2.2, -3.3, 0.45), (0, 25, 30), fase=0.0)
    _brett(teile, "Hackenkopf", eisen, (0.6, 0.05, 0.07), (2.05, -3.4, 0.86), (0, 25 - 90 + 70, 30), fase=0.0)
    _schild(teile, zufall, (-4.8, -3.6, 0.0), 0, [
        (-0.05, 0.02, 0.45, 0.05, 45, "#6E4A2A"), (0.05, 0.02, 0.45, 0.05, -45, "#6E4A2A"),
        (-0.14, 0.16, 0.26, 0.06, 45, "#4A4642"), (0.14, 0.16, 0.26, 0.06, -45, "#4A4642"),
        (0.0, -0.13, 0.14, 0.1, 0, "#B85A34"),
    ])
    return _fertig("Erzmine", teile, licht)


# ---------------------------------------------------------------------------
# Dorfhalle (Hauptgebäude einer Siedlung): Stufe 1 Langhaus, Stufe 2 Rathaus mit Glockenturm,
# Stufe 3 Burgfried – steinerner Wehrturm hinter dem Rathaus, Mauerstücke und Banner
# ---------------------------------------------------------------------------
def _fachwerkwand(teile, laenge, hoehe, ort, drehung_z, putz, balken, zufall, luecken=(), streben=True):
    """Fachwerkwand entlang ihrer X-Achse (Mitte unten bei `ort`): Putzfelder, dunkle Balken,
    Streben in jedem zweiten Feld. `luecken`: (von, bis, unten, oben) für Tür und Fenster."""
    x0, y0, z0 = ort
    rot = Matrix.Rotation(math.radians(drehung_z), 4, "Z")

    def p(dx, dy, dz):
        v = rot @ Vector((dx, dy, dz))
        return (x0 + v.x, y0 + v.y, z0 + v.z)
    felder = max(int(round(laenge / 1.5)), 2)
    breite = laenge / felder
    for i in range(felder):
        a = -laenge / 2 + i * breite
        b = a + breite
        mitte = (a + b) / 2
        offen = [l for l in luecken if l[0] < b and l[1] > a]
        if not offen:
            _brett(teile, "Putz", putz, (breite, 0.2, hoehe), p(mitte, 0, hoehe / 2), (0, 0, drehung_z), fase=0.0)
            if streben and i % 2 == 0:
                winkel = math.degrees(math.atan2(hoehe * 0.8, breite))
                _brett(teile, "Strebe", balken, (math.hypot(breite, hoehe * 0.8) * 0.95, 0.12, 0.14), p(mitte, -0.13, hoehe / 2),
                       (0, -winkel if i % 4 == 0 else winkel, drehung_z), fase=0.01)
        else:
            (la, lb, lu, lo) = offen[0]
            if lu > 0.05:
                _brett(teile, "Putz", putz, (breite, 0.2, lu), p(mitte, 0, lu / 2), (0, 0, drehung_z), fase=0.0)
            if lo < hoehe - 0.05:
                _brett(teile, "Putz", putz, (breite, 0.2, hoehe - lo), p(mitte, 0, (hoehe + lo) / 2), (0, 0, drehung_z), fase=0.0)
            for (c, d) in ((a, max(a, la)), (min(b, lb), b)):
                if d - c > 0.05:
                    _brett(teile, "Putz", putz, (d - c, 0.2, lo - lu), p((c + d) / 2, 0, (lu + lo) / 2), (0, 0, drehung_z), fase=0.0)
        _brett(teile, "Pfosten", balken, (0.2, 0.26, hoehe), p(a, -0.04, hoehe / 2), (0, 0, drehung_z), fase=0.015)
    _brett(teile, "Pfosten", balken, (0.2, 0.26, hoehe), p(laenge / 2, -0.04, hoehe / 2), (0, 0, drehung_z), fase=0.015)
    for z in (0.1, hoehe * 0.52, hoehe - 0.1):
        _brett(teile, "Riegel", balken, (laenge + 0.2, 0.26, 0.2), p(0, -0.05, z), (0, 0, drehung_z), fase=0.015)


def _dorf_fenster(teile, licht, zufall, mitte, drehung_z, breite, hoehe, rahmen, laden):
    x0, y0, z0 = mitte
    rot = Matrix.Rotation(math.radians(drehung_z), 4, "Z")

    def p(dx, dy, dz):
        v = rot @ Vector((dx, dy, dz))
        return (x0 + v.x, y0 + v.y, z0 + v.z)
    bm = brett_bm(breite, 0.05, hoehe, fase=0.0)
    setzen(bm, (0, 0, 0), (0, 0, drehung_z))
    setzen(bm, p(0, 0.02, 0))
    licht.append(objekt("Scheibe", bm, einfarbig("#FFC878", 0.02, zufall)))
    _brett(teile, "Fensterbank", rahmen, (breite + 0.3, 0.3, 0.08), p(0, -0.1, -hoehe / 2 - 0.04), (0, 0, drehung_z), fase=0.01)
    _brett(teile, "Fenstersturz", rahmen, (breite + 0.3, 0.3, 0.1), p(0, -0.1, hoehe / 2 + 0.05), (0, 0, drehung_z), fase=0.01)
    _brett(teile, "Sprosse", rahmen, (0.05, 0.08, hoehe), p(0, -0.04, 0), (0, 0, drehung_z), fase=0.0)
    _brett(teile, "Sprosse", rahmen, (breite, 0.08, 0.05), p(0, -0.04, 0), (0, 0, drehung_z), fase=0.0)
    for s in (-1, 1):
        _brett(teile, "Laden", laden, (breite / 2 + 0.05, 0.05, hoehe - 0.05), p(s * (breite * 0.75 + 0.1), -0.14, 0), (0, 0, drehung_z), fase=0.012)
    # Blumenkasten
    _brett(teile, "Blumenkasten", rahmen, (breite + 0.1, 0.25, 0.2), p(0, -0.25, -hoehe / 2 - 0.15), (0, 0, drehung_z), fase=0.01)
    for i in range(4):
        bm = stein_bm(zufall, 0.1, flach=1.0)
        setzen(bm, p(-breite / 2 + 0.15 + i * breite / 3.3, -0.25, -hoehe / 2 + 0.02))
        teile.append(objekt("Blume", bm, einfarbig(zufall.choice(["#E24A4A", "#F2C94C", "#E88AD0", "#6FA8E8"]), 0.05, zufall)))


def _banner_mast(teile, zufall, ort, hoehe=6.5, farbe_tuch="#3F6FB5"):
    x, y, z = ort
    mast = holzfarbe(zufall, "#6E4A2A", "#4E321A")
    gold = einfarbig("#D8AE4A", 0.03, zufall)
    _brett(teile, "Mast", mast, (0.16, 0.16, hoehe), (x, y, z + hoehe / 2), fase=0.03)
    _brett(teile, "Rahe", gold, (1.4, 0.08, 0.08), (x + 0.7, y, z + hoehe - 0.3), fase=0.01)
    tuch = einfarbig(farbe_tuch, 0.03, zufall)
    _brett(teile, "Banner", tuch, (1.3, 0.05, 2.4), (x + 0.72, y, z + hoehe - 1.55), fase=0.0)
    _brett(teile, "Bannerspitze", tuch, (0.9, 0.05, 0.5), (x + 0.72, y, z + hoehe - 2.9), (0, 45, 0), fase=0.0)
    _brett(teile, "Wappen", gold, (0.5, 0.06, 0.5), (x + 0.72, y - 0.02, z + hoehe - 1.3), (0, 45, 0), fase=0.0)
    bm = stein_bm(zufall, 0.12, flach=1.0)
    setzen(bm, (x, y, z + hoehe + 0.05))
    teile.append(objekt("Knauf", bm, gold))


def dorfhalle(stufe=1, seed=64):
    zufall = random.Random(seed + stufe)
    teile, licht = [], []
    B, T = 11.0, 6.4
    boden = 0.5
    geschoss = 3.2
    traufe = boden + geschoss * (2 if stufe >= 2 else 1)
    first = traufe + 3.2

    stein = _steinfarbe(zufall, farbe("#6B665E"), farbe("#948D82"), farbe("#BDB5A8"), farbe("#D6CFC2"))
    erde = einfarbig("#8C7A58", 0.06, zufall)
    putz = einfarbig("#EFE4C8", 0.025, zufall)
    balken = holzfarbe(zufall, "#5E3C22", "#3E2614")
    rahmen = holzfarbe(zufall, "#C49A62", "#96703E")
    laden = einfarbig("#3F6FB5", 0.04, zufall)
    ziegel = holzfarbe(zufall, "#B4523A", "#8A3A28", maserung=9.0)
    gold = einfarbig("#D8AE4A", 0.03, zufall)
    eisen = einfarbig("#35312E", 0.05, zufall)

    _platte(teile, "Dorfplatz", erde, 11.5, 9.5, 0.04, zufall, ecken=22)
    # Pflaster vor dem Tor
    for i in range(24):
        bm = stein_bm(zufall, zufall.uniform(0.28, 0.4), flach=0.35)
        setzen(bm, (zufall.uniform(-2.2, 2.2), -T / 2 - zufall.uniform(0.8, 3.4), 0.03), (0, 0, zufall.uniform(0, 360)))
        teile.append(objekt("Pflaster", bm, stein))
    # Sockel und Freitreppe
    _brett(teile, "Sockel", stein, (B + 0.6, T + 0.6, boden + SOCKEL), (0, 0, (boden - SOCKEL) / 2), fase=0.06)
    for i in range(3):
        _brett(teile, "Stufe", stein, (3.2 - i * 0.2, 0.45, boden - i * 0.16), (0, -T / 2 - 0.5 - i * 0.42, (boden - i * 0.16) / 2), fase=0.03)
    # Erdgeschoss: Fachwerk ringsum, Tor vorne, Fenster
    tor = (-1.0, 1.0, 0.0, 2.7)
    fenster = [(-3.6, -2.4, 1.1, 2.3), (2.4, 3.6, 1.1, 2.3)]
    _fachwerkwand(teile, B, geschoss, (0, -T / 2, boden), 0, putz, balken, zufall, luecken=[tor] + fenster)
    _fachwerkwand(teile, B, geschoss, (0, T / 2, boden), 180, putz, balken, zufall, luecken=[(-0.6, 0.6, 1.1, 2.3)])
    _fachwerkwand(teile, T, geschoss, (-B / 2, 0, boden), 90, putz, balken, zufall, luecken=[(-0.6, 0.6, 1.1, 2.3)])
    _fachwerkwand(teile, T, geschoss, (B / 2, 0, boden), -90, putz, balken, zufall, luecken=[(-0.6, 0.6, 1.1, 2.3)])
    for (a, b, u, o) in fenster:
        _dorf_fenster(teile, licht, zufall, ((a + b) / 2, -T / 2 - 0.12, boden + (u + o) / 2), 0, b - a, o - u, rahmen, laden)
    for (x, y, d) in ((0, T / 2 + 0.12, 180), (-B / 2 - 0.12, 0, 90), (B / 2 + 0.12, 0, -90)):
        _dorf_fenster(teile, licht, zufall, (x, y, boden + 1.7), d, 1.2, 1.2, rahmen, laden)
    # Zweiflügeliges Tor mit Rundbogen aus Holz und Eisenbändern
    torholz = holzfarbe(zufall, "#7C4E2A", "#5A3618")
    for i in range(8):
        _brett(teile, "Torbrett", torholz, (0.24, 0.08, 2.6), (-0.87 + i * 0.25, -T / 2 - 0.02, boden + 1.3), fase=0.01)
    for dz in (0.5, 2.1):
        _brett(teile, "Torband", eisen, (2.0, 0.04, 0.1), (0, -T / 2 - 0.08, boden + dz), fase=0.0)
    for w in range(7):
        a = math.pi * w / 6
        _brett(teile, "Torbogen", balken, (0.5, 0.3, 0.22), (math.cos(a) * 1.05, -T / 2 - 0.08, boden + 2.7 + math.sin(a) * 0.55), (0, -math.degrees(a) + 90, 0), fase=0.02)
    if stufe >= 2:
        # Obergeschoss, leicht vorkragend, mit Balkon über dem Tor und Uhr/Wappen im Giebel
        z1 = boden + geschoss
        _brett(teile, "Deckenbalken", balken, (B + 0.7, T + 0.7, 0.25), (0, 0, z1), fase=0.02)
        _fachwerkwand(teile, B + 0.4, geschoss, (0, -T / 2 - 0.2, z1 + 0.12), 0, putz, balken, zufall, luecken=[(-3.6, -2.4, 0.9, 2.2), (-0.8, 0.8, 0.0, 2.4), (2.4, 3.6, 0.9, 2.2)])
        _fachwerkwand(teile, B + 0.4, geschoss, (0, T / 2 + 0.2, z1 + 0.12), 180, putz, balken, zufall, luecken=[(-2.0, -0.8, 0.9, 2.2), (0.8, 2.0, 0.9, 2.2)])
        _fachwerkwand(teile, T + 0.4, geschoss, (-B / 2 - 0.2, 0, z1 + 0.12), 90, putz, balken, zufall)
        _fachwerkwand(teile, T + 0.4, geschoss, (B / 2 + 0.2, 0, z1 + 0.12), -90, putz, balken, zufall)
        for x in (-3.0, 3.0):
            _dorf_fenster(teile, licht, zufall, (x, -T / 2 - 0.32, z1 + 1.67), 0, 1.2, 1.3, rahmen, laden)
        for x in (-1.4, 1.4):
            _dorf_fenster(teile, licht, zufall, (x, T / 2 + 0.32, z1 + 1.67), 180, 1.2, 1.3, rahmen, laden)
        # Balkontür und Balkon
        bm = brett_bm(1.4, 0.05, 2.2, fase=0.0)
        setzen(bm, (0, -T / 2 - 0.22, z1 + 1.25))
        licht.append(objekt("Balkontür", bm, einfarbig("#FFC878", 0.02, zufall)))
        _brett(teile, "Balkonboden", rahmen, (2.8, 1.2, 0.14), (0, -T / 2 - 0.85, z1 + 0.1), fase=0.02)
        for i in range(8):
            _brett(teile, "Geländerstab", rahmen, (0.07, 0.07, 0.9), (-1.3 + i * 0.37, -T / 2 - 1.4, z1 + 0.6), fase=0.01)
        _brett(teile, "Handlauf", balken, (2.9, 0.12, 0.1), (0, -T / 2 - 1.4, z1 + 1.08), fase=0.01)
        for x in (-1.3, 1.3):
            _brett(teile, "Balkonstütze", balken, (0.14, 0.14, 1.2), (x, -T / 2 - 1.3, z1 - 0.5), (35, 0, 0), fase=0.01)
    # Giebel (Putz mit Balkenkreuz) und Dach mit Ziegeln
    for x in (-B / 2 - (0.2 if stufe >= 2 else 0.0), B / 2 + (0.2 if stufe >= 2 else 0.0)):
        _giebel(teile, putz, T + (0.7 if stufe >= 2 else 0.3), x, traufe - 0.05, first - 0.05, bretter=9)
        _brett(teile, "Giebelbalken", balken, (0.14, 0.2, first - traufe), (x * 1.01, 0, (traufe + first) / 2), fase=0.01)
        # Gekreuzte Pferdeköpfe am Giebel
        for s in (-1, 1):
            _brett(teile, "Giebelzier", balken, (0.14, 1.2, 0.2), (x * 1.02, s * 0.35, first + 0.3), (s * 40, 0, 0), fase=0.02)
    _satteldach(teile, ziegel, B + (0.4 if stufe >= 2 else 0.0), T + (0.4 if stufe >= 2 else 0.0), traufe, first, 0.65, zufall, reihen=9, name="Dachziegel")
    # Schornstein
    _brett(teile, "Kamin", stein, (0.8, 0.7, first - traufe + 1.6), (B / 2 - 2.0, 1.4, (first + traufe) / 2 + 0.8), fase=0.03)
    _brett(teile, "Kaminkrone", stein, (1.0, 0.9, 0.18), (B / 2 - 2.0, 1.4, first + 1.65), fase=0.03)
    if stufe == 2:
        # Dachreiter mit Glocke auf dem First
        z = first + 0.1
        for sx in (-1, 1):
            for sy in (-1, 1):
                _brett(teile, "Glockenpfosten", balken, (0.16, 0.16, 2.0), (sx * 0.7, sy * 0.7, z + 1.0), fase=0.02)
        _brett(teile, "Glockenboden", balken, (1.7, 1.7, 0.16), (0, 0, z + 0.05), fase=0.02)
        bm = stein_bm(zufall, 0.45, flach=1.3)
        setzen(bm, (0, 0, z + 1.2))
        teile.append(objekt("Glocke", bm, gold))
        for i in range(5):
            _brett(teile, "Turmdach", ziegel, (2.0 - i * 0.38, 2.0 - i * 0.38, 0.35), (0, 0, z + 2.1 + i * 0.32), fase=0.02)
        bm = stein_bm(zufall, 0.12, flach=1.0)
        setzen(bm, (0, 0, z + 3.8))
        teile.append(objekt("Turmspitze", bm, gold))
    # Wappenscheibe im vorderen Giebel des Obergeschosses (Rathaus)
    if stufe >= 2:
        _brett(teile, "Wappenscheibe", gold, (1.1, 0.08, 1.1), (0, -T / 2 - 0.35, traufe + 0.9), (0, 45, 0), fase=0.02)
        _brett(teile, "Wappen", einfarbig("#3F6FB5", 0.03, zufall), (0.75, 0.1, 0.75), (0, -T / 2 - 0.37, traufe + 0.9), (0, 45, 0), fase=0.02)
    if stufe >= 3:
        # Burgfried hinter dem Rathaus: Bruchstein, Zinnen, Schießscharten, blaue Banner
        kx, ky, kb, kh = -5.2, 4.6, 5.6, 14.0
        _brett(teile, "Burgfried", stein, (kb, kb, kh + SOCKEL), (kx, ky, (kh - SOCKEL) / 2), fase=0.08)
        for z in (4.0, 8.0, 11.5):
            _brett(teile, "Gurtgesims", stein, (kb + 0.3, kb + 0.3, 0.3), (kx, ky, z), fase=0.05)
        for i in range(4):
            w = i * 90
            rot = Matrix.Rotation(math.radians(w), 4, "Z")
            for zz in (6.0, 9.8):
                v = rot @ Vector((0, -kb / 2 - 0.02, 0))
                bm = brett_bm(0.25, 0.06, 0.9, fase=0.0)
                setzen(bm, (0, 0, 0), (0, 0, w))
                setzen(bm, (kx + v.x, ky + v.y, zz))
                licht.append(objekt("Scharte", bm, einfarbig("#FFC878", 0.02, zufall)))
            for j in range(4):
                v = rot @ Vector((-kb / 2 + 0.55 + j * 1.5, -kb / 2 + 0.25, 0))
                _brett(teile, "Zinne", stein, (0.8, 0.5, 0.9), (kx + v.x, ky + v.y, kh + 0.45), (0, 0, w), fase=0.04)
            v = rot @ Vector((0, -kb / 2 - 0.06, 0))
            _brett(teile, "Turmbanner", einfarbig("#3F6FB5", 0.03, zufall), (1.2, 0.05, 3.2), (kx + v.x, ky + v.y, kh - 2.4), (0, 0, w), fase=0.0)
            _brett(teile, "Bannerrand", gold, (1.25, 0.06, 0.15), (kx + v.x, ky + v.y, kh - 0.8), (0, 0, w), fase=0.0)
        for sx in (-1, 1):
            for sy in (-1, 1):
                bm = stein_bm(zufall, 0.3, flach=1.6)
                setzen(bm, (kx + sx * (kb / 2 - 0.2), ky + sy * (kb / 2 - 0.2), kh + 1.2))
                teile.append(objekt("Eckspitze", bm, gold))
        # Mauerstücke links und rechts des Dorfplatzes mit Zinnen
        for sx in (-1, 1):
            for j in range(3):
                x = sx * (B / 2 + 2.5)
                y = -T / 2 - 1.0 + j * 2.8
                _brett(teile, "Mauer", stein, (0.9, 2.8, 2.6 + SOCKEL), (x, y, (2.6 - SOCKEL) / 2), fase=0.05)
                _brett(teile, "Mauerzinne", stein, (1.0, 1.0, 0.7), (x, y - 0.6, 2.95), fase=0.04)
    # Rund um die Halle: Banner, Feuerschale, Glocke am Galgen, Fässer, Bank
    _banner_mast(teile, zufall, (-3.6, -T / 2 - 3.2, 0.0), hoehe=6.5 + stufe)
    if stufe >= 2:
        _banner_mast(teile, zufall, (3.6, -T / 2 - 3.2, 0.0), hoehe=6.5 + stufe)
    for i in range(10):
        w = math.tau * i / 10
        bm = stein_bm(zufall, 0.22, flach=0.8)
        setzen(bm, (4.8 + math.cos(w) * 0.55, -T / 2 - 1.3 + math.sin(w) * 0.55, 0.12))
        teile.append(objekt("Feuerstein", bm, stein))
    for i in range(3):
        _stamm(teile, "Feuerholz", stammfarbe(zufall, "#8A5A30", "#5A3A1C", "#E8C48C", "#C49A62"), 0.08, 0.8, (4.8, -T / 2 - 1.3, 0.18), (0, 0, i * 60), ecken=6, seed=seed + 70 + i)
    bm = brett_bm(0.5, 0.5, 0.6, fase=0.0)
    setzen(bm, (4.8, -T / 2 - 1.3, 0.45))
    licht.append(objekt("Flammen", bm, einfarbig("#FF9A3A", 0.05, zufall)))
    if stufe == 1:
        # Glocke am Holzgalgen
        for x in (-5.8, -4.6):
            _brett(teile, "Galgenpfosten", balken, (0.16, 0.16, 2.8), (x, -T / 2 - 1.2, 1.4), fase=0.02)
        _brett(teile, "Galgenbalken", balken, (1.5, 0.16, 0.16), (-5.2, -T / 2 - 1.2, 2.8), fase=0.02)
        bm = stein_bm(zufall, 0.3, flach=1.3)
        setzen(bm, (-5.2, -T / 2 - 1.2, 2.3))
        teile.append(objekt("Glocke", bm, gold))
    fass = holzfarbe(zufall, "#8A5A30", "#6A4222")
    for (x, y) in ((B / 2 + 0.9, -1.5), (B / 2 + 0.9, -0.6), (B / 2 + 1.6, -1.05)):
        bm = stamm_bm(0.36, 0.85, ecken=10, seed=seed + int(x * 10 + y))
        setzen(bm, (0, 0, 0), (0, -90, 0))
        setzen(bm, (x, y, 0.0))
        teile.append(objekt("Fass", bm, fass))
        _brett(teile, "Fassreif", eisen, (0.76, 0.76, 0.05), (x, y, 0.62), fase=0.0)
    _brett(teile, "Bank", rahmen, (2.2, 0.45, 0.08), (-B / 2 + 2.0, -T / 2 - 1.0, 0.48), fase=0.01)
    for x in (-B / 2 + 1.1, -B / 2 + 2.9):
        _brett(teile, "Bankbein", balken, (0.1, 0.4, 0.45), (x, -T / 2 - 1.0, 0.22), fase=0.01)
    _laterne(teile, licht, zufall, (1.8, -T / 2 - 1.1, 0.0), hoehe=2.3)
    _laterne(teile, licht, zufall, (-2.4, -T / 2 - 1.1, 0.0), hoehe=2.3)
    return _fertig(["Dorfhalle", "Rathaus", "Burgfried"][stufe - 1], teile, licht)
