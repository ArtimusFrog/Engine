"""Kleine Pflanzen im selben Stil wie die Bäume (gleiche Textur, gleiches Material):
Büsche aus Blattbüscheln und hohe Grasbüschel aus gekreuzten Halm-Karten.

Ein Modell-Skript ruft sie nur auf, z. B. art/modelle/natur/gras_1.py: `gras(seed=1)`.
"""

import math

from mathutils import Vector

from baeume import GRAS, OBEN, Baum, farbe


def busch(seed=1, name="Busch", laub="#5DAA35"):
    baum = Baum(name, seed, hoehe=1.6)
    r = baum.rng
    holz = baum.rinde("#5A4332")
    gruen = farbe(laub)
    for k in range(r.randint(3, 4)):
        a = math.tau * k / 4 + r.uniform(-0.4, 0.4)
        abstand = r.uniform(0.2, 0.55)
        mitte = Vector((math.cos(a) * abstand, math.sin(a) * abstand, r.uniform(0.55, 0.85)))
        baum.rohr([Vector((0, 0, 0)), mitte * 0.5, mitte], [0.06, 0.045, 0.03], 5, holz)
        baum.krone(mitte, r.uniform(0.55, 0.75), gruen * r.uniform(0.88, 1.12), dichte=1.3, platt=0.8, kern=0.65)
    return baum.fertig()


def gras(seed=1, name="Gras", unten="#3F7A28", oben="#86B83C", bueschel=4):
    """Grasflecken: mehrere Büschel aus gekreuzten, leicht nach außen gekippten Halm-Karten.
    Unten in der Farbe des Wiesenbodens, damit das Gras aus dem Boden zu wachsen scheint."""
    baum = Baum(name, seed, hoehe=1.0)
    r = baum.rng
    c_unten, c_oben = farbe(unten), farbe(oben)
    for b in range(bueschel):
        winkel = math.tau * b / bueschel + r.uniform(-0.5, 0.5)
        abstand = 0.0 if b == 0 else r.uniform(0.5, 0.9)
        mitte = Vector((math.cos(winkel) * abstand, math.sin(winkel) * abstand, 0))
        _bueschel(baum, mitte, r.uniform(0.75, 1.1) if b == 0 else r.uniform(0.5, 0.85), c_unten, c_oben)
    return baum.fertig()


def _bueschel(baum, mitte, groesse, c_unten, c_oben):
    r = baum.rng
    u0, v0, u1, v1 = GRAS
    karten = r.randint(4, 5)
    for k in range(karten):
        a = math.pi * k / karten + r.uniform(-0.2, 0.2)
        seite = Vector((math.cos(a), math.sin(a), 0))
        versatz = mitte + Vector((r.uniform(-0.12, 0.12), r.uniform(-0.12, 0.12), 0))
        breite = r.uniform(0.7, 1.0) * groesse
        hoehe = r.uniform(0.6, 0.9) * groesse
        kipp = seite.cross(OBEN) * r.uniform(-0.25, 0.25)
        fuss_l, fuss_r = versatz - seite * breite / 2, versatz + seite * breite / 2
        kopf_l = fuss_l + Vector((0, 0, hoehe)) + kipp - seite * 0.08
        kopf_r = fuss_r + Vector((0, 0, hoehe)) + kipp + seite * 0.08
        ton = r.uniform(0.9, 1.1)
        # Normalen fast senkrecht nach oben: Gras soll wie der Boden darunter beleuchtet werden
        n = OBEN + kipp * 0.5
        baum.flaeche([fuss_l, fuss_r, kopf_r, kopf_l], [(u0, v0), (u1, v0), (u1, v1), (u0, v1)],
                     [c_unten * ton, c_unten * ton, c_oben * ton, c_oben * ton], [n] * 4)
