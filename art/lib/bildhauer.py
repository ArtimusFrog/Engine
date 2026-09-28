"""Bildhauer: organische Körper wie modelliert statt aus Einzelteilen zusammengesteckt.

Ablauf für ein Körperteil (\`modellieren\`):
1. Grobform aus weichen Ellipsoiden (Metaballs) – Rumpf, Hals, Schultern, Arme, Beine verschmelzen
   zu *einer* geschlossenen Haut, Muskeln sind eigene Ellipsoide, die sich einfügen.
2. Gleichmäßig neu vernetzen (Voxel-Remesh), glätten.
3. Oberflächendetail ins Mesh prägen (\`versatz(p, n)\` → Meter entlang der Normale): Rindenfurchen,
   Schuppen, Adern, Fellsträhnen.
4. Auf ein Dreiecksbudget vereinfachen, weich schattieren.
5. Farben je Eckpunkt (fließende Übergänge statt Facetten) aus \`farbe(p, n, h)\`; \`h\` ist die
   Höhlung (> 0 in Vertiefungen, < 0 auf Kanten) – für Schmutz in Falten und helle Kanten.
Gewichte: automatisch aus dem Skelett (Wärmeverteilung wie in Blender „mit automatischen
Gewichten“) oder vom Körper auf Kleidung übertragen.
"""

import math

import bmesh
import bpy
from mathutils import Vector, noise
from mathutils.kdtree import KDTree

from figuren import _SICHTBAR
from werkstatt import skelett


# ---------------------------------------------------------------------------
# Grobform
# ---------------------------------------------------------------------------
def glied(a, b, ra, rb, muskel=0.0, lage=0.4, dichte=0.55):
    """Ellipsoid-Kette von a nach b (Radius ra → rb); \`muskel\` wölbt bei \`lage\` (0..1) auf."""
    a, b = Vector(a), Vector(b)
    laenge = (b - a).length
    n = max(2, int(laenge / (min(ra, rb) * dichte)) + 1)
    formen = []
    for i in range(n + 1):
        t = i / n
        r = ra + (rb - ra) * t
        r *= 1.0 + muskel * math.exp(-((t - lage) / 0.28) ** 2)
        formen.append((a.lerp(b, t), (r, r, r)))
    return formen


def ball_mesh(name, formen, aufloesung):
    """Metaballs → Mesh-Objekt. formen: (Mitte, Halbachsen[, negativ[, Drehung]])."""
    kugeln = bpy.data.metaballs.new(name + "Form")
    kugeln.resolution = aufloesung
    kugeln.render_resolution = aufloesung
    kugeln.threshold = 0.6
    for form in formen:
        mitte, (hx, hy, hz) = form[0], form[1]
        e = kugeln.elements.new(type="ELLIPSOID")
        e.co = Vector(mitte)
        e.radius = 1.0
        e.size_x, e.size_y, e.size_z = hx / _SICHTBAR, hy / _SICHTBAR, hz / _SICHTBAR
        e.stiffness = 2.0
        if len(form) > 2 and form[2]:
            e.use_negative = True
        if len(form) > 3 and form[3] is not None:
            e.rotation = form[3]
    obj_form = bpy.data.objects.new(name + "Form", kugeln)
    bpy.context.scene.collection.objects.link(obj_form)
    bpy.context.view_layer.update()
    _aktiv(obj_form)
    bpy.ops.object.convert(target="MESH")
    obj = bpy.context.active_object
    obj.name = name
    obj.data.name = name
    return obj


def _aktiv(obj):
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj


def _modifikator(obj, art, **werte):
    _aktiv(obj)
    mod = obj.modifiers.new(art, art)
    for k, v in werte.items():
        setattr(mod, k, v)
    bpy.ops.object.modifier_apply(modifier=mod.name)


def modellieren(obj, voxel, ziel, versatz=None, glaetten=4, nach_glaetten=1):
    """Neu vernetzen, glätten, Detail prägen, vereinfachen, weich schattieren."""
    _aktiv(obj)
    obj.data.remesh_voxel_size = voxel
    obj.data.remesh_voxel_adaptivity = 0.0
    bpy.ops.object.voxel_remesh()
    _inseln_entfernen(obj)
    if glaetten:
        _modifikator(obj, "SMOOTH", factor=0.6, iterations=glaetten)
    if versatz is not None:
        mesh = obj.data
        mesh.update()
        verschub = [(v.co + v.normal * versatz(v.co.copy(), v.normal.copy())) for v in mesh.vertices]
        for v, p in zip(mesh.vertices, verschub):
            v.co = p
        if nach_glaetten:
            _modifikator(obj, "SMOOTH", factor=0.35, iterations=nach_glaetten)
    dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    if dreiecke > ziel:
        _modifikator(obj, "DECIMATE", ratio=ziel / dreiecke, use_collapse_triangulate=True)
    for p in obj.data.polygons:
        p.use_smooth = True
    return obj


def _inseln_entfernen(obj, anteil=0.02):
    """Kleine abgelöste Stücke (z. B. eine Zehe, die nicht mit dem Fuß verschmolzen ist) löschen."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.verts.ensure_lookup_table()
    gesehen = set()
    inseln = []
    for v in bm.verts:
        if v.index in gesehen:
            continue
        stapel, insel = [v], []
        gesehen.add(v.index)
        while stapel:
            w = stapel.pop()
            insel.append(w)
            for e in w.link_edges:
                o = e.other_vert(w)
                if o.index not in gesehen:
                    gesehen.add(o.index)
                    stapel.append(o)
        inseln.append(insel)
    groesste = max(len(i) for i in inseln) if inseln else 0
    weg = [v for i in inseln if len(i) < groesste * anteil for v in i]
    if weg:
        bmesh.ops.delete(bm, geom=weg, context="VERTS")
        bm.to_mesh(obj.data)
        print(f"{obj.name}: {len(inseln) - sum(1 for i in inseln if len(i) >= groesste * anteil)} lose Stücke entfernt")
    bm.free()


# ---------------------------------------------------------------------------
# Farbe
# ---------------------------------------------------------------------------
def hoehlung(obj):
    """Je Eckpunkt: wie tief er zwischen seinen Nachbarn liegt (> 0 Vertiefung, < 0 Kante),
    ungefähr in Einheiten der Kantenlänge."""
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bm.verts.ensure_lookup_table()
    werte = []
    for v in bm.verts:
        if not v.link_edges:
            werte.append(0.0)
            continue
        mitte = Vector()
        laenge = 0.0
        for e in v.link_edges:
            o = e.other_vert(v)
            mitte += o.co
            laenge += (o.co - v.co).length
        mitte /= len(v.link_edges)
        laenge /= len(v.link_edges)
        werte.append((mitte - v.co).dot(v.normal) / max(laenge, 1e-6))
    bm.free()
    # etwas weichzeichnen (Nachbarn mitteln), damit es keine Sprenkel gibt
    return werte


def einfaerben(obj, farbe, gewicht_hoehle=1.0):
    """Farbe je Eckpunkt aus \`farbe(p, n, h)\` (fließend, keine Facetten)."""
    mesh = obj.data
    mesh.update()
    h = hoehlung(obj)
    je_punkt = [farbe(v.co.copy(), v.normal.copy(), h[i] * gewicht_hoehle) for i, v in enumerate(mesh.vertices)]
    attr = mesh.color_attributes.get("Farbe") or mesh.color_attributes.new("Farbe", "FLOAT_COLOR", "CORNER")
    for poly in mesh.polygons:
        for li, vi in zip(poly.loop_indices, poly.vertices):
            c = je_punkt[vi]
            attr.data[li].color = (c.x, c.y, c.z, 1.0)


def rausch(p, skala=1.0, oktaven=3):
    """Weiches Rauschen −1…1."""
    return noise.fractal(Vector(p) * skala, 0.5, 2.0, oktaven, noise_basis="PERLIN_ORIGINAL")


def zellen(p, skala=1.0):
    """Abstand zur nächsten Zellgrenze (Schuppen, Platten): 0 an der Fuge, ~0.5 in der Mitte."""
    d = noise.voronoi(Vector(p) * skala, distance_metric="DISTANCE", exponent=2.5)[0]
    return d[1] - d[0]


def schmutz(c, h, staerke=0.35, glanz=0.15):
    """Vertiefungen dunkler, Kanten heller."""
    return c * (1.0 - staerke * max(0.0, min(1.0, h * 3.0)) + glanz * max(0.0, min(1.0, -h * 3.0)))


# ---------------------------------------------------------------------------
# Gewichte
# ---------------------------------------------------------------------------
def auto_gewichte(obj, knochen, zusammen=None):
    """Knochengewichte automatisch (Wärmeverteilung) aus einem Hilfsskelett; \`zusammen\` legt
    Gruppen zusammen (z. B. {"Hut": "Kopf"})."""
    zusammen = zusammen or {"Hut": "Kopf"}
    hilfe = skelett(obj.name + "Hilfsskelett", knochen)
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    hilfe.select_set(True)
    bpy.context.view_layer.objects.active = hilfe
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    # Gewichte behalten, Skelett wieder weg
    for mod in list(obj.modifiers):
        obj.modifiers.remove(mod)
    matrix = obj.matrix_world.copy()
    obj.parent = None
    obj.matrix_world = matrix
    daten = hilfe.data
    bpy.data.objects.remove(hilfe)
    bpy.data.armatures.remove(daten)
    for von, nach in zusammen.items():
        g = obj.vertex_groups.get(von)
        if g is None:
            continue
        ziel = obj.vertex_groups.get(nach) or obj.vertex_groups.new(name=nach)
        for v in obj.data.vertices:
            for eintrag in v.groups:
                if eintrag.group == g.index and eintrag.weight > 0:
                    ziel.add([v.index], eintrag.weight, "ADD")
        obj.vertex_groups.remove(g)
    _normalisieren(obj)


def _normalisieren(obj):
    namen = {g.index: g for g in obj.vertex_groups}
    for v in obj.data.vertices:
        eintraege = sorted([(e.group, e.weight) for e in v.groups if e.weight > 0.001], key=lambda x: -x[1])[:4]
        summe = sum(w for _, w in eintraege) or 1.0
        for e in list(v.groups):
            namen[e.group].remove([v.index])
        for g, w in eintraege:
            namen[g].add([v.index], w / summe, "REPLACE")
    if all(len(v.groups) == 0 for v in obj.data.vertices[:50]):
        print(f"WARNUNG {obj.name}: keine automatischen Gewichte")


def gewichte_uebertragen(obj, quelle, erlaubt=None, k=5):
    """Jede Ecke übernimmt die gemischten Gewichte der nächsten Stellen der Quelle (Kleidung vom
    Körper). \`erlaubt(knochen)\` filtert Knochen."""
    namen = {g.index: g.name for g in quelle.vertex_groups}
    punkte = []
    for v in quelle.data.vertices:
        g = {namen[e.group]: e.weight for e in v.groups if e.weight > 0.001 and (erlaubt is None or erlaubt(namen[e.group]))}
        if g:
            punkte.append((quelle.matrix_world @ v.co, g))
    baum = KDTree(len(punkte))
    for i, (p, _) in enumerate(punkte):
        baum.insert(p, i)
    baum.balance()
    gruppen = {}
    for v in obj.data.vertices:
        summe = {}
        for _, i, d in baum.find_n(obj.matrix_world @ v.co, k):
            w = 1.0 / (d + 0.005)
            for knochen, g in punkte[i][1].items():
                summe[knochen] = summe.get(knochen, 0.0) + g * w
        beste = sorted(summe.items(), key=lambda kv: -kv[1])[:4]
        gesamt = sum(w for _, w in beste) or 1.0
        for knochen, w in beste:
            if knochen not in gruppen:
                gruppen[knochen] = obj.vertex_groups.get(knochen) or obj.vertex_groups.new(name=knochen)
            gruppen[knochen].add([v.index], w / gesamt, "REPLACE")


# ---------------------------------------------------------------------------
# In die Figur übernehmen
# ---------------------------------------------------------------------------
def aufnehmen(f, obj):
    """Fertiges Teil in die Figur übernehmen (wird in \`Figur.fertig\` mit dem Rest vereint)."""
    f.teile.append(obj)
    return obj


def teil(f, name, formen, voxel, ziel, farbe, gewichte=None, versatz=None, glaetten=4, aufloesung=None, knochen=None, quelle=None,
         erlaubt=None, hoehle=1.0):
    """Alles in einem: Grobform, modellieren, einfärben, Gewichte (Funktion, automatisch aus
    \`knochen\` oder übertragen von \`quelle\`)."""
    obj = ball_mesh(name, formen, aufloesung or voxel * 1.6)
    modellieren(obj, voxel, ziel, versatz, glaetten)
    einfaerben(obj, farbe, hoehle)
    if gewichte is not None:
        f._gewichten(obj, gewichte)
    elif knochen is not None:
        auto_gewichte(obj, knochen)
    elif quelle is not None:
        gewichte_uebertragen(obj, quelle, erlaubt)
    return aufnehmen(f, obj)


def glatt_einfaerben(obj, farbe, hoehle=1.0):
    """Für Lofts und andere Teile: Farbe je Eckpunkt statt je Fläche."""
    for p in obj.data.polygons:
        p.use_smooth = True
    einfaerben(obj, farbe, hoehle)


# ---------------------------------------------------------------------------
# Kleidung an die Haut anpassen
# ---------------------------------------------------------------------------
def umfang(koerper, z, dz=0.02, mitte_x=0.0, nur=None):
    """Querschnitt des Körpers in Höhe z: (Mitte y, halbe Breite, halbe Tiefe). \`nur(p)\` filtert
    (z. B. nur eine Seite für ein Bein)."""
    punkte = [v.co for v in koerper.data.vertices if abs(v.co.z - z) < dz and (nur is None or nur(v.co))]
    if not punkte:
        return 0.0, 0.1, 0.1
    xs = sorted(p.x for p in punkte)
    ys = sorted(p.y for p in punkte)
    q = lambda w, a: w[min(len(w) - 1, int(a * len(w)))]
    return (q(ys, 0.02) + q(ys, 0.98)) / 2, (q(xs, 0.99) - q(xs, 0.01)) / 2, (q(ys, 0.98) - q(ys, 0.02)) / 2


def auf_haut(koerper, punkt, abstand=0.008):
    """Nächster Punkt auf der Haut, etwas nach außen versetzt (Riemen, Nieten, Schnallen)."""
    ok, ort, normale, _ = koerper.closest_point_on_mesh(Vector(punkt))
    if not ok:
        return Vector(punkt)
    return ort + normale * abstand


def huelle(f, koerper, name, auswahl, dicke, farbe, glaetten=2):
    """Eng anliegende Kleidung: der ausgewählte Teil der Haut, um \`dicke\` nach außen versetzt,
    mit Stoffdicke (Hose, Ärmel, Wams). Übernimmt die Gewichte der Haut."""
    import bmesh as _bm
    kopie = koerper.copy()
    kopie.data = koerper.data.copy()
    kopie.name = name
    kopie.data.name = name
    bpy.context.scene.collection.objects.link(kopie)
    bm = _bm.new()
    bm.from_mesh(kopie.data)
    weg = [v for v in bm.verts if not auswahl(v.co, v.normal)]
    _bm.ops.delete(bm, geom=weg, context="VERTS")
    for v in bm.verts:
        v.co += v.normal * dicke
    bm.to_mesh(kopie.data)
    bm.free()
    if glaetten:
        _modifikator(kopie, "SMOOTH", factor=0.5, iterations=glaetten)
    dreiecke = sum(len(p.vertices) - 2 for p in kopie.data.polygons)
    if dreiecke > 1600:
        _modifikator(kopie, "DECIMATE", ratio=1600 / dreiecke)
    _modifikator(kopie, "SOLIDIFY", thickness=dicke * 0.8, offset=-1.0)
    for p in kopie.data.polygons:
        p.use_smooth = True
    einfaerben(kopie, farbe)
    f.teile.append(kopie)
    return kopie
