"""Tier-Baukasten im Mid-Poly-Stil – passend zu den Bäumen.

Jedes Tier entsteht so:
1. Körper aus weichen Metaball-Formen (Rumpf, Hals, Kopf, Beine, Schwanz …), die wie
   Muskeln ineinander übergehen,
2. auf rund 2.000–2.400 Dreiecke vereinfacht und facettiert (flach schattiert) – das ergibt
   den kantigen, aber detailreichen Polygon-Look,
3. Fellzonen als Vertexfarben (z. B. Fuchs: orange, weiße Brust, schwarze Beine),
4. Ohren, Augen mit Glanzpunkt und Nase als eigene, scharfkantige Teile,
5. Skelett mit Rumpf, Hals, Kopf, zweiteiligem Schwanz und Beinen mit Knie; weiche
   Gewichtung, Ohren/Augen/Nase sitzen fest am Kopf,
6. Animationen Idle (Reh und Schaf grasen dabei), Laufen, Rennen.

Koordinaten wie in der Werkstatt: Z oben, das Tier schaut nach -Y, Ursprung am Boden
unter der Körpermitte. Arten: baer, wolf, fuchs, reh, schaf.
"""

import math
import random

import bmesh
import bpy
from mathutils import Quaternion, Vector

from werkstatt import animation, ruhepose, skelett, srgb_zu_linear

# Sichtbarer Radius einer Metaball-Kugel ≈ 0,575 × Einflussradius (Schwelle 0,6, Härte 2).
_SICHTBAR = 0.575
VORNE = (0, 0, 1)  # Rumpf/Kopf/Schwanz: lokale Z-Achse nach oben – Drehung um X nickt
UNTEN = (0, 1, 0)  # Beine: Drehung um X schwingt sie vor und zurück


def farbe(hex_farbe):
    return Vector(srgb_zu_linear(hex_farbe)[:3])


def _weich(kante0, kante1, x):
    t = max(0.0, min(1.0, (x - kante0) / (kante1 - kante0)))
    return t * t * (3 - 2 * t)


class Tier:
    def __init__(self, name, seed=1, massstab=1.0):
        self.name = name
        self.s = massstab
        self.rng = random.Random(seed)
        self.formen = []    # (Mitte, Halbachsen, Drehung)
        self.teile = []     # zusätzliche Objekte (Ohren, Augen, Nase)
        self.knochen = []   # (Name, Kopf, Ende, Eltern, Oben)

    def v(self, x, y, z):
        """Punkt im Maßstab des Tiers."""
        return Vector((x, y, z)) * self.s

    # --- Form -------------------------------------------------------------
    def form(self, mitte, halb, drehung=None, spiegeln=False):
        """Weiches Ellipsoid (Maße in Einheiten des Maßstabs). `spiegeln`: auch auf der anderen Seite."""
        for x in ((1, -1) if spiegeln else (1,)):
            m = Vector((mitte[0] * x, mitte[1], mitte[2])) * self.s
            self.formen.append((m, Vector(halb) * self.s, drehung))

    def glied(self, a, b, dicke_a, dicke_b, spiegeln=True, flach=1.0):
        """Bein- oder Schwanzabschnitt von a nach b, sich verjüngend (zwei Ellipsoide)."""
        a, b = Vector(a), Vector(b)
        richtung = (b - a)
        laenge = richtung.length
        drehung = Vector((0, 0, 1)).rotation_difference(richtung.normalized())
        for t, d in ((0.3, dicke_a), (0.72, (dicke_a + dicke_b) / 2)):
            self.form(a.lerp(b, t), (d * flach, d, laenge * 0.36 + d * 0.3), drehung, spiegeln)

    def koerper(self, zonen, aufloesung=0.03, beulen=0.012, ziel=2200):
        """Metaballs → Mesh, vereinfacht, facettiert, Fellzonen eingefärbt."""
        kugeln = bpy.data.metaballs.new(self.name + "Form")
        kugeln.resolution = aufloesung * self.s
        kugeln.render_resolution = aufloesung * self.s
        kugeln.threshold = 0.6
        for mitte, (hx, hy, hz), drehung in self.formen:
            e = kugeln.elements.new(type="ELLIPSOID")
            e.co = mitte
            e.radius = 1.0
            e.size_x, e.size_y, e.size_z = hx / _SICHTBAR, hy / _SICHTBAR, hz / _SICHTBAR
            e.stiffness = 2.0
            if drehung is not None:
                e.rotation = drehung
        form = bpy.data.objects.new(self.name + "Form", kugeln)
        bpy.context.scene.collection.objects.link(form)
        bpy.context.view_layer.update()
        bpy.ops.object.select_all(action="DESELECT")
        form.select_set(True)
        bpy.context.view_layer.objects.active = form
        bpy.ops.object.convert(target="MESH")
        obj = bpy.context.active_object
        obj.name = self.name
        if beulen > 0:
            textur = bpy.data.textures.new(self.name + "Beulen", "CLOUDS")
            textur.noise_scale = 0.25 * self.s
            mod = obj.modifiers.new("Beulen", "DISPLACE")
            mod.texture = textur
            mod.strength = beulen * self.s
            bpy.ops.object.modifier_apply(modifier=mod.name)
        dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
        if dreiecke > ziel:
            mod = obj.modifiers.new("Vereinfachen", "DECIMATE")
            mod.ratio = ziel / dreiecke
            bpy.ops.object.modifier_apply(modifier=mod.name)
        # Nichts unter dem Boden
        for vert in obj.data.vertices:
            vert.co.z = max(vert.co.z, 0.0)
        self._einfaerben(obj, lambda p, n: zonen(p / self.s, n))
        self.rumpf = obj
        return obj

    def _einfaerben(self, obj, farbe_von):
        """Je Fläche eine Farbe (mit leichter Schwankung, damit die Facetten lebendig wirken)."""
        mesh = obj.data
        attr = mesh.color_attributes.new("Farbe", "FLOAT_COLOR", "CORNER")
        for poly in mesh.polygons:
            c = farbe_von(poly.center.copy(), poly.normal.copy()) * self.rng.uniform(0.94, 1.05)
            for li in poly.loop_indices:
                attr.data[li].color = (c.x, c.y, c.z, 1.0)
            poly.use_smooth = False

    # --- Anbauteile -------------------------------------------------------
    def _teil(self, bm, name, farbe_von, knochen):
        mesh = bpy.data.meshes.new(name)
        bm.to_mesh(mesh)
        bm.free()
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        self._einfaerben(obj, farbe_von)
        # Merker: fest an diesem Knochen (nach der automatischen Gewichtung)
        gruppe = obj.vertex_groups.new(name="starr:" + knochen)
        gruppe.add([v.index for v in mesh.vertices], 1.0, "REPLACE")
        self.teile.append(obj)
        return obj

    def kugel(self, ort, radius, c, groesse=(1, 1, 1), segmente=8, ringe=6, knochen="Kopf", name="Kugel"):
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=segmente, v_segments=ringe, radius=radius)
        for vert in bm.verts:
            vert.co = Vector((vert.co.x * groesse[0], vert.co.y * groesse[1], vert.co.z * groesse[2])) + ort
        return self._teil(bm, name, lambda p, n: c, knochen)

    def auge(self, ort, radius, c="#17120F", glanz_richtung=Vector((0.3, -0.6, 0.6))):
        """Dunkles Auge mit kleinem weißen Glanzpunkt."""
        self.kugel(ort, radius, farbe(c), name="Auge")
        self.kugel(ort + glanz_richtung.normalized() * radius * 0.75, radius * 0.32, farbe("#F4F1EA"), segmente=6, ringe=4, name="Glanz")

    def ohr(self, basis, spitze, breite, dicke, aussen, innen, rund=0.0, knochen="Kopf"):
        """Flaches, spitzes (oder mit `rund` > 0 abgerundetes) Ohr; die Innenseite zeigt nach vorne."""
        basis, spitze = Vector(basis), Vector(spitze)
        achse = spitze - basis
        vorne = Vector((0, -1, 0))
        seite = achse.cross(vorne).normalized()
        tiefe = seite.cross(achse.normalized()).normalized()  # zeigt nach vorne
        bm = bmesh.new()
        ringe = []
        for t, w in ((0.0, 0.8), (0.4, 1.0 + rund * 0.3), (0.75, 0.75 + rund * 0.5)):
            mitte = basis + achse * t
            ringe.append([bm.verts.new(mitte + seite * breite / 2 * w * sx + tiefe * dicke / 2 * dz)
                          for sx, dz in ((1, 0), (0, 1), (-1, 0), (0, -1.6))])
        oben = bm.verts.new(spitze)
        bm.faces.new(list(reversed(ringe[0])))
        for a, b in zip(ringe, ringe[1:]):
            for k in range(4):
                bm.faces.new((a[k], a[(k + 1) % 4], b[(k + 1) % 4], b[k]))
        for k in range(4):
            bm.faces.new((ringe[-1][k], ringe[-1][(k + 1) % 4], oben))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        c_aussen, c_innen = farbe(aussen), farbe(innen)
        return self._teil(bm, "Ohr", lambda p, n: c_innen if n.dot(tiefe) > 0.45 else c_aussen, knochen)

    def augen_orte(self, x, z, tiefe=0.004):
        """Wo die Augen auf der Kopfoberfläche sitzen (Strahl von vorne, Maße im Maßstab)."""
        orte = []
        for sx in (-1, 1):
            start = self.v(x * sx, -5.0, z)
            treffer, ort, normale, _ = self.rumpf.ray_cast(start, Vector((0, 1, 0)))
            orte.append(ort - normale * tiefe * self.s if treffer else self.v(x * sx, -0.5, z))
        return orte

    # --- Skelett ----------------------------------------------------------
    def knochen_dazu(self, name, kopf, ende, eltern, oben=VORNE):
        self.knochen.append((name, tuple(Vector(kopf) * self.s), tuple(Vector(ende) * self.s), eltern, oben))

    def bein_knochen(self, seite, x, gelenk, knie, fuss):
        """Bein.<seite>.oben (Gelenk → Knie) und .unten (Knie → Fuß), links und rechts."""
        for s, sx in (("L", 1), ("R", -1)):
            g = (gelenk[0] * sx, gelenk[1], gelenk[2])
            k = (knie[0] * sx, knie[1], knie[2])
            f = (fuss[0] * sx, fuss[1], fuss[2])
            self.knochen_dazu(f"Bein.{seite}{s}.oben", g, k, "Koerper", UNTEN)
            self.knochen_dazu(f"Bein.{seite}{s}.unten", k, f, f"Bein.{seite}{s}.oben", UNTEN)

    def fertig(self, stil):
        """Teile vereinen, Material, Skelett, Gewichte, Animationen."""
        bpy.ops.object.select_all(action="DESELECT")
        for teil in self.teile:
            teil.select_set(True)
        self.rumpf.select_set(True)
        bpy.context.view_layer.objects.active = self.rumpf
        bpy.ops.object.join()
        obj = self.rumpf

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
        bsdf.inputs["Roughness"].default_value = 0.9
        obj.data.materials.clear()
        obj.data.materials.append(mat)
        for poly in obj.data.polygons:
            poly.material_index = 0

        armatur = skelett(self.name + "Skelett", self.knochen)
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        armatur.select_set(True)
        bpy.context.view_layer.objects.active = armatur
        bpy.ops.object.parent_set(type="ARMATURE_AUTO")

        # Ohren, Augen, Nase: fest an ihrem Knochen statt weich verteilt
        for gruppe in [g for g in obj.vertex_groups if g.name.startswith("starr:")]:
            ziel = obj.vertex_groups.get(gruppe.name.split(":", 1)[1])
            indizes = [v.index for v in obj.data.vertices if any(g.group == gruppe.index for g in v.groups)]
            for andere in obj.vertex_groups:
                if andere.name in {k[0] for k in self.knochen}:
                    andere.remove(indizes)
            ziel.add(indizes, 1.0, "REPLACE")
            obj.vertex_groups.remove(gruppe)

        _animieren(armatur, stil, {k[0] for k in self.knochen})
        ruhepose(armatur)
        dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
        print(f"TIER {self.name}: {dreiecke} Dreiecke, {len(self.knochen)} Knochen")
        return obj


# ---------------------------------------------------------------------------
# Animationen (30 Bilder pro Sekunde; Schleifen: letztes Bild = erstes)
# Beine: Drehung um X, positiv = Fuß nach hinten. Hals/Kopf: positiv um X = nach oben.
# ---------------------------------------------------------------------------
def _rot(bild, name, x=0.0, y=0.0, z=0.0):
    return (bild, name, "rot", (x, y, z))


def _pos(bild, name, z=0.0):
    return (bild, name, "pos", (0.0, 0.0, z))


def _bein(laenge, bein, versatz, schwung, knick, stand=0.5):
    """Ein Bein: in der Luft von hinten (+schwung) nach vorne (-schwung), am Boden zurück.
    `stand`: Anteil des Zyklus am Boden."""
    luft = 1.0 - stand
    schluessel = []
    for i in range(9):
        t = (i / 8 + versatz) % 1.0
        bild = round(i * laenge / 8)
        if t < luft:
            u = t / luft
            winkel = schwung * math.cos(math.pi * u)
            beuge = knick * math.sin(math.pi * u)
        else:
            u = (t - luft) / stand
            winkel = -schwung * math.cos(math.pi * u)
            beuge = 0.0
        schluessel += [_rot(bild, f"{bein}.oben", x=winkel), _rot(bild, f"{bein}.unten", x=beuge)]
    return schluessel


def _animieren(armatur, stil, knochen):
    schwanz = [k for k in ("Schwanz1", "Schwanz2") if k in knochen]

    # Idle: atmen, sich umschauen, Schwanz schwingt; Grasfresser senken den Kopf zum Gras.
    idle = []
    lang = 180
    for bild, atmen, blick in ((0, 0.0, -12), (45, 0.012, 0), (90, 0.0, 14), (135, 0.012, 0), (180, 0.0, -12)):
        idle.append(_pos(bild, "Koerper", atmen * stil.get("atmen", 1.0)))
        if not stil.get("grasen"):
            idle += [_rot(bild, "Kopf", x=-3 if bild % 90 else 2, z=blick * 0.6), _rot(bild, "Hals", z=blick * 0.4)]
    if stil.get("grasen"):
        # (Hals, Kopf) in Grad nach unten, bis die Nase das Gras erreicht; dazwischen kleines Rupfen
        h, k = stil["grasen"]
        for bild, tief, rupfen, blick in ((0, 0, 0, -10), (30, 0, 0, 8), (55, 1, 0, 0), (70, 1, 4, 3), (85, 1, 0, -3),
                                          (100, 1, 4, 2), (115, 1, 0, 0), (140, 0, 0, 6), (180, 0, 0, -10)):
            idle += [_rot(bild, "Hals", x=-h * tief, z=blick * 0.4), _rot(bild, "Kopf", x=-(k + rupfen) * tief, z=blick * 0.6)]
    for s, name in enumerate(schwanz):
        for bild, w in ((0, 0), (40, 10), (60, -12), (80, 8), (100, 0), (180, 0)):
            idle.append(_rot(bild, name, z=w * stil.get("wedeln", 1.0) * (1 + s * 0.5)))
    animation(armatur, "Idle", lang, idle)

    # Laufen: Diagonalgang (Vorderbein und gegenüberliegendes Hinterbein fast gleichzeitig)
    t_lauf = stil["schritt"]
    laufen = []
    for bein, versatz in (("Bein.VL", 0.0), ("Bein.HR", 0.06), ("Bein.VR", 0.5), ("Bein.HL", 0.56)):
        laufen += _bein(t_lauf, bein, versatz, stil["schwung"], stil["knick"], stand=0.6)
    for i in range(5):
        bild = round(i * t_lauf / 4)
        laufen += [_pos(bild, "Koerper", stil["wippen"] * (i % 2)), _rot(bild, "Kopf", x=2 if i % 2 else -2)]
        for s, name in enumerate(schwanz):
            laufen.append(_rot(bild, name, z=(6 + s * 4) * (1 if i in (1,) else -1 if i == 3 else 0)))
    animation(armatur, "Laufen", t_lauf, laufen)

    # Rennen: Galopp – Vorderbeine kurz nacheinander, dann die Hinterbeine; Rücken streckt
    # und beugt sich, Kopf gleicht aus.
    t_renn = stil["sprung"]
    rennen = []
    for bein, versatz in (("Bein.VL", 0.0), ("Bein.VR", 0.1), ("Bein.HL", 0.5), ("Bein.HR", 0.6)):
        rennen += _bein(t_renn, bein, versatz, stil["schwung_renn"], stil["knick"] * 1.4, stand=0.4)
    for i, (hoehe, neigung) in enumerate(((0.0, 5), (0.6, 0), (0.0, -5), (0.4, 0), (0.0, 5))):
        bild = round(i * t_renn / 4)
        rennen += [_pos(bild, "Koerper", hoehe * stil["huepfen"]), _rot(bild, "Koerper", x=neigung * stil.get("biegen", 1.0)),
                   _rot(bild, "Kopf", x=-neigung * 0.8)]
        for name in schwanz:
            rennen.append(_rot(bild, name, x=-neigung * 1.5))
    animation(armatur, "Rennen", t_renn, rennen)


def schrittweite(bein_laenge, schwung, bilder, stand):
    """Wie schnell die Füße am Boden nach hinten wandern (m/s) – so schnell muss das Tier
    laufen, damit sie nicht rutschen."""
    weg = 2 * bein_laenge * math.sin(math.radians(schwung))
    return weg / (bilder / 30 * stand)


# ---------------------------------------------------------------------------
# Hundeartige: Wolf und Fuchs teilen den Körperbau, Proportionen und Farben unterscheiden sich
# ---------------------------------------------------------------------------
def _hundeartig(t, schnauze=1.0, ohr=1.0, schwanz=1.0, bein=1.0):
    """Körperformen und Knochen eines schlanken Hundeartigen (Maße wie ein Wolf, Schulter 0,8)."""
    b = bein
    t.form((0, -0.33, 0.6 * b + 0.02), (0.16, 0.25, 0.23))      # Brust, tief
    t.form((0, -0.28, 0.74 * b + 0.05), (0.12, 0.2, 0.1))       # Widerrist
    t.form((0, 0.02, 0.6 * b + 0.04), (0.12, 0.24, 0.16))       # schlanke Taille
    t.form((0, 0.32, 0.62 * b + 0.03), (0.14, 0.19, 0.17))      # Becken
    t.form((0, -0.56, 0.72 * b + 0.06), (0.15, 0.14, 0.17))     # Halskrause
    t.form((0, -0.62, 0.8 * b + 0.08), (0.1, 0.13, 0.12))       # Hals
    kz = 0.87 * b + 0.1
    t.form((0, -0.77, kz), (0.115, 0.12, 0.105))                # Kopf
    t.form((0.075, -0.79, kz - 0.05), (0.065, 0.07, 0.065), spiegeln=True)  # Wangen
    t.form((0, -0.77 - 0.17 * schnauze, kz - 0.045), (0.05, 0.11 * schnauze, 0.05))  # Schnauze
    t.form((0, -0.74 - 0.13 * schnauze, kz - 0.005), (0.045, 0.09 * schnauze, 0.04))  # Nasenrücken
    # Buschiger, hängender Schwanz
    sd = schwanz
    t.glied((0, 0.46, 0.66 * b + 0.05), (0, 0.66, 0.52 * b), 0.06 * sd, 0.085 * sd, spiegeln=False)
    t.glied((0, 0.66, 0.52 * b), (0, 0.8 + 0.08 * sd, 0.36 * b), 0.085 * sd, 0.06 * sd, spiegeln=False)
    # Vorderbeine
    t.glied((0.1, -0.34, 0.62 * b), (0.1, -0.33, 0.34 * b), 0.075, 0.05)
    t.glied((0.1, -0.33, 0.34 * b), (0.1, -0.36, 0.04), 0.045, 0.035)
    t.form((0.1, -0.385, 0.035), (0.045, 0.065, 0.035), spiegeln=True)
    # Hinterbeine: Oberschenkel, Unterschenkel, Mittelfuß
    t.glied((0.11, 0.34, 0.62 * b), (0.11, 0.27, 0.4 * b), 0.1, 0.07)
    t.glied((0.11, 0.27, 0.4 * b), (0.1, 0.42, 0.18 * b), 0.05, 0.04)
    t.glied((0.1, 0.42, 0.18 * b), (0.1, 0.4, 0.04), 0.038, 0.034)
    t.form((0.1, 0.38, 0.035), (0.045, 0.065, 0.035), spiegeln=True)

    t.knochen_dazu("Koerper", (0, 0.36, 0.66 * b + 0.04), (0, -0.36, 0.7 * b + 0.05), None)
    t.knochen_dazu("Hals", (0, -0.36, 0.7 * b + 0.05), (0, -0.7, 0.84 * b + 0.09), "Koerper")
    t.knochen_dazu("Kopf", (0, -0.7, 0.84 * b + 0.09), (0, -0.95 - 0.1 * schnauze, kz - 0.05), "Hals")
    t.knochen_dazu("Schwanz1", (0, 0.46, 0.66 * b + 0.05), (0, 0.66, 0.52 * b), "Koerper")
    t.knochen_dazu("Schwanz2", (0, 0.66, 0.52 * b), (0, 0.84 + 0.08 * sd, 0.34 * b), "Schwanz1")
    t.bein_knochen("V", 0.1, (0.1, -0.34, 0.64 * b), (0.1, -0.33, 0.34 * b), (0.1, -0.37, 0.03))
    t.bein_knochen("H", 0.11, (0.11, 0.33, 0.64 * b), (0.11, 0.27, 0.4 * b), (0.1, 0.4, 0.03))
    return kz


def wolf(seed=4, name="Wolf"):
    t = Tier(name, seed, massstab=1.0)
    kz = _hundeartig(t, schnauze=1.05, ohr=1.0, schwanz=1.15, bein=1.0)
    grau, sattel, hell = farbe("#8C9098"), farbe("#5C6069"), farbe("#DAD6CE")
    wange, spitze = farbe("#CFCAC1"), farbe("#35363A")

    def zonen(p, n):
        c = grau
        if p.z > 0.72 and n.z > 0.35 and -0.45 < p.y < 0.5:
            c = sattel                                          # dunkler Sattel auf dem Rücken
        if n.z < -0.35 or (p.y < -0.45 and p.z < 0.72 and n.y < -0.2):
            c = hell                                            # Bauch, Brust, Kehle
        if p.y < -0.8 and p.z < kz - 0.03:
            c = wange                                           # helle Lefzen und Kinn
        if p.z < 0.3 and abs(p.x) > 0.04:
            c = grau.lerp(hell, 0.5)                            # Läufe heller
        if p.y > 0.82:
            c = spitze                                          # dunkle Schwanzspitze
        return c * (0.92 + 0.12 * _weich(0.2, 0.9, p.z))

    t.koerper(zonen, ziel=2200)
    t.ohr(t.v(0.065, -0.72, kz + 0.07), t.v(0.1, -0.69, kz + 0.2), 0.08, 0.03, "#5C6069", "#C9C2B8")
    t.ohr(t.v(-0.065, -0.72, kz + 0.07), t.v(-0.1, -0.69, kz + 0.2), 0.08, 0.03, "#5C6069", "#C9C2B8")
    for ort in t.augen_orte(0.058, kz + 0.035):
        t.auge(ort, 0.018)
    t.kugel(t.v(0, -1.06, kz - 0.02), 0.026, farbe("#141111"), groesse=(1.2, 0.9, 0.8), name="Nase")
    stil = dict(schritt=24, schwung=26, knick=45, wippen=0.02, sprung=15, schwung_renn=42, huepfen=0.08, wedeln=0.6)
    t.fertig(stil)
    print("SCHRITT Wolf", round(schrittweite(0.6, 26, 24, 0.6), 2), round(schrittweite(0.6, 42, 15, 0.4), 2))


def fuchs(seed=5, name="Fuchs"):
    t = Tier(name, seed, massstab=0.52)
    kz = _hundeartig(t, schnauze=1.1, ohr=1.4, schwanz=1.5, bein=0.9)
    orange, ruecken = farbe("#D8742A"), farbe("#BE5E20")
    weiss, schwarz = farbe("#F2EDE3"), farbe("#2A2320")

    def zonen(p, n):
        c = orange
        if p.z > 0.72 and n.z > 0.4:
            c = ruecken
        if n.z < -0.3 or (p.y < -0.42 and p.z < 0.68 and n.y < -0.1):
            c = weiss                                           # Brust, Bauch, Kehle
        if p.y < -0.8 and p.z < kz - 0.02:
            c = weiss                                           # weiße Wangen und Kinn
        if p.z < 0.3 and abs(p.x) > 0.04:
            c = schwarz                                         # schwarze „Strümpfe“
        if p.y > 0.86:
            c = weiss                                           # weiße Schwanzspitze
        return c

    t.koerper(zonen, ziel=2000)
    for sx in (1, -1):
        t.ohr(t.v(0.06 * sx, -0.72, kz + 0.07), t.v(0.1 * sx, -0.69, kz + 0.2), 0.1 * t.s, 0.035 * t.s, "#2A2320", "#F2EDE3")
    for ort in t.augen_orte(0.055, kz + 0.035):
        t.auge(ort, 0.02 * t.s)
    t.kugel(t.v(0, -1.1, kz - 0.02), 0.028 * t.s, farbe("#141111"), groesse=(1.2, 0.9, 0.8), name="Nase")
    stil = dict(schritt=18, schwung=32, knick=50, wippen=0.012, sprung=12, schwung_renn=50, huepfen=0.05, wedeln=0.8)
    t.fertig(stil)
    print("SCHRITT Fuchs", round(schrittweite(0.3, 32, 18, 0.6), 2), round(schrittweite(0.3, 50, 12, 0.4), 2))


# ---------------------------------------------------------------------------
# Reh: zierlich, lange dünne Beine, langer Hals, große Ohren, weißer Spiegel am Hinterteil
# ---------------------------------------------------------------------------
def reh(seed=6, name="Reh"):
    t = Tier(name, seed, massstab=1.0)
    t.form((0, -0.28, 0.73), (0.13, 0.24, 0.19))       # Brust
    t.form((0, 0.02, 0.72), (0.14, 0.26, 0.17))        # Bauch
    t.form((0, 0.3, 0.75), (0.13, 0.18, 0.17))         # Becken
    t.form((0, 0.32, 0.86), (0.1, 0.14, 0.07))         # Kruppe
    t.glied((0, -0.4, 0.84), (0, -0.58, 1.1), 0.085, 0.065, spiegeln=False)  # schräger Hals
    t.form((0, -0.66, 1.19), (0.07, 0.09, 0.075))      # Kopf
    t.form((0, -0.79, 1.14), (0.045, 0.08, 0.05))      # Schnauze
    t.form((0, 0.43, 0.83), (0.035, 0.03, 0.045))      # Stummelschwanz
    t.glied((0.08, -0.3, 0.7), (0.08, -0.29, 0.42), 0.055, 0.035)
    t.glied((0.08, -0.29, 0.42), (0.08, -0.31, 0.04), 0.028, 0.022)
    t.form((0.08, -0.325, 0.035), (0.025, 0.035, 0.035), spiegeln=True)
    t.glied((0.09, 0.3, 0.74), (0.09, 0.23, 0.5), 0.085, 0.055)
    t.glied((0.09, 0.23, 0.5), (0.08, 0.36, 0.26), 0.035, 0.026)
    t.glied((0.08, 0.36, 0.26), (0.08, 0.34, 0.04), 0.024, 0.022)
    t.form((0.08, 0.325, 0.035), (0.025, 0.035, 0.035), spiegeln=True)

    braun, ruecken, bauch = farbe("#B8683A"), farbe("#9A532C"), farbe("#E1C29C")
    weiss, huf = farbe("#F3EDE2"), farbe("#2A2420")

    def zonen(p, n):
        c = braun
        if p.z > 0.8 and n.z > 0.4 and p.y > -0.45:
            c = ruecken
        if n.z < -0.35 or (p.z < 0.55 and abs(p.x) < 0.1 and p.y > -0.3):
            c = bauch                                   # Bauch und Innenseite der Beine
        if p.y > 0.36 and p.z > 0.62 and n.y > 0.25:
            c = weiss                                   # heller Spiegel am Hinterteil
        if p.y < -0.78 and p.z < 1.14:
            c = weiss                                   # helles Maul
        if p.z < 0.07:
            c = huf
        return c

    t.koerper(zonen, aufloesung=0.024, ziel=2200)
    for sx in (1, -1):
        t.ohr(t.v(0.05 * sx, -0.62, 1.25), t.v(0.13 * sx, -0.6, 1.34), 0.075, 0.022, "#9A532C", "#E8CDB6", rund=0.8)
    for ort in t.augen_orte(0.045, 1.21):
        t.auge(ort, 0.02)
    t.kugel(t.v(0, -0.865, 1.15), 0.025, farbe("#141111"), groesse=(1.2, 0.8, 0.9), name="Nase")

    t.knochen_dazu("Koerper", (0, 0.32, 0.78), (0, -0.32, 0.8), None)
    t.knochen_dazu("Hals", (0, -0.36, 0.82), (0, -0.62, 1.15), "Koerper")
    t.knochen_dazu("Kopf", (0, -0.62, 1.15), (0, -0.86, 1.13), "Hals")
    t.knochen_dazu("Schwanz1", (0, 0.4, 0.84), (0, 0.46, 0.8), "Koerper")
    t.bein_knochen("V", 0.08, (0.08, -0.3, 0.72), (0.08, -0.29, 0.42), (0.08, -0.31, 0.03))
    t.bein_knochen("H", 0.09, (0.09, 0.3, 0.76), (0.09, 0.23, 0.5), (0.08, 0.34, 0.03))
    stil = dict(schritt=26, schwung=24, knick=55, wippen=0.015, sprung=16, schwung_renn=46, huepfen=0.14, biegen=1.4,
                grasen=(75, 45), wedeln=1.5)
    t.fertig(stil)
    print("SCHRITT Reh", round(schrittweite(0.72, 24, 26, 0.6), 2), round(schrittweite(0.72, 46, 16, 0.4), 2))


# ---------------------------------------------------------------------------
# Schaf: flauschiger, buckliger Wollkörper, dunkles Gesicht und dunkle Beine
# ---------------------------------------------------------------------------
def schaf(seed=7, name="Schaf"):
    t = Tier(name, seed, massstab=1.0)
    r = t.rng
    t.form((0, 0.02, 0.62), (0.25, 0.4, 0.23))          # Wollkörper
    t.form((0, -0.36, 0.68), (0.17, 0.16, 0.18))        # Wollbrust/Hals
    # Wollbüschel: machen die Silhouette buckelig und flauschig
    for _ in range(46):
        w = r.uniform(-0.3, math.pi + 0.3)          # nur obere Hälfte und Flanken
        y = r.uniform(-0.42, 0.42)
        m = Vector((math.cos(w) * 0.25, y, 0.6 + math.sin(w) * 0.22))
        t.form(m, (r.uniform(0.09, 0.13),) * 3)
    t.form((0, -0.55, 0.76), (0.075, 0.1, 0.085))       # Kopf
    t.form((0, -0.66, 0.71), (0.055, 0.075, 0.06))      # Schnauze
    t.form((0, -0.5, 0.86), (0.09, 0.08, 0.06))         # Wollkappe
    t.form((0, 0.44, 0.62), (0.06, 0.06, 0.08))         # Schwanz
    t.glied((0.12, -0.26, 0.46), (0.12, -0.27, 0.24), 0.045, 0.035)
    t.glied((0.12, -0.27, 0.24), (0.12, -0.28, 0.04), 0.032, 0.03)
    t.form((0.12, -0.29, 0.03), (0.035, 0.04, 0.03), spiegeln=True)
    t.glied((0.12, 0.28, 0.46), (0.12, 0.25, 0.25), 0.05, 0.035)
    t.glied((0.12, 0.25, 0.25), (0.12, 0.3, 0.04), 0.032, 0.03)
    t.form((0.12, 0.29, 0.03), (0.035, 0.04, 0.03), spiegeln=True)

    wolle, creme, gesicht = farbe("#F4EFE4"), farbe("#E6DCC8"), farbe("#3B3431")

    def zonen(p, n):
        c = wolle.lerp(creme, 0.5 + 0.5 * math.sin(p.x * 23 + p.y * 17 + p.z * 29))
        if p.z < 0.38:
            c = gesicht                                     # dunkle Beine
        if (p - Vector((0, -0.61, 0.74))).length < 0.15 and not (p.z > 0.82 and p.y > -0.6):
            c = gesicht                                     # dunkles Gesicht, Wollkappe bleibt weiß
        if p.z < 0.05:
            c = gesicht * 0.6                               # Hufe
        return c

    t.koerper(zonen, aufloesung=0.026, beulen=0.03, ziel=2400)
    for sx in (1, -1):
        t.ohr(t.v(0.06 * sx, -0.53, 0.82), t.v(0.18 * sx, -0.5, 0.78), 0.065, 0.022, "#3B3431", "#6E5B55", rund=0.6)
    for ort in t.augen_orte(0.05, 0.79):
        t.auge(ort, 0.016, c="#1C1511")
    t.kugel(t.v(0, -0.715, 0.72), 0.02, farbe("#1A1412"), groesse=(1.3, 0.8, 0.8), name="Nase")

    t.knochen_dazu("Koerper", (0, 0.34, 0.64), (0, -0.34, 0.68), None)
    t.knochen_dazu("Hals", (0, -0.34, 0.68), (0, -0.5, 0.78), "Koerper")
    t.knochen_dazu("Kopf", (0, -0.5, 0.78), (0, -0.7, 0.72), "Hals")
    t.knochen_dazu("Schwanz1", (0, 0.42, 0.64), (0, 0.5, 0.56), "Koerper")
    t.bein_knochen("V", 0.12, (0.12, -0.26, 0.5), (0.12, -0.27, 0.24), (0.12, -0.28, 0.03))
    t.bein_knochen("H", 0.12, (0.12, 0.28, 0.5), (0.12, 0.25, 0.25), (0.12, 0.3, 0.03))
    stil = dict(schritt=22, schwung=24, knick=40, wippen=0.015, sprung=14, schwung_renn=34, huepfen=0.06, grasen=(22, 40), wedeln=1.0)
    t.fertig(stil)
    print("SCHRITT Schaf", round(schrittweite(0.47, 24, 22, 0.6), 2), round(schrittweite(0.47, 34, 14, 0.4), 2))


# ---------------------------------------------------------------------------
# Bär: massig, Schulterbuckel, runde Ohren (Körperform aus baer_form, wie der realistische Bär)
# ---------------------------------------------------------------------------
def baer(seed=8, name="Baer"):
    import baer_form

    t = Tier(name, seed, massstab=1.0)
    for mitte, halb in baer_form.FORMEN:
        t.form(mitte, halb)
    braun, dunkel, schnauze = farbe("#6E4526"), farbe("#452A17"), farbe("#A07650")

    def zonen(p, n):
        c = braun
        if p.z > 1.05 and n.z > 0.3:
            c = braun * 1.12                                # sonnengebleichter Rücken
        if p.z < 0.45:
            c = dunkel                                      # dunkle Beine
        if p.y < -1.2 and p.z < 1.0:
            c = schnauze
        return c

    t.koerper(zonen, aufloesung=0.035, beulen=0.02, ziel=2300)
    for ort in t.augen_orte(0.11, 1.04, tiefe=0.008):
        t.auge(ort, 0.024)
    t.kugel(baer_form.NASE, 0.045, farbe("#141111"), groesse=(1.35, 0.75, 0.85), name="Nase")
    t.knochen_dazu("Koerper", (0, 0.62, 0.97), (0, -0.5, 1.05), None)
    t.knochen_dazu("Hals", (0, -0.5, 1.05), (0, -0.86, 1.02), "Koerper")
    t.knochen_dazu("Kopf", (0, -0.86, 1.02), (0, -1.42, 0.92), "Hals")
    t.knochen_dazu("Schwanz1", (0, 0.92, 0.92), (0, 1.1, 0.9), "Koerper")
    t.bein_knochen("V", 0.24, (0.24, -0.42, 0.92), (0.24, -0.46, 0.5), (0.24, -0.5, 0.07))
    t.bein_knochen("H", 0.23, (0.23, 0.6, 0.92), (0.24, 0.63, 0.5), (0.24, 0.6, 0.07))
    stil = dict(schritt=26, schwung=24, knick=40, wippen=0.025, sprung=18, schwung_renn=34, huepfen=0.1, wedeln=0.3, atmen=1.5)
    t.fertig(stil)
    print("SCHRITT Baer", round(schrittweite(0.85, 24, 26, 0.6), 2), round(schrittweite(0.85, 34, 18, 0.4), 2))
