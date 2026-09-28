"""Rüstung an die Helden anpassen: die Teile aus ruestung.py (gebaut wie auf einem Ständer) werden
an Kopf, Oberkörper und Füße der Figur eingepasst, übernehmen die Gewichte der nächsten Stellen des
Körpers und hängen als eigene, verformbare Knoten am Skelett. Im Spiel sind sie ausgeblendet, bis
das Teil angelegt ist (game/src/characters.rs).
"""

import math

import bpy
from mathutils import Vector
from mathutils.kdtree import KDTree

import ruestung
from figuren import am_skelett, vereinen

KLASSEN = ["magier", "zwerg", "bogenschuetze", "schurke"]
# Reihenfolge in ruestung.ALLE: je Klasse zwei Kopf-, zwei Brust-, zwei Fußteile
PLAETZE = ["kopf", "kopf", "brust", "brust", "fuesse", "fuesse"]
ROCK = {"robe_adept": 0.25, "robe_erzmagier": 0.25}
RUMPF = ("Becken", "Bauch", "Brust", "Hals")


def fuer_klasse(klasse):
    i = KLASSEN.index(klasse)
    return ruestung.ALLE[i * 6:(i + 1) * 6]


def _platz(art):
    return PLAETZE[ruestung.ALLE.index(art) % 6]


def _koerper(haupt):
    """Eckpunkte des Körpers mit ihren Gewichten und dem stärksten Knochen."""
    namen = {g.index: g.name for g in haupt.vertex_groups}
    punkte = []
    for v in haupt.data.vertices:
        gewichte = {namen[g.group]: g.weight for g in v.groups if g.weight > 0.001}
        staerkster = max(gewichte, key=gewichte.get) if gewichte else None
        punkte.append((v.co.copy(), gewichte, staerkster))
    return punkte


def _quantil(werte, q):
    werte = sorted(werte)
    return werte[min(len(werte) - 1, int(q * len(werte)))]


def _kopf(obj, art, punkte, armatur):
    kopf = armatur.data.bones["Kopf"]
    z0, z1 = kopf.head_local.z, kopf.tail_local.z
    oben = [p for p, _, s in punkte if s == "Kopf" and p.z > z0 + 0.4 * (z1 - z0)]
    # Mitte auf der Achse des Kopfknochens; die Breite aus einer Scheibe quer durch den Schädel
    # (ohne Pferdeschwanz und langes Haar hinten)
    cx, cy = kopf.head_local.x, kopf.head_local.y + 0.01
    scheibe = [p for p in oben if abs(p.y - cy) < 0.05] or oben
    breite = _quantil([abs(p.x - cx) for p in scheibe], 0.9)
    # Mitte des Schädels auf halber Länge des Kopfknochens; Hüte und Kronen sitzen höher als Helme
    hoeher = {"hut_lehrling": 0.5, "hut_sterne": 0.5, "kappe_jaeger": 0.35, "krone_mond": 0.55, "kapuze": 0.1, "maske_schatten": 0.1}
    s = breite * 1.12 / 0.115
    mitte = Vector((cx, cy, z0 + 0.5 * (z1 - z0) + hoeher.get(art, 0.05) * breite))
    for v in obj.data.vertices:
        v.co = mitte + v.co * s
    gruppe = obj.vertex_groups.new(name="Kopf")
    gruppe.add([v.index for v in obj.data.vertices], 1.0, "REPLACE")


def _brust(obj, art, punkte, armatur):
    bauch, brust = armatur.data.bones["Bauch"], armatur.data.bones["Brust"]
    unten, oben = bauch.head_local.z, brust.tail_local.z
    h = oben - unten
    band = [p for p, _, s in punkte if s in ("Bauch", "Brust") and unten + 0.2 * h < p.z < unten + 0.8 * h]
    # Mitte auf der Wirbelsäule (eine Schulterplatte auf einer Seite soll nichts verschieben)
    cx = bauch.head_local.x
    cy = (_quantil([p.y for p in band], 0.05) + _quantil([p.y for p in band], 0.95)) / 2
    halb_breit = _quantil([abs(p.x - cx) for p in band], 0.85)
    halb_tief = _quantil([abs(p.y - cy) for p in band], 0.92)
    anfang = unten - 0.08 * h
    sz = (oben - anfang) / 0.54
    sx, sy = halb_breit * 1.08 / 0.165, halb_tief * 1.06 / 0.115
    rock = ROCK.get(art, 0.0)
    ursprung = Vector((cx, cy, anfang - rock * sz))

    def strecken(w, rumpf, s):
        # Der Rumpf passt sich an; was darüber hinausragt (Schulterschalen, Umhang), wächst nur
        # so wie die Figur hoch ist, sonst werden die Schultern riesig
        b = min(abs(w), rumpf)
        return math.copysign(b * s + (abs(w) - b) * min(s, sz * 1.1), w)
    for v in obj.data.vertices:
        v.co = ursprung + Vector((strecken(v.co.x, 0.17, sx), strecken(v.co.y, 0.12, sy), v.co.z * sz))
    # Ein Bart bleibt vorne: was davor läge, rückt hinter ihn
    kinn = armatur.data.bones["Kopf"].head_local.z
    bart = [p for p, g, _ in punkte if g.get("Kopf", 0.0) > 0.3 and p.z < kinn]
    if bart:
        baum = KDTree(len(bart))
        for i, p in enumerate(bart):
            baum.insert(p, i)
        baum.balance()
        for v in obj.data.vertices:
            _, i, _ = baum.find(v.co)
            b = bart[i]
            if abs(v.co.x - b.x) < 0.05 and abs(v.co.z - b.z) < 0.05 and v.co.y < b.y + 0.03:
                v.co.y = b.y + 0.03
    beine = unten - 0.02
    _gewichte(obj, punkte, lambda q, s: s in RUMPF or (q.z < beine and s and s.startswith("Oberschenkel")))


def _fuesse(obj, punkte):
    teile = {}
    for seite, vorzeichen in (("L", 1), ("R", -1)):
        fuss = [p for p, _, s in punkte if s == f"Fuss.{seite}"]
        lo = Vector((min(p.x for p in fuss), min(p.y for p in fuss), min(p.z for p in fuss)))
        hi = Vector((max(p.x for p in fuss), max(p.y for p in fuss), max(p.z for p in fuss)))
        sw = (hi.x - lo.x) * 1.12 / 0.116
        sl = (hi.y - lo.y) * 1.08 / 0.285
        teile[vorzeichen] = (Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z)), sw, sl, (sw + sl) / 2)
    for v in obj.data.vertices:
        vorzeichen = 1 if v.co.x > 0 else -1
        mitte, sw, sl, sz = teile[vorzeichen]
        v.co = mitte + Vector(((v.co.x - 0.075 * vorzeichen) * sw, (v.co.y + 0.0225) * sl, v.co.z * sz))
    _gewichte(obj, punkte, lambda q, s: bool(s) and (s.startswith("Fuss") or s.startswith("Unterschenkel")))


def _gewichte(obj, punkte, erlaubt, k=6):
    """Jede Ecke übernimmt die gemischten Gewichte der nächsten Körperstellen."""
    kandidaten = [(p, g) for p, g, s in punkte if g and erlaubt(p, s)]
    baum = KDTree(len(kandidaten))
    for i, (p, _) in enumerate(kandidaten):
        baum.insert(p, i)
    baum.balance()
    gruppen = {}
    for v in obj.data.vertices:
        summe = {}
        for _, i, d in baum.find_n(v.co, k):
            w = 1.0 / (d + 0.01)
            for knochen, g in kandidaten[i][1].items():
                summe[knochen] = summe.get(knochen, 0.0) + g * w
        beste = sorted(summe.items(), key=lambda kv: -kv[1])[:4]
        gesamt = sum(w for _, w in beste) or 1.0
        for knochen, w in beste:
            if knochen not in gruppen:
                gruppen[knochen] = obj.vertex_groups.new(name=knochen)
            gruppen[knochen].add([v.index], w / gesamt, "REPLACE")


def alle(f, haupt, armatur, mat):
    punkte = _koerper(haupt)
    for art in f.ruestungen:
        f.teile = []
        f.angelegt = True
        ruestung.bauen(f, art)
        obj = vereinen(f.teile, art)
        obj.vertex_groups.clear()
        platz = _platz(art)
        if platz == "kopf":
            _kopf(obj, art, punkte, armatur)
        elif platz == "brust":
            _brust(obj, art, punkte, armatur)
        else:
            _fuesse(obj, punkte)
        obj.data.update()
        obj.data.materials.clear()
        obj.data.materials.append(mat)
        am_skelett(obj, armatur)
        dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
        print(f"RÜSTUNG {art} ({platz}): {dreiecke} Dreiecke")
    f.teile = [haupt]
