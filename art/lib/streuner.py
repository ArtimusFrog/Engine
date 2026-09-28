"""Streuner – Gegner, die frei über die Insel ziehen (game/src/wildnis.rs): Goblin, Goblin-Schamane,
Ork-Berserker, Echsenkrieger, Pilzling, Waldschrat, Minotaurus und Keiler.

Gebaut wie die Helden (figuren.py): ausgeformte Köpfe aus Metaballs mit Augen (Augapfel, Iris,
Pupille, Glanz), Gliedmaßen als Schläuche mit Muskeln, Hände als Metaball-Fäuste, Kleidung und
Rüstung als Lofts. Jede Figur hat ihr eigenes Skelett (Knochennamen wie beim Magier) mit eigenen
Maßen; Animationen Idle, Laufen, Angriff (gegner.py). Der Keiler ist ein Tier (tiere.py).
Koordinaten: Z oben, die Figur schaut nach -Y, Füße im Ursprung, links (L) ist +X.
"""

import math

import bmesh
from mathutils import Matrix, Quaternion, Vector

import tiere
from figuren import HAENGT, HOCH, Figur, _achsen, _glocke, _mischen, _platte, _schale, _schlauch, farbe, weich
from gegner import _angriff_hieb, _angriff_stampfen, _angriff_stoss, _angriff_zauber, _animationen
from werkstatt import animation

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))
KOPF = lambda co: {"Kopf": 1.0}
WEISS = farbe("#EDE6DA")
PUPILLE = farbe("#0E0B0A")


def _v(p, seite=1):
    return Vector((p[0] * seite, p[1], p[2]))


class Bau:
    """Maße einer Figur (Gelenke in Metern) mit Skelett, Gewichten und Körperteilen."""

    def __init__(self, f, **g):
        self.f = f
        self.g = g
        self.hoehe = g["scheitel"]

    def p(self, name, seite=1):
        return _v(self.g[name], seite)

    # --- Skelett ------------------------------------------------------------
    def skelett(self):
        g, f = self.g, self.f
        ky = g.get("kopf_y", 0.0)
        f.knochen_dazu("Becken", (0, 0, g["becken"]), (0, 0, g["bauch"]), None, HOCH)
        f.knochen_dazu("Bauch", (0, 0, g["bauch"]), (0, 0, g["brust"]), "Becken", HOCH)
        f.knochen_dazu("Brust", (0, 0, g["brust"]), (0, 0, g["hals"]), "Bauch", HOCH)
        f.knochen_dazu("Hals", (0, 0, g["hals"]), (0, ky, g["kopf"]), "Brust", HOCH)
        f.knochen_dazu("Kopf", (0, ky, g["kopf"]), (0, ky, g["scheitel"]), "Hals", HOCH)
        f.knochen_dazu("Hut", (0, ky + 0.02, g["scheitel"] + 0.04), (0, ky + 0.12, g["scheitel"] + 0.1), "Kopf", HOCH)
        for seite, sn in ((1, "L"), (-1, "R")):
            p = lambda n: tuple(self.p(n, seite))
            f.knochen_dazu(f"Oberarm.{sn}", p("schulter"), p("ellbogen"), "Brust", HAENGT)
            f.knochen_dazu(f"Unterarm.{sn}", p("ellbogen"), p("hand"), f"Oberarm.{sn}", HAENGT)
            f.knochen_dazu(f"Hand.{sn}", p("hand"), p("finger"), f"Unterarm.{sn}", HAENGT)
            f.knochen_dazu(f"Oberschenkel.{sn}", p("huefte"), p("knie"), "Becken", HAENGT)
            f.knochen_dazu(f"Unterschenkel.{sn}", p("knie"), p("knoechel"), f"Oberschenkel.{sn}", HAENGT)
            f.knochen_dazu(f"Fuss.{sn}", p("knoechel"), p("zehen"), f"Unterschenkel.{sn}", (0, 0, 1))

    # --- Gewichte -------------------------------------------------------------
    def rumpf(self, co):
        g = self.g
        d = 0.035 * self.hoehe
        b, c, h = g["bauch"], g["brust"], g["hals"]
        z = co.z
        return _mischen(("Becken", 1 - weich(b - d, b + d, z)), ("Bauch", weich(b - d, b + d, z) * (1 - weich(c - d, c + d, z))),
                        ("Brust", weich(c - d, c + d, z) * (1 - weich(h - d, h + d * 0.5, z))), ("Hals", weich(h - d, h + d * 0.5, z)))

    def hals_kopf(self, co):
        k = self.g["kopf"]
        d = 0.02 * self.hoehe
        return _mischen(("Kopf", weich(k - d, k + d, co.z)), ("Hals", 1 - weich(k - d, k + d, co.z)))

    def rock(self, co):
        """Oben am Rumpf, unten schwingt der Rock mit den Oberschenkeln."""
        b = self.g["becken"]
        if co.z > b:
            return self.rumpf(co)
        bein = weich(b, b - 0.35 * b, co.z)
        links = weich(-0.1, 0.1, co.x)
        return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))

    def arm(self, seite):
        s = "L" if seite > 0 else "R"
        schulter, ellbogen, hand = self.p("schulter", seite), self.p("ellbogen", seite), self.p("hand", seite)

        def gewichte(co):
            oben = ellbogen - schulter
            t1 = (co - schulter).dot(oben) / oben.length_squared
            unten = hand - ellbogen
            t2 = (co - ellbogen).dot(unten) / unten.length_squared
            if t1 < 0.15:
                return _mischen(("Brust", 1 - weich(-0.1, 0.15, t1)), (f"Oberarm.{s}", weich(-0.1, 0.15, t1)))
            if t2 < 0.0:
                return _mischen((f"Oberarm.{s}", 1 - weich(-0.15, 0.1, t2)), (f"Unterarm.{s}", weich(-0.15, 0.1, t2)))
            if t2 > 0.95:
                return _mischen((f"Unterarm.{s}", 1 - weich(0.95, 1.1, t2)), (f"Hand.{s}", weich(0.95, 1.1, t2)))
            return {f"Unterarm.{s}": 1.0}
        return gewichte

    def bein(self, seite):
        s = "L" if seite > 0 else "R"
        huefte, knie, knoechel = self.p("huefte", seite), self.p("knie", seite), self.p("knoechel", seite)

        def gewichte(co):
            if co.z < knoechel.z + 0.02:
                return _mischen((f"Fuss.{s}", 1 - weich(knoechel.z - 0.01, knoechel.z + 0.03, co.z)),
                                (f"Unterschenkel.{s}", weich(knoechel.z - 0.01, knoechel.z + 0.03, co.z)))
            oben = knie - huefte
            t1 = (co - huefte).dot(oben) / oben.length_squared
            unten = knoechel - knie
            t2 = (co - knie).dot(unten) / unten.length_squared
            if t1 < 0.12:
                return _mischen(("Becken", 1 - weich(-0.12, 0.12, t1)), (f"Oberschenkel.{s}", weich(-0.12, 0.12, t1)))
            if t2 < 0.05:
                return _mischen((f"Oberschenkel.{s}", 1 - weich(-0.12, 0.08, t2)), (f"Unterschenkel.{s}", weich(-0.12, 0.08, t2)))
            return {f"Unterschenkel.{s}": 1.0}
        return gewichte

    def hand(self, seite):
        s = "L" if seite > 0 else "R"
        return lambda co: {f"Hand.{s}": 1.0}

    def fuss(self, seite):
        s = "L" if seite > 0 else "R"
        return lambda co: {f"Fuss.{s}": 1.0}

    # --- Körperteile ----------------------------------------------------------
    def rumpf_loft(self, name, ringe, seg, farbe_von, gewicht=None, oben_zu=True, unten_zu=True, teilung=2):
        """Ringe: (z, rx, ry[, dy[, form]]) von unten nach oben."""
        liste = []
        for r in ringe:
            z, rx, ry = r[:3]
            dy = r[3] if len(r) > 3 else 0.0
            ring = (Vector((0, dy, z)), X, Y, rx, ry)
            if len(r) > 4:
                ring += (r[4],)
            liste.append(ring)
        return self.f.loft(name, liste, seg, farbe_von, gewicht or self.rumpf, oben_zu=oben_zu, unten_zu=unten_zu, teilung=teilung, glatt=True)

    def arm_schlauch(self, seite, radien, farbe_von, name="Arm", seg=16, form=None):
        """Radien: Schulter, Bizeps, Ellbogen, Unterarm, Handgelenk."""
        s, e, h = self.p("schulter", seite), self.p("ellbogen", seite), self.p("hand", seite)
        innen = Vector((-0.03 * seite, 0, 0.02))
        punkte = [s + innen, s.lerp(e, 0.45), e, e.lerp(h, 0.4), h]
        return _schlauch(self.f, name, punkte, radien, seg, farbe_von, self.arm(seite), form=form)

    def bein_schlauch(self, seite, radien, farbe_von, name="Bein", seg=16, form=None, bis=1.0):
        """Radien: Hüfte, Oberschenkel, Knie, Wade, Knöchel (`bis` < 1 endet über dem Knöchel)."""
        h, k, a = self.p("huefte", seite), self.p("knie", seite), self.p("knoechel", seite)
        punkte = [h + Vector((0, 0, 0.04)), h.lerp(k, 0.4), k, k.lerp(a, 0.35), k.lerp(a, bis)]
        return _schlauch(self.f, name, punkte, radien, seg, farbe_von, self.bein(seite), form=form)

    def griff(self, seite):
        """Mitte der Faust (dort hält sie eine Waffe)."""
        return self.p("hand", seite).lerp(self.p("finger", seite), 0.5)

    def faust(self, seite, g, farbe_von, krallen=None, name="Faust"):
        """Faust mit vier gekrümmten Fingern und Daumen (g = Größe, 1 ≈ Menschenhand)."""
        griff = self.griff(seite)
        formen = [(griff + Vector((0.026 * seite, 0.01, 0.028)) * g, (0.034 * g, 0.042 * g, 0.046 * g))]
        formen[0] = (griff + Vector((0.026 * seite * g, 0.01 * g, 0.028 * g)), formen[0][1])
        knoechel = []
        for i in range(4):
            z = griff.z + (0.022 - 0.021 * i) * g
            for j, w in enumerate((0.4, -0.5, -1.5, -2.5)):
                rad = (0.034 - 0.002 * j) * g
                formen.append((Vector((griff.x - math.cos(w) * rad * seite, griff.y + math.sin(w) * rad, z)), (0.012 * g,) * 3))
            knoechel.append(Vector((griff.x - math.cos(-1.5) * 0.034 * g * seite, griff.y - 0.036 * g, z)))
        for p in (Vector((0.03 * seite, -0.03, 0.035)), Vector((0.01 * seite, -0.045, 0.04)), Vector((-0.012 * seite, -0.04, 0.042))):
            formen.append((griff + p * g, (0.013 * g,) * 3))
        self.f.metaball(name, formen, 0.0042 * g, 1500, farbe_von, self.hand(seite), glatt=True)
        if krallen is not None:
            for k in knoechel:
                self.f.straehne("Kralle", [k + Vector((0, 0.0, -0.005 * g)), k + Vector((0, -0.016 * g, -0.012 * g)), k + Vector((0, -0.02 * g, -0.026 * g))],
                                0.006 * g, 0.001, krallen, self.hand(seite), 5, 0.0, 1.0)

    def fuss_nackt(self, seite, laenge, breite, farbe_von, zehen=3, krallen=None, name="Fuss"):
        """Nackter Fuß mit Ferse, Ballen und Zehen (Spitze nach -Y), optional mit Krallen."""
        a = self.p("knoechel", seite)
        boden = Vector((a.x, a.y, 0.0))
        formen = [(boden + Vector((0, 0.02, 0.05)), (breite * 0.42, 0.06, 0.05)),
                  (boden + Vector((0, -laenge * 0.35, 0.035)), (breite * 0.5, laenge * 0.3, 0.035))]
        spitzen = []
        for i in range(zehen):
            x = (i - (zehen - 1) / 2) * breite * 0.6 / max(1, zehen - 1) * 1.6
            spitze = boden + Vector((x, -laenge * 0.78, 0.025))
            formen.append((spitze, (breite * 0.16, laenge * 0.14, 0.024)))
            spitzen.append(spitze)
        self.f.metaball(name, formen, 0.005, 1200, farbe_von, self.fuss(seite), glatt=True)
        if krallen is not None:
            for s_ in spitzen:
                self.f.straehne("Kralle", [s_ + Vector((0, -laenge * 0.09, 0.005)), s_ + Vector((0, -laenge * 0.17, 0.0)), s_ + Vector((0, -laenge * 0.22, -0.018))],
                                0.012, 0.002, krallen, self.fuss(seite), 5, 0.0, 1.0)

    def augen(self, orte, r, iris, schlitz=False, weiss=WEISS, blick=Vector((0, -1, 0)), gewicht=KOPF):
        for ort in orte:
            ort = Vector(ort)
            self.f.kugel("Augapfel", ort, (r, r, r), weiss, gewicht, 14, 10)
            self.f.kugel("Iris", ort + blick * r * 0.72, (r * 0.62,) * 3, iris, gewicht, 12, 8)
            groesse = (r * 0.13, r * 0.13, r * 0.5) if schlitz else (r * 0.3,) * 3
            self.f.kugel("Pupille", ort + blick * r * 0.95, groesse, PUPILLE, gewicht, 10, 6)
            self.f.kugel("Glanz", ort + blick * r * 1.02 + Vector((0.3 * r, 0, 0.35 * r)), (r * 0.14,) * 3, farbe("#FFFFFF"), gewicht, 6, 4)

    def starr(self, name, knochen, bauen):
        """Waffe/Schild als starres Anbauteil: `bauen()` baut die Teile."""
        anfang = len(self.f.teile)
        bauen()
        self.f.als_starr(name, knochen, anfang)


def _falten(r, staerke, n=6):
    werte = [r.uniform(0.6, 1.4) for _ in range(n)]
    return lambda w: 1.0 + staerke * sum(math.sin(w * (3 + i) + werte[i]) * werte[i] / (3 + i) for i in range(n))


def _zacken(r, tiefe, n=9):
    """Ausgefranster Saum: Ring-Form mit spitzen Zacken."""
    phasen = [r.uniform(0, 1) for _ in range(n)]
    return lambda w: 1.0 - tiefe * max(0.0, math.sin(w * n / 2 + phasen[int(w / math.tau * n) % n] * 2))


def _vorne(k, seg):
    """Winkelabstand eines Loft-Segments von vorne (-Y), 0 … π."""
    w = math.tau * (k + 0.5) / seg
    return abs(math.atan2(math.sin(w + math.pi / 2), math.cos(w + math.pi / 2)))


def _strecke(f, name, a, b, r_a, r_b, c, gewicht, seg=10):
    q1, q2 = _achsen(a, b)
    farbe_von = c if callable(c) else (lambda i, k, p: c)
    return f.loft(name, [(a, q1, q2, r_a, r_a), (b, q1, q2, r_b, r_b)], seg, farbe_von, gewicht, oben_zu=True, unten_zu=True, glatt=True)


def _kegel(f, name, basis, richtung, laenge, radius, c, gewicht, seg=8, krumm=None):
    """Spitzer Kegel (Stachel, Zahn, Horn); `krumm` biegt die Spitze in diese Richtung."""
    richtung = richtung.normalized()
    punkte = [basis, basis + richtung * laenge * 0.5, basis + richtung * laenge]
    if krumm is not None:
        punkte[1] += krumm * laenge * 0.12
        punkte[2] += krumm * laenge * 0.4
    return f.straehne(name, punkte, radius, radius * 0.08, c, gewicht, seg, 0.0, 1.0)


# ===========================================================================
# Goblin und Goblin-Schamane
# ===========================================================================
def _goblin(f, haut, haut_hell, gebueckt=0.0):
    """Klein (gut 1,1 m), dürr mit Kugelbauch, großer Kopf mit Hakennase, langen Ohren und
    gelben Schlitzaugen; lange Arme, große Hände und Füße mit Krallen."""
    r = f.rng
    ky = -0.04 - gebueckt
    b = Bau(f, becken=0.5, bauch=0.58, brust=0.7, hals=0.84, kopf=0.88, scheitel=1.14, kopf_y=ky,
            schulter=(0.14, 0.0, 0.81), ellbogen=(0.21, 0.03, 0.64), hand=(0.245, 0.0, 0.47), finger=(0.255, -0.03, 0.4),
            huefte=(0.08, 0.0, 0.5), knie=(0.1, -0.03, 0.29), knoechel=(0.1, 0.03, 0.07), zehen=(0.1, -0.12, 0.02))
    kralle = farbe("#2E2A22")
    lende, lende_dunkel = farbe("#6B4A2C"), farbe("#4A3220")

    def haut_farbe(i, k, p):
        vorne = p.normal.y < -0.4 and p.center.z < 0.72
        return (haut_hell if vorne else haut) * (0.94 + 0.08 * max(0.0, p.normal.z))

    # Rumpf: schmale Brust, runder Bauch, spitze Schultern
    b.rumpf_loft("Rumpf", [(0.46, 0.085, 0.07), (0.53, 0.105, 0.09, -0.01), (0.6, 0.125, 0.115, -0.03), (0.66, 0.122, 0.11, -0.03),
                           (0.72, 0.112, 0.085, -0.01), (0.78, 0.125, 0.075), (0.82, 0.1, 0.065), (0.855, 0.045, 0.04, -0.01)], 28, haut_farbe)
    # Rippen und Schlüsselbeine angedeutet
    for s in (1, -1):
        _strecke(f, "Schluesselbein", Vector((0.02 * s, -0.06, 0.805)), Vector((0.11 * s, -0.035, 0.815)), 0.012, 0.01, haut, b.rumpf, 6)
    # Lendenschurz mit Fransen, Gürtel mit Knochenschnalle, Riemen über der Schulter
    b.f.loft("Lendenschurz", [(Vector((0, -0.005, 0.54)), X, Y, 0.11, 0.098), (Vector((0, -0.01, 0.45)), X, Y, 0.125, 0.11, _falten(r, 0.06)),
                              (Vector((0, -0.01, 0.36)), X, Y, 0.135, 0.12, _zacken(r, 0.18, 11))], 30,
             lambda i, k, p: lende_dunkel if i > 1.6 else lende * (0.86 if k % 5 == 0 else 1.0), b.rock, teilung=2, glatt=True)
    b.f.loft("Guertel", [(Vector((0, -0.005, 0.53)), X, Y, 0.112, 0.1), (Vector((0, -0.005, 0.565)), X, Y, 0.114, 0.103)], 24,
             lambda i, k, p: farbe("#3A2616"), b.rumpf, teilung=1, glatt=True)
    b.f.kugel("Schnalle", (0, -0.105, 0.548), (0.022, 0.012, 0.018), farbe("#DCD4C0"), b.rumpf, 10, 6)
    riemen = [Vector((0.1, -0.06, 0.8)), Vector((0.02, -0.105, 0.7)), Vector((-0.08, -0.11, 0.6)), Vector((-0.11, -0.05, 0.54))]
    b.f.straehne("Riemen", riemen, 0.016, 0.016, farbe("#3A2616"), b.rumpf, 6, 0.0, 0.35)
    b.f.kiste("Beutel", (-0.115, -0.06, 0.5), (0.05, 0.035, 0.06), lende, b.rumpf)
    # Arme: dürr, knotige Ellbogen, Stoffwickel an den Handgelenken
    for s in (1, -1):
        b.arm_schlauch(s, [0.042, 0.036, 0.03, 0.033, 0.024], haut_farbe)
        b.f.kugel("Ellbogen", b.p("ellbogen", s) + Vector((0, 0.012, 0)), (0.028, 0.03, 0.03), haut, b.arm(s), 10, 8)
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        b.f.loft("Wickel", [(e.lerp(h, 0.72), q1, q2, 0.034, 0.034), (e.lerp(h, 0.95), q1, q2, 0.03, 0.03)], 12,
                 lambda i, k, p: farbe("#8A7A5A") * (0.85 if k % 3 == 0 else 1.0), b.arm(s), glatt=True)
        b.faust(s, 1.05, lambda poly: haut * (0.93 if poly.normal.z < 0 else 1.0), krallen=kralle)
    # Beine: dünn, knubbelige Knie, große Füße mit Krallen
    for s in (1, -1):
        b.bein_schlauch(s, [0.055, 0.045, 0.036, 0.038, 0.028], haut_farbe)
        b.f.kugel("Knie", b.p("knie", s) + Vector((0, -0.02, 0)), (0.032, 0.03, 0.035), haut, b.bein(s), 10, 8)
        b.fuss_nackt(s, 0.2, 0.1, lambda poly: haut * 0.95, zehen=3, krallen=kralle)

    # Kopf: großer Schädel, Hakennase, breites Grinsen mit Zähnen, schwere Brauen
    k = Vector((0, ky, 0.98))
    kopf = [
        (k + Vector((0, 0.02, 0.03)), (0.11, 0.12, 0.105)),             # Schädel
        (k + Vector((0, -0.04, -0.045)), (0.1, 0.075, 0.06)),           # Kiefer, breit
        (k + Vector((0, -0.1, 0.015)), (0.085, 0.025, 0.02)),           # Brauenwulst
        (k + Vector((0, -0.12, -0.01)), (0.02, 0.03, 0.03)),            # Nasenwurzel
        (k + Vector((0, -0.155, -0.03)), (0.022, 0.04, 0.024)),         # Nase …
        (k + Vector((0, -0.19, -0.055)), (0.018, 0.03, 0.022)),         # … lang und hakig
        (k + Vector((0, -0.2, -0.075)), (0.013, 0.014, 0.018)),
        (k + Vector((0, -0.105, -0.075)), (0.07, 0.016, 0.006), True),  # breites Maul
        (k + Vector((0, -0.08, -0.105)), (0.045, 0.03, 0.02)),          # spitzes Kinn
        (k + Vector((0, 0.0, -0.12)), (0.04, 0.04, 0.05)),              # dünner Hals
    ]
    for s in (1, -1):
        kopf += [(k + Vector((0.042 * s, -0.098, 0.0)), (0.026, 0.018, 0.02), True),       # Augenhöhle
                 (k + Vector((0.065 * s, -0.08, -0.035)), (0.03, 0.025, 0.022))]          # Wangen
    b.f.metaball("Kopf", kopf, 0.0045, 3200, lambda poly: haut * (0.9 if poly.center.z < k.z - 0.08 else 1.0) * (0.95 + 0.07 * max(0.0, poly.normal.z)),
                 b.hals_kopf, glatt=True)
    b.augen([k + Vector((0.042 * s, -0.093, 0.0)) for s in (1, -1)], 0.021, farbe("#F2C21C"), schlitz=True, weiss=farbe("#E8E0A8"))
    # Schwere Oberlider (verschlagener Blick)
    for s in (1, -1):
        b.f.kugel("Lid", k + Vector((0.042 * s, -0.1, 0.014)), (0.026, 0.016, 0.009), haut * 0.9, KOPF, 12, 6)
    # Zähne: spitz, schief, oben und unten
    for i, x in enumerate((-0.05, -0.03, -0.01, 0.012, 0.032, 0.052)):
        oben = i % 2 == 0
        basis = k + Vector((x, -0.12 + abs(x) * 0.35, -0.07 if oben else -0.082))
        _kegel(f, "Zahn", basis, Vector((r.uniform(-0.2, 0.2), -0.2, -1 if oben else 1)), r.uniform(0.018, 0.028), 0.006, farbe("#E8DDB8"), KOPF, 5)
    # Lange, spitze Ohren (leicht nach oben und hinten), innen rosig
    for s in (1, -1):
        wurzel = k + Vector((0.1 * s, 0.0, 0.0))
        punkte = [wurzel, wurzel + Vector((0.08 * s, 0.03, 0.03)), wurzel + Vector((0.16 * s, 0.07, 0.07)), wurzel + Vector((0.22 * s, 0.1, 0.1))]
        b.f.straehne("Ohr", punkte, 0.045, 0.003, haut, KOPF, 10, 0.0, 0.28)
        b.f.straehne("Ohr innen", [p + Vector((0, -0.008, 0.004)) for p in punkte[:3]], 0.03, 0.004, farbe("#C98A7A"), KOPF, 8, 0.0, 0.2)
    # Ein paar Haarbüschel
    for i in range(7):
        w = r.uniform(-0.8, 0.8)
        basis = k + Vector((math.sin(w) * 0.06, 0.03 + r.uniform(-0.03, 0.04), 0.13))
        b.f.straehne("Haar", [basis, basis + Vector((math.sin(w) * 0.03, 0.03, 0.05)), basis + Vector((math.sin(w) * 0.07, 0.07, 0.07))],
                     0.008, 0.001, farbe("#2A2420"), KOPF, 5, 0.0, 1.0)
    return b


def _hackmesser(b, rostig=True):
    """Grobes Hackmesser mit gezackter Klinge in der rechten Hand."""
    f = b.f
    griff = b.griff(-1)
    unten = Vector((0, -0.55, -0.83)).normalized()        # Klinge schräg nach vorne unten
    quer = unten.cross(X).normalized()
    stahl, rost = farbe("#8E8E88"), farbe("#7A4A2E")
    _strecke(f, "Heft", griff - unten * 0.06, griff + unten * 0.08, 0.017, 0.017, farbe("#4A3220"), b.hand(-1))
    a = griff + unten * 0.08
    ringe = []
    for j in range(8):
        t = j / 7
        breite = 0.045 + 0.03 * t
        zacke = 0.012 if j % 2 else 0.0
        ringe.append((a + unten * 0.4 * t + quer * (breite * 0.5 - zacke * 0.5), quer, X, breite * 0.5 + zacke * 0.5, 0.006))
    f.loft("Klinge", ringe, 6, lambda i, k, p: rost if (rostig and (i * 3 + k) % 5 == 0) else stahl * (0.85 + 0.2 * max(0.0, p.normal.z)),
           b.hand(-1), oben_zu=True, unten_zu=True)


def goblin(seed=401):
    f = Figur("Goblin", seed)
    b = _goblin(f, farbe("#5C7A34"), farbe("#86A04E"))
    b.starr("Hackmesser", "Hand.R", lambda: _hackmesser(b))
    b.skelett()
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-10, -25), (-18, -45)), gehen=(30, 46, 20, 0.03, 10, 18)))


def goblin_schamane(seed=402):
    f = Figur("GoblinSchamane", seed)
    r = f.rng
    haut, hell = farbe("#4E6A34"), farbe("#76925A")
    b = _goblin(f, haut, hell, gebueckt=0.04)
    k = Vector((0, -0.08, 0.98))
    federn = [farbe("#C8322A"), farbe("#E8B030"), farbe("#2E6AA8"), farbe("#F2EEE0")]
    # Vogelschädel als Maske auf der Stirn, darüber ein Federkranz
    b.f.kugel("Vogelschaedel", k + Vector((0, -0.05, 0.12)), (0.06, 0.07, 0.045), farbe("#E4DAC2"), KOPF, 14, 10)
    _kegel(f, "Schnabel", k + Vector((0, -0.11, 0.12)), Vector((0, -1, -0.5)), 0.12, 0.03, farbe("#D6C69A"), KOPF, 8, krumm=Vector((0, 0, -1)))
    for s in (1, -1):
        b.f.kugel("Augenloch", k + Vector((0.03 * s, -0.1, 0.13)), (0.014, 0.01, 0.012), farbe("#1A140E"), KOPF, 8, 6)
    for i in range(9):
        w = -1.2 + 2.4 * i / 8
        basis = k + Vector((math.sin(w) * 0.09, 0.03 + math.cos(w) * 0.02, 0.13))
        spitze = basis + Vector((math.sin(w) * 0.12, 0.08, 0.22 - abs(w) * 0.06))
        b.f.straehne("Feder", [basis, basis.lerp(spitze, 0.5) + Vector((0, 0.02, 0.02)), spitze], 0.018, 0.003, federn[i % 4], KOPF, 6, 0.0, 0.3)
    # Knochenkette mit Zähnen und ein Umhang aus Blättern und Fell
    for i in range(13):
        w = math.pi * (0.15 + 0.7 * i / 12)
        p = Vector((math.cos(w) * 0.1, -0.07 - math.sin(w) * 0.025, 0.8 - math.sin(w) * 0.07))
        if i % 3 == 1:
            _kegel(f, "Kettenzahn", p, Vector((0, -0.3, -1)), 0.035, 0.009, farbe("#E8DDB8"), b.rumpf, 5)
        else:
            b.f.kugel("Perle", p, (0.011, 0.011, 0.011), [farbe("#B8322A"), farbe("#DCD4C0"), farbe("#2E6AA8")][i % 3], b.rumpf, 8, 6)
    blatt, blatt_dunkel = farbe("#4E7A2A"), farbe("#35561E")
    _glocke(f, "Umhang", [(Vector((0, 0.02, 0.83)), 0.13, 0.1), (Vector((0, 0.06, 0.62)), 0.19, 0.15), (Vector((0, 0.08, 0.38)), 0.22, 0.17)],
            (70, 290), lambda poly: (blatt if int(poly.center.z * 40 + poly.center.x * 30) % 3 else blatt_dunkel) * (0.9 + 0.2 * max(0.0, poly.normal.z)),
            b.rumpf, seg=34, welle=0.08)
    # Knorriger Stab mit grünem Kristall, Federn und klappernden Knochen (rechte Hand)

    def stab():
        g = b.griff(-1)
        oben = Vector((0.02, 0.2, 1.0)).normalized()
        holz = farbe("#5A3E26")
        punkte = [g - oben * 0.38 + Vector((0.01, 0, 0)), g - oben * 0.1, g + oben * 0.25 + Vector((0.012, 0, 0)), g + oben * 0.55 - Vector((0.01, 0, 0)),
                  g + oben * 0.72]
        b.f.straehne("Stab", punkte, 0.018, 0.014, lambda i, k2, p: holz * (0.8 if k2 % 3 == 0 else 1.0), b.hand(-1), 8, 0.4, 1.0)
        spitze = g + oben * 0.8
        for j in range(4):
            w = math.tau * j / 4
            _kegel(f, "Krallenfassung", spitze + Vector((math.cos(w) * 0.03, math.sin(w) * 0.03, -0.06)), Vector((math.cos(w) * 0.3, math.sin(w) * 0.3, 1)),
                   0.1, 0.01, holz * 0.8, b.hand(-1), 5, krumm=Vector((-math.cos(w), -math.sin(w), 0)))
        b.f.kugel("Kristall", spitze, (0.035, 0.035, 0.05), farbe("#7CFF6A"), b.hand(-1), 8, 6, glatt=False)
        for j, farbe_feder in enumerate(federn[:3]):
            a = spitze + Vector((0.02 * (j - 1), 0.01, -0.09))
            b.f.straehne("Stabfeder", [a, a + Vector((0.03 * (j - 1), 0.01, -0.07)), a + Vector((0.05 * (j - 1), 0.02, -0.14))], 0.012, 0.002,
                         farbe_feder, b.hand(-1), 5, 0.0, 0.3)
    b.starr("Stab", "Hand.R", stab)
    b.skelett()
    return f.fertig(_animationen(_angriff_zauber, arme_ruhe=((-8, -20), (-8, -20)), gehen=(24, 38, 14, 0.025, 14, 22)))


def _haut_poly(c, hell):
    return lambda poly: c.lerp(hell, max(0.0, -poly.normal.y) * 0.3) * (0.92 + 0.1 * max(0.0, poly.normal.z))


# ===========================================================================
# Ork-Berserker
# ===========================================================================
def ork_berserker(seed=403):
    """Gut zwei Meter, massiger Oberkörper, kleiner Kopf mit Unterbiss und Hauern, Haarknoten,
    rote Kriegsbemalung; Pelzschurz, Lederharnisch, Stachel-Schulterpanzer, große Streitaxt."""
    f = Figur("OrkBerserker", seed)
    r = f.rng
    b = Bau(f, becken=1.02, bauch=1.18, brust=1.4, hals=1.7, kopf=1.74, scheitel=2.02, kopf_y=-0.08,
            schulter=(0.31, 0.0, 1.64), ellbogen=(0.42, 0.04, 1.33), hand=(0.46, 0.0, 1.06), finger=(0.47, -0.03, 0.95),
            huefte=(0.14, 0.0, 1.0), knie=(0.16, -0.03, 0.56), knoechel=(0.16, 0.03, 0.1), zehen=(0.16, -0.17, 0.03))
    haut, haut_hell = farbe("#556838"), farbe("#71844C")
    bemalung = farbe("#8E1E1A")
    leder, leder_dunkel = farbe("#5A3A22"), farbe("#3A2416")
    eisen, eisen_hell = farbe("#4A4C50"), farbe("#7A7E86")
    fell, fell_hell = farbe("#5E4430"), farbe("#8A6A4A")
    muskel = _haut_poly(haut, haut_hell)

    def haut_farbe(i, k, p):
        c = haut.lerp(haut_hell, max(0.0, -p.normal.y) * 0.3) * (0.92 + 0.1 * max(0.0, p.normal.z))
        # Kriegsbemalung: Streifen quer über Brust und Oberarme
        if abs(p.center.x) > 0.3 and (1.46 < p.center.z < 1.5 or 1.53 < p.center.z < 1.56):
            return bemalung
        return c

    b.rumpf_loft("Rumpf", [(0.98, 0.19, 0.15), (1.1, 0.2, 0.17, -0.01), (1.22, 0.23, 0.19, -0.03), (1.34, 0.29, 0.21, -0.03),
                           (1.46, 0.33, 0.23, -0.03), (1.56, 0.34, 0.22, -0.02), (1.63, 0.3, 0.19), (1.69, 0.19, 0.15, 0.0),
                           (1.73, 0.1, 0.1, -0.03)], 40, haut_farbe)
    for s in (1, -1):
        b.f.kugel("Brustmuskel", (0.13 * s, -0.19, 1.5), (0.14, 0.06, 0.1), muskel, b.rumpf, 16, 10)
        b.f.kugel("Nacken", (0.14 * s, 0.02, 1.67), (0.12, 0.1, 0.07), muskel, b.rumpf, 12, 8)
    for i in range(3):
        for s in (1, -1):
            b.f.kugel("Bauchmuskel", (0.05 * s, -0.19 + 0.01 * i, 1.33 - 0.07 * i), (0.045, 0.025, 0.032), muskel, b.rumpf, 10, 6)
    # Harnisch: zwei gekreuzte Riemen mit Eisenring, breiter Gürtel mit Schädelschnalle
    for s in (1, -1):
        punkte = [Vector((0.24 * s, -0.05, 1.63)), Vector((0.12 * s, -0.22, 1.5)), Vector((0.0, -0.235, 1.38)), Vector((-0.12 * s, -0.22, 1.22)),
                  Vector((-0.2 * s, -0.12, 1.1))]
        b.f.straehne("Harnischriemen", punkte, 0.032, 0.032, lambda i, k, p: leder_dunkel if k % 4 == 0 else leder, b.rumpf, 6, 0.0, 0.3)
    b.f.loft("Harnischring", [(Vector((0, -0.25, 1.38)), X, Z, 0.05, 0.05), (Vector((0, -0.24, 1.38)), X, Z, 0.035, 0.035)], 16,
             lambda i, k, p: eisen_hell, b.rumpf, glatt=True)
    b.f.loft("Guertel", [(Vector((0, -0.01, 1.0)), X, Y, 0.205, 0.165), (Vector((0, -0.01, 1.1)), X, Y, 0.21, 0.172)], 36,
             lambda i, k, p: leder_dunkel * (0.8 if k % 6 == 0 else 1.0), b.rumpf, glatt=True)
    b.f.kugel("Schaedelschnalle", (0, -0.18, 1.05), (0.055, 0.035, 0.06), farbe("#DCD4C0"), b.rumpf, 14, 10)
    for s in (1, -1):
        b.f.kugel("Schnallenauge", (0.02 * s, -0.212, 1.06), (0.013, 0.008, 0.013), farbe("#1A1410"), b.rumpf, 8, 6)
    b.f.loft("Pelzschurz", [(Vector((0, -0.01, 1.02)), X, Y, 0.21, 0.17), (Vector((0, -0.02, 0.86)), X, Y, 0.24, 0.2, _falten(r, 0.07)),
                            (Vector((0, -0.02, 0.68)), X, Y, 0.26, 0.22, _zacken(r, 0.22, 13))], 40,
             lambda i, k, p: (fell_hell if i > 1.7 else fell) * (0.85 if k % 3 == 0 else 1.0), b.rock, teilung=2, glatt=True)
    for s in (1, -1):
        b.arm_schlauch(s, [0.13, 0.12, 0.09, 0.1, 0.075], haut_farbe, seg=20)
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        b.f.loft("Armschiene", [(e.lerp(h, 0.42), q1, q2, 0.1, 0.1), (e.lerp(h, 0.98), q1, q2, 0.085, 0.085)], 18,
                 lambda i, k, p: leder_dunkel if k % 5 == 0 else leder, b.arm(s), teilung=2, glatt=True)
        for j in range(3):
            b.f.kugel("Niete", e.lerp(h, 0.55 + 0.15 * j) + Vector((0.09 * s, -0.03, 0)), (0.013, 0.013, 0.013), eisen_hell, b.arm(s), 8, 6)
        b.faust(s, 1.55, lambda poly: haut * (0.92 if poly.normal.z < 0 else 1.0))
    # Stachel-Schulterpanzer links (drei Lagen)
    sl = b.p("schulter", 1)
    for j in range(3):
        _schale(f, "Schulterplatte", sl + Vector((0.02, 0.0, 0.05 - 0.06 * j)), 0.17 - 0.015 * j, 0.17 - 0.015 * j, 0.12, (0, 360), (0, 80),
                lambda poly, j=j: eisen.lerp(eisen_hell, max(0.0, poly.normal.z) * 0.6) * (1.0 - 0.08 * j), b.arm(1), seg=(24, 8))
    for j in range(3):
        w = -0.6 + 0.6 * j
        basis = sl + Vector((0.06 + 0.04 * math.sin(w), 0.1 * math.sin(w), 0.15))
        _kegel(f, "Stachel", basis, Vector((0.4, math.sin(w) * 0.3, 1)), 0.16, 0.028, eisen_hell, b.arm(1), 8)
    hose = farbe("#3E3024")
    for s in (1, -1):
        b.bein_schlauch(s, [0.15, 0.135, 0.1, 0.105, 0.08], lambda i, k, p: hose if i < 2.1 else haut * 0.95, seg=20)
        a = b.p("knoechel", s)
        b.f.loft("Fellwickel", [(a + Vector((0, 0, 0.06)), X, Y, 0.095, 0.095, _falten(r, 0.08)), (a + Vector((0, 0, 0.26)), X, Y, 0.105, 0.105, _falten(r, 0.08))],
                 24, lambda i, k, p: fell_hell if k % 4 == 0 else fell, b.bein(s), teilung=2, glatt=True)
        fussring = [(Vector((a.x, 0.07, 0.06)), X, Z, 0.08, 0.06), (Vector((a.x, 0.04, 0.025)), X, Z, 0.09, 0.035),
                    (Vector((a.x, -0.07, 0.06)), X, Z, 0.095, 0.06), (Vector((a.x, -0.18, 0.05)), X, Z, 0.08, 0.045),
                    (Vector((a.x, -0.22, 0.05)), X, Z, 0.01, 0.01)]
        b.f.loft("Stiefel", fussring, 18, lambda i, k, p: leder_dunkel * (0.6 if p.center.z < 0.015 else 1.0), b.fuss(s), oben_zu=True, unten_zu=True,
                 teilung=2, glatt=True)

    # Kopf: kleiner Schädel, wuchtiger Unterkiefer, flache breite Nase, schwere Brauen
    k = Vector((0, -0.1, 1.86))
    kopf = [
        (k + Vector((0, 0.03, 0.04)), (0.11, 0.12, 0.11)),
        (k + Vector((0, -0.04, -0.07)), (0.12, 0.1, 0.075)),
        (k + Vector((0, -0.11, -0.085)), (0.085, 0.04, 0.05)),
        (k + Vector((0, -0.105, 0.035)), (0.1, 0.035, 0.028)),
        (k + Vector((0, -0.135, 0.0)), (0.035, 0.03, 0.03)),
        (k + Vector((0.022, -0.14, -0.015)), (0.02, 0.018, 0.018)),
        (k + Vector((-0.022, -0.14, -0.015)), (0.02, 0.018, 0.018)),
        (k + Vector((0, -0.14, -0.045)), (0.055, 0.02, 0.006), True),
        (k + Vector((0, 0.03, -0.14)), (0.1, 0.09, 0.08)),
    ]
    for s in (1, -1):
        kopf += [(k + Vector((0.045 * s, -0.105, 0.005)), (0.024, 0.016, 0.017), True),
                 (k + Vector((0.08 * s, -0.08, -0.03)), (0.04, 0.035, 0.03))]
    b.f.metaball("Kopf", kopf, 0.005, 3200, lambda poly: haut * (0.95 + 0.07 * max(0.0, poly.normal.z)), b.hals_kopf, glatt=True)
    b.augen([k + Vector((0.045 * s, -0.1, 0.005)) for s in (1, -1)], 0.018, farbe("#E07A1A"))
    for s in (1, -1):
        b.f.kugel("Lid", k + Vector((0.045 * s, -0.107, 0.019)), (0.024, 0.014, 0.008), haut * 0.85, KOPF, 12, 6)
        _kegel(f, "Hauer", k + Vector((0.045 * s, -0.135, -0.07)), Vector((0.2 * s, -0.25, 1)), 0.08, 0.017, farbe("#E8DDB8"), KOPF, 8,
               krumm=Vector((0.3 * s, 0.4, 0)))
        wurzel = k + Vector((0.105 * s, 0.02, 0.0))
        b.f.straehne("Ohr", [wurzel, wurzel + Vector((0.04 * s, 0.05, 0.02)), wurzel + Vector((0.06 * s, 0.1, 0.05))], 0.035, 0.003, haut, KOPF, 8, 0.0, 0.3)
        for j in range(2):
            b.f.loft("Ohrring", [(wurzel + Vector((0.03 * s, 0.03 + 0.03 * j, -0.02)), Y, Z, 0.014, 0.014),
                                 (wurzel + Vector((0.032 * s, 0.03 + 0.03 * j, -0.02)), Y, Z, 0.01, 0.01)], 10, lambda i, k2, p: farbe("#C9A040"), KOPF)
    haar = farbe("#1A1614")
    b.f.kugel("Haarknoten", k + Vector((0, 0.06, 0.17)), (0.04, 0.04, 0.05), haar, KOPF, 12, 8)
    for i in range(6):
        w = math.tau * i / 6
        basis = k + Vector((math.cos(w) * 0.02, 0.06 + math.sin(w) * 0.02, 0.2))
        b.f.straehne("Zopf", [basis, basis + Vector((math.cos(w) * 0.04, 0.06 + math.sin(w) * 0.03, 0.03)), basis + Vector((math.cos(w) * 0.06, 0.16, -0.06))],
                     0.014, 0.003, haar, KOPF, 5, 0.0, 1.0)
    b.f.loft("Haarband", [(k + Vector((0, 0.06, 0.155)), X, Y, 0.042, 0.042), (k + Vector((0, 0.06, 0.175)), X, Y, 0.043, 0.043)], 12,
             lambda i, k2, p: farbe("#C9A040"), KOPF)

    def axt():
        g = b.griff(-1)
        oben = Vector((0.0, -0.3, 1.0)).normalized()
        holz = farbe("#4A3220")
        a, e = g - oben * 0.35, g + oben * 1.0
        _strecke(f, "Axtstiel", a, e, 0.026, 0.024, lambda i, k2, p: holz * (0.8 if k2 % 4 == 0 else 1.0), b.hand(-1), 10)
        q1, q2 = _achsen(a, e)
        for t in (0.0, 0.12):
            f.loft("Wicklung", [(g + oben * (t - 0.05), q1, q2, 0.03, 0.03), (g + oben * (t + 0.05), q1, q2, 0.03, 0.03)], 10,
                   lambda i, k2, p: leder_dunkel, b.hand(-1))
        kopfmitte = g + oben * 0.86
        vorne = Vector((0, -1, -0.3)).normalized()
        vorne = (vorne - oben * vorne.dot(oben)).normalized()
        ringe = []
        for j in range(9):
            t = (j - 4) / 4
            weite = 0.06 + 0.26 * (1 - t * t) ** 0.6
            ringe.append((kopfmitte + oben * t * 0.24 + vorne * weite * 0.5, vorne, X, weite * 0.5 + 0.01, 0.012 * (1 - 0.5 * abs(t))))
        f.loft("Axtblatt", ringe, 8, lambda i, k2, p: eisen_hell if k2 in (0, 7) or i < 0.5 or i > 7.5 else eisen * (0.9 + 0.2 * max(0.0, p.normal.z)),
               b.hand(-1), oben_zu=True, unten_zu=True, teilung=1)
        _kegel(f, "Dorn", kopfmitte - vorne * 0.03, -vorne, 0.16, 0.03, eisen, b.hand(-1), 6)
        f.kugel("Axtkappe", e, (0.035, 0.035, 0.04), eisen, b.hand(-1), 8, 6)
    b.starr("Streitaxt", "Hand.R", axt)
    b.skelett()
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-8, -18), (-14, -40)), gehen=(24, 36, 14, 0.035, 8, 14)))


# ===========================================================================
# Echsenkrieger
# ===========================================================================
def echsenkrieger(seed=404):
    """Knapp zwei Meter, schlank, türkise Schuppen mit gelbem Bauch, langer Schwanz, Echsenkopf mit
    Stachelkamm, Speer mit Bronzeblatt und Schild aus einem Schildkrötenpanzer."""
    f = Figur("Echsenkrieger", seed)
    r = f.rng
    b = Bau(f, becken=0.98, bauch=1.12, brust=1.32, hals=1.58, kopf=1.66, scheitel=1.88, kopf_y=-0.1,
            schulter=(0.22, 0.02, 1.54), ellbogen=(0.3, 0.05, 1.27), hand=(0.34, 0.0, 1.02), finger=(0.35, -0.03, 0.92),
            huefte=(0.11, 0.02, 0.98), knie=(0.13, -0.08, 0.56), knoechel=(0.13, 0.06, 0.12), zehen=(0.13, -0.12, 0.02))
    schuppe, schuppe_dunkel, bauch = farbe("#2E7A6A"), farbe("#1E5248"), farbe("#D8C27A")
    bronze, stoff = farbe("#B07A3A"), farbe("#8E2A22")
    becken = lambda co: {"Becken": 1.0}

    def schuppen(i, k, p):
        if p.normal.y < -0.35 and abs(p.center.x) < 0.12:
            return bauch * (0.86 if int(p.center.z * 28) % 2 else 1.0)
        muster = (k + int(p.center.z * 40)) % 2
        return schuppe.lerp(schuppe_dunkel, 0.35 * muster + 0.3 * max(0.0, p.normal.z))

    b.rumpf_loft("Rumpf", [(0.94, 0.14, 0.13, 0.02), (1.05, 0.15, 0.13), (1.18, 0.15, 0.13, -0.01), (1.3, 0.19, 0.14, -0.01),
                           (1.42, 0.22, 0.15, 0.0), (1.52, 0.21, 0.14, 0.01), (1.58, 0.14, 0.11, 0.0), (1.64, 0.07, 0.08, -0.06)], 36, schuppen)
    _schlauch(f, "Hals", [Vector((0, 0.0, 1.56)), Vector((0, -0.06, 1.64)), Vector((0, -0.12, 1.7))], [0.085, 0.07, 0.065], 18, schuppen, b.hals_kopf)
    punkte = [Vector((0, 0.08, 1.0)), Vector((0, 0.3, 0.84)), Vector((0, 0.56, 0.6)), Vector((0, 0.8, 0.38)), Vector((0.02, 1.02, 0.3)),
              Vector((0.05, 1.2, 0.34))]
    _schlauch(f, "Schwanz", punkte, [0.12, 0.1, 0.08, 0.06, 0.035, 0.008], 18, schuppen, becken)
    for j in range(6):
        p = punkte[1].lerp(punkte[4], j / 5)
        _kegel(f, "Schwanzstachel", p + Vector((0, 0, 0.09 - 0.012 * j)), Vector((0, 0.6, 1)), 0.08 - 0.008 * j, 0.022, schuppe_dunkel, becken, 5)
    b.f.loft("Lendentuch", [(Vector((0, 0.0, 1.0)), X, Y, 0.15, 0.14), (Vector((0, -0.02, 0.84)), X, Y, 0.17, 0.155, _falten(r, 0.05)),
                            (Vector((0, -0.02, 0.72)), X, Y, 0.18, 0.16, _zacken(r, 0.2, 9))], 30,
             lambda i, k, p: stoff * (0.8 if k % 5 == 0 else 1.0), b.rock, teilung=2, glatt=True)
    b.f.loft("Guertel", [(Vector((0, 0.0, 0.99)), X, Y, 0.152, 0.142), (Vector((0, 0.0, 1.04)), X, Y, 0.154, 0.144)], 30,
             lambda i, k, p: bronze * (0.8 if k % 3 == 0 else 1.0), b.rumpf, glatt=True)
    for j in range(9):
        w = math.pi * (0.2 + 0.6 * j / 8)
        p = Vector((math.cos(w) * 0.13, -0.1 - math.sin(w) * 0.04, 1.5 - math.sin(w) * 0.1))
        _kegel(f, "Kettenzahn", p, Vector((0, -0.3, -1)), 0.04, 0.01, farbe("#E8DDB8"), b.rumpf, 5)
    for s in (1, -1):
        b.arm_schlauch(s, [0.07, 0.062, 0.048, 0.052, 0.038], schuppen)
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        b.f.loft("Armreif", [(e.lerp(h, 0.55), q1, q2, 0.058, 0.058), (e.lerp(h, 0.9), q1, q2, 0.048, 0.048)], 16,
                 lambda i, k, p: bronze * (0.8 if k % 4 == 0 else 1.0), b.arm(s), glatt=True)
        b.faust(s, 1.15, lambda poly: schuppe * (0.9 if poly.normal.z < 0 else 1.0), krallen=farbe("#2A2620"))
    for s in (1, -1):
        b.bein_schlauch(s, [0.1, 0.09, 0.062, 0.065, 0.045], schuppen)
        b.fuss_nackt(s, 0.26, 0.12, lambda poly: schuppe_dunkel, zehen=3, krallen=farbe("#2A2620"))

    k = Vector((0, -0.18, 1.74))
    kopf = [
        (k + Vector((0, 0.03, 0.02)), (0.075, 0.09, 0.07)),
        (k + Vector((0, -0.08, -0.005)), (0.06, 0.1, 0.05)),
        (k + Vector((0, -0.17, -0.01)), (0.045, 0.07, 0.035)),
        (k + Vector((0, -0.08, -0.05)), (0.055, 0.1, 0.03)),
        (k + Vector((0, -0.12, -0.03)), (0.05, 0.1, 0.006), True),
    ]
    for s in (1, -1):
        kopf += [(k + Vector((0.055 * s, -0.02, 0.035)), (0.03, 0.035, 0.022))]
    b.f.metaball("Kopf", kopf, 0.0042, 2800, lambda poly: bauch if poly.normal.z < -0.5 else schuppe.lerp(schuppe_dunkel, max(0.0, poly.normal.z) * 0.5),
                 KOPF, glatt=True)
    b.augen([k + Vector((0.062 * s, -0.03, 0.042)) for s in (1, -1)], 0.023, farbe("#F2B81C"), schlitz=True, weiss=farbe("#C8B840"),
            blick=Vector((0, -0.6, 0.1)).normalized())
    for s in (1, -1):
        b.f.kugel("Nuester", k + Vector((0.018 * s, -0.235, 0.012)), (0.007, 0.006, 0.005), farbe("#101010"), KOPF, 6, 4)
        for j in range(5):
            _kegel(f, "Zahn", k + Vector((0.042 * s * (1 - j * 0.1), -0.05 - 0.04 * j, -0.03)), Vector((0, 0, -1)), 0.022, 0.006, farbe("#EDE6D0"), KOPF, 4)
    for j in range(7):
        basis = k + Vector((0, 0.02 + 0.04 * j, 0.08 - 0.02 * j))
        _kegel(f, "Kamm", basis, Vector((0, 0.8, 1)), 0.16 - 0.015 * j, 0.018, farbe("#C8322A") if j % 2 == 0 else farbe("#E8702A"), KOPF, 5)
    for s in (1, -1):
        _kegel(f, "Horn", k + Vector((0.05 * s, 0.06, 0.06)), Vector((0.4 * s, 0.8, 0.5)), 0.1, 0.018, farbe("#E4DAC2"), KOPF, 6)

    def speer():
        g = b.griff(-1)
        oben = Vector((0.0, -0.15, 1.0)).normalized()
        _strecke(f, "Speerschaft", g - oben * 0.7, g + oben * 1.25, 0.018, 0.017, farbe("#6A4A2A"), b.hand(-1), 8)
        spitze = g + oben * 1.25
        f.loft("Speerblatt", [(spitze, oben, X, 0.02, 0.008), (spitze + oben * 0.1, oben, X, 0.05, 0.01), (spitze + oben * 0.26, oben, X, 0.002, 0.002)], 8,
               lambda i, k2, p: bronze * (1.1 if k2 in (0, 4) else 0.9), b.hand(-1), oben_zu=True, unten_zu=True)
        for j, c in enumerate((farbe("#C8322A"), farbe("#F2EEE0"), farbe("#2E6AA8"))):
            a = spitze - oben * 0.02
            b.f.straehne("Speerfeder", [a, a + Vector((0.03 * (j - 1), 0.02, -0.08)), a + Vector((0.05 * (j - 1), 0.03, -0.16))], 0.012, 0.002, c,
                         b.hand(-1), 5, 0.0, 0.3)
    b.starr("Speer", "Hand.R", speer)

    def schild():
        h = b.p("hand", 1).lerp(b.p("ellbogen", 1), 0.4) + Vector((0.07, -0.02, 0))
        panzer, rand = farbe("#5A6A3A"), farbe("#3A4424")
        arm = lambda co: {"Unterarm.L": 1.0}
        _schale(f, "Schildpanzer", h, 0.1, 0.3, 0.34, (5, 175), (-85, 85),
                lambda poly: panzer * (0.8 if int(poly.center.y * 12 + poly.center.z * 12) % 2 else 1.0), arm, seg=(20, 16))
        _schale(f, "Schildinnen", h, 0.095, 0.295, 0.335, (5, 175), (-85, 85), lambda poly: rand, arm, innen=True, seg=(12, 10))
        for j in range(6):
            w = math.tau * j / 6
            y, z = math.cos(w) * 0.16, math.sin(w) * 0.19
            tiefe = 0.1 * math.sqrt(max(0.0, 1 - (y / 0.3) ** 2 - (z / 0.34) ** 2))
            b.f.kugel("Schildbuckel", h + Vector((tiefe, y, z)), (0.018, 0.055, 0.055), farbe("#7A8A4A"), arm, 10, 6)
        b.f.kugel("Schildmitte", h + Vector((0.1, 0, 0)), (0.02, 0.075, 0.075), farbe("#7A8A4A"), arm, 12, 8)
    b.starr("Schild", "Unterarm.L", schild)
    b.skelett()
    return f.fertig(_animationen(_angriff_stoss, arme_ruhe=((-35, -50), (-10, -35)), gehen=(28, 44, 14, 0.03, 12, 16)))


# ===========================================================================
# Pilzling
# ===========================================================================
def pilzling(seed=405):
    """Ein Meter groß: dicker Stiel als Körper, riesiger roter Hut mit weißen Tupfen und Lamellen,
    große dunkle Glanzaugen, Stummelarme, Wurzelfüße; auf der Schulter wachsen kleine Pilze."""
    f = Figur("Pilzling", seed)
    r = f.rng
    b = Bau(f, becken=0.3, bauch=0.38, brust=0.5, hals=0.6, kopf=0.62, scheitel=0.95, kopf_y=0.0,
            schulter=(0.14, 0.0, 0.52), ellbogen=(0.2, 0.02, 0.4), hand=(0.23, -0.01, 0.29), finger=(0.24, -0.03, 0.23),
            huefte=(0.08, 0.0, 0.3), knie=(0.09, -0.01, 0.17), knoechel=(0.09, 0.01, 0.05), zehen=(0.09, -0.08, 0.02))
    stiel, stiel_dunkel = farbe("#E8DCC0"), farbe("#BFAE8A")
    hut, hut_dunkel, tupfen = farbe("#C8322A"), farbe("#8E1E1A"), farbe("#F4EEE2")
    lamelle = farbe("#E6D2B0")
    moos = farbe("#5E8A34")

    def stiel_farbe(i, k, p):
        faser = 0.93 if k % 3 == 0 else 1.0
        if p.center.z < 0.34 and p.normal.z > -0.2 and (k * 7 + int(p.center.z * 50)) % 5 == 0:
            return moos
        return stiel.lerp(stiel_dunkel, weich(0.5, 0.25, p.center.z) * 0.5) * faser

    # Körper: dicker, leicht bauchiger Stiel, oben unter dem Hut schmaler; ein Ring (Manschette)
    b.rumpf_loft("Stiel", [(0.24, 0.12, 0.11), (0.32, 0.15, 0.14, -0.01), (0.42, 0.155, 0.145, -0.015), (0.52, 0.14, 0.13, -0.01),
                           (0.62, 0.12, 0.11), (0.7, 0.11, 0.1)], 30, stiel_farbe)
    b.f.loft("Manschette", [(Vector((0, -0.005, 0.58)), X, Y, 0.13, 0.12), (Vector((0, -0.005, 0.55)), X, Y, 0.165, 0.155, _falten(r, 0.08)),
                            (Vector((0, -0.005, 0.51)), X, Y, 0.17, 0.16, _zacken(r, 0.1, 12))], 30, lambda i, k, p: stiel * 1.03, b.rumpf, glatt=True)
    # Der Hut: flache Kuppel mit welligem Rand, Tupfen, darunter Lamellen
    rand = _falten(r, 0.07)
    kuppe = []
    for j in range(9):
        t = j / 8
        rad = 0.32 * math.cos(t * math.pi / 2) ** 0.7 + 0.004
        kuppe.append((Vector((0, 0.01, 0.76 + 0.22 * math.sin(t * math.pi / 2))), X, Y, rad, rad * 0.97, rand if j < 3 else None))
    kuppe = [ring[:5] + ((ring[5],) if ring[5] else ()) for ring in kuppe]
    tupfen_orte = [Vector((math.cos(w) * d, math.sin(w) * d, 0)) for w, d in ((r.uniform(0, math.tau), r.uniform(0.04, 0.27)) for _ in range(12))]

    def hut_farbe(i, k, p):
        c = hut.lerp(hut_dunkel, weich(0.9, 0.76, p.center.z) * 0.5)
        flach = Vector((p.center.x, p.center.y, 0))
        if any((flach - t).length < 0.045 + 0.02 * (hash((round(t.x, 3), round(t.y, 3))) % 3) / 2 for t in tupfen_orte) and p.normal.z > -0.1:
            return tupfen
        return c * (0.92 + 0.1 * max(0.0, p.normal.z))
    b.f.loft("Hut", kuppe, 48, hut_farbe, KOPF, oben_zu=True, teilung=2, glatt=True)
    # Unterseite: Lamellen strahlenförmig
    b.f.loft("Hutunterseite", [(Vector((0, 0.01, 0.765)), X, Y, 0.32, 0.31, rand), (Vector((0, 0.01, 0.725)), X, Y, 0.1, 0.1)], 48,
             lambda i, k, p: lamelle * (0.8 if k % 2 else 1.0), KOPF, teilung=2)
    # Gesicht vorne am Stiel unter dem Hut: große Glanzaugen, kleiner Mund, rosige Wangen
    k = Vector((0, -0.125, 0.62))
    for s in (1, -1):
        auge = k + Vector((0.045 * s, 0.0, 0.0))
        b.f.kugel("Auge", auge, (0.028, 0.018, 0.034), farbe("#1A1210"), KOPF, 14, 10)
        b.f.kugel("Glanz", auge + Vector((0.008 * s, -0.016, 0.012)), (0.008, 0.004, 0.008), farbe("#FFFFFF"), KOPF, 8, 6)
        b.f.kugel("Glanz klein", auge + Vector((-0.006 * s, -0.016, -0.01)), (0.004, 0.003, 0.004), farbe("#FFFFFF"), KOPF, 6, 4)
        b.f.kugel("Wange", k + Vector((0.075 * s, 0.012, -0.04)), (0.022, 0.01, 0.014), farbe("#E89A8A"), KOPF, 10, 6)
    b.f.loft("Mund", [(k + Vector((0, -0.004, -0.05)), X, Z, 0.022, 0.008), (k + Vector((0, 0.004, -0.05)), X, Z, 0.018, 0.005)], 12,
             lambda i, k2, p: farbe("#5A2A20"), KOPF, oben_zu=True, unten_zu=True)
    # Stummelarme mit Fäustlingen
    for s in (1, -1):
        b.arm_schlauch(s, [0.04, 0.036, 0.032, 0.034, 0.03], lambda i, k, p: stiel * 0.97, seg=12)
        g = b.griff(s)
        b.f.metaball("Faeustling", [(g, (0.04, 0.04, 0.045)), (g + Vector((0.02 * s, -0.03, 0.02)), (0.016, 0.016, 0.016))], 0.004, 800,
                     lambda poly: stiel_dunkel, b.hand(s), glatt=True)
    # Stummelbeine mit Wurzelzehen
    for s in (1, -1):
        b.bein_schlauch(s, [0.06, 0.055, 0.05, 0.05, 0.045], lambda i, k, p: stiel_dunkel, seg=12)
        a = b.p("knoechel", s)
        for j, w in enumerate((-0.6, 0.0, 0.6, 3.0)):
            richtung = Vector((math.sin(w) * 0.7, -math.cos(w), -0.35)).normalized()
            basis = Vector((a.x, a.y, 0.05))
            b.f.straehne("Wurzel", [basis, basis + richtung * 0.05, basis + richtung * 0.1 + Vector((0, 0, -0.04))], 0.03, 0.006,
                         lambda i, k2, p: farbe("#8A6A48"), b.fuss(s), 6, 0.0, 1.0)
    # Kleine Pilze auf der Schulter und am Fuß
    for ort, g_ in ((Vector((0.11, 0.03, 0.55)), 1.0), (Vector((0.14, 0.02, 0.5)), 0.7), (Vector((-0.12, 0.05, 0.3)), 0.8)):
        _strecke(f, "Pilzstiel", ort, ort + Vector((0.02, 0, 0.05 * g_)), 0.008 * g_, 0.006 * g_, stiel, b.rumpf, 6)
        b.f.kugel("Pilzhut", ort + Vector((0.02, 0, 0.055 * g_)), (0.025 * g_, 0.025 * g_, 0.012 * g_), farbe("#D8A040"), b.rumpf, 10, 6)
    b.skelett()
    return f.fertig(_animationen(_angriff_stampfen, arme_ruhe=((-5, -20), (-5, -20)), gehen=(34, 40, 30, 0.035, 4, 16)))


# ===========================================================================
# Waldschrat
# ===========================================================================
def waldschrat(seed=406):
    """Knapp drei Meter hoher, wandelnder Baum: Rindenkörper mit tiefen Furchen, Astarme mit
    Zweigfingern, Wurzelbeine, ein Gesicht in der Rinde mit glühenden Augen, eine Krone aus Ästen
    und Laub, Moos und Pilze auf den Schultern."""
    f = Figur("Waldschrat", seed)
    r = f.rng
    b = Bau(f, becken=1.3, bauch=1.55, brust=1.85, hals=2.22, kopf=2.26, scheitel=2.66, kopf_y=-0.04,
            schulter=(0.44, 0.0, 2.12), ellbogen=(0.6, 0.06, 1.68), hand=(0.68, 0.0, 1.26), finger=(0.7, -0.04, 1.06),
            huefte=(0.2, 0.0, 1.3), knie=(0.24, -0.04, 0.72), knoechel=(0.25, 0.03, 0.16), zehen=(0.26, -0.2, 0.04))
    rinde, rinde_dunkel, rinde_hell = farbe("#5B4330"), farbe("#35261A"), farbe("#7A6048")
    moos, moos_hell = farbe("#4E7A2A"), farbe("#7AA23A")
    laub = [farbe("#3E7A2A"), farbe("#5A9A34"), farbe("#7AB23E"), farbe("#A8B838")]
    glut = farbe("#FFC23A")
    furchen = lambda w: 1.0 + 0.05 * math.sin(w * 9) + 0.025 * math.sin(w * 23 + 1.3)

    def rinden_farbe(i, k, p):
        if p.normal.z > 0.55:
            return moos.lerp(moos_hell, (k % 3) / 3)
        furche = math.sin(math.atan2(p.center.y, p.center.x) * 9)
        return rinde_dunkel if furche < -0.55 else rinde.lerp(rinde_hell, max(0.0, furche) * 0.4)

    b.rumpf_loft("Stamm", [(1.2, 0.3, 0.26, 0.0, furchen), (1.4, 0.33, 0.28, -0.02, furchen), (1.6, 0.34, 0.29, -0.03, furchen),
                           (1.8, 0.38, 0.3, -0.03, furchen), (2.0, 0.44, 0.31, -0.02, furchen), (2.14, 0.42, 0.28, 0.0, furchen),
                           (2.26, 0.3, 0.24, -0.02, furchen), (2.46, 0.24, 0.21, -0.03, furchen), (2.6, 0.16, 0.15, -0.02, furchen)], 44,
                 rinden_farbe, gewicht=lambda co: b.rumpf(co) if co.z < 2.2 else b.hals_kopf(co), oben_zu=True)
    # Astknoten
    for j in range(5):
        w = r.uniform(0, math.tau)
        z = r.uniform(1.35, 1.95)
        b.f.kugel("Astknoten", (math.cos(w) * 0.33, math.sin(w) * 0.27, z), (0.06, 0.06, 0.05), rinde_dunkel, b.rumpf, 10, 6)
    # Gesicht: tiefe Augenhöhlen mit Glut, Rindenbrauen, ein klaffender Mund
    k = Vector((0, -0.23, 2.36))
    for s in (1, -1):
        b.f.kugel("Augenhoehle", k + Vector((0.075 * s, 0.01, 0.0)), (0.05, 0.03, 0.035), rinde_dunkel * 0.5, KOPF, 12, 8)
        b.f.kugel("Glutauge", k + Vector((0.075 * s, -0.012, 0.0)), (0.028, 0.012, 0.022), glut, KOPF, 12, 8)
        _platte(f, "Braue", k + Vector((0.08 * s, -0.02, 0.05)), 0.16, 0.035, 0.03, Vector((s, 0, -0.35)), -Y, lambda i, k2, p: rinde_hell, KOPF,
                spitz=0.5, wolbung=0.8)
    b.f.kugel("Mund", k + Vector((0, 0.0, -0.14)), (0.08, 0.03, 0.035), farbe("#140E0A"), KOPF, 14, 8)
    _kegel(f, "Nase", k + Vector((0, -0.02, -0.05)), Vector((0, -1, -0.5)), 0.1, 0.035, rinde, KOPF, 7)
    # Moosbart
    for j in range(9):
        x = -0.12 + 0.03 * j
        basis = k + Vector((x, 0.0, -0.19))
        b.f.straehne("Moosbart", [basis, basis + Vector((x * 0.2, -0.03, -0.12)), basis + Vector((x * 0.4, -0.02, -0.26 - 0.04 * (j % 2)))], 0.03, 0.006,
                     moos if j % 2 else moos_hell, lambda co: _mischen(("Kopf", weich(2.0, 2.2, co.z)), ("Brust", 1 - weich(2.0, 2.2, co.z))), 6, 0.3, 0.6)
    # Krone: Äste aus Kopf und Schultern, Laubbüschel an den Spitzen
    kronen_aeste = [(Vector((0, 0.02, 2.58)), Vector((0.1, 0.1, 1)), 0.55), (Vector((0.1, 0.05, 2.5)), Vector((0.8, 0.2, 1)), 0.5),
                    (Vector((-0.1, 0.05, 2.5)), Vector((-0.8, 0.3, 1)), 0.48), (Vector((0.3, 0.05, 2.14)), Vector((1, 0.2, 0.8)), 0.42),
                    (Vector((-0.3, 0.05, 2.14)), Vector((-1, 0.4, 0.7)), 0.4), (Vector((0, 0.18, 2.4)), Vector((0, 1, 0.8)), 0.45)]
    for ort, richtung, laenge in kronen_aeste:
        richtung = richtung.normalized()
        spitze = ort + richtung * laenge
        mitte = ort.lerp(spitze, 0.5) + Vector((r.uniform(-0.04, 0.04), r.uniform(-0.04, 0.04), 0.04))
        gewicht = KOPF if ort.z > 2.3 else b.rumpf
        b.f.straehne("Ast", [ort, mitte, spitze], 0.06, 0.02, lambda i, k2, p: rinde * (0.85 if k2 % 2 else 1.0), gewicht, 7, 0.3, 1.0)
        for j in range(5):
            o = spitze + Vector((r.uniform(-0.13, 0.13), r.uniform(-0.13, 0.13), r.uniform(-0.05, 0.12)))
            g_ = r.uniform(0.09, 0.15)
            b.f.kugel("Laub", o, (g_, g_, g_ * 0.8), lambda poly: laub[(hash((round(poly.center.x, 2), round(poly.center.z, 2))) % 4)] *
                      (0.85 + 0.2 * max(0.0, poly.normal.z)), gewicht, 8, 6, glatt=False)
    # Pilze auf der Schulter
    for j in range(4):
        o = Vector((0.36 + 0.04 * j, 0.1 - 0.05 * j, 2.2 - 0.05 * j))
        b.f.kugel("Schulterpilz", o, (0.05, 0.05, 0.018), farbe("#D8A040"), b.rumpf, 10, 6)
    # Astarme: knorrig, mit Zweigfingern statt Händen
    for s in (1, -1):
        knorrig = lambda w: 1.0 + 0.08 * math.sin(w * 5) + 0.05 * math.sin(w * 11)
        b.arm_schlauch(s, [0.16, 0.13, 0.11, 0.1, 0.08], rinden_farbe, seg=18, form=knorrig)
        h, fi = b.p("hand", s), b.p("finger", s)
        achse = (fi - h).normalized()
        for j, (w, l) in enumerate(((-0.6, 0.3), (-0.2, 0.36), (0.25, 0.33), (0.7, 0.26))):
            seitlich = Vector((math.sin(w) * 0.9 * s, -math.cos(w) * 0.6, 0))
            a = h + seitlich * 0.05
            mitte = a + achse * l * 0.5 + seitlich * 0.06
            spitze = a + achse * l + seitlich * 0.02 + Vector((0, -0.08, 0))
            b.f.straehne("Zweigfinger", [a, mitte, spitze], 0.035, 0.008, rinde, b.hand(s), 6, 0.0, 1.0)
        a = h + Vector((0, -0.07, 0.02))
        b.f.straehne("Zweigdaumen", [a, a + Vector((0.02 * s, -0.12, -0.08)), a + Vector((0.0, -0.18, -0.2))], 0.035, 0.008, rinde, b.hand(s), 6, 0.0, 1.0)
        for j in range(2):
            o = b.p("schulter", s).lerp(b.p("ellbogen", s), 0.4 + 0.3 * j) + Vector((0.06 * s, 0, 0.06))
            b.f.kugel("Armlaub", o, (0.09, 0.09, 0.07), laub[j + 1], b.arm(s), 8, 6, glatt=False)
    # Wurzelbeine mit ausgreifenden Wurzeln als Füße
    for s in (1, -1):
        b.bein_schlauch(s, [0.2, 0.18, 0.15, 0.14, 0.12], rinden_farbe, seg=18, form=furchen)
        a = b.p("knoechel", s)
        for j, w in enumerate((-0.9, -0.3, 0.3, 0.9, 2.8)):
            richtung = Vector((math.sin(w), -math.cos(w), 0)).normalized()
            basis = Vector((a.x, a.y, 0.14))
            spitze = basis + richtung * 0.32 + Vector((0, 0, -0.14))
            b.f.straehne("Wurzel", [basis, basis + richtung * 0.15 + Vector((0, 0, -0.06)), spitze], 0.08, 0.015, rinde_dunkel, b.fuss(s), 7, 0.2, 1.0)
    b.skelett()
    return f.fertig(_animationen(_angriff_stampfen, arme_ruhe=((-6, -15), (-6, -15)), gehen=(18, 26, 12, 0.03, 6, 12)))


# ===========================================================================
# Minotaurus
# ===========================================================================
def minotaurus(seed=407):
    """Zweieinhalb Meter: Stierkopf mit Goldring in der Nase und mächtigen Hörnern, bulliger
    Oberkörper mit Nackenbuckel, zottige Beine mit Hufen, Lederschurz mit Nieten, Doppelaxt."""
    f = Figur("Minotaurus", seed)
    r = f.rng
    b = Bau(f, becken=1.18, bauch=1.36, brust=1.6, hals=1.94, kopf=1.98, scheitel=2.3, kopf_y=-0.12,
            schulter=(0.37, 0.02, 1.88), ellbogen=(0.49, 0.05, 1.5), hand=(0.54, 0.0, 1.18), finger=(0.55, -0.03, 1.06),
            huefte=(0.16, 0.0, 1.18), knie=(0.19, -0.06, 0.66), knoechel=(0.19, 0.08, 0.16), zehen=(0.19, -0.08, 0.02))
    fell, fell_dunkel, fell_hell = farbe("#6B4226"), farbe("#3E2616"), farbe("#8E6444")
    haut = farbe("#7A5234")
    horn, horn_dunkel = farbe("#E8DCC0"), farbe("#8A7A5E")
    leder, eisen, gold = farbe("#4A2E1C"), farbe("#6A6E76"), farbe("#D8AE4A")

    def fell_farbe(i, k, p):
        c = fell.lerp(fell_dunkel, max(0.0, -p.normal.z) * 0.4)
        if p.normal.y < -0.4 and 1.3 < p.center.z < 1.8:
            c = c.lerp(fell_hell, 0.35)                                   # hellere Brust
        return c * (0.9 if (k + int(p.center.z * 25)) % 4 == 0 else 1.0)

    zottig = lambda w: 1.0 + 0.06 * math.sin(w * 13) + 0.04 * math.sin(w * 29 + 2)
    b.rumpf_loft("Rumpf", [(1.14, 0.22, 0.18), (1.28, 0.23, 0.19, -0.02), (1.42, 0.27, 0.21, -0.03), (1.56, 0.33, 0.24, -0.03),
                           (1.7, 0.38, 0.25, -0.02), (1.82, 0.37, 0.24, 0.02), (1.9, 0.28, 0.23, 0.05), (1.98, 0.16, 0.18, 0.02)], 40, fell_farbe)
    # Nackenbuckel und Brustmuskeln
    b.f.kugel("Nackenbuckel", (0, 0.1, 1.9), (0.26, 0.17, 0.14), lambda poly: fell_dunkel * (0.95 + 0.1 * max(0.0, poly.normal.z)), b.rumpf, 18, 10)
    for s in (1, -1):
        b.f.kugel("Brustmuskel", (0.15 * s, -0.22, 1.68), (0.16, 0.06, 0.11), lambda poly: fell_hell * 0.9, b.rumpf, 14, 8)
    # Lederschurz mit Nieten, breiter Gürtel mit Goldschnalle
    b.f.loft("Lederschurz", [(Vector((0, -0.01, 1.16)), X, Y, 0.235, 0.195), (Vector((0, -0.02, 0.98)), X, Y, 0.26, 0.22, _falten(r, 0.05)),
                             (Vector((0, -0.02, 0.8)), X, Y, 0.27, 0.23, _zacken(r, 0.12, 8))], 36,
             lambda i, k, p: leder * (0.8 if k % 4 == 0 else 1.0), b.rock, teilung=2, glatt=True)
    b.f.loft("Guertel", [(Vector((0, -0.01, 1.12)), X, Y, 0.24, 0.2), (Vector((0, -0.01, 1.22)), X, Y, 0.245, 0.205)], 36,
             lambda i, k, p: leder * 0.7, b.rumpf, glatt=True)
    b.f.kugel("Schnalle", (0, -0.21, 1.17), (0.07, 0.02, 0.05), gold, b.rumpf, 12, 8)
    for j in range(10):
        w = math.pi * (0.1 + 0.8 * j / 9)
        b.f.kugel("Niete", (math.cos(w) * 0.255, -math.sin(w) * 0.215, 0.95), (0.012, 0.012, 0.012), eisen, b.rock, 6, 4)
    # Arme: muskulös, Eisenmanschetten, eine Kette um den linken Arm
    for s in (1, -1):
        b.arm_schlauch(s, [0.14, 0.13, 0.1, 0.11, 0.08], fell_farbe, seg=20)
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        b.f.loft("Manschette", [(e.lerp(h, 0.6), q1, q2, 0.105, 0.105), (e.lerp(h, 0.95), q1, q2, 0.095, 0.095)], 18,
                 lambda i, k, p: eisen * (0.85 if k % 3 == 0 else 1.0), b.arm(s), glatt=True)
        b.faust(s, 1.6, lambda poly: haut)
    so, eo = b.p("schulter", 1), b.p("ellbogen", 1)
    for j in range(7):
        p = so.lerp(eo, 0.3 + 0.08 * j)
        q1, q2 = _achsen(so, eo)
        b.f.loft("Kettenglied", [(p + q1 * 0.12, q2, (eo - so).normalized(), 0.022, 0.022), (p + q1 * 0.125, q2, (eo - so).normalized(), 0.014, 0.014)],
                 8, lambda i, k2, p_: eisen, b.arm(1))
    # Zottige Stierbeine mit Hufen
    for s in (1, -1):
        b.bein_schlauch(s, [0.17, 0.16, 0.11, 0.1, 0.07], lambda i, k, p: fell_dunkel if i > 2 else fell, seg=20, form=zottig)
        a = b.p("knoechel", s)
        b.f.loft("Fesselhaar", [(a + Vector((0, 0, 0.02)), X, Y, 0.1, 0.1, zottig), (a + Vector((0, 0, 0.14)), X, Y, 0.085, 0.085, zottig)], 24,
                 lambda i, k, p: fell_dunkel * 0.8, b.bein(s), teilung=2)
        huf = [(Vector((a.x, a.y + 0.02, 0.0)), X, Y, 0.09, 0.1), (Vector((a.x, a.y + 0.01, 0.1)), X, Y, 0.075, 0.08)]
        b.f.loft("Huf", huf, 16, lambda i, k, p: farbe("#2A2420"), b.fuss(s), oben_zu=True, unten_zu=True, glatt=True)
        b.f.kiste("Hufspalt", (a.x, a.y - 0.08, 0.05), (0.008, 0.04, 0.1), farbe("#141010"), b.fuss(s))

    # Stierkopf: breite Stirn, lange Schnauze mit dunklem Maul, Nüstern, Goldring, Hörner
    k = Vector((0, -0.16, 2.14))
    kopf = [
        (k + Vector((0, 0.04, 0.03)), (0.14, 0.14, 0.13)),               # Stirn und Schädel
        (k + Vector((0, -0.1, -0.04)), (0.1, 0.12, 0.09)),               # Schnauze
        (k + Vector((0, -0.21, -0.07)), (0.09, 0.07, 0.07)),             # Maul
        (k + Vector((0, -0.14, -0.13)), (0.07, 0.08, 0.035)),            # Unterkiefer
        (k + Vector((0, 0.06, -0.12)), (0.14, 0.12, 0.12)),              # Wamme und Nacken
    ]
    for s in (1, -1):
        kopf += [(k + Vector((0.075 * s, -0.08, 0.04)), (0.03, 0.02, 0.02), True)]
    b.f.metaball("Kopf", kopf, 0.006, 3400, lambda poly: farbe("#2E221A") if poly.center.y < k.y - 0.2 else fell.lerp(fell_dunkel, 0.3),
                 b.hals_kopf, glatt=True)
    b.augen([k + Vector((0.078 * s, -0.075, 0.04)) for s in (1, -1)], 0.021, farbe("#C8201A"), blick=Vector((0.4, -1, 0.1)).normalized())
    for s in (1, -1):
        b.f.kugel("Nuester", k + Vector((0.035 * s, -0.275, -0.06)), (0.018, 0.01, 0.014), farbe("#0E0A08"), KOPF, 8, 6)
        # Hörner: nach außen, dann nach vorne oben geschwungen
        wurzel = k + Vector((0.12 * s, 0.03, 0.1))
        punkte = [wurzel, wurzel + Vector((0.12 * s, 0.0, 0.02)), wurzel + Vector((0.24 * s, -0.06, 0.12)), wurzel + Vector((0.28 * s, -0.14, 0.26))]
        b.f.straehne("Horn", punkte, 0.05, 0.004, lambda i, k2, p: horn_dunkel.lerp(horn, min(1.0, i / 1.5)), KOPF, 12, 0.0, 1.0, glatt=True)
        # Ohren seitlich unter den Hörnern
        ohr = k + Vector((0.14 * s, 0.05, 0.02))
        b.f.straehne("Ohr", [ohr, ohr + Vector((0.07 * s, 0.02, -0.02)), ohr + Vector((0.13 * s, 0.04, -0.05))], 0.035, 0.006, fell, KOPF, 8, 0.0, 0.35)
    b.f.loft("Nasenring", [(k + Vector((0, -0.27, -0.1)), X, Z, 0.035, 0.035), (k + Vector((0, -0.265, -0.1)), X, Z, 0.026, 0.026)], 14,
             lambda i, k2, p: gold, KOPF, glatt=True)
    # Stirnlocken
    for j in range(7):
        basis = k + Vector((-0.07 + 0.023 * j, -0.07, 0.13))
        b.f.straehne("Locke", [basis, basis + Vector((0.01, -0.04, 0.0)), basis + Vector((0.0, -0.06, -0.05))], 0.022, 0.004, fell_dunkel, KOPF, 5, 0.0, 1.0)

    def doppelaxt():
        g = b.griff(-1)
        oben = Vector((0.0, -0.25, 1.0)).normalized()
        a, e = g - oben * 0.5, g + oben * 1.1
        _strecke(f, "Axtstiel", a, e, 0.03, 0.028, farbe("#3E2A1A"), b.hand(-1), 10)
        kopfmitte = g + oben * 0.95
        vorne = (Vector((0, -1, 0)) - oben * Vector((0, -1, 0)).dot(oben)).normalized()
        for richtung in (vorne, -vorne):
            ringe = []
            for j in range(9):
                t = (j - 4) / 4
                weite = 0.05 + 0.3 * (1 - t * t) ** 0.5
                ringe.append((kopfmitte + oben * t * 0.3 + richtung * (0.04 + weite * 0.5), richtung, X, weite * 0.5, 0.014 * (1 - 0.5 * abs(t))))
            f.loft("Axtblatt", ringe, 8, lambda i, k2, p: farbe("#B8BCC4") if k2 in (0, 7) else eisen * (0.9 + 0.2 * max(0.0, p.normal.z)),
                   b.hand(-1), oben_zu=True, unten_zu=True)
        f.kugel("Axtmitte", kopfmitte, (0.05, 0.06, 0.09), gold, b.hand(-1), 10, 8)
        _kegel(f, "Axtspitze", e, oben, 0.14, 0.03, eisen, b.hand(-1), 6)
    b.starr("Doppelaxt", "Hand.R", doppelaxt)
    b.skelett()
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-10, -22), (-14, -38)), gehen=(22, 34, 12, 0.04, 8, 14)))


# ===========================================================================
# Keiler (Tier mit vier Beinen)
# ===========================================================================
def keiler(seed=408):
    """Wildschwein, Schulterhöhe gut ein Meter: wuchtige Vorhand mit Borstenkamm, schmales Becken,
    langer Kopf mit Rüsselscheibe und gebogenen Hauern, kleine rote Augen."""
    t = tiere.Tier("Keiler", seed, massstab=1.1)
    t.form((0, -0.2, 0.58), (0.22, 0.26, 0.26))           # Vorhand, massig
    t.form((0, -0.26, 0.76), (0.15, 0.18, 0.12))          # Widerrist, Buckel
    t.form((0, 0.08, 0.56), (0.19, 0.24, 0.2))            # Rumpf
    t.form((0, 0.3, 0.54), (0.16, 0.16, 0.17))            # Becken, schmaler
    t.form((0, -0.44, 0.6), (0.17, 0.14, 0.18))           # Nacken
    t.form((0, -0.58, 0.5), (0.14, 0.16, 0.15))           # Kopf, keilförmig
    t.form((0, -0.74, 0.42), (0.085, 0.11, 0.08))         # kurzer, kräftiger Rüssel
    t.form((0, -0.84, 0.4), (0.07, 0.035, 0.065))         # Rüsselscheibe
    t.form((0.09, -0.6, 0.45), (0.07, 0.09, 0.07), spiegeln=True)   # Backen
    t.glied((0.12, -0.24, 0.44), (0.12, -0.22, 0.2), 0.085, 0.055)
    t.glied((0.12, -0.22, 0.2), (0.12, -0.25, 0.04), 0.05, 0.04)
    t.glied((0.12, 0.32, 0.44), (0.12, 0.27, 0.22), 0.085, 0.055)
    t.glied((0.12, 0.27, 0.22), (0.12, 0.33, 0.04), 0.05, 0.04)
    t.glied((0, 0.44, 0.56), (0, 0.52, 0.44), 0.03, 0.02, spiegeln=False)          # Ringelschwanz
    borste, borste_dunkel, schnauze = farbe("#4A3A2E"), farbe("#2A201A"), farbe("#8A6A5A")

    def zonen(p, n):
        c = borste
        if n.z > 0.4 and p.y < 0.2:
            c = borste_dunkel                                   # dunkler Rücken
        if p.z < 0.3:
            c = borste_dunkel                                   # dunkle Läufe
        if p.y < -0.8:
            c = schnauze
        if n.z < -0.4 and p.z > 0.35:
            c = borste * 1.15                                   # hellerer Bauch
        return c

    t.koerper(zonen, aufloesung=0.03, beulen=0.018, ziel=2600)
    for ort in t.augen_orte(0.075, 0.56, tiefe=0.006):
        t.kugel(ort, 0.017 * t.s, farbe("#B01E14"), name="Auge")
    for sx in (-1, 1):
        t.kugel(t.v(sx * 0.024, -0.875, 0.41), 0.013 * t.s, farbe("#1A1010"), name="Nasenloch")
        # Hauer: aus dem Unterkiefer nach oben und hinten gebogen
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=6, radius1=0.022 * t.s, radius2=0.0, depth=0.15 * t.s)
        for v in bm.verts:
            z = v.co.z / (0.15 * t.s) + 0.5
            v.co = Matrix.Rotation(0.5 - 0.9 * z, 3, "X") @ Matrix.Rotation(sx * 0.35, 3, "Y") @ v.co + t.v(sx * 0.075, -0.76, 0.42) + Vector((0, 0, 0.06 * t.s))
        t._teil(bm, "Hauer", lambda p, n: farbe("#EDE3CB"), "Kopf")
        t.ohr(t.v(0.08 * sx, -0.5, 0.63), t.v(0.14 * sx, -0.44, 0.74), 0.06, 0.025, "#3A2C22", "#6A4A3A")
    # Borstenkamm auf Nacken und Rücken
    for i in range(16):
        y = -0.52 + i * 0.05
        hoehe = 0.9 - 0.35 * abs(y + 0.26) ** 1.3
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=4, radius1=0.026 * t.s, radius2=0.0, depth=(0.12 - 0.003 * i) * t.s)
        for v in bm.verts:
            v.co = Matrix.Rotation(0.7, 3, "X") @ v.co + t.v(0, y, hoehe) + Vector((0, 0, 0.03 * t.s))
        t._teil(bm, "Borste", lambda p, n: borste_dunkel, "Koerper")
    t.knochen_dazu("Koerper", (0, 0.34, 0.56), (0, -0.3, 0.62), None)
    t.knochen_dazu("Hals", (0, -0.3, 0.62), (0, -0.5, 0.56), "Koerper")
    t.knochen_dazu("Kopf", (0, -0.5, 0.56), (0, -0.86, 0.4), "Hals")
    t.knochen_dazu("Schwanz1", (0, 0.44, 0.56), (0, 0.54, 0.42), "Koerper")
    t.bein_knochen("V", 0.12, (0.12, -0.24, 0.48), (0.12, -0.22, 0.2), (0.12, -0.26, 0.03))
    t.bein_knochen("H", 0.12, (0.12, 0.32, 0.48), (0.12, 0.27, 0.22), (0.12, 0.34, 0.03))
    stil = dict(schritt=16, schwung=26, knick=40, wippen=0.03, sprung=12, schwung_renn=40, huepfen=0.08, wedeln=0.2)
    original = tiere._animieren

    def mit_angriff(armatur, stil, knochen):
        original(armatur, stil, knochen)
        # Anlauf und Hauerstoß: ducken, nach vorne schnellen, den Kopf von unten nach oben reißen
        stoss = []
        for bild, koerper_x, hoehe, hals, kopf, vorne, hinten in ((0, 0, 0.0, 0, 0, 0, 0), (7, 6, -0.06, -20, -15, 15, 20),
                                                                  (12, -6, 0.04, -10, -30, -35, -30), (16, -10, 0.06, 15, 35, -25, -15),
                                                                  (22, -2, 0.02, 5, 10, -5, 0), (28, 0, 0.0, 0, 0, 0, 0)):
            stoss += [tiere._rot(bild, "Koerper", x=koerper_x), tiere._pos(bild, "Koerper", hoehe), tiere._rot(bild, "Hals", x=hals),
                      tiere._rot(bild, "Kopf", x=kopf)]
            for s in ("L", "R"):
                stoss += [tiere._rot(bild, f"Bein.V{s}.oben", x=vorne), tiere._rot(bild, f"Bein.V{s}.unten", x=-vorne * 0.3),
                          tiere._rot(bild, f"Bein.H{s}.oben", x=hinten), tiere._rot(bild, f"Bein.H{s}.unten", x=abs(hinten) * 0.5)]
        animation(armatur, "Angriff", 28, stoss)

    tiere._animieren = mit_angriff
    try:
        t.fertig(stil)
    finally:
        tiere._animieren = original
