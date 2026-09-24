"""Baum-Generator im detaillierten Polygon-Stil – zwischen freundlich und Dark Fantasy.

Flache Flächen wie bisher, aber deutlich mehr Details: knorrige, sich verzweigende Äste, die
sich unter ihrem Gewicht biegen, Wurzeln, die aus dem Boden brechen, Kronen aus vielen
Blattbüscheln, Rindenstreifen, Moos am Stammfuß. Farben gedeckt, aber lebendig.

Licht in der Geometrie: Blattflächen, die nach oben zeigen, bekommen eine hellere Farbe
(wie von der Sonne getroffen), Unterseiten eine dunklere – so wirken Kronen plastisch.

Jede Art ist eine Funktion (eiche, tanne, palme, zauberbaum), `seed` ergibt die Variante.
Ein Modell-Skript ruft sie nur auf, z. B. art/modelle/natur/eiche_1.py: `eiche(seed=11)`.
Koordinaten wie in der Werkstatt: Z oben, Ursprung am Stammfuß.
"""

import math
import random

import bmesh
import bpy
from mathutils import Quaternion, Vector

from werkstatt import PALETTE, material

# Palette „goldene Mitte“: gedeckter als Comic-Grün, lebendiger als düsteres Oliv.
PALETTE.update({
    "bm_rinde": "#4A3B30",
    "bm_rinde_dunkel": "#33291F",
    "bm_rinde_hell": "#6B5847",
    "bm_moos": "#5C7438",
    "bm_flechte": "#8A9678",
    "bm_laub_dunkel": "#2A4A2C",
    "bm_laub": "#3C6236",
    "bm_laub_oliv": "#566B34",
    "bm_laub_hell": "#6F8C45",
    "bm_laub_herbst": "#8A7338",
    "bm_nadel_dunkel": "#1F3D33",
    "bm_nadel": "#2B5242",
    "bm_nadel_hell": "#3E6B52",
    "bm_zapfen": "#5A4030",
    "bm_palme_stamm": "#6A5844",
    "bm_palme_ring": "#4E4033",
    "bm_palmblatt_dunkel": "#35602F",
    "bm_palmblatt": "#4A7A3A",
    "bm_palmblatt_hell": "#6A9A48",
    "bm_palmblatt_spitze": "#8C8A45",
    "bm_nuss": "#4A3622",
    "bm_zauber_rinde": "#2C2833",
    "bm_zauber_rinde_hell": "#3E3848",
    "bm_zauber_violett": "#4E3F7A",
    "bm_zauber_violett_hell": "#6A56A0",
    "bm_zauber_tuerkis": "#3F7F80",
    "bm_zauber_dunkel": "#2E2650",
    "bm_zauber_glut": "#B08CFF",
})


class Baum:
    """Sammelt die Geometrie eines Baums in einem bmesh, mit Material je Fläche."""

    def __init__(self, name, seed):
        self.name = name
        self.bm = bmesh.new()
        self.rng = random.Random(seed)
        self.materialien = []
        self.aeste = []  # (Punkte, Radien, Tiefe, tot)
        # Material → (hell für Oberseiten, dunkel für Unterseiten)
        self.licht = {}

    def mat(self, name, **optionen):
        """Index eines Materials (wird bei Bedarf angelegt)."""
        if name not in self.materialien:
            self.materialien.append(name)
            material(name, **optionen)
        return self.materialien.index(name)

    def zufallsrichtung(self):
        r = self.rng
        return Vector((r.uniform(-1, 1), r.uniform(-1, 1), r.uniform(-1, 1)))

    # ------------------------------------------------------------------
    # Bausteine
    # ------------------------------------------------------------------
    def rohr(self, punkte, radien, ecken, mat, spitze=True, streifen=None):
        """Konisches Rohr entlang einer Punktfolge. `streifen`: zweites Material für
        Rindenrillen (einige Längsbahnen bekommen es)."""
        bm = self.bm
        ringe = []
        normale = None
        versatz = self.rng.uniform(0, math.tau)
        bahnen = {k for k in range(ecken) if self.rng.random() < 0.35} if streifen is not None else set()
        for i, p in enumerate(punkte):
            t = (punkte[i + 1] - p) if i + 1 < len(punkte) else (p - punkte[i - 1])
            t.normalize()
            if normale is None:
                hilfe = Vector((0, 0, 1)) if abs(t.z) < 0.9 else Vector((1, 0, 0))
                normale = t.cross(hilfe).normalized()
            else:
                normale = (normale - t * normale.dot(t)).normalized()
            binormale = t.cross(normale)
            ring = []
            for k in range(ecken):
                a = versatz + math.tau * k / ecken
                r = radien[i] * self.rng.uniform(0.9, 1.07)
                ring.append(bm.verts.new(p + (normale * math.cos(a) + binormale * math.sin(a)) * r))
            ringe.append(ring)
        for a, b in zip(ringe, ringe[1:]):
            for k in range(ecken):
                flaeche = bm.faces.new((a[k], a[(k + 1) % ecken], b[(k + 1) % ecken], b[k]))
                flaeche.material_index = streifen if k in bahnen else mat
        if spitze:
            ende = punkte[-1] + (punkte[-1] - punkte[-2]).normalized() * radien[-1] * 1.5
            s = bm.verts.new(ende)
            for k in range(ecken):
                bm.faces.new((ringe[-1][k], ringe[-1][(k + 1) % ecken], s)).material_index = mat

    def pfad(self, start, richtung, laenge, schritte, knick, schwere, aufwaerts=0.0):
        """Punkte eines Asts: zufällige Knicke, Schwerkraft zieht nach unten, Licht nach oben."""
        punkte = [start.copy()]
        d = richtung.normalized()
        for _ in range(schritte):
            d = (d + self.zufallsrichtung() * knick + Vector((0, 0, aufwaerts - schwere))).normalized()
            punkte.append(punkte[-1] + d * (laenge / schritte))
        return punkte

    def buschel(self, mitte, groesse, mats, stufen=2, platt=0.72, beulen=0.18):
        """Blattbüschel: verbeulte, flach schattierte Kugel mit gemischten Blattfarben."""
        ergebnis = bmesh.ops.create_icosphere(self.bm, subdivisions=stufen, radius=groesse)
        verts = ergebnis["verts"]
        achse = self.zufallsrichtung()
        drehung = Quaternion(achse.normalized() if achse.length > 0.01 else Vector((0, 0, 1)), self.rng.uniform(0, math.tau))
        for v in verts:
            p = drehung @ v.co
            p.z *= platt
            p += self.zufallsrichtung() * groesse * beulen
            v.co = mitte + p
        for f in {f for v in verts for f in v.link_faces}:
            f.material_index = self.rng.choice(mats)

    def flechte(self, oben, laenge, mat):
        """Herabhängender Flechtenstreifen (drei Zickzack-Segmente, beidseitig sichtbar)."""
        bm = self.bm
        breite = self.rng.uniform(0.04, 0.08)
        seite = self.zufallsrichtung()
        seite.z = 0
        seite = seite.normalized() * breite if seite.length > 0.01 else Vector((breite, 0, 0))
        punkte = [oben]
        for _ in range(3):
            versatz = self.zufallsrichtung() * 0.05
            versatz.z = 0
            punkte.append(punkte[-1] + Vector((0, 0, -laenge / 3)) + versatz)
        links = [bm.verts.new(p - seite * (1 - i / 3)) for i, p in enumerate(punkte)]
        rechts = [bm.verts.new(p + seite * (1 - i / 3)) for i, p in enumerate(punkte)]
        for i in range(3):
            bm.faces.new((links[i], rechts[i], rechts[i + 1], links[i + 1])).material_index = mat

    # ------------------------------------------------------------------
    # Fertigstellen
    # ------------------------------------------------------------------
    def fertig(self, moos=None, rinde=()):
        """Mesh-Objekt erzeugen: Normalen nach außen, Licht in die Blattfarben, Moos am Fuß,
        flach schattiert, nichts unter dem Boden."""
        bm = self.bm
        for v in bm.verts:
            if v.co.z < 0:
                v.co.z = 0.0
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        for f in bm.faces:
            n = f.normal
            if f.material_index in self.licht:
                hell, dunkel = self.licht[f.material_index]
                if n.z > 0.55 and self.rng.random() < 0.8:
                    f.material_index = hell
                elif n.z < -0.35:
                    f.material_index = dunkel
            elif moos is not None and f.material_index in rinde:
                # Moos wächst unten am Stamm und auf den Wurzeln, an nach oben gewandten Flächen
                # (nach oben hin immer seltener, damit kein hartes Band entsteht)
                hoehe = f.calc_center_median().z
                if hoehe < 1.5 and n.z > -0.1 and self.rng.random() < 0.8 * (1 - hoehe / 1.5) ** 1.3:
                    f.material_index = moos
        mesh = bpy.data.meshes.new(self.name)
        bm.to_mesh(mesh)
        bm.free()
        obj = bpy.data.objects.new(self.name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        for name in self.materialien:
            mesh.materials.append(bpy.data.materials[name])
        for polygon in mesh.polygons:
            polygon.use_smooth = False
        dreiecke = sum(len(p.vertices) - 2 for p in mesh.polygons)
        print(f"BAUM {self.name}: {dreiecke} Dreiecke")
        return obj


def _drehen(richtung, winkel, azimut):
    """Richtung um `winkel` von `richtung` weg kippen, rundherum um `azimut` gedreht."""
    senkrecht = richtung.orthogonal().normalized()
    senkrecht = Quaternion(richtung, azimut) @ senkrecht
    return (richtung * math.cos(winkel) + senkrecht * math.sin(winkel)).normalized()


def _wurzeln(baum, radius, anzahl, mat, spreizung=1.0, streifen=None):
    """Wurzeln brechen aus dem Boden und laufen flach aus."""
    for i in range(anzahl):
        a = math.tau * i / anzahl + baum.rng.uniform(-0.3, 0.3)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        start = aussen * radius * 0.55 + Vector((0, 0, radius * 1.2))
        punkte = baum.pfad(start, aussen + Vector((0, 0, -0.55)), radius * baum.rng.uniform(2.4, 3.4) * spreizung, 5, 0.18, 0.22)
        radien = [radius * 0.52 * (1 - 0.82 * j / 5) for j in range(6)]
        baum.rohr(punkte, radien, 6, mat, streifen=streifen)


# ---------------------------------------------------------------------------
# Eiche: kräftig, knorrig, mit voller, plastischer Krone
# ---------------------------------------------------------------------------
def eiche(seed=1, name="Eiche"):
    baum = Baum(name, seed)
    r = baum.rng
    rinde = baum.mat("bm_rinde")
    rille = baum.mat("bm_rinde_dunkel")
    totholz = baum.mat("bm_rinde_hell")
    moos = baum.mat("bm_moos")
    laub = [baum.mat("bm_laub"), baum.mat("bm_laub_oliv")]
    hell = baum.mat("bm_laub_hell")
    dunkel = baum.mat("bm_laub_dunkel")
    herbst = baum.mat("bm_laub_herbst")
    flechte = baum.mat("bm_flechte", beidseitig=True)
    for m in laub + [herbst]:
        baum.licht[m] = (hell, dunkel)

    stamm_radius = r.uniform(0.32, 0.4)
    stamm_hoehe = r.uniform(2.2, 2.8)

    def krone(p, groesse):
        """Hauptbüschel plus kleine Nebenbüschel drumherum – so wirkt die Krone lockerer."""
        farben = laub + ([herbst] if r.random() < 0.18 else [])
        baum.buschel(p, groesse, farben, stufen=2)
        for _ in range(r.randint(1, 2)):
            if len(baum.bm.faces) > 3700:  # Dreiecks-Budget (natur ≤ 5000) – Nebenbüschel sind verzichtbar
                break
            neben = p + baum.zufallsrichtung() * groesse * 0.9
            neben.z = max(neben.z, p.z - groesse * 0.3)
            baum.buschel(neben, groesse * r.uniform(0.45, 0.65), farben, stufen=1, beulen=0.12)

    def ast(start, richtung, laenge, radius, tiefe, tot):
        schritte = max(3, int(laenge / 0.4))
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.24, 0.035 * tiefe, 0.1)
        radien = [radius * (1 - 0.55 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, radien, max(5, 8 - tiefe), totholz if tot else rinde, streifen=None if tot else rille)
        baum.aeste.append((punkte, radien, tiefe, tot))
        if tiefe >= 3 or radien[-1] < 0.035:
            if not tot:
                krone(punkte[-1], r.uniform(0.55, 0.8))
            return
        kinder = r.randint(2, 3)
        for k in range(kinder):
            i = min(schritte, int(r.uniform(0.45, 1.0) * schritte))
            d = (punkte[min(i + 1, schritte)] - punkte[max(i - 1, 0)]).normalized()
            neu = _drehen(d, r.uniform(0.45, 0.9), math.tau * k / kinder + r.uniform(-0.6, 0.6))
            kind_tot = tot or (tiefe >= 2 and r.random() < 0.05)
            ast(punkte[i], neu, laenge * r.uniform(0.62, 0.76), radien[i] * 0.62, tiefe + 1, kind_tot)
        if tiefe == 2 and not tot:
            krone(punkte[len(punkte) // 2] + Vector((0, 0, 0.3)), r.uniform(0.5, 0.68))

    # Stamm: kurz, dick, leicht gedreht; teilt sich in 3–4 kräftige Hauptäste
    stamm = baum.pfad(Vector((0, 0, 0)), Vector((0, 0, 1)), stamm_hoehe, 6, 0.05, 0.0, 0.25)
    stamm_radien = [stamm_radius * (1.3 - 0.4 * i / 6) for i in range(7)]
    baum.rohr(stamm, stamm_radien, 10, rinde, spitze=False, streifen=rille)
    _wurzeln(baum, stamm_radius, r.randint(5, 6), rinde, streifen=rille)
    haupt = r.randint(3, 4)
    for k in range(haupt):
        richtung = _drehen(Vector((0, 0, 1)), r.uniform(0.72, 1.0), math.tau * k / haupt + r.uniform(-0.3, 0.3))
        ast(stamm[-1], richtung, r.uniform(2.6, 3.1), stamm_radien[-1] * 0.72, 1, False)
    ast(stamm[-1], _drehen(Vector((0, 0, 1)), 0.12, r.uniform(0, math.tau)), r.uniform(1.8, 2.2), stamm_radien[-1] * 0.55, 2, False)

    # Wenige, kurze Flechten – ein Hauch Dunkelheit, kein Spukwald
    kandidaten = [p for punkte, _, t, _ in baum.aeste if 1 <= t <= 2 for p in punkte[1:]]
    for p in r.sample(kandidaten, min(len(kandidaten), r.randint(2, 4))):
        baum.flechte(p, r.uniform(0.3, 0.65), flechte)
    return baum.fertig(moos=moos, rinde=(rinde, rille))


# ---------------------------------------------------------------------------
# Tanne: hoch, dicht gestuft, mit Zapfen
# ---------------------------------------------------------------------------
def tanne(seed=1, name="Tanne"):
    baum = Baum(name, seed)
    r = baum.rng
    rinde = baum.mat("bm_rinde_dunkel")
    rille = baum.mat("bm_rinde")
    moos = baum.mat("bm_moos")
    nadeln = [baum.mat("bm_nadel"), baum.mat("bm_nadel_dunkel")]
    hell = baum.mat("bm_nadel_hell")
    dunkel = baum.mat("bm_nadel_dunkel")
    zapfen = baum.mat("bm_zapfen")
    for m in nadeln:
        baum.licht[m] = (hell, dunkel)

    hoehe = r.uniform(7.8, 9.8)
    radius = r.uniform(0.21, 0.27)
    stamm = baum.pfad(Vector((0, 0, 0)), Vector((0, 0, 1)), hoehe, 9, 0.03, 0.0, 0.0)
    baum.rohr(stamm, [radius * (1.15 - i / 9) + 0.02 for i in range(10)], 7, rinde, streifen=rille)
    _wurzeln(baum, radius, 5, rinde, 0.85, streifen=rille)

    # Quirle von Ästen: unten lang und leicht hängend, oben kurz
    etagen = 10
    for e in range(etagen):
        t = e / etagen
        z = hoehe * (0.17 + 0.78 * t)
        laenge = (1 - t) ** 0.9 * 2.5 + 0.35
        anzahl = r.randint(5, 7) if t < 0.8 else r.randint(3, 5)
        for k in range(anzahl):
            a = math.tau * k / anzahl + r.uniform(-0.3, 0.3) + e * 0.7
            aussen = Vector((math.cos(a), math.sin(a), 0))
            richtung = aussen + Vector((0, 0, r.uniform(-0.3, -0.08)))
            punkte = baum.pfad(Vector((0, 0, z)), richtung, laenge * r.uniform(0.85, 1.1), 3, 0.1, 0.07)
            baum.rohr(punkte, [0.065 * (1 - i / 3) + 0.01 for i in range(4)], 4, rinde)
            # Zweig aus zwei gestaffelten Nadelkegeln – wirkt voller als einer
            _nadelzweig(baum, punkte, laenge, nadeln)
            oben = [p + Vector((0, 0, 0.16)) for p in punkte[: max(2, len(punkte) - 1)]]
            _nadelzweig(baum, oben, laenge * 0.75, nadeln, breite_faktor=0.7)
            if e < 6 and r.random() < 0.18:
                spitze = punkte[-2]
                baum.buschel(spitze + Vector((0, 0, -0.18)), 0.07, [zapfen], stufen=1, platt=1.9, beulen=0.05)
    _nadelzweig(baum, [Vector((0, 0, hoehe - 1.1)), Vector((0, 0, hoehe + 0.55))], 0.6, nadeln, aufrecht=True)
    return baum.fertig(moos=moos, rinde=(rinde, rille))


def _nadelzweig(baum, punkte, laenge, nadeln, aufrecht=False, breite_faktor=1.0):
    """Flacher, ausgefranster Nadelkegel entlang eines Asts (typischer Tannenzweig im Polygon-Stil)."""
    r = baum.rng
    start, ende = punkte[0], punkte[-1]
    achse = ende - start
    lang = max(achse.length, 0.2)
    achse.normalize()
    breite = ((0.36 + laenge * 0.27) if not aufrecht else 0.58) * breite_faktor
    ergebnis = bmesh.ops.create_cone(baum.bm, cap_ends=True, cap_tris=False, segments=8, radius1=breite, radius2=0.0, depth=lang * 1.05)
    drehung = Vector((0, 0, 1)).rotation_difference(achse)
    mitte = (start + ende) / 2
    for v in ergebnis["verts"]:
        p = v.co.copy()
        if not aufrecht:
            p.y *= 0.42  # abgeflacht wie ein Zweig
        p += baum.zufallsrichtung() * 0.07
        # Zweigspitzen hängen ein wenig durch
        v.co = mitte + drehung @ p + Vector((0, 0, -0.12 * (p.z / lang + 0.5) if not aufrecht else 0))
    for f in {f for v in ergebnis["verts"] for f in v.link_faces}:
        f.material_index = r.choice(nadeln)


# ---------------------------------------------------------------------------
# Palme: geringelter, geschwungener Stamm, gefiederte Wedel, Kokosnüsse
# ---------------------------------------------------------------------------
def palme(seed=1, name="Palme"):
    baum = Baum(name, seed)
    r = baum.rng
    stamm_mat = baum.mat("bm_palme_stamm")
    ring_mat = baum.mat("bm_palme_ring")
    blatt = [baum.mat("bm_palmblatt", beidseitig=True), baum.mat("bm_palmblatt_dunkel", beidseitig=True)]
    hell = baum.mat("bm_palmblatt_hell", beidseitig=True)
    spitze_mat = baum.mat("bm_palmblatt_spitze", beidseitig=True)
    nuss = baum.mat("bm_nuss")
    for m in blatt:
        baum.licht[m] = (hell, blatt[1])

    hoehe = r.uniform(5.4, 6.6)
    neigung = Vector((r.uniform(-1, 1), r.uniform(-1, 1), 0)).normalized()
    ringe = 14
    punkte = [neigung * ((i / ringe) ** 2 * 1.5) + Vector((0, 0, hoehe * i / ringe)) for i in range(ringe + 1)]
    radien = [0.21 - 0.08 * i / ringe for i in range(ringe + 1)]
    # Geringelter Stamm: jeder Ring etwas dicker am unteren Rand
    for i in range(ringe):
        mitte = (punkte[i] + punkte[i + 1]) / 2
        baum.rohr([punkte[i], mitte, punkte[i + 1]], [radien[i] * 1.12, radien[i] * 1.0, radien[i + 1] * 0.96], 8, ring_mat if i % 2 else stamm_mat, spitze=False)
    krone = punkte[-1]
    bm = baum.bm

    wedel = r.randint(14, 16)
    for k in range(wedel):
        a = math.tau * k / wedel + r.uniform(-0.18, 0.18)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        laenge = r.uniform(2.4, 3.1)
        hang = r.uniform(0.45, 0.85) * (1.5 if k % 4 == 0 else 1.0) * (0.55 if k % 3 == 1 else 1.0)
        laenge *= 0.75 if k % 3 == 1 else 1.0  # innere, jüngere Wedel: kürzer und steiler
        rippe = [krone]
        d = (aussen + Vector((0, 0, r.uniform(0.4, 0.8)))).normalized()
        segmente = 9
        for i in range(segmente):
            d = (d + Vector((0, 0, -hang / 3.5))).normalized()
            rippe.append(rippe[-1] + d * laenge / segmente)
        seite = aussen.cross(Vector((0, 0, 1))).normalized()
        # Mittelrippe als schmaler Streifen
        for i in range(segmente):
            p0, p1 = rippe[i], rippe[i + 1]
            bm.faces.new((bm.verts.new(p0 - seite * 0.025), bm.verts.new(p0 + seite * 0.025), bm.verts.new(p1 + seite * 0.02), bm.verts.new(p1 - seite * 0.02))).material_index = blatt[1]
        # Fiederblättchen: je Segment und Seite ein schmales, hängendes Blatt
        for i in range(1, segmente + 1):
            t = i / segmente
            b = 0.72 * math.sin(math.pi * min(0.95, t * 0.95 + 0.05)) + 0.08
            for s in (1, -1):
                basis_a = rippe[i - 1].lerp(rippe[i], 0.1)
                basis_b = rippe[i]
                richtung = (seite * s * 0.85 + d * 0.35 + Vector((0, 0, -0.45 - 0.3 * t))).normalized()
                spitze = basis_a.lerp(basis_b, 0.5) + richtung * b * r.uniform(0.8, 1.1)
                mat = spitze_mat if t > 0.82 and r.random() < 0.6 else r.choice(blatt)
                bm.faces.new((bm.verts.new(basis_a), bm.verts.new(basis_b), bm.verts.new(spitze))).material_index = mat
    for _ in range(r.randint(3, 5)):
        a = r.uniform(0, math.tau)
        baum.buschel(krone + Vector((math.cos(a) * 0.22, math.sin(a) * 0.22, -0.28)), 0.12, [nuss], stufen=1, platt=1.05, beulen=0.06)
    return baum.fertig()


# ---------------------------------------------------------------------------
# Zauberbaum: verdrehter schiefergrauer Stamm, Laub in Violett und Türkis, Leuchtfrüchte
# ---------------------------------------------------------------------------
def zauberbaum(seed=1, name="Zauberbaum"):
    baum = Baum(name, seed)
    r = baum.rng
    rinde = baum.mat("bm_zauber_rinde")
    rille = baum.mat("bm_zauber_rinde_hell")
    laub = [baum.mat("bm_zauber_violett"), baum.mat("bm_zauber_tuerkis")]
    hell = baum.mat("bm_zauber_violett_hell")
    dunkel = baum.mat("bm_zauber_dunkel")
    glut = baum.mat("bm_zauber_glut", leuchten=4.0)
    for m in laub:
        baum.licht[m] = (hell, dunkel)
    fruechte = []

    def ast(start, richtung, laenge, radius, tiefe):
        schritte = max(3, int(laenge / 0.32))
        # Verdreht und knorrig; Äste krümmen sich nach oben wie Finger
        richtung = richtung.copy()
        richtung.z = max(richtung.z, 0.3)  # kein Ast hängt nach unten
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.4, 0.0, 0.16 + 0.08 * tiefe)
        radien = [radius * (1 - 0.68 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, radien, max(5, 7 - tiefe), rinde, streifen=rille)
        baum.aeste.append((punkte, radien, tiefe, False))
        if tiefe >= 3 or radien[-1] < 0.03:
            baum.buschel(punkte[-1], r.uniform(0.42, 0.62), laub, stufen=2 if r.random() < 0.5 else 1)
            for _ in range(r.randint(1, 2)):
                neben = punkte[-1] + baum.zufallsrichtung() * 0.45
                baum.buschel(neben, r.uniform(0.22, 0.32), laub, stufen=1, beulen=0.12)
            for _ in range(r.randint(1, 2)):
                fruechte.append(punkte[-1] + baum.zufallsrichtung() * 0.3 + Vector((0, 0, -0.35)))
            return
        for k in range(r.randint(2, 3)):
            i = min(schritte, int(r.uniform(0.5, 1.0) * schritte))
            d = (punkte[min(i + 1, schritte)] - punkte[max(i - 1, 0)]).normalized()
            ast(punkte[i], _drehen(d, r.uniform(0.55, 1.0), r.uniform(0, math.tau)), laenge * 0.72, radien[i] * 0.62, tiefe + 1)

    stamm = baum.pfad(Vector((0, 0, 0)), Vector((0, 0, 1)), r.uniform(2.5, 3.1), 7, 0.28, 0.0, 0.05)
    stamm_radien = [0.34 * (1.25 - 0.45 * i / 7) for i in range(8)]
    baum.rohr(stamm, stamm_radien, 9, rinde, spitze=False, streifen=rille)
    _wurzeln(baum, 0.32, 6, rinde, 1.35, streifen=rille)
    for k in range(3):
        ast(stamm[-1], _drehen(Vector((0, 0, 1)), r.uniform(0.5, 0.85), math.tau * k / 3 + r.uniform(-0.3, 0.3)), 2.4, stamm_radien[-1] * 0.72, 1)
    for p in fruechte:
        baum.buschel(p, r.uniform(0.09, 0.13), [glut], stufen=1, platt=1.2, beulen=0.08)
    return baum.fertig()
