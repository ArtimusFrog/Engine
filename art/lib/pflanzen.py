"""Kleine Pflanzen im selben Stil wie die Bäume (gleiche Textur, gleiches Material):
Büsche aus Blattbüscheln und hohe Grasbüschel aus gekreuzten Halm-Karten.

Ein Modell-Skript ruft sie nur auf, z. B. art/modelle/natur/gras_1.py: `gras(seed=1)`.
"""

import math

from mathutils import Vector

from baeume import FLECK, GRAS, OBEN, Baum, farbe


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


def schilf(seed=1, name="Schilf"):
    """Schilf am Ufer: hohe, schmale Halm-Karten in dichten Büscheln, dazwischen Rohrkolben
    (braune Kolben auf dünnen Stängeln)."""
    baum = Baum(name, seed, hoehe=2.0)
    r = baum.rng
    unten, oben = farbe("#4E6B2A"), farbe("#A8B455")
    u0, v0, u1, v1 = GRAS
    for b in range(5):
        w = math.tau * b / 5 + r.uniform(-0.5, 0.5)
        mitte = Vector((math.cos(w), math.sin(w), 0)) * (0.0 if b == 0 else r.uniform(0.35, 0.8))
        for k in range(r.randint(4, 6)):
            a = math.pi * k / 5 + r.uniform(-0.3, 0.3)
            seite = Vector((math.cos(a), math.sin(a), 0))
            fuss = mitte + Vector((r.uniform(-0.1, 0.1), r.uniform(-0.1, 0.1), 0))
            breite, hoehe = r.uniform(0.3, 0.45), r.uniform(1.2, 1.9)
            kipp = seite.cross(OBEN) * r.uniform(-0.3, 0.3)
            l, rr = fuss - seite * breite / 2, fuss + seite * breite / 2
            ton = r.uniform(0.9, 1.1)
            n = OBEN + kipp * 0.5
            baum.flaeche([l, rr, rr + Vector((0, 0, hoehe)) + kipp, l + Vector((0, 0, hoehe)) + kipp],
                         [(u0, v0), (u1, v0), (u1, v1), (u0, v1)], [unten * ton, unten * ton, oben * ton, oben * ton], [n] * 4)
    # Rohrkolben: schmale Stängel, oben ein brauner Kolben (vier gekreuzte Karten, ohne Textur-Ausschnitt)
    stiel, kolben = farbe("#5E6E32"), farbe("#5A3A22")
    fu, fv = FLECK
    for k in range(r.randint(3, 5)):
        w = r.uniform(0, math.tau)
        fuss = Vector((math.cos(w), math.sin(w), 0)) * r.uniform(0.1, 0.6)
        hoehe = r.uniform(1.5, 2.1)
        for d in (Vector((1, 0, 0)), Vector((0, 1, 0))):
            s = d * 0.015
            baum.flaeche([fuss - s, fuss + s, fuss + s + Vector((0, 0, hoehe)), fuss - s + Vector((0, 0, hoehe))],
                         [(fu, fv)] * 4, [stiel] * 4, [OBEN] * 4)
            k0 = fuss + Vector((0, 0, hoehe - 0.3))
            s = d * 0.045
            baum.flaeche([k0 - s, k0 + s, k0 + s + Vector((0, 0, 0.26)), k0 - s + Vector((0, 0, 0.26))],
                         [(fu, fv)] * 4, [kolben] * 4, [OBEN] * 4)
    return baum.fertig()


def farn(seed=1, name="Farn"):
    """Farn: 8–11 Wedel aus dem Boden, bogig nach außen hängend (Wedel-Karten aus der Atlas-Textur)."""
    from baeume import WEDEL
    baum = Baum(name, seed, hoehe=1.0)
    r = baum.rng
    gruen = farbe(r.choice(["#3F7A2A", "#4A8A30", "#2F6B28"]))
    u0, v0, u1, v1 = WEDEL
    vm = (v0 + v1) / 2
    wedel = r.randint(8, 11)
    for k in range(wedel):
        a = math.tau * k / wedel + r.uniform(-0.2, 0.2)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        laenge = r.uniform(0.85, 1.25)
        d = (aussen * 0.3 + Vector((0, 0, 1))).normalized()
        segmente = 6
        rippe = [Vector((0, 0, 0))]
        for _ in range(segmente):
            d = (d + aussen * 0.22 + Vector((0, 0, -0.13))).normalized()
            rippe.append(rippe[-1] + d * laenge / segmente)
        seite = aussen.cross(OBEN).normalized()
        n = OBEN + aussen * 0.3
        for i in range(segmente):
            s0, s1 = i / segmente, (i + 1) / segmente
            b0 = 0.22 * math.sin(math.pi * (0.12 + 0.85 * s0)) ** 0.7 + 0.02
            b1 = 0.22 * math.sin(math.pi * (0.12 + 0.85 * s1)) ** 0.7 + 0.02
            m0, m1 = rippe[i], rippe[i + 1]
            l0, l1, r0, r1 = m0 - seite * b0, m1 - seite * b1, m0 + seite * b0, m1 + seite * b1
            ua, ub = u0 + (u1 - u0) * s0, u0 + (u1 - u0) * s1
            c0, c1 = gruen * (0.55 + 0.6 * s0), gruen * (0.55 + 0.6 * s1)
            baum.flaeche([l0, l1, m1, m0], [(ua, v0), (ub, v0), (ub, vm), (ua, vm)], [c0 * 0.9, c1 * 0.9, c1, c0], [n] * 4)
            baum.flaeche([m0, m1, r1, r0], [(ua, vm), (ub, vm), (ub, v1), (ua, v1)], [c0, c1, c1 * 0.9, c0 * 0.9], [n] * 4)
    return baum.fertig()


def efeu(seed=1, name="Efeu", breite=3.0, hoehe=2.5):
    """Efeuvorhang für Felswände: Blattkarten auf einer senkrechten Fläche (entlang X, nach −Y
    schauend), unten dichter, oben in Ranken auslaufend."""
    from baeume import LAUB
    baum = Baum(name, seed, hoehe=hoehe)
    r = baum.rng
    u0, v0, u1, v1 = LAUB
    ranken = int(breite / 0.22)
    for k in range(ranken):
        x = -breite / 2 + (k + r.uniform(0.2, 0.8)) * breite / ranken
        oben = hoehe * r.uniform(0.35, 1.0)
        z = 0.0
        while z < oben:
            g = r.uniform(0.3, 0.44) * (1.0 - z / hoehe * 0.35)
            mitte = Vector((x + r.uniform(-0.08, 0.08), -r.uniform(0.02, 0.1), z))
            c = farbe(r.choice(["#2F5E22", "#3E7428", "#4C8430"])) * r.uniform(0.85, 1.1)
            n = Vector((0, -1, 0.4)).normalized()
            q = [mitte + Vector((-g, 0, 0)), mitte + Vector((g, 0, 0)), mitte + Vector((g, -0.05, g * 1.6)), mitte + Vector((-g, -0.05, g * 1.6))]
            baum.flaeche(q, [(u0, v0), (u1, v0), (u1, v1), (u0, v1)], [c * 0.8, c * 0.8, c, c], [n] * 4)
            z += g * r.uniform(0.55, 0.85)
    return baum.fertig()
