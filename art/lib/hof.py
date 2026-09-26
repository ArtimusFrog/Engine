"""Der Mühlenhof auf einer Wiese der Insel: Windmühle (Flügel drehen sich im Spiel), Brunnen,
Bienenstöcke, ein Weizenfeld und eine Vogelscheuche. Mid-Poly, hell und freundlich; Vorderseite
der Kulissen-Modelle ist +X (wie beim Wachturm)."""

import math
import random

import bmesh
from mathutils import Matrix, Vector

from lager import brett_bm, einfarbig, fertig, holzfarbe, objekt, setzen, stamm_bm, stammfarbe, stein_bm
from vorkommen import _material, _steinfarbe, farbe
from werkstatt import vereinen


def _brett(teile, name, farbe_von, masse, ort, drehung=(0, 0, 0), fase=0.0):
    bm = brett_bm(*masse, fase=fase)
    setzen(bm, (0, 0, 0), drehung)
    setzen(bm, ort)
    teile.append(objekt(name, bm, farbe_von))


def _ring(bm, radius, z, ecken, versatz=0.0):
    return [bm.verts.new((math.cos(math.tau * (k + versatz) / ecken) * radius, math.sin(math.tau * (k + versatz) / ecken) * radius, z)) for k in range(ecken)]


def _mantel(bm, a, b):
    n = len(a)
    for k in range(n):
        bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k]))


def _kegelstumpf(radien_hoehen, ecken, deckel=True):
    """Rotationskörper aus (Radius, Höhe)-Paaren von unten nach oben."""
    bm = bmesh.new()
    ringe = [_ring(bm, r, z, ecken) for r, z in radien_hoehen]
    for a, b in zip(ringe, ringe[1:]):
        _mantel(bm, a, b)
    if deckel:
        bm.faces.new(list(reversed(ringe[0])))
        if radien_hoehen[-1][0] > 0.001:
            bm.faces.new(ringe[-1])
    bm.faces.layers.int.new("fase")
    return bm


# ---------------------------------------------------------------------------
# Windmühle: weißer, achteckiger Turm auf Bruchsteinsockel, Holzkappe; Flügel extra
# ---------------------------------------------------------------------------
NABE = (2.05, 0.0, 8.35)  # Mitte der Flügelnabe (Blender), die Flügel drehen um die X-Achse


def windmuehle(seed=71):
    zufall = random.Random(seed)
    teile = []
    stein = _steinfarbe(zufall, farbe("#6E6960"), farbe("#948E83"), farbe("#BAB2A5"), farbe("#D2CABD"))
    putz = einfarbig("#F2EBDD", 0.03, zufall)
    holz = holzfarbe(zufall, "#8A5A32", "#643E20")
    dach = holzfarbe(zufall, "#A8472F", "#7C3220", maserung=9.0)
    # Sockel
    bm = _kegelstumpf([(2.55, -0.4), (2.5, 0.9)], 8)
    teile.append(objekt("Sockel", bm, stein))
    # Turm, leicht verjüngt
    bm = _kegelstumpf([(2.3, 0.9), (2.05, 4.0), (1.75, 7.4)], 8)
    teile.append(objekt("Turm", bm, putz))
    # Holzbänder und Fachwerk-Streifen
    for z in (0.95, 4.0, 7.35):
        r = 2.3 - (z - 0.9) / 6.5 * 0.55 + 0.04
        bm = _kegelstumpf([(r, z - 0.07), (r, z + 0.07)], 8, deckel=False)
        teile.append(objekt("Band", bm, holz))
    # Galerie (Umlauf) mit Geländer auf halber Höhe
    bm = _kegelstumpf([(2.75, 3.85), (2.75, 3.97)], 12)
    teile.append(objekt("Galerie", bm, holz))
    for k in range(12):
        w = math.tau * k / 12
        _brett(teile, "Geländerpfosten", holz, (0.07, 0.07, 0.8), (math.cos(w) * 2.68, math.sin(w) * 2.68, 4.37))
    bm = _kegelstumpf([(2.7, 4.72), (2.7, 4.8)], 12, deckel=False)
    teile.append(objekt("Handlauf", bm, holz))
    # Tür nach vorne (+X) und Fenster
    _brett(teile, "Tür", holzfarbe(zufall, "#6E4424", "#4E2E14"), (0.14, 1.0, 1.8), (2.3, 0.0, 1.8))
    _brett(teile, "Türsturz", stein, (0.3, 1.3, 0.2), (2.33, 0.0, 2.8))
    for (w, z) in ((0.0, 5.5), (math.pi * 0.5, 2.6), (math.pi * 1.1, 5.9), (math.pi * 1.5, 2.4)):
        r = 2.3 - (z - 0.9) / 6.5 * 0.55
        _brett(teile, "Fenster", einfarbig("#3A5A78", 0.05, zufall), (0.1, 0.55, 0.75), (math.cos(w) * r, math.sin(w) * r, z), (0, 0, math.degrees(w)))
        _brett(teile, "Fensterrahmen", holz, (0.08, 0.7, 0.9), (math.cos(w) * (r - 0.02), math.sin(w) * (r - 0.02), z), (0, 0, math.degrees(w)))
    # Kappe: Boden, Satteldach nach vorne, Giebel
    _brett(teile, "Kappenboden", holz, (4.2, 3.8, 0.25), (0.1, 0.0, 7.5), fase=0.03)
    for seite in (-1, 1):
        for i in range(5):
            y = seite * (0.25 + i * 0.4)
            z = 9.35 - i * 0.36
            _brett(teile, "Kappendach", dach, (4.5, 0.46, 0.08), (0.1, y, z), (seite * 42, 0, 0), fase=0.012)
    for x in (-2.0, 2.0):
        bm = bmesh.new()
        vs = [bm.verts.new((x, y, z)) for y, z in ((-1.9, 7.62), (1.9, 7.62), (0.0, 9.45))]
        bm.faces.new(vs if x > 0 else list(reversed(vs)))
        bm.faces.layers.int.new("fase")
        teile.append(objekt("Kappengiebel", bm, putz))
    _brett(teile, "First", holz, (4.7, 0.24, 0.2), (0.1, 0.0, 9.55), fase=0.02)
    # Welle aus der Kappe nach vorne bis zur Nabe
    bm = stamm_bm(0.22, 1.2, ecken=10, seed=seed)
    setzen(bm, (NABE[0] - 1.2, NABE[1], NABE[2]))
    teile.append(objekt("Welle", bm, stammfarbe(zufall)))
    # Mehlsäcke und Karre vor der Tür
    sack = einfarbig("#E3D6B6", 0.05, zufall)
    for i, (x, y) in enumerate(((3.1, 0.9), (3.3, 1.5), (2.9, -1.2))):
        bm = _kegelstumpf([(0.25, 0.0), (0.32, 0.3), (0.24, 0.62), (0.08, 0.72)], 8)
        setzen(bm, (x, y, 0.0), (0, 0, zufall.uniform(0, 90)))
        teile.append(objekt("Mehlsack", bm, sack))
    return fertig("Windmuehle", teile)


def windmuehle_fluegel(seed=72):
    """Vier Flügel mit Lattengitter und Segeltuch, Nabe im Ursprung, drehen um X."""
    zufall = random.Random(seed)
    teile = []
    holz = holzfarbe(zufall, "#B08050", "#86603A")
    tuch = einfarbig("#F4ECDA", 0.03, zufall)
    _brett(teile, "Nabe", holzfarbe(zufall, "#6E4424", "#4E2E14"), (0.5, 0.6, 0.6), (0.0, 0.0, 0.0), fase=0.04)
    for arm in range(4):
        w = math.tau * arm / 4 + 0.35
        rot = Matrix.Rotation(w, 4, "X")
        stueck = []
        _brett(stueck, "Rute", holz, (0.16, 0.16, 7.2), (0.1, 0.0, 3.6))
        for seite in (0.0, 1.0):
            _brett(stueck, "Rahmen", holz, (0.08, 0.08, 5.6), (0.1, 0.35 + seite * 0.9, 4.2))
        for i in range(7):
            _brett(stueck, "Latte", holz, (0.07, 1.3, 0.07), (0.1, 0.8, 1.5 + i * 0.9))
        _brett(stueck, "Segel", tuch, (0.03, 0.85, 5.3), (0.16, 0.8, 4.2))
        for obj in stueck:
            obj.data.transform(rot)
        teile += stueck
    # Ursprung bleibt in der Nabe (nicht unten wie sonst): um ihn dreht das Spiel die Flügel
    obj = vereinen("WindmuehleFluegel", teile)
    _material(obj)
    return obj


# ---------------------------------------------------------------------------
# Brunnen: runde Steinmauer, Dach auf zwei Pfosten, Kurbel, Eimer
# ---------------------------------------------------------------------------
def brunnen(seed=73):
    zufall = random.Random(seed)
    teile = []
    stein = _steinfarbe(zufall, farbe("#6E6960"), farbe("#948E83"), farbe("#BCB4A6"), farbe("#D4CCBF"), schicht=9.0)
    holz = holzfarbe(zufall, "#8A5A32", "#643E20")
    for reihe in range(3):
        for k in range(12):
            w = math.tau * (k + (0.5 if reihe % 2 else 0)) / 12
            bm = brett_bm(0.62, 0.36, 0.3, fase=0.03)
            setzen(bm, (0, 0, 0), (0, 0, math.degrees(w) + 90))
            setzen(bm, (math.cos(w) * 1.0, math.sin(w) * 1.0, 0.15 + reihe * 0.3))
            teile.append(objekt("Brunnenstein", bm, stein))
    bm = _kegelstumpf([(0.85, 0.3), (0.85, 0.32)], 12)
    teile.append(objekt("Wasser", bm, einfarbig("#2E6F8E", 0.03, zufall)))
    for y in (-1.15, 1.15):
        _brett(teile, "Pfosten", holz, (0.16, 0.16, 2.4), (0.0, y, 1.2), fase=0.02)
    _brett(teile, "Querbalken", holz, (0.14, 2.5, 0.14), (0.0, 0.0, 2.3), fase=0.02)
    dach = holzfarbe(zufall, "#A8472F", "#7C3220", maserung=9.0)
    for seite in (-1, 1):
        for i in range(3):
            _brett(teile, "Dach", dach, (0.5, 2.9, 0.07), (seite * (0.25 + i * 0.42), 0.0, 2.95 - i * 0.3), (0, seite * 36, 0), fase=0.012)
    bm = stamm_bm(0.12, 1.9, ecken=8, seed=seed)
    setzen(bm, (0, 0, 0), (0, 0, 90))
    setzen(bm, (0.0, -0.95, 1.75))
    teile.append(objekt("Welle", bm, stammfarbe(zufall)))
    _brett(teile, "Kurbel", einfarbig("#3A3634", 0.05, zufall), (0.05, 0.05, 0.4), (0.0, 1.2, 1.6))
    _brett(teile, "Seil", einfarbig("#C8B58A", 0.03, zufall), (0.03, 0.03, 0.6), (0.0, 0.0, 1.4))
    bm = _kegelstumpf([(0.16, 1.0), (0.2, 1.3)], 8)
    teile.append(objekt("Eimer", bm, holzfarbe(zufall, "#9A6B3F", "#6E4826")))
    return fertig("Brunnen", teile)


# ---------------------------------------------------------------------------
# Bienenstöcke: drei Strohkörbe auf einer Bank, Blumen drumherum
# ---------------------------------------------------------------------------
def bienenstoecke(seed=74):
    zufall = random.Random(seed)
    teile = []
    holz = holzfarbe(zufall, "#8A5A32", "#643E20")
    stroh = einfarbig("#D9B25A", 0.05, zufall)
    _brett(teile, "Bank", holz, (0.6, 2.6, 0.08), (0.0, 0.0, 0.6), fase=0.01)
    for y in (-1.1, 1.1):
        for x in (-0.22, 0.22):
            _brett(teile, "Bein", holz, (0.07, 0.07, 0.6), (x, y, 0.3))
    for i, y in enumerate((-0.85, 0.0, 0.85)):
        profil = [(0.3, 0.64), (0.34, 0.8), (0.32, 0.95), (0.26, 1.08), (0.16, 1.17), (0.02, 1.21)]
        bm = _kegelstumpf(profil, 10)
        teile.append(objekt("Korb", bm, stroh))
        for z, r in ((0.8, 0.345), (0.95, 0.325)):
            bm = _kegelstumpf([(r, z - 0.02), (r, z + 0.02)], 10, deckel=False)
            setzen(bm, (0, 0, 0))
            teile.append(objekt("Wulst", bm, einfarbig("#B8903E", 0.04, zufall)))
        _brett(teile, "Flugloch", einfarbig("#2A1E12", 0.02, zufall), (0.06, 0.12, 0.07), (0.3, 0.0, 0.69))
        for obj in teile[-4:]:
            obj.location.y += y
    blumen = ["#E85A8C", "#F2C230", "#8E6FE0", "#F07A3A", "#FFFFFF"]
    for i in range(22):
        x, y = zufall.uniform(-1.4, 1.6), zufall.uniform(-1.8, 1.8)
        if abs(x) < 0.5 and abs(y) < 1.4:
            continue
        _brett(teile, "Stiel", einfarbig("#4E8A3A", 0.05, zufall), (0.03, 0.03, 0.35), (x, y, 0.17))
        bm = stein_bm(zufall, 0.07, flach=0.5)
        setzen(bm, (x, y, 0.37))
        teile.append(objekt("Blüte", bm, einfarbig(zufall.choice(blumen), 0.05, zufall)))
    return fertig("Bienenstoecke", teile)


# ---------------------------------------------------------------------------
# Weizenfeld: Beet mit Reihen goldener Halme
# ---------------------------------------------------------------------------
def weizenfeld(seed=75, laenge=9.0, breite=6.0):
    zufall = random.Random(seed)
    teile = []
    _brett(teile, "Acker", einfarbig("#7A5A3A", 0.05, zufall), (laenge + 0.6, breite + 0.6, 0.2), (0.0, 0.0, 0.02))
    gold = [farbe("#E8C45A"), farbe("#D9A93E"), farbe("#F0D57A")]
    bm = bmesh.new()
    bm.faces.layers.int.new("fase")
    for ix in range(int(laenge / 0.62)):
        for iy in range(int(breite / 0.62)):
            cx = -laenge / 2 + 0.31 + ix * 0.62 + zufall.uniform(-0.12, 0.12)
            cy = -breite / 2 + 0.31 + iy * 0.62 + zufall.uniform(-0.12, 0.12)
            h = zufall.uniform(0.85, 1.15)
            for k in range(3):
                w = zufall.uniform(0, math.tau)
                dx, dy = math.cos(w) * 0.09, math.sin(w) * 0.09
                kx, ky = zufall.uniform(-0.12, 0.12), zufall.uniform(-0.12, 0.12)
                a = bm.verts.new((cx - dx, cy - dy, 0.1))
                b = bm.verts.new((cx + dx, cy + dy, 0.1))
                c = bm.verts.new((cx + kx, cy + ky, 0.1 + h))
                bm.faces.new((a, b, c))
                # Rückseite mit eigenen Eckpunkten (dieselben drei gibt es nur einmal als Fläche)
                a2, b2, c2 = (bm.verts.new(v.co) for v in (a, b, c))
                bm.faces.new((b2, a2, c2))
            # Ähre oben
            top = Vector((cx, cy, 0.1 + h * 0.95))
            ring = [bm.verts.new(top + Vector((math.cos(math.tau * k / 3) * 0.05, math.sin(math.tau * k / 3) * 0.05, 0.0))) for k in range(3)]
            spitze = bm.verts.new(top + Vector((0, 0, 0.28)))
            fuss = bm.verts.new(top - Vector((0, 0, 0.08)))
            for k in range(3):
                bm.faces.new((ring[k], ring[(k + 1) % 3], spitze))
                bm.faces.new((ring[(k + 1) % 3], ring[k], fuss))
    teile.append(objekt("Weizen", bm, lambda poly, fase=False: gold[int(abs(poly.center.x * 7 + poly.center.y * 3)) % 3] * zufall.uniform(0.9, 1.1)))
    return fertig("Weizenfeld", teile)


# ---------------------------------------------------------------------------
# Vogelscheuche
# ---------------------------------------------------------------------------
def vogelscheuche(seed=76):
    zufall = random.Random(seed)
    teile = []
    holz = holzfarbe(zufall, "#7A5230", "#553820")
    _brett(teile, "Pfahl", holz, (0.1, 0.1, 2.3), (0.0, 0.0, 1.15))
    _brett(teile, "Arme", holz, (0.08, 1.8, 0.08), (0.0, 0.0, 1.6))
    _brett(teile, "Hemd", einfarbig("#B8403A", 0.05, zufall), (0.36, 0.7, 0.8), (0.0, 0.0, 1.35), fase=0.04)
    for y in (-1, 1):
        _brett(teile, "Ärmel", einfarbig("#A83A34", 0.05, zufall), (0.28, 0.6, 0.26), (0.0, y * 0.62, 1.6), fase=0.03)
        _brett(teile, "Stroh", einfarbig("#E0BE5E", 0.06, zufall), (0.12, 0.18, 0.18), (0.0, y * 0.98, 1.6), (0, 0, 0), fase=0.02)
    _brett(teile, "Hose", einfarbig("#3F5E8A", 0.05, zufall), (0.3, 0.6, 0.35), (0.0, 0.0, 0.82), fase=0.03)
    bm = stein_bm(zufall, 0.24, flach=1.0)
    setzen(bm, (0.0, 0.0, 2.05))
    teile.append(objekt("Kopf", bm, einfarbig("#D8C49A", 0.04, zufall)))
    bm = _kegelstumpf([(0.42, 2.18), (0.42, 2.22), (0.2, 2.24), (0.16, 2.5), (0.02, 2.52)], 10)
    teile.append(objekt("Hut", bm, einfarbig("#8A6A3A", 0.05, zufall)))
    for dy in (-0.08, 0.08):
        _brett(teile, "Auge", einfarbig("#2A2018", 0.02, zufall), (0.03, 0.05, 0.05), (0.24, dy, 2.1))
    _brett(teile, "Mund", einfarbig("#2A2018", 0.02, zufall), (0.03, 0.16, 0.03), (0.23, 0.0, 1.97))
    return fertig("Vogelscheuche", teile)
