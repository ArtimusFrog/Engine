"""Die großen Bosse der Schattenfestung (je fünfte Welle einer): Bergtroll, Lichkönig,
Spinnenkönigin, Dämonenfürst und Schattendrache. Jeder hat Idle, Laufen und Angriff.

Die Zweibeiner nutzen Grundkörper und Skelett der Gegner (gegner.py) und werden groß skaliert;
Spinne und Drache haben eigene Skelette. Koordinaten: Z oben, Blick nach -Y, Füße im Ursprung.
"""

import math

import bmesh
from mathutils import Matrix, Vector

import tiere
from figuren import (ELLBOGEN, HANDGELENK, KNIE, SCHULTER, STAB_X, STAB_Y, X, Y, Z, Figur, _arm_gewichte, _mischen, _rumpf_gewichte, _schleife,
                     _spiegel, farbe, weich)
from gegner import (GLUT, GRIFF_R, HAND_L, HAND_R, KNOCHEN, KNOCHEN_DUNKEL, KOPF, LEDER, SCHATTEN, STAHL_DUNKEL, STAHL_SCHWARZ, VIOLETT,
                    VIOLETT_DUNKEL, _angriff_hieb, _angriff_zauber, _animationen, _arme, _augen, _beine, _bein_gewichte, _platte, _ring, _rock,
                    _rumpf, _skalieren, _skelett, _strecke)
from werkstatt import animation

EIS = farbe("#8FD8FF")
FEUER = farbe("#FF7A1A")
FEUER_HELL = farbe("#FFD24A")
GOLD = farbe("#D8AE4A")


# ---------------------------------------------------------------------------
# 1. Bergtroll: riesig, gebeugt, graugrüne Haut, Hauer, Lendenschurz aus Fell, Stachelkeule
# ---------------------------------------------------------------------------
def bergtroll(seed=201):
    f = Figur("Bergtroll", seed)
    r = f.rng
    haut = farbe("#7C8A68")
    haut_dunkel = farbe("#5C6A4C")
    fell = farbe("#6A4A30")

    def haut_farbe(i, k, p):
        c = haut * (0.85 + 0.25 * max(0.0, p.normal.z))
        return haut_dunkel if r.random() < 0.08 else c
    _beine(f, haut, haut_dunkel * 0.8, dick=1.7)
    _rumpf(f, lambda i, k, p: haut_farbe(i, k, p), dick=1.55)
    _arme(f, haut, haut_dunkel, dick=1.7, schulter=fell)
    _rock(f, lambda i, k, p: fell * (0.8 if k % 3 == 0 else 1.0), laenge=0.42, weite=1.35)
    f.loft("Guertel", [_ring((0, 0.0, 1.0), 0.27, 0.21), _ring((0, 0.0, 1.07), 0.265, 0.205)], 24, lambda i, k, p: LEDER * 0.7, _rumpf_gewichte)
    f.kugel("Guertelschaedel", Vector((0, -0.215, 1.04)), (0.05, 0.04, 0.055), KNOCHEN, _rumpf_gewichte, 8, 6)
    # Großer, nach vorne geschobener Kopf mit Unterbiss, Knollnase, Hauern und spitzen Ohren
    f.kugel("Kopf", Vector((0, -0.06, 1.74)), (0.13, 0.14, 0.13), haut, KOPF, 12, 8)
    f.kugel("Kiefer", Vector((0, -0.12, 1.66)), (0.12, 0.09, 0.07), haut_dunkel, KOPF, 10, 6)
    f.kugel("Nase", Vector((0, -0.2, 1.74)), (0.04, 0.05, 0.045), haut_dunkel, KOPF, 8, 6)
    f.kugel("Stirn", Vector((0, -0.14, 1.8)), (0.12, 0.05, 0.035), haut_dunkel, KOPF, 10, 6)
    _augen(f, farbe("#FFB020"), z=1.765, abstand=0.05, vorne=-0.18, groesse=0.016)
    for sx in (-1, 1):
        f.straehne("Hauer", [Vector((sx * 0.06, -0.19, 1.66)), Vector((sx * 0.075, -0.22, 1.72)), Vector((sx * 0.07, -0.21, 1.77))], 0.022, 0.005, KNOCHEN, KOPF, 6)
        f.straehne("Ohr", [Vector((sx * 0.12, -0.04, 1.76)), Vector((sx * 0.2, 0.0, 1.8)), Vector((sx * 0.25, 0.04, 1.86))], 0.04, 0.006, haut, KOPF, 5, 0.0, 0.4)
    for k in range(5):
        x = (k - 2) * 0.03
        f.straehne("Haar", [Vector((x, 0.0, 1.86)), Vector((x * 1.4, 0.08, 1.9)), Vector((x * 1.8, 0.15, 1.82))], 0.025, 0.006, farbe("#2E2A22"), KOPF, 4)
    # Warzen und Steinbrocken auf den Schultern
    for i in range(10):
        f.kugel("Warze", Vector((r.uniform(-0.2, 0.2), r.uniform(-0.18, 0.1), r.uniform(1.15, 1.55))), (0.02, 0.02, 0.02), haut_dunkel, _rumpf_gewichte, 6, 4)
    # Stachelkeule in der rechten Hand: dicker Knüppel, Eisenstacheln, Lederwicklung
    anfang = len(f.teile)
    richtung = Vector((0, -0.35, -0.94)).normalized()
    griff = GRIFF_R
    _strecke(f, "Keulengriff", griff - richtung * 0.15, griff + richtung * 0.2, 0.035, 0.04, LEDER, HAND_R, 8)
    _strecke(f, "Keule", griff + richtung * 0.15, griff + richtung * 0.95, 0.05, 0.12, farbe("#6A4A2A"), HAND_R, 10)
    quer = richtung.cross(X).normalized()
    for k in range(10):
        t = 0.45 + (k % 5) * 0.11
        w = math.tau * k / 10 * 3.3
        basis = griff + richtung * t
        nach = (X * math.cos(w) + quer * math.sin(w)).normalized()
        f.straehne("Stachel", [basis + nach * 0.08, basis + nach * 0.2], 0.022, 0.003, STAHL_DUNKEL, HAND_R, 5)
    f.als_starr("Keule", "Hand.R", anfang)
    _skalieren(f, 2.4)
    _boden(f)
    _skelett(f, 2.4)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-8, -30), (-25, -45)), gehen=(20, 30, 12, 0.05, 16, 20)))


# ---------------------------------------------------------------------------
# 2. Lichkönig: schwebender Totenkönig in Robe, Krone, Schädel mit Eisaugen, Knochenstab mit Eiskristall
# ---------------------------------------------------------------------------
def lichkoenig(seed=202):
    f = Figur("Lichkoenig", seed)
    robe = farbe("#2A2A3E")
    robe_hell = farbe("#4A4A6E")
    _rumpf(f, lambda i, k, p: robe * (0.85 + 0.25 * max(0.0, -p.normal.y)), dick=1.0)
    _rock(f, lambda i, k, p: robe_hell if i >= 3.2 else (robe * 0.8 if k % 6 in (0, 1) else robe), laenge=0.95, weite=1.3)
    f.loft("Schaerpe", [_ring((0, 0.0, 1.02), 0.2, 0.16), _ring((0, 0.0, 1.08), 0.198, 0.158)], 24, lambda i, k, p: EIS * 0.7, _rumpf_gewichte)
    _arme(f, robe, KNOCHEN_DUNKEL, dick=0.95, schulter=STAHL_SCHWARZ)
    # Stachelige Schulterstücke
    for sx in (-1, 1):
        s = _spiegel(SCHULTER, sx)
        for j in range(3):
            f.straehne("Schulterdorn", [s + Vector((sx * 0.05, 0.0, 0.08)), s + Vector((sx * (0.12 + j * 0.03), (j - 1) * 0.06, 0.26 + j * 0.04))], 0.03, 0.004, STAHL_DUNKEL,
                       lambda co, sx=sx: _mischen(("Brust", 0.45), (f"Oberarm.{'L' if sx > 0 else 'R'}", 0.55)), 5)
    # Totenschädel mit Eisaugen, Kiefer und Krone
    f.loft("Schaedel", [_ring((0, 0.0, 1.66), 0.07, 0.08), _ring((0, -0.01, 1.72), 0.1, 0.11), _ring((0, 0.0, 1.8), 0.105, 0.115),
                        _ring((0, 0.01, 1.87), 0.09, 0.1), _ring((0, 0.01, 1.91), 0.04, 0.045)], 16, lambda i, k, p: KNOCHEN * (0.85 + 0.2 * max(0.0, p.normal.z)),
           KOPF, oben_zu=True, unten_zu=True, teilung=2)
    f.kiste("Kiefer", (0, -0.05, 1.66), (0.1, 0.07, 0.04), KNOCHEN_DUNKEL, KOPF)
    for sx in (-1, 1):
        f.kugel("Augenhoehle", Vector((sx * 0.04, -0.1, 1.76)), (0.03, 0.02, 0.028), SCHATTEN, KOPF, 8, 6)
    _augen(f, EIS, z=1.76, abstand=0.04, vorne=-0.112, groesse=0.014)
    f.loft("Kronreif", [_ring((0, 0.0, 1.84), 0.115, 0.125), _ring((0, 0.0, 1.88), 0.118, 0.128)], 16, lambda i, k, p: GOLD, KOPF)
    for k in range(7):
        w = math.tau * k / 7
        basis = Vector((math.cos(w) * 0.115, math.sin(w) * 0.125, 1.87))
        f.straehne("Krone", [basis, basis + Vector((math.cos(w) * 0.02, math.sin(w) * 0.02, 0.1 + 0.04 * (k % 2)))], 0.022, 0.003, GOLD, KOPF, 4)
    # Kapuze/Kragen hinten hoch
    f.loft("Kragen", [_ring((0, 0.03, 1.55), 0.2, 0.17), _ring((0, 0.08, 1.75), 0.2, 0.16), _ring((0, 0.12, 1.95), 0.16, 0.1)], 18,
           lambda i, k, p: robe_hell if p.normal.y < 0 else robe, _rumpf_gewichte, teilung=1)
    # Zerfetzter Umhang
    umhang = []
    for i, z in enumerate((1.55, 1.25, 0.9, 0.5, 0.15)):
        t = i / 4
        umhang.append((Vector((0, 0.16 + 0.14 * t, z)), X, Y, 0.24 + 0.12 * t, 0.02, lambda w, t=t: 1.0 + 0.08 * math.sin(w * 7 + t * 5)))
    f.loft("Umhang", umhang, 22, lambda i, k, p: robe * (0.7 + 0.3 * weich(0.2, 1.5, p.center.z)), lambda co: _mischen(("Brust", 0.6), ("Becken", 0.4)),
           oben_zu=True, unten_zu=True, teilung=2)
    # Knochenstab mit Schädel und Eiskristall
    anfang = len(f.teile)
    unten, oben = Vector((STAB_X, STAB_Y, 0.0)), Vector((STAB_X, STAB_Y, 2.2))
    _strecke(f, "Stab", unten, oben, 0.025, 0.02, KNOCHEN_DUNKEL, HAND_R, 8)
    f.kugel("Stabschaedel", oben + Vector((0, 0, 0.06)), (0.07, 0.075, 0.08), KNOCHEN, HAND_R, 10, 6)
    f.loft("Eiskristall", [(oben + Vector((0, 0, 0.12)), X, Y, 0.001, 0.001), (oben + Vector((0, 0, 0.3)), X, Y, 0.07, 0.07),
                           (oben + Vector((0, 0, 0.55)), X, Y, 0.001, 0.001)], 6, lambda i, k, p: EIS, HAND_R, teilung=1)
    f.als_starr("Stab", "Hand.R", anfang)
    _skalieren(f, 2.1)
    _skelett(f, 2.1)
    return f.fertig(_animationen(_angriff_zauber, arme_ruhe=((-8, -25), (0, -10)), schweben=True))


# ---------------------------------------------------------------------------
# 4. Dämonenfürst: muskulös, rot und schwarz, Widderhörner, Fledermausflügel, Hufe, Schwanz, Flammenschwert
# ---------------------------------------------------------------------------
def daemonenfuerst(seed=204):
    f = Figur("Daemonenfuerst", seed)
    rot = farbe("#8A2A22")
    rot_dunkel = farbe("#4A1210")
    schwarz = farbe("#1A1214")
    _beine(f, rot, schwarz, dick=1.45, schienen=rot_dunkel)

    def rumpf_farbe(i, k, p):
        # Muskelzeichnung: dunkle Rillen, glühende Risse auf der Brust
        if -p.normal.y > 0.6 and 1.25 < p.center.z < 1.45 and abs(p.center.x) < 0.04:
            return FEUER
        return (rot if (int(p.center.z * 14) + k) % 5 else rot_dunkel) * (0.85 + 0.25 * max(0.0, -p.normal.y))
    _rumpf(f, rumpf_farbe, dick=1.45)
    _arme(f, rot, schwarz, dick=1.45, schulter=schwarz)
    _rock(f, lambda i, k, p: schwarz if k % 4 else rot_dunkel, laenge=0.35, weite=1.2)
    f.loft("Guertel", [_ring((0, 0.0, 1.0), 0.25, 0.2), _ring((0, 0.0, 1.07), 0.245, 0.195)], 24, lambda i, k, p: GOLD * 0.8, _rumpf_gewichte)
    # Kopf mit Widderhörnern und Feueraugen
    f.kugel("Kopf", Vector((0, -0.02, 1.75)), (0.1, 0.115, 0.12), rot_dunkel, KOPF, 12, 8)
    f.kugel("Kinn", Vector((0, -0.1, 1.67)), (0.06, 0.05, 0.05), rot_dunkel, KOPF, 8, 6)
    _augen(f, FEUER_HELL, z=1.77, abstand=0.042, vorne=-0.11, groesse=0.018)
    for sx in (-1, 1):
        punkte = [Vector((sx * 0.07, -0.02, 1.84)), Vector((sx * 0.17, 0.02, 1.95)), Vector((sx * 0.26, 0.12, 1.95)), Vector((sx * 0.27, 0.16, 1.82)),
                  Vector((sx * 0.22, 0.08, 1.74))]
        f.straehne("Horn", punkte, 0.06, 0.01, schwarz, KOPF, 8)
    # Fledermausflügel am Rücken: Knochenfinger und Flughaut
    for sx in (-1, 1):
        wurzel = Vector((sx * 0.1, 0.15, 1.45))
        finger = [wurzel + Vector((sx * 0.9, 0.35, 0.75)), wurzel + Vector((sx * 1.1, 0.4, 0.2)), wurzel + Vector((sx * 0.95, 0.35, -0.35)),
                  wurzel + Vector((sx * 0.55, 0.25, -0.65))]
        for spitze in finger:
            f.straehne("Fluegelfinger", [wurzel, wurzel.lerp(spitze, 0.5) + Vector((0, 0.05, 0.05)), spitze], 0.035, 0.008, schwarz, lambda co: {"Brust": 1.0}, 5)
        bm = bmesh.new()
        punkte = [bm.verts.new(wurzel)] + [bm.verts.new(p + Vector((0, 0.01, 0))) for p in finger]
        for a, b in zip(punkte[1:], punkte[2:]):
            bm.faces.new((punkte[0], a, b))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        _doppelseitig(bm)
        f._objekt(bm, "Flughaut", lambda poly: rot_dunkel * 0.8, lambda co: {"Brust": 1.0})
    # Schwanz mit Spitze
    f.straehne("Schwanz", [Vector((0, 0.15, 0.98)), Vector((0, 0.45, 0.7)), Vector((0.1, 0.7, 0.35)), Vector((0.2, 0.85, 0.2))], 0.05, 0.012, rot_dunkel,
               lambda co: {"Becken": 1.0}, 6)
    f.straehne("Schwanzspitze", [Vector((0.2, 0.85, 0.2)), Vector((0.28, 0.98, 0.14))], 0.06, 0.004, schwarz, lambda co: {"Becken": 1.0}, 4)
    # Flammenschwert
    anfang = len(f.teile)
    richtung = Vector((0, -0.62, -0.78)).normalized()
    griff = GRIFF_R
    _strecke(f, "Schwertgriff", griff - richtung * 0.14, griff + richtung * 0.06, 0.02, 0.02, schwarz, HAND_R, 6)
    f.kiste("Parier", griff + richtung * 0.08, (0.26, 0.04, 0.04), GOLD, HAND_R)
    umriss = [(-0.05, 0.1), (0.05, 0.1), (0.07, 0.5), (0.03, 0.9), (0.0, 1.15), (-0.04, 0.8), (-0.06, 0.45)]
    _platte(f, "Flammenklinge", umriss, 0.02, griff, X, richtung, lambda poly: FEUER_HELL if abs(poly.normal.x) < 0.5 else FEUER, HAND_R)
    f.als_starr("Flammenschwert", "Hand.R", anfang)
    _skalieren(f, 2.3)
    _boden(f)
    _skelett(f, 2.3)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-15, -40), (-10, -25)), gehen=(24, 34, 14, 0.04, 6, 14)))


def _doppelseitig(bm, dicke=0.02):
    """Flughäute von beiden Seiten sichtbar: jede Fläche noch einmal, umgedreht und etwas versetzt."""
    bm.normal_update()
    kopie = {}
    flaechen = list(bm.faces)
    for fl in flaechen:
        for v in fl.verts:
            if v not in kopie:
                kopie[v] = bm.verts.new(v.co - fl.normal * dicke)
    for fl in flaechen:
        bm.faces.new([kopie[v] for v in reversed(fl.verts)])


def _boden(f):
    """Nach dem Vergrößern: nichts unter den Boden (Sohlen flach auf z = 0)."""
    for obj in f.teile + [o for _, _, objekte in f.starr for o in objekte]:
        for v in obj.data.vertices:
            v.co.z = max(v.co.z, -0.02)


# ---------------------------------------------------------------------------
# 3. Spinnenkönigin: riesige Spinne mit acht Beinen, dickem Hinterleib mit violetter Zeichnung,
#    Augenkranz und Kristallkrone. Eigenes Skelett: Körper, Hinterleib, Kopf, je Bein zwei Glieder.
# ---------------------------------------------------------------------------
_SPINNE_WINKEL = (-55.0, -20.0, 18.0, 52.0)   # Richtung der Beine: vorne → hinten (Grad gegen „seitlich“)
_SPINNE_ANSATZ = (-0.75, -0.3, 0.15, 0.55)    # wo sie am Körper ansetzen (y)


def _spinne_bein(i, seite):
    """Ansatz, Knie und Fuß eines Beins (Welt, Meter)."""
    w = math.radians(_SPINNE_WINKEL[i])
    aussen = Vector((seite * math.cos(w), math.sin(w), 0.0))
    ansatz = Vector((seite * 0.55, _SPINNE_ANSATZ[i], 1.45))
    knie = ansatz + aussen * 1.5 + Vector((0, 0, 1.2))
    fuss = ansatz + aussen * 3.1
    fuss.z = 0.02
    return ansatz, knie, fuss, aussen


def _spinne_animationen(armatur):
    beine = [(i, s) for i in range(4) for s in ("L", "R")]

    def bein_werte(i, s, phase, heben, schwung):
        # L schwingt mit negativem Z-Winkel nach vorne, R mit positivem
        vor = -1.0 if s == "L" else 1.0
        return [(f"Bein{i}{s}.oben", "rot", (heben, 0, vor * schwung)), (f"Bein{i}{s}.unten", "rot", (heben * 0.8, 0, 0))]

    def idle(phi):
        werte = [("Koerper", "pos", (0, 0, 0.04 * math.sin(phi * 2))), ("Hinterleib", "rot", (3 * math.sin(phi), 0, 5 * math.sin(phi))),
                 ("Kopf", "rot", (2 * math.sin(phi * 2), 0, 6 * math.sin(phi)))]
        for i, s in beine:
            werte += bein_werte(i, s, phi, 3 * math.sin(phi * 2 + i), 2 * math.sin(phi + i))
        return werte
    animation(armatur, "Idle", 90, _schleife(90, 3, idle))

    def laufen(phi):
        werte = [("Koerper", "pos", (0, 0, 0.05 * abs(math.sin(phi)))), ("Hinterleib", "rot", (0, 0, 4 * math.sin(phi)))]
        for i, s in beine:
            gruppe_a = (i % 2 == 0) == (s == "L")
            p = (phi / math.tau + (0.0 if gruppe_a else 0.5)) % 1.0
            if p < 0.5:
                u = p / 0.5
                schwung = -16 + 32 * u
                heben = 28 * math.sin(math.pi * u)
            else:
                u = (p - 0.5) / 0.5
                schwung = 16 - 32 * u
                heben = 0.0
            werte += bein_werte(i, s, phi, heben, schwung)
        return werte
    animation(armatur, "Laufen", 24, _schleife(24, 2, laufen))

    # Angriff: Vorderbeine hoch, Körper bäumt sich, dann zustoßen
    schluessel = []
    for bild, heben, stoss, koerper in ((0, 0, 0, 0), (10, 70, 10, -12), (15, 75, 12, -15), (20, -10, -20, 10), (26, 10, 0, 4), (32, 0, 0, 0)):
        for s in ("L", "R"):
            vor = -1.0 if s == "L" else 1.0
            schluessel += [(bild, f"Bein0{s}.oben", "rot", (heben, 0, vor * stoss)), (bild, f"Bein0{s}.unten", "rot", (heben * 0.6, 0, 0)),
                           (bild, f"Bein1{s}.oben", "rot", (heben * 0.4, 0, 0)), (bild, f"Bein1{s}.unten", "rot", (0, 0, 0))]
        schluessel += [(bild, "Koerper", "rot", (koerper, 0, 0)), (bild, "Kopf", "rot", (-koerper * 0.5, 0, 0))]
    animation(armatur, "Angriff", 32, schluessel)


def spinnenkoenigin(seed=203):
    f = Figur("Spinnenkoenigin", seed)
    r = f.rng
    chitin = farbe("#241C2C")
    chitin_hell = farbe("#3E3050")
    violett = farbe("#8E4FC4")
    koerper = lambda co: {"Koerper": 1.0}
    hinterleib = lambda co: {"Hinterleib": 1.0}
    kopf = lambda co: {"Kopf": 1.0}
    # Vorderkörper, Kopf, Hinterleib
    f.kugel("Vorderkoerper", Vector((0, -0.25, 1.45)), (0.7, 0.95, 0.5), chitin, koerper, 14, 10)
    f.kugel("Kopf", Vector((0, -1.1, 1.4)), (0.42, 0.45, 0.35), chitin_hell, kopf, 12, 8)

    def leib_farbe(poly):
        p = poly.center
        # Violette Sanduhr-Zeichnung oben auf dem Hinterleib
        if poly.normal.z > 0.5 and abs(p.x) < 0.25 - abs(p.y - 1.55) * 0.18:
            return violett
        return chitin * (0.8 + 0.35 * max(0.0, poly.normal.z)) * r.uniform(0.92, 1.08)
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=18, v_segments=12, radius=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * 1.15, v.co.y * 1.45, v.co.z * 1.0)) + Vector((0, 1.55, 1.8))
    f._objekt(bm, "Hinterleib", leib_farbe, hinterleib)
    # Stacheln und Kristalle auf dem Hinterleib, Kristallkrone auf dem Kopf
    for k in range(9):
        w = r.uniform(-0.9, 0.9)
        basis = Vector((w * 0.8, 1.2 + r.uniform(-0.3, 0.9), 2.6 - abs(w) * 0.4))
        f.straehne("Leibdorn", [basis, basis + Vector((w * 0.2, 0.15, 0.45))], 0.08, 0.005, chitin_hell, hinterleib, 5)
    for k in range(5):
        w = (k - 2) * 0.12
        basis = Vector((w, -1.05, 1.7))
        f.loft("Kronkristall", [(basis, X, Y, 0.001, 0.001), (basis + Vector((w * 0.3, 0.05, 0.2)), X, Y, 0.06, 0.06),
                                (basis + Vector((w * 0.5, 0.1, 0.45 - abs(w))), X, Y, 0.001, 0.001)], 5, lambda i, kk, p: GLUT, kopf, teilung=1)
    # Augenkranz (acht rote Augen) und Beißklauen
    for k in range(8):
        x = (k % 4 - 1.5) * 0.12
        z = 1.5 + (k // 4) * 0.12
        f.kugel("Auge", Vector((x, -1.45, z)), (0.05, 0.035, 0.05), farbe("#FF3A2A"), kopf, 6, 4)
    for sx in (-1, 1):
        f.straehne("Klaue", [Vector((sx * 0.14, -1.4, 1.25)), Vector((sx * 0.18, -1.6, 1.0)), Vector((sx * 0.08, -1.62, 0.8))], 0.08, 0.01, farbe("#15101A"), kopf, 6)
    # Acht Beine: je zwei Glieder, Gelenkkugeln, Borsten
    for i in range(4):
        for seite, sn in ((1, "L"), (-1, "R")):
            ansatz, knie, fuss, aussen = _spinne_bein(i, seite)
            oben = lambda co, n=f"Bein{i}{sn}.oben": {n: 1.0}
            unten = lambda co, n=f"Bein{i}{sn}.unten": {n: 1.0}
            _strecke(f, "Oberbein", ansatz, knie, 0.16, 0.12, chitin_hell, oben, 8)
            f.kugel("Kniegelenk", knie, (0.14, 0.14, 0.14), chitin, oben, 8, 6)
            _strecke(f, "Unterbein", knie, fuss, 0.11, 0.035, chitin, unten, 8)
            for j in range(3):
                p = ansatz.lerp(knie, 0.3 + j * 0.25)
                f.straehne("Borste", [p, p + Vector((0, 0, 0.18)) + aussen * 0.06], 0.02, 0.003, farbe("#4A3A58"), oben, 3)
            f.knochen_dazu(f"Bein{i}{sn}.oben", ansatz, knie, "Koerper", (0, 0, 1))
            f.knochen_dazu(f"Bein{i}{sn}.unten", knie, fuss, f"Bein{i}{sn}.oben", tuple(aussen))
    f.knochen_dazu("Koerper", (0, 0.5, 1.45), (0, -0.8, 1.45), None, (0, 0, 1))
    f.knochen_dazu("Hinterleib", (0, 0.45, 1.6), (0, 2.9, 1.9), "Koerper", (0, 0, 1))
    f.knochen_dazu("Kopf", (0, -0.8, 1.45), (0, -1.6, 1.35), "Koerper", (0, 0, 1))
    # Die Knochen müssen vor den Beinen stehen (Eltern zuerst)
    f.knochen = sorted(f.knochen, key=lambda k: (0 if k[3] is None else 1 if k[3] == "Koerper" and not k[0].startswith("Bein") else 2 if k[0].endswith(".oben") else 3))
    return f.fertig(_spinne_animationen)


# ---------------------------------------------------------------------------
# 5. Schattendrache: gewaltiger Drache mit langem Hals, Hörnern, Rückenstacheln, großen Schwingen.
#    Tier-Baukasten (Metaball-Körper), Flügel als starre Teile an eigenen Knochen.
# ---------------------------------------------------------------------------
def _drache_animieren(armatur, stil, knochen):
    def fluegel(bild, winkel, falte=0.0):
        return [(bild, "Fluegel.L", "rot", (winkel, 0, falte)), (bild, "Fluegel.R", "rot", (winkel, 0, -falte))]

    idle = []
    for bild, atmen, blick, flug in ((0, 0.0, -10, 0), (45, 0.05, 0, 6), (90, 0.0, 12, 0), (135, 0.05, 0, 6), (180, 0.0, -10, 0)):
        idle += [tiere._pos(bild, "Koerper", atmen), tiere._rot(bild, "Hals", x=4 if bild % 90 else -2, z=blick * 0.5), tiere._rot(bild, "Kopf", x=-3, z=blick * 0.4)]
        idle += fluegel(bild, flug, 20)
        for s, name in enumerate(("Schwanz1", "Schwanz2")):
            idle.append(tiere._rot(bild, name, z=(8 + s * 6) * math.sin(bild / 180 * math.tau)))
    animation(armatur, "Idle", 180, idle)

    t_lauf = stil["schritt"]
    laufen = []
    for bein, versatz in (("Bein.VL", 0.0), ("Bein.HR", 0.06), ("Bein.VR", 0.5), ("Bein.HL", 0.56)):
        laufen += tiere._bein(t_lauf, bein, versatz, stil["schwung"], stil["knick"], stand=0.6)
    for i in range(9):
        bild = round(i * t_lauf / 8)
        laufen += [tiere._pos(bild, "Koerper", stil["wippen"] * (i % 2)), tiere._rot(bild, "Hals", x=3 * math.sin(i * math.pi / 4)),
                   tiere._rot(bild, "Kopf", x=-3 * math.sin(i * math.pi / 4))]
        # Schwingen schlagen langsam mit (beim Aufsteigen tragen sie ihn)
        laufen += fluegel(bild, 25 * math.sin(i * math.pi / 4), 10)
        for s, name in enumerate(("Schwanz1", "Schwanz2")):
            laufen.append(tiere._rot(bild, name, z=(10 + s * 6) * math.sin(i * math.pi / 4 + 1)))
    animation(armatur, "Laufen", t_lauf, laufen)

    # Angriff: Hals zurück, Schwingen weit auf, dann nach vorne speien
    angriff = []
    for bild, hals, kopf, flug in ((0, 0, 0, 0), (10, 35, 20, 45), (16, 40, 25, 55), (22, -30, -15, 20), (30, -25, -12, 10), (40, 0, 0, 0)):
        angriff += [tiere._rot(bild, "Hals", x=hals), tiere._rot(bild, "Kopf", x=kopf)] + fluegel(bild, flug, 0)
        angriff.append(tiere._pos(bild, "Koerper", 0.15 if 8 < bild < 26 else 0.0))
    animation(armatur, "Angriff", 40, angriff)


def schattendrache(seed=205):
    t = tiere.Tier("Schattendrache", seed, massstab=1.0)
    schuppe = farbe("#2B2433")
    ruecken = farbe("#1C1622")
    bauch = farbe("#5E4E62")

    def zonen(p, n):
        c = schuppe
        if n.z > 0.45 and abs(p.x) < 0.5:
            c = ruecken
        if n.z < -0.3:
            c = bauch
        return c * (0.88 + 0.2 * max(0.0, n.z))

    # Rumpf
    t.form((0, -1.2, 2.2), (0.95, 1.25, 0.95))
    t.form((0, 0.3, 2.05), (0.9, 1.3, 0.85))
    t.form((0, 1.5, 2.1), (0.85, 0.95, 0.85))
    # Hals und Kopf
    t.glied((0, -2.0, 2.7), (0, -3.3, 4.1), 0.55, 0.38, spiegeln=False)
    t.form((0, -3.9, 4.4), (0.42, 0.62, 0.38))
    t.form((0, -4.55, 4.3), (0.27, 0.55, 0.22))
    t.form((0, -4.35, 4.08), (0.23, 0.48, 0.13))
    # Schwanz
    t.glied((0, 2.3, 2.0), (0, 4.0, 1.45), 0.55, 0.32, spiegeln=False)
    t.glied((0, 4.0, 1.45), (0, 6.2, 0.75), 0.32, 0.1, spiegeln=False)
    # Beine
    t.glied((0.72, -1.3, 1.85), (0.82, -1.2, 0.95), 0.38, 0.27)
    t.glied((0.82, -1.2, 0.95), (0.82, -1.5, 0.12), 0.26, 0.18)
    t.form((0.82, -1.72, 0.14), (0.27, 0.38, 0.14), spiegeln=True)
    t.glied((0.78, 1.55, 1.9), (0.88, 1.2, 0.95), 0.48, 0.32)
    t.glied((0.88, 1.2, 0.95), (0.88, 1.65, 0.12), 0.27, 0.2)
    t.form((0.88, 1.45, 0.14), (0.28, 0.4, 0.14), spiegeln=True)
    t.koerper(zonen, aufloesung=0.11, beulen=0.03, ziel=6500)

    # Hörner, Augen, Zähne, Rückenstacheln
    for sx in (-1, 1):
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=6, radius1=0.12, radius2=0.0, depth=1.0)
        for v in bm.verts:
            v.co = Matrix.Rotation(-1.1, 3, "X") @ Matrix.Rotation(sx * 0.35, 3, "Y") @ v.co + Vector((sx * 0.25, -3.55, 4.85))
        t._teil(bm, "Horn", lambda p, n: farbe("#C9BFA8"), "Kopf")
        t.kugel(Vector((sx * 0.28, -4.25, 4.5)), 0.08, GLUT, name="Auge")
        for k in range(4):
            bm = bmesh.new()
            bmesh.ops.create_cone(bm, cap_ends=True, segments=4, radius1=0.04, radius2=0.0, depth=0.18)
            for v in bm.verts:
                v.co = Matrix.Rotation(math.pi, 3, "X") @ v.co + Vector((sx * 0.18, -4.35 - k * 0.14, 4.02))
            t._teil(bm, "Zahn", lambda p, n: farbe("#E8E0CC"), "Kopf")
    for k in range(14):
        y = -3.0 + k * 0.62
        z = 2.95 + (0.9 if y < -1.8 else 0.0) * (1.0 - (y + 3.0) / 1.2) - max(0.0, y - 2.0) * 0.28
        knochen = "Hals" if y < -1.8 else "Koerper" if y < 2.2 else "Schwanz1"
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=4, radius1=0.16, radius2=0.0, depth=0.5 - abs(k - 6) * 0.02)
        for v in bm.verts:
            v.co = Matrix.Rotation(0.4, 3, "X") @ v.co + Vector((0, y, z + 0.2))
        t._teil(bm, "Rueckenstachel", lambda p, n: farbe("#6A3A8A"), knochen)
    # Schwingen: Knochenfinger und Flughaut, starr an den Flügelknochen
    for sx, sn in ((1, "L"), (-1, "R")):
        wurzel = Vector((sx * 0.55, -1.0, 2.95))
        finger = [wurzel + Vector((sx * 3.8, 0.2, 2.0)), wurzel + Vector((sx * 4.8, 1.2, 0.8)), wurzel + Vector((sx * 4.0, 2.4, 0.0)),
                  wurzel + Vector((sx * 2.3, 3.1, -0.4))]
        for spitze in finger:
            bm = bmesh.new()
            bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.09, radius2=0.03, depth=(spitze - wurzel).length)
            achse = (spitze - wurzel).normalized()
            dreh = Vector((0, 0, 1)).rotation_difference(achse).to_matrix()
            for v in bm.verts:
                v.co = dreh @ v.co + (wurzel + spitze) / 2
            t._teil(bm, "Fluegelknochen", lambda p, n: ruecken, f"Fluegel.{sn}")
        bm = bmesh.new()
        punkte = [bm.verts.new(wurzel + Vector((0, 0.1, 0)))] + [bm.verts.new(p) for p in finger] + [bm.verts.new(wurzel + Vector((sx * 0.3, 2.2, -0.6)))]
        for a, b in zip(punkte[1:], punkte[2:]):
            bm.faces.new((punkte[0], a, b))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        _doppelseitig(bm, 0.04)
        t._teil(bm, "Flughaut", lambda p, n: farbe("#3A2448"), f"Fluegel.{sn}")

    t.knochen_dazu("Koerper", (0, 1.6, 2.1), (0, -1.6, 2.3), None)
    t.knochen_dazu("Hals", (0, -1.6, 2.3), (0, -3.5, 4.2), "Koerper")
    t.knochen_dazu("Kopf", (0, -3.5, 4.2), (0, -4.95, 4.2), "Hals")
    t.knochen_dazu("Schwanz1", (0, 2.0, 2.0), (0, 4.0, 1.45), "Koerper")
    t.knochen_dazu("Schwanz2", (0, 4.0, 1.45), (0, 6.2, 0.75), "Schwanz1")
    t.bein_knochen("V", 0.75, (0.75, -1.3, 1.9), (0.82, -1.2, 0.95), (0.82, -1.6, 0.05))
    t.bein_knochen("H", 0.8, (0.8, 1.5, 2.0), (0.88, 1.2, 0.95), (0.88, 1.6, 0.05))
    for sx, sn in ((1, "L"), (-1, "R")):
        t.knochen_dazu(f"Fluegel.{sn}", (sx * 0.55, -1.0, 2.95), (sx * 2.4, -0.7, 3.6), "Koerper", (0, 0, 1))

    stil = dict(schritt=40, schwung=20, knick=40, wippen=0.06)
    original = tiere._animieren
    tiere._animieren = _drache_animieren
    try:
        return t.fertig(stil)
    finally:
        tiere._animieren = original
