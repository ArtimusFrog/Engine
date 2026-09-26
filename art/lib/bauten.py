"""Bausteine für die Gebäude der Spieler (Baumenü mit B): Bretter, Stämme, Bodenplatten, Dächer,
Giebel, Laternen, Schilder, Fachwerkwände, Fenster und Bannermasten.

Die Gebäude selbst (Holzfäller, Steinbruch, Erzmine, Dorfhalle) stehen in `siedlung.py`, die Türme in
`tuerme.py`. Der Ursprung liegt auf dem Boden in der Mitte, die Vorderseite zeigt nach −Y (im Spiel
nach +Z). Ein Sockel reicht `SOCKEL` Meter unter den Boden, damit die Gebäude auch an leichten Hängen
nicht schweben.
"""

import math

import bmesh
from mathutils import Matrix, Vector

from lager import brett_bm, einfarbig, holzfarbe, leucht_material, objekt, setzen, stamm_bm, stein_bm
from vorkommen import _material
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
# Fachwerk, Fenster und Bannermast (für die Dorfhalle in siedlung.py)
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


