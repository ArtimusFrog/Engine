"""Weitere Wirtschaftsgebäude der Siedlung: Lehmgrube und Kristallturm.

Gleicher Stil und Werkzeugkasten wie Holzfäller, Steinbruch und Erzmine (siedlung.py): einzeln
gesetzte Steine und Bretter, Schindeldächer, Laternen, Kisten. Ursprung am Boden in der Mitte,
Vorderseite nach −Y (im Spiel nach +Z).

- Lehmgrube: hinten ein Wall aus hellem Lehm mit frisch abgestochener Wand und einer nassen
  Grube davor (Leiter, Bohlen, Pfütze), links ein offener Trockenschuppen mit Regalen voller
  Lehmziegel, rechts ein runder Ziegelofen mit Glut; vorne gebrannte Ziegel, Schubkarre mit Lehm,
  Streichtisch mit Holzformen, Wassertrog und Spaten.
- Kristallturm: schlanker Magierturm aus violettblauem Stein mit Spitzdach, leuchtenden Fenstern
  und Runenbändern; über der Spitze schwebt ein großer Kristall mit kleineren Splittern. Vorne ein
  leuchtender Runenkreis, daneben ein Kristallsockel (Arbeitsplatz der Magier), Kristalle in
  Kisten, ein Lesepult und Wimpel.
"""

import math

import bmesh
from mathutils import Matrix, Vector

from bauten import _laterne, _platte, _schild
from lager import einfarbig, holzfarbe, objekt, setzen, stammfarbe, stein_bm
from siedlung import busch, fass, fertig, rad, setze, tanne, zaun
from tuerme import GOLD, Werk, _kiste, fahne, kristalle, mauerring, schindeldach, steinfarbe, wandbanner
from vorkommen import _brocken_bm, _rauschen, _setzen, farbe


# ---------------------------------------------------------------------------
# Lehmgrube
# ---------------------------------------------------------------------------
def _lehmziegel(w, farbe_von, ort, drehung_z=0.0, gebrannt=False):
    w.brett("Ziegel" if gebrannt else "Lehmziegel", farbe_von, (0.3, 0.15, 0.08), ort, (0, 0, drehung_z + w.z.uniform(-4, 4)), fase=0.012)


def lehmgrube(seed=65):
    w = Werk(seed)
    teile, licht, zufall = w.teile, w.licht, w.z
    ocker, ton, rost = farbe("#E0B878"), farbe("#A8B6BC"), farbe("#CC9A64")
    gras = farbe("#6A9A48")
    narbe = _rauschen(seed + 3)

    def lehm(poly, fase=False):
        p, n = poly.center, poly.normal
        if n.y < -0.75:
            return rost * zufall.uniform(0.95, 1.02)                         # frisch abgestochene Wand
        c = ocker if p.z > 0.35 else ton
        c = farbe("#8A6A4C").lerp(c, min(1.0, 0.6 + 0.4 * (n.z + 0.4)))
        if n.z > 0.75 and p.z > 1.0 and narbe(p) > 0.5:
            c = gras * (0.9 + 0.15 * narbe(p * 2.0))
        return c * zufall.uniform(0.97, 1.03)

    erde = einfarbig("#9C8468", 0.05, zufall)
    _platte(teile, "Hof", erde, 7.4, 6.8, 0.05, zufall)

    # Lehmwall hinten: weiche, helle Hügel, vorne senkrecht abgestochen
    for i, (x, y, r, hoch) in enumerate(((0.0, 4.4, 2.6, 1.05), (-3.1, 4.0, 2.1, 0.95), (3.2, 4.1, 2.2, 0.98), (-1.6, 5.8, 2.0, 0.85), (1.8, 5.9, 2.1, 0.88))):
        bm = _brocken_bm(zufall, r, (1.15, 0.9, hoch), 3, tiefe=(0.8, 0.97), oben=0.1, fase=0.0)
        normale = Vector((0.0, -1.0, 0.15)).normalized()
        bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=normale * r * 0.55, plane_no=normale, clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
        bmesh.ops.triangulate(bm, faces=bm.faces[:])
        _setzen(bm, (x, y, r * hoch * 0.45 - 0.2), 0.0)
        obj = objekt(f"Lehmwall{i}", bm, lehm)
        for poly in obj.data.polygons:
            poly.use_smooth = True
        teile.append(obj)
    # Nasse Grube vor der Wand: dunkler Lehmboden, Pfütze, Bohlen, Leiter
    w.dreh("Grubenboden", einfarbig("#8C7A62", 0.04, zufall), [(0.0, 0.07), (2.4, 0.07), (2.6, 0.02)], (0.0, 2.2, 0.0), ecken=14)
    w.dreh("Grubenwasser", einfarbig("#7C98A6", 0.03, zufall), [(0.0, 0.085), (0.9, 0.085)], (0.8, 2.4, 0.0), ecken=10)
    bohle = holzfarbe(zufall, "#9A7048", "#6E4A2A")
    for k in range(4):
        w.brett("Bohle", bohle, (0.35, 2.6, 0.06), (-1.2 + k * 0.4, 1.2, 0.1), (0, 0, zufall.uniform(-3, 3)), fase=0.01)
    leiter = holzfarbe(zufall, "#A87A48", "#7A5430")
    for s in (-1, 1):
        w.saeule("Leiterholm", leiter, 0.04, (1.9 + s * 0.22, 2.9, 0.0), (1.9 + s * 0.22, 3.4, 2.1), ecken=5)
    for k in range(6):
        z = 0.3 + k * 0.32
        t = z / 2.1
        w.saeule("Leitersprosse", leiter, 0.025, (1.68, 2.9 + 0.5 * t, z), (2.12, 2.9 + 0.5 * t, z), ecken=4)
    # Spaten im Lehm und abgestochene Soden am Grubenrand
    stiel = holzfarbe(zufall, "#C08A4E", "#94663A")
    eisen = einfarbig("#6A6E76", 0.04, zufall)
    w.saeule("Spatenstiel", stiel, 0.035, (-1.6, 3.0, 0.2), (-1.75, 3.1, 1.35), ecken=6)
    w.brett("Spatengriff", stiel, (0.22, 0.05, 0.05), (-1.76, 3.1, 1.38), fase=0.0)
    w.brett("Spatenblatt", eisen, (0.2, 0.03, 0.28), (-1.58, 2.99, 0.15), (0, 0, 0), fase=0.005)
    for k in range(6):
        w.brett("Sode", einfarbig("#D8B070", 0.05, zufall), (0.28, 0.18, 0.13), (-2.6 + zufall.uniform(-0.4, 0.4), 1.9 + zufall.uniform(-0.3, 0.3), 0.1 + 0.13 * (k // 3)),
                (0, 0, zufall.uniform(0, 90)), fase=0.02)

    # Trockenschuppen links: Pfosten, Schindeldach, drei Regalböden voller Lehmziegel
    lehmziegel = einfarbig("#D8B27A", 0.05, zufall)
    ziegel = einfarbig("#B85A3A", 0.07, zufall)

    def schuppen(t):
        holz = holzfarbe(t.z, "#8E6238", "#5E3F22")
        dach = holzfarbe(t.z, "#7E6A4A", "#5A4A34", maserung=9.0)
        for x in (-1.4, 1.4):
            for y in (-0.8, 0.8):
                t.brett("Schuppenpfosten", holz, (0.15, 0.15, 2.6 if y > 0 else 2.2), (x, y, 1.3 if y > 0 else 1.1), fase=0.02)
        for z in (0.45, 1.0, 1.55):
            t.brett("Regalboden", holz, (2.9, 1.5, 0.05), (0, 0, z), fase=0.01)
            for i in range(9):
                for j in range(3):
                    _lehmziegel(t, lehmziegel, (-1.2 + i * 0.3, -0.45 + j * 0.45, z + 0.07), 90)
        neigung = math.degrees(math.atan2(0.4, 1.9))
        for r in range(5):
            t.brett("Schuppendach", dach, (3.4, 0.5, 0.05), (0, -1.05 + r * 0.45, 2.25 + r * 0.1), (-neigung, 0, 0), fase=0.01)
    setze(w, (-4.7, -0.6, 0.0), 90, schuppen)

    # Ziegelofen rechts: runder Kuppelofen aus Ziegeln mit glühender Öffnung und Schornstein
    def ofen(t):
        ofenziegel = einfarbig("#A85A3E", 0.07, t.z)
        mauerring(t, ofenziegel, 1.2, 1.0, 0.0, 1.3, h=0.2, tiefe=0.3, kern="#5A3426")
        t.dreh("Ofenkuppel", ofenziegel, [(1.1, 1.3), (0.95, 1.75), (0.6, 2.05), (0.0, 2.15)], (0, 0, 0), ecken=14)
        t.brett("Ofenmund", einfarbig("#FF9A3A", 0.05, t.z), (0.6, 0.1, 0.55), (0, -1.12, 0.45), fase=0.0, glut=True)
        t.brett("Ofensturz", ofenziegel, (0.95, 0.35, 0.18), (0, -1.12, 0.82), fase=0.02)
        for s in (-1, 1):
            t.brett("Ofenwange", ofenziegel, (0.2, 0.35, 0.8), (s * 0.4, -1.12, 0.4), fase=0.02)
        t.saeule("Schornstein", ofenziegel, 0.22, (0.4, 0.4, 1.6), (0.45, 0.45, 3.0), ecken=6)
        t.dreh("Schornsteinkopf", ofenziegel, [(0.28, 3.0), (0.28, 3.12), (0.18, 3.12)], (0.45, 0.45, 0), ecken=6)
        # Brennholz
        rinde = stammfarbe(t.z, "#6E4A2A", "#4E321A", "#C49A62", "#96703E")
        for k in range(6):
            t.stamm("Brennholz", rinde, 0.08, 0.7, (1.35, -0.3 + (k % 3) * 0.17, 0.08 + (k // 3) * 0.15), (0, 0, 90), ecken=6)
    setze(w, (4.4, 0.4, 0.0), 0, ofen)

    # Vorne: Stapel gebrannter Ziegel, Streichtisch mit Formen, Schubkarre voll Lehm, Wassertrog
    for stapel, (sx, sy) in enumerate(((2.0, -3.4), (3.6, -3.1))):
        for lage in range(7 - stapel * 2):
            for i in range(4):
                for j in range(3):
                    _lehmziegel(w, ziegel, (sx + i * 0.32, sy + j * 0.17 + (0.08 if lage % 2 else 0.0), 0.04 + lage * 0.085), 0, gebrannt=True)
    tisch = holzfarbe(zufall, "#A87A48", "#7A5430")
    for x in (-1.8, -0.4):
        for y in (-3.3, -2.7):
            w.brett("Tischbein", tisch, (0.1, 0.1, 0.85), (x, y, 0.42), fase=0.01)
    w.brett("Streichtisch", tisch, (1.7, 0.8, 0.08), (-1.1, -3.0, 0.88), fase=0.015)
    for k in range(3):
        x = -1.6 + k * 0.45
        for s in (-1, 1):
            w.brett("Ziegelform", tisch, (0.34, 0.03, 0.1), (x, -3.0 + s * 0.1, 0.97), fase=0.0)
            w.brett("Ziegelform", tisch, (0.03, 0.2, 0.1), (x + s * 0.17, -3.0, 0.97), fase=0.0)
        w.brett("Lehm in Form", lehmziegel, (0.3, 0.17, 0.06), (x, -3.0, 0.95), fase=0.0)
    w.dreh("Lehmklumpen", einfarbig("#D8B070", 0.05, zufall), [(0.0, 0.92), (0.22, 0.95), (0.18, 1.1), (0.0, 1.14)], (-0.3, -3.1, 0), ecken=8)

    def karre(t):
        holz = holzfarbe(t.z, "#9A6B3F", "#6E4826")
        rad(t, (0.75, 0, 0.3), 0.3, speichen=8, rz=90)
        for s in (-1, 1):
            t.saeule("Karrenholm", holz, 0.04, (0.8, s * 0.3, 0.3), (-0.85, s * 0.35, 0.65), ecken=5)
            t.brett("Karrenbein", holz, (0.06, 0.06, 0.4), (-0.4, s * 0.3, 0.2), fase=0.0)
        t.dreh("Karrenmulde", holz, [(0.2, 0.35), (0.45, 0.72), (0.4, 0.72), (0.16, 0.38)], (0.05, 0, 0), ecken=8)
        for k in range(5):
            t.stein("Karrenlehm", einfarbig("#D8B070", 0.06, t.z), 0.13, (0.05 + t.z.uniform(-0.2, 0.2), t.z.uniform(-0.18, 0.18), 0.72), flach=0.7)
    setze(w, (0.9, -2.2, 0.0), 30, karre)
    w.dreh("Wassertrog", holzfarbe(zufall, "#8E6238", "#5E3F22"), [(0.0, 0.0), (0.55, 0.0), (0.6, 0.5), (0.52, 0.5), (0.48, 0.08), (0.0, 0.08)], (-3.2, -3.6, 0), ecken=10)
    w.dreh("Trogwasser", einfarbig("#6F9AB2", 0.03, zufall), [(0.0, 0.42), (0.5, 0.42)], (-3.2, -3.6, 0), ecken=10)

    # Schild mit Ziegel, Laternen, Zaun, Büsche
    _schild(teile, zufall, (-5.6, -3.6, 0.0), 0, [
        (0.0, 0.05, 0.4, 0.2, 0, "#B85A3A"), (0.0, -0.13, 0.4, 0.12, 0, "#D8B27A"), (-0.15, 0.18, 0.25, 0.04, 30, "#6E4A2A"),
    ])
    for x in (-2.4, 2.8):
        _laterne(teile, licht, zufall, (x, -4.3, 0.0), hoehe=2.2)
    zaun(w, [(-6.8, 1.8), (-6.8, 5.6), (-4.6, 6.6)])
    zaun(w, [(6.6, 2.2), (6.6, 5.2), (4.8, 6.6)])
    busch(w, (-6.0, -2.0), 0.55, blueten="#F3D24A")
    busch(w, (6.3, -2.8), 0.5)
    fass(w, (5.9, -1.2, 0.0), 0.9)
    _kiste(w, (-4.4, -3.9, 0.0), 0.55)
    return fertig(w, "Lehmgrube")


# ---------------------------------------------------------------------------
# Kristallturm
# ---------------------------------------------------------------------------
KRISTALL = ("#1D4FB8", "#3C9CFF", "#D2F1FF")


def kristallturm(seed=66):
    w = Werk(seed)
    teile, licht, zufall = w.teile, w.licht, w.z
    stein = steinfarbe(w, "#4A4868", "#66648C", "#8886B0", "#A6A4CC")
    hell = steinfarbe(w, "#8C8AA8", "#B0AECC", "#CFCDE6", "#E6E4F4")
    blau = einfarbig("#7FD0FF", 0.05, zufall)
    _platte(teile, "Vorplatz", einfarbig("#A8A49A", 0.05, zufall), 7.0, 6.6, 0.05, zufall)

    # Turmschaft: Sockel, drei Geschosse mit Gesimsen, leuchtende Spitzbogenfenster, Runenband
    tx, ty = 0.0, 1.6
    w.dreh("Turmsockel", hell, [(2.6, -0.4), (2.6, 0.5), (2.35, 0.7), (0.0, 0.7)], (tx, ty, 0), ecken=16)

    def schaft(t):
        mauerring(t, stein, 2.05, 1.7, 0.7, 7.6, h=0.46, tiefe=0.42, kern="#34324C")
        for z in (3.0, 5.4):
            t.dreh("Gesims", hell, [(1.95, z), (2.2, z + 0.12), (2.15, z + 0.26), (1.9, z + 0.3)], (0, 0, 0), ecken=18)
        t.dreh("Kranz", hell, [(1.75, 7.5), (2.35, 7.7), (2.35, 7.95), (1.7, 8.0)], (0, 0, 0), ecken=18)
        # Fenster (leuchtend, Spitzbogen aus zwei Steinen)
        for z, anzahl, versatz in ((2.0, 3, 0.5), (4.3, 4, 0.0), (6.5, 4, 0.5)):
            for k in range(anzahl):
                wi = math.tau * (k + versatz) / anzahl + (math.pi / 2 if anzahl == 3 else 0.0)
                if anzahl == 3 and abs(math.sin(wi) + 1) < 0.3:
                    continue
                r = 1.98 - (z - 0.7) / 6.9 * 0.35
                p = (math.cos(wi) * r, math.sin(wi) * r)
                t.brett("Fenster", blau, (0.42, 0.06, 0.8), (p[0], p[1], z), (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)
                for s in (-1, 1):
                    q = (math.cos(wi) * (r + 0.05), math.sin(wi) * (r + 0.05))
                    t.brett("Fensterbogen", hell, (0.28, 0.12, 0.1), (q[0] - math.sin(wi) * s * 0.12, q[1] + math.cos(wi) * s * 0.12, z + 0.47), (s * 35, 0, math.degrees(wi) + 90), fase=0.01)
                t.brett("Fensterbank", hell, (0.6, 0.18, 0.08), (math.cos(wi) * (r + 0.07), math.sin(wi) * (r + 0.07), z - 0.44), (0, 0, math.degrees(wi) + 90), fase=0.01)
        # Leuchtendes Runenband zwischen den Geschossen
        for k in range(18):
            wi = math.tau * k / 18
            r = 1.93
            t.brett("Rune", blau if k % 3 else einfarbig("#B69CFF", 0.04, t.z), (0.16, 0.04, 0.22), (math.cos(wi) * r, math.sin(wi) * r, 3.5 + 0.05 * (k % 2)),
                    (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)
    setze(w, (tx, ty, 0.0), 0, schaft)

    # Spitzdach aus Schieferschindeln, Knauf, darüber der schwebende Kristall mit Splittern
    def dach(t):
        schindeldach(t, einfarbig("#3A3F7A", 0.06, t.z), 2.05, 7.95, 3.3, ueberstand=0.4, spitze_farbe=GOLD)
        # Kleine Gaube mit rundem Fenster
        t.brett("Gaube", stein, (0.8, 0.9, 0.9), (0, -1.75, 8.45), fase=0.02)
        t.dreh("Gaubenfenster", blau, [(0.0, 0.0), (0.22, 0.0)], (0, -2.21, 8.5), ecken=10, drehung=(90, 0, 0), glut=True)
        t.spitze("Gaubendach", einfarbig("#3A3F7A", 0.06, t.z), 0.62, 0.55, (0, -1.75, 8.9), ecken=4, drehung=(0, 0, 45))
    setze(w, (tx, ty, 0.0), 0, dach)
    kristalle(w, (tx, ty, 12.6), (0, 0, 1), 2.4, KRISTALL)
    kristalle(w, (tx, ty, 12.65), (0, 0, -1), 1.3, KRISTALL)
    for k in range(6):
        wi = math.tau * k / 6
        kristalle(w, (tx + math.cos(wi) * 1.9, ty + math.sin(wi) * 1.9, 12.4 + 0.4 * math.sin(wi * 2)), (math.cos(wi) * 0.4, math.sin(wi) * 0.4, 1), 0.55, KRISTALL)
    # Zwei leuchtende Ringe um den Kristall, schräg gegeneinander
    for n, kipp in enumerate((0.0, 0.35)):
        for k in range(28):
            a, b = math.tau * k / 28, math.tau * (k + 1) / 28
            pa = Vector((math.cos(a) * 2.4, math.sin(a) * 2.4 * math.cos(kipp), math.sin(a) * 2.4 * math.sin(kipp)))
            pb = Vector((math.cos(b) * 2.4, math.sin(b) * 2.4 * math.cos(kipp), math.sin(b) * 2.4 * math.sin(kipp)))
            o = Vector((tx, ty, 12.8 + n * 0.1))
            w.saeule("Arkanring", einfarbig("#9FE0FF", 0.03, zufall), 0.04, o + pa, o + pb, ecken=4, glut=True)

    # Tür mit Spitzbogen, Stufen, Wappen und Wandbannern
    tuer_y = ty - 2.02
    w.brett("Tür", holzfarbe(zufall, "#4A3A5E", "#2E2440"), (1.0, 0.12, 1.9), (tx, tuer_y, 1.65), fase=0.01)
    for s in (-1, 1):
        w.brett("Türbogen", hell, (0.62, 0.3, 0.2), (tx + s * 0.3, tuer_y - 0.05, 2.75), (0, s * -35, 0), fase=0.02)
        w.brett("Türgewände", hell, (0.25, 0.3, 2.0), (tx + s * 0.62, tuer_y - 0.05, 1.7), fase=0.02)
    w.dreh("Türring", einfarbig(GOLD, 0.03, zufall), [(0.08, 0.0), (0.1, 0.02)], (tx + 0.25, tuer_y - 0.08, 1.55), ecken=8, drehung=(90, 0, 0))
    for k in range(3):
        w.brett("Stufe", hell, (1.8 - k * 0.2, 0.45, 0.24), (tx, tuer_y - 0.8 + k * 0.3, 0.12 + k * 0.2), fase=0.02)
    for s in (-1, 1):
        wandbanner(w, (tx + s * 1.25, tuer_y + 0.22, 5.1), s * 20, breite=0.7, hoehe=1.5, tuch="#3A4FB0", zeichen="#9FE0FF")

    # Runenkreis vor dem Turm (leuchtende Steine im Boden)
    rx, ry = -2.6, -2.6
    w.dreh("Runenkreis", einfarbig("#6E6A86", 0.04, zufall), [(0.0, 0.07), (1.5, 0.07), (1.6, 0.02)], (rx, ry, 0), ecken=16)
    for k in range(8):
        wi = math.tau * k / 8
        w.brett("Kreisrune", blau, (0.28, 0.14, 0.05), (rx + math.cos(wi) * 1.2, ry + math.sin(wi) * 1.2, 0.09), (0, 0, math.degrees(wi)), fase=0.0, glut=True)
        w.stein("Kreisstein", stein, 0.22, (rx + math.cos(wi + 0.4) * 1.75, ry + math.sin(wi + 0.4) * 1.75, 0.1), flach=0.9)
    w.dreh("Kreismitte", blau, [(0.0, 0.1), (0.35, 0.1)], (rx, ry, 0), ecken=8, glut=True)

    # Kristallsockel rechts (Arbeitsplatz der Magier, wenn draußen nichts zu holen ist)
    sx, sy = 3.3, -1.2
    w.dreh("Kristallsockel", hell, [(0.7, 0.0), (0.6, 0.3), (0.35, 0.45), (0.4, 0.95), (0.55, 1.05), (0.0, 1.05)], (sx, sy, 0), ecken=8)
    kristalle(w, (sx, sy, 1.0), (0, 0, 1), 0.9, KRISTALL)
    # Kisten mit Kristallen, Lesepult mit Buch, Wimpel, Laternen
    for (x, y) in ((4.8, -3.4), (5.5, -2.6)):
        _kiste(w, (x, y, 0.0), 0.6)
        kristalle(w, (x, y, 0.6), (0.2, 0.1, 1), 0.35, KRISTALL)
    holz = holzfarbe(zufall, "#6A4A6E", "#46304A")
    w.saeule("Pultfuss", holz, 0.07, (-4.6, 0.6, 0.0), (-4.6, 0.6, 1.0), ecken=6)
    w.brett("Pult", holz, (0.7, 0.5, 0.06), (-4.6, 0.55, 1.08), (-25, 0, 0), fase=0.01)
    w.brett("Buch", einfarbig("#E9D9B0", 0.03, zufall), (0.55, 0.38, 0.05), (-4.6, 0.52, 1.14), (-25, 0, 0), fase=0.005)
    for x in (-4.4, 4.4):
        fahne(w, (x, 3.2, 0.0), hoehe=3.6, tuch="#3A4FB0", breite=0.7, tuch_hoehe=0.5)
    for x in (-1.9, 1.9):
        _laterne(teile, licht, zufall, (x, -4.2, 0.0), hoehe=2.2)
    tanne(w, (-5.6, 4.3), 3.0)
    tanne(w, (5.3, 4.6), 2.6)
    busch(w, (-5.8, -3.8), 0.5, blueten="#9FC8FF")
    kristalle(w, (-5.3, 2.2, 0.0), (0.3, 0.1, 1), 0.55, KRISTALL)
    kristalle(w, (5.7, 1.0, 0.0), (-0.2, 0.2, 1), 0.45, KRISTALL)
    return fertig(w, "Kristallturm")
