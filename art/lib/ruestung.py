"""Rüstungsteile (siehe game/src/ruestung.rs): je Klasse Kopf, Brust und Füße in zwei Ausführungen.

Jedes Teil steht aufrecht wie auf einem Ausstellungsständer (Ursprung unten in der Mitte, etwa
0,3–0,6 m groß), damit es am Boden liegend und als Symbol gut zu erkennen ist. Gebaut mit den
Bausteinen der Figuren (Lofts, Kugeln, Strähnen, Platten, Sterne); Farben als Vertexfarben.
Aufruf über art/lib/beute.py (modell("<datei>")).
"""

import math

from mathutils import Vector

from figuren import _glocke, _platte, _schale, farbe

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))
OHNE = lambda co: {}


def _f(hexwert):
    return farbe(hexwert)


# ---------------------------------------------------------------------------
# Bausteine
# ---------------------------------------------------------------------------
def _spitzhut(f, haupt, band, zier, hoehe=0.42, knick=0.12, krempe=0.26, sterne=False):
    """Zauberhut: breite Krempe, Kegel mit geknickter Spitze, Hutband."""
    f.loft("Krempe", [(Vector((0, 0, 0.02)), X, Y, 0.11, 0.11), (Vector((0, 0, 0.015)), X, Y, krempe, krempe * 0.96),
                      (Vector((0, 0, 0.0)), X, Y, krempe * 1.02, krempe * 0.98), (Vector((0, 0, 0.03)), X, Y, 0.11, 0.11)], 36,
           lambda i, k, p: haupt * (0.8 if 1 <= i < 2 else 0.95), OHNE, teilung=2, glatt=True)
    ringe = []
    for j in range(14):
        t = j / 13
        biege = max(0.0, t - 0.55) / 0.45
        mitte = Vector((0, knick * biege ** 1.6, 0.02 + hoehe * t - 0.06 * biege ** 2))
        r = 0.115 * (1 - t) ** 0.9 + 0.004
        ringe.append((mitte, X, Y, r, r * 1.02, lambda w, t=t: 1.0 + 0.05 * math.sin(w * 3 + t * 6)))

    def hut_farbe(i, k, p):
        if i < 1.0:
            return band
        if sterne and (k + int(i * 3)) % 9 == 0:
            return zier
        return haupt * (0.9 + 0.1 * max(0.0, p.normal.z))
    f.loft("Hut", ringe, 28, hut_farbe, OHNE, oben_zu=True, teilung=2, glatt=True)
    if sterne:
        for j, (w, h) in enumerate(((0.3, 0.12), (2.1, 0.2), (4.0, 0.1), (5.2, 0.26))):
            r = 0.115 * (1 - h / hoehe) + 0.01
            f.stern("Stern", Vector((math.sin(w) * r, -math.cos(w) * r, h)), Vector((math.sin(w), -math.cos(w), 0.2)), 0.03, zier, OHNE, zacken=5)


def _helm(f, stahl, hell, rand, hoerner=None, kamm=None, rune=None):
    """Zwergenhelm: Kuppel, Goldrand, Nasenschutz; wahlweise Hörner, Kamm und Rune."""
    ringe = [(Vector((0, 0, h)), X, Y, rx, ry) for h, rx, ry in ((0.0, 0.13, 0.14), (0.015, 0.135, 0.145), (0.07, 0.13, 0.14), (0.13, 0.11, 0.12),
                                                                 (0.18, 0.075, 0.08), (0.205, 0.03, 0.03), (0.21, 0.005, 0.005))]
    f.loft("Helm", ringe, 36, lambda i, k, p: rand if i < 1.0 else stahl.lerp(hell, max(0.0, p.normal.z) * 0.7), OHNE, oben_zu=True, teilung=3, glatt=True)
    _platte(f, "Nasenschutz", Vector((0, -0.145, 0.0)), 0.12, 0.02, 0.012, -Z, -Y, lambda i, k, p: hell, OHNE, spitz=0.5, wolbung=0.4)
    for k in range(10):
        w = math.tau * k / 10
        f.kugel("Niete", Vector((math.cos(w) * 0.137, math.sin(w) * 0.147, 0.012)), (0.009, 0.009, 0.009), rand, OHNE, 8, 4)
    if kamm:
        kammringe = []
        for j in range(9):
            a = math.radians(-70 + 150 * j / 8)
            kammringe.append((Vector((0, math.sin(a) * 0.14, 0.02 + math.cos(a) * 0.19)), Y, Vector((0, math.sin(a), math.cos(a))), 0.012,
                              0.03 * math.sin(math.pi * (j + 0.5) / 9) + 0.008))
        f.loft("Kamm", kammringe, 8, lambda i, k, p: kamm, OHNE, oben_zu=True, unten_zu=True, teilung=2)
    if hoerner:
        for s in (1, -1):
            w0 = Vector((0.12 * s, 0.0, 0.09))
            punkte = [w0, w0 + Vector((0.08 * s, -0.01, 0.03)), w0 + Vector((0.14 * s, -0.03, 0.1)), w0 + Vector((0.15 * s, -0.05, 0.19))]
            f.straehne("Horn", punkte, 0.035, 0.004, hoerner, OHNE, 12, 0.0, 1.0, glatt=True)
    if rune:
        f.stern("Rune", Vector((0, -0.14, 0.09)), Vector((0, -1, 0.3)), 0.035, rune, OHNE, zacken=4)


def _schuh(f, x, haupt, sohle, schaft=0.0, kappe=None, zier=None, stulpe=None):
    """Ein Schuh bzw. Stiefel (Spitze nach -Y). `schaft`: Höhe des Schafts über dem Knöchel."""
    fuss = [(Vector((x, 0.07, 0.05)), X, Z, 0.05, 0.05), (Vector((x, 0.05, 0.02)), X, Z, 0.056, 0.03), (Vector((x, -0.02, 0.05)), X, Z, 0.058, 0.055),
            (Vector((x, -0.09, 0.045)), X, Z, 0.052, 0.042), (Vector((x, -0.14, 0.04)), X, Z, 0.036, 0.032), (Vector((x, -0.165, 0.04)), X, Z, 0.006, 0.006)]
    f.loft("Schuh", fuss, 16, lambda i, k, p: sohle if p.center.z < 0.016 else (kappe if kappe and p.center.y < -0.1 else haupt), OHNE, oben_zu=True, unten_zu=True,
           teilung=3, glatt=True)
    if schaft > 0:
        ringe = [(Vector((x, 0.035, 0.06 + schaft * t)), X, Y, 0.052 + 0.01 * t, 0.055 + 0.01 * t) for t in (0.0, 0.5, 1.0)]
        f.loft("Schaft", ringe, 18, lambda i, k, p: haupt * (0.9 if k % 6 == 0 else 1.0), OHNE, teilung=2, glatt=True)
        if stulpe:
            oben = 0.06 + schaft
            f.loft("Stulpe", [(Vector((x, 0.035, oben - 0.04)), X, Y, 0.066, 0.069), (Vector((x, 0.035, oben + 0.01)), X, Y, 0.072, 0.075)], 18,
                   lambda i, k, p: stulpe, OHNE, oben_zu=False, unten_zu=False, teilung=2, glatt=True)
    if zier:
        f.stern("Zier", Vector((x + 0.058 * (1 if x > 0 else -1), 0.0, 0.07 + schaft * 0.4)), Vector((1 if x > 0 else -1, 0, 0)), 0.022, zier, OHNE, zacken=4)


def _paar(f, **kw):
    _schuh(f, 0.075, **kw)
    _schuh(f, -0.075, **kw)


def _torso(f, farbe_von, breit=1.0, rock=0.0):
    """Oberkörper wie auf einer Schneiderpuppe (Hals bis Hüfte, 0,55 m), optional mit Rock."""
    ringe = []
    for z, rx, ry in ((0.0, 0.16, 0.12), (0.12, 0.155, 0.115), (0.24, 0.15, 0.11), (0.34, 0.17, 0.12), (0.44, 0.19, 0.12), (0.5, 0.16, 0.1),
                      (0.54, 0.07, 0.06)):
        ringe.append((Vector((0, 0, z + rock)), X, Y, rx * breit, ry * breit))
    f.loft("Torso", ringe, 36, farbe_von, OHNE, oben_zu=True, unten_zu=True, teilung=2, glatt=True)
    if rock > 0:
        glocke = [(Vector((0, 0, rock + 0.02)), 0.165 * breit, 0.125 * breit), (Vector((0, 0, rock * 0.5)), 0.2 * breit, 0.15 * breit),
                  (Vector((0, 0, 0.0)), 0.23 * breit, 0.17 * breit)]
        _glocke(f, "Rock", glocke, (0, 360), lambda poly: farbe_von(0, 0, poly), OHNE, seg=36, welle=0.05)


def _schulterschale(f, x, z, groesse, farbe_von):
    f.kugel("Schulter", Vector((x, 0, z)), (0.08 * groesse, 0.09 * groesse, 0.055 * groesse), farbe_von, OHNE, 18, 10, glatt=True)


# ---------------------------------------------------------------------------
# Die 24 Teile
# ---------------------------------------------------------------------------
def bauen(f, art):
    if art == "hut_lehrling":
        _spitzhut(f, _f("#3E5FA8"), _f("#6B4A2A"), _f("#E8D48A"), hoehe=0.36, knick=0.08, krempe=0.22)
    elif art == "hut_sterne":
        _spitzhut(f, _f("#3A2A6E"), _f("#D8AE4A"), _f("#E6E8F2"), hoehe=0.48, knick=0.16, krempe=0.28, sterne=True)
    elif art == "helm_eisen":
        _helm(f, _f("#7A8088"), _f("#A8AEB6"), _f("#6A6560"))
    elif art == "helm_runen":
        _helm(f, _f("#6E7886"), _f("#A9B3C0"), _f("#D8AE4A"), hoerner=_f("#E8DCC0"), kamm=_f("#D8AE4A"), rune=_f("#6FC8FF"))
    elif art == "kappe_jaeger":
        _schale(f, "Kappe", Vector((0, 0, 0.0)), 0.12, 0.13, 0.11, (0, 360), (0, 90), lambda poly: _f("#4E6A34") * (0.9 + 0.1 * poly.normal.z), OHNE)
        _platte(f, "Schirm", Vector((0, -0.15, 0.01)), 0.1, 0.1, 0.008, -Y, Z * 1 + Y * 0.3, lambda i, k, p: _f("#3A4E26"), OHNE, spitz=0.2, wolbung=0.3)
        f.straehne("Feder", [Vector((0.1, 0.02, 0.06)), Vector((0.14, 0.08, 0.16)), Vector((0.15, 0.16, 0.26))], 0.03, 0.004, _f("#C8322A"), OHNE, 8, 0.0, 0.2)
    elif art == "krone_mond":
        silber, stein = _f("#E6E8F2"), _f("#8FE0FF")
        f.loft("Reif", [(Vector((0, 0, 0.0)), X, Y, 0.11, 0.12), (Vector((0, 0, 0.025)), X, Y, 0.112, 0.122)], 36, lambda i, k, p: silber, OHNE,
               oben_zu=False, unten_zu=False, teilung=1, glatt=True)
        for s in (1, -1):
            _platte(f, "Sichel", Vector((0.04 * s, -0.12, 0.07)), 0.12, 0.025, 0.008, Vector((s * 0.6, 0, 1)), -Y, lambda i, k, p: silber, OHNE, spitz=0.9, wolbung=0.8)
        f.kugel("Mondstein", Vector((0, -0.125, 0.035)), (0.02, 0.012, 0.024), stein, OHNE, 12, 8)
        for k in range(6):
            w = math.tau * k / 6
            f.kugel("Perle", Vector((math.sin(w) * 0.114, -math.cos(w) * 0.124, 0.03)), (0.008, 0.008, 0.008), stein, OHNE, 8, 4)
    elif art in ("kapuze", "maske_schatten"):
        stoff = _f("#5A4632") if art == "kapuze" else _f("#221E2A")
        # Getragen (anlegen.py) reicht die Kapuze an den Seiten bis zum Kinn und lässt das Gesicht frei
        offen, tief = ((48, 312), -55) if getattr(f, "angelegt", False) else ((35, 325), -10)
        _schale(f, "Kapuze", Vector((0, 0.01, 0.0)), 0.14, 0.15, 0.2, offen, (tief, 90), lambda poly: stoff * (0.85 + 0.2 * max(0.0, poly.normal.z)), OHNE)
        _schale(f, "Kapuze innen", Vector((0, 0.01, 0.0)), 0.135, 0.145, 0.195, offen, (tief, 90), lambda poly: stoff * 0.4, OHNE, innen=True)
        _glocke(f, "Kragen", [(Vector((0, 0.01, 0.0)), 0.14, 0.15), (Vector((0, 0.02, -0.08)), 0.2, 0.18)], (0, 360), lambda poly: stoff * 0.9, OHNE, seg=30)
        if art == "maske_schatten":
            # Getragen liegt die Maske dicht vor dem Gesicht (anlegen.py)
            vorn, hoehe = (-0.108, 0.015) if getattr(f, "angelegt", False) else (-0.14, 0.06)
            if getattr(f, "angelegt", False):
                # quer über die Augenpartie, unten spitz
                _platte(f, "Maske", Vector((0, vorn, hoehe - 0.01)), 0.09, 0.075, 0.012, Z, -Y, lambda i, k, p: _f("#101014"), OHNE, spitz=0.35, wolbung=1.2)
            else:
                _platte(f, "Maske", Vector((0, vorn, hoehe)), 0.11, 0.1, 0.012, X, -Y, lambda i, k, p: _f("#101014"), OHNE, spitz=0.1, wolbung=1.2)
            for s in (1, -1):
                f.kugel("Auge", Vector((0.035 * s, vorn - 0.015, hoehe + 0.005)), (0.016, 0.006, 0.008), _f("#B070FF"), OHNE, 10, 6)
    elif art in ("robe_adept", "robe_erzmagier"):
        haupt, zier = (_f("#3E5FA8"), _f("#E8D48A")) if art == "robe_adept" else (_f("#2A1E4E"), _f("#D8AE4A"))

        def robe(i, k, p):
            z = p.center.z
            if abs(p.center.x) < 0.02 and p.normal.y < -0.3:
                return zier
            if z < 0.03 or (art == "robe_erzmagier" and (int(z * 30) + k) % 11 == 0):
                return zier
            return haupt * (0.9 + 0.1 * max(0.0, -p.normal.y))
        _torso(f, robe, rock=0.25)
        if art == "robe_erzmagier":
            _glocke(f, "Kragen", [(Vector((0, 0.01, 0.8)), 0.1, 0.08), (Vector((0, 0.02, 0.9)), 0.17, 0.14)], (60, 300), lambda poly: haupt * 0.8, OHNE, seg=24)
            for z in (0.5, 0.65):
                f.stern("Stern", Vector((0.1, -0.13, z)), -Y, 0.03, zier, OHNE, zacken=5)
    elif art in ("brust_kette", "brust_ahnen"):
        stahl, gold = _f("#7A8088"), _f("#D8AE4A")
        if art == "brust_kette":
            _torso(f, lambda i, k, p: stahl * (0.75 if (k + int(p.center.z * 60)) % 2 else 1.0), breit=1.05)
        else:
            _torso(f, lambda i, k, p: gold if p.center.z < 0.03 or abs(p.center.x) < 0.015 else _f("#8A94A2") * (0.85 + 0.2 * max(0.0, -p.normal.y)), breit=1.1)
            for s in (1, -1):
                _schulterschale(f, 0.2 * s, 0.48, 1.3, lambda poly: gold if poly.normal.z > 0.6 else _f("#A9B3C0"))
            f.stern("Rune", Vector((0, -0.135, 0.36)), -Y, 0.05, _f("#6FC8FF"), OHNE, zacken=4)
    elif art in ("wams_leder", "harnisch_nachtwind"):
        if art == "wams_leder":
            leder = _f("#7A4A2E")
            _torso(f, lambda i, k, p: _f("#D8C8A0") if abs(p.center.x) < 0.01 and p.normal.y < -0.4 and int(p.center.z * 30) % 2 else leder * (0.9 + 0.1 * (k % 2)))
        else:
            navy, silber = _f("#1E2A5A"), _f("#E6E8F2")
            _torso(f, lambda i, k, p: silber if p.center.z > 0.47 or p.center.z < 0.03 else navy * (0.9 + 0.15 * max(0.0, -p.normal.y)))
            _platte(f, "Mond", Vector((0, -0.125, 0.36)), 0.14, 0.03, 0.01, Vector((0.6, 0, 1)), -Y, lambda i, k, p: silber, OHNE, spitz=0.9, wolbung=1.0)
            for s in (1, -1):
                _schulterschale(f, 0.19 * s, 0.47, 1.0, lambda poly: silber)
    elif art in ("weste_leder", "mantel_daemmerung"):
        if art == "weste_leder":
            leder = _f("#4E3222")
            _torso(f, lambda i, k, p: leder * (0.85 + 0.15 * (k % 3) / 2))
            for x, z in ((-0.1, 0.12), (0.1, 0.12), (-0.08, 0.3)):
                f.kiste("Tasche", (x, -0.125, z), (0.07, 0.03, 0.07), _f("#35231A"), OHNE)
            f.loft("Riemen", [(Vector((-0.15, -0.1, 0.5)), X, Y, 0.02, 0.006), (Vector((0.14, -0.12, 0.08)), X, Y, 0.02, 0.006)], 6, lambda i, k, p: _f("#221812"), OHNE,
                   oben_zu=True, unten_zu=True)
        else:
            dunkel, violett = _f("#1E1A26"), _f("#7A4AC8")
            _torso(f, lambda i, k, p: dunkel * 1.2)
            _glocke(f, "Umhang", [(Vector((0, 0.02, 0.56)), 0.2, 0.14), (Vector((0, 0.06, 0.3)), 0.28, 0.2), (Vector((0, 0.08, 0.0)), 0.32, 0.23)], (40, 320),
                    lambda poly: violett if poly.center.z < 0.03 else dunkel * (0.9 + 0.2 * max(0.0, poly.normal.z)), OHNE, seg=36, welle=0.06)
            _schale(f, "Kapuze", Vector((0, 0.08, 0.6)), 0.14, 0.12, 0.14, (70, 290), (-20, 70), lambda poly: dunkel, OHNE)
    elif art == "schuhe_wander":
        _paar(f, haupt=_f("#7A5436"), sohle=_f("#3A2618"))
    elif art == "schuhe_mondschritt":
        _paar(f, haupt=_f("#34479A"), sohle=_f("#1E2A5A"), kappe=_f("#E6E8F2"), zier=_f("#E6E8F2"))
    elif art == "stiefel_gruben":
        _paar(f, haupt=_f("#5A4030"), sohle=_f("#1E1812"), schaft=0.12, kappe=_f("#8A9098"))
    elif art == "stiefel_eisen":
        _paar(f, haupt=_f("#7A8088"), sohle=_f("#2A2A2E"), schaft=0.16, kappe=_f("#A9B3C0"), stulpe=_f("#D8AE4A"))
    elif art == "stiefel_pirsch":
        _paar(f, haupt=_f("#6A5A3A"), sohle=_f("#2E2418"), schaft=0.2, stulpe=_f("#4E6A34"))
    elif art == "stiefel_elfen":
        _paar(f, haupt=_f("#E6DFCB"), sohle=_f("#5EA85A"), schaft=0.24, kappe=_f("#5EA85A"), zier=_f("#5EA85A"), stulpe=_f("#5EA85A"))
    elif art == "stiefel_leise":
        _paar(f, haupt=_f("#2A2420"), sohle=_f("#141210"), schaft=0.06)
    elif art == "stiefel_schatten":
        _paar(f, haupt=_f("#1E1A1E"), sohle=_f("#0E0C0E"), schaft=0.2, zier=_f("#C8CCD4"), stulpe=_f("#2E2A34"))
    else:
        raise ValueError(f"Unbekanntes Rüstungsteil {art}")


ALLE = ["hut_lehrling", "hut_sterne", "robe_adept", "robe_erzmagier", "schuhe_wander", "schuhe_mondschritt",
        "helm_eisen", "helm_runen", "brust_kette", "brust_ahnen", "stiefel_gruben", "stiefel_eisen",
        "kappe_jaeger", "krone_mond", "wams_leder", "harnisch_nachtwind", "stiefel_pirsch", "stiefel_elfen",
        "kapuze", "maske_schatten", "weste_leder", "mantel_daemmerung", "stiefel_leise", "stiefel_schatten"]
