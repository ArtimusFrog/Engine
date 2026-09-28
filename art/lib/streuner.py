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
from mathutils import Matrix, Quaternion, Vector, noise

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


    # --- Formen für modellierte Körper (bildhauer.py) -------------------------
    def arm_formen(self, seite, radien, muskel=0.15):
        """Schulterkugel, Oberarm mit Bizeps, Ellbogen, Unterarm – verschmelzen mit dem Rumpf."""
        import bildhauer as bh
        s, e, h = self.p("schulter", seite), self.p("ellbogen", seite), self.p("hand", seite)
        r_s, r_o, r_e, r_u, r_h = radien
        formen = [(s + Vector((-0.02 * seite, 0, 0.0)), (r_s * 1.1, r_s, r_s * 0.95))]
        formen += bh.glied(s, e, r_o, r_e, muskel=muskel, lage=0.4)
        formen += bh.glied(e, h, r_u, r_h, muskel=muskel * 0.6, lage=0.25)
        formen += [(e + Vector((0, 0.01, 0)), (r_e * 1.05, r_e * 1.1, r_e))]
        return formen

    def bein_formen(self, seite, radien, muskel=0.12):
        import bildhauer as bh
        hu, kn, ks = self.p("huefte", seite), self.p("knie", seite), self.p("knoechel", seite)
        r_h, r_o, r_k, r_w, r_f = radien
        formen = bh.glied(hu + Vector((0, 0, 0.03)), kn, r_h, r_k, muskel=muskel, lage=0.3)
        formen += bh.glied(kn, ks, r_w, r_f, muskel=muskel, lage=0.22)
        formen += [(kn + Vector((0, -0.015, 0)), (r_k * 1.05, r_k * 1.05, r_k * 1.1))]
        return formen

    def faust_formen(self, seite, g):
        """Faust mit Handrücken, vier gekrümmten Fingern, Knöcheln und Daumen."""
        griff = self.griff(seite)
        h = self.p("hand", seite)
        formen = [(h.lerp(griff, 0.4) + Vector((0.01 * seite * g, 0.005, 0.0)), (0.03 * g, 0.036 * g, 0.036 * g)),
                  (griff + Vector((0.026 * seite * g, 0.01 * g, 0.028 * g)), (0.03 * g, 0.04 * g, 0.042 * g))]
        for i in range(4):
            z = griff.z + (0.022 - 0.021 * i) * g
            for j, w in enumerate((0.4, -0.5, -1.5, -2.4)):
                rad = (0.033 - 0.002 * j) * g
                formen.append((Vector((griff.x - math.cos(w) * rad * seite, griff.y + math.sin(w) * rad, z)), (0.0115 * g,) * 3))
        for p in (Vector((0.03 * seite, -0.03, 0.035)), Vector((0.01 * seite, -0.045, 0.04)), Vector((-0.012 * seite, -0.04, 0.042))):
            formen.append((griff + p * g, (0.0125 * g,) * 3))
        return formen

    def zehen_formen(self, seite, laenge, breite, zehen=3):
        """Nackter Fuß: Ferse, Ballen, Zehen (Spitze nach -Y)."""
        a = self.p("knoechel", seite)
        boden = Vector((a.x, a.y, 0.0))
        formen = [(boden + Vector((0, 0.02, 0.045)), (breite * 0.4, 0.055, 0.05)),
                  (boden + Vector((0, -laenge * 0.33, 0.032)), (breite * 0.5, laenge * 0.3, 0.032))]
        spitzen = []
        for i in range(zehen):
            x = (i - (zehen - 1) / 2) * breite * 0.55
            spitze = boden + Vector((x, -laenge * 0.72, 0.024))
            # jede Zehe als kurze Kette vom Ballen aus, damit sie sicher mit dem Fuß verschmilzt
            ansatz = boden + Vector((x * 0.6, -laenge * 0.45, 0.03))
            for t in (0.0, 0.5, 1.0):
                formen.append((ansatz.lerp(spitze, t), (breite * 0.17, laenge * 0.12, 0.024 - 0.004 * t)))
            spitzen.append(spitze)
        return formen, spitzen

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
# Goblin und Goblin-Schamane (modelliert)
# ===========================================================================
def _goblin(f, haut, haut_hell, gebueckt=0.0):
    """Klein (gut 1,1 m), dürr mit Kugelbauch, großer Kopf mit Hakennase, langen Ohren und
    gelben Schlitzaugen; lange Arme, große Hände und Füße mit Krallen. Körper, Kopf, Hände und
    Füße sind eine durchgehende Haut."""
    import bildhauer as bh
    r = f.rng
    ky = -0.04 - gebueckt
    b = Bau(f, becken=0.5, bauch=0.58, brust=0.7, hals=0.84, kopf=0.88, scheitel=1.14, kopf_y=ky,
            schulter=(0.14, 0.0, 0.8), ellbogen=(0.21, 0.035, 0.63), hand=(0.245, 0.0, 0.47), finger=(0.255, -0.03, 0.4),
            huefte=(0.075, 0.0, 0.5), knie=(0.095, -0.03, 0.29), knoechel=(0.095, 0.03, 0.075), zehen=(0.095, -0.12, 0.02))
    b.skelett()
    kralle = farbe("#2E2A22")
    lende, lende_dunkel = farbe("#6B4A2C"), farbe("#43301E")
    rosig, dunkel = farbe("#B8765E"), haut * 0.62
    k = Vector((0, ky, 0.98))

    # ----- Die Haut: ein Körper aus einem Guss -----
    formen = [
        ((0, 0.0, 0.5), (0.095, 0.08, 0.07)),              # Becken
        ((0, -0.035, 0.6), (0.118, 0.115, 0.1)),           # Kugelbauch
        ((0, -0.01, 0.715), (0.108, 0.08, 0.085)),         # schmale Brust
        ((0, 0.01, 0.785), (0.13, 0.068, 0.045)),          # Schultergürtel
        ((0, 0.035, 0.76), (0.08, 0.05, 0.06)),            # Buckel
        ((0, -0.005, 0.8), (0.045, 0.045, 0.04)),
    ]
    formen += bh.glied((0, 0.0, 0.8), k + Vector((0, 0.04, -0.1)), 0.04, 0.036)                    # dünner Hals
    for s in (1, -1):
        formen += [((0.055 * s, -0.09, 0.72), (0.045, 0.02, 0.03)),                               # Rippen angedeutet
                   ((0.07 * s, -0.08, 0.675), (0.04, 0.02, 0.025))]
        formen += b.arm_formen(s, (0.045, 0.036, 0.03, 0.034, 0.026), muskel=0.18)
        formen += b.faust_formen(s, 1.1)
        formen += b.bein_formen(s, (0.058, 0.046, 0.035, 0.037, 0.028), muskel=0.15)
        zehen, _ = b.zehen_formen(s, 0.2, 0.1)
        formen += zehen
    # Kopf: großer Schädel, Hakennase, breites Maul, spitzes Kinn, schwere Lider
    formen += [
        (k + Vector((0, 0.02, 0.03)), (0.11, 0.12, 0.105)),
        (k + Vector((0, -0.035, -0.045)), (0.1, 0.08, 0.06)),
        (k + Vector((0, -0.1, 0.018)), (0.088, 0.028, 0.022)),          # Brauenwulst
        (k + Vector((0, -0.125, -0.01)), (0.022, 0.032, 0.032)),        # Nasenwurzel
        (k + Vector((0, -0.16, -0.03)), (0.024, 0.04, 0.026)),          # lange Nase …
        (k + Vector((0, -0.195, -0.056)), (0.019, 0.03, 0.024)),        # … mit Haken
        (k + Vector((0, -0.205, -0.078)), (0.014, 0.016, 0.019)),
        (k + Vector((0.017, -0.18, -0.07)), (0.012, 0.012, 0.01)),      # Nasenflügel
        (k + Vector((-0.017, -0.18, -0.07)), (0.012, 0.012, 0.01)),
        (k + Vector((0, -0.115, -0.08)), (0.07, 0.03, 0.012)),          # Oberlippe
        (k + Vector((0, -0.112, -0.09)), (0.068, 0.02, 0.007), True),   # Maulspalte
        (k + Vector((0, -0.105, -0.1)), (0.058, 0.026, 0.012)),         # Unterlippe
        (k + Vector((0, -0.08, -0.115)), (0.042, 0.035, 0.022)),        # spitzes Kinn
    ]
    for s in (1, -1):
        auge = k + Vector((0.043 * s, -0.093, 0.0))
        formen += [(auge + Vector((0, -0.004, 0)), (0.024, 0.017, 0.019), True),              # Augenhöhle
                   (auge + Vector((0, -0.006, 0.013)), (0.028, 0.017, 0.01)),                  # schweres Oberlid
                   (auge + Vector((0, -0.004, -0.017)), (0.022, 0.012, 0.007)),                # Tränensack
                   (k + Vector((0.068 * s, -0.075, -0.035)), (0.032, 0.026, 0.024)),           # Wangen
                   (k + Vector((0.1 * s, 0.0, 0.0)), (0.02, 0.03, 0.035))]                     # Ohrwurzel

    ohrspitzen = [k + Vector((0.3 * s, 0.1, 0.1)) for s in (1, -1)]
    nasenspitze = k + Vector((0, -0.21, -0.08))

    def haut_versatz(p, n):
        warze = bh.zellen(p, 30.0)
        beule = 0.0025 * max(0.0, 0.12 - warze) / 0.12 if bh.rausch(p, 5.0) > 0.35 else 0.0
        return 0.0008 * bh.rausch(p, 60.0, 2) + beule + 0.0012 * bh.rausch(p, 12.0)

    def haut_farbe(p, n, h):
        c = haut.lerp(haut_hell, weich(0.1, -0.7, n.y) * weich(0.78, 0.55, p.z) * 0.8)       # heller Bauch
        c = c.lerp(dunkel, weich(0.1, 0.8, n.y) * 0.35)                                       # dunklerer Rücken
        c = c * (0.93 + 0.1 * bh.rausch(p, 7.0))
        c = c.lerp(rosig, weich(0.07, 0.0, (p - nasenspitze).length) * 0.7)                  # rote Nasenspitze
        c = c.lerp(rosig, weich(0.06, 0.02, abs(p.z - (k.z - 0.09))) * weich(0.1, 0.06, abs(p.y - (k.y - 0.11))) * weich(0.08, 0.03, abs(p.x)) * 0.5)
        c = c.lerp(farbe("#3A2E20"), weich(0.12, 0.02, p.z) * 0.55)                          # Dreck an den Füßen
        if bh.zellen(p, 30.0) < 0.05 and bh.rausch(p, 5.0) > 0.35:
            c = c * 0.85                                                                     # Warzen
        return bh.schmutz(c, h, 0.45, 0.15)
    koerper = bh.teil(f, "Haut", formen, 0.0055, 16000, haut_farbe, versatz=haut_versatz, knochen=f.knochen)

    # ----- Augen, Zähne, Ohren, Haare, Krallen -----
    b.augen([k + Vector((0.043 * s, -0.093, 0.0)) for s in (1, -1)], 0.019, farbe("#F2C21C"), schlitz=True, weiss=farbe("#E8E0A8"))
    for i, x in enumerate((-0.048, -0.028, 0.03, 0.05)):
        oben = i % 2 == 0
        basis = k + Vector((x, -0.117 + abs(x) * 0.25, -0.086 if oben else -0.094))
        obj = _kegel(f, "Zahn", basis, Vector((r.uniform(-0.15, 0.15), -0.3, -1 if oben else 1)), r.uniform(0.02, 0.028), 0.0065, farbe("#E8DDB8"), KOPF, 8)
        bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#E8DDB8").lerp(farbe("#9A8A5A"), weich(0.4, -0.2, n.z) * 0.4))
    for s in (1, -1):
        wurzel = k + Vector((0.095 * s, 0.0, 0.0))
        punkte = [wurzel, wurzel + Vector((0.07 * s, 0.025, 0.025)), wurzel + Vector((0.15 * s, 0.06, 0.06)), wurzel + Vector((0.22 * s, 0.1, 0.1))]
        ringe = []
        for j, p in enumerate(punkte):
            t = j / 3
            ringe.append((p, Vector((0, 0.35, -1)).normalized(), Vector((0, 1, 0.35)).normalized(), 0.045 * (1 - t) ** 0.8 + 0.003, 0.011 * (1 - t) + 0.002))
        obj = f.loft("Ohr", ringe, 16, lambda i, k2, p: haut, KOPF, oben_zu=True, unten_zu=True, teilung=4, glatt=True)
        spitze = punkte[-1]
        bh.glatt_einfaerben(obj, lambda p, n, h, spitze=spitze: haut.lerp(rosig, weich(0.1, 0.0, (p - spitze).length) * 0.5 + weich(0.2, -0.2, abs(n.x)) * 0.35))
        # Ein Ohrring aus Knochen, eine Kerbe
        b.f.loft("Ohrring", [(wurzel + Vector((0.06 * s, 0.03, -0.02)), Y, Z, 0.013, 0.013), (wurzel + Vector((0.062 * s, 0.03, -0.02)), Y, Z, 0.009, 0.009)],
                 12, lambda i, k2, p: farbe("#DCD4C0"), KOPF, glatt=True)
    for i in range(11):
        w = r.uniform(-0.9, 0.9)
        basis = k + Vector((math.sin(w) * 0.06, 0.03 + r.uniform(-0.03, 0.05), 0.125))
        obj = f.straehne("Haar", [basis, basis + Vector((math.sin(w) * 0.03, 0.035, 0.05)), basis + Vector((math.sin(w) * 0.07, 0.08, 0.07))],
                         0.009, 0.001, farbe("#2A2420"), KOPF, 6, 0.0, 1.0, glatt=True)
    for s in (1, -1):
        # Krallen an den Fingerknöcheln und Zehen
        griff = b.griff(s)
        for i in range(4):
            z = griff.z + (0.022 - 0.021 * i) * 1.1
            kn = Vector((griff.x - math.cos(-1.5) * 0.036 * 1.1 * s, griff.y - 0.04, z))
            obj = f.straehne("Kralle", [kn, kn + Vector((0, -0.015, -0.012)), kn + Vector((0, -0.018, -0.028))], 0.0065, 0.001, kralle, b.hand(s), 6, 0.0, 1.0, glatt=True)
        _, spitzen = b.zehen_formen(s, 0.2, 0.1)
        for sp in spitzen:
            f.straehne("Kralle", [sp + Vector((0, -0.02, 0.005)), sp + Vector((0, -0.04, 0.0)), sp + Vector((0, -0.05, -0.018))], 0.011, 0.002, kralle, b.fuss(s), 6, 0.0, 1.0, glatt=True)

    # ----- Kleidung: Lendenschurz, Gürtel, Riemen, Beutel, Wickel -----
    def stoff_farbe(basis):
        return lambda p, n, h: bh.schmutz(basis * (0.88 + 0.16 * bh.rausch(p, 14.0)) * (0.9 + 0.1 * weich(0.3, 0.5, p.z)), h, 0.4, 0.1)
    obj = f.loft("Lendenschurz", [(Vector((0, -0.01, 0.545)), X, Y, 0.112, 0.1), (Vector((0, -0.015, 0.46)), X, Y, 0.128, 0.115, _falten(r, 0.06)),
                                  (Vector((0, -0.015, 0.36)), X, Y, 0.138, 0.124, _zacken(r, 0.16, 11))], 40,
                 lambda i, k2, p: lende, b.rock, teilung=4, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: stoff_farbe(lende)(p, n, h).lerp(lende_dunkel, weich(0.42, 0.36, p.z) * 0.6))
    bh.gewichte_uebertragen(obj, koerper, lambda kn: kn in ("Becken", "Bauch") or kn.startswith("Oberschenkel"))
    obj = f.loft("Guertel", [(Vector((0, -0.01, 0.528)), X, Y, 0.116, 0.104), (Vector((0, -0.01, 0.566)), X, Y, 0.118, 0.106)], 32,
                 lambda i, k2, p: farbe("#3A2616"), b.rumpf, teilung=2, glatt=True)
    bh.glatt_einfaerben(obj, stoff_farbe(farbe("#3A2616")))
    b.f.kugel("Schnalle", (0, -0.11, 0.548), (0.022, 0.012, 0.018), farbe("#DCD4C0"), b.rumpf, 12, 8)
    riemen = [Vector((0.105, -0.055, 0.8)), Vector((0.03, -0.115, 0.71)), Vector((-0.07, -0.13, 0.61)), Vector((-0.115, -0.06, 0.545))]
    obj = f.straehne("Riemen", riemen, 0.016, 0.016, farbe("#3A2616"), b.rumpf, 8, 0.0, 0.3, teilung=3, glatt=True)
    bh.glatt_einfaerben(obj, stoff_farbe(farbe("#4A3220")))
    beutel = bh.ball_mesh("Beutel", [((-0.12, -0.055, 0.49), (0.045, 0.035, 0.05)), ((-0.12, -0.055, 0.535), (0.03, 0.025, 0.015))], 0.01)
    bh.modellieren(beutel, 0.006, 1200, lambda p, n: 0.002 * bh.rausch(p, 40.0))
    bh.einfaerben(beutel, stoff_farbe(lende))
    f._gewichten(beutel, lambda co: {"Becken": 1.0})
    bh.aufnehmen(f, beutel)
    for s in (1, -1):
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        ringe = [(e.lerp(h, 0.66 + 0.06 * j), q1, q2, 0.033 - 0.001 * j, 0.033 - 0.001 * j) for j in range(6)]
        obj = f.loft("Wickel", ringe, 16, lambda i, k2, p: farbe("#8A7A5A"), b.arm(s), teilung=2, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(farbe("#8A7A5A") * (0.8 + 0.25 * bh.rausch(p, 30.0)), h, 0.6, 0.1))
    return b, koerper


def _hackmesser(b, rostig=True):
    """Grobes Hackmesser mit gezackter, fleckiger Klinge in der rechten Hand."""
    import bildhauer as bh
    f = b.f
    griff = b.griff(-1)
    unten = Vector((0, -0.55, -0.83)).normalized()        # Klinge schräg nach vorne unten
    quer = unten.cross(X).normalized()
    stahl, rost = farbe("#8E8E88"), farbe("#7A4A2E")
    obj = _strecke(f, "Heft", griff - unten * 0.07, griff + unten * 0.08, 0.017, 0.017, farbe("#4A3220"), b.hand(-1), 12)
    bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#4A3220") * (0.8 + 0.3 * bh.rausch(p, 60.0)))
    for t in (-0.04, 0.0, 0.04):
        q1, q2 = _achsen(griff, griff + unten)
        f.loft("Wicklung", [(griff + unten * (t - 0.012), q1, q2, 0.02, 0.02), (griff + unten * (t + 0.012), q1, q2, 0.02, 0.02)], 12,
               lambda i, k2, p: farbe("#2A1E14"), b.hand(-1), glatt=True)
    a = griff + unten * 0.08
    ringe = []
    for j in range(12):
        t = j / 11
        breite = 0.045 + 0.035 * t
        zacke = 0.01 if j % 3 == 1 else 0.0
        ringe.append((a + unten * 0.42 * t + quer * (breite * 0.5 - zacke * 0.5), quer, X, breite * 0.5 + zacke * 0.5, 0.005))
    obj = f.loft("Klinge", ringe, 8, lambda i, k2, p: stahl, b.hand(-1), oben_zu=True, unten_zu=True, teilung=2)
    bh.glatt_einfaerben(obj, lambda p, n, h: stahl.lerp(rost, weich(0.0, 0.5, bh.rausch(p, 25.0)) * (0.8 if rostig else 0.0)).lerp(farbe("#D8D8D2"), weich(0.3, 0.9, -h) * 0.6))


def goblin(seed=401):
    f = Figur("Goblin", seed)
    b, _ = _goblin(f, farbe("#56703A"), farbe("#8C9A5E"))
    b.starr("Hackmesser", "Hand.R", lambda: _hackmesser(b))
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-10, -25), (-18, -45)), gehen=(30, 46, 20, 0.03, 10, 18)))


def goblin_schamane(seed=402):
    import bildhauer as bh
    f = Figur("GoblinSchamane", seed)
    r = f.rng
    haut, hell = farbe("#4A6238"), farbe("#788C5C")
    b, koerper = _goblin(f, haut, hell, gebueckt=0.04)
    k = Vector((0, -0.08, 0.98))
    federn = [farbe("#C8322A"), farbe("#E8B030"), farbe("#2E6AA8"), farbe("#F2EEE0")]
    # Vogelschädel als Maske auf der Stirn, darüber ein Federkranz
    schaedel = bh.ball_mesh("Vogelschaedel", [(k + Vector((0, -0.05, 0.125)), (0.06, 0.07, 0.045)), (k + Vector((0, -0.1, 0.12)), (0.035, 0.05, 0.03)),
                                              (k + Vector((0.03, -0.1, 0.132)), (0.014, 0.012, 0.012), True),
                                              (k + Vector((-0.03, -0.1, 0.132)), (0.014, 0.012, 0.012), True)], 0.008)
    bh.modellieren(schaedel, 0.005, 2500, lambda p, n: 0.0015 * bh.rausch(p, 50.0))
    bh.einfaerben(schaedel, lambda p, n, h: bh.schmutz(farbe("#E4DAC2") * (0.9 + 0.1 * bh.rausch(p, 20.0)), h, 0.6, 0.1))
    f._gewichten(schaedel, KOPF)
    bh.aufnehmen(f, schaedel)
    obj = _kegel(f, "Schnabel", k + Vector((0, -0.13, 0.12)), Vector((0, -1, -0.5)), 0.12, 0.028, farbe("#D6C69A"), KOPF, 10, krumm=Vector((0, 0, -1)))
    bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#D6C69A").lerp(farbe("#8A7A4A"), weich(-0.1, -0.2, p.y - k.y) * 0.5))
    for i in range(11):
        w = -1.25 + 2.5 * i / 10
        basis = k + Vector((math.sin(w) * 0.09, 0.03 + math.cos(w) * 0.02, 0.13))
        spitze = basis + Vector((math.sin(w) * 0.12, 0.08, 0.22 - abs(w) * 0.06))
        c = federn[i % 4]
        obj = f.straehne("Feder", [basis, basis.lerp(spitze, 0.5) + Vector((0, 0.02, 0.02)), spitze], 0.018, 0.003, c, KOPF, 8, 0.0, 0.25, teilung=3, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h, c=c, basis=basis: c.lerp(c * 0.5, weich(0.05, 0.0, (p - basis).length)))
    # Knochenkette mit Zähnen und Perlen
    for i in range(13):
        w = math.pi * (0.15 + 0.7 * i / 12)
        p = Vector((math.cos(w) * 0.1, -0.075 - math.sin(w) * 0.03, 0.8 - math.sin(w) * 0.07))
        if i % 3 == 1:
            _kegel(f, "Kettenzahn", p, Vector((0, -0.3, -1)), 0.035, 0.009, farbe("#E8DDB8"), b.rumpf, 6)
        else:
            b.f.kugel("Perle", p, (0.011, 0.011, 0.011), [farbe("#B8322A"), farbe("#DCD4C0"), farbe("#2E6AA8")][i % 3], b.rumpf, 10, 6)
    # Umhang aus Blättern: einzelne überlappende Blätter in Reihen
    blatt, blatt_dunkel, blatt_hell = farbe("#4E7A2A"), farbe("#2E4A18"), farbe("#8AAA3A")
    for reihe in range(5):
        z = 0.8 - reihe * 0.095
        weite = 0.12 + reihe * 0.022
        n = 9 + reihe
        for j in range(n):
            w = math.radians(70 + 220 * (j + 0.5 * (reihe % 2)) / n)
            mitte = Vector((math.sin(w) * weite, 0.03 + math.cos(w) * -weite * 0.85 * -1, z))
            mitte.y = abs(mitte.y) * 0.9 + 0.02
            aussen = Vector((mitte.x, mitte.y, 0)).normalized()
            c = [blatt, blatt_dunkel, blatt_hell][(j + reihe) % 3]
            obj = _platte(f, "Blatt", mitte + aussen * 0.01 - Vector((0, 0, 0.04)), 0.13, 0.035, 0.005, Vector((0, 0, -1)) + aussen * 0.25, aussen,
                          lambda i2, k2, p, c=c: c, b.rumpf, spitz=0.7, wolbung=0.6)
            bh.glatt_einfaerben(obj, lambda p, n, h, c=c: c * (0.8 + 0.3 * bh.rausch(p, 30.0)))
            bh.gewichte_uebertragen(obj, koerper, lambda kn: kn in ("Brust", "Bauch", "Becken"))

    def stab():
        g = b.griff(-1)
        oben = Vector((0.02, 0.2, 1.0)).normalized()
        holz = farbe("#5A3E26")
        punkte = [g - oben * 0.38 + Vector((0.01, 0, 0)), g - oben * 0.1, g + oben * 0.25 + Vector((0.012, 0, 0)), g + oben * 0.55 - Vector((0.01, 0, 0)),
                  g + oben * 0.72]
        obj = b.f.straehne("Stab", punkte, 0.018, 0.014, holz, b.hand(-1), 10, 0.4, 1.0, teilung=4, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: holz * (0.75 + 0.35 * bh.rausch(Vector((p.x * 40, p.y * 40, p.z * 6)), 1.0)))
        spitze = g + oben * 0.8
        for j in range(4):
            w = math.tau * j / 4
            _kegel(f, "Krallenfassung", spitze + Vector((math.cos(w) * 0.03, math.sin(w) * 0.03, -0.06)), Vector((math.cos(w) * 0.3, math.sin(w) * 0.3, 1)),
                   0.1, 0.01, holz * 0.8, b.hand(-1), 6, krumm=Vector((-math.cos(w), -math.sin(w), 0)))
        b.f.kugel("Kristall", spitze, (0.035, 0.035, 0.05), farbe("#7CFF6A"), b.hand(-1), 8, 6, glatt=False)
        for j, farbe_feder in enumerate(federn[:3]):
            a = spitze + Vector((0.02 * (j - 1), 0.01, -0.09))
            b.f.straehne("Stabfeder", [a, a + Vector((0.03 * (j - 1), 0.01, -0.07)), a + Vector((0.05 * (j - 1), 0.02, -0.14))], 0.012, 0.002,
                         farbe_feder, b.hand(-1), 6, 0.0, 0.3, glatt=True)
    b.starr("Stab", "Hand.R", stab)
    return f.fertig(_animationen(_angriff_zauber, arme_ruhe=((-8, -20), (-8, -20)), gehen=(24, 38, 14, 0.025, 14, 22)))


def _haut_poly(c, hell):
    return lambda poly: c.lerp(hell, max(0.0, -poly.normal.y) * 0.3) * (0.92 + 0.1 * max(0.0, poly.normal.z))


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


def _veredeln(obj, c, skala=20.0, staerke=0.25, hoehle=0.45, kante=0.15):
    """Teile aus Lofts: weiche Farbverläufe mit etwas Rauschen, Schmutz in Vertiefungen."""
    import bildhauer as bh
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(c * (1.0 - staerke * 0.5 + staerke * (0.5 + 0.5 * bh.rausch(p, skala))), h, hoehle, kante))
    return obj


# ===========================================================================
# Ork-Berserker (modelliert)
# ===========================================================================
def ork_berserker(seed=403):
    """Gut zwei Meter, massiger Oberkörper mit ausgeprägten Muskeln, kleiner Kopf mit Unterbiss
    und Hauern, Haarknoten, rote Kriegsbemalung und Narben; Pelzschurz, Lederharnisch,
    Stachel-Schulterpanzer, große Streitaxt."""
    import bildhauer as bh
    f = Figur("OrkBerserker", seed)
    r = f.rng
    b = Bau(f, becken=1.02, bauch=1.18, brust=1.4, hals=1.7, kopf=1.74, scheitel=2.02, kopf_y=-0.08,
            schulter=(0.3, 0.0, 1.62), ellbogen=(0.42, 0.05, 1.32), hand=(0.46, 0.0, 1.06), finger=(0.47, -0.03, 0.95),
            huefte=(0.14, 0.0, 1.0), knie=(0.16, -0.03, 0.56), knoechel=(0.16, 0.03, 0.1), zehen=(0.16, -0.17, 0.03))
    b.skelett()
    haut, haut_hell, haut_dunkel = farbe("#5A6E38"), farbe("#7E8E52"), farbe("#3A4824")
    bemalung, narbe = farbe("#8E1E1A"), farbe("#8A7A5A")
    leder, leder_dunkel = farbe("#5A3A22"), farbe("#3A2416")
    eisen, eisen_hell = farbe("#4A4C50"), farbe("#8A8E96")
    fell, fell_hell = farbe("#5E4430"), farbe("#8A6A4A")
    k = Vector((0, -0.1, 1.86))

    formen = [
        ((0, 0.0, 1.02), (0.19, 0.15, 0.1)),                 # Becken
        ((0, -0.01, 1.14), (0.2, 0.16, 0.1)),                # Taille
        ((0, -0.03, 1.28), (0.24, 0.19, 0.12)),              # Bauch
        ((0, -0.04, 1.44), (0.31, 0.22, 0.13)),              # Brustkorb
        ((0, -0.02, 1.56), (0.33, 0.21, 0.1)),
        ((0, 0.02, 1.64), (0.3, 0.17, 0.07)),                # Schultergürtel
        ((0, 0.06, 1.7), (0.2, 0.14, 0.08)),                 # Nackenbuckel
    ]
    for s in (1, -1):
        formen += [((0.13 * s, -0.17, 1.51), (0.13, 0.07, 0.09)),        # Brustmuskeln
                   ((0.14 * s, 0.03, 1.68), (0.12, 0.1, 0.07)),          # Trapez
                   ((0.2 * s, 0.1, 1.45), (0.1, 0.07, 0.14)),            # breiter Rücken
                   ((0.17 * s, -0.12, 1.24), (0.07, 0.07, 0.09))]        # schräge Bauchmuskeln
        for i in range(3):
            formen.append(((0.048 * s, -0.19 + 0.006 * i, 1.35 - 0.07 * i), (0.045, 0.028, 0.032)))    # Bauchmuskeln
        formen += b.arm_formen(s, (0.135, 0.115, 0.085, 0.098, 0.072), muskel=0.25)
        formen += [(b.p("schulter", s) + Vector((0.03 * s, -0.02, -0.04)), (0.13, 0.12, 0.13))]      # Deltamuskel
        formen += b.faust_formen(s, 1.55)
        formen += b.bein_formen(s, (0.15, 0.12, 0.1, 0.105, 0.075), muskel=0.2)
    formen += bh.glied((0, 0.02, 1.66), k + Vector((0, 0.04, -0.12)), 0.13, 0.11)
    formen += [
        (k + Vector((0, 0.03, 0.04)), (0.105, 0.115, 0.1)),
        (k + Vector((0, -0.04, -0.07)), (0.12, 0.1, 0.075)),               # Unterkiefer, vorgeschoben
        (k + Vector((0, -0.11, -0.09)), (0.09, 0.045, 0.05)),
        (k + Vector((0, -0.1, 0.035)), (0.1, 0.035, 0.03)),               # Brauenwulst
        (k + Vector((0, -0.135, 0.0)), (0.036, 0.032, 0.03)),             # breite, flache Nase
        (k + Vector((0.024, -0.14, -0.018)), (0.02, 0.018, 0.017)),
        (k + Vector((-0.024, -0.14, -0.018)), (0.02, 0.018, 0.017)),
        (k + Vector((0.014, -0.157, -0.02)), (0.007, 0.006, 0.006), True),   # Nasenlöcher
        (k + Vector((-0.014, -0.157, -0.02)), (0.007, 0.006, 0.006), True),
        (k + Vector((0, -0.14, -0.05)), (0.06, 0.03, 0.007), True),       # Maul
    ]
    for s in (1, -1):
        auge = k + Vector((0.045 * s, -0.1, 0.005))
        formen += [(auge + Vector((0, -0.003, 0)), (0.024, 0.016, 0.017), True),
                   (auge + Vector((0, -0.007, 0.014)), (0.028, 0.016, 0.009)),
                   (k + Vector((0.08 * s, -0.08, -0.03)), (0.042, 0.036, 0.03)),
                   (k + Vector((0.1 * s, 0.02, 0.0)), (0.025, 0.035, 0.04))]

    narben = [(Vector((0.1, -0.2, 1.52)), Vector((0.2, -0.17, 1.4))), (Vector((-0.08, -0.2, 1.3)), Vector((-0.02, -0.21, 1.2))),
              (Vector((0.03, k.y - 0.1, k.z + 0.06)), Vector((0.07, k.y - 0.08, k.z - 0.02)))]

    def naehe(p, a, b_):
        t = max(0.0, min(1.0, (p - a).dot(b_ - a) / (b_ - a).length_squared))
        return (a.lerp(b_, t) - p).length

    def haut_versatz(p, n):
        ader = abs(noise.noise(Vector((p.x * 18, p.y * 18, p.z * 5))))
        adern = 0.003 * max(0.0, 0.08 - ader) / 0.08 if abs(p.x) > 0.3 else 0.0      # Adern an den Armen
        narbe_tief = sum(-0.004 * max(0.0, 0.012 - naehe(p, a, b_)) / 0.012 for a, b_ in narben)
        return 0.0012 * bh.rausch(p, 50.0, 2) + adern + narbe_tief

    def haut_farbe(p, n, h):
        c = haut.lerp(haut_hell, weich(0.0, -0.8, n.y) * 0.35).lerp(haut_dunkel, weich(0.2, 0.9, n.y) * 0.35)
        c = c * (0.92 + 0.12 * bh.rausch(p, 6.0))
        # Kriegsbemalung: zwei Bänder um jeden Oberarm, drei Krallenstreifen über die Brust, schwarze Augenbinde
        if abs(p.x) > 0.3 and (1.44 < p.z < 1.48 or 1.51 < p.z < 1.54):
            c = bemalung
        for j in range(3):
            a = Vector((0.05 + 0.06 * j, -0.2, 1.58))
            if naehe(p, a, a + Vector((-0.14, 0, -0.2))) < 0.016 and n.y < -0.3:
                c = c.lerp(bemalung, 0.9)
        if abs(p.z - (k.z + 0.005)) < 0.025 and p.y < k.y - 0.05 and abs(p.x) < 0.1:
            c = c.lerp(farbe("#1A1410"), 0.75)
        for a, b_ in narben:
            if naehe(p, a, b_) < 0.012:
                c = narbe
        return bh.schmutz(c, h, 0.5, 0.18)
    koerper = bh.teil(f, "Haut", formen, 0.008, 22000, haut_farbe, versatz=haut_versatz, knochen=f.knochen)

    b.augen([k + Vector((0.045 * s, -0.1, 0.005)) for s in (1, -1)], 0.017, farbe("#E07A1A"))
    for s in (1, -1):
        obj = _kegel(f, "Hauer", k + Vector((0.045 * s, -0.14, -0.075)), Vector((0.2 * s, -0.25, 1)), 0.085, 0.017, farbe("#E8DDB8"), KOPF, 10,
                     krumm=Vector((0.3 * s, 0.4, 0)))
        bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#E8DDB8").lerp(farbe("#8A7A4A"), weich(k.z - 0.02, k.z - 0.08, p.z) * 0.6))
        wurzel = k + Vector((0.1 * s, 0.02, 0.0))
        ringe = [(wurzel + Vector((0.04 * s * t, 0.07 * t, 0.035 * t)), Vector((0, 0.4, -1)).normalized(), Vector((0, 1, 0.4)).normalized(),
                  0.035 * (1 - t) + 0.003, 0.01 * (1 - t) + 0.002) for t in (0.0, 0.5, 1.0)]
        obj = f.loft("Ohr", ringe, 14, lambda i, k2, p: haut, KOPF, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        _veredeln(obj, haut, 30.0, 0.15)
        for j in range(2):
            f.loft("Ohrring", [(wurzel + Vector((0.03 * s, 0.03 + 0.03 * j, -0.02)), Y, Z, 0.014, 0.014),
                               (wurzel + Vector((0.032 * s, 0.03 + 0.03 * j, -0.02)), Y, Z, 0.01, 0.01)], 12, lambda i, k2, p: farbe("#C9A040"), KOPF, glatt=True)
    haar = farbe("#1A1614")
    knoten = bh.auf_haut(koerper, k + Vector((0, 0.08, 0.2)), 0.03)
    b.f.kugel("Haarknoten", knoten, (0.042, 0.042, 0.05), haar, KOPF, 16, 10)
    # Ein dicker Zopf fällt vom Knoten über den Nacken
    zopf = [knoten + Vector((0, 0.03, 0.02)), knoten + Vector((0, 0.1, -0.04)), knoten + Vector((0, 0.13, -0.16)), knoten + Vector((0, 0.12, -0.3))]
    obj = f.straehne("Zopf", zopf, 0.03, 0.012, haar, lambda co: _mischen(("Kopf", weich(1.6, 1.8, co.z)), ("Hals", 1 - weich(1.6, 1.8, co.z))), 10, 0.8, 0.8,
                     teilung=4, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: haar * (0.8 + 0.6 * max(0.0, math.sin((p.z * 60 + p.x * 40)))))
    for t in (0.35, 0.7):
        m = zopf[1].lerp(zopf[3], t)
        f.loft("Zopfring", [(m + Vector((0, 0, -0.012)), X, Y, 0.024, 0.024), (m + Vector((0, 0, 0.012)), X, Y, 0.024, 0.024)], 14,
               lambda i, k2, p: farbe("#C9A040"), KOPF, glatt=True)
    f.loft("Haarband", [(knoten + Vector((0, 0, -0.02)), X, Y, 0.044, 0.044), (knoten + Vector((0, 0, 0.0)), X, Y, 0.045, 0.045)], 14,
           lambda i, k2, p: farbe("#C9A040"), KOPF, glatt=True)

    # Harnisch, Gürtel, Schurz, Armschienen, Schulterpanzer, Stiefel
    # Hose: eng an der Haut von der Hüfte bis unters Knie
    hose = farbe("#3E3024")
    kn_z = b.p("knie", 1).z
    obj = bh.huelle(f, koerper, "Hose", lambda p, n: kn_z - 0.06 < p.z < 1.06 and not (p.z > 0.98 and abs(p.x) < 0.08 and p.y < -0.1), 0.012,
                    lambda p, n, h: bh.schmutz(hose * (0.85 + 0.25 * bh.rausch(Vector((p.x * 20, p.y * 20, p.z * 60)), 1.0)), h, 0.5, 0.15))
    for s in (1, -1):
        punkte = [Vector((0.24 * s, -0.08, 1.64)), Vector((0.12 * s, -0.23, 1.5)), Vector((0.0, -0.245, 1.38)), Vector((-0.12 * s, -0.23, 1.24)),
                  Vector((-0.2 * s, -0.14, 1.12))]
        punkte = [bh.auf_haut(koerper, p, 0.014) for p in punkte]
        obj = f.straehne("Harnischriemen", punkte, 0.034, 0.034, leder, b.rumpf, 8, 0.0, 0.25, teilung=4, glatt=True)
        _veredeln(obj, leder, 40.0)
    ring = bh.auf_haut(koerper, Vector((0, -0.3, 1.38)), 0.02)
    f.loft("Harnischring", [(ring, X, Z, 0.05, 0.05), (ring + Vector((0, 0.01, 0)), X, Z, 0.035, 0.035)], 20, lambda i, k2, p: eisen_hell, b.rumpf, glatt=True)
    my, rx, ry = bh.umfang(koerper, 1.05, 0.03, nur=lambda p: abs(p.x) < 0.3)
    obj = f.loft("Guertel", [(Vector((0, my, 1.0)), X, Y, rx + 0.02, ry + 0.02), (Vector((0, my, 1.1)), X, Y, rx + 0.025, ry + 0.025)], 40,
                 lambda i, k2, p: leder_dunkel, b.rumpf, teilung=2, glatt=True)
    _veredeln(obj, leder_dunkel, 40.0)
    sp = Vector((0, my - ry - 0.04, 1.06))
    schnalle = bh.ball_mesh("Schaedelschnalle", [(sp, (0.055, 0.035, 0.05)), (sp + Vector((0, -0.01, -0.035)), (0.035, 0.03, 0.025)),
                                                 (sp + Vector((0.021, -0.035, 0.005)), (0.014, 0.012, 0.014), True), (sp + Vector((-0.021, -0.035, 0.005)), (0.014, 0.012, 0.014), True)], 0.008)
    bh.modellieren(schnalle, 0.005, 1500)
    bh.einfaerben(schnalle, lambda p, n, h: bh.schmutz(farbe("#DCD4C0"), h, 0.8, 0.1))
    f._gewichten(schnalle, b.rumpf)
    bh.aufnehmen(f, schnalle)
    obj = f.loft("Pelzschurz", [(Vector((0, my, 1.02)), X, Y, rx + 0.03, ry + 0.03), (Vector((0, my, 0.86)), X, Y, rx + 0.07, ry + 0.06, _falten(r, 0.07)),
                                (Vector((0, my, 0.68)), X, Y, rx + 0.09, ry + 0.08, _zacken(r, 0.2, 15))], 48,
                 lambda i, k2, p: fell, b.rock, teilung=4, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(fell.lerp(fell_hell, 0.5 + 0.5 * bh.rausch(Vector((p.x * 60, p.y * 60, p.z * 8)), 1.0)), h, 0.5, 0.1))
    bh.gewichte_uebertragen(obj, koerper, lambda kn: kn in ("Becken", "Bauch") or kn.startswith("Oberschenkel"))
    for s in (1, -1):
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        obj = f.loft("Armschiene", [(e.lerp(h, 0.42), q1, q2, 0.124, 0.124), (e.lerp(h, 0.7), q1, q2, 0.116, 0.116), (e.lerp(h, 0.98), q1, q2, 0.096, 0.096)], 24,
                     lambda i, k2, p: leder, b.arm(s), teilung=2, glatt=True)
        _veredeln(obj, leder, 40.0)
        for j in range(3):
            b.f.kugel("Niete", e.lerp(h, 0.55 + 0.15 * j) + Vector((0.115 * s, -0.03, 0)), (0.013, 0.013, 0.013), eisen_hell, b.arm(s), 10, 6)
    sl = b.p("schulter", 1)
    for j in range(3):
        obj = _schale(f, "Schulterplatte", sl + Vector((0.02, 0.0, 0.06 - 0.06 * j)), 0.18 - 0.015 * j, 0.18 - 0.015 * j, 0.12, (0, 360), (0, 80),
                      lambda poly: eisen, b.arm(1), seg=(32, 10))
        bh.glatt_einfaerben(obj, lambda p, n, h, j=j: bh.schmutz(eisen.lerp(eisen_hell, max(0.0, n.z) * 0.6) * (1.0 - 0.08 * j) * (0.9 + 0.2 * bh.rausch(p, 30.0)), h, 0.6, 0.4))
    for j in range(3):
        w = -0.6 + 0.6 * j
        basis = sl + Vector((0.06 + 0.04 * math.sin(w), 0.1 * math.sin(w), 0.16))
        _kegel(f, "Stachel", basis, Vector((0.4, math.sin(w) * 0.3, 1)), 0.16, 0.028, eisen_hell, b.arm(1), 10)
    hose = farbe("#3E3024")
    for s in (1, -1):
        a = b.p("knoechel", s)
        obj = f.loft("Fellwickel", [(a + Vector((0, 0, 0.06)), X, Y, 0.1, 0.1, _falten(r, 0.08)), (a + Vector((0, 0, 0.18)), X, Y, 0.11, 0.11, _falten(r, 0.1)),
                                    (a + Vector((0, 0, 0.28)), X, Y, 0.105, 0.105, _falten(r, 0.08))], 30, lambda i, k2, p: fell, b.bein(s), teilung=2, glatt=True)
        _veredeln(obj, fell, 50.0, 0.4)
        fussring = [(Vector((a.x, 0.07, 0.06)), X, Z, 0.08, 0.06), (Vector((a.x, 0.04, 0.025)), X, Z, 0.09, 0.035),
                    (Vector((a.x, -0.07, 0.06)), X, Z, 0.095, 0.06), (Vector((a.x, -0.18, 0.05)), X, Z, 0.08, 0.045),
                    (Vector((a.x, -0.22, 0.05)), X, Z, 0.01, 0.01)]
        obj = f.loft("Stiefel", fussring, 24, lambda i, k2, p: leder_dunkel, b.fuss(s), oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(leder_dunkel * (0.6 if p.z < 0.015 else 1.0) * (0.9 + 0.2 * bh.rausch(p, 30.0)), h, 0.5, 0.2))

    def axt():
        g = b.griff(-1)
        oben = Vector((0.0, -0.3, 1.0)).normalized()
        holz = farbe("#4A3220")
        a, e = g - oben * 0.35, g + oben * 1.0
        obj = _strecke(f, "Axtstiel", a, e, 0.026, 0.024, holz, b.hand(-1), 14)
        bh.glatt_einfaerben(obj, lambda p, n, h: holz * (0.75 + 0.35 * bh.rausch(Vector((p.x * 50, p.y * 50, p.z * 6)), 1.0)))
        q1, q2 = _achsen(a, e)
        for t in (0.0, 0.12):
            f.loft("Wicklung", [(g + oben * (t - 0.05), q1, q2, 0.031, 0.031), (g + oben * (t + 0.05), q1, q2, 0.031, 0.031)], 14,
                   lambda i, k2, p: leder_dunkel, b.hand(-1), glatt=True)
        kopfmitte = g + oben * 0.86
        vorne = Vector((0, -1, -0.3)).normalized()
        vorne = (vorne - oben * vorne.dot(oben)).normalized()
        ringe = []
        for j in range(13):
            t = (j - 6) / 6
            weite = 0.06 + 0.26 * (1 - t * t) ** 0.6
            ringe.append((kopfmitte + oben * t * 0.24 + vorne * weite * 0.5, vorne, X, weite * 0.5 + 0.01, 0.012 * (1 - 0.5 * abs(t))))
        obj = f.loft("Axtblatt", ringe, 10, lambda i, k2, p: eisen, b.hand(-1), oben_zu=True, unten_zu=True, teilung=1)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(eisen.lerp(eisen_hell, weich(0.1, 0.25, (p - kopfmitte).dot(vorne)) * 0.8) * (0.85 + 0.25 * bh.rausch(p, 35.0)), h, 0.5, 0.3))
        _kegel(f, "Dorn", kopfmitte - vorne * 0.03, -vorne, 0.16, 0.03, eisen, b.hand(-1), 8)
        f.kugel("Axtkappe", e, (0.035, 0.035, 0.04), eisen, b.hand(-1), 10, 8)
    b.starr("Streitaxt", "Hand.R", axt)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-8, -18), (-14, -40)), gehen=(24, 36, 14, 0.035, 8, 14)))


# ===========================================================================
# Echsenkrieger (modelliert)
# ===========================================================================
def echsenkrieger(seed=404):
    """Knapp zwei Meter, schlank, türkise Schuppen mit gelbem Bauchpanzer, langer Schwanz,
    Echsenkopf mit Stachelkamm, Speer mit Bronzeblatt und Schild aus einem Schildkrötenpanzer."""
    import bildhauer as bh
    f = Figur("Echsenkrieger", seed)
    r = f.rng
    b = Bau(f, becken=0.98, bauch=1.12, brust=1.32, hals=1.58, kopf=1.66, scheitel=1.88, kopf_y=-0.1,
            schulter=(0.21, 0.02, 1.53), ellbogen=(0.3, 0.05, 1.27), hand=(0.34, 0.0, 1.02), finger=(0.35, -0.03, 0.92),
            huefte=(0.11, 0.02, 0.98), knie=(0.13, -0.08, 0.56), knoechel=(0.13, 0.06, 0.12), zehen=(0.13, -0.12, 0.02))
    b.skelett()
    schuppe, schuppe_dunkel, bauch = farbe("#2E7A6A"), farbe("#163E38"), farbe("#D8C27A")
    zeichnung = farbe("#1E4E6A")
    bronze, stoff = farbe("#B07A3A"), farbe("#8E2A22")
    k = Vector((0, -0.18, 1.74))
    schwanz = [Vector((0, 0.08, 1.0)), Vector((0, 0.3, 0.84)), Vector((0, 0.56, 0.6)), Vector((0, 0.8, 0.38)), Vector((0.02, 1.02, 0.28)),
               Vector((0.06, 1.22, 0.3))]

    formen = [
        ((0, 0.02, 0.98), (0.14, 0.13, 0.1)),
        ((0, 0.0, 1.1), (0.14, 0.12, 0.09)),
        ((0, -0.01, 1.24), (0.17, 0.13, 0.1)),
        ((0, 0.0, 1.38), (0.21, 0.145, 0.11)),
        ((0, 0.01, 1.5), (0.2, 0.13, 0.08)),
    ]
    for s in (1, -1):
        formen += [((0.1 * s, -0.1, 1.42), (0.1, 0.05, 0.08))]
        formen += b.arm_formen(s, (0.075, 0.062, 0.048, 0.054, 0.038), muskel=0.2)
        formen += b.faust_formen(s, 1.15)
        formen += b.bein_formen(s, (0.11, 0.09, 0.065, 0.07, 0.045), muskel=0.25)
        zehen, _ = b.zehen_formen(s, 0.26, 0.12)
        formen += zehen
    formen += bh.glied((0, 0.0, 1.5), (0, -0.07, 1.63), 0.085, 0.07) + bh.glied((0, -0.07, 1.63), k + Vector((0, 0.04, -0.02)), 0.07, 0.065)
    for i in range(len(schwanz) - 1):
        ra = [0.12, 0.1, 0.08, 0.06, 0.035][i]
        rb = [0.1, 0.08, 0.06, 0.035, 0.012][i]
        formen += bh.glied(schwanz[i], schwanz[i + 1], ra, rb)
    formen += [
        (k + Vector((0, 0.03, 0.02)), (0.075, 0.09, 0.068)),
        (k + Vector((0, -0.08, -0.002)), (0.06, 0.1, 0.048)),
        (k + Vector((0, -0.17, -0.008)), (0.045, 0.075, 0.034)),
        (k + Vector((0, -0.08, -0.048)), (0.055, 0.1, 0.028)),
        (k + Vector((0, -0.125, -0.028)), (0.05, 0.11, 0.006), True),          # Maulspalte
        (k + Vector((0.017, -0.235, 0.012)), (0.007, 0.006, 0.005), True),      # Nüstern
        (k + Vector((-0.017, -0.235, 0.012)), (0.007, 0.006, 0.005), True),
    ]
    for s in (1, -1):
        formen += [(k + Vector((0.055 * s, -0.025, 0.036)), (0.032, 0.036, 0.024)),               # Augenwülste
                   (k + Vector((0.064 * s, -0.035, 0.043)), (0.02, 0.02, 0.018), True)]

    def schuppen_versatz(p, n):
        skala = 55.0 if (n.y < -0.35 and abs(p.x) < 0.12 and p.z > 0.9) else 70.0
        z = bh.zellen(p, skala)
        return 0.0022 * weich(0.0, 0.18, z) - 0.001

    def schuppen_farbe(p, n, h):
        bauchseite = weich(-0.25, -0.55, n.y) * weich(0.16, 0.1, abs(p.x)) * (1.0 if p.z > 0.9 and p.y < 0.1 else 0.0)
        c = schuppe.lerp(schuppe_dunkel, weich(-0.1, 0.8, n.z * 0.5 + n.y * 0.5) * 0.6)
        # Querstreifen auf dem Rücken
        if n.y > 0.2 and math.sin(p.z * 32 + bh.rausch(p, 4.0) * 2) > 0.55:
            c = c.lerp(zeichnung, 0.6)
        if bauchseite > 0:
            c = c.lerp(bauch * (0.88 if int(p.z * 28) % 2 else 1.0), bauchseite)
        c = c.lerp(schuppe_dunkel * 0.6, weich(0.12, 0.02, bh.zellen(p, 70.0)) * 0.35)      # Fugen
        return bh.schmutz(c, h, 0.35, 0.25)
    koerper = bh.teil(f, "Schuppen", formen, 0.0065, 20000, schuppen_farbe, versatz=schuppen_versatz, knochen=f.knochen)
    # Der Schwanz hängt am Becken (die automatischen Gewichte verteilen ihn sonst auf die Beine)
    becken_g = koerper.vertex_groups.get("Becken")
    for v in koerper.data.vertices:
        if v.co.y > 0.2 and v.co.z > 0.2:
            for e in list(v.groups):
                koerper.vertex_groups[e.group].remove([v.index])
            becken_g.add([v.index], 1.0, "REPLACE")

    b.augen([k + Vector((0.064 * s, -0.035, 0.043)) for s in (1, -1)], 0.02, farbe("#F2B81C"), schlitz=True, weiss=farbe("#C8B840"),
            blick=Vector((0.4 * 0, -0.6, 0.1)).normalized())
    for s in (1, -1):
        for j in range(6):
            obj = _kegel(f, "Zahn", k + Vector((0.043 * s * (1 - j * 0.1), -0.045 - 0.035 * j, -0.024)), Vector((0, -0.2, -1)), 0.022, 0.0055, farbe("#EDE6D0"), KOPF, 6)
    for j in range(8):
        basis = k + Vector((0, 0.02 + 0.035 * j, 0.075 - 0.018 * j))
        c = farbe("#C8322A") if j % 2 == 0 else farbe("#E8702A")
        obj = _kegel(f, "Kamm", basis, Vector((0, 0.8, 1)), 0.16 - 0.013 * j, 0.017, c, KOPF, 8)
        bh.glatt_einfaerben(obj, lambda p, n, h, c=c, basis=basis: c.lerp(farbe("#F2D04A"), weich(0.06, 0.14, (p - basis).length) * 0.6))
    for s in (1, -1):
        _kegel(f, "Horn", k + Vector((0.05 * s, 0.06, 0.055)), Vector((0.4 * s, 0.8, 0.5)), 0.1, 0.018, farbe("#E4DAC2"), KOPF, 8)
    # Rückenstacheln entlang des Schwanzes
    for j in range(7):
        p = schwanz[1].lerp(schwanz[4], j / 6)
        obj = _kegel(f, "Schwanzstachel", p + Vector((0, 0, 0.085 - 0.012 * j)), Vector((0, 0.6, 1)), 0.08 - 0.007 * j, 0.02, schuppe_dunkel, lambda co: {"Becken": 1.0}, 6)
    for s in (1, -1):
        _, spitzen = b.zehen_formen(s, 0.26, 0.12)
        for sp in spitzen:
            f.straehne("Kralle", [sp + Vector((0, -0.025, 0.005)), sp + Vector((0, -0.05, 0.0)), sp + Vector((0, -0.06, -0.02))], 0.012, 0.002, farbe("#2A2620"),
                       b.fuss(s), 6, 0.0, 1.0, glatt=True)
    # Lendentuch mit Bronzegürtel, Zahnkette, Armreifen
    my, rx, ry = bh.umfang(koerper, 1.0, 0.03, nur=lambda p: p.y < 0.2 and abs(p.x) < 0.24)
    obj = f.loft("Lendentuch", [(Vector((0, my, 1.0)), X, Y, rx + 0.015, ry + 0.015), (Vector((0, my - 0.01, 0.84)), X, Y, rx + 0.045, ry + 0.035, _falten(r, 0.05)),
                                (Vector((0, my - 0.01, 0.72)), X, Y, rx + 0.06, ry + 0.045, _zacken(r, 0.2, 11))], 40, lambda i, k2, p: stoff, b.rock, teilung=4, glatt=True)
    _veredeln(obj, stoff, 18.0, 0.3)
    bh.gewichte_uebertragen(obj, koerper, lambda kn: kn in ("Becken", "Bauch") or kn.startswith("Oberschenkel"))
    obj = f.loft("Guertel", [(Vector((0, my, 0.99)), X, Y, rx + 0.02, ry + 0.02), (Vector((0, my, 1.04)), X, Y, rx + 0.022, ry + 0.022)], 36,
                 lambda i, k2, p: bronze, b.rumpf, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: bronze.lerp(farbe("#4A8A6A"), weich(0.2, 0.6, bh.rausch(p, 30.0)) * 0.5))
    for j in range(9):
        w = math.pi * (0.2 + 0.6 * j / 8)
        p = Vector((math.cos(w) * 0.13, -0.11 - math.sin(w) * 0.04, 1.5 - math.sin(w) * 0.1))
        _kegel(f, "Kettenzahn", p, Vector((0, -0.3, -1)), 0.04, 0.01, farbe("#E8DDB8"), b.rumpf, 6)
    for s in (1, -1):
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        obj = f.loft("Armreif", [(e.lerp(h, 0.55), q1, q2, 0.068, 0.068), (e.lerp(h, 0.72), q1, q2, 0.064, 0.064), (e.lerp(h, 0.9), q1, q2, 0.056, 0.056)], 20,
                     lambda i, k2, p: bronze, b.arm(s), glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bronze.lerp(farbe("#4A8A6A"), weich(0.2, 0.6, bh.rausch(p, 30.0)) * 0.5))

    def speer():
        g = b.griff(-1)
        oben = Vector((0.0, -0.15, 1.0)).normalized()
        obj = _strecke(f, "Speerschaft", g - oben * 0.7, g + oben * 1.25, 0.018, 0.017, farbe("#6A4A2A"), b.hand(-1), 12)
        bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#6A4A2A") * (0.8 + 0.3 * bh.rausch(Vector((p.x * 60, p.y * 60, p.z * 5)), 1.0)))
        spitze = g + oben * 1.25
        obj = f.loft("Speerblatt", [(spitze, oben, X, 0.02, 0.008), (spitze + oben * 0.1, oben, X, 0.05, 0.01), (spitze + oben * 0.26, oben, X, 0.002, 0.002)], 10,
                     lambda i, k2, p: bronze, b.hand(-1), oben_zu=True, unten_zu=True, teilung=3)
        bh.glatt_einfaerben(obj, lambda p, n, h: bronze.lerp(farbe("#F0C878"), weich(0.0, -0.4, h)))
        for j, c in enumerate((farbe("#C8322A"), farbe("#F2EEE0"), farbe("#2E6AA8"))):
            a = spitze - oben * 0.02
            b.f.straehne("Speerfeder", [a, a + Vector((0.03 * (j - 1), 0.02, -0.08)), a + Vector((0.05 * (j - 1), 0.03, -0.16))], 0.012, 0.002, c,
                         b.hand(-1), 6, 0.0, 0.3, glatt=True)
        f.loft("Speerwicklung", [(spitze - oben * 0.05, X, Y, 0.022, 0.022), (spitze + oben * 0.01, X, Y, 0.022, 0.022)], 12, lambda i, k2, p: stoff, b.hand(-1), glatt=True)
    b.starr("Speer", "Hand.R", speer)

    def schild():
        h = b.p("hand", 1).lerp(b.p("ellbogen", 1), 0.4) + Vector((0.07, -0.02, 0))
        panzer, rand = farbe("#5A6A3A"), farbe("#3A4424")
        arm = lambda co: {"Unterarm.L": 1.0}
        form = bh.ball_mesh("Schildpanzer", [(h + Vector((0.02, 0, 0)), (0.07, 0.3, 0.34))], 0.02)

        def platten(p, n):
            return 0.006 * weich(0.0, 0.2, bh.zellen(p, 11.0)) + 0.004 * bh.rausch(p, 25.0)
        bh.modellieren(form, 0.008, 4000, platten, glaetten=3)
        # innen flach abschneiden: alles, was näher als 1 cm am Arm ist, auf eine Ebene
        for v in form.data.vertices:
            v.co.x = max(v.co.x, h.x + 0.005)
        bh.einfaerben(form, lambda p, n, hh: bh.schmutz(panzer.lerp(farbe("#8A9A4A"), weich(0.1, 0.25, bh.zellen(p, 11.0)) * 0.5).lerp(rand, weich(0.05, 0.0, bh.zellen(p, 11.0))), hh, 0.5, 0.3))
        f._gewichten(form, arm)
        bh.aufnehmen(f, form)
    b.starr("Schild", "Unterarm.L", schild)
    return f.fertig(_animationen(_angriff_stoss, arme_ruhe=((-35, -50), (-10, -35)), gehen=(28, 44, 14, 0.03, 12, 16)))


# ===========================================================================
# Pilzling (modelliert)
# ===========================================================================
def pilzling(seed=405):
    """Ein Meter groß: dicker, leicht bauchiger Stiel als Körper mit Fasern, riesiger roter Hut mit
    erhabenen weißen Tupfen und feinen Lamellen, große Glanzaugen, rosige Wangen, Stummelarme,
    Wurzelfüße; auf der Schulter wachsen kleine Pilze, am Fuß Moos."""
    import bildhauer as bh
    f = Figur("Pilzling", seed)
    r = f.rng
    b = Bau(f, becken=0.3, bauch=0.38, brust=0.5, hals=0.6, kopf=0.62, scheitel=0.95, kopf_y=0.0,
            schulter=(0.14, 0.0, 0.5), ellbogen=(0.2, 0.02, 0.39), hand=(0.23, -0.01, 0.29), finger=(0.24, -0.03, 0.23),
            huefte=(0.08, 0.0, 0.3), knie=(0.09, -0.01, 0.17), knoechel=(0.09, 0.01, 0.05), zehen=(0.09, -0.08, 0.02))
    b.skelett()
    stiel, stiel_dunkel = farbe("#EDE2C8"), farbe("#BFAE8A")
    hut, hut_dunkel, tupfen = farbe("#C8322A"), farbe("#7E1A16"), farbe("#F6F0E4")
    lamelle, moos = farbe("#E6D2B0"), farbe("#4E6A2E")
    k = Vector((0, -0.125, 0.6))

    formen = [((0, 0.0, 0.26), (0.12, 0.11, 0.08)), ((0, -0.012, 0.36), (0.152, 0.142, 0.1)), ((0, -0.012, 0.46), (0.15, 0.14, 0.09)),
              ((0, -0.005, 0.56), (0.135, 0.125, 0.08)), ((0, 0.0, 0.66), (0.115, 0.105, 0.06)), ((0, 0.0, 0.72), (0.1, 0.095, 0.04))]
    for s in (1, -1):
        formen += b.arm_formen(s, (0.045, 0.04, 0.036, 0.036, 0.032), muskel=0.1)
        formen += [(b.griff(s), (0.04, 0.042, 0.046)), (b.griff(s) + Vector((0.022 * s, -0.03, 0.022)), (0.018, 0.018, 0.018))]
        formen += b.bein_formen(s, (0.065, 0.058, 0.055, 0.053, 0.048), muskel=0.05)
        a = b.p("knoechel", s)
        for w in (-0.6, 0.0, 0.6, 3.0):
            richtung = Vector((math.sin(w) * 0.7, -math.cos(w), -0.3)).normalized()
            basis = Vector((a.x, a.y, 0.05))
            formen += bh.glied(basis, basis + richtung * 0.11 + Vector((0, 0, -0.025)), 0.035, 0.014)
    formen += [(k + Vector((0.075 * s, 0.01, -0.04)), (0.03, 0.015, 0.02)) for s in (1, -1)]     # Pausbacken
    formen += [(k + Vector((0, 0.0, -0.05)), (0.02, 0.012, 0.008), True)]                          # kleiner Mund

    def stiel_versatz(p, n):
        faser = noise.noise(Vector((p.x * 55, p.y * 55, p.z * 5)))
        return 0.0015 * faser + 0.0015 * bh.rausch(p, 18.0)

    def stiel_farbe(p, n, h):
        c = stiel.lerp(stiel_dunkel, weich(0.45, 0.15, p.z) * 0.6) * (0.94 + 0.08 * noise.noise(Vector((p.x * 55, p.y * 55, p.z * 5))))
        c = c.lerp(farbe("#E89A8A"), weich(0.03, 0.0, min((p - (k + Vector((0.075 * s, -0.01, -0.04)))).length for s in (1, -1)) - 0.005) * 0.7)
        c = c.lerp(moos, weich(0.08, 0.02, p.z) * weich(-0.2, 0.3, bh.rausch(p, 12.0)))
        return bh.schmutz(c, h, 0.45, 0.1)
    koerper = bh.teil(f, "Stiel", formen, 0.005, 14000, stiel_farbe, versatz=stiel_versatz, knochen=f.knochen)

    # Manschette unter dem Hut, weich gefaltet
    obj = f.loft("Manschette", [(Vector((0, -0.005, 0.58)), X, Y, 0.13, 0.12), (Vector((0, -0.005, 0.55)), X, Y, 0.165, 0.155, _falten(r, 0.08)),
                                (Vector((0, -0.005, 0.51)), X, Y, 0.172, 0.162, _zacken(r, 0.08, 14))], 48, lambda i, k2, p: stiel, b.rumpf, teilung=4, glatt=True)
    _veredeln(obj, stiel * 1.02, 40.0, 0.12, 0.6)
    # Der Hut: Kuppel mit welligem Rand, erhabene Tupfen, feine Lamellen unten
    tupfen_orte = []
    while len(tupfen_orte) < 16:
        w, d = r.uniform(0, math.tau), r.uniform(0.02, 0.27)
        q = Vector((math.cos(w) * d, math.sin(w) * d, 0))
        if all((q - t).length > 0.075 for t, _ in tupfen_orte):
            tupfen_orte.append((q, r.uniform(0.025, 0.045)))
    hut_formen = [((0, 0.01, 0.81), (0.33, 0.33, 0.11)), ((0, 0.01, 0.87), (0.26, 0.26, 0.1)), ((0, 0.01, 0.92), (0.15, 0.15, 0.06))]

    def tupfen_anteil(p):
        q = Vector((p.x, p.y - 0.01, 0))
        return max((weich(g + 0.006, g - 0.004, (q - t).length) for t, g in tupfen_orte), default=0.0)

    def hut_versatz(p, n):
        oben = weich(-0.2, 0.3, n.z)
        return 0.004 * tupfen_anteil(p) * oben + 0.0015 * bh.rausch(p, 15.0)

    def hut_farbe(p, n, h):
        c = hut.lerp(hut_dunkel, weich(0.9, 0.78, p.z) * 0.55) * (0.93 + 0.1 * bh.rausch(p, 8.0))
        if n.z < -0.2:
            return lamelle * (0.8 if int(math.atan2(p.y - 0.01, p.x) * 40) % 2 else 1.0)
        c = c.lerp(tupfen, tupfen_anteil(p) * weich(-0.1, 0.2, n.z))
        return bh.schmutz(c, h, 0.35, 0.25)
    hutobj = bh.ball_mesh("Hut", hut_formen, 0.012)
    # unten flach und hohl: unter den Rand drücken
    bh.modellieren(hutobj, 0.006, 9000, hut_versatz, glaetten=4)
    for v in hutobj.data.vertices:
        if v.co.z < 0.76:
            v.co.z = 0.76 - (0.76 - v.co.z) * 0.25
    bh.einfaerben(hutobj, hut_farbe)
    f._gewichten(hutobj, KOPF)
    bh.aufnehmen(f, hutobj)
    # Gesicht: große Glanzaugen mit Lidern
    for s in (1, -1):
        auge = k + Vector((0.045 * s, 0.0, 0.005))
        b.f.kugel("Auge", auge, (0.028, 0.02, 0.034), farbe("#1A1210"), KOPF, 18, 12)
        b.f.kugel("Glanz", auge + Vector((0.008 * s, -0.018, 0.012)), (0.008, 0.004, 0.008), farbe("#FFFFFF"), KOPF, 10, 6)
        b.f.kugel("Glanz klein", auge + Vector((-0.006 * s, -0.018, -0.01)), (0.004, 0.003, 0.004), farbe("#FFFFFF"), KOPF, 8, 4)
    # Kleine Pilze auf der Schulter und am Fuß
    for ort, g_ in ((Vector((0.11, 0.03, 0.55)), 1.0), (Vector((0.14, 0.02, 0.5)), 0.7), (Vector((-0.12, 0.05, 0.3)), 0.8), (Vector((-0.1, 0.06, 0.33)), 0.55)):
        _strecke(f, "Pilzstiel", ort, ort + Vector((0.02, 0, 0.05 * g_)), 0.008 * g_, 0.006 * g_, stiel, b.rumpf, 8)
        b.f.kugel("Pilzhut", ort + Vector((0.02, 0, 0.055 * g_)), (0.025 * g_, 0.025 * g_, 0.012 * g_), farbe("#D8A040"), b.rumpf, 14, 8)
    return f.fertig(_animationen(_angriff_stampfen, arme_ruhe=((-5, -20), (-5, -20)), gehen=(34, 40, 30, 0.035, 4, 16)))


# ===========================================================================
# Waldschrat (modelliert)
# ===========================================================================
def waldschrat(seed=406):
    """Knapp drei Meter hoher, wandelnder Baum: ein durchgehender Rindenkörper mit tiefen
    Furchen und Astknoten, Astarme mit Zweigfingern, Wurzelbeine, ein Gesicht in der Rinde mit
    glühenden Augen und Moosbart, eine Krone aus Ästen und Laub, Moos und Pilze auf den Schultern."""
    import bildhauer as bh
    f = Figur("Waldschrat", seed)
    r = f.rng
    b = Bau(f, becken=1.3, bauch=1.55, brust=1.85, hals=2.22, kopf=2.26, scheitel=2.66, kopf_y=-0.04,
            schulter=(0.44, 0.0, 2.12), ellbogen=(0.62, 0.08, 1.7), hand=(0.72, 0.0, 1.3), finger=(0.75, -0.05, 1.1),
            huefte=(0.2, 0.0, 1.3), knie=(0.24, -0.05, 0.74), knoechel=(0.25, 0.03, 0.18), zehen=(0.26, -0.22, 0.04))
    b.skelett()
    rinde, rinde_dunkel, rinde_hell = farbe("#5E4631"), farbe("#2A1D14"), farbe("#8C7358")
    moos, moos_hell, flechte = farbe("#3F6A22"), farbe("#78A035"), farbe("#A8B48A")
    glut = farbe("#FFD24A")

    def furchen(p, n):
        # Längliche Risse: gestrecktes Rauschen, dessen Nulllinien zu tiefen Furchen werden
        q = Vector((p.x * 7.0, p.y * 7.0, p.z * 1.3))
        riss = 1.0 - abs(noise.noise(q))
        tief = max(0.0, (riss - 0.78) / 0.22) ** 1.5
        beule = bh.rausch(p, 3.0) * 0.012 + bh.rausch(p, 11.0, 2) * 0.004
        return -0.034 * tief + beule

    def rinden_farbe(p, n, h):
        q = Vector((p.x * 7.0, p.y * 7.0, p.z * 1.3))
        riss = 1.0 - abs(noise.noise(q))
        c = rinde.lerp(rinde_hell, max(0.0, bh.rausch(p, 4.0)) * 0.6)
        c = c.lerp(rinde_dunkel, weich(0.72, 0.95, riss))
        # Moos oben auf Schultern, Armen, Wurzeln; Flechten als helle Flecken
        moosig = weich(0.45, 0.8, n.z) * weich(-0.1, 0.35, bh.rausch(p, 2.5))
        c = c.lerp(moos.lerp(moos_hell, weich(0.0, 0.6, bh.rausch(p, 9.0)) * 0.6), moosig * 0.85)
        c = c.lerp(flechte, weich(0.3, 0.55, bh.rausch(p + Vector((5, 5, 5)), 5.0)) * 0.25)
        return bh.schmutz(c, h, 0.55, 0.2)

    formen = [
        ((0, 0.0, 1.22), (0.3, 0.26, 0.24)),
        ((0, -0.02, 1.5), (0.32, 0.27, 0.26)),
        ((0, -0.03, 1.8), (0.38, 0.29, 0.26)),
        ((0, -0.02, 2.04), (0.43, 0.3, 0.2)),
        ((0, -0.03, 2.3), (0.25, 0.23, 0.22)),       # Kopf: aus dem Stamm gewachsen
        ((0, -0.02, 2.5), (0.19, 0.18, 0.16)),
        ((0, -0.13, 2.3), (0.2, 0.15, 0.2)),        # Gesicht nach vorne
        ((0, -0.26, 2.41), (0.18, 0.07, 0.05)),     # Brauenwulst
        ((0, -0.29, 2.3), (0.045, 0.08, 0.09)),     # Nase, knorrig
        ((0, -0.33, 2.25), (0.035, 0.05, 0.035)),
        ((0.078, -0.27, 2.34), (0.05, 0.05, 0.042), True),     # Augenhöhlen
        ((-0.078, -0.27, 2.34), (0.05, 0.05, 0.042), True),
        ((0, -0.27, 2.17), (0.085, 0.05, 0.035), True),        # Maul
    ]
    for s in (1, -1):
        sh, el, ha = b.p("schulter", s), b.p("ellbogen", s), b.p("hand", s)
        formen += [(sh + Vector((0, 0, 0.02)), (0.19, 0.18, 0.17))]
        formen += bh.glied(sh, el, 0.15, 0.12, muskel=0.12)
        formen += bh.glied(el, ha, 0.12, 0.085, muskel=0.08, lage=0.3)
        formen += [(el + Vector((0.03 * s, 0.03, 0)), (0.11, 0.11, 0.1))]                  # Astknoten am Ellbogen
        hu, kn, ks = b.p("huefte", s), b.p("knie", s), b.p("knoechel", s)
        formen += bh.glied(hu + Vector((0, 0, 0.06)), kn, 0.21, 0.16, muskel=0.08)
        formen += bh.glied(kn, ks + Vector((0, 0, 0.05)), 0.16, 0.14, muskel=0.05)
        formen += [(kn + Vector((0, -0.04, 0)), (0.16, 0.14, 0.14))]
        # Wurzelfüße: fünf Wurzeln, die sich in den Boden krallen
        for w in (-1.0, -0.4, 0.2, 0.8, 2.9):
            richtung = Vector((math.sin(w) * 0.9, -math.cos(w), 0)).normalized()
            fuss = Vector((ks.x, ks.y, 0.18))
            mitte = fuss + richtung * 0.2 + Vector((0, 0, -0.08))
            ende = fuss + richtung * 0.42 + Vector((0, 0, -0.16))
            formen += bh.glied(fuss, mitte, 0.1, 0.07) + bh.glied(mitte, ende, 0.07, 0.035)
    for j in range(6):
        w = r.uniform(0, math.tau)
        formen.append(((math.cos(w) * 0.3, math.sin(w) * 0.26, r.uniform(1.3, 1.95)), (0.08, 0.08, 0.07)))       # Astknoten
    koerper = bh.teil(f, "Rinde", formen, 0.014, 16000, rinden_farbe, versatz=furchen, knochen=f.knochen)

    # Glühende Augen tief in den Höhlen, Glutpunkte im Maul
    for s in (1, -1):
        b.f.kugel("Glutauge", (0.078 * s, -0.25, 2.34), (0.03, 0.02, 0.025), glut, KOPF, 16, 10)
        b.f.kugel("Glutkern", (0.078 * s, -0.266, 2.342), (0.014, 0.01, 0.012), farbe("#FFF6C8"), KOPF, 10, 6)
    b.f.kugel("Maulglut", (0, -0.245, 2.17), (0.06, 0.02, 0.022), farbe("#6A3008"), KOPF, 12, 6)
    # Moosbart: weiche, hängende Strähnen
    for j in range(13):
        x = -0.12 + 0.02 * j
        basis = Vector((x, -0.31 + abs(x) * 0.6, 2.13 - abs(x) * 0.3))
        laenge = 0.28 - abs(x) * 0.8 + r.uniform(-0.03, 0.05)
        punkte = [basis, basis + Vector((x * 0.1, -0.03, -laenge * 0.5)), basis + Vector((x * 0.25, -0.01, -laenge))]
        obj = f.straehne("Moosbart", punkte, 0.026, 0.004, lambda i, k, p: moos, lambda co: _mischen(("Kopf", weich(1.9, 2.1, co.z)), ("Brust", 1 - weich(1.9, 2.1, co.z))),
                         8, r.uniform(0, 1), 0.7, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: moos.lerp(moos_hell, weich(1.95, 2.12, p.z) * 0.7) * (0.9 + 0.2 * bh.rausch(p, 20.0)))

    # Zweigfinger: knorrige, gebogene Zweige an jeder Hand
    for s in (1, -1):
        ha, fi = b.p("hand", s), b.p("finger", s)
        achse = (fi - ha).normalized()
        for j, (w, l) in enumerate(((-0.7, 0.3), (-0.25, 0.36), (0.2, 0.34), (0.65, 0.27), (2.6, 0.22))):
            seitlich = Vector((math.sin(w) * s, -math.cos(w) * 0.8, 0.0))
            a = ha + seitlich * 0.06 + achse * 0.04
            mitte = a + achse * l * 0.5 + seitlich * 0.07 + Vector((0, -0.03, 0))
            spitze = a + achse * l + seitlich * 0.03 + Vector((0, -0.1, 0.02))
            obj = f.straehne("Zweigfinger", [a, mitte, spitze], 0.04, 0.008, rinde, b.hand(s), 8, 0.4, 1.0, teilung=3, glatt=True)
            bh.glatt_einfaerben(obj, lambda p, n, h: rinde.lerp(rinde_dunkel, 0.3 + 0.3 * bh.rausch(p, 25.0)))
            if j < 2:
                zweig = mitte + seitlich * 0.02
                obj = f.straehne("Zweig", [zweig, zweig + seitlich * 0.07 + Vector((0, 0, 0.06)), zweig + seitlich * 0.1 + Vector((0, 0, 0.13))], 0.014, 0.003,
                                 rinde, b.hand(s), 6, 0.0, 1.0, glatt=True)
                bh.glatt_einfaerben(obj, lambda p, n, h: rinde)
                b.f.kugel("Blatt", zweig + seitlich * 0.1 + Vector((0, 0, 0.15)), (0.03, 0.012, 0.045), moos_hell, b.hand(s), 8, 6)

    # Krone: knorrige Äste aus Kopf und Schultern, daran weiche Laubwolken
    laub, laub_hell, laub_dunkel = farbe("#4E8A2E"), farbe("#9CC24A"), farbe("#28501A")
    aeste = [(Vector((0, 0.0, 2.58)), Vector((0.1, 0.15, 1)), 0.55), (Vector((0.1, 0.05, 2.5)), Vector((0.9, 0.2, 1)), 0.52),
             (Vector((-0.1, 0.05, 2.5)), Vector((-0.9, 0.3, 1)), 0.5), (Vector((0.34, 0.06, 2.16)), Vector((1, 0.3, 0.7)), 0.45),
             (Vector((-0.34, 0.06, 2.16)), Vector((-1, 0.4, 0.65)), 0.42), (Vector((0, 0.2, 2.42)), Vector((0, 1, 0.7)), 0.46)]
    laub_formen = []
    for ort, richtung, laenge in aeste:
        richtung = richtung.normalized()
        spitze = ort + richtung * laenge
        mitte = ort.lerp(spitze, 0.5) + Vector((r.uniform(-0.05, 0.05), r.uniform(-0.05, 0.05), 0.05))
        gewicht = KOPF if ort.z > 2.3 else b.rumpf
        obj = f.straehne("Ast", [ort, mitte, spitze], 0.07, 0.025, rinde, gewicht, 10, 0.3, 1.0, teilung=3, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: rinde.lerp(rinde_dunkel, 0.2 + 0.3 * bh.rausch(p, 18.0)))
        for j in range(6):
            o = spitze + Vector((r.uniform(-0.16, 0.16), r.uniform(-0.16, 0.16), r.uniform(-0.02, 0.16)))
            g_ = r.uniform(0.12, 0.19)
            laub_formen.append((o, (g_, g_, g_ * 0.8)))

    def laub_farbe(p, n, h):
        c = laub_dunkel.lerp(laub, weich(-0.6, 0.3, n.z)).lerp(laub_hell, weich(0.4, 1.0, n.z) * weich(-0.1, 0.5, bh.rausch(p, 6.0)))
        return bh.schmutz(c, h, 0.5, 0.25)

    def laub_beulen(p, n):
        return 0.035 * bh.rausch(p, 7.0) + 0.012 * bh.rausch(p, 19.0, 2)
    laubwerk = bh.ball_mesh("Laub", laub_formen, 0.03)
    bh.modellieren(laubwerk, 0.022, 7000, laub_beulen, glaetten=2)
    bh.einfaerben(laubwerk, laub_farbe)
    f._gewichten(laubwerk, lambda co: {"Kopf": 1.0} if co.z > 2.45 else b.rumpf(co))
    bh.aufnehmen(f, laubwerk)
    # Pilze auf der Schulter
    for j in range(4):
        o = Vector((0.36 + 0.045 * j, 0.1 - 0.05 * j, 2.2 - 0.04 * j))
        _strecke(f, "Pilzstiel", o - Vector((0, 0, 0.03)), o, 0.012, 0.01, farbe("#E8DCC0"), b.rumpf, 8)
        b.f.kugel("Schulterpilz", o, (0.055, 0.055, 0.02), farbe("#D89A3A"), b.rumpf, 14, 8)
    return f.fertig(_animationen(_angriff_stampfen, arme_ruhe=((-6, -15), (-6, -15)), gehen=(18, 26, 12, 0.03, 6, 12)))


# ===========================================================================
# Minotaurus (modelliert)
# ===========================================================================
def minotaurus(seed=407):
    """Zweieinhalb Meter: Stierkopf mit Goldring in der Nase und mächtigen Hörnern, bulliger
    Oberkörper mit Nackenbuckel, zottiges Fell an Brust und Beinen, Hufe, Lederschurz mit Nieten,
    Doppelaxt."""
    import bildhauer as bh
    f = Figur("Minotaurus", seed)
    r = f.rng
    b = Bau(f, becken=1.18, bauch=1.36, brust=1.6, hals=1.94, kopf=1.98, scheitel=2.3, kopf_y=-0.12,
            schulter=(0.36, 0.02, 1.86), ellbogen=(0.49, 0.06, 1.5), hand=(0.54, 0.0, 1.18), finger=(0.55, -0.03, 1.06),
            huefte=(0.16, 0.0, 1.18), knie=(0.19, -0.08, 0.66), knoechel=(0.19, 0.1, 0.16), zehen=(0.19, -0.06, 0.02))
    b.skelett()
    fell, fell_dunkel, fell_hell = farbe("#6B4226"), farbe("#3A2214"), farbe("#9A7050")
    maul = farbe("#2E221A")
    horn, horn_dunkel = farbe("#EDE3CB"), farbe("#7A6A4E")
    leder, eisen, gold = farbe("#4A2E1C"), farbe("#6A6E76"), farbe("#D8AE4A")
    k = Vector((0, -0.16, 2.14))

    formen = [
        ((0, 0.0, 1.16), (0.22, 0.18, 0.11)),
        ((0, -0.02, 1.3), (0.24, 0.19, 0.12)),
        ((0, -0.03, 1.46), (0.3, 0.22, 0.13)),
        ((0, -0.03, 1.62), (0.37, 0.25, 0.13)),
        ((0, 0.0, 1.76), (0.39, 0.24, 0.11)),
        ((0, 0.08, 1.88), (0.28, 0.18, 0.12)),          # Nackenbuckel
        ((0, 0.02, 1.84), (0.33, 0.18, 0.08)),
    ]
    for s in (1, -1):
        formen += [((0.15 * s, -0.22, 1.7), (0.16, 0.07, 0.11)),          # Brustmuskeln
                   ((0.18 * s, 0.1, 1.9), (0.14, 0.12, 0.08)),            # Trapez
                   ((0.24 * s, 0.1, 1.62), (0.11, 0.08, 0.16))]           # Rücken
        for i in range(3):
            formen.append(((0.05 * s, -0.215, 1.5 - 0.075 * i), (0.05, 0.03, 0.034)))
        formen += b.arm_formen(s, (0.15, 0.13, 0.1, 0.11, 0.08), muskel=0.25)
        formen += [(b.p("schulter", s) + Vector((0.03 * s, -0.02, -0.05)), (0.15, 0.14, 0.15))]
        formen += b.faust_formen(s, 1.6)
        # Stierbeine: kräftige Keulen, schmale Fesseln
        formen += b.bein_formen(s, (0.18, 0.15, 0.11, 0.1, 0.065), muskel=0.25)
    formen += bh.glied((0, 0.04, 1.86), k + Vector((0, 0.06, -0.12)), 0.18, 0.14)
    formen += [
        (k + Vector((0, 0.04, 0.03)), (0.14, 0.14, 0.13)),               # Stirn und Schädel
        (k + Vector((0, -0.1, -0.03)), (0.1, 0.12, 0.09)),               # Schnauze
        (k + Vector((0, -0.2, -0.065)), (0.092, 0.075, 0.075)),          # Maul
        (k + Vector((0, -0.14, -0.13)), (0.07, 0.08, 0.035)),            # Unterkiefer
        (k + Vector((0, 0.06, -0.12)), (0.15, 0.12, 0.12)),              # Wamme
        (k + Vector((0.032, -0.275, -0.055)), (0.016, 0.01, 0.013), True),     # Nüstern
        (k + Vector((-0.032, -0.275, -0.055)), (0.016, 0.01, 0.013), True),
        (k + Vector((0, -0.23, -0.115)), (0.06, 0.03, 0.006), True),     # Maulspalte
    ]
    for s in (1, -1):
        auge = k + Vector((0.082 * s, -0.075, 0.035))
        formen += [(auge, (0.03, 0.022, 0.024), True), (auge + Vector((0, -0.005, 0.02)), (0.034, 0.022, 0.012)),
                   (k + Vector((0.12 * s, 0.03, 0.09)), (0.05, 0.05, 0.045))]                           # Hornansatz

    def zotteln(p, n):
        # Fell: Strähnen nach unten gekämmt, an Brust, Unterarmen und Beinen länger
        straehne = noise.noise(Vector((p.x * 90, p.y * 90, p.z * 14)))
        lang = weich(1.45, 1.3, p.z) * weich(0.1, 0.4, abs(p.x) + max(0.0, -p.y) * 0.3) + weich(0.3, 0.6, p.z) * weich(0.9, 0.7, p.z)
        return 0.0025 * straehne + 0.004 * lang * max(0.0, straehne) + 0.001 * bh.rausch(p, 30.0)

    def fell_farbe(p, n, h):
        c = fell.lerp(fell_dunkel, weich(0.1, 0.9, n.y) * 0.4).lerp(fell_hell, weich(-0.3, -0.8, n.y) * weich(1.4, 1.75, p.z) * 0.5)
        c = c.lerp(fell_dunkel, weich(0.9, 0.3, p.z) * 0.6)                                # dunkle Beine
        c = c * (0.9 + 0.14 * noise.noise(Vector((p.x * 90, p.y * 90, p.z * 14))) + 0.08 * bh.rausch(p, 5.0))
        if p.y < k.y - 0.17 and p.z > 1.95:
            c = maul.lerp(farbe("#5A4034"), weich(-0.4, 0.4, n.z) * 0.4)                   # dunkles Maul
        c = c.lerp(fell_hell * 1.1, weich(0.05, 0.0, (Vector((abs(p.x), p.y, p.z)) - (k + Vector((0.0, -0.1, 0.14)))).length - 0.02) * 0.6)   # Blesse
        return bh.schmutz(c, h, 0.5, 0.15)
    koerper = bh.teil(f, "Fell", formen, 0.0085, 24000, fell_farbe, versatz=zotteln, knochen=f.knochen)

    for s in (1, -1):
        b.augen([k + Vector((0.082 * s, -0.078, 0.035))], 0.02, farbe("#B01E14"), blick=Vector((0.5 * s, -1, 0.1)).normalized())
    for s in (1, -1):
        wurzel = k + Vector((0.13 * s, 0.03, 0.1))
        punkte = [wurzel, wurzel + Vector((0.12 * s, 0.0, 0.03)), wurzel + Vector((0.24 * s, -0.06, 0.13)), wurzel + Vector((0.28 * s, -0.15, 0.28))]
        obj = f.straehne("Horn", punkte, 0.055, 0.004, horn, KOPF, 16, 0.0, 1.0, teilung=5, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h, w=wurzel: horn_dunkel.lerp(horn, weich(0.05, 0.25, (p - w).length)) * (0.9 + 0.12 * math.sin((p - w).length * 160)))
        ohr = k + Vector((0.15 * s, 0.06, 0.02))
        ringe = [(ohr + Vector((0.07 * s * t, 0.02 * t, -0.03 * t)), Vector((0, 0.3, 1)).normalized(), Vector((0, 1, -0.3)).normalized(),
                  0.04 * (1 - t) ** 0.6 + 0.006, 0.012) for t in (0.0, 0.5, 1.0)]
        obj = f.loft("Ohr", ringe, 14, lambda i, k2, p: fell, KOPF, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: fell.lerp(farbe("#C8907A"), weich(0.2, -0.6, n.y * s) * 0.5))
    nring = k + Vector((0, -0.285, -0.1))
    f.loft("Nasenring", [(nring, X, Z, 0.035, 0.035), (nring + Vector((0, 0.005, 0)), X, Z, 0.026, 0.026)], 20, lambda i, k2, p: gold, KOPF, glatt=True)
    for j in range(7):
        basis = bh.auf_haut(koerper, k + Vector((-0.06 + 0.02 * j + r.uniform(-0.005, 0.005), -0.1, 0.16)), -0.005)
        f.straehne("Locke", [basis, basis + Vector((r.uniform(-0.01, 0.01), -0.035, 0.01)), basis + Vector((r.uniform(-0.02, 0.02), -0.055, -0.035))],
                   0.02, 0.003, fell_dunkel, KOPF, 7, 0.6, 1.0, teilung=3, glatt=True)
    # Lederschurz, Gürtel, Nieten, Manschetten, Kette, Hufe
    my, rx, ry = bh.umfang(koerper, 1.18, 0.03, nur=lambda p: abs(p.x) < 0.34)
    obj = f.loft("Lederschurz", [(Vector((0, my, 1.16)), X, Y, rx + 0.02, ry + 0.02), (Vector((0, my - 0.01, 0.98)), X, Y, rx + 0.05, ry + 0.04, _falten(r, 0.05)),
                                 (Vector((0, my - 0.01, 0.8)), X, Y, rx + 0.07, ry + 0.05, _zacken(r, 0.12, 8))], 44, lambda i, k2, p: leder, b.rock, teilung=4, glatt=True)
    _veredeln(obj, leder, 18.0, 0.35)
    bh.gewichte_uebertragen(obj, koerper, lambda kn: kn in ("Becken", "Bauch") or kn.startswith("Oberschenkel"))
    obj = f.loft("Guertel", [(Vector((0, my, 1.13)), X, Y, rx + 0.03, ry + 0.03), (Vector((0, my, 1.24)), X, Y, rx + 0.035, ry + 0.035)], 44,
                 lambda i, k2, p: leder * 0.7, b.rumpf, glatt=True)
    _veredeln(obj, leder * 0.7, 30.0)
    b.f.kugel("Schnalle", (0, my - ry - 0.045, 1.185), (0.07, 0.022, 0.05), gold, b.rumpf, 16, 10)
    for j in range(12):
        w = math.pi * (0.1 + 0.8 * j / 11)
        b.f.kugel("Niete", (math.cos(w) * (rx + 0.055), my - math.sin(w) * (ry + 0.045), 0.95), (0.012, 0.012, 0.012), eisen, b.rock, 8, 6)
    for s in (1, -1):
        e, h = b.p("ellbogen", s), b.p("hand", s)
        q1, q2 = _achsen(e, h)
        obj = f.loft("Manschette", [(e.lerp(h, 0.6), q1, q2, 0.125, 0.125), (e.lerp(h, 0.78), q1, q2, 0.12, 0.12), (e.lerp(h, 0.95), q1, q2, 0.105, 0.105)], 24,
                     lambda i, k2, p: eisen, b.arm(s), glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(eisen * (0.8 + 0.35 * bh.rausch(p, 30.0)), h, 0.6, 0.4))
        a = b.p("knoechel", s)
        huf = [(Vector((a.x, a.y - 0.03, 0.0)), X, Y, 0.085, 0.1), (Vector((a.x, a.y - 0.03, 0.06)), X, Y, 0.08, 0.092), (Vector((a.x, a.y - 0.02, 0.13)), X, Y, 0.068, 0.075)]
        obj = f.loft("Huf", huf, 20, lambda i, k2, p: farbe("#2A2420"), b.fuss(s), oben_zu=True, unten_zu=True, teilung=2, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#2A2420") * (0.8 + 0.4 * math.sin(p.z * 120) ** 2))
        b.f.kiste("Hufspalt", (a.x, a.y - 0.12, 0.05), (0.008, 0.03, 0.1), farbe("#141010"), b.fuss(s))
    so, eo = b.p("schulter", 1), b.p("ellbogen", 1)
    for j in range(7):
        p = so.lerp(eo, 0.3 + 0.08 * j)
        q1, q2 = _achsen(so, eo)
        ach = (eo - so).normalized()
        quer = q1 if j % 2 else q2
        f.loft("Kettenglied", [(p + q1 * 0.14, quer, ach, 0.024, 0.024), (p + q1 * 0.145, quer, ach, 0.016, 0.016)], 10, lambda i, k2, p_: eisen, b.arm(1), glatt=True)

    def doppelaxt():
        g = b.griff(-1)
        oben = Vector((0.0, -0.25, 1.0)).normalized()
        a, e = g - oben * 0.5, g + oben * 1.1
        obj = _strecke(f, "Axtstiel", a, e, 0.03, 0.028, farbe("#3E2A1A"), b.hand(-1), 14)
        bh.glatt_einfaerben(obj, lambda p, n, h: farbe("#3E2A1A") * (0.8 + 0.35 * bh.rausch(Vector((p.x * 50, p.y * 50, p.z * 6)), 1.0)))
        kopfmitte = g + oben * 0.95
        vorne = (Vector((0, -1, 0)) - oben * Vector((0, -1, 0)).dot(oben)).normalized()
        for richtung in (vorne, -vorne):
            ringe = []
            for j in range(13):
                t = (j - 6) / 6
                weite = 0.05 + 0.3 * (1 - t * t) ** 0.5
                ringe.append((kopfmitte + oben * t * 0.3 + richtung * (0.04 + weite * 0.5), richtung, X, weite * 0.5, 0.014 * (1 - 0.5 * abs(t))))
            obj = f.loft("Axtblatt", ringe, 10, lambda i, k2, p: eisen, b.hand(-1), oben_zu=True, unten_zu=True)
            bh.glatt_einfaerben(obj, lambda p, n, h, rr=richtung: bh.schmutz(eisen.lerp(farbe("#C8CCD4"), weich(0.2, 0.33, (p - kopfmitte).dot(rr))) * (0.85 + 0.25 * bh.rausch(p, 35.0)), h, 0.5, 0.3))
        f.kugel("Axtmitte", kopfmitte, (0.05, 0.06, 0.09), gold, b.hand(-1), 14, 10)
        _kegel(f, "Axtspitze", e, oben, 0.14, 0.03, eisen, b.hand(-1), 8)
    b.starr("Doppelaxt", "Hand.R", doppelaxt)
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
        c = c.lerp(borste_dunkel, weich(0.2, 0.6, n.z) * weich(0.4, 0.0, p.y) * 0.7)     # dunkler Rücken
        c = c.lerp(borste_dunkel, weich(0.35, 0.2, p.z) * 0.8)                           # dunkle Läufe
        c = c.lerp(schnauze, weich(-0.76, -0.84, p.y))
        c = c.lerp(borste * 1.2, weich(-0.3, -0.7, n.z) * weich(0.3, 0.4, p.z) * 0.6)    # hellerer Bauch
        c = c.lerp(farbe("#161210"), weich(0.075, 0.05, p.z))                              # Hufe
        return c

    # Modelliert statt facettiert: weiche Form, borstiges Fell nach hinten gekämmt, Farbverläufe
    import bildhauer as bh
    formen = [(m, h, False, d) for m, h, d in t.formen]
    rumpf = bh.ball_mesh("Keiler", formen, 0.02)

    def borsten(p, n):
        q = p / t.s
        return 0.004 * noise.noise(Vector((q.x * 70, q.y * 9, q.z * 70))) + 0.006 * weich(0.6, 0.85, q.z) * weich(0.3, -0.5, q.y) * max(0.0, noise.noise(Vector((q.x * 50, q.y * 8, q.z * 50))))

    def fell(p, n, h):
        q = p / t.s
        c = zonen(q, n) * (0.88 + 0.2 * noise.noise(Vector((q.x * 70, q.y * 9, q.z * 70))))
        c = c.lerp(borste_dunkel, weich(0.35, 0.8, n.z) * weich(0.2, -0.5, q.y) * 0.5)
        return bh.schmutz(c, h, 0.45, 0.12)
    bh.modellieren(rumpf, 0.009, 9000, borsten, glaetten=3)
    for v in rumpf.data.vertices:
        v.co.z = max(v.co.z, 0.0)
    bh.einfaerben(rumpf, fell)
    t.rumpf = rumpf
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
