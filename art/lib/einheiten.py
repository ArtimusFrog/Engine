"""Weitere Einheiten für die Tower Defense: Harpyie (fliegt), Schattenmeuchler (getarnt) und der
Soldat der Kaserne (verbündet, hell und freundlich wie die Türme der Spieler).

Gleicher Aufbau wie die Gegner in gegner.py: Zweibeiner mit Skelett und Grundkörper des Magiers,
Animationen Idle, Laufen und Angriff. Koordinaten: Z oben, Blick nach -Y, Füße im Ursprung.
"""

import math

from mathutils import Vector

from figuren import HANDGELENK, SCHULTER, STAB_X, STAB_Y, X, Y, Z, Figur, _arm_gewichte, _rumpf_gewichte, _schleife, _spiegel, farbe
from gegner import (GLUT, GRIFF_L, GRIFF_R, HAND_L, HAND_R, KOPF, LEDER, SCHATTEN, _animationen, _angriff_hieb, _angriff_stoss, _arme,
                    _augen, _beine, _kapuze, _platte, _ring, _rock, _rumpf, _schild, _skelett, _strecke)
from werkstatt import animation


# ---------------------------------------------------------------------------
# Harpyie: gefiederter Körper, Flügel an den Armen, Vogelkrallen – schwebt mit Flügelschlag
# ---------------------------------------------------------------------------
def _harpyie_animationen(armatur):
    def fluegel(phi, hoch=1.0):
        s = math.sin(phi)
        auf = 50 + 45 * s * hoch
        return [("Oberarm.L", "rot", (-8, 0, auf)), ("Oberarm.R", "rot", (-8, 0, -auf)),
                ("Unterarm.L", "rot", (-12 - 18 * max(0.0, -s), 0, 0)), ("Unterarm.R", "rot", (-12 - 18 * max(0.0, -s), 0, 0)),
                ("Becken", "pos", (0, 0, 0.07 * math.cos(phi)))]

    def idle(phi):
        return fluegel(phi) + [("Oberschenkel.L", "rot", (-25, 0, 4)), ("Oberschenkel.R", "rot", (-25, 0, -4)),
                               ("Unterschenkel.L", "rot", (45, 0, 0)), ("Unterschenkel.R", "rot", (45, 0, 0)),
                               ("Fuss.L", "rot", (25, 0, 0)), ("Fuss.R", "rot", (25, 0, 0)),
                               ("Brust", "rot", (6 + 3 * math.sin(phi), 0, 0)), ("Kopf", "rot", (-6, 10 * math.sin(phi * 0.5), 0))]
    animation(armatur, "Idle", 36, _schleife(36, 2, idle))

    def laufen(phi):
        return fluegel(phi, 1.2) + [("Oberschenkel.L", "rot", (30, 0, 4)), ("Oberschenkel.R", "rot", (30, 0, -4)),
                                    ("Unterschenkel.L", "rot", (50, 0, 0)), ("Unterschenkel.R", "rot", (50, 0, 0)),
                                    ("Fuss.L", "rot", (35, 0, 0)), ("Fuss.R", "rot", (35, 0, 0)),
                                    ("Bauch", "rot", (12, 0, 0)), ("Brust", "rot", (16, 0, 0)), ("Kopf", "rot", (-24, 0, 0))]
    animation(armatur, "Laufen", 24, _schleife(24, 2, laufen))

    # Sturzflug: Flügel zurück, Krallen nach vorne, zupacken, wieder hoch
    schluessel = []
    for bild, arm_x, arm_z, bein, knie, brust in ((0, -8, 70, -25, 45, 6), (8, 45, 85, -10, 20, 25), (13, 30, 40, -85, 10, 30),
                                                  (18, 20, 30, -95, 30, 28), (24, -5, 80, -40, 40, 10), (30, -8, 70, -25, 45, 6)):
        schluessel += [(bild, "Oberarm.L", "rot", (arm_x, 0, arm_z)), (bild, "Oberarm.R", "rot", (arm_x, 0, -arm_z)),
                       (bild, "Oberschenkel.L", "rot", (bein, 0, 4)), (bild, "Oberschenkel.R", "rot", (bein, 0, -4)),
                       (bild, "Unterschenkel.L", "rot", (knie, 0, 0)), (bild, "Unterschenkel.R", "rot", (knie, 0, 0)),
                       (bild, "Brust", "rot", (brust, 0, 0)), (bild, "Bauch", "rot", (brust * 0.5, 0, 0)), (bild, "Kopf", "rot", (-brust * 0.6, 0, 0))]
    animation(armatur, "Angriff", 30, schluessel)


def harpyie(seed=109):
    f = Figur("Harpyie", seed)
    feder = farbe("#5A4A6E")
    feder_hell = farbe("#9A86B8")
    feder_dunkel = farbe("#3A2E4A")
    haut = farbe("#A8929E")
    kralle = farbe("#241C22")
    _beine(f, feder_dunkel, kralle, dick=0.78)
    # Krallen: drei nach vorne, eine nach hinten
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        x = 0.1 * seite
        for k in (-1, 0, 1):
            punkte = [Vector((x + 0.025 * k, -0.08, 0.04)), Vector((x + 0.045 * k, -0.17, 0.035)), Vector((x + 0.055 * k, -0.24, 0.0))]
            f.straehne("Kralle", punkte, 0.02, 0.003, kralle, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, 5)
        f.straehne("Sporn", [Vector((x, 0.02, 0.05)), Vector((x, 0.1, 0.01))], 0.016, 0.003, kralle, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, 5)

    def koerper_farbe(i, k, p):
        if -p.normal.y > 0.5 and p.center.z > 1.15:
            return feder_hell
        return (feder if int(p.center.z * 16 + k * 0.3) % 2 else feder * 0.85) * (0.9 + 0.15 * max(0.0, p.normal.z))
    _rumpf(f, koerper_farbe, dick=0.82)
    _rock(f, lambda i, k, p: feder_dunkel if k % 3 else feder, laenge=0.34, weite=0.9)
    # Federkragen um den Hals
    for k in range(16):
        w = math.tau * k / 16
        a = Vector((math.cos(w) * 0.13, math.sin(w) * 0.11, 1.6))
        b = Vector((math.cos(w) * 0.24, math.sin(w) * 0.2 + 0.02, 1.43))
        f.straehne("Kragen", [a, a.lerp(b, 0.5) + Vector((0, 0, 0.02)), b], 0.05, 0.006, feder_hell if k % 2 else feder, _rumpf_gewichte, 5, 0.0, 0.35)
    _arme(f, feder, kralle, dick=0.78)
    # Flügel entlang der Arme, Schwungfedern nach hinten
    for seite in (1, -1):
        s_, h_ = _spiegel(SCHULTER, seite), _spiegel(HANDGELENK, seite)
        arm = h_ - s_
        laenge = arm.length
        u = arm.normalized()
        v = Y
        umriss = [(-0.02, 0.02), (laenge * 0.5, -0.01), (laenge, 0.0), (laenge + 0.4, 0.04), (laenge + 0.34, 0.16), (laenge + 0.22, 0.3),
                  (laenge * 0.95, 0.42), (laenge * 0.72, 0.52), (laenge * 0.5, 0.56), (laenge * 0.28, 0.5), (0.05, 0.36), (-0.02, 0.2)]

        def fluegel_farbe(poly, u=u):
            rand = abs(poly.normal.dot(u.cross(v))) < 0.5
            return feder_dunkel if rand else feder_hell * 0.9
        _platte(f, "Fluegel", umriss, 0.025, s_ + Vector((0.0, 0.03, 0.0)), u, v, fluegel_farbe, _arm_gewichte(seite))
        # Einzelne Schwungfedern am Rand
        for j in range(6):
            t = 0.35 + j * 0.13
            basis = s_ + u * (laenge * min(t, 1.05)) + v * (0.35 + 0.05 * j)
            spitze = basis + v * 0.2 + u * 0.05
            f.straehne("Schwungfeder", [basis, spitze], 0.035, 0.004, feder_dunkel, _arm_gewichte(seite), 4, 0.0, 0.25)
    # Kopf: fahles Gesicht, gelbe Raubvogelaugen, wildes Haar
    f.kugel("Kopf", Vector((0, -0.01, 1.76)), (0.082, 0.092, 0.108), haut, KOPF, 12, 8)
    _augen(f, farbe("#FFD24A"), z=1.775, abstand=0.036, vorne=-0.095, groesse=0.016)
    f.straehne("Schnabelnase", [Vector((0, -0.09, 1.76)), Vector((0, -0.125, 1.735)), Vector((0, -0.12, 1.71))], 0.018, 0.004, farbe("#3A3034"), KOPF, 5)
    for k in range(11):
        x = (k - 5) * 0.018
        punkte = [Vector((x, 0.0, 1.86)), Vector((x * 1.6, 0.1, 1.82)), Vector((x * 2.2, 0.17, 1.68)), Vector((x * 2.6, 0.2, 1.52))]
        f.straehne("Haar", punkte, 0.03, 0.006, feder_dunkel * (1.1 if k % 2 else 0.9), KOPF, 5, 0.3, 0.5)
    _skelett(f)
    return f.fertig(_harpyie_animationen)


# ---------------------------------------------------------------------------
# Schattenmeuchler: dunkles Leder, Kapuze und Maske, zwei Dolche – geduckte Haltung
# ---------------------------------------------------------------------------
def _dolch(f, griff, name, gewichte, knochen):
    anfang = len(f.teile)
    unten = Vector((0, 0, -1))
    _strecke(f, "Griff", griff + Vector((0, 0, 0.07)), griff - Vector((0, 0, 0.05)), 0.014, 0.014, LEDER * 0.6, gewichte, 6)
    f.kiste("Parier", griff - Vector((0, 0, 0.055)), (0.09, 0.02, 0.018), farbe("#555A66"), gewichte)
    umriss = [(-0.022, 0.0), (0.022, 0.0), (0.016, 0.22), (0.0, 0.31), (-0.016, 0.22)]
    _platte(f, "Klinge", umriss, 0.008, griff - Vector((0, 0, 0.06)), Y, unten, lambda poly: farbe("#B9C0CC") if abs(poly.normal.x) > 0.5 else farbe("#6A6E78"), gewichte)
    f.als_starr(name, knochen, anfang)


def schattenmeuchler(seed=110):
    f = Figur("Schattenmeuchler", seed)
    stoff = farbe("#24202C")
    leder = farbe("#3E322C")
    violett = farbe("#4A2466")
    _beine(f, stoff, leder * 0.7, dick=0.9)
    _rumpf(f, lambda i, k, p: leder if abs(p.center.x) < 0.05 else stoff * (0.9 + 0.25 * max(0.0, -p.normal.y)), dick=0.88)
    _rock(f, lambda i, k, p: violett * 0.7 if k % 5 == 0 else stoff, laenge=0.34, weite=0.92)
    # Gekreuzte Gurte mit Wurfmessern
    for sx in (-1, 1):
        _strecke(f, "Gurt", Vector((0.17 * sx, -0.14, 1.5)), Vector((-0.16 * sx, -0.15, 1.02)), 0.016, 0.016, leder * 0.8, _rumpf_gewichte, 6)
    for j in range(3):
        f.kiste("Wurfmesser", (0.05 - j * 0.05, -0.165, 1.33 - j * 0.07), (0.018, 0.012, 0.07), farbe("#8A909C"), _rumpf_gewichte)
    _arme(f, stoff, leder * 0.6, dick=0.86)
    _kapuze(f, stoff * 1.25, spitz=0.3)
    # Maske über Mund und Nase, nur die Augen leuchten
    f.loft("Maske", [_ring((0, -0.02, 1.64), 0.085, 0.09), _ring((0, -0.03, 1.72), 0.092, 0.1), _ring((0, -0.025, 1.76), 0.088, 0.095)], 16,
           lambda i, k, p: violett * 0.8, KOPF, teilung=1)
    _augen(f, GLUT, z=1.785, abstand=0.034, vorne=-0.1, groesse=0.014)
    # Zerfetzter Umhang
    umhang = []
    for i, z in enumerate((1.56, 1.3, 1.0, 0.72)):
        t = i / 3
        umhang.append((Vector((0, 0.13 + 0.1 * t, z)), X, Y, 0.2 + 0.08 * t, 0.015, lambda w, t=t: 1.0 + 0.08 * math.sin(w * 7 + t * 4)))
    f.loft("Umhang", umhang, 18, lambda i, k, p: stoff * (0.8 + 0.3 * p.center.z / 1.6), _rumpf_gewichte, oben_zu=True, unten_zu=True, teilung=2)
    _dolch(f, GRIFF_R, "DolchR", HAND_R, "Hand.R")
    _dolch(f, GRIFF_L, "DolchL", HAND_L, "Hand.L")
    _skelett(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-40, -75), (-40, -75)), gehen=(30, 45, 18, 0.03, 12, 25)))


# ---------------------------------------------------------------------------
# Soldat der Kaserne: blanker Stahl, blauer Waffenrock mit Gold, Helm mit Federbusch, Speer und Schild
# ---------------------------------------------------------------------------
def soldat(seed=111):
    f = Figur("Soldat", seed)
    blau = farbe("#3F6FB5")
    blau_dunkel = farbe("#2D4F86")
    gold = farbe("#D8AE4A")
    stahl = farbe("#B4BCC8")
    leder = farbe("#6E4A32")
    haut = farbe("#E2B894")
    _beine(f, blau_dunkel, leder, dick=1.0, schienen=stahl)
    _rumpf(f, lambda i, k, p: stahl * (0.85 + 0.25 * max(0.0, p.normal.z) + 0.1 * max(0.0, -p.normal.y)), dick=1.05)
    _rock(f, lambda i, k, p: gold if i >= 2.7 else (blau if abs(p.center.x) < 0.12 else blau_dunkel), laenge=0.45)
    f.loft("Waffenrock", [_ring((0, -0.005, 1.52), 0.2, 0.15), _ring((0, -0.01, 1.2), 0.2, 0.162), _ring((0, 0.0, 1.0), 0.19, 0.155)], 28,
           lambda i, k, p: gold if abs(p.center.x) < 0.025 else blau, _rumpf_gewichte, teilung=2)
    f.loft("Guertel", [_ring((0, 0.0, 1.03), 0.195, 0.158), _ring((0, 0.0, 1.08), 0.193, 0.156)], 28, lambda i, k, p: leder, _rumpf_gewichte)
    f.kiste("Schnalle", (0, -0.16, 1.055), (0.05, 0.015, 0.04), gold, _rumpf_gewichte)
    f.stern("Wappen", Vector((0, -0.19, 1.32)), -Y, 0.05, gold, _rumpf_gewichte, zacken=5)
    _arme(f, stahl * 0.9, leder, dick=1.0, schulter=stahl)
    # Gesicht mit Helm (offen), Nasenschutz und blauem Federbusch
    f.kugel("Kopf", Vector((0, -0.01, 1.75)), (0.088, 0.098, 0.112), haut, KOPF, 12, 8)
    _augen(f, SCHATTEN, z=1.77, abstand=0.034, vorne=-0.098, groesse=0.012)
    helm = [_ring((0, 0.0, 1.76), 0.108, 0.112), _ring((0, 0.0, 1.83), 0.112, 0.116), _ring((0, 0.0, 1.9), 0.09, 0.094), _ring((0, 0.0, 1.95), 0.03, 0.03)]
    f.loft("Helm", helm, 24, lambda i, k, p: stahl * (0.9 + 0.3 * max(0.0, p.normal.z)), KOPF, oben_zu=True, teilung=2)
    f.kiste("Nasenschutz", (0, -0.112, 1.76), (0.018, 0.012, 0.09), stahl, KOPF)
    f.loft("Helmrand", [_ring((0, 0.0, 1.755), 0.125, 0.13), _ring((0, 0.0, 1.77), 0.125, 0.13)], 24, lambda i, k, p: gold, KOPF)
    for k in range(7):
        punkte = [Vector((0, -0.02 + k * 0.03, 1.95)), Vector((0, 0.02 + k * 0.035, 2.05 - k * 0.005)), Vector((0, 0.08 + k * 0.04, 1.98 - k * 0.03))]
        f.straehne("Federbusch", punkte, 0.035, 0.008, blau * (1.2 if k % 2 else 1.0), KOPF, 5, 0.0, 0.4)
    _schild(f, blau, gold, zeichen=gold)
    # Speer senkrecht durch die rechte Faust, mit blauem Wimpel
    anfang = len(f.teile)
    unten, oben = Vector((STAB_X, STAB_Y, 0.02)), Vector((STAB_X, STAB_Y, 2.2))
    _strecke(f, "Schaft", unten, oben, 0.02, 0.018, farbe("#9A6B3F"), HAND_R, 8)
    _platte(f, "Spitze", [(-0.035, 0.0), (0.035, 0.0), (0.02, 0.18), (0.0, 0.3), (-0.02, 0.18)], 0.014, oben, X, Z,
            lambda poly: stahl * 1.1, HAND_R)
    f.kiste("Tuelle", oben + Vector((0, 0, -0.03)), (0.035, 0.035, 0.08), gold, HAND_R)
    f.kiste("Wimpel", oben + Vector((0.0, 0.1, -0.18)), (0.01, 0.18, 0.12), blau, HAND_R)
    f.als_starr("Speer", "Hand.R", anfang)
    _skelett(f)
    return f.fertig(_animationen(_angriff_stoss, arme_ruhe=((-20, -60), (0, -12))))
