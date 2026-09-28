"""Kristallmagier (NPC am Kristallturm) – Baukasten der Arbeiter (arbeiter.py).

Ein alter Magier in tiefblauer Robe mit silbernen Sternzeichen, hellblauem Schulterumhang mit
Silberborte und zurückgeschlagener Kapuze, Stirnreif mit Kristall, langer weißer Bart. In der
Rechten ein Eichenstab, dessen Krone einen Kristall umschließt. Er schwebt eine Handbreit über
dem Boden: gleitet zu den Kristallvorkommen, löst die Kristalle mit einem Zauber und trägt sie,
zwischen den Händen schwebend, zum Turm.

Anbauteile wie bei den Arbeitern: „Werkzeug“ (Stab in der Hand), „Werkzeug_Ruecken“ (Stab auf
dem Rücken) und „Last“ (die schwebenden Kristalle).
Animationen: Idle, Laufen (Gleiten), Arbeiten (Zauber in Schleife), Buecken, Tragen.
"""

import math

from mathutils import Matrix, Vector

from arbeiter import (GRIFF_R, KOPF_GEWICHT, BRUST_GEWICHT, X, Y, Z, _huefte_bein_schuerze, _ik_backen, _kopf, _koerper, _rumpf, _ruecken_matrix,
                      _skelett, _spiegel, _treffpunkt)
from figuren import Figur, _clip, _mit, _schale, _schleife, farbe, weich
from werkstatt import animation

SCHWEBEN = 0.2


def _stab(f, m, gewicht):
    """Eichenstab: Fuß unten am Boden, oben eine Krone aus drei Holzklauen um einen Kristall."""
    holz, holz_dunkel = farbe("#7A5232"), farbe("#553820")
    silber = farbe("#D8DEE8")
    kristall, kristall_hell = farbe("#3C9CFF"), farbe("#D2F1FF")
    mr = m.to_3x3()
    x, y = GRIFF_R.x, GRIFF_R.y
    ringe = []
    for i in range(16):
        t = i / 15
        z = 0.06 + (1.72 - 0.06) * t
        r = 0.02 + 0.004 * math.sin(t * 17) + 0.006 * t
        ringe.append((m @ Vector((x + 0.006 * math.sin(t * 9), y, z)), mr @ X, mr @ Y, r, r * 0.92))
    f.loft("Stab", ringe, 8, lambda i, k, p: holz_dunkel if (k + int(i * 1.3)) % 5 == 0 else holz, gewicht, oben_zu=True, unten_zu=True, teilung=1, glatt=True)
    for z in (0.98, 1.02):
        f.loft("Stabring", [(m @ Vector((x, y, z)), mr @ X, mr @ Y, 0.026, 0.026), (m @ Vector((x, y, z + 0.018)), mr @ X, mr @ Y, 0.026, 0.026)], 10,
               lambda i, k, p: silber, gewicht)
    krone = Vector((x, y, 1.8))
    for k in range(3):
        w = math.tau * k / 3
        aussen = Vector((math.cos(w) * 0.06, math.sin(w) * 0.06, 0))
        punkte = [m @ (krone + Vector((0, 0, -0.1)) + aussen * 0.3), m @ (krone + aussen * 1.2 + Vector((0, 0, -0.02))), m @ (krone + aussen * 0.5 + Vector((0, 0, 0.1)))]
        f.straehne("Klaue", punkte, 0.022, 0.006, holz, gewicht, 6, 0.0, 1.0)
    spitze = [(m @ (krone + Vector((0, 0, dz))), mr @ X, mr @ Y, rad, rad) for dz, rad in ((-0.07, 0.004), (-0.03, 0.04), (0.05, 0.045), (0.14, 0.004))]
    f.loft("Stabkristall", spitze, 6, lambda i, k, p: kristall_hell if k % 2 else kristall, gewicht, oben_zu=True, unten_zu=True)


def _kristalle_schwebend(f, mitte, gewicht):
    """Drei Kristalle, die zwischen den Händen schweben (die Ladung)."""
    kristall, hell, dunkel = farbe("#3C9CFF"), farbe("#D2F1FF"), farbe("#1D4FB8")
    for k, (versatz, laenge, neigung) in enumerate(((Vector((0, 0, 0)), 0.22, 0.0), (Vector((0.07, 0.02, -0.04)), 0.15, 0.5), (Vector((-0.07, -0.01, -0.03)), 0.13, -0.6))):
        achse = Vector((math.sin(neigung), 0, math.cos(neigung)))
        q = achse.cross(Y).normalized()
        q2 = achse.cross(q).normalized()
        basis = mitte + versatz - achse * laenge * 0.5
        ringe = [(basis + achse * laenge * t, q, q2, rad, rad) for t, rad in ((0.0, 0.004), (0.18, 0.035), (0.75, 0.035), (1.0, 0.003))]
        f.loft("Schwebekristall", ringe, 6, lambda i, kk, p: hell if kk % 3 == 0 else (kristall if kk % 3 == 1 else dunkel), gewicht, oben_zu=True, unten_zu=True)


def kristallmagier(seed=75, name="Kristallmagier"):
    f = Figur(name, seed)
    haut, lippe, wange = farbe("#E4B292"), farbe("#B8806E"), farbe("#E0987E")
    bart = farbe("#E4E2DC")
    robe, robe_dunkel = farbe("#23346E"), farbe("#17224A")
    umhang, umhang_dunkel = farbe("#5E8CC8"), farbe("#3E6498")
    silber, silber_dunkel = farbe("#DCE2EC"), farbe("#8C96A8")
    kristall = farbe("#3C9CFF")

    def stern(p):
        # Silberne Sternzeichen, locker über die Robe verteilt
        return math.sin(p.center.x * 60 + p.center.z * 23) * math.sin(p.center.z * 41 - p.center.y * 50) > 0.93

    def robe_farbe(p):
        if stern(p):
            return silber
        return robe * (0.9 + 0.14 * max(0.0, -p.normal.y))

    def hose(p, s):
        return robe_dunkel

    # Körper wie bei den Arbeitern (Beine verschwinden unter der Robe); weite Ärmel in Robenblau
    _koerper(f, haut, robe_dunkel, hose, robe_farbe, farbe("#2A2230"), farbe("#141018"), haut, robe, lambda p: robe * (0.9 + 0.1 * max(0.0, p.normal.z)), False,
             hand_ziel=1400)
    # Weite Trichterärmel ab dem Ellbogen
    for s in (1, -1):
        sn = "L" if s > 0 else "R"
        e_, h_ = _spiegel(Vector((0.27, 0.02, 1.2)), s), _spiegel(Vector((0.315, -0.01, 0.96)), s)
        ach = (h_ - e_).normalized()
        q1 = ach.cross(Y).normalized()
        q2 = ach.cross(q1).normalized()
        ringe = [(e_.lerp(h_, t), q1, q2, rad, rad) for t, rad in ((0.1, 0.056), (0.5, 0.075), (0.85, 0.1), (0.95, 0.105))]
        f.loft("Trichteraermel", ringe, 18, lambda i, k, p: silber if i > 2.6 else robe * (0.88 + 0.12 * (k % 2)),
               lambda co, sn=sn: {f"Unterarm.{sn}": 1.0}, teilung=2, glatt=True)

    # Robe: vom Gürtel bis knapp über den Boden, unten weit, mit Silbersaum; die Beine
    # bewegen sich im Schweben kaum, der Saum schwingt mit
    ringe = []
    for t, (z, rx, ry) in enumerate(((1.05, 0.19, 0.145), (0.9, 0.215, 0.165), (0.7, 0.245, 0.19), (0.45, 0.28, 0.22), (0.2, 0.32, 0.26), (0.06, 0.34, 0.28), (0.03, 0.33, 0.27))):
        ringe.append((Vector((0, 0.01, z)), X, Y, rx, ry, lambda w, t=t: 1.0 + 0.045 * t / 6 * math.sin(w * 9 + t)))

    def rock(i, k, p):
        if i > 4.6:
            return silber if k % 4 else silber_dunkel
        if abs(p.center.x) < 0.02 and p.normal.y < 0:
            return silber_dunkel
        return robe_farbe(p)
    f.loft("Robe", ringe, 44, rock, lambda co: _mischen_robe(co), teilung=3, glatt=True)
    # Schärpe mit Kristallbeutel
    f.loft("Schaerpe", [(Vector((0, 0.008, 1.0)), X, Y, 0.196, 0.15), (Vector((0, 0.008, 1.06)), X, Y, 0.192, 0.148)], 40, lambda i, k, p: silber_dunkel, _rumpf, teilung=2)
    f.kugel("Guertelschliesse", Vector((0, -0.155, 1.03)), (0.03, 0.012, 0.03), kristall, _rumpf, 8, 6)
    f.metaball("Beutel", [(Vector((0.19, -0.06, 0.93)), (0.05, 0.04, 0.06)), (Vector((0.19, -0.06, 0.99)), (0.025, 0.022, 0.025))], 0.006, 500,
               lambda poly: farbe("#6A4A34") * (0.85 + 0.2 * max(0.0, poly.normal.z)), _rumpf, glatt=True)

    # Schulterumhang mit Silberborte, dahinter die zurückgeschlagene Kapuze
    kragen = _schale(f, "Schulterumhang", Vector((0, 0.01, 1.43)), 0.25, 0.18, 0.13, (-180, 180), (-60, 42),
                     lambda poly: silber if poly.center.z < 1.34 else umhang * (0.85 + 0.2 * max(0.0, poly.normal.z)),
                     lambda co: _mischen_umhang(co), seg=(44, 10))
    kapuze = _schale(f, "Kapuze", Vector((0, 0.1, 1.5)), 0.13, 0.1, 0.13, (110, 250), (-60, 70),
                     lambda poly: umhang_dunkel if poly.normal.y < 0 else umhang, lambda co: {"Brust": 1.0}, seg=(20, 10))
    for v in kapuze.data.vertices:
        v.co.y += 0.04 * max(0.0, v.co.z - 1.45)
    # Silberne Brosche
    f.stern("Brosche", Vector((0, -0.2, 1.43)), Vector((0, -1, 0.2)), 0.03, silber, _rumpf, zacken=6)

    _kopf(f, haut, lippe, wange, bart, "voll")
    # Langer Bart bis auf die Brust
    for n in range(10):
        w = (n / 9 - 0.5) * 2.2
        start = Vector((math.sin(w) * 0.06, -0.11 - math.cos(w) * 0.01, 1.58))
        ende = start + Vector((math.sin(w) * 0.02, -0.06, -0.24 - 0.05 * math.cos(w)))
        f.straehne("Bartstraehne", [start, start.lerp(ende, 0.5) + Vector((0, -0.03, 0)), ende], 0.028, 0.004, bart * (0.92 + 0.08 * (n % 2)),
                   lambda co: _mischen(("Kopf", weich(1.45, 1.6, co.z)), ("Brust", 1 - weich(1.45, 1.6, co.z))), 7, 0.4, 0.6, glatt=True)
    # Silbernes Haar hinten, Stirnreif mit Kristall
    f.metaball("Haar", [(Vector((0, 0.04, 1.72)), (0.108, 0.1, 0.1)), (Vector((0, 0.08, 1.62)), (0.09, 0.06, 0.09))], 0.008, 800,
               lambda poly: bart * (0.9 + 0.1 * (poly.index % 3) / 2), KOPF_GEWICHT)
    f.loft("Stirnreif", [(Vector((0, 0.005, 1.745)), X, Y, 0.103, 0.114), (Vector((0, 0.005, 1.76)), X, Y, 0.104, 0.115)], 32, lambda i, k, p: silber, KOPF_GEWICHT)
    f.loft("Stirnkristall", [(Vector((0, -0.118, 1.752 + dz)), X, Y, rad, rad * 0.5) for dz, rad in ((-0.02, 0.003), (-0.005, 0.014), (0.012, 0.012), (0.03, 0.002))], 6,
           lambda i, k, p: kristall if k % 2 else farbe("#D2F1FF"), KOPF_GEWICHT, oben_zu=True, unten_zu=True)

    # Ladung: drei Kristalle schweben vor der Brust
    anfang = len(f.teile)
    _kristalle_schwebend(f, Vector((0, -0.34, 1.14)), BRUST_GEWICHT)
    f.als_starr("Last", "Brust", anfang)

    # Stab in der Rechten, beim Tragen quer auf dem Rücken
    hand = lambda co: {"Hand.R": 1.0}
    anfang = len(f.teile)
    _stab(f, Matrix.Identity(4), hand)
    f.als_starr("Werkzeug", "Hand.R", anfang)
    anfang = len(f.teile)
    _stab(f, _ruecken_matrix(Vector((0.08, 0.22, 1.05)), Vector((-0.3, 0.05, -0.95))) @ Matrix.Translation(Vector((0, 0, 0.0))), BRUST_GEWICHT)
    f.als_starr("Werkzeug_Ruecken", "Brust", anfang)
    _skelett(f)
    return f.fertig(_animationen)


def _mischen(*paare):
    from figuren import _mischen as m
    return m(*paare)


def _mischen_robe(co):
    """Oben am Rumpf, unten schwingt die Robe ein wenig mit den Beinen."""
    if co.z > 1.0:
        return _rumpf(co)
    bein = weich(1.0, 0.3, co.z) * 0.35
    links = weich(-0.12, 0.12, co.x)
    return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))


def _mischen_umhang(co):
    seite = weich(0.16, 0.27, abs(co.x)) * 0.45
    return _mischen(("Brust", 1.0 - seite), ("Oberarm.L" if co.x > 0 else "Oberarm.R", seite))


def _animationen(armatur):
    # Beine hängen im Schweben locker herab, die Füße zeigen etwas nach unten
    haengen = lambda phi: [("Oberschenkel.L", "rot", (-6 + 3 * math.sin(phi), 0, 0)), ("Oberschenkel.R", "rot", (4 + 3 * math.sin(phi + 1), 0, 0)),
                           ("Unterschenkel.L", "rot", (14, 0, 0)), ("Unterschenkel.R", "rot", (10, 0, 0)),
                           ("Fuss.L", "rot", (22, 0, 0)), ("Fuss.R", "rot", (22, 0, 0))]

    # Idle: schwebt auf und ab, der Stab ruht neben ihm, die Linke hält einen kleinen Zauber
    def idle(phi):
        heben = SCHWEBEN + 0.04 * math.sin(phi * 2)
        return haengen(phi * 2) + [
            ("Becken", "pos", (0, heben, 0)), ("Brust", "rot", (1.5 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (3 * math.sin(phi * 2 + 1), 18 * math.sin(phi), 0)),
            ("Oberarm.L", "rot", (-22, 0, 10)), ("Unterarm.L", "rot", (-60 - 6 * math.sin(phi * 2), 0, 0)), ("Hand.L", "rot", (-20, 0, 0)),
            ("Oberarm.R", "rot", (-8, 0, 0)), ("Unterarm.R", "rot", (-10, 0, 0)),
        ]
    animation(armatur, "Idle", 120, _schleife(120, 6, idle))

    # Laufen: gleitet vorgebeugt, die Beine schleifen nach hinten, der Stab zeigt voraus
    def gleiten(phi):
        return [("Oberschenkel.L", "rot", (4 + 2 * math.sin(phi), 0, 0)), ("Oberschenkel.R", "rot", (7 + 2 * math.sin(phi + 1), 0, 0)),
                ("Unterschenkel.L", "rot", (14, 0, 0)), ("Unterschenkel.R", "rot", (16, 0, 0)), ("Fuss.L", "rot", (28, 0, 0)), ("Fuss.R", "rot", (28, 0, 0)),
                ("Becken", "pos", (0, SCHWEBEN + 0.05 + 0.03 * math.sin(phi * 2), 0)), ("Becken", "rot", (6, 0, 0)),
                ("Brust", "rot", (8, 4 * math.sin(phi), 0)), ("Kopf", "rot", (-10, 0, 0)),
                ("Oberarm.R", "rot", (-18, 0, 0)), ("Unterarm.R", "rot", (-24, 0, 0)), ("Hand.R", "rot", (30, 0, 0)),
                ("Oberarm.L", "rot", (18 + 4 * math.sin(phi), 0, 8)), ("Unterarm.L", "rot", (-18, 0, 0))]
    animation(armatur, "Laufen", 40, _schleife(40, 4, gleiten))

    # Tragen: gleitet langsamer, die Kristalle schweben zwischen den Händen (IK)
    def tragen(phi):
        return [w for w in gleiten(phi) if "arm" not in w[0] and w[0] != "Hand.R"] + [
            ("Oberarm.L", "rot", (-30, 0, 0)), ("Unterarm.L", "rot", (-70, 0, 0)), ("Oberarm.R", "rot", (-30, 0, 0)), ("Unterarm.R", "rot", (-70, 0, 0))]
    animation(armatur, "Tragen", 36, _schleife(36, 3, tragen))
    pol_l, pol_r = Vector((0.55, 0.35, 1.05)), Vector((-0.55, 0.35, 1.05))
    _ik_backen(armatur, 36, [("L", "Brust", Vector((0.13, -0.3, 1.14)), pol_l), ("R", "Brust", Vector((-0.13, -0.3, 1.14)), pol_r)])

    # Bücken: sinkt zu Boden, verneigt sich und rafft mit beiden Händen (Aufheben/Abladen)
    oben = {"Becken.pos": (0, 0, SCHWEBEN), "Oberarm.L": (-10, 0, 0), "Unterarm.L": (-20, 0, 0), "Oberarm.R": (-8, 0, 0), "Unterarm.R": (-12, 0, 0),
            "Oberschenkel.L": (-6, 0, 0), "Oberschenkel.R": (4, 0, 0), "Unterschenkel.L": (14, 0, 0), "Unterschenkel.R": (10, 0, 0), "Fuss.L": (22, 0, 0), "Fuss.R": (22, 0, 0)}
    unten = _mit(oben, **{"Becken.pos": (0, 0, 0.02)}, Bauch=(16, 0, 0), Brust=(22, 0, 0), Kopf=(-10, 0, 0), Oberarm_L=(-60, 0, 10), Unterarm_L=(-30, 0, 0),
                 Oberarm_R=(-55, 0, -10), Unterarm_R=(-30, 0, 0))
    _clip(armatur, "Buecken", 40, [(0, oben), (14, unten), (26, unten), (40, oben)])

    # Arbeiten: hebt den Stab gegen das Vorkommen, die Linke lenkt den Zauber, Stoß bei Bild 30
    bereit = dict(oben, **{"Brust": (4, 0, 0), "Oberarm.R": (-62, 0, 0), "Unterarm.R": (-22, 0, 0), "Hand.R": (95, 0, 0),
                           "Oberarm.L": (-70, 0, 12), "Unterarm.L": (-20, 0, 0), "Hand.L": (-30, 0, 0)})
    sammeln = _mit(bereit, Brust=(-6, 0, 0), Kopf=(-6, 0, 0), Oberarm_R=(-80, 0, 0), Oberarm_L=(-40, 0, 20), Unterarm_L=(-60, 0, 0))
    stoss = _mit(bereit, Brust=(10, 0, 0), Kopf=(4, 0, 0), Oberarm_R=(-72, 0, 0), Unterarm_R=(-8, 0, 0), Oberarm_L=(-86, 0, 6), Unterarm_L=(-6, 0, 0),
                 **{"Becken.pos": (0, 0, SCHWEBEN + 0.06)})
    _clip(armatur, "Arbeiten", 48, [(0, bereit), (16, sammeln), (26, _mit(sammeln, Oberarm_R=(-86, 0, 0))), (30, stoss), (38, _mit(stoss, Brust=(6, 0, 0))),
                                    (48, bereit)])
    treff = _treffpunkt(armatur, 30, Vector((GRIFF_R.x, GRIFF_R.y, 1.8)))
    print(f"TREFFPUNKT kristall: vorne {-treff.y:.2f} m, seitlich {treff.x:.2f} m, Höhe {treff.z:.2f} m")
