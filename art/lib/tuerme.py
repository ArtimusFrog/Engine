"""Verteidigungstürme für die Heerstraßen (Tower Defense): fünfzehn Arten in je drei Stufen.

Jeder Turm besteht aus zwei Modellen:
- `turm_<art>_<stufe>`: der feste Turm bis zur Plattform. Die Plattform liegt bei `KOPF_Z[stufe]`,
  jede Stufe ist höher, reicher gebaut und verziert.
- `turm_<art>_kopf`: das bewegliche Oberteil (Armbrust, Balliste, Katapultarm, Kristall …), Fuß im
  Ursprung. Das Spiel setzt es auf die Plattform und dreht es zum Ziel. Die Vorderseite zeigt nach −Y.

Gebaut wie die Wirtschaftsgebäude (lager.py/bauten.py): einzeln gesetzte Mauersteine mit Fase und
Farbschwankung, Bohlen, Rundhölzer, Schindeln, Eisenbeschläge, Tuch, leuchtende Kristalle und Glut.
Hell und freundlich – die Spieler verteidigen die Insel gegen die düstere Festung.
Stufe 1: schlicht, Stufe 2: Banner, Laternen, Beschläge, Stufe 3: Gold, Wimpel, Leuchten.
"""

import math
import random

import bmesh
from mathutils import Matrix, Vector

from bauten import _fertig, _laterne
from burg import Bau, M, drehkoerper, platte, pyramide, quader, zylinder
from lager import brett_bm, einfarbig, holzfarbe, objekt, setzen, stamm_bm, stammfarbe, stein_bm
from vorkommen import _kristallbueschel_mid, _rauschen, _steinfarbe, farbe

KOPF_Z = {1: 4.2, 2: 5.4, 3: 6.6}

GOLD = "#D8AE4A"
EISEN = "#45484F"
TUCH_ROT = "#B23A3A"
TUCH_BLAU = "#3F6FB5"


# ---------------------------------------------------------------------------
# Werkzeugkasten
# ---------------------------------------------------------------------------
class Werk:
    """Sammelt Teile und leuchtende Teile eines Modells."""

    def __init__(self, seed):
        self.z = random.Random(seed)
        self.teile = []
        self.licht = []

    def bm(self, name, bm, farbe_von, glut=False):
        (self.licht if glut else self.teile).append(objekt(name, bm, farbe_von))

    def brett(self, name, farbe_von, masse, ort, drehung=(0, 0, 0), fase=0.02, glut=False):
        bm = brett_bm(*masse, fase=fase)
        setzen(bm, (0, 0, 0), drehung)
        setzen(bm, ort)
        self.bm(name, bm, farbe_von, glut)

    def stamm(self, name, farbe_von, radius, laenge, ort, drehung=(0, 0, 0), ecken=8, radius_ende=None):
        """Rundholz mit der Mitte bei `ort` (ohne Drehung entlang X)."""
        bm = stamm_bm(radius, laenge, ecken=ecken, radius_ende=radius_ende, seed=self.z.randint(1, 9999), knorrig=0.05)
        setzen(bm, (-laenge / 2, 0, 0))
        setzen(bm, (0, 0, 0), drehung)
        setzen(bm, ort)
        self.bm(name, bm, farbe_von)

    def saeule(self, name, farbe_von, radius, von, bis, ecken=8, glut=False, radius_ende=None):
        """Rundes Stück von `von` nach `bis` (Pfosten, Rohr, Stange)."""
        von, bis = Vector(von), Vector(bis)
        achse = bis - von
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=ecken, radius1=radius, radius2=radius if radius_ende is None else radius_ende,
                              depth=achse.length)
        dreh = Vector((0, 0, 1)).rotation_difference(achse.normalized()).to_matrix().to_4x4()
        bmesh.ops.transform(bm, matrix=Matrix.Translation((von + bis) / 2) @ dreh, verts=bm.verts)
        bm.faces.layers.int.new("fase")
        self.bm(name, bm, farbe_von, glut)

    def spitze(self, name, farbe_von, radius, hoehe, ort, ecken=4, drehung=(0, 0, 0), glut=False):
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=ecken, radius1=radius, radius2=0.0, depth=hoehe)
        setzen(bm, (0, 0, hoehe / 2))
        setzen(bm, (0, 0, 0), drehung)
        setzen(bm, ort)
        bm.faces.layers.int.new("fase")
        self.bm(name, bm, farbe_von, glut)

    def dreh(self, name, farbe_von, profil, ort, ecken=10, drehung=(0, 0, 0), glut=False):
        """Rotationskörper aus (Radius, Höhe)-Paaren (Schalen, Kuppeln, Knäufe)."""
        bm = bmesh.new()
        ringe = []
        for r, z in profil:
            if r < 1e-4:
                ringe.append([bm.verts.new((0, 0, z))])
            else:
                ringe.append([bm.verts.new((math.cos(math.tau * k / ecken) * r, math.sin(math.tau * k / ecken) * r, z)) for k in range(ecken)])
        for a, b in zip(ringe, ringe[1:]):
            if len(a) == 1 and len(b) == 1:
                continue
            if len(a) == 1:
                for k in range(ecken):
                    bm.faces.new((a[0], b[k], b[(k + 1) % ecken]))
            elif len(b) == 1:
                for k in range(ecken):
                    bm.faces.new((a[k], a[(k + 1) % ecken], b[0]))
            else:
                for k in range(ecken):
                    bm.faces.new((a[k], a[(k + 1) % ecken], b[(k + 1) % ecken], b[k]))
        if len(ringe[0]) > 1:
            bm.faces.new(list(reversed(ringe[0])))
        if len(ringe[-1]) > 1:
            bm.faces.new(ringe[-1])
        setzen(bm, (0, 0, 0), drehung)
        setzen(bm, ort)
        bm.faces.layers.int.new("fase")
        self.bm(name, bm, farbe_von, glut)

    def stein(self, name, farbe_von, radius, ort, flach=0.6, drehung_z=0.0):
        bm = stein_bm(self.z, radius, flach=flach)
        setzen(bm, (0, 0, 0), (0, 0, drehung_z))
        setzen(bm, ort)
        self.bm(name, bm, farbe_von)

    def fertig(self, name):
        return _fertig(name, self.teile, self.licht)


def steinfarbe(w, dunkel="#6B665E", mittel="#948D82", hell="#BDB5A8", kante="#D6CFC2", moos=None):
    return _steinfarbe(w.z, farbe(dunkel), farbe(mittel), farbe(hell), farbe(kante), moos=farbe(moos) if moos else None,
                       moos_rauschen=_rauschen(w.z.randint(1, 999)) if moos else None)


def mauerring(w, farbe_von, r0, r1, z0, z1, h=0.45, tiefe=0.42, kern="#4E4A45"):
    """Runder Turmschaft aus einzeln gesetzten Mauersteinen (versetzte Lagen), innen ein dunkler Kern."""
    lagen = max(1, round((z1 - z0) / h))
    h = (z1 - z0) / lagen
    for l in range(lagen):
        z = z0 + (l + 0.5) * h
        r = r0 + (r1 - r0) * (l + 0.5) / lagen
        n = max(8, round(math.tau * r / 0.95))
        versatz = 0.5 if l % 2 else 0.0
        for k in range(n):
            wi = math.tau * (k + versatz) / n
            laenge = math.tau * r / n * w.z.uniform(0.9, 0.98)
            bm = brett_bm(laenge, tiefe * w.z.uniform(0.9, 1.05), h * w.z.uniform(0.88, 0.95), fase=0.04)
            setzen(bm, (0, 0, 0), (w.z.uniform(-1.5, 1.5), 0, math.degrees(wi) + 90))
            setzen(bm, (math.cos(wi) * r, math.sin(wi) * r, z))
            w.bm("Mauerstein", bm, farbe_von)
    kern_r0, kern_r1 = r0 - tiefe * 0.45, r1 - tiefe * 0.45
    w.dreh("Mauerkern", einfarbig(kern, 0.05, w.z), [(kern_r0, z0), (kern_r1, z1)], (0, 0, 0), ecken=16)


def mauerblock(w, farbe_von, b, d, z0, z1, h=0.44, tiefe=0.42, kern="#4E4A45", mitte=(0.0, 0.0)):
    """Rechteckiger Mauerkörper (Breite b entlang X, Tiefe d entlang Y) aus Steinlagen im Verband."""
    lagen = max(1, round((z1 - z0) / h))
    h = (z1 - z0) / lagen
    mx, my = mitte
    for l in range(lagen):
        z = z0 + (l + 0.5) * h
        for seite, (laenge, ort, rz) in enumerate(((b, (0, -d / 2 + tiefe / 2), 0), (b, (0, d / 2 - tiefe / 2), 0),
                                                   (d - tiefe * 2, (-b / 2 + tiefe / 2, 0), 90), (d - tiefe * 2, (b / 2 - tiefe / 2, 0), 90))):
            n = max(1, round(laenge / 0.95))
            versatz = 0.5 if (l + seite) % 2 else 0.0
            teil = laenge / n
            for k in range(n + (1 if versatz else 0)):
                a = max(-laenge / 2 + (k - versatz) * teil, -laenge / 2)
                e = min(-laenge / 2 + (k - versatz + 1) * teil, laenge / 2)
                if e - a < 0.1:
                    continue
                m = (a + e) / 2
                bm = brett_bm((e - a) * w.z.uniform(0.93, 0.99), tiefe * w.z.uniform(0.95, 1.05), h * w.z.uniform(0.88, 0.95), fase=0.04)
                setzen(bm, (0, 0, 0), (0, 0, rz))
                setzen(bm, (mx + ort[0] + (m if rz == 0 else 0), my + ort[1] + (m if rz == 90 else 0), z))
                w.bm("Mauerstein", bm, farbe_von)
    w.brett("Mauerkern", einfarbig(kern, 0.05, w.z), (b - tiefe, d - tiefe, z1 - z0), (mx, my, (z0 + z1) / 2), fase=0.0)


def sockelsteine(w, farbe_von, r, anzahl=None, z=0.05):
    """Kranz grober Bruchsteine um den Fuß (halb im Boden)."""
    anzahl = anzahl or max(10, round(math.tau * r / 0.7))
    for k in range(anzahl):
        wi = math.tau * k / anzahl + w.z.uniform(-0.1, 0.1)
        w.stein("Sockelstein", farbe_von, w.z.uniform(0.28, 0.4), (math.cos(wi) * r, math.sin(wi) * r, z), flach=0.7, drehung_z=w.z.uniform(0, 360))


def fundament(w, farbe_von, r, hoehe=0.9):
    """Breiter, flacher Fundamentsockel (reicht 0,6 m in den Boden) mit Abschlussplatte und Bruchsteinkranz."""
    mauerring(w, farbe_von, r, r * 0.97, -0.6, hoehe, h=0.5)
    w.dreh("Fundamentplatte", farbe_von, [(r + 0.12, hoehe), (r + 0.12, hoehe + 0.14), (r * 0.9, hoehe + 0.18)], (0, 0, 0), ecken=16)
    sockelsteine(w, farbe_von, r + 0.25)


def bohlen(w, holz, r, z, rand=None):
    """Runde Plattform aus Bohlen (Oberkante bei z), außen ein Kranz aus Balkenstücken."""
    zeilen = max(4, round(2 * r / 0.3))
    for i in range(zeilen):
        y = -r + (i + 0.5) * 2 * r / zeilen
        halb = math.sqrt(max(r * r - y * y, 0.0))
        if halb < 0.15:
            continue
        w.brett("Bohle", holz, (2 * halb * w.z.uniform(0.96, 1.0), 2 * r / zeilen * 0.94, 0.08), (w.z.uniform(-0.03, 0.03), y, z - 0.04), (0, 0, w.z.uniform(-0.6, 0.6)), fase=0.012)
    rand = rand or holz
    n = max(8, round(math.tau * r / 1.1))
    for k in range(n):
        wi = math.tau * (k + 0.5) / n
        laenge = math.tau * r / n * 1.02
        w.brett("Randbalken", rand, (laenge, 0.22, 0.24), (math.cos(wi) * r, math.sin(wi) * r, z - 0.16), (0, 0, math.degrees(wi) + 90), fase=0.02)


def steinplattform(w, farbe_von, r, z):
    """Plattform mit vorstehendem Gesims und Steinplatten (Oberkante bei z)."""
    w.dreh("Gesims", farbe_von, [(r * 0.8, z - 0.6), (r + 0.18, z - 0.3), (r + 0.18, z - 0.1), (r + 0.05, z)], (0, 0, 0), ecken=16)
    for ring, n in ((r * 0.4, 6), (r * 0.78, 12)):
        for k in range(n):
            wi = math.tau * (k + 0.5 * (n == 12)) / n
            w.brett("Bodenplatte", farbe_von, (math.tau * ring / n * 0.95, r * 0.36, 0.06), (math.cos(wi) * ring, math.sin(wi) * ring, z + 0.02),
                    (0, 0, math.degrees(wi) + 90), fase=0.02)


def zinnen(w, farbe_von, r, z, anzahl=8, hoehe=0.75, breite=0.55):
    """Zinnenkranz: je Zinne zwei aufeinandergesetzte Steine."""
    for k in range(anzahl):
        wi = math.tau * (k + 0.5) / anzahl
        for j, (hj, bj) in enumerate(((hoehe * 0.55, breite), (hoehe * 0.45, breite * 0.92))):
            w.brett("Zinne", farbe_von, (bj * w.z.uniform(0.95, 1.05), 0.4, hj), (math.cos(wi) * (r - 0.2), math.sin(wi) * (r - 0.2), z + (0 if j == 0 else hoehe * 0.55) + hj / 2),
                    (0, 0, math.degrees(wi) + 90), fase=0.035)


def gelaender(w, holz, r, z, pfosten=8, hoehe=0.95):
    for k in range(pfosten):
        wi = math.tau * k / pfosten
        w.brett("Pfosten", holz, (0.12, 0.12, hoehe), (math.cos(wi) * (r - 0.1), math.sin(wi) * (r - 0.1), z + hoehe / 2), fase=0.015)
        wi2 = math.tau * (k + 0.5) / pfosten
        laenge = math.tau * (r - 0.1) / pfosten
        for zz in (hoehe * 0.45, hoehe - 0.05):
            w.brett("Handlauf", holz, (laenge, 0.08, 0.08), (math.cos(wi2) * (r - 0.1), math.sin(wi2) * (r - 0.1), z + zz), (0, 0, math.degrees(wi2) + 90), fase=0.01)


def holzgeruest(w, breite, z0, z1, rinde, holz):
    """Vier Eckstämme mit Querriegeln und Kreuzstreben, vorne eine Leiter."""
    for sx in (-1, 1):
        for sy in (-1, 1):
            w.stamm("Eckstamm", rinde, 0.17, z1 - z0 + 0.2, (sx * breite / 2, sy * breite / 2, (z0 + z1) / 2), (0, 90, 0), ecken=9)
    etagen = max(int((z1 - z0) / 1.5), 1)
    for e in range(etagen + 1):
        z = z0 + (z1 - z0) * e / etagen
        for s in range(4):
            mitte = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((0, -breite / 2, z))
            w.stamm("Riegel", rinde, 0.1, breite + 0.3, tuple(mitte), (0, 0, 90 * s), ecken=7)
    for e in range(etagen):
        za, zb = z0 + (z1 - z0) * e / etagen, z0 + (z1 - z0) * (e + 1) / etagen
        laenge = math.hypot(breite, zb - za)
        winkel = math.degrees(math.atan2(zb - za, breite))
        for s in range(4):
            mitte = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((0, -breite / 2 - 0.05, (za + zb) / 2))
            w.brett("Strebe", holz, (laenge, 0.1, 0.14), tuple(mitte), (0, -winkel if (e + s) % 2 else winkel, 90 * s), fase=0.015)
    for x in (-0.28, 0.28):
        w.brett("Leiterholm", holz, (0.07, 0.07, z1 - z0), (x, -breite / 2 - 0.3, (z0 + z1) / 2), fase=0.01)
    for i in range(int((z1 - z0) / 0.38)):
        w.brett("Sprosse", holz, (0.56, 0.05, 0.05), (0, -breite / 2 - 0.3, z0 + 0.25 + i * 0.38), fase=0.005)


def schindeldach(w, farbe_von, r, z, hoehe, ueberstand=0.35, spitze_farbe=GOLD):
    """Kegeldach aus überlappenden Schindelreihen mit Knauf."""
    reihen = max(4, round(hoehe / 0.35))
    neigung = math.degrees(math.atan2(hoehe, r + ueberstand))
    for i in range(reihen):
        t = i / reihen
        rr = (r + ueberstand) * (1 - t)
        zz = z + hoehe * t
        n = max(5, round(math.tau * rr / 0.42))
        for k in range(n):
            wi = math.tau * (k + (0.5 if i % 2 else 0)) / n
            w.brett("Schindel", farbe_von, (math.tau * rr / n * 1.15, hoehe / reihen * 1.6, 0.05), (math.cos(wi) * rr * 0.93, math.sin(wi) * rr * 0.93, zz + 0.12),
                    (neigung, 0, math.degrees(wi) + 90), fase=0.01)
    w.dreh("Dachknauf", einfarbig(spitze_farbe, 0.03, w.z), [(0.0, z + hoehe - 0.1), (0.12, z + hoehe + 0.05), (0.08, z + hoehe + 0.25), (0.0, z + hoehe + 0.45)], (0, 0, 0), ecken=8)


def fahne(w, ort, hoehe=1.6, tuch=TUCH_ROT, breite=0.8, tuch_hoehe=0.55, mast="#6E4A2A"):
    """Mast mit wehendem, beidseitig sichtbarem Wimpel."""
    x, y, z = ort
    w.saeule("Fahnenmast", einfarbig(mast, 0.05, w.z), 0.035, (x, y, z), (x, y, z + hoehe), ecken=6)
    w.dreh("Mastknauf", einfarbig(GOLD, 0.03, w.z), [(0.0, 0.0), (0.06, 0.04), (0.0, 0.12)], (x, y, z + hoehe), ecken=6)
    bm = bmesh.new()
    spalten, zeilen = 6, 3
    phase = w.z.uniform(0, math.tau)
    verts = []
    for i in range(spalten + 1):
        u = i / spalten
        spalte = []
        for j in range(zeilen + 1):
            v = j / zeilen
            spitz = 1.0 - u * 0.55 * (1 - abs(v - 0.5) * 2)
            welle = math.sin(u * 5.0 + phase) * 0.08 * u
            spalte.append(bm.verts.new((x + u * breite, y + welle, z + hoehe - 0.05 - (1 - v) * tuch_hoehe * (0.55 + 0.45 * spitz))))
        verts.append(spalte)
    rueck = [[bm.verts.new(v.co + Vector((0, 0.012, 0))) for v in spalte] for spalte in verts]
    for i in range(spalten):
        for j in range(zeilen):
            bm.faces.new((verts[i][j], verts[i + 1][j], verts[i + 1][j + 1], verts[i][j + 1]))
            bm.faces.new((rueck[i][j + 1], rueck[i + 1][j + 1], rueck[i + 1][j], rueck[i][j]))
    w.bm("Wimpel", bm, einfarbig(tuch, 0.06, w.z))


def wandbanner(w, ort, drehung_z, breite=0.8, hoehe=1.6, tuch=TUCH_ROT, zeichen=GOLD):
    """Hängendes Banner mit Stange, Spitze unten und einem Zeichen (Vorderseite nach außen)."""
    rot = Matrix.Rotation(math.radians(drehung_z), 3, "Z")
    o = Vector(ort)
    stange = rot @ Vector((1, 0, 0))
    w.saeule("Bannerstange", einfarbig(GOLD, 0.03, w.z), 0.03, o - stange * (breite / 2 + 0.08), o + stange * (breite / 2 + 0.08), ecken=6)
    bm = bmesh.new()
    punkte = [(-breite / 2, 0), (breite / 2, 0), (breite / 2, -hoehe * 0.8), (0, -hoehe), (-breite / 2, -hoehe * 0.8)]
    vorne = [bm.verts.new(o + rot @ Vector((px, -0.02, pz))) for px, pz in punkte]
    hinten = [bm.verts.new(o + rot @ Vector((px, 0.02, pz))) for px, pz in punkte]
    bm.faces.new(vorne)
    bm.faces.new(list(reversed(hinten)))
    for k in range(len(punkte)):
        j = (k + 1) % len(punkte)
        bm.faces.new((vorne[k], hinten[k], hinten[j], vorne[j]))
    w.bm("Banner", bm, einfarbig(tuch, 0.05, w.z))
    raute = [(0, -hoehe * 0.3), (breite * 0.22, -hoehe * 0.5), (0, -hoehe * 0.7), (-breite * 0.22, -hoehe * 0.5)]
    for dy in (-0.035, 0.035):
        bm = bmesh.new()
        flaeche = [bm.verts.new(o + rot @ Vector((px, dy, pz))) for px, pz in raute]
        bm.faces.new(flaeche if dy > 0 else list(reversed(flaeche)))
        w.bm("Bannerzeichen", bm, einfarbig(zeichen, 0.03, w.z))


def goldband(w, r, z, hoehe=0.14):
    w.dreh("Goldband", einfarbig(GOLD, 0.04, w.z), [(r, z), (r + 0.05, z + hoehe * 0.5), (r, z + hoehe)], (0, 0, 0), ecken=16)


def feuerschale(w, ort, farbe_glut="#FF9A3A", groesse=1.0):
    x, y, z = ort
    eisen = einfarbig(EISEN, 0.05, w.z)
    g = groesse
    for k in range(3):
        wi = math.tau * k / 3
        w.saeule("Schalenbein", eisen, 0.03 * g, (x + math.cos(wi) * 0.3 * g, y + math.sin(wi) * 0.3 * g, z), (x + math.cos(wi) * 0.12 * g, y + math.sin(wi) * 0.12 * g, z + 0.55 * g), ecken=5)
    w.dreh("Feuerschale", eisen, [(0.08 * g, 0.5 * g), (0.3 * g, 0.6 * g), (0.38 * g, 0.78 * g), (0.34 * g, 0.8 * g)], (x, y, z), ecken=10)
    for k in range(3):
        w.spitze("Flamme", einfarbig(farbe_glut if k else "#FFD36A", 0.05, w.z), 0.13 * g, (0.45 + 0.15 * k) * g, (x + (k - 1) * 0.08 * g, y, z + 0.7 * g), ecken=5, glut=True)


def kristalle(w, basis, achse=(0, 0, 1), groesse=0.8, farben=("#2F6FD8", "#6FC8FF", "#E8FAFF")):
    w.licht.extend(_kristallbueschel_mid(w.z, Vector(basis), Vector(achse).normalized(), groesse, tuple(farbe(f) for f in farben)))


def _kiste(w, ort, s):
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    w.brett("Kiste", holz, (s, s, s), (ort[0], ort[1], ort[2] + s / 2), (0, 0, w.z.uniform(-10, 10)), fase=0.02)
    for dz in (0.08, s - 0.08):
        w.brett("Kistenleiste", einfarbig(EISEN, 0.04, w.z), (s + 0.02, s + 0.02, 0.05), (ort[0], ort[1], ort[2] + dz), fase=0.0)


# ---------------------------------------------------------------------------
# Die fünfzehn Türme (feste Teile). z = Oberkante der Plattform (dort sitzt der Kopf)
# ---------------------------------------------------------------------------
def _zier(w, stufe, r, z, wimpel=TUCH_ROT, zinnen_an=True):
    """Gemeinsamer Abschluss: ab Stufe 2 zwei Banner, ab Stufe 3 Goldbänder und vier Wimpel."""
    if stufe >= 2:
        for k in range(2):
            wi = math.tau * k / 2 + math.pi / 4
            wandbanner(w, (math.cos(wi) * (r + 0.08), math.sin(wi) * (r + 0.08), z - 0.7), math.degrees(wi) - 90, 0.7, 1.5, wimpel)
    if stufe >= 3:
        goldband(w, r - 0.25, z - 1.35)
        goldband(w, r - 0.3, z - 2.9)
        for k in range(4):
            wi = math.tau * k / 4
            fahne(w, (math.cos(wi) * (r - 0.25), math.sin(wi) * (r - 0.25), z + (0.7 if zinnen_an else 0.9)), 1.4, wimpel)


def _turm_pfeil(w, stufe, z):
    stein = steinfarbe(w)
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    rinde = stammfarbe(w.z, "#6E4A2A", "#4E321A", "#D8A868", "#9C6A38")
    fundament(w, stein, 1.6, 0.9)
    holzgeruest(w, 2.3, 0.9, z - 0.3, rinde, holz)
    bohlen(w, holz, 1.75, z)
    gelaender(w, holz, 1.75, z)
    for sy in (-1, 1):
        w.brett("Pfeilbündel", holzfarbe(w.z, "#C49A62", "#96703E"), (0.12, 0.12, 0.9), (1.95, sy * 0.4, 0.45), (0, 8 * sy, 0), fase=0.01)
    if stufe >= 2:
        for sx in (-1, 1):
            w.brett("Schutzbrett", holz, (1.2, 0.08, 0.8), (sx * 0.9, -1.72, z + 0.4), fase=0.012)
        _laterne(w.teile, w.licht, w.z, (1.25, -1.45, 0.0), hoehe=2.1)
        wandbanner(w, (0, -1.3, z - 0.5), 0, 0.8, 1.6, TUCH_ROT)
    if stufe >= 3:
        for k in range(4):
            wi = math.tau * k / 4 + math.pi / 4
            fahne(w, (math.cos(wi) * 1.62, math.sin(wi) * 1.62, z + 0.95), 1.6, TUCH_ROT)
        goldband(w, 1.62, 0.95)


def _turm_balliste(w, stufe, z):
    stein = steinfarbe(w, "#6F6A62", "#9A938A", "#C2BAAE", "#DAD2C6")
    fundament(w, stein, 2.05, 0.8)
    mauerring(w, stein, 1.85, 1.62, 0.8, z - 0.55)
    steinplattform(w, stein, 1.95, z)
    zinnen(w, stein, 2.0, z, 8)
    if stufe >= 2:
        for k in range(8):
            wi = math.tau * (k + 0.5) / 8
            w.brett("Konsole", stein, (0.35, 0.5, 0.45), (math.cos(wi) * 1.75, math.sin(wi) * 1.75, z - 0.8), (0, 0, math.degrees(wi) + 90), fase=0.04)
        w.brett("Tür", holzfarbe(w.z, "#7C4E2A", "#5A3618"), (0.9, 0.12, 1.7), (0, -1.8, 1.75), fase=0.02)
        for dz in (1.2, 2.2):
            w.brett("Türband", einfarbig(EISEN, 0.04, w.z), (0.95, 0.04, 0.08), (0, -1.88, dz), fase=0.0)
    _zier(w, stufe, 1.95, z)


def _turm_katapult(w, stufe, z):
    stein = steinfarbe(w, "#7A7064", "#A49888", "#C8BCA8", "#DCD2C0")
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    mauerblock(w, stein, 4.2, 4.2, -0.6, z - 0.35, h=0.5)
    for sx in (-1, 1):
        for sy in (-1, 1):
            for l in range(int((z + 0.6) / 0.5) + 1):
                w.brett("Eckstein", stein, (0.8 if l % 2 else 0.75, 0.75 if l % 2 else 0.8, 0.46), (sx * 1.95, sy * 1.95, l * 0.5 - 0.37), fase=0.04)
            w.spitze("Eckdach", einfarbig("#B5563A" if stufe < 3 else GOLD, 0.05, w.z), 0.6, 0.7, (sx * 1.95, sy * 1.95, z + 0.72))
    bohlen(w, holz, 2.05, z, rand=holz)
    for i in range(2 + stufe):
        w.stein("Steinkugel", stein, 0.3, (1.5 - i * 0.55, -2.5, 0.3), flach=1.0)
    if stufe >= 2:
        for x in (-1.5, -0.9):
            _kiste(w, (x, -2.5, 0.0), 0.55)
        wandbanner(w, (0, -2.12, z - 0.6), 0, 0.9, 1.6, TUCH_ROT)
    if stufe >= 3:
        for sx in (-1, 1):
            for sy in (-1, 1):
                fahne(w, (sx * 1.95, sy * 1.95, z + 1.4), 1.3, TUCH_ROT)


def _turm_feuer(w, stufe, z):
    basalt = steinfarbe(w, "#3A3432", "#554C48", "#6E6560", "#877C74")
    fundament(w, basalt, 1.8, 0.8)
    mauerring(w, basalt, 1.55, 1.3, 0.8, z - 0.55, kern="#2A2422")
    for k in range(5 + stufe * 2):
        wi = w.z.uniform(0, math.tau)
        zz = w.z.uniform(1.2, z - 1.0)
        r = 1.55 - 0.25 * (zz / z) + 0.03
        w.brett("Glutriss", einfarbig("#FF7A1A", 0.1, w.z), (0.08, 0.05, w.z.uniform(0.4, 0.9)), (math.cos(wi) * r, math.sin(wi) * r, zz),
                (0, w.z.uniform(-25, 25), math.degrees(wi) + 90), fase=0.0, glut=True)
    steinplattform(w, basalt, 1.72, z)
    zinnen(w, basalt, 1.75, z, 8, hoehe=0.6)
    for k in range(2 if stufe < 3 else 4):
        wi = math.tau * k / (2 if stufe < 3 else 4) + math.pi / 4
        feuerschale(w, (math.cos(wi) * 2.4, math.sin(wi) * 2.4, 0.0), groesse=0.9)
    _zier(w, stufe, 1.72, z, wimpel="#E0582A")


def _turm_frost(w, stufe, z):
    eis = steinfarbe(w, "#8FA2B4", "#B4C6D6", "#D6E4F0", "#EEF6FC")
    fundament(w, eis, 1.7, 0.8)
    mauerring(w, eis, 1.45, 1.12, 0.8, z - 0.55, kern="#7E90A2")
    steinplattform(w, eis, 1.6, z)
    zinnen(w, eis, 1.62, z, 8, hoehe=0.65)
    for k in range(10 + stufe * 4):
        wi = math.tau * k / (10 + stufe * 4)
        w.spitze("Eiszapfen", einfarbig("#BDEBFF", 0.06, w.z), 0.06, w.z.uniform(0.3, 0.7), (math.cos(wi) * 1.68, math.sin(wi) * 1.68, z - 0.55), ecken=5, drehung=(180, 0, 0),
                 glut=stufe >= 3)
    for k in range(2 + stufe):
        wi = math.tau * k / (2 + stufe) + 0.4
        kristalle(w, (math.cos(wi) * 1.95, math.sin(wi) * 1.95, 0.0), (math.cos(wi) * 0.4, math.sin(wi) * 0.4, 1.0), 0.9 + 0.2 * stufe, ("#2F6FD8", "#8FD8FF", "#F0FCFF"))
    _zier(w, stufe, 1.6, z, wimpel=TUCH_BLAU)


def _turm_blitz(w, stufe, z):
    stein = steinfarbe(w, "#4A4C56", "#646876", "#80859A", "#9AA0B4")
    kupfer = einfarbig("#C27A45", 0.06, w.z)
    fundament(w, stein, 1.6, 0.8)
    mauerring(w, stein, 1.32, 1.02, 0.8, z - 0.55, kern="#3A3C46")
    for i in range(3 + stufe * 2):
        zi = 1.3 + i * (z - 2.1) / (3 + stufe * 2)
        r = 1.32 - 0.3 * (zi / z) + 0.06
        w.dreh("Spule", kupfer, [(r, zi), (r + 0.06, zi + 0.06), (r, zi + 0.12)], (0, 0, 0), ecken=16)
    for k in range(3):
        wi = math.tau * k / 3
        p = (math.cos(wi) * 1.4, math.sin(wi) * 1.4)
        w.saeule("Leiter", kupfer, 0.05, (p[0], p[1], 0.8), (p[0], p[1], z + 0.4), ecken=6)
        w.dreh("Glaskugel", einfarbig("#7FD3FF", 0.04, w.z), [(0.0, 0.0), (0.16, 0.12), (0.16, 0.24), (0.0, 0.36)], (p[0], p[1], z + 0.4), ecken=8, glut=True)
    steinplattform(w, stein, 1.45, z)
    gelaender(w, kupfer, 1.45, z, pfosten=10, hoehe=0.7)
    _zier(w, stufe, 1.45, z, wimpel=TUCH_BLAU, zinnen_an=False)


def _turm_sonne(w, stufe, z):
    marmor = steinfarbe(w, "#CFC8BA", "#E6E0D2", "#F4F0E6", "#FFFFFF")
    gold = einfarbig(GOLD, 0.04, w.z)
    fundament(w, marmor, 1.75, 0.8)
    mauerring(w, marmor, 1.35, 1.18, 0.8, z - 0.55, kern="#B8B0A0")
    for k in range(6):
        wi = math.tau * k / 6
        p = (math.cos(wi) * 1.58, math.sin(wi) * 1.58)
        w.dreh("Säule", marmor, [(0.2, 0.8), (0.16, 1.0), (0.15, z - 0.9), (0.2, z - 0.75)], (p[0], p[1], 0), ecken=10)
        w.dreh("Kapitell", gold, [(0.22, z - 0.75), (0.28, z - 0.55), (0.2, z - 0.5)], (p[0], p[1], 0), ecken=10)
    steinplattform(w, marmor, 1.72, z)
    goldband(w, 1.8, z - 0.2, 0.18)
    for k in range(8):
        wi = math.tau * k / 8
        w.spitze("Strahl", gold, 0.08, 0.5, (math.cos(wi) * 1.75, math.sin(wi) * 1.75, z), ecken=4, glut=stufe >= 3)
    if stufe >= 2:
        for k in range(4):
            wi = math.tau * k / 4 + math.pi / 4
            w.brett("Sonnenfenster", einfarbig("#FFE9A0", 0.04, w.z), (0.4, 0.05, 0.9), (math.cos(wi) * 1.3, math.sin(wi) * 1.3, z - 2.0), (0, 0, math.degrees(wi) + 90),
                    fase=0.0, glut=True)
    _zier(w, stufe, 1.72, z, wimpel="#F2C94C", zinnen_an=False)


def _turm_arkan(w, stufe, z):
    stein = steinfarbe(w, "#46405E", "#5E5880", "#7A74A0", "#958EBA")
    fundament(w, stein, 1.7, 0.8)
    mauerring(w, stein, 1.38, 1.05, 0.8, z - 0.55, kern="#36304C")
    for k in range(3 + stufe * 2):
        wi = math.tau * k / (3 + stufe * 2)
        zr = 1.6 + (k % 3) * 0.95
        w.brett("Runenstein", einfarbig("#A98BFF", 0.06, w.z), (0.3, 0.1, 0.45), (math.cos(wi) * 1.9, math.sin(wi) * 1.9, zr), (0, 12, math.degrees(wi) + 90), fase=0.02, glut=True)
    for k in range(4):
        wi = math.tau * k / 4 + math.pi / 4
        w.brett("Fenster", einfarbig("#B69CFF", 0.04, w.z), (0.35, 0.05, 0.8), (math.cos(wi) * 1.24, math.sin(wi) * 1.24, z - 2.1), (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)
    steinplattform(w, stein, 1.55, z)
    zinnen(w, stein, 1.58, z, 8, hoehe=0.6)
    if stufe >= 3:
        kristalle(w, (0, 1.45, z - 0.5), (0, 0.3, 1), 1.0, ("#4B2A9E", "#A98BFF", "#F0E8FF"))
    _zier(w, stufe, 1.55, z, wimpel="#6A4FC8")


def _turm_gift(w, stufe, z):
    stein = steinfarbe(w, "#5E6B4E", "#7A8A66", "#96A680", "#B0BE98", moos="#4E7A30")
    holz = holzfarbe(w.z, "#8A6A40", "#5E4424")
    rinde = stammfarbe(w.z, "#5E4A2A", "#3E2E16", "#C8A060", "#8C6A38")
    kupfer = einfarbig("#B87A45", 0.06, w.z)
    fundament(w, stein, 1.7, 0.8)
    holzgeruest(w, 2.0, 0.8, z - 0.3, rinde, holz)
    for k in range(1 + stufe):
        wi = math.tau * k / 3 + 0.5
        p = (math.cos(wi) * 2.15, math.sin(wi) * 2.15)
        w.dreh("Kessel", kupfer, [(0.2, 0.0), (0.45, 0.2), (0.5, 0.55), (0.45, 0.75), (0.42, 0.75)], (p[0], p[1], 0), ecken=12)
        w.dreh("Sud", einfarbig("#8CFF6A", 0.05, w.z), [(0.42, 0.72), (0.0, 0.74)], (p[0], p[1], 0), ecken=12, glut=True)
        w.saeule("Rohr", kupfer, 0.07, (p[0] * 0.8, p[1] * 0.8, 0.7), (p[0] * 0.5, p[1] * 0.5, z - 0.4), ecken=6)
    bohlen(w, holz, 1.7, z)
    gelaender(w, holz, 1.7, z)
    for k in range(6):
        wi = w.z.uniform(0, math.tau)
        w.stein("Moospolster", einfarbig("#4E7A30", 0.1, w.z), 0.3, (math.cos(wi) * 1.95, math.sin(wi) * 1.95, 0.1), flach=0.4)
    _zier(w, stufe, 1.7, z, wimpel="#4E8A3A", zinnen_an=False)


def _turm_banner(w, stufe, z):
    stein = steinfarbe(w)
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    rinde = stammfarbe(w.z, "#6E4A2A", "#4E321A", "#D8A868", "#9C6A38")
    fundament(w, stein, 1.6, 0.8)
    holzgeruest(w, 1.9, 0.8, z - 0.3, rinde, holz)
    bohlen(w, holz, 1.55, z)
    gelaender(w, holz, 1.55, z)
    for k in range(2 + stufe):
        wi = math.tau * k / (2 + stufe) + 0.3
        p = (math.cos(wi) * 2.35, math.sin(wi) * 2.35, 0.0)
        w.dreh("Trommel", einfarbig(TUCH_ROT, 0.05, w.z), [(0.4, 0.0), (0.44, 0.3), (0.4, 0.6)], p, ecken=12)
        w.dreh("Fell", einfarbig("#E8DCC0", 0.04, w.z), [(0.4, 0.6), (0.0, 0.62)], p, ecken=12)
        for dz in (0.05, 0.55):
            w.dreh("Reif", einfarbig(GOLD, 0.03, w.z), [(0.43, dz), (0.44, dz + 0.05)], p, ecken=12)
    _zier(w, stufe, 1.55, z, zinnen_an=False)


def _turm_kaserne(w, stufe, z):
    stein = steinfarbe(w)
    b = 4.0
    mauerblock(w, stein, b, b, -0.6, z - 0.35)
    w.brett("Wehrgang", stein, (b + 0.4, b + 0.4, 0.3), (0, 0, z - 0.2), fase=0.04)
    for s in range(4):
        for i in range(4):
            x = -b / 2 + 0.5 + i * (b - 1.0) / 3
            p = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((x, -b / 2 - 0.05, z + 0.35))
            w.brett("Zinne", stein, (0.6, 0.45, 0.7), tuple(p), (0, 0, 90 * s), fase=0.04)
    torholz = holzfarbe(w.z, "#7C4E2A", "#5A3618")
    for i in range(6):
        w.brett("Torbrett", torholz, (0.25, 0.1, 2.2), (-0.62 + i * 0.25, -b / 2 - 0.06, 1.1), fase=0.012)
    for dz in (0.5, 1.7):
        w.brett("Torband", einfarbig(EISEN, 0.04, w.z), (1.6, 0.04, 0.1), (0, -b / 2 - 0.12, dz), fase=0.0)
    for k in range(7):
        a = math.pi * k / 6
        w.brett("Keilstein", stein, (0.35, 0.45, 0.28), (math.cos(a) * 0.95, -b / 2 - 0.05, 2.2 + math.sin(a) * 0.55), (0, -math.degrees(a) + 90, 0), fase=0.03)
    for sx in (-1, 1):
        w.saeule("Fackelhalter", einfarbig(EISEN, 0.04, w.z), 0.04, (sx * 1.25, -b / 2 - 0.1, 2.0), (sx * 1.3, -b / 2 - 0.35, 2.4), ecken=5)
        w.spitze("Fackel", einfarbig("#FF9A3A", 0.05, w.z), 0.1, 0.35, (sx * 1.3, -b / 2 - 0.37, 2.4), ecken=5, glut=True)
        for j in range(3):
            w.saeule("Speer", holzfarbe(w.z, "#C49A62", "#96703E"), 0.03, (sx * (1.7 + j * 0.15), -b / 2 - 0.5, 0.0), (sx * (1.65 + j * 0.15), -b / 2 - 0.45, 2.2), ecken=5)
            w.spitze("Speerspitze", einfarbig("#B8BEC6", 0.03, w.z), 0.05, 0.22, (sx * (1.65 + j * 0.15), -b / 2 - 0.45, 2.2), ecken=4)
    for s in (1, 3):
        for x in (-0.9, 0.9):
            p = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((x, -b / 2 - 0.1, 2.0))
            w.dreh("Schild", einfarbig(TUCH_BLAU, 0.04, w.z), [(0.0, 0.0), (0.38, 0.02), (0.4, 0.08), (0.0, 0.12)], tuple(p), ecken=12, drehung=(90, 0, 90 * s))
            w.dreh("Schildbuckel", einfarbig(GOLD, 0.03, w.z), [(0.0, 0.1), (0.1, 0.12), (0.0, 0.2)], tuple(p), ecken=8, drehung=(90, 0, 90 * s))
    for s in (1, 2, 3):
        p = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((0, -b / 2 - 0.03, z - 2.0))
        w.brett("Fenster", einfarbig("#FFC878", 0.03, w.z), (0.45, 0.05, 0.85), tuple(p), (0, 0, 90 * s), fase=0.0, glut=True)
    if stufe >= 2:
        for sx in (-1, 1):
            wandbanner(w, (sx * 1.3, -b / 2 - 0.1, z - 0.6), 0, 0.75, 1.9, TUCH_BLAU)
    if stufe >= 3:
        for sx in (-1, 1):
            for sy in (-1, 1):
                fahne(w, (sx * (b / 2 - 0.2), sy * (b / 2 - 0.2), z + 0.7), 1.5, TUCH_BLAU)
        w.brett("Goldleiste", einfarbig(GOLD, 0.03, w.z), (b + 0.45, 0.06, 0.12), (0, -b / 2 - 0.22, z - 0.1), fase=0.0)


def _turm_spaeher(w, stufe, z):
    stein = steinfarbe(w)
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    rinde = stammfarbe(w.z, "#6E4A2A", "#4E321A", "#D8A868", "#9C6A38")
    fundament(w, stein, 1.35, 0.8)
    holzgeruest(w, 1.6, 0.8, z - 0.3, rinde, holz)
    bohlen(w, holz, 1.45, z)
    gelaender(w, holz, 1.45, z)
    for k in range(4):
        wi = math.tau * k / 4 + math.pi / 4
        w.stamm("Dachpfosten", rinde, 0.09, 2.6, (math.cos(wi) * 1.3, math.sin(wi) * 1.3, z + 1.3), (0, 90, 0), ecken=7)
    schindeldach(w, holzfarbe(w.z, "#B4523A", "#8A3A28", maserung=9.0), 1.45, z + 2.55, 1.5)
    fahne(w, (0, 0, z + 4.45), 1.3, TUCH_BLAU)
    if stufe >= 2:
        for k in range(2):
            wi = math.tau * k / 2 + math.pi / 4
            _laterne(w.teile, w.licht, w.z, (math.cos(wi) * 1.25, math.sin(wi) * 1.25, z), hoehe=1.9)
    if stufe >= 3:
        goldband(w, 1.35, 0.95)


def _turm_sturm(w, stufe, z):
    stein = steinfarbe(w, "#8A95A0", "#AAB6C2", "#C8D2DC", "#E2EAF0")
    fundament(w, stein, 1.7, 0.8)
    mauerring(w, stein, 1.45, 1.1, 0.8, z - 0.55, kern="#6E7884")
    n = 6 + stufe * 3
    for k in range(n):
        wi = math.tau * k / 5.0
        zz = 1.3 + (z - 2.2) * k / n
        r = 1.45 - 0.35 * (zz / z) + 0.1
        w.brett("Windwimpel", einfarbig("#E8F4FF" if k % 2 else TUCH_BLAU, 0.05, w.z), (0.6, 0.03, 0.22), (math.cos(wi) * (r + 0.3), math.sin(wi) * (r + 0.3), zz),
                (0, 10, math.degrees(wi)), fase=0.0)
    nabe = Vector((0.0, -1.55, z * 0.55))
    w.saeule("Achse", einfarbig(EISEN, 0.04, w.z), 0.08, nabe + Vector((0, 0.45, 0)), nabe, ecken=8)
    for k in range(4):
        wi = math.radians(45 + 90 * k)
        mitte = nabe + Vector((math.cos(wi) * 0.75, -0.05, math.sin(wi) * 0.75))
        w.brett("Windradflügel", holzfarbe(w.z, "#E8E2D0", "#C8C2B0"), (1.4, 0.04, 0.32), tuple(mitte), (0, -math.degrees(wi), 0), fase=0.01)
    steinplattform(w, stein, 1.6, z)
    zinnen(w, stein, 1.62, z, 8, hoehe=0.6)
    _zier(w, stufe, 1.6, z, wimpel=TUCH_BLAU)


def _turm_runen(w, stufe, z):
    stein = steinfarbe(w, "#5E5A68", "#77727F", "#928D9A", "#AAA6B2")
    fundament(w, stein, 2.0, 0.8)
    mauerring(w, stein, 1.85, 1.55, 0.8, z - 0.55, kern="#46424E")
    rune = einfarbig("#7FE0FF", 0.05, w.z)
    for k in range(6):
        wi = math.tau * k / 6
        zz = 1.8 + (k % 2) * 0.5
        r = 1.85 - 0.3 * (zz / z) + 0.02
        w.brett("Runenstrich", rune, (0.07, 0.05, 0.8), (math.cos(wi) * r, math.sin(wi) * r, zz), (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)
        w.brett("Runenstrich", rune, (0.4, 0.05, 0.07), (math.cos(wi) * r, math.sin(wi) * r, zz + 0.3), (0, 30, math.degrees(wi) + 90), fase=0.0, glut=True)
    for k in range(3 + stufe):
        wi = math.tau * k / (3 + stufe) + 0.3
        p = (math.cos(wi) * 2.65, math.sin(wi) * 2.65)
        w.brett("Menhir", stein, (0.55, 0.4, 1.8), (p[0], p[1], 0.6), (w.z.uniform(-5, 5), w.z.uniform(-5, 5), math.degrees(wi) + 90), fase=0.08)
        w.brett("Menhirrune", rune, (0.1, 0.05, 0.7), (p[0] * 0.92, p[1] * 0.92, 0.9), (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)
    steinplattform(w, stein, 1.85, z)
    zinnen(w, stein, 1.88, z, 6, hoehe=0.6, breite=0.8)
    _zier(w, stufe, 1.85, z, wimpel=TUCH_BLAU)


def _turm_schatz(w, stufe, z):
    marmor = steinfarbe(w, "#CFC8BA", "#E6E0D2", "#F4F0E6", "#FFFFFF")
    gold = einfarbig(GOLD, 0.04, w.z)
    b = 3.6
    mauerblock(w, marmor, b, b, -0.6, z - 0.35, kern="#B8B0A0")
    for zz in (1.2, z - 1.0):
        w.brett("Goldgesims", gold, (b + 0.12, b + 0.12, 0.16), (0, 0, zz), fase=0.02)
    w.brett("Dachplatte", marmor, (b + 0.3, b + 0.3, 0.3), (0, 0, z - 0.15), fase=0.04)
    for s in range(4):
        rot = Matrix.Rotation(math.radians(90 * s), 3, "Z")
        for i in range(5):
            p = rot @ Vector((-b / 2 + 0.3 + i * (b - 0.6) / 4, -b / 2 - 0.05, z + 0.3))
            w.dreh("Baluster", gold, [(0.07, -0.3), (0.1, -0.15), (0.06, 0.0), (0.1, 0.2), (0.07, 0.3)], tuple(p), ecken=6)
        p = rot @ Vector((0, -b / 2 - 0.05, z + 0.62))
        w.brett("Handlauf", gold, (b + 0.2, 0.12, 0.08), tuple(p), (0, 0, 90 * s), fase=0.01)
    w.dreh("Tresortür", einfarbig(EISEN, 0.04, w.z), [(0.0, 0.0), (0.85, 0.0), (0.85, 0.14), (0.0, 0.16)], (0, -b / 2 - 0.02, 1.6), ecken=16, drehung=(90, 0, 0))
    w.dreh("Tresorring", gold, [(0.95, 0.0), (0.97, 0.08), (0.85, 0.1)], (0, -b / 2 - 0.1, 1.6), ecken=16, drehung=(90, 0, 0))
    for k in range(4):
        w.brett("Speiche", gold, (1.2, 0.06, 0.08), (0, -b / 2 - 0.2, 1.6), (0, 45 * k, 0), fase=0.0)
    for x, g in ((-1.3, 0.45), (1.3, 0.38), (0.85, 0.25)):
        w.dreh("Münzhaufen", gold, [(0.0, 0.0), (g, 0.02), (g * 0.6, g * 0.45), (0.0, g * 0.6)], (x, -b / 2 - 0.65, 0.0), ecken=9)
    _kiste(w, (-1.4, -b / 2 - 0.3, 0.0), 0.55)
    if stufe >= 2:
        for sx in (-1, 1):
            for sy in (-1, 1):
                w.dreh("Goldkuppel", gold, [(0.0, 0.0), (0.35, 0.05), (0.38, 0.3), (0.2, 0.62), (0.0, 0.8)], (sx * b / 2, sy * b / 2, z + 0.1), ecken=10)
    if stufe >= 3:
        for s in range(4):
            p = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((0, -b / 2 - 0.03, z - 2.4))
            w.brett("Edelsteinfenster", einfarbig("#7FE0FF", 0.04, w.z), (0.55, 0.05, 0.9), tuple(p), (0, 0, 90 * s), fase=0.0, glut=True)


TUERME = {
    "pfeil": _turm_pfeil, "balliste": _turm_balliste, "katapult": _turm_katapult, "feuer": _turm_feuer, "frost": _turm_frost,
    "blitz": _turm_blitz, "sonne": _turm_sonne, "arkan": _turm_arkan, "gift": _turm_gift, "banner": _turm_banner,
    "kaserne": _turm_kaserne, "spaeher": _turm_spaeher, "sturm": _turm_sturm, "runen": _turm_runen, "schatz": _turm_schatz,
}


def turm(art, stufe):
    w = Werk(sum(map(ord, art)) * 7 + stufe)
    TUERME[art](w, stufe, KOPF_Z[stufe])
    return w.fertig(f"Turm_{art}_{stufe}")


# ---------------------------------------------------------------------------
# Köpfe (drehbar, Vorderseite −Y, Fuß im Ursprung; im Spiel 1,3-fach groß)
# ---------------------------------------------------------------------------
def _drehteller(w, r=0.65, holzig=True):
    holz = holzfarbe(w.z, "#7A5230", "#553820")
    eisen = einfarbig(EISEN, 0.05, w.z)
    w.dreh("Drehteller", eisen, [(r, 0.0), (r, 0.08), (r * 0.9, 0.12)], (0, 0, 0), ecken=14)
    if holzig:
        w.dreh("Drehbock", holz, [(r * 0.55, 0.1), (r * 0.5, 0.35), (r * 0.45, 0.4)], (0, 0, 0), ecken=8)
    for k in range(8):
        wi = math.tau * k / 8
        w.dreh("Niete", eisen, [(0.0, 0.08), (0.04, 0.1), (0.0, 0.13)], (math.cos(wi) * r * 0.85, math.sin(wi) * r * 0.85, 0), ecken=5)


def _bogen(w, mitte_y, z, spannweite, dicke, farbe_von, spitze_farbe, tiefe=0.35):
    """Gebogener Bogen quer zur Schussrichtung (aus Segmenten), mit Sehne."""
    punkte = []
    for i in range(7):
        t = (i / 6) * 2 - 1
        punkte.append(Vector((t * spannweite / 2, mitte_y + tiefe * (t * t), z)))
    for a, b in zip(punkte, punkte[1:]):
        dicke_i = dicke * (1.0 - 0.4 * abs((a.x + b.x) / spannweite))
        w.saeule("Bogenarm", farbe_von, dicke_i, a, b, ecken=6)
    for p in (punkte[0], punkte[-1]):
        w.dreh("Bogenspitze", spitze_farbe, [(0.0, -0.06), (dicke * 1.2, 0.0), (0.0, 0.1)], tuple(p), ecken=6)
    w.saeule("Sehne", einfarbig("#E8DCC0", 0.03, w.z), 0.012, punkte[0], Vector((0, mitte_y + tiefe * 1.9, z)), ecken=4)
    w.saeule("Sehne", einfarbig("#E8DCC0", 0.03, w.z), 0.012, punkte[-1], Vector((0, mitte_y + tiefe * 1.9, z)), ecken=4)


def _kopf_pfeil(w):
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    dunkel = holzfarbe(w.z, "#6E4A2A", "#4E321A")
    stahl = einfarbig("#9AA3AE", 0.04, w.z)
    _drehteller(w, 0.55)
    w.brett("Lafette", dunkel, (0.3, 0.5, 0.45), (0, 0, 0.55), fase=0.03)
    w.brett("Säule", holz, (0.2, 1.7, 0.18), (0, -0.25, 0.85), fase=0.02)
    _bogen(w, -0.95, 0.9, 1.9, 0.055, holz, stahl)
    w.saeule("Bolzen", holzfarbe(w.z, "#C49A62", "#96703E"), 0.025, (0, 0.2, 0.98), (0, -1.35, 0.98), ecken=5)
    w.spitze("Bolzenspitze", stahl, 0.05, 0.22, (0, -1.35, 0.98), ecken=4, drehung=(90, 0, 0))
    for sx in (-1, 1):
        w.brett("Federn", einfarbig(TUCH_ROT, 0.05, w.z), (0.01, 0.12, 0.08), (sx * 0.03, 0.15, 0.98), fase=0.0)
    w.dreh("Kurbel", einfarbig(EISEN, 0.05, w.z), [(0.0, 0.0), (0.08, 0.02), (0.08, 0.1), (0.0, 0.12)], (0.22, 0.45, 0.85), ecken=8, drehung=(0, 90, 0))
    w.brett("Kurbelgriff", dunkel, (0.05, 0.05, 0.25), (0.35, 0.45, 0.95), fase=0.01)
    # Köcher hinten
    w.dreh("Köcher", einfarbig("#6E4A32", 0.05, w.z), [(0.1, 0.0), (0.12, 0.5), (0.13, 0.55)], (-0.35, 0.35, 0.3), ecken=8)
    for k in range(5):
        w.saeule("Pfeil", holzfarbe(w.z, "#C49A62", "#96703E"), 0.015, (-0.35 + (k - 2) * 0.03, 0.35, 0.8), (-0.35 + (k - 2) * 0.04, 0.37, 1.05), ecken=4)


def _kopf_balliste(w):
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    dunkel = holzfarbe(w.z, "#6E4A2A", "#4E321A")
    eisen = einfarbig(EISEN, 0.05, w.z)
    stahl = einfarbig("#9AA3AE", 0.04, w.z)
    _drehteller(w, 0.85)
    for sx in (-1, 1):
        w.brett("Wange", dunkel, (0.16, 1.0, 0.85), (sx * 0.42, 0.1, 0.55), fase=0.025)
    w.brett("Schiene", holz, (0.34, 2.5, 0.22), (0, -0.35, 1.02), fase=0.03)
    # Torsionsbündel links und rechts, darin die Bogenarme
    for sx in (-1, 1):
        w.dreh("Torsion", einfarbig("#C8B890", 0.05, w.z), [(0.14, 0.0), (0.16, 0.1), (0.16, 0.5), (0.14, 0.6)], (sx * 0.45, -1.1, 0.75), ecken=10)
        for dz in (0.75, 1.35):
            w.dreh("Eisenring", eisen, [(0.18, 0.0), (0.18, 0.06)], (sx * 0.45, -1.1, dz), ecken=10)
    _bogen(w, -1.2, 1.12, 2.8, 0.08, dunkel, stahl, tiefe=0.45)
    w.saeule("Bolzen", holzfarbe(w.z, "#C49A62", "#96703E"), 0.045, (0, 0.8, 1.2), (0, -1.75, 1.2), ecken=6)
    w.spitze("Bolzenspitze", stahl, 0.09, 0.35, (0, -1.75, 1.2), ecken=4, drehung=(90, 0, 0))
    # Winde hinten mit Seil
    w.dreh("Winde", dunkel, [(0.14, 0.0), (0.14, 0.9)], (-0.45, 0.8, 0.9), ecken=10, drehung=(0, 90, 0))
    w.dreh("Seil", einfarbig("#C8B890", 0.05, w.z), [(0.16, 0.2), (0.16, 0.7)], (-0.45, 0.8, 0.9), ecken=10, drehung=(0, 90, 0))
    for sx in (-1, 1):
        w.brett("Speiche", dunkel, (0.06, 0.5, 0.06), (sx * 0.55, 0.8, 0.9), (0, 0, 0), fase=0.01)
        w.brett("Speiche", dunkel, (0.06, 0.06, 0.5), (sx * 0.55, 0.8, 0.9), (0, 0, 0), fase=0.01)


def _kopf_katapult(w):
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    dunkel = holzfarbe(w.z, "#6E4A2A", "#4E321A")
    eisen = einfarbig(EISEN, 0.05, w.z)
    stein = steinfarbe(w)
    _drehteller(w, 1.15, holzig=False)
    for sx in (-1, 1):
        w.brett("Längsholm", dunkel, (0.22, 2.0, 0.22), (sx * 0.62, 0, 0.2), fase=0.03)
        for sy in (-1, 1):
            w.brett("Ständer", holz, (0.18, 0.18, 1.55), (sx * 0.62, sy * 0.35, 0.95), (sy * 22, 0, 0), fase=0.025)
    w.saeule("Achse", eisen, 0.08, (-0.8, 0, 1.6), (0.8, 0, 1.6), ecken=8)
    w.dreh("Torsionsseil", einfarbig("#C8B890", 0.05, w.z), [(0.2, 0.0), (0.2, 0.9)], (-0.45, 0, 1.6), ecken=10, drehung=(0, 90, 0))
    # Wurfarm nach hinten geneigt mit Schale und Stein, Anschlagbalken vorne
    w.brett("Wurfarm", holz, (0.18, 2.6, 0.18), (0, 0.75, 1.1), (-32, 0, 0), fase=0.03)
    w.dreh("Schale", dunkel, [(0.0, 0.0), (0.35, 0.05), (0.4, 0.25), (0.33, 0.25)], (0, 1.85, 0.42), ecken=10)
    w.stein("Wurfstein", stein, 0.3, (0, 1.85, 0.7), flach=1.0)
    w.brett("Anschlag", dunkel, (1.4, 0.2, 0.25), (0, -0.5, 1.95), fase=0.03)
    w.brett("Polster", einfarbig("#8C6A48", 0.05, w.z), (0.5, 0.25, 0.2), (0, -0.4, 1.95), fase=0.03)
    for sx in (-1, 1):
        w.brett("Eisenband", eisen, (0.24, 0.24, 0.05), (sx * 0.62, 0.35, 0.3), fase=0.0)


def _kopf_feuer(w):
    eisen = einfarbig("#3E3C40", 0.05, w.z)
    dunkel = einfarbig("#2A2628", 0.05, w.z)
    gold = einfarbig(GOLD, 0.04, w.z)
    _drehteller(w, 0.55, holzig=False)
    w.dreh("Glutkessel", eisen, [(0.1, 0.1), (0.5, 0.3), (0.6, 0.7), (0.62, 0.9), (0.55, 0.9)], (0, 0.25, 0), ecken=12)
    feuerschale(w, (0, 0.25, 0.45), groesse=0.9)
    # Drachenkopf-Düse nach vorne: Schädel, Kiefer, Hörner, Augen
    w.brett("Drachenhals", eisen, (0.34, 0.8, 0.32), (0, -0.35, 0.85), (-10, 0, 0), fase=0.05)
    w.brett("Drachenschädel", eisen, (0.46, 0.55, 0.36), (0, -0.85, 0.95), fase=0.06)
    w.brett("Oberkiefer", dunkel, (0.36, 0.5, 0.16), (0, -1.25, 0.98), (8, 0, 0), fase=0.04)
    w.brett("Unterkiefer", dunkel, (0.32, 0.45, 0.12), (0, -1.2, 0.76), (-14, 0, 0), fase=0.04)
    for sx in (-1, 1):
        w.spitze("Horn", gold, 0.07, 0.45, (sx * 0.16, -0.72, 1.1), ecken=5, drehung=(-60, sx * 20, 0))
        w.dreh("Auge", einfarbig("#FFB020", 0.04, w.z), [(0.0, 0.0), (0.05, 0.02), (0.0, 0.05)], (sx * 0.2, -1.05, 1.05), ecken=6, drehung=(90, 0, 0), glut=True)
        for k in range(3):
            w.spitze("Zahn", einfarbig("#E8E0CC", 0.03, w.z), 0.025, 0.1, (sx * (0.06 + k * 0.05), -1.4 + k * 0.06, 0.9), ecken=4, drehung=(180, 0, 0))
    for k in range(3):
        w.spitze("Züngelnde Flamme", einfarbig("#FF8A2A" if k else "#FFD36A", 0.05, w.z), 0.08, 0.35 - k * 0.06, (0, -1.5 - k * 0.12, 0.88), ecken=5, drehung=(90, 0, 0), glut=True)


def _kopf_frost(w):
    eis = steinfarbe(w, "#8FA2B4", "#B4C6D6", "#D6E4F0", "#EEF6FC")
    _drehteller(w, 0.55, holzig=False)
    w.dreh("Sockel", eis, [(0.5, 0.1), (0.42, 0.35), (0.3, 0.45), (0.35, 0.55)], (0, 0, 0), ecken=8)
    for k in range(6):
        wi = math.tau * k / 6
        w.brett("Rune", einfarbig("#9FE0FF", 0.04, w.z), (0.12, 0.03, 0.12), (math.cos(wi) * 0.44, math.sin(wi) * 0.44, 0.3), (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)
    kristalle(w, (0, 0, 0.5), (0, -0.15, 1), 1.6, ("#2F6FD8", "#8FD8FF", "#F0FCFF"))
    for k in range(4):
        wi = math.tau * k / 4 + 0.4
        kristalle(w, (math.cos(wi) * 0.35, math.sin(wi) * 0.35, 0.45), (math.cos(wi) * 0.6, math.sin(wi) * 0.6, 1), 0.6, ("#2F6FD8", "#8FD8FF", "#F0FCFF"))


def _kopf_blitz(w):
    kupfer = einfarbig("#C27A45", 0.06, w.z)
    stein = steinfarbe(w, "#4A4C56", "#646876", "#80859A", "#9AA0B4")
    _drehteller(w, 0.5, holzig=False)
    w.dreh("Isolator", stein, [(0.4, 0.1), (0.32, 0.4), (0.2, 0.5)], (0, 0, 0), ecken=10)
    w.saeule("Säule", kupfer, 0.1, (0, 0, 0.4), (0, 0, 1.5), ecken=8)
    for i in range(7):
        w.dreh("Spulenring", kupfer, [(0.34 - i * 0.03, 0.0), (0.36 - i * 0.03, 0.04), (0.34 - i * 0.03, 0.08)], (0, 0, 0.5 + i * 0.14), ecken=12)
    w.dreh("Blitzkugel", einfarbig("#7FD3FF", 0.04, w.z), [(0.0, 0.0), (0.26, 0.12), (0.32, 0.35), (0.26, 0.58), (0.0, 0.7)], (0, 0, 1.45), ecken=12, glut=True)
    for k in range(4):
        wi = math.tau * k / 4 + math.pi / 4
        w.saeule("Zinke", kupfer, 0.03, (math.cos(wi) * 0.2, math.sin(wi) * 0.2, 1.6), (math.cos(wi) * 0.55, math.sin(wi) * 0.55, 2.2), ecken=5)
        w.dreh("Zinkenkugel", einfarbig("#BFEAFF", 0.04, w.z), [(0.0, 0.0), (0.06, 0.05), (0.0, 0.1)], (math.cos(wi) * 0.55, math.sin(wi) * 0.55, 2.18), ecken=6, glut=True)


def _kopf_sonne(w):
    gold = einfarbig(GOLD, 0.04, w.z)
    marmor = steinfarbe(w, "#CFC8BA", "#E6E0D2", "#F4F0E6", "#FFFFFF")
    _drehteller(w, 0.5, holzig=False)
    w.dreh("Fuß", marmor, [(0.4, 0.1), (0.25, 0.3), (0.2, 0.55)], (0, 0, 0), ecken=10)
    for sx in (-1, 1):
        w.brett("Gabel", gold, (0.1, 0.12, 1.1), (sx * 0.72, 0, 0.95), fase=0.02)
    w.brett("Gabelsteg", gold, (1.55, 0.14, 0.12), (0, 0, 0.45), fase=0.02)
    w.dreh("Spiegelrand", gold, [(0.0, -0.06), (0.66, -0.06), (0.7, 0.0), (0.66, 0.06), (0.0, 0.06)], (0, 0, 1.35), ecken=18, drehung=(90, 0, 0))
    w.dreh("Spiegel", einfarbig("#FFF1B0", 0.03, w.z), [(0.0, -0.08), (0.55, -0.08), (0.0, -0.1)], (0, 0, 1.35), ecken=18, drehung=(90, 0, 0), glut=True)
    for k in range(12):
        wi = math.tau * k / 12
        w.spitze("Strahl", gold, 0.06, 0.32, (math.cos(wi) * 0.75, 0.02, 1.35 + math.sin(wi) * 0.75), ecken=4, drehung=(0, -math.degrees(wi) + 90, 0))


def _kopf_arkan(w):
    stein = steinfarbe(w, "#46405E", "#5E5880", "#7A74A0", "#958EBA")
    ring_farbe = einfarbig("#D8C8FF", 0.04, w.z)
    _drehteller(w, 0.45, holzig=False)
    w.dreh("Sockel", stein, [(0.4, 0.1), (0.25, 0.35), (0.3, 0.5)], (0, 0, 0), ecken=8)
    w.dreh("Arkankugel", einfarbig("#B69CFF", 0.04, w.z), [(0.0, 0.0), (0.22, 0.08), (0.3, 0.3), (0.22, 0.52), (0.0, 0.6)], (0, 0, 0.85), ecken=12, glut=True)
    for n, (rx, ry) in enumerate(((90, 0), (30, 60), (-40, -30))):
        for k in range(16):
            a = math.tau * k / 16
            b = math.tau * (k + 1) / 16
            dreh = Matrix.Rotation(math.radians(ry), 3, "Z") @ Matrix.Rotation(math.radians(rx), 3, "X")
            pa = dreh @ Vector((math.cos(a) * 0.65, math.sin(a) * 0.65, 0)) + Vector((0, 0, 1.15))
            pb = dreh @ Vector((math.cos(b) * 0.65, math.sin(b) * 0.65, 0)) + Vector((0, 0, 1.15))
            w.saeule("Arkanring", ring_farbe, 0.025, pa, pb, ecken=4, glut=n == 0)
    for k in range(3):
        wi = math.tau * k / 3
        w.brett("Schwebestein", stein, (0.18, 0.12, 0.25), (math.cos(wi) * 0.85, math.sin(wi) * 0.85, 1.4), (0, 20, math.degrees(wi)), fase=0.02)


def _kopf_gift(w):
    eisen = einfarbig("#3E3C40", 0.05, w.z)
    kupfer = einfarbig("#B87A45", 0.06, w.z)
    _drehteller(w, 0.5, holzig=False)
    w.dreh("Tank", eisen, [(0.1, 0.15), (0.5, 0.3), (0.62, 0.65), (0.55, 1.0), (0.45, 1.08)], (0, 0.1, 0), ecken=12)
    for dz in (0.45, 0.85):
        w.dreh("Tankband", kupfer, [(0.6, dz), (0.62, dz + 0.05), (0.6, dz + 0.1)], (0, 0.1, 0), ecken=12)
    w.dreh("Giftsud", einfarbig("#8CFF6A", 0.05, w.z), [(0.45, 1.06), (0.0, 1.08)], (0, 0.1, 0), ecken=12, glut=True)
    for k in range(5):
        wi = math.tau * k / 5
        w.dreh("Blase", einfarbig("#B8FF8C", 0.05, w.z), [(0.0, 0.0), (0.08, 0.05), (0.0, 0.12)], (math.cos(wi) * 0.25, 0.1 + math.sin(wi) * 0.25, 1.06), ecken=6, glut=True)
    w.saeule("Düse", kupfer, 0.08, (0, -0.35, 0.6), (0, -1.1, 0.85), ecken=8, radius_ende=0.12)
    w.dreh("Düsenmund", kupfer, [(0.12, 0.0), (0.2, 0.12), (0.18, 0.14)], (0, -1.1, 0.85), ecken=8, drehung=(-75, 0, 0))
    w.dreh("Ventil", einfarbig(TUCH_ROT, 0.05, w.z), [(0.0, 0.0), (0.12, 0.02), (0.12, 0.05), (0.0, 0.06)], (0.4, -0.3, 0.8), ecken=8, drehung=(0, 90, 0))


def _kopf_banner(w, tuch=TUCH_ROT):
    holz = holzfarbe(w.z, "#6E4A2A", "#4E321A")
    gold = einfarbig(GOLD, 0.04, w.z)
    _drehteller(w, 0.35)
    w.saeule("Mast", holz, 0.08, (0, 0, 0.2), (0, 0, 4.4), ecken=8)
    w.dreh("Spitze", gold, [(0.0, 0.0), (0.14, 0.1), (0.0, 0.45)], (0, 0, 4.4), ecken=6)
    w.saeule("Querstange", gold, 0.045, (-0.05, 0, 4.0), (1.85, 0, 4.0), ecken=6)
    # Großes, gewelltes Banner mit Rautenzeichen
    bm = bmesh.new()
    spalten, zeilen = 6, 8
    grid = []
    for i in range(spalten + 1):
        u = i / spalten
        spalte = []
        for j in range(zeilen + 1):
            v = j / zeilen
            unten = 1.9 + 0.5 * abs(u - 0.5) * 2
            zz = 3.95 - (3.95 - unten) * v
            welle = math.sin(v * 4.0 + u * 2.0) * 0.07
            spalte.append(bm.verts.new((0.05 + u * 1.8, welle, zz)))
        grid.append(spalte)
    rueck = [[bm.verts.new(v.co + Vector((0, 0.02, 0))) for v in s] for s in grid]
    for i in range(spalten):
        for j in range(zeilen):
            bm.faces.new((grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]))
            bm.faces.new((rueck[i][j + 1], rueck[i + 1][j + 1], rueck[i + 1][j], rueck[i][j]))
    w.bm("Banner", bm, einfarbig(tuch, 0.05, w.z))
    for dy in (-0.04, 0.06):
        bm = bmesh.new()
        flaeche = [bm.verts.new((0.95 + px, dy + math.sin(2.0 + pz) * 0.07, 3.1 + pz)) for px, pz in ((0, 0.45), (0.4, 0.0), (0, -0.45), (-0.4, 0.0))]
        bm.faces.new(flaeche if dy > 0 else list(reversed(flaeche)))
        w.bm("Bannerzeichen", bm, gold)
    w.dreh("Horn", einfarbig("#E8DCC0", 0.04, w.z), [(0.03, 0.0), (0.05, 0.3), (0.12, 0.55), (0.2, 0.62)], (0, -0.45, 0.6), ecken=8, drehung=(70, 0, 0))


def _kopf_kaserne(w):
    _kopf_banner(w, TUCH_BLAU)
    for s in (-1, 1):
        w.saeule("Speer", holzfarbe(w.z, "#C49A62", "#96703E"), 0.03, (0.95 - s * 0.45, 0.08, 2.0), (0.95 + s * 0.45, 0.08, 3.3), ecken=5)


def _kopf_spaeher(w):
    holz = holzfarbe(w.z, "#8A6A40", "#5E4424")
    messing = einfarbig("#C8A050", 0.05, w.z)
    _drehteller(w, 0.4)
    for k in range(3):
        wi = math.tau * k / 3
        w.saeule("Dreibein", holz, 0.035, (math.cos(wi) * 0.4, math.sin(wi) * 0.4, 0.1), (0, 0, 1.2), ecken=6)
    w.dreh("Gelenk", messing, [(0.0, 0.0), (0.08, 0.04), (0.0, 0.12)], (0, 0, 1.15), ecken=8)
    for i, (r0, r1, laenge) in enumerate(((0.11, 0.1, 0.55), (0.09, 0.08, 0.45), (0.07, 0.065, 0.35))):
        y0 = 0.25 - i * 0.45
        w.saeule("Fernrohr", messing if i % 2 == 0 else einfarbig("#6E4A32", 0.05, w.z), r0, (0, y0, 1.28 - i * 0.03), (0, y0 - laenge, 1.3 - i * 0.03), ecken=10, radius_ende=r1)
    w.dreh("Linse", einfarbig("#9FE0FF", 0.04, w.z), [(0.0, 0.0), (0.06, 0.0), (0.0, 0.02)], (0, -0.95, 1.24), ecken=8, drehung=(90, 0, 0), glut=True)
    _laterne(w.teile, w.licht, w.z, (0.45, 0.35, 0.1), hoehe=1.0)


def _kopf_sturm(w):
    hell = einfarbig("#DDE6EE", 0.04, w.z)
    stein = steinfarbe(w, "#8A95A0", "#AAB6C2", "#C8D2DC", "#E2EAF0")
    _drehteller(w, 0.5, holzig=False)
    w.brett("Bock", holzfarbe(w.z, "#6E4A2A", "#4E321A"), (0.35, 0.5, 0.65), (0, 0, 0.45), fase=0.03)
    w.saeule("Trommel", hell, 0.42, (0, 0.55, 1.0), (0, -0.35, 1.0), ecken=14)
    for y in (0.45, 0.05, -0.3):
        w.saeule("Band", einfarbig(TUCH_BLAU, 0.04, w.z), 0.45, (0, y + 0.04, 1.0), (0, y - 0.04, 1.0), ecken=14)
    w.dreh("Trichter", stein, [(0.4, 0.0), (0.46, 0.3), (0.75, 0.8), (0.72, 0.82)], (0, -0.35, 1.0), ecken=14, drehung=(90, 0, 0))
    for k in range(5):
        wi = math.radians(72 * k + 20)
        mitte = Vector((math.cos(wi) * 0.3, 0.7, 1.0 + math.sin(wi) * 0.3))
        w.brett("Flügelrad", holzfarbe(w.z, "#E8E2D0", "#C8C2B0"), (0.14, 0.03, 0.55), tuple(mitte), (0, -math.degrees(wi) + 90, 0), fase=0.01)


def _kopf_runen(w):
    stein = steinfarbe(w, "#5E5A68", "#77727F", "#928D9A", "#AAA6B2")
    rune = einfarbig("#7FE0FF", 0.05, w.z)
    _drehteller(w, 0.6, holzig=False)
    for sx in (-1, 1):
        for l in range(4):
            w.brett("Pfeilerstein", stein, (0.28, 0.34, 0.46), (sx * 0.58, 0, 0.3 + l * 0.48), fase=0.03)
    w.brett("Sturz", stein, (1.5, 0.36, 0.3), (0, 0, 2.2), fase=0.04)
    w.saeule("Kette", einfarbig(EISEN, 0.04, w.z), 0.04, (0, 0, 2.05), (0, 0, 1.5), ecken=5)
    w.dreh("Hammer", stein, [(0.0, 0.7), (0.42, 0.75), (0.45, 1.1), (0.4, 1.45), (0.0, 1.5)], (0, 0, 0), ecken=6)
    for k in range(6):
        wi = math.tau * k / 6 + math.pi / 6
        w.brett("Hammerrune", rune, (0.06, 0.03, 0.4), (math.cos(wi) * 0.43, math.sin(wi) * 0.43, 1.1), (0, 0, math.degrees(wi) + 90), fase=0.0, glut=True)


def _kopf_schatz(w):
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    gold = einfarbig(GOLD, 0.04, w.z)
    _drehteller(w, 0.7, holzig=False)
    w.brett("Truhe", holz, (1.2, 0.8, 0.55), (0, 0, 0.4), fase=0.03)
    for x in (-0.45, 0.45):
        w.brett("Truhenband", gold, (0.1, 0.84, 0.58), (x, 0, 0.4), fase=0.01)
    w.brett("Deckel", holz, (1.22, 0.1, 0.6), (0, 0.45, 0.95), (-20, 0, 0), fase=0.02)
    w.dreh("Goldberg", gold, [(0.0, 0.62), (0.55, 0.64), (0.4, 0.8), (0.0, 0.92)], (0, 0, 0), ecken=12)
    for k in range(9):
        wi = w.z.uniform(0, math.tau)
        rr = w.z.uniform(0.15, 0.45)
        w.dreh("Münze", gold, [(0.0, 0.0), (0.07, 0.0), (0.07, 0.02), (0.0, 0.02)], (math.cos(wi) * rr, math.sin(wi) * rr * 0.6, 0.85), ecken=8,
               drehung=(w.z.uniform(-40, 40), w.z.uniform(-40, 40), 0))
    w.dreh("Edelstein", einfarbig("#7FE0FF", 0.04, w.z), [(0.0, 0.0), (0.16, 0.12), (0.0, 0.36)], (0, 0, 0.9), ecken=6, glut=True)


KOEPFE = {
    "pfeil": _kopf_pfeil, "balliste": _kopf_balliste, "katapult": _kopf_katapult, "feuer": _kopf_feuer, "frost": _kopf_frost,
    "blitz": _kopf_blitz, "sonne": _kopf_sonne, "arkan": _kopf_arkan, "gift": _kopf_gift, "banner": _kopf_banner,
    "kaserne": _kopf_kaserne, "spaeher": _kopf_spaeher, "sturm": _kopf_sturm, "runen": _kopf_runen, "schatz": _kopf_schatz,
}


def kopf(art):
    w = Werk(400 + len(art) * 13 + sum(map(ord, art)))
    KOEPFE[art](w)
    return w.fertig(f"Kopf_{art}")



HOLZ = "#9A6B3F"
HOLZ_DUNKEL = "#6E4A2A"
HOLZ_HELL = "#C0935A"


# ---------------------------------------------------------------------------
# Fallen auf der Straße (Mitte im Ursprung, quer zur Straße entlang X)
# ---------------------------------------------------------------------------
def _falle_stacheln(bau, zufall):
    # Rahmen aus Bohlen, dazwischen Reihen eiserner Stacheln
    for seite in range(4):
        bau.teil(quader(3.2, 0.25, 0.22), HOLZ_DUNKEL, m=M((0, 0, 0), 90 * seite) @ M((0, -1.5, -0.1)))
    bau.teil(quader(3.0, 3.0, 0.12), HOLZ, m=M((0, 0, -0.08)))
    for i in range(6):
        for j in range(6):
            x, y = -1.25 + i * 0.5 + zufall.uniform(-0.06, 0.06), -1.25 + j * 0.5 + zufall.uniform(-0.06, 0.06)
            bau.teil(pyramide(0.07, zufall.uniform(0.35, 0.5), 4), "#6E7078", m=M((x, y, 0.02), zufall.uniform(0, 90), zufall.uniform(-10, 10), zufall.uniform(-10, 10)))


def _falle_teer(bau, zufall):
    # Schwarze Teerlache mit Blasen, Bretter am Rand und ein umgekipptes Fass
    punkte = []
    for k in range(18):
        w = math.tau * k / 18
        r = 1.9 * (1.0 + 0.12 * math.sin(w * 3 + 1.0) + zufall.uniform(-0.05, 0.05))
        punkte.append((math.cos(w) * r, math.sin(w) * r))
    lache = ([(x, y, 0.03) for x, y in punkte] + [(0.0, 0.0, 0.05)], [(k, (k + 1) % 18, 18) for k in range(18)])
    bau.teil(lache, "#17130F")
    bau.teil(zylinder(1.95, 0.03, 18, 1.9), "#2A221A", m=M((0, 0, -0.01)))
    for k in range(7):
        x, y = zufall.uniform(-1.2, 1.2), zufall.uniform(-1.2, 1.2)
        r = zufall.uniform(0.06, 0.16)
        bau.teil(drehkoerper([(0.0, 0.0), (r, r * 0.4), (0.0, r * 0.9)], 6), "#2B241E", m=M((x, y, 0.02)))
    for x in (-1.9, 1.9):
        bau.teil(quader(0.3, 2.6, 0.12), HOLZ, m=M((x, 0, 0.0), zufall.uniform(-8, 8)))
    fass = M((2.1, 1.2, 0.35), 30, 0, 90)
    bau.teil(zylinder(0.35, 0.8, 10), HOLZ, m=fass @ M((0, 0, -0.4)))
    for zz in (-0.3, 0.3):
        bau.teil(zylinder(0.37, 0.06, 10), EISEN, m=fass @ M((0, 0, zz)))


def _falle_barrikade(bau, zufall):
    # Spanische Reiter quer über die Straße: Balken mit gekreuzten, angespitzten Pfählen, dazu Säcke
    for x0 in (-1.6, 1.6):
        balken = M((x0, 0, 0.75), 0, 0, 90)
        bau.teil(zylinder(0.18, 3.0, 8), HOLZ_DUNKEL, m=balken @ M((0, 0, -1.5)))
        for i in range(5):
            x = x0 - 1.2 + i * 0.6
            for s in (-1, 1):
                pfahl = M((x, 0, 0.75), 0, 38 * s + zufall.uniform(-4, 4))
                bau.teil(zylinder(0.07, 2.2, 6), HOLZ, m=pfahl @ M((0, 0, -1.1)))
                bau.teil(pyramide(0.08, 0.3, 6), HOLZ_HELL, m=pfahl @ M((0, 0, 1.1)))
    for x in (-3.2, 3.2):
        for j in range(3):
            bau.teil(drehkoerper([(0.0, 0.0), (0.3, 0.05), (0.32, 0.18), (0.0, 0.3)], 8), "#B89C70", m=M((x + zufall.uniform(-0.1, 0.1), -0.3 + j * 0.3, 0.0 + (j % 2) * 0.25), 0, 0, 0, (1.3, 1.0, 1.0)))
    bau.teil(zylinder(0.05, 1.2, 6), HOLZ_DUNKEL, m=M((0.0, 0.5, 0)))
    bau.teil(platte([(0.0, 1.15), (0.55, 1.0), (0.0, 0.8)], 0.03, 0.015), TUCH_BLAU, m=M((0.03, 0.5, 0), 90))


FALLEN = {"falle_stacheln": _falle_stacheln, "falle_teer": _falle_teer, "barrikade": _falle_barrikade}


def falle(name):
    bau = Bau(600 + len(name))
    zufall = random.Random(sum(map(ord, name)))
    FALLEN[name](bau, zufall)
    return bau.fertig(name.capitalize(), glas_leuchten=2.0, ursprung=(0.0, 0.0))
