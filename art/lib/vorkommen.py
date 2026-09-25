"""Abbaubare Vorkommen: Stein- und Erzvorkommen (mit der Spitzhacke abzubauen).

Ein Vorkommen ist eine Gruppe kantiger, gebrochener Brocken, etwa 1,5 m breit und 1 m hoch.
- Steinvorkommen: heller, grauer Fels mit frischen Bruchkanten und etwas Geröll.
- Erzvorkommen: dunkler Basalt, aus dem rostrote Erzadern und metallisch glänzende
  Erzbrocken herauswachsen – schon von Weitem vom normalen Stein zu unterscheiden.
"""

import math
import random

import bmesh
from mathutils import Vector

from werkstatt import boden_abflachen, einfaerben_nach_richtung, flach, kugel, material, ursprung_unten, vereinen


def _abschlagen(obj, zufall, ebenen, tiefe):
    """Schneidet ein paar Seiten flach ab: sieht aus wie frisch gebrochener Stein."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    mitte = sum((v.co for v in bm.verts), Vector()) / len(bm.verts)
    radius = max((v.co - mitte).length for v in bm.verts)
    for _ in range(ebenen):
        n = Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.2, 1))).normalized()
        grenze = radius * tiefe
        for v in bm.verts:
            abstand = (v.co - mitte).dot(n) - grenze
            if abstand > 0:
                v.co -= n * abstand
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()
    flach(obj)


def _brocken(name, zufall, radius, ort, mat, streckung):
    k = kugel(name, radius, mat, stufen=2, ort=ort, groesse=streckung,
              drehung=(zufall.uniform(-15, 15), zufall.uniform(-15, 15), zufall.uniform(0, 360)))
    _abschlagen(k, zufall, zufall.randint(5, 8), zufall.uniform(0.45, 0.6))
    # leichte Unebenheit, aber die Bruchflächen bleiben flach genug
    for v in k.data.vertices:
        v.co += Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))) * radius * 0.025
    k.data.update()
    return k


def _gruppe(zufall, mat, groesse=1.0):
    """Hauptbrocken und 2–4 kleinere drumherum, leicht in den Boden gesunken."""
    teile = []
    anzahl = zufall.randint(3, 5)
    for i in range(anzahl):
        haupt = i == 0
        r = (0.62 if haupt else zufall.uniform(0.28, 0.42)) * groesse
        winkel = math.tau * i / anzahl + zufall.uniform(-0.4, 0.4)
        abstand = 0.0 if haupt else zufall.uniform(0.5, 0.72) * groesse
        ort = (math.cos(winkel) * abstand, math.sin(winkel) * abstand, r * (0.55 if haupt else 0.45))
        streckung = (zufall.uniform(0.95, 1.2), zufall.uniform(0.85, 1.05), zufall.uniform(0.9, 1.25) if haupt else zufall.uniform(0.7, 1.0))
        teile.append(_brocken(f"Brocken{i}", zufall, r, ort, mat, streckung))
    return teile


def _geroell(zufall, mat, anzahl, weite):
    teile = []
    for i in range(anzahl):
        winkel = zufall.uniform(0, math.tau)
        abstand = zufall.uniform(0.75, weite)
        r = zufall.uniform(0.05, 0.11)
        teile.append(kugel(f"Kiesel{i}", r, mat, stufen=0, ort=(math.cos(winkel) * abstand, math.sin(winkel) * abstand, r * 0.4),
                           groesse=(1.2, 1.0, 0.7), drehung=(0, 0, zufall.uniform(0, 360))))
    return teile


def _auf_oberflaeche(obj, zufall, anzahl, min_z=0.15):
    """Zufällige Punkte (mit Normale) auf der Oberfläche, nicht ganz unten am Boden."""
    flaechen = [p for p in obj.data.polygons if p.center.z > min_z and p.normal.z > -0.3]
    zufall.shuffle(flaechen)
    return [(Vector(p.center), Vector(p.normal)) for p in flaechen[:anzahl]]


def _kristall(name, ort, normale, laenge, dicke, mat, zufall):
    """Kantiger Erzbrocken, halb im Fels steckend, entlang der Normale herausragend."""
    k = kugel(name, 1.0, mat, stufen=0, ort=(0, 0, 0), groesse=(dicke, dicke * zufall.uniform(0.8, 1.2), laenge))
    richtung = (normale + Vector((0, 0, 0.35))).normalized()
    drehung = Vector((0, 0, 1)).rotation_difference(richtung).to_matrix().to_4x4()
    k.data.transform(drehung)
    k.data.transform(__import__("mathutils").Matrix.Translation(ort + richtung * laenge * 0.25))
    k.data.update()
    flach(k)
    return k


def steinvorkommen(seed):
    zufall = random.Random(seed)
    hell = material("stein_hell")
    fels = material("stein")
    dunkel = material("stein_dunkel")
    teile = _gruppe(zufall, fels)
    for teil in teile:
        # Bruchflächen oben frisch und hell, sonst verwittert grau
        einfaerben_nach_richtung(teil, hell, grenze=0.55)
    teile += _geroell(zufall, dunkel, zufall.randint(6, 9), 1.15)
    obj = vereinen("Steinvorkommen", teile)
    boden_abflachen(obj, 0.0)
    ursprung_unten(obj)
    return obj


def erzvorkommen(seed):
    zufall = random.Random(seed)
    basalt = material("basalt")
    basalt_oben = material("basalt_hell")
    rost = material("erz_rost")
    glanz = material("erz_glanz")
    ader = material("erz_dunkel")
    teile = _gruppe(zufall, basalt, groesse=0.95)
    for teil in teile:
        einfaerben_nach_richtung(teil, basalt_oben, grenze=0.7)
    # Erzadern: einzelne Flächen der Brocken rostrot färben
    for teil in teile:
        if ader.name not in teil.data.materials:
            teil.data.materials.append(ader)
        index = list(teil.data.materials).index(ader)
        for polygon in teil.data.polygons:
            if zufall.random() < 0.1 and polygon.center.z > 0.1:
                polygon.material_index = index
    # Herausragende Erzbrocken: rostig mit metallisch glänzenden Spitzen
    kristalle = []
    for i, (ort, normale) in enumerate(_auf_oberflaeche(teile[0], zufall, 7) + [p for t in teile[1:] for p in _auf_oberflaeche(t, zufall, 2, 0.1)]):
        mat = glanz if i % 3 == 0 else rost
        kristalle.append(_kristall(f"Erz{i}", ort, normale, zufall.uniform(0.1, 0.2), zufall.uniform(0.05, 0.09), mat, zufall))
    teile += kristalle
    teile += _geroell(zufall, rost, zufall.randint(3, 5), 1.05)
    teile += _geroell(zufall, basalt, zufall.randint(3, 5), 1.1)
    obj = vereinen("Erzvorkommen", teile)
    boden_abflachen(obj, 0.0)
    ursprung_unten(obj)
    return obj
