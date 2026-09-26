"""Tempelruine: flache Steinplattform (im Spiel etwas eingesunken), zehn Säulen (manche abgebrochen, eine umgestürzt),
zwei Säulen noch mit Gebälk, in der Mitte ein Sockel mit leuchtendem Runenstein, Efeu.
Vorderseite ist +X wie bei den anderen Kulissen-Modellen."""

import math
import random

import bmesh
from mathutils import Vector

from lager import brett_bm, einfarbig, fertig, objekt, setzen, stein_bm
from vorkommen import _steinfarbe, farbe


def _saeule(zufall, teile, stein, x, y, hoehe, liegend=False):
    """Säule aus Trommeln (achteckig), mit Basis; `liegend`: umgestürzt entlang Y."""
    bm = bmesh.new()
    trommeln = max(1, int(hoehe / 0.9))
    z = 0.0
    ringe = []
    for i in range(trommeln + 1):
        r = 0.36 - 0.03 * (i / max(trommeln, 1))
        ring = [bm.verts.new((math.cos(math.tau * k / 8) * r, math.sin(math.tau * k / 8) * r, z)) for k in range(8)]
        ringe.append(ring)
        z += hoehe / trommeln
    for a, b in zip(ringe, ringe[1:]):
        for k in range(8):
            bm.faces.new((a[k], a[(k + 1) % 8], b[(k + 1) % 8], b[k]))
    bm.faces.new(list(reversed(ringe[0])))
    bm.faces.new(ringe[-1])
    # abgebrochene Kante oben: schräg anschneiden
    if not liegend and hoehe < 3.5:
        for v in ringe[-1]:
            v.co.z += (v.co.x * 0.4 + v.co.y * 0.25) * zufall.uniform(0.6, 1.0)
    bm.faces.layers.int.new("fase")
    if liegend:
        setzen(bm, (0, 0, 0), (90, 0, zufall.uniform(-10, 10)))
        setzen(bm, (x, y, 0.33))
    else:
        setzen(bm, (x, y, 0.55))
    teile.append(objekt("Säule", bm, stein))
    if not liegend:
        _brett_teil(teile, stein, (0.95, 0.95, 0.25), (x, y, 0.55 + 0.12))


def _brett_teil(teile, farbe_von, masse, ort, drehung=(0, 0, 0), fase=0.03):
    bm = brett_bm(*masse, fase=fase)
    setzen(bm, (0, 0, 0), drehung)
    setzen(bm, ort)
    teile.append(objekt("Stein", bm, farbe_von))


def tempelruine(seed=81):
    zufall = random.Random(seed)
    teile, leuchtend = [], []
    stein = _steinfarbe(zufall, farbe("#8C8577"), farbe("#B3AB9B"), farbe("#D6CEBD"), farbe("#E8E1D2"),
                        moos=farbe("#5F8A3E"), moos_rauschen=lambda p: 0.45 + 0.15 * math.sin(p.x * 1.7 + p.y * 1.1))
    # Plattform aus Platten, einzelne fehlen oder sind verrutscht
    for ix in range(-4, 5):
        for iy in range(-3, 4):
            if zufall.random() < 0.08:
                continue
            kipp = (zufall.uniform(-3, 3), zufall.uniform(-3, 3), zufall.uniform(-2, 2))
            _brett_teil(teile, stein, (1.18, 1.18, 0.55), (ix * 1.2, iy * 1.2, 0.27 - (0.05 if zufall.random() < 0.2 else 0)), kipp, fase=0.0)
    # Säulen: zwei Reihen à fünf
    hoehen = [4.6, 2.1, 4.6, 1.3, 3.0, 4.6, 0.8, 4.6, 2.6, 1.7]
    orte = [(x, y) for y in (-3.0, 3.0) for x in (-4.4, -2.2, 0.0, 2.2, 4.4)]
    for (x, y), h in zip(orte, hoehen):
        if h < 1.0:
            # umgestürzt, daneben liegend
            _saeule(zufall, teile, stein, x - 0.4, y * 1.55, 3.6, liegend=True)
            _saeule(zufall, teile, stein, x, y, 0.5)
            continue
        _saeule(zufall, teile, stein, x, y, h)
    # Gebälk über den hohen Säulen an der Stirnseite
    for y in (-3.0, 3.0):
        _brett_teil(teile, stein, (5.4, 0.8, 0.55), (-2.2, y, 0.55 + 4.6 + 0.28))
    _brett_teil(teile, stein, (0.8, 6.8, 0.5), (-4.4, 0.0, 0.55 + 4.6 + 0.8))
    # Trümmer
    for i in range(14):
        r = zufall.uniform(0.18, 0.4)
        bm = stein_bm(zufall, r, flach=0.7)
        w = zufall.uniform(0, math.tau)
        d = zufall.uniform(5.5, 8.0)
        setzen(bm, (math.cos(w) * d, math.sin(w) * d * 0.8, r * 0.3), (0, 0, zufall.uniform(0, 360)))
        teile.append(objekt("Trümmer", bm, stein))
    # Sockel mit Runenstein in der Mitte
    _brett_teil(teile, stein, (1.4, 1.4, 0.8), (0.0, 0.0, 0.95))
    bm = brett_bm(0.5, 0.35, 1.6, fase=0.05)
    for v in bm.verts:
        v.co.x *= 1.0 - 0.35 * (v.co.z + 0.8) / 1.6
    setzen(bm, (0.0, 0.0, 1.35 + 0.8))
    teile.append(objekt("Runenstein", bm, einfarbig("#5A6272", 0.05, zufall)))
    for i, z in enumerate((1.75, 2.15, 2.55)):
        _brett_teil(leuchtend, einfarbig("#8FE8FF", 0.03, zufall), (0.06, 0.2 - i * 0.03, 0.08), (0.27 - i * 0.04, 0.0, z), fase=0.0)
    # Efeu an zwei Säulen
    efeu = einfarbig("#4E8A3A", 0.12, zufall)
    for (x, y) in ((-4.4, -3.0), (2.2, 3.0)):
        for i in range(9):
            w = zufall.uniform(0, math.tau)
            _brett_teil(teile, efeu, (0.3, 0.05, 0.35), (x + math.cos(w) * 0.37, y + math.sin(w) * 0.37, 0.9 + i * 0.38),
                        (0, zufall.uniform(-20, 20), math.degrees(w) + 90), fase=0.0)
    return fertig("Tempelruine", teile, leuchtend, leucht=("#7FDFFF", 3.0))
