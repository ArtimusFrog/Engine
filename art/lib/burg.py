"""Große Burg im gotischen Fantasy-Stil (Detailgrad wie POLYGON Dark Fantasy).

Die feinen Oberflächen – Steinquader, Dachschindeln, Bodenplatten, Buntglas – zeichnet der Shader
der Engine (`surface_pattern` in basic.wgsl). Dafür legt dieses Skript Musterkoordinaten in
Metern in die UVs: u = Art·1000 + 500 + x, v = y. Modelliert werden die Formen und die gotischen
Details: Strebepfeiler mit Fialen, Strebebögen, Spitzbogenfenster mit Maßwerk, Fensterrose,
Portal, Balustraden, Gauben, Türme mit Turmhelmen, Banner.

Alles wird in einem einzigen Mesh gesammelt (schnell, auch bei vielen tausend Teilen).
Koordinaten: Z oben, Vorderseite (Portal) nach -Y, Maße in Metern.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

from lager import leucht_material
from werkstatt import srgb_zu_linear, ursprung_unten

# Musterarten (siehe basic.wgsl)
MAUER, ZIEGEL, PLATTEN, GLAS, KOPFSTEIN = 1, 2, 3, 4, 5

STEIN = "#D9C29E"
STEIN_HELL = "#E8D8B8"
SOCKEL = "#9C8C7A"
DACH = "#63A36A"
DACH_DUNKEL = "#3F7A4E"
GOLD = "#E8B64A"
HOLZ = "#6B4426"
EISEN = "#3B3B42"
GLAS_FARBE = "#FFFFFF"
BANNER = "#2E6B46"


def farbe(h):
    return Vector(srgb_zu_linear(h)[:3])


# ---------------------------------------------------------------------------
# Sammelbecken: alle Teile landen in einer Liste von Punkten und Flächen
# ---------------------------------------------------------------------------
class Bau:
    def __init__(self, seed=1):
        self.punkte = []
        self.flaechen = []
        self.farben = []
        self.muster = []
        self.leuchten = []
        self.zufall = random.Random(seed)

    def teil(self, geo, hex_farbe, muster=0, m=None, leuchten=False, schwankung=0.025):
        punkte, flaechen = geo
        m = Matrix.Identity(4) if m is None else m
        spiegeln = m.to_3x3().determinant() < 0
        basis = len(self.punkte)
        self.punkte.extend(m @ Vector(p) for p in punkte)
        c = farbe(hex_farbe)
        for f in flaechen:
            f = list(reversed(f)) if spiegeln else f
            self.flaechen.append([basis + i for i in f])
            self.farben.append(c * (1.0 + self.zufall.uniform(-schwankung, schwankung)))
            self.muster.append(muster)
            self.leuchten.append(leuchten)

    def fertig(self, name, glas_leuchten=0.3, ursprung=None):
        """Ein Objekt aus allen Teilen. `ursprung` = (x, y): dieser Punkt wird zum Ursprung
        (Boden bleibt bei z = 0), sonst mittig unter das Modell."""
        mesh = bpy.data.meshes.new(name)
        mesh.from_pydata([tuple(p) for p in self.punkte], [], self.flaechen)
        attr = mesh.color_attributes.new("Farbe", "FLOAT_COLOR", "CORNER")
        uv = mesh.uv_layers.new(name="Muster")
        for poly in mesh.polygons:
            i = poly.index
            c, art = self.farben[i], self.muster[i]
            n = poly.normal
            for li in poly.loop_indices:
                attr.data[li].color = (c.x, c.y, c.z, 1.0)
                if art:
                    u, v = muster_uv(art, mesh.vertices[mesh.loops[li].vertex_index].co, n)
                    uv.data[li].uv = (art * 1000.0 + 500.0 + u, v)
                else:
                    uv.data[li].uv = (0.0, 0.0)
            poly.use_smooth = False
            poly.material_index = 1 if self.leuchten[i] else 0
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        mesh.materials.append(_material())
        mesh.materials.append(leucht_material(name + "Glas", "#FFE2A8", glas_leuchten))
        if ursprung is None:
            ursprung_unten(obj)
        else:
            mesh.transform(Matrix.Translation((-ursprung[0], -ursprung[1], 0.0)))
            mesh.update()
            print(f"MODELL {name}: Unterkante {min(v.co.z for v in mesh.vertices):.3f} m")
        dreiecke = sum(len(p.vertices) - 2 for p in mesh.polygons)
        print(f"MODELL {name}: {dreiecke} Dreiecke")
        return obj


def _material():
    mat = bpy.data.materials.get("Burg")
    if mat is None:
        mat = bpy.data.materials.new("Burg")
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
    return mat


def muster_uv(art, p, n):
    """Musterkoordinaten einer Fläche in Metern: an Wänden waagerecht entlang der Wand und
    senkrecht nach oben, auf Dächern quer und die Schräge hinauf, auf Böden x/y."""
    t = Vector((-n.y, n.x, 0.0))
    if art in (PLATTEN, KOPFSTEIN) or t.length < 0.2:
        return p.x, p.y
    t.normalize()
    if art == ZIEGEL:
        b = n.cross(t)
        if b.z < 0:
            b = -b
        return p.dot(t), p.dot(b)
    return p.dot(t), p.z


# ---------------------------------------------------------------------------
# Grundformen: liefern (Punkte, Flächen) mit nach außen zeigenden Flächen
# ---------------------------------------------------------------------------
def aus_bm(bm):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.verts.index_update()
    geo = ([tuple(v.co) for v in bm.verts], [[v.index for v in f.verts] for f in bm.faces])
    bm.free()
    return geo


def quader(sx, sy, sz, fase=0.0):
    """Quader, Mitte der Unterseite im Ursprung."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.transform(bm, matrix=Matrix.Translation((0, 0, 0.5)), verts=bm.verts)
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((sx, sy, sz, 1.0)), verts=bm.verts)
    if fase > 0:
        bmesh.ops.bevel(bm, geom=bm.edges[:], offset=min(fase, sx * 0.3, sy * 0.3, sz * 0.3), offset_type="OFFSET",
                        segments=1, profile=0.5, affect="EDGES", clamp_overlap=True)
    return aus_bm(bm)


def zylinder(r, h, ecken=8, r_oben=None, drehung=0.0):
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=ecken, radius1=r,
                          radius2=r if r_oben is None else r_oben, depth=h)
    bmesh.ops.transform(bm, matrix=Matrix.Translation((0, 0, h / 2)) @ Matrix.Rotation(drehung, 4, "Z"), verts=bm.verts)
    return aus_bm(bm)


def pyramide(r, h, ecken=4, drehung=math.pi / 4):
    """Spitze (Turmhelm, Fialenspitze); bei 4 Ecken mit den Seiten parallel zu den Achsen."""
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=ecken, radius1=r, radius2=0.0, depth=h)
    bmesh.ops.transform(bm, matrix=Matrix.Translation((0, 0, h / 2)) @ Matrix.Rotation(drehung, 4, "Z"), verts=bm.verts)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    return aus_bm(bm)


def drehkoerper(profil, ecken=6):
    """Rotationskörper aus (Radius, Höhe)-Paaren von unten nach oben (Baluster, Knäufe)."""
    bm = bmesh.new()
    ringe = []
    for r, z in profil:
        ringe.append([bm.verts.new((math.cos(math.tau * k / ecken) * r, math.sin(math.tau * k / ecken) * r, z)) for k in range(ecken)])
    for a, b in zip(ringe, ringe[1:]):
        for k in range(ecken):
            bm.faces.new((a[k], a[(k + 1) % ecken], b[(k + 1) % ecken], b[k]))
    bm.faces.new(ringe[0])
    bm.faces.new(ringe[-1])
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    return aus_bm(bm)


def prisma(profil, laenge):
    """2D-Profil in der (y, z)-Ebene, entlang X von 0 bis `laenge` extrudiert."""
    bm = bmesh.new()
    a = [bm.verts.new((0.0, y, z)) for y, z in profil]
    b = [bm.verts.new((laenge, y, z)) for y, z in profil]
    n = len(profil)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((a[i], a[j], b[j], b[i]))
    bm.faces.new(a)
    bm.faces.new(b)
    return aus_bm(bm)


def platte(umriss, tiefe, y0=0.0):
    """Umriss in der (x, z)-Ebene als Platte von y0 - tiefe bis y0 (Glas, Türen)."""
    bm = bmesh.new()
    vorne = [bm.verts.new((x, y0 - tiefe, z)) for x, z in umriss]
    hinten = [bm.verts.new((x, y0, z)) for x, z in umriss]
    n = len(umriss)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((vorne[i], vorne[j], hinten[j], hinten[i]))
    bm.faces.new(vorne)
    bm.faces.new(hinten)
    return aus_bm(bm)


def rahmen(aussen, innen, tiefe, y0=0.0, offen=False):
    """Ring zwischen zwei gleich langen Umrissen (x, z), von y0 - tiefe bis y0.
    `offen`: Linien statt geschlossener Umrisse (Enden werden verschlossen)."""
    bm = bmesh.new()
    ov = [bm.verts.new((x, y0 - tiefe, z)) for x, z in aussen]
    ob = [bm.verts.new((x, y0, z)) for x, z in aussen]
    iv = [bm.verts.new((x, y0 - tiefe, z)) for x, z in innen]
    ib = [bm.verts.new((x, y0, z)) for x, z in innen]
    n = len(aussen)
    for i in range(n - 1 if offen else n):
        j = (i + 1) % n
        bm.faces.new((ov[i], ov[j], iv[j], iv[i]))
        bm.faces.new((ob[i], ib[i], ib[j], ob[j]))
        bm.faces.new((ov[i], ob[i], ob[j], ov[j]))
        bm.faces.new((iv[i], iv[j], ib[j], ib[i]))
    if offen:
        for i in (0, n - 1):
            bm.faces.new((ov[i], iv[i], ib[i], ob[i]))
    return aus_bm(bm)


def bogen(hw, hs, radius, z0=0.0, schritte=6, nur_bogen=False):
    """Umriss eines Spitzbogens (radius > hw) oder Rundbogens (radius = hw): halbe Breite `hw`,
    Kämpferhöhe `hs`, Fuß bei z0. Gegen den Uhrzeigersinn von vorne gesehen."""
    cx = hw - radius
    a_max = math.acos(max(-1.0, min(1.0, -cx / radius)))
    rechts = [(cx + radius * math.cos(a_max * i / schritte), hs + radius * math.sin(a_max * i / schritte)) for i in range(schritte + 1)]
    links = [(-x, z) for x, z in reversed(rechts[:-1])]
    linie = rechts + links
    if nur_bogen:
        return linie
    return [(hw, z0)] + linie + [(-hw, z0)]


def bogen_hoehe(hw, radius):
    cx = hw - radius
    return math.sqrt(max(radius * radius - cx * cx, 0.0))


def kreis(r, ecken, cx=0.0, cz=0.0):
    return [(cx + math.cos(math.tau * k / ecken) * r, cz + math.sin(math.tau * k / ecken) * r) for k in range(ecken)]


# ---------------------------------------------------------------------------
# Lage: Matrizen
# ---------------------------------------------------------------------------
def M(ort=(0, 0, 0), rz=0.0, rx=0.0, ry=0.0, s=1.0):
    rot = Matrix.Rotation(math.radians(rz), 4, "Z") @ Matrix.Rotation(math.radians(ry), 4, "Y") @ Matrix.Rotation(math.radians(rx), 4, "X")
    skala = Matrix.Scale(s, 4) if isinstance(s, (int, float)) else Matrix.Diagonal((*s, 1.0))
    return Matrix.Translation(Vector(ort)) @ rot @ skala


# ---------------------------------------------------------------------------
# Bauteile (lokal: Wandfläche bei y = 0, außen = -Y, Fuß bei z = 0)
# ---------------------------------------------------------------------------
def fenster(bau, m, breite, hoehe, spitz=1.25, masswerk=True, tiefe=0.4, rand=0.2, glas=True, innenseite=None):
    """Spitzbogenfenster: Gewände mit zweiter Laibung, Buntglas, Mittelpfosten mit Maßwerk
    (zwei Lanzetten und ein Kreis), Fensterbank und Überstab mit Kopfsteinen."""
    hw = breite / 2
    r = breite * spitz
    hs = hoehe - bogen_hoehe(hw, r)
    aussen = bogen(hw, hs, r)
    innen = bogen(hw - rand, hs, r - rand, rand)
    bau.teil(rahmen(aussen, innen, tiefe), STEIN_HELL, m=m)
    tief = bogen(hw - rand - 0.1, hs, r - rand - 0.1, rand + 0.1)
    bau.teil(rahmen(innen, tief, tiefe * 0.55), STEIN, m=m)
    if glas:
        bau.teil(platte(tief, 0.04, -0.02), GLAS_FARBE, GLAS, m=m, leuchten=True)
    else:
        bau.teil(platte(tief, 0.04, -0.02), "#1C1A1E", m=m)
    if innenseite is not None:
        # Innenseite der Wand (`innenseite` Meter hinter der Außenfläche): Glas und schlichtes Gewände
        bau.teil(platte(tief, 0.04, innenseite + 0.05), GLAS_FARBE if glas else "#1C1A1E", GLAS if glas else 0, m=m, leuchten=glas)
        bau.teil(rahmen(bogen(hw - rand + 0.12, hs, r - rand + 0.12, rand - 0.12), tief, 0.1, innenseite + 0.1), STEIN_HELL, m=m)
    # Fensterbank und Überstab
    bau.teil(quader(breite + 0.35, tiefe + 0.2, 0.16, 0.03), STEIN_HELL, m=m @ M((0, -(tiefe + 0.2) / 2, -0.12)))
    ueber = bogen(hw + 0.28, hs, r + 0.28, nur_bogen=True)
    unter = bogen(hw + 0.1, hs, r + 0.1, nur_bogen=True)
    bau.teil(rahmen(ueber, unter, 0.18, offen=True), STEIN_HELL, m=m)
    for s in (-1, 1):
        bau.teil(quader(0.26, 0.26, 0.3, 0.03), STEIN_HELL, m=m @ M((s * (hw + 0.19), -0.13, hs - 0.3)))
    if masswerk and breite >= 1.5:
        innen_hw = hw - rand - 0.1
        d = tiefe * 0.45
        # Mittelpfosten
        bau.teil(quader(0.14, d, hs - rand + 0.2), STEIN_HELL, m=m @ M((0, -d / 2, rand)))
        # Zwei kleine Lanzetten
        klein = innen_hw / 2
        rk = klein * 2 * spitz
        for s in (-1, 1):
            a = bogen(klein, hs, rk, nur_bogen=True)
            b = bogen(klein - 0.08, hs, rk - 0.08, nur_bogen=True)
            bau.teil(rahmen(a, b, d, offen=True), STEIN_HELL, m=m @ M((s * klein, 0, 0)))
        # Kreis im Bogenfeld
        oben = hs + bogen_hoehe(innen_hw, r - rand - 0.1)
        kz = (hs + bogen_hoehe(klein, rk) + oben) / 2 + 0.05
        kr = min(klein * 0.8, (oben - kz) * 0.75)
        if kr > 0.12:
            bau.teil(rahmen(kreis(kr, 12, 0, kz), kreis(kr - 0.08, 12, 0, kz), d), STEIN_HELL, m=m)


def _kreuzblume(bau, m, g):
    bau.teil(zylinder(g * 0.3, g * 0.9, 6), GOLD, m=m)
    for k in range(4):
        bau.teil(pyramide(g * 0.25, g * 0.9, 3), GOLD, m=m @ M((0, 0, g * 0.5), k * 90) @ M((g * 0.35, 0, 0), 0, 0, 55))
    bau.teil(pyramide(g * 0.28, g * 1.1, 4), GOLD, m=m @ M((0, 0, g * 0.9)))


def kreuz(bau, m, h=1.6):
    bau.teil(quader(0.14, 0.14, h, 0.02), GOLD, m=m)
    bau.teil(quader(0.8 * h / 1.6, 0.14, 0.14, 0.02), GOLD, m=m @ M((0, 0, h * 0.62)))
    bau.teil(drehkoerper([(0.0, -0.05), (0.14, 0.0), (0.14, 0.12), (0.0, 0.2)], 6), GOLD, m=m @ M((0, 0, h - 0.02)))


def fiale(bau, m, breite, hoehe, schaft=0.45, krabben=True):
    """Fiale: Schaft mit vier Ziergiebeln, darüber eine grüne Spitze mit Krabben und Kreuzblume."""
    h_schaft = hoehe * schaft
    bau.teil(quader(breite * 1.12, breite * 1.12, 0.22, 0.03), STEIN_HELL, m=m)
    bau.teil(quader(breite, breite, h_schaft), STEIN, MAUER, m=m)
    bau.teil(quader(breite * 1.1, breite * 1.1, 0.16, 0.03), STEIN_HELL, m=m @ M((0, 0, h_schaft - 0.16)))
    # Ziergiebel auf allen vier Seiten
    g = breite * 0.5
    for k in range(4):
        seite = m @ M((0, 0, h_schaft), k * 90) @ M((0, -g, 0))
        bau.teil(platte([(-g, 0), (g, 0), (0, g * 1.3)], 0.1, 0.0), STEIN_HELL, m=seite)
        bau.teil(pyramide(0.07, 0.24), STEIN_HELL, m=seite @ M((0, -0.05, g * 1.3 - 0.04)))
    # Spitze mit Krabben an den Kanten
    h_spitze = hoehe - h_schaft
    r = breite * 0.62
    bau.teil(pyramide(r, h_spitze), DACH, ZIEGEL, m=m @ M((0, 0, h_schaft)))
    if krabben:
        kante = r / math.sqrt(2)
        for i in range(1, 5):
            t = i / 5.2
            for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1)):
                x, y = sx * kante * (1 - t), sy * kante * (1 - t)
                w = math.degrees(math.atan2(sy, sx))
                bau.teil(pyramide(0.08 * (1.2 - t * 0.5), 0.22, 3), DACH_DUNKEL,
                         m=m @ M((x, y, h_schaft + h_spitze * t), w) @ M((0.05, 0, 0), 0, 0, 70))
    _kreuzblume(bau, m @ M((0, 0, hoehe - 0.05)), 0.22)


def strebepfeiler(bau, m, hoehe1, hoehe2, fiale_hoehe, breite=1.3, tiefe=2.4):
    """Gestufter Strebepfeiler vor einer Wand (y = 0 ist die Wand), mit grünem Wasserschlag."""
    bau.teil(quader(breite + 0.2, tiefe + 0.2, 1.2, 0.04), SOCKEL, MAUER, m=m @ M((0, -tiefe / 2 - 0.1, 0)))
    bau.teil(quader(breite, tiefe, hoehe1), STEIN, MAUER, m=m @ M((0, -tiefe / 2, 0)))
    t2 = tiefe * 0.55
    bau.teil(prisma([(-tiefe - 0.1, hoehe1), (-t2, hoehe1), (-t2, hoehe1 + (tiefe - t2) * 0.9)], breite + 0.16), DACH, ZIEGEL,
             m=m @ M((-(breite + 0.16) / 2, 0, 0)))
    bau.teil(quader(breite * 0.86, t2, hoehe2 - hoehe1), STEIN, MAUER, m=m @ M((0, -t2 / 2, hoehe1)))
    bau.teil(quader(breite * 0.86 + 0.2, t2 + 0.2, 0.22, 0.04), STEIN_HELL, m=m @ M((0, -t2 / 2, hoehe2 - 0.1)))
    # Kleines Blendfenster in der oberen Stufe
    bau.teil(platte(bogen(0.22, (hoehe2 - hoehe1) * 0.35, 0.3), 0.05), "#5A4A3E", m=m @ M((0, -t2, hoehe1 + (hoehe2 - hoehe1) * 0.3)))
    fiale(bau, m @ M((0, -t2 / 2, hoehe2 + 0.1)), breite * 0.7, fiale_hoehe)


def strebebogen(bau, m, spann, z_pfeiler, z_wand, breite=0.55):
    """Strebebogen vom Pfeiler (y = -spann) zur Wand (y = 0)."""
    unten_wand = z_wand - 0.9
    fuss = z_pfeiler - 2.6
    bz = unten_wand - fuss
    oben_pfeiler = z_pfeiler + 1.3
    profil = [(-spann, oben_pfeiler), (0, z_wand + 0.5), (0, unten_wand)]
    for i in range(1, 9):
        t = math.pi / 2 * (1 - i / 9)
        profil.append((-spann * math.cos(t), fuss + bz * math.sin(t)))
    profil.append((-spann, fuss))
    bau.teil(prisma(profil, breite), STEIN, MAUER, m=m @ M((-breite / 2, 0, 0)))
    # Abdeckung oben mit kleinen Krabben
    laenge = math.hypot(spann, z_wand + 0.5 - oben_pfeiler)
    w = math.degrees(math.atan2(z_wand + 0.5 - oben_pfeiler, spann))
    oben = m @ M((0, -spann, oben_pfeiler)) @ M((0, 0, 0), 0, w)
    bau.teil(quader(breite + 0.14, laenge, 0.14, 0.03), STEIN_HELL, m=oben @ M((0, laenge / 2, 0)))
    for i in range(1, 5):
        bau.teil(pyramide(0.1, 0.3, 3), DACH_DUNKEL, m=oben @ M((0, laenge * i / 5, 0.1)))


def balustrade(bau, m, laenge, pfosten=3.0):
    """Brüstung entlang +X ab 0: Sockel, Baluster, Handlauf und Pfosten."""
    bau.teil(quader(laenge, 0.5, 0.2, 0.02), STEIN_HELL, m=m @ M((laenge / 2, 0, 0)))
    bau.teil(quader(laenge, 0.44, 0.14, 0.03), STEIN_HELL, m=m @ M((laenge / 2, 0, 0.9)))
    profil = [(0.09, 0.2), (0.05, 0.28), (0.1, 0.46), (0.11, 0.54), (0.05, 0.76), (0.09, 0.9)]
    anzahl = max(int(laenge / 0.34), 1)
    for i in range(anzahl):
        bau.teil(drehkoerper(profil, 5), STEIN_HELL, m=m @ M(((i + 0.5) * laenge / anzahl, 0, 0)))
    felder = max(int(round(laenge / pfosten)), 1)
    for i in range(felder + 1):
        x = i * laenge / felder
        bau.teil(quader(0.42, 0.56, 1.1), STEIN, m=m @ M((x, 0, 0)))
        bau.teil(quader(0.54, 0.66, 0.12, 0.03), STEIN_HELL, m=m @ M((x, 0, 1.1)))
        bau.teil(pyramide(0.2, 0.32), STEIN_HELL, m=m @ M((x, 0, 1.22)))


def gesims(bau, m, laenge, vor=0.3, hoehe=0.35, farbe_=STEIN_HELL):
    """Profiliertes Gesims entlang +X ab 0 (Unterkante bei z = 0, springt nach -Y vor)."""
    profil = [(0.02, 0), (0.02, hoehe), (-vor, hoehe), (-vor, hoehe * 0.7), (-vor * 0.6, hoehe * 0.45), (-vor * 0.35, 0.0)]
    bau.teil(prisma(profil, laenge), farbe_, m=m)


def bogenfries(bau, m, laenge, abstand=0.9):
    """Reihe kleiner Konsolen unter einem Gesims (entlang +X, unter z = 0)."""
    anzahl = max(int(laenge / abstand), 1)
    for i in range(anzahl):
        x = (i + 0.5) * laenge / anzahl
        bau.teil(prisma([(0.02, 0), (-0.28, 0), (-0.28, -0.14), (-0.08, -0.42), (0.02, -0.42)], 0.24), STEIN_HELL, m=m @ M((x - 0.12, 0, 0)))


def satteldach(bau, m, laenge, hw, traufe, neigung=58.0, ueberstand=0.5, dicke=0.3, first_zier=True, farbe=DACH, first_farbe=DACH_DUNKEL):
    """Satteldach entlang +X von 0 bis `laenge`, First über y = 0, Traufe bei z = `traufe`.
    Gibt die Firsthöhe zurück."""
    tan = math.tan(math.radians(neigung))
    first = traufe + hw * tan
    auf = dicke / math.cos(math.radians(neigung))
    for s in (-1, 1):
        a = (s * (hw + ueberstand), traufe - ueberstand * tan)
        b = (0.0, first)
        profil = [a, b, (b[0], b[1] + auf), (a[0], a[1] + auf)]
        bau.teil(prisma(profil, laenge), farbe, ZIEGEL, m=m)
    bau.teil(quader(laenge + 0.2, 0.42, 0.4, 0.05), first_farbe, m=m @ M((laenge / 2, 0, first + auf - 0.3)))
    if first_zier:
        anzahl = max(int(laenge / 1.5), 1)
        for i in range(anzahl + 1):
            bau.teil(pyramide(0.1, 0.55, 4, 0), GOLD, m=m @ M((i * laenge / anzahl, 0, first + auf + 0.05)))
    return first


def giebelwand(bau, m, hw, traufe, first, dicke=0.8, krabben=True):
    """Dreieckige Giebelwand (lokal: Wand in der x/z-Ebene, außen = -Y) mit Abdeckung und Krabben."""
    bau.teil(platte([(-hw, traufe), (hw, traufe), (0.0, first)], dicke, dicke * 0.5), STEIN, MAUER, m=m)
    laenge = math.hypot(hw, first - traufe)
    w = math.degrees(math.atan2(first - traufe, hw))
    for s in (-1, 1):
        bau.teil(quader(laenge + 0.3, dicke + 0.3, 0.25, 0.04), STEIN_HELL,
                 m=m @ M((s * hw / 2, 0, (traufe + first) / 2), 0, 0, (w if s > 0 else -w)) @ M((0, 0, -0.1)))
        if krabben:
            for i in range(1, 7):
                t = i / 7
                x, z = s * hw * (1 - t), traufe + (first - traufe) * t
                bau.teil(pyramide(0.1, 0.36, 3), DACH_DUNKEL, m=m @ M((x, -dicke / 2 - 0.1, z + 0.12)))


def gaube(bau, m, breite=2.2, hoehe=2.4):
    """Dachgaube: Vorderwand mit Spitzbogenfenster, kleines Satteldach (lokal: vorne = -Y)."""
    tiefe = 3.2
    bau.teil(quader(breite, tiefe, hoehe), STEIN, MAUER, m=m @ M((0, tiefe / 2 - 0.2, 0)))
    fenster(bau, m @ M((0, -0.2, 0.3)), breite * 0.5, hoehe * 0.72, masswerk=False, tiefe=0.2, rand=0.12)
    hw = breite / 2 + 0.1
    first = satteldach(bau, m @ M((0, -0.45, 0), 90), tiefe + 0.5, hw, hoehe, 55.0, 0.35, 0.2, False)
    bau.teil(platte([(-hw + 0.05, hoehe), (hw - 0.05, hoehe), (0.0, first - 0.15)], 0.3, 0.1), STEIN, MAUER, m=m @ M((0, -0.3, 0)))
    bau.teil(quader(breite + 0.3, 0.4, 0.2, 0.03), STEIN_HELL, m=m @ M((0, -0.25, hoehe - 0.2)))
    _kreuzblume(bau, m @ M((0, -0.35, first + 0.1)), 0.18)


def banner(bau, m, breite=1.5, hoehe=4.2):
    """Grünes Banner mit goldenem Rand und Wappen an einer goldenen Stange (hängt nach unten)."""
    bau.teil(zylinder(0.05, breite + 0.5, 6), GOLD, m=m @ M((-(breite + 0.5) / 2, -0.35, 0), 0, 0, 90))
    for s in (-1, 1):
        bau.teil(drehkoerper([(0.0, -0.1), (0.1, 0.0), (0.0, 0.1)], 6), GOLD, m=m @ M((s * (breite / 2 + 0.3), -0.35, 0)))
        bau.teil(quader(0.08, 0.5, 0.08), EISEN, m=m @ M((s * breite * 0.35, -0.2, -0.04)))
    # Tuch mit Schwalbenschwanz, leicht gewellt
    umriss = [(-breite / 2, 0), (-breite / 2, -hoehe), (0, -hoehe + 0.6), (breite / 2, -hoehe), (breite / 2, 0)]
    tuch = platte(umriss, 0.05, -0.3)
    punkte = [(x, y + math.sin(z * 1.4) * 0.06, z) for x, y, z in tuch[0]]
    bau.teil((punkte, tuch[1]), BANNER, m=m, schwankung=0.0)
    # Goldener Saum und Wappen auf beiden Seiten
    for y in (-0.36, -0.29):
        vorne = y < -0.3
        tiefe = 0.012
        y0 = y if vorne else y + tiefe
        for x0, x1 in ((-breite / 2, -breite / 2 + 0.1), (breite / 2 - 0.1, breite / 2)):
            bau.teil(platte([(x0, -0.05), (x1, -0.05), (x1, -hoehe + 0.05), (x0, -hoehe + 0.05)], tiefe, y0), GOLD, m=m)
        wz = -hoehe * 0.42
        raute = [(0, wz + 0.55), (-0.42, wz), (0, wz - 0.55), (0.42, wz)]
        innen = [(0, wz + 0.3), (-0.22, wz), (0, wz - 0.3), (0.22, wz)]
        bau.teil(rahmen(raute, innen, tiefe, y0), GOLD, m=m)
        bau.teil(platte(kreis(0.13, 8, 0, wz), tiefe, y0), GOLD, m=m)


def fensterrose(bau, m, radius, innenseite=None):
    """Große Fensterrose: Rahmen, Buntglas, Speichen, innerer Ring, Pässe am Rand, Nabe."""
    n = 32
    bau.teil(rahmen(kreis(radius + 0.45, n), kreis(radius, n), 0.6), STEIN_HELL, m=m)
    bau.teil(rahmen(kreis(radius + 0.7, n), kreis(radius + 0.45, n), 0.3), STEIN, m=m)
    bau.teil(platte(kreis(radius, n), 0.04, -0.03), GLAS_FARBE, GLAS, m=m, leuchten=True)
    if innenseite is not None:
        bau.teil(platte(kreis(radius, n), 0.04, innenseite + 0.05), GLAS_FARBE, GLAS, m=m, leuchten=True)
        bau.teil(rahmen(kreis(radius + 0.35, n), kreis(radius, n), 0.14, innenseite + 0.14), STEIN_HELL, m=m)
    innen = radius * 0.32
    bau.teil(rahmen(kreis(innen + 0.12, 16), kreis(innen, 16), 0.45), STEIN_HELL, m=m)
    speichen = 12
    for k in range(speichen):
        w = 360 * k / speichen
        laenge = radius - innen
        bau.teil(quader(laenge + 0.1, 0.4, 0.12), STEIN_HELL, m=m @ M((0, 0, 0), 0, 0, -w) @ M((innen + laenge / 2, 0, -0.06)))
        # Pass (kleiner Ring) zwischen zwei Speichen am Rand
        wm = math.radians(w + 180 / speichen)
        pr = (radius - innen) * 0.3
        cx, cz = math.cos(wm) * (radius - pr - 0.12), math.sin(wm) * (radius - pr - 0.12)
        bau.teil(rahmen(kreis(pr, 10, cx, cz), kreis(pr - 0.08, 10, cx, cz), 0.35), STEIN_HELL, m=m)
    bau.teil(drehkoerper([(0.0, 0.0), (innen * 0.55, 0.02), (innen * 0.5, 0.3), (0.0, 0.45)], 8), GOLD, m=m @ M((0, 0, 0), 0, 90))


def portal(bau, m, breite=3.0, hoehe=5.6, offen=False, wand=1.0):
    """Hauptportal: drei zurückspringende Spitzbögen mit Säulchen, zweiflügliges Holztor mit
    Eisenbändern, Wimperg mit Krabben und Rundfenster darüber."""
    stufen = 3
    for i in range(stufen):
        extra = (stufen - 1 - i) * 0.45
        hw = breite / 2 + extra
        r = hw * 2.0
        hs = hoehe + extra * 0.4 - bogen_hoehe(hw, r)
        y0 = -(stufen - 1 - i) * 0.4
        bau.teil(rahmen(bogen(hw + 0.4, hs, r + 0.4), bogen(hw, hs, r), 0.5, y0), STEIN_HELL if i % 2 == 0 else STEIN, m=m)
        for s in (-1, 1):
            bau.teil(zylinder(0.13, hs, 8), STEIN_HELL, m=m @ M((s * (hw + 0.05), y0 - 0.25, 0)))
            bau.teil(quader(0.4, 0.4, 0.25, 0.04), STEIN_HELL, m=m @ M((s * (hw + 0.05), y0 - 0.25, hs - 0.25)))
    hw = breite / 2
    r = hw * 2.0
    hs = hoehe - bogen_hoehe(hw, r)
    if offen:
        # Zwei Torflügel, nach innen an die Laibung geschwenkt
        for s in (-1, 1):
            fluegel = m @ M((s * (hw - 0.08), wand + 0.8, 0))
            bau.teil(quader(0.14, 1.5, hs + 1.6, 0.02), HOLZ, m=fluegel)
            for z in (0.8, hs * 0.55, hs + 1.1):
                bau.teil(quader(0.2, 1.4, 0.12), EISEN, m=fluegel @ M((0, 0, z)))
            bau.teil(rahmen(kreis(0.16, 8, 0, hs * 0.5), kreis(0.11, 8, 0, hs * 0.5), 0.04), EISEN, m=fluegel @ M((-s * 0.08, 0.3, 0), 90 * s))
    else:
        portal_tor(bau, m, breite, hw, r, hs)
    # Wimperg über dem Portal
    wimperg_portal(bau, m, breite, hoehe)


def portal_tor(bau, m, breite, hw, r, hs):
    """Geschlossenes Holztor mit Bretterfugen, Eisenbändern und Ringen."""
    bau.teil(platte(bogen(hw, hs, r), 0.1, 0.05), HOLZ, m=m)
    for i in range(1, 6):
        x = -hw + breite * i / 6
        bau.teil(quader(0.04, 0.03, hs * 0.98), "#4A2E18", m=m @ M((x, -0.06, 0.05)))
    for z in (0.9, hs * 0.55, hs - 0.2):
        for s in (-1, 1):
            bau.teil(quader(hw - 0.2, 0.04, 0.12), EISEN, m=m @ M((s * (hw / 2 + 0.02), -0.08, z)))
    bau.teil(quader(0.06, 0.05, hs + bogen_hoehe(hw, r) - 0.1), "#2A1A10", m=m @ M((0, -0.07, 0.05)))
    for s in (-1, 1):
        bau.teil(rahmen(kreis(0.18, 8, s * 0.3, hs * 0.5), kreis(0.13, 8, s * 0.3, hs * 0.5), 0.04, -0.1), EISEN, m=m)


def wimperg_portal(bau, m, breite, hoehe):
    """Wimperg (Ziergiebel) mit Krabben, Rundfenster und Fialen über dem Portal."""
    aussen_hw = breite / 2 + 1.3
    wz = hoehe + 0.2
    spitze = wz + aussen_hw * 1.9
    bau.teil(platte([(-aussen_hw, wz), (aussen_hw, wz), (0, spitze)], 0.35, -0.6), STEIN, MAUER, m=m)
    mitte = (wz + spitze) / 2 - 0.1
    bau.teil(platte(kreis(0.55, 12, 0, mitte), 0.05, -0.95), GLAS_FARBE, GLAS, m=m, leuchten=True)
    bau.teil(rahmen(kreis(0.75, 12, 0, mitte), kreis(0.55, 12, 0, mitte), 0.2, -0.9), STEIN_HELL, m=m)
    laenge = math.hypot(aussen_hw, spitze - wz)
    w = math.degrees(math.atan2(spitze - wz, aussen_hw))
    for s in (-1, 1):
        bau.teil(quader(laenge + 0.2, 0.55, 0.2, 0.03), STEIN_HELL,
                 m=m @ M((s * aussen_hw / 2, -0.78, (wz + spitze) / 2), 0, 0, (w if s > 0 else -w)) @ M((0, 0, -0.05)))
        for i in range(1, 7):
            t = i / 7
            x, z = s * aussen_hw * (1 - t), wz + (spitze - wz) * t
            bau.teil(pyramide(0.12, 0.4, 3), DACH_DUNKEL, m=m @ M((x, -0.78, z + 0.18)))
        fiale(bau, m @ M((s * (aussen_hw + 0.35), -0.9, 0)), 0.7, wz + 2.2, schaft=0.62)
    _kreuzblume(bau, m @ M((0, -0.78, spitze + 0.05)), 0.35)


# ---------------------------------------------------------------------------
# Türme
# ---------------------------------------------------------------------------
def turmhelm(bau, m, r, hoehe, ecken=8, gauben=4):
    """Achteckiger grüner Turmhelm mit Gauben, Zierbändern, Krabben und goldener Spitze."""
    dreh = math.pi / ecken
    bau.teil(zylinder(r + 0.35, 0.35, ecken, drehung=dreh), DACH_DUNKEL, m=m)
    bau.teil(pyramide(r, hoehe, ecken, dreh), DACH, ZIEGEL, m=m @ M((0, 0, 0.3)))
    for t in (0.35, 0.62):
        rr = r * (1 - t)
        bau.teil(zylinder(rr + 0.12, 0.25, ecken, rr + 0.1 - 0.25 * r / hoehe, dreh), DACH_DUNKEL, m=m @ M((0, 0, 0.3 + hoehe * t - 0.1)))
    for k in range(ecken):
        w = math.tau * k / ecken + dreh
        for i in range(1, 7):
            t = i / 7.5
            x, y = math.cos(w) * r * (1 - t), math.sin(w) * r * (1 - t)
            bau.teil(pyramide(0.13 * (1.2 - t * 0.6), 0.36, 3), DACH_DUNKEL, m=m @ M((x, y, 0.3 + hoehe * t), math.degrees(w)) @ M((0.08, 0, 0), 0, 0, 70))
    for k in range(gauben):
        rr = r * 0.62
        gaube(bau, m @ M((0, 0, hoehe * 0.14), 360 * k / gauben) @ M((0, -rr, 0)), 1.4, 1.9)
    spitze = m @ M((0, 0, hoehe + 0.25))
    bau.teil(drehkoerper([(0.25, 0), (0.3, 0.2), (0.12, 0.45), (0.2, 0.7), (0.0, 0.95)], 8), GOLD, m=spitze)
    kreuz(bau, spitze @ M((0, 0, 0.9)), 2.2)


def zinnen(bau, m, laenge, hoehe=1.3, dicke=0.6):
    """Brüstung mit Zinnen entlang +X ab 0."""
    bau.teil(quader(laenge, dicke, hoehe * 0.55), STEIN, MAUER, m=m @ M((laenge / 2, 0, 0)))
    anzahl = max(int(laenge / 1.3), 2)
    for i in range(anzahl):
        x = (i + 0.5) * laenge / anzahl
        bau.teil(quader(laenge / anzahl * 0.55, dicke, hoehe * 0.45), STEIN, MAUER, m=m @ M((x, 0, hoehe * 0.55)))
        bau.teil(quader(laenge / anzahl * 0.55 + 0.1, dicke + 0.1, 0.1, 0.02), STEIN_HELL, m=m @ M((x, 0, hoehe)))


def eckturm(bau, ort, breite, hoehe, helm):
    """Quadratischer Turm mit Eckstreben, Fenstern auf drei Ebenen, Wehrgang mit Zinnen, vier
    Eckfialen und hohem achteckigem Helm."""
    m = M(ort)
    bau.teil(quader(breite + 0.6, breite + 0.6, 1.4, 0.05), SOCKEL, MAUER, m=m)
    bau.teil(quader(breite, breite, hoehe), STEIN, MAUER, m=m)
    hb = breite / 2
    for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1)):
        for h, d in ((hoehe * 0.45, 1.3), (hoehe * 0.8, 0.9)):
            bau.teil(quader(d, d, h), STEIN, MAUER, m=m @ M((sx * (hb + d * 0.3 - 0.25), sy * (hb + d * 0.3 - 0.25), 0)))
            bau.teil(pyramide(d * 0.72, d * 0.9), DACH, ZIEGEL, m=m @ M((sx * (hb + d * 0.3 - 0.25), sy * (hb + d * 0.3 - 0.25), h)))
    for k in range(4):
        seite = m @ M((0, 0, 0), 90 * k) @ M((0, -hb, 0))
        for z in (hoehe * 0.33, hoehe * 0.66):
            gesims(bau, seite @ M((-hb - 0.3, 0, z)), breite + 0.6, 0.35, 0.3)
        fenster(bau, seite @ M((0, 0, 3.0)), 1.1, 3.0, masswerk=False)
        for s in (-1, 1):
            fenster(bau, seite @ M((s * 1.1, 0, hoehe * 0.38)), 1.2, 4.2, masswerk=False)
        fenster(bau, seite @ M((0, 0, hoehe * 0.7)), 2.2, 5.5)
        bogenfries(bau, seite @ M((-hb, 0, hoehe - 0.1)), breite, 0.8)
        gesims(bau, seite @ M((-hb - 0.5, 0, hoehe - 0.1)), breite + 1.0, 0.5, 0.4)
        zinnen(bau, seite @ M((-hb - 0.2, -0.2, hoehe + 0.3)), breite + 0.4)
    for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1)):
        fiale(bau, m @ M((sx * (hb - 0.1), sy * (hb - 0.1), hoehe + 0.3)), 0.9, 6.5)
    turmhelm(bau, m @ M((0, 0, hoehe + 0.3)), hb * 0.78, helm)


def achteckturm(bau, ort, radius, hoehe, helm, galerie=True):
    """Achteckiger Turm mit Fenstern, Galerie mit Balustrade, acht Fialen und hohem Helm."""
    m = M(ort)
    dreh = math.pi / 8
    bau.teil(zylinder(radius + 0.5, 1.4, 8, drehung=dreh), SOCKEL, MAUER, m=m)
    bau.teil(zylinder(radius, hoehe, 8, drehung=dreh), STEIN, MAUER, m=m)
    for z in (hoehe * 0.3, hoehe * 0.62):
        bau.teil(zylinder(radius + 0.25, 0.3, 8, drehung=dreh), STEIN_HELL, m=m @ M((0, 0, z)))
    seite = radius * math.cos(math.pi / 8)
    halb = seite * math.tan(math.pi / 8)
    for k in range(8):
        f = m @ M((0, 0, 0), 45 * k) @ M((0, -seite, 0))
        if k % 2 == 0:
            fenster(bau, f @ M((0, 0, hoehe * 0.36)), min(1.3, halb * 1.2), 4.0, masswerk=False)
            fenster(bau, f @ M((0, 0, hoehe * 0.68)), min(1.8, halb * 1.5), 5.2)
        else:
            fenster(bau, f @ M((0, 0, hoehe * 0.12)), min(0.8, halb), 2.0, masswerk=False)
        # Strebe an jeder Ecke
        ecke = m @ M((0, 0, 0), 45 * k + 22.5) @ M((0, -radius, 0))
        bau.teil(quader(0.7, 0.9, hoehe * 0.9), STEIN, MAUER, m=ecke)
        bau.teil(pyramide(0.55, 0.9), DACH, ZIEGEL, m=ecke @ M((0, 0, hoehe * 0.9)))
        bogenfries(bau, f @ M((-halb, 0, hoehe - 0.1)), 2 * halb, 0.8)
    if galerie:
        bau.teil(zylinder(radius + 1.2, 0.45, 8, drehung=dreh), STEIN_HELL, m=m @ M((0, 0, hoehe)))
        ra = (radius + 1.0) / math.cos(math.pi / 8) * math.cos(math.pi / 8)
        for k in range(8):
            w1, w2 = math.tau * k / 8 + dreh, math.tau * (k + 1) / 8 + dreh
            p1 = Vector((math.cos(w1) * ra, math.sin(w1) * ra, 0))
            p2 = Vector((math.cos(w2) * ra, math.sin(w2) * ra, 0))
            richtung = p2 - p1
            balustrade(bau, m @ M((p1.x, p1.y, hoehe + 0.45), math.degrees(math.atan2(richtung.y, richtung.x))), richtung.length, 10.0)
            fiale(bau, m @ M((math.cos(w1) * (radius - 0.3), math.sin(w1) * (radius - 0.3), hoehe + 0.45)), 0.75, 5.0)
        aufsatz = 3.5
        bau.teil(zylinder(radius * 0.75, aufsatz, 8, drehung=dreh), STEIN, MAUER, m=m @ M((0, 0, hoehe + 0.45)))
        for k in range(1, 8, 2):
            f = m @ M((0, 0, hoehe + 0.45), 45 * k) @ M((0, -radius * 0.75 * math.cos(math.pi / 8), 0.6))
            fenster(bau, f, 0.9, 2.2, masswerk=False, tiefe=0.25, rand=0.14)
        turmhelm(bau, m @ M((0, 0, hoehe + 0.45 + aufsatz)), radius * 0.82, helm, 8, 4)
    else:
        gesims(bau, m @ M((0, 0, hoehe - 0.1)), 0.01)
        turmhelm(bau, m @ M((0, 0, hoehe)), radius * 1.05, helm, 8, 0)


# ---------------------------------------------------------------------------
# Bewuchs und Figuren
# ---------------------------------------------------------------------------
EFEU = ("#3F7A2E", "#4E8A34", "#5E9E3A", "#2F6B2F", "#6FA844")


def _blatt(groesse):
    """Flaches Blatt als niedrige Raute, die nach -Y aus der Wand zeigt."""
    s = groesse
    punkte = [(0, -s * 0.35, 0), (s, 0, 0), (0, 0, s * 1.25), (-s, 0, 0), (0, 0, -s * 0.8)]
    flaechen = [[0, 2, 1], [0, 3, 2], [0, 4, 3], [0, 1, 4], [1, 2, 3, 4]]
    return punkte, flaechen


def efeu(bau, m, breite, hoehe, seed, dichte=1.0, farben=None):
    """Efeu an einer Wand (lokal: Wand bei y = 0, außen = -Y, von z = 0 nach oben): mehrere
    Ranken wachsen verzweigt nach oben, dicht mit Blättern in verschiedenen Grüntönen besetzt."""
    zufall = random.Random(seed)
    ranken = max(int(breite * 2.2 * dichte), 1)
    for _ in range(ranken):
        x = zufall.uniform(-breite / 2, breite / 2)
        zz = zufall.uniform(0.0, 0.4)
        ende = hoehe * zufall.uniform(0.45, 1.0)
        while zz < ende:
            dicke = 1.0 - zz / (ende + 0.01) * 0.5
            for _ in range(4):
                lx = x + zufall.uniform(-0.5, 0.5) * dicke
                lz = max(zz + zufall.uniform(-0.22, 0.22), 0.3)
                g = zufall.uniform(0.18, 0.3) * (0.7 + dicke * 0.3)
                bau.teil(_blatt(g), zufall.choice(farben or EFEU), m=m @ M((lx, -zufall.uniform(0.02, 0.14), lz), 0, 0, zufall.uniform(-40, 40)),
                         schwankung=0.08)
            x += zufall.uniform(-0.28, 0.28)
            x = max(-breite / 2, min(breite / 2, x))
            zz += zufall.uniform(0.22, 0.36)


def wasserspeier(bau, m, g=1.0):
    """Wasserspeier: geflügeltes Fabelwesen, das aus der Wand nach -Y ragt (Fuß bei z = 0)."""
    dunkel, hell = "#8E8274", "#A89A88"
    bau.teil(quader(0.42 * g, 0.5 * g, 0.3 * g, 0.03), hell, m=m @ M((0, -0.25 * g, 0)))
    bau.teil(quader(0.34 * g, 0.9 * g, 0.34 * g, 0.05), dunkel, m=m @ M((0, -0.72 * g, 0.12 * g), 0, -8))
    bau.teil(quader(0.32 * g, 0.38 * g, 0.32 * g, 0.05), dunkel, m=m @ M((0, -1.28 * g, 0.2 * g), 0, -14))
    # Maul (oben und unten) und Hörner
    bau.teil(pyramide(0.13 * g, 0.4 * g, 4), dunkel, m=m @ M((0, -1.42 * g, 0.36 * g), 0, 100))
    bau.teil(pyramide(0.1 * g, 0.3 * g, 4), dunkel, m=m @ M((0, -1.42 * g, 0.2 * g), 0, 80))
    for s in (-1, 1):
        bau.teil(pyramide(0.05 * g, 0.28 * g, 3), hell, m=m @ M((s * 0.1 * g, -1.18 * g, 0.48 * g), 0, -30, s * 20))
        # Flügel: flache Dreiecke, schräg nach hinten oben
        fluegel = platte([(0, 0), (0.75 * g, 0.55 * g), (0.55 * g, -0.05 * g)], 0.05)
        bau.teil(fluegel, dunkel, m=m @ M((s * 0.16 * g, -0.72 * g, 0.36 * g), 90 - s * 90, 0, 0) @ M((0, 0, 0), s * 25, 70))
        # Vorderbeine
        bau.teil(quader(0.1 * g, 0.12 * g, 0.28 * g), dunkel, m=m @ M((s * 0.13 * g, -1.08 * g, -0.12 * g)))


# ---------------------------------------------------------------------------
# Innenraum des Schlosses
# ---------------------------------------------------------------------------
WAND = 1.0          # Dicke der Außenwände
BODEN_INNEN = "#CFC3B0"
TEPPICH = "#8E1F24"
HOLZ_DUNKEL = "#4A3322"
KERZE = "#F4ECD8"
FLAMME = "#FFB347"


def saeule(bau, m, hoehe, r=0.5):
    """Runde Säule mit Basis und Kapitell (Fuß bei z = 0)."""
    profil = [(r + 0.3, 0.0), (r + 0.3, 0.35), (r + 0.1, 0.5), (r, 0.6), (r, hoehe - 0.7), (r + 0.15, hoehe - 0.45),
              (r + 0.3, hoehe - 0.2), (r + 0.3, hoehe)]
    bau.teil(drehkoerper(profil, 10), STEIN_HELL, m=m)


def arkade(bau, m, laenge, joch, hs, oben, dicke=WAND):
    """Arkadenwand entlang +X ab 0 (Wand zwischen y = 0 und dicke): Spitzbögen auf Säulen,
    darüber geschlossene Wand bis `oben`."""
    felder = int(round(laenge / joch))
    pfeiler = 1.2
    hw = (joch - pfeiler) / 2
    r = hw * 1.3
    for i in range(felder):
        mitte = (i + 0.5) * joch
        feld = bogen(hw, hs, r, nur_bogen=True) + [(-hw, oben), (hw, oben)]
        bau.teil(platte(feld, dicke, dicke), STEIN, MAUER, m=m @ M((mitte, 0, 0)))
        # Bogenlaibung und Profil auf beiden Seiten
        for y, w in ((0.0, 0), (dicke, 180)):
            bau.teil(rahmen(bogen(hw + 0.25, hs, r + 0.25, nur_bogen=True), bogen(hw, hs, r, nur_bogen=True), 0.15, offen=True), STEIN_HELL,
                     m=m @ M((mitte, y, 0), w))
    for k in range(felder + 1):
        x = k * joch
        bau.teil(quader(pfeiler, dicke, hs), STEIN, MAUER, m=m @ M((x, dicke / 2, 0)))
        for y in (-0.35, dicke + 0.35):
            saeule(bau, m @ M((x, y, 0)), hs, 0.35)


def wandfackel(bau, m):
    """Fackel in einem Eisenhalter (Wand bei y = 0, Raum bei -Y)."""
    bau.teil(quader(0.16, 0.1, 0.35), EISEN, m=m)
    bau.teil(quader(0.07, 0.4, 0.07), EISEN, m=m @ M((0, -0.2, 0.12)))
    bau.teil(zylinder(0.05, 0.65, 6, 0.07), HOLZ, m=m @ M((0, -0.42, -0.05), 0, 15))
    bau.teil(zylinder(0.1, 0.1, 6), EISEN, m=m @ M((0, -0.5, 0.55)))
    bau.teil(pyramide(0.12, 0.42, 5), "#FF9A2E", m=m @ M((0, -0.5, 0.64)), leuchten=True)
    bau.teil(pyramide(0.07, 0.28, 4), "#FFE08A", m=m @ M((0, -0.5, 0.66)), leuchten=True)


def kronleuchter(bau, m, r=1.6, kerzen=10):
    """Radleuchter aus Eisen an drei Ketten mit Kerzen (hängt von `m` aus nach unten)."""
    bau.teil(zylinder(0.04, 6.0, 5), EISEN, m=m @ M((0, 0, -6.0)))
    for k in range(3):
        w = 120 * k
        bau.teil(zylinder(0.025, 2.2, 4), EISEN, m=m @ M((0, 0, -6.0), w) @ M((r * 0.5, 0, -1.9), 0, 0, -38))
    ring = m @ M((0, 0, -8.0))
    bau.teil(rahmen(kreis(r + 0.08, 16), kreis(r - 0.08, 16), 0.12), EISEN, m=ring @ M((0, 0, 0), 0, 90))
    bau.teil(rahmen(kreis(r * 0.45 + 0.06, 10), kreis(r * 0.45 - 0.06, 10), 0.1), EISEN, m=ring @ M((0, 0, 0.3), 0, 90))
    for k in range(kerzen):
        w = math.tau * k / kerzen
        x, y = math.cos(w) * r, math.sin(w) * r
        bau.teil(zylinder(0.09, 0.06, 6), GOLD, m=ring @ M((x, y, 0.06)))
        bau.teil(zylinder(0.05, 0.3, 6), KERZE, m=ring @ M((x, y, 0.12)))
        bau.teil(pyramide(0.05, 0.16, 4), FLAMME, m=ring @ M((x, y, 0.42)), leuchten=True)


def feuerschale(bau, m):
    bau.teil(drehkoerper([(0.3, 0.0), (0.12, 0.1), (0.1, 0.9), (0.2, 1.0), (0.55, 1.15), (0.6, 1.35)], 8), EISEN, m=m)
    for k in range(5):
        w = 72 * k
        bau.teil(pyramide(0.2, 0.7, 5), "#FF9A2E", m=m @ M((0, 0, 1.25), w) @ M((0.18, 0, 0)), leuchten=True)
    bau.teil(pyramide(0.25, 0.9, 5), "#FFE08A", m=m @ M((0, 0, 1.25)), leuchten=True)


def kamin(bau, m, breite=4.0):
    """Großer Kamin an einer Wand (Wand bei y = 0, Raum bei -Y): Wangen, Sims, Rauchfang,
    Holzscheite mit Feuer."""
    hw = breite / 2
    for s in (-1, 1):
        bau.teil(quader(0.6, 1.2, 2.6), STEIN_HELL, MAUER, m=m @ M((s * (hw - 0.3), -0.6, 0)))
    bau.teil(quader(breite + 0.6, 1.5, 0.35, 0.05), STEIN_HELL, m=m @ M((0, -0.7, 2.6)))
    bau.teil(prisma([(0.0, 0.0), (-1.3, 0.0), (-0.7, 2.6), (0.0, 2.6)], breite), STEIN, MAUER, m=m @ M((-hw, 0, 2.95)))
    bau.teil(quader(breite - 1.2, 1.1, 0.25), "#3A3230", m=m @ M((0, -0.6, 0)))
    bau.teil(platte([(-hw + 0.6, 0.25), (hw - 0.6, 0.25), (hw - 0.6, 2.6), (-hw + 0.6, 2.6)], 0.05, -0.05), "#1A1614", m=m)
    for i in range(3):
        bau.teil(zylinder(0.14, 1.6, 7), "#6E4826", m=m @ M((-0.8, -0.55 + i * 0.12, 0.35 + i * 0.12), 20 * (i - 1), 0, 90))
    for k in range(6):
        bau.teil(pyramide(0.22, 0.8 + (k % 3) * 0.25, 5), "#FF9A2E" if k % 2 else "#FFD36B", m=m @ M((-0.7 + k * 0.28, -0.55, 0.45)), leuchten=True)


def thron(bau, m):
    """Thron auf einem Podest (Blick nach -Y)."""
    bau.teil(quader(6.4, 3.4, 0.3, 0.04), STEIN_HELL, PLATTEN, m=m)
    bau.teil(quader(4.6, 2.4, 0.3, 0.04), STEIN_HELL, PLATTEN, m=m @ M((0, 0.4, 0.3)))
    bau.teil(quader(4.0, 2.0, 0.03), TEPPICH, m=m @ M((0, 0.4, 0.6)))
    t = m @ M((0, 0.7, 0.62))
    bau.teil(quader(1.3, 1.0, 0.55, 0.04), GOLD, m=t)
    bau.teil(quader(1.1, 0.85, 0.14, 0.05), TEPPICH, m=t @ M((0, -0.05, 0.55)))
    bau.teil(quader(1.3, 0.2, 2.6, 0.04), GOLD, m=t @ M((0, 0.45, 0.0)))
    bau.teil(quader(0.95, 0.06, 1.8, 0.03), TEPPICH, m=t @ M((0, 0.33, 0.7)))
    bau.teil(platte([(-0.65, 2.6), (0.65, 2.6), (0.0, 3.3)], 0.2, 0.55), GOLD, m=t)
    bau.teil(drehkoerper([(0.0, 0.0), (0.12, 0.05), (0.0, 0.25)], 6), "#B23A3A", m=t @ M((0, 0.45, 3.28)), leuchten=True)
    for s in (-1, 1):
        bau.teil(quader(0.16, 0.95, 0.35, 0.03), GOLD, m=t @ M((s * 0.62, -0.02, 0.55)))
        bau.teil(drehkoerper([(0.0, 0.0), (0.12, 0.05), (0.0, 0.2)], 6), GOLD, m=t @ M((s * 0.62, -0.45, 0.9)))
        bau.teil(zylinder(0.08, 2.9, 6), GOLD, m=t @ M((s * 0.62, 0.45, 0.0)))
        bau.teil(drehkoerper([(0.0, 0.0), (0.13, 0.08), (0.0, 0.26)], 6), GOLD, m=t @ M((s * 0.62, 0.45, 2.9)))


def tafel(bau, m, laenge, zufall):
    """Lange Festtafel entlang +X ab 0 mit Bänken, Tellern, Kelchen, Kerzen, Brot und Obst."""
    bau.teil(quader(laenge, 1.3, 0.1, 0.02), "#8A5A34", m=m @ M((laenge / 2, 0, 0.8)))
    bau.teil(quader(laenge - 0.2, 1.0, 0.02), "#E8DCC0", m=m @ M((laenge / 2, 0, 0.9)))
    for x in [0.5 + i * (laenge - 1.0) / max(int(laenge / 3), 1) for i in range(int(laenge / 3) + 1)]:
        bau.teil(quader(0.14, 1.0, 0.8), HOLZ_DUNKEL, m=m @ M((x, 0, 0)))
    for s in (-1, 1):
        bau.teil(quader(laenge, 0.4, 0.07, 0.02), "#8A5A34", m=m @ M((laenge / 2, s * 1.05, 0.45)))
        for x in [0.4 + i * (laenge - 0.8) / max(int(laenge / 3), 1) for i in range(int(laenge / 3) + 1)]:
            bau.teil(quader(0.1, 0.34, 0.45), HOLZ_DUNKEL, m=m @ M((x, s * 1.05, 0)))
        for i in range(int(laenge / 1.2)):
            x = 0.6 + i * 1.2
            bau.teil(zylinder(0.18, 0.02, 8), "#D8D0C4", m=m @ M((x, s * 0.35, 0.92)))
            bau.teil(drehkoerper([(0.06, 0.0), (0.02, 0.03), (0.02, 0.12), (0.06, 0.16), (0.06, 0.22)], 6), GOLD, m=m @ M((x + 0.3, s * 0.2, 0.92)))
    for i in range(int(laenge / 2.4)):
        x = 1.2 + i * 2.4
        if i % 2 == 0:
            bau.teil(drehkoerper([(0.1, 0.0), (0.05, 0.05), (0.05, 0.06), (0.1, 0.08)], 6), GOLD, m=m @ M((x, 0, 0.92)))
            bau.teil(zylinder(0.04, 0.3, 6), KERZE, m=m @ M((x, 0, 1.0)))
            bau.teil(pyramide(0.04, 0.12, 4), FLAMME, m=m @ M((x, 0, 1.3)), leuchten=True)
        else:
            bau.teil(drehkoerper([(0.0, 0.0), (0.3, 0.02), (0.34, 0.1)], 8), "#B98A56", m=m @ M((x, 0, 0.92)))
            for _ in range(5):
                bau.teil(drehkoerper([(0.0, -0.07), (0.07, 0.0), (0.0, 0.07)], 5), zufall.choice(("#E0524F", "#8FBF3A", "#F2D544")),
                         m=m @ M((x + zufall.uniform(-0.15, 0.15), zufall.uniform(-0.15, 0.15), 1.02)))
            bau.teil(drehkoerper([(0.0, 0.0), (0.12, 0.02), (0.11, 0.08), (0.0, 0.12)], 7), "#C98A42", m=m @ M((x + 0.9, 0.1, 0.92), 0, 0, 0, (1.8, 1.0, 1.0)))


def innenraum(bau, z, hl, wb, wv):
    """Einrichtung des großen Saals: Boden, Teppich zum Thron, Festtafeln, Kronleuchter, Kamin,
    Feuerschalen, Wandfackeln, Banner; Holzbalkendecke über dem Hauptschiff."""
    zufall = random.Random(91)
    innen = SCHIFF - WAND
    bau.teil(quader(LAENGE - 2 * WAND, 2 * innen, 0.05), BODEN_INNEN, PLATTEN, m=M((0, 0, z)))
    bau.teil(quader(2 * wb - 2 * WAND, wv - SCHIFF + WAND, 0.05), BODEN_INNEN, PLATTEN, m=M((0, -(wv + SCHIFF - WAND) / 2 + WAND / 2 - 0.5, z)))
    # Holzdecke mit Balken über dem Hauptschiff
    decke = z + HALLE_H - 0.5
    bau.teil(quader(LAENGE - 2 * WAND, 2 * HALLE, 0.3), "#6E4826", m=M((0, 0, decke)))
    for i in range(int(LAENGE / 3.5) + 1):
        x = -hl + WAND + 0.3 + i * (LAENGE - 2 * WAND - 0.6) / int(LAENGE / 3.5)
        bau.teil(quader(0.4, 2 * HALLE - 2 * WAND, 0.55), HOLZ_DUNKEL, m=M((x, 0, decke - 0.55)))
    bau.teil(quader(LAENGE - 2 * WAND, 0.5, 0.45), HOLZ_DUNKEL, m=M((0, 0, decke - 0.45)))
    # Teppich vom Portal zum Thron
    von, bis = -wv + WAND, innen - 3.2
    bau.teil(quader(2.6, bis - von, 0.03), TEPPICH, m=M((0, (von + bis) / 2, z + 0.05)))
    for s in (-1, 1):
        bau.teil(quader(0.14, bis - von, 0.035), GOLD, m=M((s * 1.2, (von + bis) / 2, z + 0.05)))
    thron(bau, M((0, innen - 1.8, z + 0.05)))
    for s in (-1, 1):
        feuerschale(bau, M((s * 3.6, innen - 2.2, z + 0.05)))
        banner(bau, M((s * 2.7, innen, z + 9.0)), 1.4, 5.5)
    # Festtafeln im Hauptschiff (Mitte bleibt für den Teppich frei)
    for s in (-1, 1):
        for x0 in (-hl + 4.0, 3.5):
            tafel(bau, M((x0, s * 3.4, z + 0.05)), hl - 7.5, zufall)
    # Kronleuchter über den Tafeln
    for x in (-hl / 2, 0.0, hl / 2):
        kronleuchter(bau, M((x, 0, decke - 0.55)))
    # Kamin an der Westwand, gegenüber ein Wandteppich
    kamin(bau, M((-hl + WAND, 0, z + 0.05), 90), 4.4)
    for i in range(5):
        bau.teil(quader(0.06, 1.0, 4.5), STOFFE_WAND[i % len(STOFFE_WAND)], m=M((hl - WAND - 0.05, -2.0 + i * 1.0, z + 3.0)))
    bau.teil(quader(0.08, 5.4, 0.2), GOLD, m=M((hl - WAND - 0.06, 0.5 - 0.5, z + 7.5)))
    # Wandfackeln an den Arkadenpfeilern (zum Hauptschiff hin) und Banner darüber
    for k in range(1, int(LAENGE / JOCH)):
        x = -hl + k * JOCH
        for s in (-1, 1):
            wandfackel(bau, M((x, s * (HALLE - WAND - 0.35), z + 3.6), 0 if s > 0 else 180) @ M((0, 0, 0)))
            if k % 2 == 1:
                banner(bau, M((x, s * (HALLE - WAND), z + 15.0), 0 if s > 0 else 180), 1.3, 4.5)


STOFFE_WAND = ("#7A1E24", "#2E6B46", "#7A1E24", "#C9A227", "#2E6B46")

# ---------------------------------------------------------------------------
# Die Burg
# ---------------------------------------------------------------------------
LAENGE = 49.0      # Hauptschiff entlang X (7 Joche)
HALLE = 7.0        # halbe Breite des Hauptschiffs
SCHIFF = 12.0      # Außenwand der Seitenschiffe (|y|)
SEITE_H = 12.0     # Höhe der Seitenschiffe
HALLE_H = 22.0     # Traufhöhe des Hauptschiffs
JOCH = 7.0
SOCKEL_H = 1.0     # Terrasse, auf der alles steht
NEIGUNG = 58.0


def burg(seed=77):
    """Nur die Burg (ohne Mauer und Hof)."""
    bau = Bau(seed)
    burg_teile(bau)
    return bau.fertig("Burg")


def burg_teile(bau):
    """Baut die Burg in `bau`: Terrasse von z = 0 bis 1, Portal nach -Y (Treppe bis y ≈ -22)."""
    z = SOCKEL_H
    hl = LAENGE / 2
    tan = math.tan(math.radians(NEIGUNG))
    # Westwerk: halbe Breite, wie weit es vorspringt, Traufhöhe
    wb, wv, wh = 4.4, SCHIFF + 3.5, 25.0

    # Terrasse aus Bodenplatten mit Treppe vor dem Portal
    bau.teil(quader(LAENGE + 26, 2 * SCHIFF + 14, SOCKEL_H, 0.05), SOCKEL, PLATTEN)
    for i in range(4):
        tiefe = 1.0 + (3 - i) * 0.6
        bau.teil(quader(8.0, tiefe, SOCKEL_H * (i + 1) / 4, 0.02), SOCKEL, PLATTEN, m=M((0, -(SCHIFF + 7) - tiefe / 2 + 0.3, 0)))

    # Hauptschiff und Seitenschiffe: hohle Mauern, innen ein großer Saal mit Arkaden
    def laeufe(s):
        # Abschnitte entlang der Längsseite: vorne ist die Mitte zum Westwerk hin offen
        return ((-hl, -wb), (wb, hl)) if s < 0 else ((-hl, hl),)

    for s in (-1, 1):
        bau.teil(quader(WAND, 2 * SCHIFF, SEITE_H), STEIN, MAUER, m=M((s * (hl - WAND / 2), 0, z)))
        bau.teil(quader(WAND, 2 * HALLE, HALLE_H - SEITE_H + 0.01), STEIN, MAUER, m=M((s * (hl - WAND / 2), 0, z + SEITE_H)))
        for a, b in laeufe(s):
            bau.teil(quader(b - a, WAND, SEITE_H), STEIN, MAUER, m=M(((a + b) / 2, s * (SCHIFF - WAND / 2), z)))
            bau.teil(quader(b - a + (0.3 if abs(a) == hl else 0.0) + (0.3 if abs(b) == hl else 0.0), 0.6, 1.3, 0.04), SOCKEL, MAUER,
                     m=M(((a + b) / 2, s * (SCHIFF + 0.05), z)))
        bau.teil(quader(LAENGE, SCHIFF - HALLE, 0.6), STEIN, m=M((0, s * (HALLE + SCHIFF) / 2, z + SEITE_H - 0.6)))
        bau.teil(quader(LAENGE - 0.2, SCHIFF - HALLE - 0.2, 0.08), SOCKEL, PLATTEN, m=M((0, s * (HALLE + SCHIFF) / 2, z + SEITE_H)))
        arkade(bau, M((-hl, -HALLE if s < 0 else HALLE - WAND, z)), LAENGE, JOCH, 6.5, SEITE_H)
        bau.teil(quader(LAENGE, WAND, HALLE_H - SEITE_H), STEIN, MAUER, m=M((0, s * (HALLE - WAND / 2), z + SEITE_H)))

    # Joche auf beiden Längsseiten
    joch_x = [-hl + JOCH * (i + 0.5) for i in range(int(LAENGE / JOCH))]
    for s in (-1, 1):
        rz = 0 if s < 0 else 180

        def sm(x, y, zz):
            # Auf der Rückseite ist lokales +X Welt -X: dort x spiegeln, damit die Lage stimmt
            return M((x, y, zz), rz)

        def entlang(y, zz):
            # Anfang einer Reihe entlang der ganzen Länge (lokales +X läuft über die Seite)
            return M((-hl if s < 0 else hl, y, zz), rz)

        for a, b in laeufe(s):
            start = a if s < 0 else b
            verl_a = 0.3 if abs(a) == hl else 0.0
            verl_b = 0.3 if abs(b) == hl else 0.0
            ab = M((start, s * SCHIFF, 0), rz)
            gesims(bau, ab @ M((-(verl_a if s < 0 else verl_b), 0, z + SEITE_H - 0.45)), b - a + verl_a + verl_b)
            gesims(bau, ab @ M((0, 0, z + 3.2)), b - a, 0.15, 0.2)
        # Traufgesims knapp unter der Dachkante (sonst stäche es durch die Dachfläche)
        gesims(bau, entlang(s * HALLE, z + HALLE_H - 1.6), LAENGE, 0.45, 0.4)
        bogenfries(bau, entlang(s * HALLE, z + HALLE_H - 1.6), LAENGE)
        for i, x in enumerate(joch_x):
            eingang = s < 0 and abs(x) < JOCH * 0.6
            if not eingang:
                fenster(bau, sm(x, s * SCHIFF, z + 2.4), 2.6, 7.4, innenseite=WAND)
                # Ziergiebel über dem Fenster mit grünem Dach
                g = sm(x, s * SCHIFF, z + SEITE_H - 0.6)
                gh = 2.6
                bau.teil(platte([(-1.8, 0), (1.8, 0), (0, gh)], 0.5, -0.1), STEIN, MAUER, m=g)
                bau.teil(rahmen(kreis(0.55, 12, 0, 0.95), kreis(0.4, 12, 0, 0.95), 0.2, -0.55), STEIN_HELL, m=g)
                bau.teil(platte(kreis(0.4, 12, 0, 0.95), 0.03, -0.56), GLAS_FARBE, GLAS, m=g, leuchten=True)
                satteldach(bau, g @ M((0, -0.7, 0), 90), 3.4, 1.9, 0.0, 54.0, 0.15, 0.2, False)
                _kreuzblume(bau, g @ M((0, -0.7, gh + 0.25)), 0.2)
                # Balustrade zwischen Ziergiebel und Strebepfeiler
                for a in (x - JOCH / 2 + 0.65, x + 1.85):
                    if s < 0 and abs(a + 0.5) < wb + 1.2:
                        continue
                    laenge = JOCH / 2 - 0.65 - 1.85
                    balustrade(bau, sm(a if s < 0 else a + laenge, s * (SCHIFF - 0.3), z + SEITE_H), laenge, 5.0)
            # Obergaden: zwei Lanzetten je Joch
            if not eingang:
                for d in (-1, 1):
                    fenster(bau, sm(x + d * 1.05, s * HALLE, z + 14.2), 1.3, 5.2, masswerk=False, innenseite=WAND)
        # Strebepfeiler mit Fialen und Strebebögen zwischen den Jochen
        for k in range(len(joch_x) + 1):
            x = -hl + k * JOCH
            # vorne in der Mitte steht das Westwerk, links an den Ecken Eck- und Treppenturm
            if (s < 0 and abs(x) < JOCH) or k == 0:
                continue
            strebepfeiler(bau, sm(x, s * SCHIFF, z), 8.5, SEITE_H + 3.5, 8.0)
            wasserspeier(bau, sm(x, s * (SCHIFF + 1.32), z + SEITE_H + 2.4), 0.9)
            if k % 2 == 1:
                banner(bau, sm(x, s * (SCHIFF + 2.4 * 0.55), z + SEITE_H + 2.8), 1.1, 3.6)
            if 0 < k < len(joch_x):
                strebebogen(bau, sm(x, s * HALLE, z), SCHIFF - HALLE + 1.3, SEITE_H + 2.6, 19.0)

    # Dach des Hauptschiffs mit Gauben
    satteldach(bau, M((-hl - 0.4, 0, z + HALLE_H)), LAENGE + 0.8, HALLE, 0.0, NEIGUNG, 0.7, 0.35)
    for s in (-1, 1):
        for x in ((-14.0, 14.0) if s < 0 else (-14.0, 0.0, 14.0)):
            gaube(bau, M((x, s * (HALLE - 2.6), z + HALLE_H + 2.6 * tan - 1.6), 0 if s < 0 else 180), 3.0, 3.4)
    # Giebel an den Enden
    for s in (-1, 1):
        giebel = M((s * hl, 0, z), 90 * s)
        first = HALLE_H + (HALLE + 0.3) * tan - 0.3
        giebelwand(bau, giebel, HALLE + 0.3, HALLE_H, first)
        fenster(bau, giebel @ M((0, -0.4, HALLE_H + 1.0)), 2.2, 6.0)
        fiale(bau, giebel @ M((0, -0.2, first - 0.2)), 0.9, 5.0)

    # Westwerk mit Portal und Fensterrose (vorne, Mitte)
    # Westwerk: hohe Eingangshalle hinter dem Portal, darüber (über dem Seitenschiff) geschlossen
    for sx in (-1, 1):
        bau.teil(quader(WAND, wv - SCHIFF + WAND, wh), STEIN, MAUER, m=M((sx * (wb - WAND / 2), -(wv + SCHIFF - WAND) / 2, z)))
        bau.teil(quader(WAND, SCHIFF - WAND - HALLE, wh - SEITE_H), STEIN, MAUER, m=M((sx * (wb - WAND / 2), -(SCHIFF - WAND + HALLE) / 2, z + SEITE_H)))
    bau.teil(quader(2 * wb, WAND, wh - SEITE_H + 0.6), STEIN, MAUER, m=M((0, -(SCHIFF - WAND / 2), z + SEITE_H - 0.6)))
    bau.teil(quader(2 * wb, wv - HALLE, 0.8), STEIN, m=M((0, -(wv + HALLE) / 2, z + wh - 0.8)))
    tor_hw, tor_h = 1.6, 6.2
    tor_hs = tor_h - bogen_hoehe(tor_hw, 2 * tor_hw)
    for sx in (-1, 1):
        bau.teil(quader(wb - tor_hw, WAND, wh), STEIN, MAUER, m=M((sx * (wb + tor_hw) / 2, -wv + WAND / 2, z)))
    bau.teil(platte(bogen(tor_hw, tor_hs, 2 * tor_hw, nur_bogen=True) + [(-tor_hw, wh), (tor_hw, wh)], WAND, -wv + WAND), STEIN, MAUER, m=M((0, 0, z)))
    vorne = M((0, -wv, z))
    gesims(bau, vorne @ M((-wb - 0.3, 0, wh - 0.4)), 2 * wb + 0.6, 0.45, 0.4)
    bogenfries(bau, vorne @ M((-wb, 0, wh - 0.4)), 2 * wb)
    gesims(bau, vorne @ M((-wb - 0.3, 0, 11.4)), 2 * wb + 0.6, 0.35, 0.3)
    portal(bau, vorne, 2 * tor_hw, tor_h, offen=True, wand=WAND)
    fensterrose(bau, vorne @ M((0, 0, 17.6)), 3.2, innenseite=WAND)
    for sx in (-1, 1):
        # Seitenwände des Westwerks: je ein Fenster
        seite = M((sx * wb, -(wv + SCHIFF) / 2, z), 90 * sx)
        fenster(bau, seite @ M((0, 0, 14.0)), 1.4, 5.0, masswerk=False, innenseite=WAND)
    # Eckpfeiler des Westwerks mit hohen Fialen
    for sx in (-1, 1):
        eck = M((sx * (wb + 0.2), -wv - 0.2, z))
        bau.teil(quader(1.8, 1.8, wh + 0.6), STEIN, MAUER, m=eck)
        for zz in (8.0, 16.0):
            bau.teil(pyramide(1.35, 1.2), DACH, ZIEGEL, m=eck @ M((0, 0, zz)))
            bau.teil(quader(2.0, 2.0, 0.25, 0.04), STEIN_HELL, m=eck @ M((0, 0, zz - 0.2)))
        fiale(bau, eck @ M((0, 0, wh + 0.6)), 1.5, 10.0)
    # Dach und Giebel des Westwerks
    satteldach(bau, M((0, -wv - 0.6, z + wh), 90), wv - 3.0, wb, 0.0, NEIGUNG, 0.5, 0.32)
    wfirst = wh + (wb + 0.2) * tan - 0.2
    giebelwand(bau, vorne @ M((0, 0.1, 0)), wb + 0.2, wh, wfirst)
    kreuz(bau, vorne @ M((0, -0.3, wfirst - 0.1)), 2.6)
    fenster(bau, vorne @ M((0, -0.35, wh + 0.8)), 1.4, 4.0, masswerk=False)

    innenraum(bau, z, hl, wb, wv)

    # Türme: links ein hoher Eckturm, rechts ein achteckiger Turm, hinten ein Treppenturm
    eckturm(bau, (-hl - 3.5, -HALLE - 2.5, z), 8.0, 34.0, 20.0)
    achteckturm(bau, (hl + 3.8, -HALLE + 1.0, z), 5.0, 28.0, 18.0)
    achteckturm(bau, (-hl - 0.4, SCHIFF + 0.6, z), 2.6, 21.0, 8.0, galerie=False)

    # Efeu: an ausgewählten Jochen, am Westwerk bis zur Rose, an den Turmfüßen
    for i, x in enumerate(joch_x):
        for s in (-1, 1):
            if (i * 3 + (s > 0)) % 4 == 0 and not (s < 0 and abs(x) < JOCH * 0.6):
                efeu(bau, M((x - 1.9, s * SCHIFF, z), 0 if s < 0 else 180), 1.4, 7.5, 100 + i * 2 + (s > 0))
                efeu(bau, M((x + 1.9, s * SCHIFF, z), 0 if s < 0 else 180), 1.0, 4.5, 200 + i * 2 + (s > 0))
    for sx in (-1, 1):
        efeu(bau, M((sx * 3.1, -wv, z)), 1.8, 14.5 if sx < 0 else 9.0, 300 + (sx > 0))
    efeu(bau, M((-1.2, -wv - 0.02, z + 13.0)), 2.2, 2.5, 305, 0.8)
    efeu(bau, M((-hl - 3.5 + 1.6, -HALLE - 6.5, z)), 3.0, 11.0, 310)
    efeu(bau, M((hl + 3.8, -HALLE + 1.0 - 4.6, z)), 2.4, 7.0, 311)
    # Wasserspeier an den Türmen
    for k in range(4):
        w = 90 * k + 45
        wasserspeier(bau, M((-hl - 3.5, -HALLE - 2.5, z), w) @ M((0, -5.3, 34.0 - 1.0)), 1.1)

    return bau
