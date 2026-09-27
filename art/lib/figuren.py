"""Spielfiguren – sehr detailliert, realistische Proportionen, im Farbstil von Bäumen und Tieren.

Aufbau einer Figur:
- Teile aus Querschnitt-Ringen („Lofts“: Robe, Ärmel, Hut, Strähnen …), fein unterteilt,
  und weichen Metaballs (Kopf mit ausgeformtem Gesicht, Hände mit einzelnen Fingern);
  Metaballs können auch aushöhlen (Augenhöhlen, Mundspalte)
- Kleidung facettiert, Haut weich schattiert; Farben als Vertexfarben je Fläche
  (Stickereien, Falten, Wangen … entstehen so ohne Textur)
- Gewichte werden pro Teil ausgerechnet: Robe schwingt mit den Oberschenkeln, Ärmel folgen
  Ober- und Unterarm, Hutspitze wippt an eigenem Knochen, Bart und Haare folgen dem Kopf
- Skelett: Becken, Bauch, Brust, Hals, Kopf, Hut, Arme (Ober-, Unterarm, Hand), Beine
  (Ober-, Unterschenkel, Fuß)
- Animationen: Idle, Laufen, Rennen, Springen, Hieb, Werfen

Koordinaten wie in der Werkstatt: Z oben, die Figur schaut nach -Y, Füße im Ursprung.
Links (L) ist +X, rechts (R) ist -X.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Quaternion, Vector

from werkstatt import animation, ruhepose, skelett, srgb_zu_linear

_SICHTBAR = 0.575
HOCH = (0, -1, 0)    # Knochen, die nach oben zeigen (Rumpf, Kopf): +X beugt nach vorne
HAENGT = (0, 1, 0)   # Knochen, die nach unten zeigen (Arme, Beine): +X schwingt nach hinten
X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))


def farbe(hex_farbe):
    return Vector(srgb_zu_linear(hex_farbe)[:3])


def weich(kante0, kante1, x):
    t = max(0.0, min(1.0, (x - kante0) / (kante1 - kante0)))
    return t * t * (3 - 2 * t)


class Figur:
    def __init__(self, name, seed=1):
        self.name = name
        self.rng = random.Random(seed)
        self.teile = []
        self.knochen = []
        # Starre Anbauteile (Waffen, Werkzeuge): (Name, Knochen, Objekte)
        self.starr = []

    def als_starr(self, gruppe, knochen, anfang):
        """Alle Teile ab Index `anfang` werden ein starres Anbauteil am Knochen (z. B. etwas in der
        Hand). Im Spiel ist es ein eigener Knoten, den man ein- und ausblenden kann."""
        objekte = self.teile[anfang:]
        del self.teile[anfang:]
        self.starr.append((gruppe, knochen, objekte))

    # --- Teile ------------------------------------------------------------
    def _objekt(self, bm, name, farbe_von, gewichte_von, glatt=False):
        """bmesh → Objekt mit Vertexfarben (je Fläche) und Gewichten (je Ecke)."""
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        mesh = bpy.data.meshes.new(name)
        bm.to_mesh(mesh)
        bm.free()
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        self._faerben(obj, farbe_von, glatt)
        self._gewichten(obj, gewichte_von)
        self.teile.append(obj)
        return obj

    def _faerben(self, obj, farbe_von, glatt=False, schwankung=0.045):
        mesh = obj.data
        attr = mesh.color_attributes.new("Farbe", "FLOAT_COLOR", "CORNER")
        for poly in mesh.polygons:
            c = farbe_von(poly) * self.rng.uniform(1 - schwankung, 1 + schwankung * 0.8)
            for li in poly.loop_indices:
                attr.data[li].color = (c.x, c.y, c.z, 1.0)
            poly.use_smooth = glatt

    def _gewichten(self, obj, gewichte_von):
        gruppen = {}
        for v in obj.data.vertices:
            for knochen, w in gewichte_von(v.co).items():
                if w <= 0.001:
                    continue
                if knochen not in gruppen:
                    gruppen[knochen] = obj.vertex_groups.new(name=knochen)
                gruppen[knochen].add([v.index], w, "REPLACE")

    def loft(self, name, ringe, segmente, farbe_von, gewichte_von, oben_zu=False, unten_zu=False, teilung=1, glatt=False):
        """Fläche aus Querschnitt-Ringen. `ringe` = Liste von (Mitte, Achse1, Achse2, r1, r2) oder
        (…, Form(winkel) → Faktor). `teilung` fügt zwischen je zwei Ringen weitere ein (feiner).
        `farbe_von(ring, segment, poly)`: `ring` ist die Lage zwischen den Originalringen
        (0.5 = auf halbem Weg vom ersten zum zweiten), `segment` 0 … segmente-1."""
        fein = []
        for i, ring in enumerate(ringe):
            if i == len(ringe) - 1 or teilung == 1:
                fein.append((i, ring))
                continue
            naechster = ringe[i + 1]
            for j in range(teilung):
                t = j / teilung
                fa = ring[5] if len(ring) > 5 else None
                fb = naechster[5] if len(naechster) > 5 else None
                form = (lambda w, fa=fa, fb=fb, t=t: (fa(w) if fa else 1.0) * (1 - t) + (fb(w) if fb else 1.0) * t)
                fein.append((i + t, (ring[0].lerp(naechster[0], t), ring[1].lerp(naechster[1], t).normalized(),
                                     ring[2].lerp(naechster[2], t).normalized(), ring[3] + (naechster[3] - ring[3]) * t,
                                     ring[4] + (naechster[4] - ring[4]) * t, form)))
        bm = bmesh.new()
        reihen = []
        for _, ring in fein:
            mitte, a1, a2, r1, r2 = ring[:5]
            form = ring[5] if len(ring) > 5 else None
            reihe = []
            for k in range(segmente):
                w = math.tau * k / segmente
                f = form(w) if form else 1.0
                reihe.append(bm.verts.new(mitte + (a1 * math.cos(w) * r1 + a2 * math.sin(w) * r2) * f))
            reihen.append(reihe)
        lage = {}
        for n, (a, b) in enumerate(zip(reihen, reihen[1:])):
            for k in range(segmente):
                f = bm.faces.new((a[k], a[(k + 1) % segmente], b[(k + 1) % segmente], b[k]))
                lage[f] = (fein[n][0], k)
        if unten_zu:
            lage[bm.faces.new(list(reversed(reihen[0])))] = (-1, 0)
        if oben_zu:
            spitze = bm.verts.new(sum((v.co for v in reihen[-1]), Vector()) / segmente)
            for k in range(segmente):
                lage[bm.faces.new((reihen[-1][k], reihen[-1][(k + 1) % segmente], spitze))] = (fein[-1][0], k)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.faces.ensure_lookup_table()
        reihenfolge = [lage.get(f, (0, 0)) for f in bm.faces]
        return self._objekt(bm, name, lambda poly: farbe_von(*reihenfolge[poly.index], poly), gewichte_von, glatt)

    def straehne(self, name, punkte, dicke_start, dicke_ende, c, gewichte_von, segmente=6, drall=0.0, flach=1.0, teilung=2):
        """Spitz zulaufende Strähne (Haar, Bart, Braue, Kralle) entlang einer Punktfolge."""
        ringe = []
        normale = None
        for i, p in enumerate(punkte):
            t = (punkte[min(i + 1, len(punkte) - 1)] - punkte[max(i - 1, 0)]).normalized()
            if normale is None:
                normale = t.cross(Z if abs(t.z) < 0.9 else X).normalized()
            else:
                normale = (normale - t * normale.dot(t)).normalized()
            q = Quaternion(t, drall * i)
            a1, a2 = q @ normale, q @ t.cross(normale)
            s = i / (len(punkte) - 1)
            r = dicke_start + (dicke_ende - dicke_start) * s
            ringe.append((p, a1, a2, r, r * flach))
        farbe_von = c if callable(c) else (lambda i, k, poly: c)
        return self.loft(name, ringe, segmente, farbe_von, gewichte_von, oben_zu=True, unten_zu=True, teilung=teilung)

    def metaball(self, name, formen, aufloesung, ziel, farbe_von, gewichte_von, glatt=False):
        """Weiche Form aus Ellipsoiden (Mitte, Halbachsen[, aushöhlen]), vereinfacht auf `ziel` Dreiecke."""
        kugeln = bpy.data.metaballs.new(name + "Form")
        kugeln.resolution = aufloesung
        kugeln.render_resolution = aufloesung
        kugeln.threshold = 0.6
        for form in formen:
            mitte, (hx, hy, hz) = form[0], form[1]
            e = kugeln.elements.new(type="ELLIPSOID")
            e.co = mitte
            e.radius = 1.0
            e.size_x, e.size_y, e.size_z = hx / _SICHTBAR, hy / _SICHTBAR, hz / _SICHTBAR
            e.stiffness = 2.0
            if len(form) > 2 and form[2]:
                e.use_negative = True
        obj_form = bpy.data.objects.new(name + "Form", kugeln)
        bpy.context.scene.collection.objects.link(obj_form)
        bpy.context.view_layer.update()
        bpy.ops.object.select_all(action="DESELECT")
        obj_form.select_set(True)
        bpy.context.view_layer.objects.active = obj_form
        bpy.ops.object.convert(target="MESH")
        obj = bpy.context.active_object
        obj.name = name
        dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
        if dreiecke > ziel:
            mod = obj.modifiers.new("Vereinfachen", "DECIMATE")
            mod.ratio = ziel / dreiecke
            bpy.ops.object.modifier_apply(modifier=mod.name)
        self._faerben(obj, farbe_von, glatt, schwankung=0.015 if glatt else 0.045)
        self._gewichten(obj, gewichte_von)
        self.teile.append(obj)
        return obj

    def kugel(self, name, ort, groesse, c, gewichte_von, segmente=12, ringe=8, glatt=True):
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=segmente, v_segments=ringe, radius=1.0)
        for v in bm.verts:
            v.co = Vector((v.co.x * groesse[0], v.co.y * groesse[1], v.co.z * groesse[2])) + Vector(ort)
        return self._objekt(bm, name, c if callable(c) else (lambda poly: c), gewichte_von, glatt)

    def kiste(self, name, ort, groesse, c, gewichte_von, drehung=None):
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        for v in bm.verts:
            p = Vector((v.co.x * groesse[0], v.co.y * groesse[1], v.co.z * groesse[2]))
            if drehung:
                p = drehung @ p
            v.co = p + Vector(ort)
        return self._objekt(bm, name, lambda poly: c, gewichte_von)

    def stern(self, name, ort, normale, groesse, c, gewichte_von, zacken=5):
        """Flacher Stern (Stickerei, Abzeichen), knapp über der Oberfläche."""
        normale = normale.normalized()
        a1 = normale.cross(Z if abs(normale.z) < 0.9 else X).normalized()
        a2 = normale.cross(a1)
        bm = bmesh.new()
        mitte = bm.verts.new(ort + normale * 0.003)
        rand = []
        for k in range(zacken * 2):
            w = math.pi * k / zacken + math.pi / 2
            r = groesse if k % 2 == 0 else groesse * 0.42
            rand.append(bm.verts.new(ort + normale * 0.003 + (a1 * math.cos(w) + a2 * math.sin(w)) * r))
        for k in range(zacken * 2):
            bm.faces.new((mitte, rand[k], rand[(k + 1) % (zacken * 2)]))
        obj = self._objekt(bm, name, lambda poly: c, gewichte_von)
        # Nach außen gerichtet (recalc kennt bei einer offenen Scheibe die Seite nicht)
        for poly in obj.data.polygons:
            if poly.normal.dot(normale) < 0:
                poly.flip()
        return obj

    # --- Fertigstellen ----------------------------------------------------
    def knochen_dazu(self, name, kopf, ende, eltern, oben):
        self.knochen.append((name, tuple(kopf), tuple(ende), eltern, oben))

    def fertig(self, animationen):
        bpy.ops.object.select_all(action="DESELECT")
        for teil in self.teile:
            teil.select_set(True)
        haupt = self.teile[0]
        bpy.context.view_layer.objects.active = haupt
        bpy.ops.object.join()
        haupt.name = self.name

        mat = bpy.data.materials.new(self.name)
        try:
            mat.use_nodes = True
        except (AttributeError, TypeError):
            pass
        knoten, links = mat.node_tree.nodes, mat.node_tree.links
        bsdf = next(k for k in knoten if k.type == "BSDF_PRINCIPLED")
        vc = knoten.new("ShaderNodeVertexColor")
        vc.layer_name = "Farbe"
        links.new(vc.outputs["Color"], bsdf.inputs["Base Color"])
        bsdf.inputs["Roughness"].default_value = 0.85
        haupt.data.materials.clear()
        haupt.data.materials.append(mat)

        armatur = skelett(self.name + "Skelett", self.knochen)
        haupt.parent = armatur
        mod = haupt.modifiers.new("Skelett", "ARMATURE")
        mod.object = armatur
        # Starre Anbauteile an ihren Knochen hängen (die Eckpunkte liegen schon an der richtigen Stelle).
        for gruppe, knochen, objekte in self.starr:
            bpy.ops.object.select_all(action="DESELECT")
            for obj in objekte:
                obj.select_set(True)
            bpy.context.view_layer.objects.active = objekte[0]
            bpy.ops.object.join()
            teil = objekte[0]
            teil.name = gruppe
            teil.data.name = gruppe
            teil.vertex_groups.clear()
            teil.data.materials.clear()
            teil.data.materials.append(mat)
            bone = armatur.data.bones[knochen]
            teil.parent = armatur
            teil.parent_type = "BONE"
            teil.parent_bone = knochen
            # Knochen-Eltern sitzen am Ende des Knochens: diese Lage wieder herausrechnen.
            teil.matrix_parent_inverse = (armatur.matrix_world @ bone.matrix_local @ Matrix.Translation((0, bone.length, 0))).inverted()
            dreiecke_teil = sum(len(p.vertices) - 2 for p in teil.data.polygons)
            print(f"ANBAUTEIL {gruppe} an {knochen}: {dreiecke_teil} Dreiecke")
        animationen(armatur)
        ruhepose(armatur)
        dreiecke = sum(len(p.vertices) - 2 for p in haupt.data.polygons)
        print(f"FIGUR {self.name}: {dreiecke} Dreiecke, {len(self.knochen)} Knochen")
        return haupt


def _mischen(*paare):
    """Gewichte (Knochen, Anteil) zusammenfassen und auf Summe 1 bringen."""
    summe = {}
    for knochen, w in paare:
        if w > 0:
            summe[knochen] = summe.get(knochen, 0.0) + w
    gesamt = sum(summe.values()) or 1.0
    return {k: w / gesamt for k, w in summe.items()}


# ---------------------------------------------------------------------------
# Magier
# ---------------------------------------------------------------------------
# Gelenke (Meter). L = +X, R = -X, vorne = -Y.
SCHULTER = Vector((0.2, 0.0, 1.5))
ELLBOGEN = Vector((0.29, 0.02, 1.22))
HANDGELENK = Vector((0.34, -0.01, 0.97))
FINGER = Vector((0.36, -0.03, 0.86))
HUEFTE = Vector((0.1, 0.0, 0.95))
KNIE = Vector((0.1, 0.0, 0.52))
KNOECHEL = Vector((0.1, 0.02, 0.1))
ZEHEN = Vector((0.1, -0.13, 0.03))
STAB_X, STAB_Y = -0.37, -0.075
AUGE = Vector((0.038, -0.094, 1.752))


def _spiegel(p, seite):
    return Vector((p.x * seite, p.y, p.z))


def _rumpf_gewichte(co):
    z = co.z
    return _mischen(("Becken", 1 - weich(1.0, 1.15, z)), ("Bauch", weich(1.0, 1.15, z) * (1 - weich(1.28, 1.4, z))),
                    ("Brust", weich(1.28, 1.4, z) * (1 - weich(1.56, 1.62, z))), ("Hals", weich(1.56, 1.62, z)))


def _robe_gewichte(co):
    """Oben am Rumpf, unten schwingt der Rock mit den Oberschenkeln (links/rechts gemischt)."""
    if co.z > 0.98:
        return _rumpf_gewichte(co)
    bein = weich(0.98, 0.45, co.z)
    links = weich(-0.12, 0.12, co.x)
    return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))


def _arm_gewichte(seite):
    s = "L" if seite > 0 else "R"
    schulter, ellbogen, hand = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)

    def gewichte(co):
        oben = (ellbogen - schulter)
        t1 = (co - schulter).dot(oben) / oben.length_squared
        unten = (hand - ellbogen)
        t2 = (co - ellbogen).dot(unten) / unten.length_squared
        if t1 < 0.15:
            return _mischen(("Brust", 1 - weich(-0.1, 0.15, t1)), (f"Oberarm.{s}", weich(-0.1, 0.15, t1)))
        if t2 < 0.0:
            return _mischen((f"Oberarm.{s}", 1 - weich(-0.15, 0.1, t2)), (f"Unterarm.{s}", weich(-0.15, 0.1, t2)))
        if t2 > 0.95:
            return _mischen((f"Unterarm.{s}", 1 - weich(0.95, 1.1, t2)), (f"Hand.{s}", weich(0.95, 1.1, t2)))
        return {f"Unterarm.{s}": 1.0}
    return gewichte


def _kopf_und_brust(von, bis):
    return lambda co: _mischen(("Kopf", weich(von, bis, co.z)), ("Brust", 1 - weich(von, bis, co.z)))


def _abstand_zur_strecke(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / ab.length_squared))
    return (p - (a + ab * t)).length


def magier(seed=12, name="Magier"):
    f = Figur(name, seed)
    r = f.rng
    blau, blau_dunkel, futter = farbe("#262A36"), farbe("#1A1D26"), farbe("#101218")
    gold, gold_dunkel = farbe("#D8AE4A"), farbe("#A9812E")
    kragen, stola = farbe("#2B2436"), farbe("#9A2A36")
    haut, bart, haar = farbe("#EFC4A0"), farbe("#EDEAE3"), farbe("#D6D2CA")
    wange, lippe, nasenloch = farbe("#E9A688"), farbe("#C98A7A"), farbe("#5A3A34")
    leder, stiefel, holz = farbe("#5A3A24"), farbe("#4A3322"), farbe("#6E4B2E")
    kristall, hut_blau = farbe("#A6ECFF"), farbe("#252935")
    kopf_gewicht = lambda co: {"Kopf": 1.0}

    # ================= Robe =================
    falten = [r.uniform(0.6, 1.4) for _ in range(8)]

    def faltig(staerke, phase=0.0):
        return lambda w: 1.0 + staerke * sum(math.sin(w * (3 + i) + phase + falten[i]) * falten[i] / (3 + i) for i in range(6))

    robe_ringe = [
        (Vector((0, 0.005, 1.56)), X, Y, 0.16, 0.11),
        (Vector((0, 0.0, 1.45)), X, Y, 0.19, 0.135),
        (Vector((0, 0.0, 1.3)), X, Y, 0.18, 0.13),
        (Vector((0, 0.0, 1.12)), X, Y, 0.16, 0.12),
        (Vector((0, 0.0, 0.95)), X, Y, 0.19, 0.145, faltig(0.02)),
        (Vector((0, 0.01, 0.75)), X, Y, 0.225, 0.175, faltig(0.05)),
        (Vector((0, 0.02, 0.5)), X, Y, 0.26, 0.21, faltig(0.08)),
        (Vector((0, 0.03, 0.26)), X, Y, 0.3, 0.25, faltig(0.1)),
        (Vector((0, 0.035, 0.1)), X, Y, 0.33, 0.28, faltig(0.11)),
        (Vector((0, 0.035, 0.06)), X, Y, 0.335, 0.285, faltig(0.11)),
        (Vector((0, 0.03, 0.03)), X, Y, 0.31, 0.26, faltig(0.1)),
    ]
    ROBE_SEG = 72

    def vorne_winkel(k, seg):
        w = math.tau * k / seg
        return abs(math.atan2(math.sin(w + math.pi / 2), math.cos(w + math.pi / 2)))

    def robe_farbe(i, k, poly):
        vorne = vorne_winkel(k + 0.5, ROBE_SEG)
        if i >= 8:
            return gold                                          # Saum
        if 7.2 <= i < 8:
            # gestickte Rauten über dem Saum
            u = (k / ROBE_SEG * 18) % 1.0
            v = (i - 7.2) / 0.8
            return gold_dunkel if abs(u - 0.5) + abs(v - 0.5) < 0.33 else blau_dunkel
        if poly.center.z < 1.0 and vorne < 0.2:
            return gold * (0.95 if int(poly.center.z * 40) % 3 else 0.8)   # Borte vorne, mit Stichen
        if poly.center.z < 1.0 and vorne < 0.3:
            return blau_dunkel                                   # Schlitz
        return blau * (0.88 + 0.14 * weich(0.0, 1.5, poly.center.z))

    f.loft("Robe", robe_ringe, ROBE_SEG, robe_farbe, _robe_gewichte, teilung=3)
    f.loft("Futter", [(Vector((0, 0.03, 0.035)), X, Y, 0.3, 0.25), (Vector((0, 0.02, 0.4)), X, Y, 0.15, 0.1)], 24,
           lambda i, k, p: futter, _robe_gewichte, oben_zu=True)

    def robe_punkt(z, w):
        """Punkt und Normale auf der Robe (für Sterne)."""
        for a, b in zip(robe_ringe, robe_ringe[1:]):
            if b[0].z <= z <= a[0].z:
                t = (a[0].z - z) / (a[0].z - b[0].z)
                fa = a[5](w) if len(a) > 5 else 1.0
                fb = b[5](w) if len(b) > 5 else 1.0
                fak = fa + (fb - fa) * t
                rx, ry = (a[3] + (b[3] - a[3]) * t) * fak, (a[4] + (b[4] - a[4]) * t) * fak
                mitte = a[0].lerp(b[0], t)
                p = mitte + Vector((math.cos(w) * rx, math.sin(w) * ry, 0))
                return p, Vector((math.cos(w) / rx, math.sin(w) / ry, 0.15)).normalized()
        return None

    for _ in range(18):
        w = r.uniform(0, math.tau)
        if vorne_winkel(w / math.tau * ROBE_SEG, ROBE_SEG) < 0.45:
            continue
        treffer = robe_punkt(r.uniform(0.3, 0.92), w)
        if treffer:
            f.stern("Stern", treffer[0], treffer[1], r.uniform(0.018, 0.03), gold, _robe_gewichte)

    # ================= Schulterkragen =================
    def zacken(w):
        return 1.0 + 0.09 * max(0.0, math.cos(w * 9)) ** 2

    kragen_ringe = [
        (Vector((0, 0.01, 1.605)), X, Y, 0.08, 0.07),
        (Vector((0, 0.0, 1.57)), X, Y, 0.17, 0.13),
        (Vector((0, 0.0, 1.5)), X, Y, 0.235, 0.165, zacken),
        (Vector((0, 0.0, 1.46)), X, Y, 0.25, 0.175, zacken),
        (Vector((0, 0.0, 1.445)), X, Y, 0.245, 0.172, zacken),
    ]

    def kragen_farbe(i, k, p):
        if i >= 3:
            return gold
        if 2.5 <= i < 3:
            return gold_dunkel if (k // 2) % 4 == 0 else kragen     # Zierstiche über dem Goldrand
        return kragen * (0.9 + 0.15 * (i / 3))

    f.loft("Kragen", kragen_ringe, 72, kragen_farbe, _rumpf_gewichte, teilung=3)
    # Brosche vorne am Kragen: Goldfassung mit Edelstein
    f.kugel("Brosche", (0, -0.125, 1.565), (0.022, 0.008, 0.022), gold, _rumpf_gewichte, 16, 8, glatt=False)
    f.kugel("Stein", (0, -0.132, 1.565), (0.012, 0.006, 0.012), farbe("#B0203A"), _rumpf_gewichte, 12, 6, glatt=False)

    # ================= Stola =================
    for seite in (1, -1):
        pfad = [Vector((0.07 * seite, -0.135, 1.5)), Vector((0.075 * seite, -0.14, 1.3)), Vector((0.075 * seite, -0.135, 1.12)),
                Vector((0.085 * seite, -0.16, 0.95)), Vector((0.1 * seite, -0.2, 0.7)), Vector((0.11 * seite, -0.24, 0.45))]
        ringe = [(p, X, Y, 0.036, 0.007) for p in pfad]

        def stola_farbe(i, k, p):
            if i >= 4.3:
                return gold if (int(i * 6) % 2 == 0) else gold_dunkel   # Goldende mit Fransen-Streifen
            if any(abs(p.center.z - zz) < 0.012 for zz in (1.35, 1.1, 0.85)) and k in (0, 1, 7, 8, 9, 15):
                return gold                                              # gestickte Kreuze
            return stola

        f.loft("Stola", ringe, 16, stola_farbe, _robe_gewichte, oben_zu=True, unten_zu=True, teilung=4)

    # ================= Gürtel mit Schnalle, Tasche, Trank, Zauberbuch =================
    f.loft("Guertel", [(Vector((0, 0.0, 1.06)), X, Y, 0.172, 0.13), (Vector((0, 0.0, 1.13)), X, Y, 0.168, 0.128)], 56,
           lambda i, k, p: leder * (0.8 if k % 7 == 0 else 1.0), _rumpf_gewichte, teilung=2)
    for ort, groesse in (((0, -0.133, 1.118), (0.056, 0.012, 0.01)), ((0, -0.133, 1.072), (0.056, 0.012, 0.01)),
                         ((0.024, -0.133, 1.095), (0.01, 0.012, 0.056)), ((-0.024, -0.133, 1.095), (0.01, 0.012, 0.056))):
        f.kiste("Schnalle", ort, groesse, gold, _rumpf_gewichte)
    f.kiste("Dorn", (0, -0.136, 1.095), (0.036, 0.008, 0.006), gold_dunkel, _rumpf_gewichte)
    f.kiste("Tasche", (0.16, -0.04, 0.98), (0.08, 0.05, 0.1), leder * 1.15, _rumpf_gewichte)
    f.kiste("Taschenklappe", (0.16, -0.068, 1.02), (0.086, 0.012, 0.055), leder * 0.8, _rumpf_gewichte)
    f.kugel("Knopf", (0.16, -0.076, 1.0), (0.008, 0.005, 0.008), gold, _rumpf_gewichte, 10, 6)
    flasche = [(Vector((-0.15, -0.08, 0.93)), X, Y, 0.01, 0.01), (Vector((-0.15, -0.08, 0.94)), X, Y, 0.036, 0.036),
               (Vector((-0.15, -0.08, 0.97)), X, Y, 0.042, 0.042), (Vector((-0.15, -0.08, 0.995)), X, Y, 0.03, 0.03),
               (Vector((-0.15, -0.08, 1.01)), X, Y, 0.012, 0.012), (Vector((-0.15, -0.08, 1.035)), X, Y, 0.012, 0.012)]
    f.loft("Trank", flasche, 16, lambda i, k, p: farbe("#3FB6A8") * (1.25 if i > 3.5 else 1.0), _rumpf_gewichte, unten_zu=True, teilung=2)
    f.loft("Korken", [(Vector((-0.15, -0.08, 1.035)), X, Y, 0.014, 0.014), (Vector((-0.15, -0.08, 1.055)), X, Y, 0.012, 0.012)], 10,
           lambda i, k, p: farbe("#9C7A52"), _rumpf_gewichte, oben_zu=True, unten_zu=True)
    buch = Quaternion(Y, math.radians(-8))
    f.kiste("Buchdeckel", (0.1, 0.13, 0.99), (0.13, 0.04, 0.17), farbe("#6E1E2A"), _rumpf_gewichte, buch)
    f.kiste("Buchseiten", (0.1, 0.13, 0.99), (0.122, 0.044, 0.158), farbe("#EDE3C8"), _rumpf_gewichte, buch)
    f.kiste("Buchecken", (0.1, 0.13, 0.99), (0.134, 0.036, 0.04), gold, _rumpf_gewichte, buch)
    f.stern("Buchstern", Vector((0.1, 0.152, 0.99)), Y, 0.028, gold, _rumpf_gewichte)

    # ================= Ärmel =================
    for seite in (1, -1):
        s, e, h = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ringe = []
        for t, (a, b), radius in ((0.0, (s + Vector((-0.04 * seite, 0, 0.03)), e), 0.085), (0.5, (s, e), 0.085),
                                  (1.0, (s, e), 0.08), (0.45, (e, h), 0.095), (0.85, (e, h), 0.13), (1.05, (e, h), 0.15),
                                  (1.1, (e, h), 0.145)):
            achse = (b - a).normalized()
            q = achse.cross(Y).normalized()  # gleiche Lage an Ober- und Unterarm, sonst verdreht sich der Ärmel
            ringe.append((a.lerp(b, t), q, achse.cross(q).normalized(), radius, radius * 0.9, faltig(0.03, seite)))

        def aermel_farbe(i, k, p):
            if i >= 5:
                return gold
            if 4.4 <= i < 5:
                u = (k / 40 * 10) % 1.0
                return gold_dunkel if abs(u - 0.5) + abs((i - 4.4) / 0.6 - 0.5) < 0.33 else blau_dunkel
            return blau * (0.93 + 0.08 * math.sin(k / 40 * math.tau * 3))

        f.loft("Aermel", ringe, 40, aermel_farbe, _arm_gewichte(seite), teilung=3)
        achse = (h - e).normalized()
        q = achse.cross(Y).normalized()
        f.loft("AermelInnen", [(e.lerp(h, 1.08), q, achse.cross(q), 0.13, 0.117), (e.lerp(h, 0.7), q, achse.cross(q), 0.06, 0.055)],
               16, lambda i, k, p: futter, _arm_gewichte(seite), oben_zu=True)

    # ================= Hände mit einzelnen Fingern =================
    def finger_kette(punkte, dicke):
        """Finger aus dicht gesetzten Kugeln (sonst zerfällt er in Klümpchen), Gelenke etwas dicker."""
        formen = []
        for a, b in zip(punkte, punkte[1:]):
            for t in (0.0, 0.25, 0.5, 0.75):
                knoechel = 1.08 if t == 0.0 else 1.0
                formen.append((a.lerp(b, t), (dicke * knoechel,) * 3))
            dicke *= 0.9
        formen.append((punkte[-1], (dicke * 0.85,) * 3))
        return formen

    # Linke Hand: locker hängend, Finger leicht gekrümmt, Handfläche zum Körper
    lh = []
    handteller = Vector((0.352, -0.022, 0.925))
    lh.append((handteller, (0.013, 0.038, 0.04)))
    lh.append((Vector((0.345, -0.02, 0.955)), (0.016, 0.03, 0.02)))
    for i in range(4):
        y = -0.042 + 0.0135 * i
        laenge = (0.03, 0.034, 0.032, 0.026)[i]
        basis = Vector((0.356, y, 0.892))
        punkte = [basis, basis + Vector((-0.002, -0.002, -laenge * 0.45)), basis + Vector((-0.008, -0.004, -laenge * 0.82)),
                  basis + Vector((-0.016, -0.004, -laenge * 1.05))]
        lh += finger_kette(punkte, 0.0072 - 0.0005 * i)
    daumen = [Vector((0.345, -0.045, 0.935)), Vector((0.342, -0.062, 0.91)), Vector((0.338, -0.07, 0.888)), Vector((0.332, -0.072, 0.872))]
    lh += finger_kette(daumen, 0.0085)
    f.metaball("HandL", lh, 0.003, 2400, lambda poly: haut * (0.93 if poly.normal.x < -0.3 else 1.0),
               lambda co: {"Hand.L": 1.0}, glatt=True)

    # Rechte Hand: umgreift den Stab – Finger legen sich um den Stock
    rh = []
    stab = Vector((STAB_X, STAB_Y, 0.0))
    rh.append((Vector((STAB_X + 0.03, STAB_Y + 0.008, 0.905)), (0.014, 0.036, 0.045)))
    rh.append((Vector((STAB_X + 0.032, STAB_Y + 0.02, 0.945)), (0.016, 0.028, 0.02)))
    for i in range(4):
        z = 0.93 - 0.0165 * i
        punkte = []
        for j, w in enumerate((0.3, -0.6, -1.5, -2.4, -3.1)):
            rad = 0.031 - 0.0015 * j
            punkte.append(stab + Vector((math.cos(w) * rad, math.sin(w) * rad, z - 0.002 * j)))
        rh += finger_kette(punkte, 0.0075 - 0.0005 * i)
    daumen = [stab + Vector((0.028, -0.02, 0.945)), stab + Vector((0.012, -0.034, 0.95)), stab + Vector((-0.008, -0.032, 0.952)),
              stab + Vector((-0.022, -0.024, 0.95))]
    rh += finger_kette(daumen, 0.0085)
    f.metaball("HandR", rh, 0.003, 2400, lambda poly: haut, lambda co: {"Hand.R": 1.0}, glatt=True)

    # ================= Stiefel und Beine =================
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        ringe = [(Vector((0.1 * seite, 0.055, 0.045)), X, Z, 0.052, 0.045), (Vector((0.1 * seite, 0.05, 0.02)), X, Z, 0.058, 0.03),
                 (Vector((0.1 * seite, -0.02, 0.05)), X, Z, 0.06, 0.055), (Vector((0.1 * seite, -0.09, 0.045)), X, Z, 0.05, 0.04),
                 (Vector((0.1 * seite, -0.14, 0.05)), X, Z, 0.03, 0.024), (Vector((0.1 * seite, -0.175, 0.07)), X, Z, 0.012, 0.01),
                 (Vector((0.1 * seite, -0.18, 0.09)), X, Z, 0.003, 0.003)]
        f.loft("Stiefel", ringe, 16, lambda i, k, p: stiefel * (0.55 if p.center.z < 0.012 else 1.0),
               lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3)
        k_, h_ = _spiegel(KNIE, seite), _spiegel(HUEFTE, seite)
        ringe = [(Vector((0.1 * seite, 0.02, 0.06)), X, Y, 0.05, 0.05), (Vector((0.1 * seite, 0.02, 0.16)), X, Y, 0.058, 0.058),
                 (k_, X, Y, 0.065, 0.065), (h_ + Vector((0, 0, -0.05)), X, Y, 0.09, 0.09)]
        f.loft("Bein", ringe, 12, lambda i, k, p: stiefel * 1.1 if i < 1 else futter,
               lambda co, sn=sn: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.45, 0.6, co.z)), (f"Oberschenkel.{sn}", weich(0.45, 0.6, co.z))))

    # ================= Kopf mit ausgeformtem Gesicht =================
    kopf = [
        (Vector((0, 0.005, 1.75)), (0.093, 0.105, 0.112)),              # Schädel
        (Vector((0, -0.02, 1.685)), (0.078, 0.075, 0.07)),               # Kiefer und Wangen
        (Vector((0.05, -0.07, 1.72)), (0.03, 0.025, 0.022)),             # Wangenknochen
        (Vector((-0.05, -0.07, 1.72)), (0.03, 0.025, 0.022)),
        (Vector((0, -0.082, 1.776)), (0.068, 0.028, 0.02)),              # Brauenbogen
        (Vector((0, -0.102, 1.742)), (0.014, 0.022, 0.03)),              # Nasenrücken
        (Vector((0, -0.115, 1.722)), (0.017, 0.024, 0.021)),
        (Vector((0, -0.132, 1.706)), (0.021, 0.021, 0.019)),             # knollige Nasenspitze
        (Vector((0.019, -0.117, 1.701)), (0.012, 0.013, 0.011)),         # Nasenflügel
        (Vector((-0.019, -0.117, 1.701)), (0.012, 0.013, 0.011)),
        (Vector((0, -0.075, 1.645)), (0.035, 0.03, 0.03)),               # Kinn
        (Vector((0, -0.098, 1.672)), (0.02, 0.012, 0.007)),              # Unterlippe
        (Vector((0, -0.106, 1.679)), (0.022, 0.01, 0.0035), True),       # Mundspalte
        (Vector((0, 0.0, 1.61)), (0.052, 0.052, 0.06)),                  # Hals
    ]
    for s in (1, -1):
        kopf += [
            (_spiegel(AUGE, s) + Vector((0, -0.006, 0)), (0.02, 0.014, 0.014), True),        # Augenhöhle
            (_spiegel(AUGE, s) + Vector((0, -0.007, 0.01)), (0.018, 0.01, 0.006)),           # Oberlid
            (_spiegel(AUGE, s) + Vector((0, -0.006, -0.01)), (0.016, 0.008, 0.0045)),        # Unterlid, Tränensack
            (Vector((0.094 * s, 0.008, 1.735)), (0.014, 0.03, 0.042)),                       # Ohr
            (Vector((0.097 * s, 0.002, 1.702)), (0.011, 0.014, 0.013)),                      # Ohrläppchen
            (Vector((0.101 * s, 0.006, 1.735)), (0.006, 0.018, 0.026), True),                # Ohrmuschel
        ]
    kraehenfuesse = []
    for s in (1, -1):
        for w in (-0.45, 0.0, 0.45):
            a = Vector((0.062 * s, -0.08, 1.752))
            kraehenfuesse.append((a, a + Vector((math.cos(w) * 0.016 * s, 0.006, math.sin(w) * 0.016))))

    def gesicht(poly):
        p, n = poly.center, poly.normal
        c = haut * (0.95 if p.z < 1.66 else 1.0)
        if (p - Vector((0, -0.098, 1.672))).length < 0.019 and p.y < -0.09:
            return lippe
        if (p - Vector((0, -0.107, 1.679))).length < 0.02 and p.y < -0.095 and abs(p.z - 1.679) < 0.003:
            return lippe * 0.5                                              # Mundwinkel/Spalte
        if p.y < -0.06 and abs(p.x) < 0.065 and any(abs(p.z - zz + 0.004 * math.cos(p.x * 40)) < 0.0018 for zz in (1.792, 1.803, 1.814)):
            return c * 0.82                                                 # Stirnfalten
        if p.y < -0.05 and any(_abstand_zur_strecke(p, a, b) < 0.0016 for a, b in kraehenfuesse):
            return c * 0.85                                                 # Lachfältchen
        if abs(p.x) > 0.088 and n.x * p.x > 0 and abs(p.z - 1.73) < 0.035:
            c = c.lerp(wange * 0.9, 0.5)                                   # Ohren etwas röter
        rosig = math.exp(-(((abs(p.x) - 0.052) ** 2 + (p.z - 1.718) ** 2) / 0.018 ** 2)) if p.y < -0.05 else 0.0
        rosig = max(rosig, 0.35 * math.exp(-((p - Vector((0, -0.132, 1.712))).length_squared / 0.014 ** 2)))
        return c.lerp(wange, rosig * 0.8)

    f.metaball("Kopf", kopf, 0.0045, 5200, gesicht,
               lambda co: _mischen(("Kopf", weich(1.6, 1.66, co.z)), ("Hals", 1 - weich(1.6, 1.66, co.z))), glatt=True)

    # Augen: Augapfel, Iris, Pupille, Glanzpunkt; Nasenlöcher
    for s in (1, -1):
        auge = _spiegel(AUGE, s)
        f.kugel("Augapfel", auge, (0.0125, 0.0125, 0.0125), farbe("#EDE6DA"), kopf_gewicht, 16, 12)
        f.kugel("Iris", auge + Vector((-0.001 * s, -0.0105, 0.0005)), (0.0068, 0.0028, 0.0068), farbe("#557A9E"), kopf_gewicht, 14, 8)
        f.kugel("Pupille", auge + Vector((-0.001 * s, -0.0122, 0.0005)), (0.0034, 0.0016, 0.0034), farbe("#0E1116"), kopf_gewicht, 10, 6)
        f.kugel("Glanz", auge + Vector((-0.004 * s, -0.0128, 0.0035)), (0.0016, 0.0008, 0.0016), farbe("#FFFFFF"), kopf_gewicht, 8, 4)
        f.kugel("Nasenloch", Vector((0.0105 * s, -0.13, 1.692)), (0.0045, 0.004, 0.0028), nasenloch, kopf_gewicht, 10, 6)

    # Buschige Augenbrauen aus einzelnen Strähnen
    for s in (1, -1):
        for j in range(8):
            x = 0.012 + 0.0095 * j
            basis = Vector((x * s, -0.106 + 1.4 * (x - 0.03) ** 2, 1.771 + 0.007 * math.sin(math.pi * x / 0.085)))
            richtung = Vector((0.65 * s, -0.25 - r.uniform(0, 0.25), 0.45 + r.uniform(-0.15, 0.25))).normalized()
            laenge = 0.016 + 0.018 * (j / 7) + r.uniform(0, 0.006)
            mitte = basis + richtung * laenge * 0.55 + Vector((0, -0.003, 0.002))
            ende = basis + richtung * laenge + Vector((0, 0, -0.004 * (j / 7)))
            f.straehne("Braue", [basis, mitte, ende], 0.0055, 0.0008, bart * r.uniform(0.9, 1.0), kopf_gewicht, 6, 0.4, 0.7)

    # Langes Haar: Grundmasse hinten und einzelne Strähnen, die unter dem Hut hervorfallen
    f.metaball("Haar", [(Vector((0, 0.04, 1.73)), (0.105, 0.09, 0.11)), (Vector((0, 0.07, 1.62)), (0.1, 0.06, 0.08)),
                        (Vector((0, 0.08, 1.54)), (0.09, 0.05, 0.06))], 0.009, 900, lambda poly: haar,
               _kopf_und_brust(1.55, 1.65))
    for n in range(24):
        theta = math.radians(78 + 204 * n / 23 + r.uniform(-4, 4))
        aussen = Vector((math.sin(theta), -math.cos(theta), 0))
        start = aussen * 0.098 + Vector((0, 0.01, 1.8 + r.uniform(-0.01, 0.01)))
        ende_z = 1.46 + r.uniform(0, 0.1) + (0.08 if abs(math.cos(theta)) < 0.4 else 0.0)
        punkte = []
        for j in range(7):
            t = j / 6
            welle = aussen.cross(Z) * math.sin(t * math.pi * 2.2 + n) * 0.012 * t
            weiter = 0.1 + 0.045 * math.sin(t * math.pi * 0.8) + 0.02 * t
            punkte.append(aussen * weiter + Vector((0, 0.01 + 0.03 * t, 1.8 - (1.8 - ende_z) * t ** 1.1)) + welle)
        f.straehne("Haar", punkte, r.uniform(0.012, 0.017), 0.0025, haar * r.uniform(0.86, 1.05), _kopf_und_brust(1.55, 1.68), 7, 0.5, 0.75)

    # ================= Bart: Grundmasse, Locken, Schnurrbart =================
    bart_ringe = []
    for t in (0.0, 0.12, 0.3, 0.48, 0.66, 0.84):
        z = 1.672 - 0.42 * t
        breite = 0.068 * math.sin(math.pi * min(1.0, 0.35 + t)) ** 0.6 + 0.012
        tiefe = 0.038 * (1 - t) ** 0.6 + 0.012
        bart_ringe.append((Vector((0, -0.072 - 0.075 * t, z)), X, Y, breite, tiefe, lambda w: 1.0 + 0.1 * math.sin(w * 7)))
    bart_ringe.append((Vector((0, -0.152, 1.23)), X, Y, 0.004, 0.004))
    f.loft("Bart", bart_ringe, 28, lambda i, k, p: bart * (0.9 + 0.1 * ((k * 7 + int(i * 3)) % 5) / 4), _kopf_und_brust(1.45, 1.62),
           oben_zu=True, teilung=3)
    for n in range(16):
        x0 = r.uniform(-0.06, 0.06)
        z0 = 1.668 - abs(x0) * 0.4 - r.uniform(0, 0.02)
        laenge = r.uniform(0.2, 0.42) * (1 - abs(x0) * 4)
        start = Vector((x0, -0.08 - 0.02 * (1 - abs(x0) / 0.06), z0))
        punkte = []
        for j in range(5):
            t = j / 4
            welle = math.sin(t * math.pi * 2 + n) * 0.008
            punkte.append(start + Vector((-x0 * 0.55 * t + welle, -0.07 * t - 0.012 * math.sin(math.pi * t), -laenge * t)))
        f.straehne("Bartlocke", punkte, r.uniform(0.013, 0.02), 0.0015, bart * r.uniform(0.86, 1.02), _kopf_und_brust(1.45, 1.62), 7, 0.5, 0.75)
    for s in (1, -1):
        for j in range(7):
            start = Vector((s * (0.006 + 0.005 * j), -0.126 + 0.0025 * j, 1.69 - 0.0012 * j))
            ende = Vector((s * (0.058 + 0.006 * j), -0.098 + 0.004 * j, 1.635 - 0.008 * j))
            punkte = [start, start + Vector((s * 0.018, 0.003, -0.004)), start.lerp(ende, 0.6) + Vector((s * 0.01, -0.006, 0.004)), ende]
            f.straehne("Schnurrbart", punkte, 0.0075, 0.0012, bart * r.uniform(0.92, 1.03), kopf_gewicht, 6, 0.4, 0.8)

    # ================= Hut =================
    krempe = [(Vector((0, 0.005, 1.835)), X, Y, 0.125, 0.13), (Vector((0, 0.005, 1.83)), X, Y, 0.24, 0.24, faltig(0.03, 1.0)),
              (Vector((0, 0.005, 1.8)), X, Y, 0.32, 0.31, faltig(0.05, 1.0)), (Vector((0, 0.005, 1.785)), X, Y, 0.315, 0.305, faltig(0.05, 1.0)),
              (Vector((0, 0.005, 1.82)), X, Y, 0.125, 0.13)]
    f.loft("Krempe", krempe, 80, lambda i, k, p: hut_blau * (0.82 if 2 <= i < 3 else 0.9), kopf_gewicht, teilung=3)
    # Mittellinie des Kegels: gerade nach oben, dann nach hinten geknickt; Ringe stehen quer dazu
    linie = []
    for i in range(24):
        t = i / 23
        biegung = weich(0.45, 1.0, t)
        linie.append((t, Vector((0, 0.005 + 0.2 * biegung ** 1.5, 1.83 + 0.5 * t - 0.12 * biegung ** 2))))
    kegel = []
    for i, (t, mitte) in enumerate(linie):
        richtung = (linie[min(i + 1, 23)][1] - linie[max(i - 1, 0)][1]).normalized()
        quer = X.cross(richtung).normalized()
        radius = 0.128 * (1 - t) ** 0.85 + 0.004
        kegel.append((mitte, X, quer, radius, radius * 1.02, lambda w, t=t: 1.0 + 0.045 * math.sin(w * 3 + t * 7) + 0.02 * math.sin(w * 7 + t * 3)))

    def hut_farbe(i, k, poly):
        if i < 1.6:
            return gold if not 0.6 < i < 1.0 else gold_dunkel             # doppeltes Hutband
        return hut_blau * (0.93 + 0.1 * (i / 23))

    hut_gewicht = lambda co: _mischen(("Kopf", 1 - weich(2.0, 2.15, co.z)), ("Hut", weich(2.0, 2.15, co.z)))
    f.loft("Hut", kegel, 44, hut_farbe, hut_gewicht, oben_zu=True)
    f.stern("Mond", Vector((0, -0.126, 1.865)), -Y, 0.03, gold, kopf_gewicht, zacken=6)
    for t, w in ((0.35, 0.8), (0.5, 2.6), (0.6, 4.1), (0.72, 5.3), (0.42, 3.4)):
        i = int(t * 23)
        mitte = linie[i][1]
        richtung = (linie[min(i + 1, 23)][1] - linie[max(i - 1, 0)][1]).normalized()
        quer = X.cross(richtung).normalized()
        radius = 0.128 * (1 - t) ** 0.85 + 0.004
        aussen = X * math.cos(w) + quer * math.sin(w)
        f.stern("Hutstern", mitte + aussen * radius * 1.02, aussen, 0.02 * (1 - t * 0.5), gold, hut_gewicht)

    # ================= Stab =================
    stab_anfang = len(f.teile)
    stab_gewicht = lambda co: {"Hand.R": 1.0}
    stab_ringe = []
    for i in range(20):
        z = 0.03 + 1.9 * i / 19
        wackeln = Vector((0.008 * math.sin(i * 0.9), 0.008 * math.cos(i * 1.2), 0))
        knoten = 0.005 if i in (5, 12, 16) else 0.0
        stab_ringe.append((Vector((STAB_X, STAB_Y, z)) + wackeln, X, Y, 0.021 - 0.004 * i / 19 + knoten, 0.021 - 0.004 * i / 19 + knoten))

    def stab_farbe(i, k, p):
        if 0.82 < p.center.z < 1.0:
            return leder * (0.75 if int(p.center.z * 90) % 2 else 1.0)    # gewickelter Ledergriff
        return holz * (0.82 + 0.18 * ((k * 3 + int(i * 5)) % 4) / 3)

    f.loft("Stab", stab_ringe, 12, stab_farbe, stab_gewicht, unten_zu=True, teilung=2)
    oben = Vector((STAB_X, STAB_Y, 1.93))
    for n in range(3):
        w = math.tau * n / 3
        aussen = Vector((math.cos(w), math.sin(w), 0))
        punkte = [oben, oben + aussen * 0.05 + Z * 0.05, oben + aussen * 0.052 + Z * 0.11, oben + aussen * 0.03 + Z * 0.16,
                  oben + aussen * 0.008 + Z * 0.185]
        f.straehne("Kralle", punkte, 0.013, 0.003, holz * 0.9, stab_gewicht, 8, 0.3)
    bm = bmesh.new()
    spitze_o = bm.verts.new(oben + Z * 0.21)
    spitze_u = bm.verts.new(oben + Z * 0.035)
    kranz = [bm.verts.new(oben + Z * 0.11 + Vector((math.cos(math.tau * k / 8), math.sin(math.tau * k / 8), 0)) * 0.042) for k in range(8)]
    kranz2 = [bm.verts.new(oben + Z * 0.15 + Vector((math.cos(math.tau * (k + 0.5) / 8), math.sin(math.tau * (k + 0.5) / 8), 0)) * 0.03) for k in range(8)]
    for k in range(8):
        bm.faces.new((kranz[k], kranz[(k + 1) % 8], kranz2[k]))
        bm.faces.new((kranz2[k], kranz[(k + 1) % 8], kranz2[(k + 1) % 8]))
        bm.faces.new((kranz2[k], kranz2[(k + 1) % 8], spitze_o))
        bm.faces.new((kranz[(k + 1) % 8], kranz[k], spitze_u))
    f._objekt(bm, "Kristall", lambda poly: kristall * (0.85 + 0.4 * max(0.0, poly.normal.z) + 0.15 * (poly.index % 3)), stab_gewicht)
    f.als_starr("Stab", "Hand.R", stab_anfang)

    # ================= Erbeutbare Stäbe (waffen.py), im Spiel statt des Stabs sichtbar =================
    from waffen import STAEBE, stab as stab_bauen
    for art in STAEBE:
        anfang = len(f.teile)
        stab_bauen(f, art, STAB_X, STAB_Y, stab_gewicht)
        f.als_starr(art, "Hand.R", anfang)

    # ================= Spitzhacke (statt des Stabs in der Hand, im Spiel umschaltbar) =================
    hacke_anfang = len(f.teile)
    _spitzhacke(f, STAB_X, STAB_Y, stab_gewicht)
    f.als_starr("Spitzhacke", "Hand.R", hacke_anfang)

    # ================= Axt (für Bäume) =================
    axt_anfang = len(f.teile)
    _axt(f, STAB_X, STAB_Y, stab_gewicht)
    f.als_starr("Axt", "Hand.R", axt_anfang)

    # ================= Skelett =================
    f.knochen_dazu("Becken", (0, 0, 0.95), (0, 0, 1.1), None, HOCH)
    f.knochen_dazu("Bauch", (0, 0, 1.1), (0, 0, 1.3), "Becken", HOCH)
    f.knochen_dazu("Brust", (0, 0, 1.3), (0, 0, 1.56), "Bauch", HOCH)
    f.knochen_dazu("Hals", (0, 0, 1.56), (0, 0, 1.65), "Brust", HOCH)
    f.knochen_dazu("Kopf", (0, 0, 1.65), (0, 0, 1.9), "Hals", HOCH)
    f.knochen_dazu("Hut", (0, 0.02, 2.0), (0, 0.15, 2.25), "Kopf", HOCH)
    for seite, sn in ((1, "L"), (-1, "R")):
        f.knochen_dazu(f"Oberarm.{sn}", _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), "Brust", HAENGT)
        f.knochen_dazu(f"Unterarm.{sn}", _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite), f"Oberarm.{sn}", HAENGT)
        f.knochen_dazu(f"Hand.{sn}", _spiegel(HANDGELENK, seite), _spiegel(FINGER, seite), f"Unterarm.{sn}", HAENGT)
        f.knochen_dazu(f"Oberschenkel.{sn}", _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), "Becken", HAENGT)
        f.knochen_dazu(f"Unterschenkel.{sn}", _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite), f"Oberschenkel.{sn}", HAENGT)
        f.knochen_dazu(f"Fuss.{sn}", _spiegel(KNOECHEL, seite), _spiegel(ZEHEN, seite), f"Unterschenkel.{sn}", (0, 0, 1))

    return f.fertig(_magier_animationen)


# ---------------------------------------------------------------------------
# Animationen (30 Bilder pro Sekunde). Drehungen in Grad um die lokalen Achsen:
# Rumpf/Kopf +X = nach vorne beugen, +Y = um die eigene Achse drehen;
# Arme/Beine +X = nach hinten schwingen (Knie beugen = +X am Unterschenkel,
# Ellbogen beugen = -X am Unterarm).
# ---------------------------------------------------------------------------
def _schleife(laenge, schritt, werte):
    """Schlüsselbilder aus Funktionen der Phase (0..2π): werte(phi) → Liste (Knochen, art, (x, y, z))."""
    schluessel = []
    for bild in range(0, laenge + 1, schritt):
        phi = math.tau * (bild % laenge) / laenge
        for knochen, art, wert in werte(phi):
            schluessel.append((bild, knochen, art, wert))
    return schluessel


def _gehen(phi, schwung, knie, arm, huepfen, vorbeugen, ellbogen):
    s, c = math.sin(phi), math.cos(phi)
    kniel = knie * max(0.0, -c) ** 1.3 + 6
    knier = knie * max(0.0, c) ** 1.3 + 6
    return [
        ("Oberschenkel.L", "rot", (schwung * s, 0, 0)), ("Oberschenkel.R", "rot", (-schwung * s, 0, 0)),
        ("Unterschenkel.L", "rot", (kniel, 0, 0)), ("Unterschenkel.R", "rot", (knier, 0, 0)),
        ("Fuss.L", "rot", (-schwung * s * 0.3, 0, 0)), ("Fuss.R", "rot", (schwung * s * 0.3, 0, 0)),
        ("Oberarm.L", "rot", (-arm * s, 0, 0)), ("Unterarm.L", "rot", (-ellbogen - 10 * max(0.0, s), 0, 0)),
        # Rechts hält den Stab: schwingt weniger, Ellbogen etwas gebeugt
        ("Oberarm.R", "rot", (arm * 0.45 * s, 0, 0)), ("Unterarm.R", "rot", (-ellbogen * 0.8, 0, 0)),
        ("Becken", "pos", (0, -huepfen * abs(c), 0)), ("Becken", "rot", (0, 6 * s, 0)),
        ("Bauch", "rot", (vorbeugen * 0.5, 0, 0)), ("Brust", "rot", (vorbeugen * 0.5, -8 * s, 0)),
        ("Kopf", "rot", (-vorbeugen * 0.6, 2 * s, 0)), ("Hut", "rot", (4 * math.sin(2 * phi) + vorbeugen * 0.5, 0, 3 * s)),
    ]


def _spitzhacke(f, x, y, gewicht):
    """Spitzhacke, am oberen Stielende gegriffen: der Stiel hängt aus der Faust nach unten, der
    Kopf sitzt unten, die Spitze zeigt nach vorne (-Y) – beim Schwung die führende Kante."""
    holz = farbe("#8B5A2B")
    holz_dunkel = farbe("#6A4220")
    leder = farbe("#3D2A1C")
    eisen = farbe("#5E646E")
    eisen_hell = farbe("#C3CAD4")
    eisen_dunkel = farbe("#3E434B")
    X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))
    oben, unten, kopf_z = 1.02, 0.25, 0.3

    # Stiel: leicht gebaucht, oben ein Knauf, am Griff mit Leder umwickelt
    ringe = []
    for i in range(16):
        t = i / 15
        z = unten + (oben - unten) * t
        r = 0.019 + 0.004 * math.sin(t * math.pi) + (0.004 if t > 0.97 else 0.0)
        ringe.append((Vector((x, y, z)), X, Y, r, r * 0.92))

    def stiel_farbe(i, k, p):
        z = p.center.z
        if z > 0.8:
            return leder * (0.8 if int(z * 70) % 2 else 1.0)
        return (holz if (k + int(i * 3)) % 5 else holz_dunkel) * (0.9 + 0.1 * math.sin(z * 40))

    f.loft("Hackenstiel", ringe, 10, stiel_farbe, gewicht, oben_zu=True, unten_zu=True, teilung=2)

    # Eisenkopf: von der breiten Flachseite (+Y) über das Auge bis zur gebogenen Spitze (-Y).
    # Beide Enden biegen sich leicht zum Griff (nach oben).
    profil = [(0.2, 0.012, 0.05, 0.03), (0.15, 0.017, 0.042, 0.012), (0.08, 0.024, 0.034, 0.002), (0.03, 0.031, 0.037, 0.0),
              (0.0, 0.033, 0.04, 0.0), (-0.04, 0.03, 0.035, 0.0), (-0.1, 0.022, 0.026, 0.005), (-0.16, 0.016, 0.018, 0.016),
              (-0.22, 0.01, 0.011, 0.032), (-0.27, 0.004, 0.005, 0.05), (-0.295, 0.001, 0.001, 0.06)]
    kopf = [(Vector((x, y + dy, kopf_z + hoch)), X, Z, breite, hoehe) for dy, breite, hoehe, hoch in profil]

    def kopf_farbe(i, k, p):
        ende = abs(p.center.y - y) > 0.17
        return (eisen_hell if ende else eisen) * (0.92 + 0.08 * (k % 2))

    f.loft("Hackenkopf", kopf, 8, kopf_farbe, gewicht, oben_zu=True, unten_zu=True, teilung=2)
    # Eisenbeschlag, wo der Stiel durch den Kopf geht
    beschlag = [(Vector((x, y, kopf_z + dz)), X, Y, 0.028, 0.026) for dz in (-0.075, -0.05, 0.045, 0.07)]
    f.loft("Beschlag", beschlag, 10, lambda i, k, p: eisen_dunkel, gewicht, oben_zu=True, unten_zu=True)
    # Keil oben im Auge
    f.kugel("Keil", Vector((x, y, kopf_z + 0.043)), Vector((0.012, 0.024, 0.01)), eisen_dunkel, gewicht, segmente=8, ringe=4, glatt=False)


def _axt(f, x, y, gewicht):
    """Axt, am oberen Stielende gegriffen: der Stiel hängt aus der Faust nach unten (leicht nach
    außen), unten sitzt der Kopf. Die Schneide zeigt zur Körpermitte (+X) – beim waagerechten
    Schlag von rechts nach links ist sie vorne."""
    holz = farbe("#9A6534")
    holz_dunkel = farbe("#744A24")
    leder = farbe("#4A2F1C")
    eisen = farbe("#5B616B")
    eisen_hell = farbe("#D2D8E0")
    eisen_dunkel = farbe("#3B3F47")
    X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))
    oben, unten = 1.02, 0.3

    def stiel_punkt(t):
        # Leicht geschwungener Stiel („Haft“), unten etwas nach außen (-X)
        return Vector((x - 0.06 * t * t + 0.012 * math.sin(t * math.pi), y, oben + (unten - oben) * t))

    ringe = []
    for i in range(16):
        t = i / 15
        r = 0.018 + 0.003 * math.sin(t * math.pi) + (0.006 if t > 0.94 else 0.0)
        ringe.append((stiel_punkt(t), X, Y, r * 0.9, r))

    def stiel_farbe(i, k, p):
        z = p.center.z
        if z > 0.8:
            return leder * (0.78 if int(z * 70) % 2 else 1.0)
        return (holz if (k + int(i * 3)) % 5 else holz_dunkel) * (0.9 + 0.1 * math.sin(z * 37))

    f.loft("Axtstiel", ringe, 10, stiel_farbe, gewicht, oben_zu=True, unten_zu=True, teilung=2)

    # Kopf: vom stumpfen Nacken (-X) über das Auge bis zur breiten, geschwungenen Schneide (+X).
    # (Abstand, halbe Dicke in Y, halbe Höhe in Z, Versatz in Z)
    auge = stiel_punkt(0.92)
    profil = [(-0.045, 0.02, 0.028, 0.0), (-0.02, 0.024, 0.036, 0.0), (0.0, 0.025, 0.038, 0.0), (0.03, 0.02, 0.032, -0.004),
              (0.065, 0.014, 0.032, -0.01), (0.1, 0.009, 0.05, -0.016), (0.13, 0.006, 0.07, -0.02), (0.155, 0.004, 0.085, -0.022),
              (0.17, 0.0015, 0.09, -0.022)]
    kopf = [(auge + X * dx + Z * dz, Y, Z, dicke, hoehe) for dx, dicke, hoehe, dz in profil]

    def kopf_farbe(i, k, p):
        schneide = p.center.x - auge.x > 0.13
        return (eisen_hell if schneide else eisen) * (0.93 + 0.07 * (k % 2))

    f.loft("Axtkopf", kopf, 8, kopf_farbe, gewicht, oben_zu=True, unten_zu=True, teilung=2)
    # Eisenring und Keil am Auge
    ring = [(auge + Z * dz, X, Y, 0.024, 0.026) for dz in (0.05, 0.065)]
    f.loft("Axtring", ring, 10, lambda i, k, p: eisen_dunkel, gewicht, oben_zu=True, unten_zu=True)
    f.kugel("Axtkeil", auge - Z * 0.042, Vector((0.02, 0.011, 0.008)), eisen_dunkel, gewicht, segmente=8, ringe=4, glatt=False)


def _magier_animationen(armatur):
    # Idle: ruhiges Atmen, der Blick wandert, der Hut wippt nach
    def idle(phi):
        return [
            ("Brust", "rot", (1.5 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-0.8 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (2 * math.sin(phi * 2 + 1), 14 * math.sin(phi), 2 * math.sin(phi))),
            ("Hut", "rot", (3 * math.sin(phi * 2 + 1.5), 0, 2 * math.sin(phi + 0.8))),
            ("Oberarm.L", "rot", (3 * math.sin(phi * 2), 0, 0)), ("Unterarm.L", "rot", (-12 - 3 * math.sin(phi * 2), 0, 0)),
            ("Oberarm.R", "rot", (0, 0, 0)), ("Unterarm.R", "rot", (-6, 0, 0)),
            ("Becken", "pos", (0, -0.004 * (1 - math.cos(phi * 2)), 0)),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))
    animation(armatur, "Laufen", 30, _schleife(30, 2, lambda phi: _gehen(phi, 28, 45, 22, 0.03, 4, 14)))
    animation(armatur, "Rennen", 20, _schleife(20, 2, lambda phi: _gehen(phi, 44, 80, 40, 0.06, 14, 55)))

    # Springen (in der Luft): Beine angezogen, Arme leicht ausgebreitet, Robe flattert
    def springen(phi):
        s = math.sin(phi)
        return [
            ("Oberschenkel.L", "rot", (-35 + 4 * s, 0, 0)), ("Oberschenkel.R", "rot", (-20 - 4 * s, 0, 0)),
            ("Unterschenkel.L", "rot", (70, 0, 0)), ("Unterschenkel.R", "rot", (55, 0, 0)),
            ("Oberarm.L", "rot", (-35 + 5 * s, 0, 25)), ("Unterarm.L", "rot", (-30, 0, 0)),
            ("Oberarm.R", "rot", (-20, 0, -15)), ("Unterarm.R", "rot", (-25, 0, 0)),
            ("Brust", "rot", (6, 0, 0)), ("Hut", "rot", (-12 + 4 * s, 0, 0)),
        ]
    animation(armatur, "Springen", 20, _schleife(20, 2, springen))

    # Hieb: Stab mit beiden Händen über die rechte Schulter heben und kräftig schräg niederschlagen
    hieb = []
    for bild, arm_r, ell_r, arm_l, ell_l, brust_x, brust_y, knie in (
            (0, 0, -6, 0, -12, 0, 0, 6), (9, -150, -40, -110, -70, -8, 25, 10), (13, -160, -30, -120, -60, -10, 30, 12),
            (17, -45, -15, -40, -30, 22, -20, 25), (22, -35, -20, -30, -35, 18, -15, 20), (32, 0, -6, 0, -12, 0, 0, 6)):
        hieb += [(bild, "Oberarm.R", "rot", (arm_r, 0, 0)), (bild, "Unterarm.R", "rot", (ell_r, 0, 0)),
                 (bild, "Oberarm.L", "rot", (arm_l, 0, -10)), (bild, "Unterarm.L", "rot", (ell_l, 0, 0)),
                 (bild, "Brust", "rot", (brust_x, brust_y, 0)), (bild, "Bauch", "rot", (brust_x * 0.4, brust_y * 0.4, 0)),
                 (bild, "Oberschenkel.L", "rot", (-knie * 0.6, 0, 0)), (bild, "Unterschenkel.L", "rot", (knie, 0, 0)),
                 (bild, "Oberschenkel.R", "rot", (knie * 0.3, 0, 0)), (bild, "Unterschenkel.R", "rot", (knie * 0.6, 0, 0)),
                 (bild, "Hut", "rot", (-brust_x * 0.5, 0, 0))]
    animation(armatur, "Hieb", 32, hieb)

    # Werfen: linker Arm holt weit aus und schleudert nach vorne
    werfen = []
    for bild, arm, ell, brust_y, brust_x in ((0, 0, -12, 0, 0), (9, 55, -80, 25, -5), (14, -120, -10, -25, 12),
                                             (20, -80, -20, -15, 8), (30, 0, -12, 0, 0)):
        werfen += [(bild, "Oberarm.L", "rot", (arm, 0, 15)), (bild, "Unterarm.L", "rot", (ell, 0, 0)),
                   (bild, "Brust", "rot", (brust_x, brust_y, 0)), (bild, "Bauch", "rot", (brust_x * 0.3, brust_y * 0.5, 0)),
                   (bild, "Hut", "rot", (-brust_x * 0.6, 0, 0))]
    animation(armatur, "Werfen", 30, werfen)

    # Zaubern: Stab hochreißen, Kraft sammeln, dann den Stab nach vorne stoßen; die linke Hand
    # zeigt mit gespreizter Handfläche aufs Ziel. Das Geschoss fliegt bei Bild 9 los (0,22 s bei Tempo 1,35).
    zaubern = []
    for bild, arm_r, ell_r, arm_l, ell_l, brust_x, brust_y, kopf_x in (
            (0, 0, -6, 0, -12, 0, 0, 0), (5, -115, -55, -30, -40, -8, 18, -6), (9, -85, -8, -80, -8, 12, -10, 4),
            (14, -80, -6, -75, -10, 14, -12, 4), (20, -40, -10, -35, -20, 6, -5, 2), (28, 0, -6, 0, -12, 0, 0, 0)):
        zaubern += [(bild, "Oberarm.R", "rot", (arm_r, 0, 0)), (bild, "Unterarm.R", "rot", (ell_r, 0, 0)),
                    (bild, "Oberarm.L", "rot", (arm_l, 0, 12)), (bild, "Unterarm.L", "rot", (ell_l, 0, 0)),
                    (bild, "Brust", "rot", (brust_x, brust_y, 0)), (bild, "Bauch", "rot", (brust_x * 0.4, brust_y * 0.4, 0)),
                    (bild, "Kopf", "rot", (kopf_x, 0, 0)), (bild, "Hut", "rot", (-brust_x * 0.6, 0, 0)),
                    (bild, "Oberschenkel.L", "rot", (-brust_x * 0.8, 0, 0)), (bild, "Unterschenkel.L", "rot", (6 + abs(brust_x), 0, 0)),
                    (bild, "Oberschenkel.R", "rot", (brust_x * 0.5, 0, 0)), (bild, "Unterschenkel.R", "rot", (6 + abs(brust_x) * 0.5, 0, 0))]
    animation(armatur, "Zaubern", 28, zaubern)

    # Abbauen: Spitzhacke mit beiden Händen hoch über den Kopf, dann kraftvoll nach vorne unten
    # schlagen (Einschlag bei Bild 16), kurz nachziehen, zurück. Der Stiel hängt in Ruhe nach unten,
    # über dem Kopf zeigt er nach oben, beim Einschlag schräg nach vorne zum Boden.
    abbauen = []
    for bild, arm_r, ell_r, arm_l, ell_l, brust_x, knie, senken in (
            (0, 0, -6, 0, -12, 0, 6, 0.0), (9, -170, -30, -150, -50, -12, 10, 0.0), (13, -120, -15, -110, -30, 8, 16, -0.02),
            (16, -40, -4, -48, -12, 28, 30, -0.07), (21, -34, -10, -40, -18, 24, 26, -0.06), (30, 0, -6, 0, -12, 0, 6, 0.0)):
        abbauen += [(bild, "Oberarm.R", "rot", (arm_r, 0, 0)), (bild, "Unterarm.R", "rot", (ell_r, 0, 0)),
                    (bild, "Oberarm.L", "rot", (arm_l, 0, -12)), (bild, "Unterarm.L", "rot", (ell_l, 0, 0)),
                    (bild, "Brust", "rot", (brust_x, 0, 0)), (bild, "Bauch", "rot", (brust_x * 0.5, 0, 0)),
                    (bild, "Kopf", "rot", (-brust_x * 0.4, 0, 0)), (bild, "Hut", "rot", (-brust_x * 0.5, 0, 0)),
                    (bild, "Oberschenkel.L", "rot", (-knie * 0.7, 0, 0)), (bild, "Unterschenkel.L", "rot", (knie, 0, 0)),
                    (bild, "Oberschenkel.R", "rot", (-knie * 0.4, 0, 0)), (bild, "Unterschenkel.R", "rot", (knie * 0.7, 0, 0)),
                    (bild, "Becken", "pos", (0, senken, 0))]
    animation(armatur, "Abbauen", 30, abbauen)

    # Hacken (Axt am Baum): Arme nach vorne, Oberkörper weit nach rechts ausholen, dann waagerecht
    # von rechts nach links durchziehen (Einschlag bei Bild 11), nachschwingen, zurück.
    # Mit dem Arm nach vorne zeigt der hängende Stiel waagerecht nach vorne, die Schneide nach links.
    hacken = []
    for bild, arm_r, ell_r, arm_l, ell_l, dreh, beugen, knie in (
            (0, 0, -6, 0, -12, 0, 0, 6), (7, -95, -25, -80, -40, 55, -4, 14), (11, -88, -6, -80, -18, -8, 8, 18),
            (15, -82, -8, -76, -22, -35, 10, 16), (22, -50, -15, -45, -20, -15, 4, 10), (24, 0, -6, 0, -12, 0, 0, 6)):
        hacken += [(bild, "Oberarm.R", "rot", (arm_r, 0, 0)), (bild, "Unterarm.R", "rot", (ell_r, 0, 0)),
                   (bild, "Oberarm.L", "rot", (arm_l, 0, -18)), (bild, "Unterarm.L", "rot", (ell_l, 0, 0)),
                   (bild, "Brust", "rot", (beugen, dreh, 0)), (bild, "Bauch", "rot", (beugen * 0.5, dreh * 0.5, 0)),
                   (bild, "Becken", "rot", (0, dreh * 0.25, 0)), (bild, "Kopf", "rot", (-beugen * 0.5, -dreh * 0.4, 0)),
                   (bild, "Hut", "rot", (0, 0, -dreh * 0.1)),
                   (bild, "Oberschenkel.L", "rot", (-knie * 0.5, 0, 0)), (bild, "Unterschenkel.L", "rot", (knie, 0, 0)),
                   (bild, "Oberschenkel.R", "rot", (-knie * 0.3, 0, 0)), (bild, "Unterschenkel.R", "rot", (knie * 0.7, 0, 0))]
    animation(armatur, "Hacken", 24, hacken)

    # ---- Fähigkeiten: eigene Clips mit Ausholen, Wirkung und Nachschwung. Im Spiel laufen sie
    # als zweite Ebene: im Stand mit dem ganzen Körper, beim Laufen nur auf dem Oberkörper.
    # Das Bild, in dem die Fähigkeit wirkt, steht in game/src/faehigkeiten.rs (`animation`).
    _magier_faehigkeiten(armatur)


def _clip(armatur, name, laenge, posen):
    """Clip aus Posen: Liste (Bild, {Knochen: (x, y, z)}) – Drehungen in Grad, "Becken.pos" ist
    die Verschiebung des Beckens in Metern (z = nach oben). Jeder Knochen, der irgendwo vorkommt,
    bekommt in jedem Bild einen Schlüssel (fehlt er, gilt die Grundstellung), damit nichts
    ungewollt von einer Pose in die übernächste schwingt."""
    knochen = sorted({k for _, pose in posen for k in pose})
    schluessel = []
    for bild, pose in posen:
        for k in knochen:
            if k == "Becken.pos":
                # Im Raum des Beckenknochens zeigt Y nach oben (entlang des Knochens)
                x, y, z = pose.get(k, (0, 0, 0))
                schluessel.append((bild, "Becken", "pos", (x, z, -y)))
            else:
                schluessel.append((bild, k, "rot", pose.get(k, (0, 0, 0))))
    animation(armatur, name, laenge, schluessel)


def _mit(basis, **aenderungen):
    """Kopie einer Pose mit Änderungen (Knochennamen mit Punkt: "Oberarm_R" → "Oberarm.R")."""
    pose = dict(basis)
    for k, v in aenderungen.items():
        pose[k.replace("_", ".")] = v
    return pose


def _magier_faehigkeiten(armatur):
    ruhe = {"Oberarm.R": (0, 0, 0), "Unterarm.R": (-6, 0, 0), "Oberarm.L": (0, 0, 0), "Unterarm.L": (-12, 0, 0)}

    # Arkan: den Stab zurückziehen, blitzschnell nach vorne stoßen (Bild 5), die linke Hand
    # reißt nach hinten, Ausfallschritt, kurz halten, zurück.
    ausholen = {"Oberarm.R": (-55, 0, -10), "Unterarm.R": (-75, 0, 0), "Oberarm.L": (-45, 0, 8), "Unterarm.L": (-25, 0, 0),
                "Brust": (-4, 20, 0), "Bauch": (0, 8, 0), "Kopf": (0, -14, 0), "Hut": (3, 0, 0),
                "Oberschenkel.L": (-6, 0, 0), "Unterschenkel.L": (8, 0, 0)}
    stoss = {"Oberarm.R": (-98, 0, -4), "Unterarm.R": (-4, 0, 0), "Oberarm.L": (-15, 0, 20), "Unterarm.L": (-70, 0, 0),
             "Brust": (10, -14, 0), "Bauch": (4, -6, 0), "Kopf": (-6, 10, 0), "Hut": (-7, 0, 0),
             "Oberschenkel.L": (-22, 0, 0), "Unterschenkel.L": (22, 0, 0), "Oberschenkel.R": (12, 0, 0), "Unterschenkel.R": (12, 0, 0),
             "Becken.pos": (0, 0, -0.03)}
    _clip(armatur, "Arkan", 16, [(0, ruhe), (3, ausholen), (5, stoss),
                                 (8, _mit(stoss, Oberarm_R=(-93, 0, -4), Brust=(12, -16, 0))), (16, ruhe)])

    # Feuerball: beide Hände formen die Glut vor der Brust, weit nach rechts hinten ausholen,
    # mit Körperdrehung und Ausfallschritt nach vorne schleudern (Bild 11), nachschwingen.
    sammeln = {"Oberarm.R": (-50, 0, 18), "Unterarm.R": (-100, 0, 0), "Oberarm.L": (-50, 0, -18), "Unterarm.L": (-100, 0, 0),
               "Brust": (6, 0, 0), "Bauch": (3, 0, 0), "Kopf": (8, 0, 0),
               "Oberschenkel.L": (-10, 0, 0), "Unterschenkel.L": (16, 0, 0), "Oberschenkel.R": (-6, 0, 0), "Unterschenkel.R": (12, 0, 0),
               "Becken.pos": (0, 0, -0.03)}
    zurueck = {"Oberarm.R": (-35, 0, -10), "Unterarm.R": (-95, 0, 0), "Oberarm.L": (-80, 0, -5), "Unterarm.L": (-40, 0, 0),
               "Brust": (-6, 42, 0), "Bauch": (-2, 20, 0), "Becken": (0, 10, 0), "Kopf": (4, -30, 0), "Hut": (4, 0, 0),
               "Oberschenkel.L": (-20, 0, 0), "Unterschenkel.L": (10, 0, 0), "Oberschenkel.R": (8, 0, 0), "Unterschenkel.R": (24, 0, 0),
               "Becken.pos": (0, 0, -0.02)}
    wurf = {"Oberarm.R": (-102, 0, 6), "Unterarm.R": (-8, 0, 0), "Oberarm.L": (-96, 0, -8), "Unterarm.L": (-6, 0, 0),
            "Brust": (16, -28, 0), "Bauch": (8, -14, 0), "Becken": (0, -8, 0), "Kopf": (-10, 22, 0), "Hut": (-10, 0, 0),
            "Oberschenkel.L": (-34, 0, 0), "Unterschenkel.L": (36, 0, 0), "Oberschenkel.R": (20, 0, 0), "Unterschenkel.R": (10, 0, 0),
            "Becken.pos": (0, 0, -0.06)}
    nach = _mit(wurf, Oberarm_R=(-80, 0, 6), Unterarm_R=(-15, 0, 0), Oberarm_L=(-70, 0, -8), Unterarm_L=(-20, 0, 0),
                Brust=(18, -32, 0), Bauch=(8, -16, 0), Becken=(0, -10, 0), Kopf=(-10, 24, 0))
    _clip(armatur, "Feuerball", 26, [(0, ruhe), (5, sammeln), (9, zurueck), (11, wurf), (15, nach), (26, ruhe)])

    # Frostnova: den Stab mit beiden Händen hoch über den Kopf, auf die Zehen, dann tief in die
    # Knie und das Stabende auf den Boden stoßen (Bild 14), die linke Hand schleudert zur Seite.
    hoch = {"Oberarm.R": (-165, 0, -8), "Unterarm.R": (-25, 0, 0), "Oberarm.L": (-150, 0, 10), "Unterarm.L": (-45, 0, 0),
            "Brust": (-12, 0, 0), "Bauch": (-4, 0, 0), "Kopf": (-12, 0, 0), "Hut": (8, 0, 0), "Becken.pos": (0, 0, 0.03)}
    runter = {"Oberarm.R": (-130, 0, -8), "Unterarm.R": (-12, 0, 0), "Oberarm.L": (-115, 0, 15), "Unterarm.L": (-25, 0, 0),
              "Brust": (6, 0, 0), "Bauch": (3, 0, 0), "Kopf": (-2, 0, 0),
              "Oberschenkel.L": (-15, 0, 4), "Unterschenkel.L": (20, 0, 0), "Oberschenkel.R": (-12, 0, -4), "Unterschenkel.R": (18, 0, 0),
              "Becken.pos": (0, 0, -0.02)}
    aufschlag = {"Oberarm.R": (-78, 0, -6), "Unterarm.R": (-4, 0, 0), "Oberarm.L": (-45, 0, 70), "Unterarm.L": (-10, 0, 0),
                 "Brust": (26, 0, 0), "Bauch": (12, 0, 0), "Kopf": (-18, 0, 0), "Hut": (-8, 0, 0),
                 "Oberschenkel.L": (-48, 0, 10), "Unterschenkel.L": (62, 0, 0), "Oberschenkel.R": (-30, 0, -10), "Unterschenkel.R": (52, 0, 0),
                 "Fuss.L": (-14, 0, 0), "Fuss.R": (-18, 0, 0), "Becken.pos": (0, 0, -0.14)}
    _clip(armatur, "Frostnova", 30, [(0, ruhe), (6, hoch), (11, runter), (14, aufschlag),
                                     (19, _mit(aufschlag, Brust=(22, 0, 0), Oberarm_L=(-30, 0, 75))), (30, ruhe)])

    # Meteor (ultimativ): Stab und Hände zum Himmel, auf die Zehen, Kraft sammeln – dann den Stab
    # mit Ausfallschritt aufs Ziel richten (Bild 20), die Linke öffnet sich zum Ziel
    ruf = {"Oberarm.R": (-170, 0, -8), "Unterarm.R": (-15, 0, 0), "Oberarm.L": (-160, 0, 25), "Unterarm.L": (-30, 0, 0),
           "Brust": (-14, 0, 0), "Bauch": (-5, 0, 0), "Kopf": (-18, 0, 0), "Hut": (10, 0, 0), "Becken.pos": (0, 0, 0.03)}
    zeigen = {"Oberarm.R": (-95, 0, -4), "Unterarm.R": (-5, 0, 0), "Oberarm.L": (-85, 0, 30), "Unterarm.L": (-5, 0, 0),
              "Brust": (18, -10, 0), "Bauch": (8, -5, 0), "Kopf": (-8, 8, 0), "Hut": (-8, 0, 0),
              "Oberschenkel.L": (-30, 0, 0), "Unterschenkel.L": (32, 0, 0), "Oberschenkel.R": (18, 0, 0), "Unterschenkel.R": (10, 0, 0),
              "Becken.pos": (0, 0, -0.05)}
    _clip(armatur, "Meteor", 36, [(0, ruhe), (8, ruf), (16, _mit(ruf, Oberarm_R=(-176, 0, -8), Brust=(-17, 4, 0))), (20, zeigen),
                                  (27, _mit(zeigen, Brust=(20, -12, 0))), (36, ruhe)])
