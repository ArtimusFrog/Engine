"""Baum-Generator im detaillierten Polygon-Stil mit Dark-Fantasy-Anstrich.

Flache Flächen wie bisher, aber mehr davon: knorrige, sich verzweigende Äste, die sich unter
ihrem Gewicht biegen, Wurzeln, die aus dem Boden brechen, zerzauste Kronen aus Blattbüscheln,
tote Äste und herabhängende Flechten. Gedeckte, entsättigte Farben.

Jede Art ist eine Funktion (eiche, tanne, palme, zauberbaum), `seed` ergibt die Variante.
Ein Modell-Skript ruft sie nur auf, z. B. art/modelle/natur/eiche_1.py: `eiche(seed=1)`.
Koordinaten wie in der Werkstatt: Z oben, Ursprung am Stammfuß.
"""

import math
import random

import bmesh
import bpy
from mathutils import Quaternion, Vector

from werkstatt import PALETTE, material

# Dark-Fantasy-Palette (in die Werkstatt-Palette eingetragen, damit alles an einer Stelle bleibt)
PALETTE.update({
    "df_rinde": "#3A302A",
    "df_rinde_dunkel": "#26201C",
    "df_rinde_hell": "#5B4C40",
    "df_laub_dunkel": "#1F3522",
    "df_laub": "#2C472A",
    "df_laub_oliv": "#46512D",
    "df_laub_welk": "#5E5230",
    "df_moos": "#4A5A31",
    "df_flechte": "#76826A",
    "df_nadel_dunkel": "#18302B",
    "df_nadel": "#233F37",
    "df_nadel_hell": "#2F5044",
    "df_schnee": "#D5DCE3",
    "df_palme_stamm": "#4D4236",
    "df_palme_ring": "#3A3129",
    "df_palmblatt": "#34512F",
    "df_palmblatt_welk": "#6A6236",
    "df_nuss": "#2E241C",
    "df_zauber_rinde": "#1B181F",
    "df_zauber_blatt": "#33294A",
    "df_zauber_blatt_hell": "#4A3A66",
    "df_zauber_glut": "#A57BFF",
})


class Baum:
    """Sammelt die Geometrie eines Baums in einem bmesh, mit Material je Fläche."""

    def __init__(self, name, seed):
        self.name = name
        self.bm = bmesh.new()
        self.rng = random.Random(seed)
        self.materialien = []
        self.aeste = []  # (Punkte, Radien, Tiefe, tot) – für Flechten und Blätter

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
    def rohr(self, punkte, radien, ecken, mat, spitze=True):
        """Konisches Rohr entlang einer Punktfolge (Äste, Stamm, Wurzeln)."""
        bm = self.bm
        ringe = []
        normale = None
        versatz = self.rng.uniform(0, math.tau)
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
                # Leicht unregelmäßige Rinde
                r = radien[i] * self.rng.uniform(0.88, 1.08)
                ring.append(bm.verts.new(p + (normale * math.cos(a) + binormale * math.sin(a)) * r))
            ringe.append(ring)
        for a, b in zip(ringe, ringe[1:]):
            for k in range(ecken):
                flaeche = bm.faces.new((a[k], a[(k + 1) % ecken], b[(k + 1) % ecken], b[k]))
                flaeche.material_index = mat
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

    def buschel(self, mitte, groesse, mats, stufen=2, platt=0.72):
        """Blattbüschel: verbeulte, flach schattierte Kugel mit gemischten Blattfarben."""
        ergebnis = bmesh.ops.create_icosphere(self.bm, subdivisions=stufen, radius=groesse)
        verts = ergebnis["verts"]
        drehung = Quaternion(self.zufallsrichtung().normalized() or Vector((0, 0, 1)), self.rng.uniform(0, math.tau))
        for v in verts:
            p = drehung @ v.co
            p.z *= platt
            p += self.zufallsrichtung() * groesse * 0.18
            v.co = mitte + p
        flaechen = {f for v in verts for f in v.link_faces}
        for f in flaechen:
            f.material_index = self.rng.choice(mats)

    def flechte(self, oben, laenge, mat):
        """Herabhängender Flechtenstreifen (zwei Zickzack-Segmente, beidseitig sichtbar)."""
        bm = self.bm
        breite = self.rng.uniform(0.05, 0.1)
        seite = self.zufallsrichtung()
        seite.z = 0
        seite = seite.normalized() * breite if seite.length > 0.01 else Vector((breite, 0, 0))
        punkte = [oben]
        for i in range(3):
            versatz = self.zufallsrichtung() * 0.06
            versatz.z = 0
            punkte.append(punkte[-1] + Vector((0, 0, -laenge / 3)) + versatz)
        links = [bm.verts.new(p - seite * (1 - i / 3)) for i, p in enumerate(punkte)]
        rechts = [bm.verts.new(p + seite * (1 - i / 3)) for i, p in enumerate(punkte)]
        for i in range(3):
            bm.faces.new((links[i], rechts[i], rechts[i + 1], links[i + 1])).material_index = mat

    # ------------------------------------------------------------------
    # Fertigstellen
    # ------------------------------------------------------------------
    def fertig(self):
        """Mesh-Objekt erzeugen: Normalen nach außen, flach schattiert, nichts unter dem Boden."""
        bm = self.bm
        for v in bm.verts:
            if v.co.z < 0:
                v.co.z = 0.0
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
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


def _wurzeln(baum, radius, anzahl, mat, spreizung=1.0):
    """Wurzeln brechen aus dem Boden und laufen flach aus."""
    for i in range(anzahl):
        a = math.tau * i / anzahl + baum.rng.uniform(-0.3, 0.3)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        start = aussen * radius * 0.55 + Vector((0, 0, radius * 1.1))
        punkte = baum.pfad(start, aussen + Vector((0, 0, -0.6)), radius * baum.rng.uniform(2.2, 3.2) * spreizung, 4, 0.2, 0.25)
        radien = [radius * 0.5 * (1 - 0.8 * j / 4) for j in range(5)]
        baum.rohr(punkte, radien, 5, mat)


# ---------------------------------------------------------------------------
# Laubbaum: knorrige Eiche
# ---------------------------------------------------------------------------
def eiche(seed=1, name="Eiche"):
    baum = Baum(name, seed)
    r = baum.rng
    rinde = baum.mat("df_rinde")
    rinde_hell = baum.mat("df_rinde_hell")
    laub = [baum.mat("df_laub_dunkel"), baum.mat("df_laub"), baum.mat("df_laub_oliv")]
    welk = baum.mat("df_laub_welk")
    flechte = baum.mat("df_flechte", beidseitig=True)

    stamm_radius = r.uniform(0.3, 0.38)
    stamm_hoehe = r.uniform(2.1, 2.7)

    def ast(start, richtung, laenge, radius, tiefe, tot):
        schritte = max(3, int(laenge / 0.45))
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.26, 0.035 * tiefe, 0.1)
        radien = [radius * (1 - 0.55 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, radien, max(4, 7 - tiefe), rinde_hell if tot else rinde)
        baum.aeste.append((punkte, radien, tiefe, tot))
        if tiefe >= 3 or radien[-1] < 0.035:
            if not tot:
                baum.buschel(punkte[-1], r.uniform(0.6, 0.88), laub + ([welk] if r.random() < 0.3 else []))
            return
        kinder = r.randint(2, 3) if tiefe < 2 else r.randint(1, 3)
        for k in range(kinder):
            u = r.uniform(0.45, 1.0)
            i = min(schritte, int(u * schritte))
            d = (punkte[min(i + 1, schritte)] - punkte[max(i - 1, 0)]).normalized()
            neu = _drehen(d, r.uniform(0.45, 0.95), math.tau * k / kinder + r.uniform(-0.6, 0.6))
            kind_tot = tot or (tiefe >= 1 and r.random() < 0.12)
            ast(punkte[i], neu, laenge * r.uniform(0.62, 0.78), radien[i] * 0.62, tiefe + 1, kind_tot)
        # Zusätzliches Büschel mitten auf äußeren Ästen macht die Krone dichter.
        if tiefe >= 2 and not tot:
            mitte = punkte[len(punkte) // 2]
            baum.buschel(mitte + Vector((0, 0, 0.25)), r.uniform(0.52, 0.72), laub)

    # Stamm: kurz, dick, leicht gedreht; teilt sich in 3–4 kräftige Hauptäste
    stamm = baum.pfad(Vector((0, 0, 0)), Vector((0, 0, 1)), stamm_hoehe, 5, 0.06, 0.0, 0.25)
    stamm_radien = [stamm_radius * (1.25 - 0.35 * i / 5) for i in range(6)]
    baum.rohr(stamm, stamm_radien, 8, rinde, spitze=False)
    _wurzeln(baum, stamm_radius, r.randint(4, 6), rinde)
    haupt = r.randint(3, 4)
    for k in range(haupt):
        richtung = _drehen(Vector((0, 0, 1)), r.uniform(0.75, 1.05), math.tau * k / haupt + r.uniform(-0.3, 0.3))
        ast(stamm[-1], richtung, r.uniform(2.6, 3.2), stamm_radien[-1] * 0.72, 1, False)
    # Mittlerer Leittrieb nach oben, damit die Krone auch oben geschlossen ist
    ast(stamm[-1], _drehen(Vector((0, 0, 1)), 0.15, r.uniform(0, math.tau)), r.uniform(1.8, 2.2), stamm_radien[-1] * 0.55, 2, False)

    # Flechten hängen von den unteren Ästen
    kandidaten = [(p, t) for punkte, _, t, _ in baum.aeste if 1 <= t <= 2 for p in punkte[1:]]
    for p, _ in r.sample(kandidaten, min(len(kandidaten), r.randint(5, 9))):
        baum.flechte(p, r.uniform(0.45, 1.1), flechte)
    return baum.fertig()


# ---------------------------------------------------------------------------
# Nadelbaum: dunkle, zerzauste Tanne (optional verschneit)
# ---------------------------------------------------------------------------
def tanne(seed=1, schnee=False, name="Tanne"):
    baum = Baum(name, seed)
    r = baum.rng
    rinde = baum.mat("df_rinde_dunkel")
    tot_mat = baum.mat("df_rinde_hell")
    nadeln = [baum.mat("df_nadel_dunkel"), baum.mat("df_nadel"), baum.mat("df_nadel_hell")]
    schnee_mat = baum.mat("df_schnee") if schnee else None

    hoehe = r.uniform(7.5, 9.5)
    radius = r.uniform(0.2, 0.26)
    stamm = baum.pfad(Vector((0, 0, 0)), Vector((0, 0, 1)), hoehe, 8, 0.04, 0.0, 0.0)
    baum.rohr(stamm, [radius * (1.1 - i / 8) + 0.02 for i in range(9)], 6, rinde)
    _wurzeln(baum, radius, 4, rinde, 0.8)

    # Quirle von Ästen: unten lang und hängend, oben kurz; unterste Etage teils tot.
    etagen = 9
    for e in range(etagen):
        t = e / etagen
        z = hoehe * (0.18 + 0.78 * t)
        laenge = (1 - t) * 2.6 + 0.35
        anzahl = r.randint(5, 7) if t < 0.8 else r.randint(3, 5)
        for k in range(anzahl):
            a = math.tau * k / anzahl + r.uniform(-0.3, 0.3) + e * 0.7
            aussen = Vector((math.cos(a), math.sin(a), 0))
            start = Vector((0, 0, z))
            tot = e == 0 and r.random() < 0.6
            richtung = aussen + Vector((0, 0, r.uniform(-0.35, -0.1) if not tot else 0.05))
            punkte = baum.pfad(start, richtung, laenge * r.uniform(0.8, 1.1), 3, 0.12, 0.08)
            baum.rohr(punkte, [0.06 * (1 - i / 3) + 0.01 for i in range(4)], 4, tot_mat if tot else rinde)
            if tot:
                continue
            # Nadelzweig: zerzauster, flacher Kegel entlang des Asts
            _nadelzweig(baum, punkte, laenge, nadeln)
    # Spitze
    _nadelzweig(baum, [Vector((0, 0, hoehe - 1.2)), Vector((0, 0, hoehe + 0.3))], 0.9, nadeln, aufrecht=True)

    obj = baum.fertig()
    if schnee_mat is not None:
        # Schnee liegt auf allem, was nach oben zeigt.
        for polygon in obj.data.polygons:
            if polygon.normal.z > 0.62 and polygon.material_index in nadeln:
                polygon.material_index = schnee_mat
    return obj


def _nadelzweig(baum, punkte, laenge, nadeln, aufrecht=False):
    """Flacher, ausgefranster Nadelkegel entlang eines Asts (typischer Tannenzweig im Polygon-Stil)."""
    r = baum.rng
    start, ende = punkte[0], punkte[-1]
    achse = (ende - start)
    lang = achse.length
    achse.normalize()
    breite = 0.35 + laenge * 0.28 if not aufrecht else 0.55
    ergebnis = bmesh.ops.create_cone(baum.bm, cap_ends=True, cap_tris=False, segments=7, radius1=breite, radius2=0.0, depth=lang * 1.05)
    drehung = Vector((0, 0, 1)).rotation_difference(achse)
    mitte = (start + ende) / 2
    for v in ergebnis["verts"]:
        p = v.co.copy()
        if not aufrecht:
            p.x *= 1.0
            p.y *= 0.45  # abgeflacht wie ein Zweig
        p += baum.zufallsrichtung() * 0.08
        # Spitzen hängen durch
        v.co = mitte + drehung @ p + Vector((0, 0, -0.15 * (p.z / lang + 0.5) if not aufrecht else 0))
    for f in {f for v in ergebnis["verts"] for f in v.link_faces}:
        f.material_index = r.choice(nadeln)


# ---------------------------------------------------------------------------
# Palme: schiefer, geringelter Stamm, zerfledderte Wedel
# ---------------------------------------------------------------------------
def palme(seed=1, name="Palme"):
    baum = Baum(name, seed)
    r = baum.rng
    stamm_mat = baum.mat("df_palme_stamm")
    ring_mat = baum.mat("df_palme_ring")
    blatt = baum.mat("df_palmblatt", beidseitig=True)
    welk = baum.mat("df_palmblatt_welk", beidseitig=True)
    nuss = baum.mat("df_nuss")

    hoehe = r.uniform(5.2, 6.5)
    neigung = Vector((r.uniform(-1, 1), r.uniform(-1, 1), 0)).normalized()
    punkte = [Vector((0, 0, 0))]
    for i in range(1, 11):
        t = i / 10
        punkte.append(neigung * (t * t * 1.4) + Vector((0, 0, hoehe * t)))
    radien = [0.2 - 0.07 * i / 10 for i in range(11)]
    # Geringelter Stamm: abwechselnd hell/dunkel
    for i in range(10):
        baum.rohr(punkte[i : i + 2], [radien[i] * 1.08, radien[i + 1]], 7, ring_mat if i % 2 else stamm_mat, spitze=False)
    krone = punkte[-1]

    wedel = r.randint(8, 10)
    for k in range(wedel):
        a = math.tau * k / wedel + r.uniform(-0.2, 0.2)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        laenge = r.uniform(2.3, 3.0)
        abgestorben = r.random() < 0.2
        hang = r.uniform(0.35, 0.7) + (0.6 if abgestorben else 0)
        mittelrippe = [krone]
        d = (aussen + Vector((0, 0, 0.55))).normalized()
        for i in range(6):
            d = (d + Vector((0, 0, -hang / 3))).normalized()
            mittelrippe.append(mittelrippe[-1] + d * laenge / 6)
        seite = aussen.cross(Vector((0, 0, 1))).normalized()
        bm = baum.bm
        mat = welk if abgestorben else blatt
        for i in range(6):
            b0 = 0.42 * math.sin(math.pi * (i + 0.2) / 6.4)
            b1 = 0.42 * math.sin(math.pi * (i + 1.2) / 6.4)
            p0, p1 = mittelrippe[i], mittelrippe[i + 1]
            # Ausgefranste Kanten: jedes Segment eigene Zacke, leicht nach unten geklappt
            for s in (1, -1):
                aussen0 = p0 + seite * s * b0 + Vector((0, 0, -0.12))
                aussen1 = p1 + seite * s * b1 * r.uniform(0.7, 1.0) + Vector((0, 0, -0.18))
                bm.faces.new((bm.verts.new(p0), bm.verts.new(p1), bm.verts.new(aussen1), bm.verts.new(aussen0))).material_index = mat
    for k in range(r.randint(2, 4)):
        a = r.uniform(0, math.tau)
        baum.buschel(krone + Vector((math.cos(a) * 0.2, math.sin(a) * 0.2, -0.25)), 0.11, [nuss], stufen=1, platt=1.0)
    return baum.fertig()


# ---------------------------------------------------------------------------
# Zauberbaum: schwarzer, verdrehter Stamm, Klauenäste, glimmende Früchte
# ---------------------------------------------------------------------------
def zauberbaum(seed=1, name="Zauberbaum"):
    baum = Baum(name, seed)
    r = baum.rng
    rinde = baum.mat("df_zauber_rinde")
    laub = [baum.mat("df_zauber_blatt"), baum.mat("df_zauber_blatt_hell")]
    glut = baum.mat("df_zauber_glut", leuchten=4.0)
    fruechte = []

    def ast(start, richtung, laenge, radius, tiefe):
        schritte = max(3, int(laenge / 0.35))
        # Stark verdreht und knorrig; Äste krümmen sich wie Klauen nach oben
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.45, 0.0, 0.1 + 0.08 * tiefe)
        radien = [radius * (1 - 0.7 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, radien, max(4, 6 - tiefe), rinde)
        baum.aeste.append((punkte, radien, tiefe, False))
        if tiefe >= 3 or radien[-1] < 0.03:
            if r.random() < 0.55:
                baum.buschel(punkte[-1], r.uniform(0.35, 0.6), laub, stufen=1)
            if r.random() < 0.6:
                fruechte.append(punkte[-1] + Vector((0, 0, -0.25)))
            return
        for k in range(r.randint(2, 3)):
            i = min(schritte, int(r.uniform(0.5, 1.0) * schritte))
            d = (punkte[min(i + 1, schritte)] - punkte[max(i - 1, 0)]).normalized()
            ast(punkte[i], _drehen(d, r.uniform(0.6, 1.1), r.uniform(0, math.tau)), laenge * 0.7, radien[i] * 0.6, tiefe + 1)

    stamm = baum.pfad(Vector((0, 0, 0)), Vector((0, 0, 1)), r.uniform(2.4, 3.0), 6, 0.3, 0.0, 0.0)
    stamm_radien = [0.32 * (1.2 - 0.45 * i / 6) for i in range(7)]
    baum.rohr(stamm, stamm_radien, 7, rinde, spitze=False)
    _wurzeln(baum, 0.3, 6, rinde, 1.3)
    for k in range(3):
        ast(stamm[-1], _drehen(Vector((0, 0, 1)), r.uniform(0.5, 0.9), math.tau * k / 3), 2.2, stamm_radien[-1] * 0.7, 1)
    # Glimmende Früchte hängen an den Astenden
    for p in fruechte:
        baum.buschel(p, 0.1, [glut], stufen=1, platt=1.2)
    return baum.fertig()
