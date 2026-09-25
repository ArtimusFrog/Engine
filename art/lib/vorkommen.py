"""Abbaubare Vorkommen im Mid-Poly-Stil: Stein- und Erzvorkommen (mit der Spitzhacke abzubauen).

Ein Vorkommen ist eine Gruppe von Felsbrocken, etwa 1,5 m breit und 1 m hoch:
- Jeder Brocken entsteht aus einer fein aufgelösten Kugel, von der mit ebenen Schnitten große
  Bruchflächen abgetrennt werden. Die Kanten bekommen eine schmale Fase – sie fängt das Licht
  und macht den typischen Mid-Poly-Look.
- Farben als Vertexfarben je Fläche: Gesteinsschichten, leichte Schwankungen, hellere
  Kanten, Moos oben (Stein) bzw. Erzadern und Rostflecken (Erz).
- Steinvorkommen: heller Kalkstein mit Moospolstern, angelehnten Platten und Splittern.
- Erzvorkommen: dunkler Basalt, durchzogen von rostroten Erzadern mit metallisch
  glänzenden Stellen, dazu rostrote Erzbrocken am Boden.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

from werkstatt import boden_abflachen, srgb_zu_linear, ursprung_unten, vereinen


def farbe(hex_farbe):
    return Vector(srgb_zu_linear(hex_farbe)[:3])


# ---------------------------------------------------------------------------
# Bausteine
# ---------------------------------------------------------------------------
def _objekt(name, bm, farbe_von):
    """bmesh → Objekt mit Vertexfarben je Fläche (Attribut „Farbe“), flach schattiert."""
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    attr = mesh.color_attributes.new("Farbe", "FLOAT_COLOR", "CORNER")
    fasen = mesh.attributes.get("fase")
    for poly in mesh.polygons:
        c = farbe_von(poly, fasen.data[poly.index].value if fasen else 0)
        for li in poly.loop_indices:
            attr.data[li].color = (c.x, c.y, c.z, 1.0)
        poly.use_smooth = False
    return obj


def _material(obj):
    """Ein Material für alles: Grundfarbe aus den Vertexfarben."""
    mat = bpy.data.materials.get("Vorkommen")
    if mat is None:
        mat = bpy.data.materials.new("Vorkommen")
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
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    for poly in obj.data.polygons:
        poly.material_index = 0


def _brocken_bm(zufall, radius, streckung, schnitte, tiefe=(0.5, 0.85), oben=0.6, fase=0.035):
    """Ein Felsbrocken: Kugel, große ebene Bruchflächen, schmale Fasen an den Kanten."""
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=2, radius=radius)
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((*streckung, 1.0)), verts=bm.verts)
    # Unregelmäßige Grundform: sanfte Beulen, bevor geschnitten wird
    for v in bm.verts:
        n = v.co.normalized()
        beule = math.sin(n.x * 3.1 + zufall.random()) * math.sin(n.y * 2.7 + 1.3) * math.cos(n.z * 2.3)
        v.co += n * beule * radius * 0.08
    for _ in range(schnitte):
        normale = Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-oben, 1))).normalized()
        abstand = radius * zufall.uniform(*tiefe) * (Matrix.Diagonal(streckung) @ normale).length
        geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=normale * abstand, plane_no=normale, clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
    # Fast ebene Nachbarflächen zu großen Facetten zusammenfassen
    bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(7), use_dissolve_boundaries=False, verts=bm.verts, edges=bm.edges)
    # Scharfe Kanten anfasen (fängt das Licht)
    kanten = [e for e in bm.edges if len(e.link_faces) == 2 and e.calc_face_angle(0.0) > math.radians(28)]
    if kanten and fase > 0:
        # Fasenflächen merken (die Markierung bleibt beim Unterteilen erhalten): sie werden heller
        markierung = bm.faces.layers.int.new("fase")
        try:
            ergebnis = bmesh.ops.bevel(bm, geom=kanten, offset=radius * fase, offset_type="OFFSET", segments=1, profile=0.5,
                                       affect="EDGES", clamp_overlap=True)
            for f in ergebnis.get("faces", []):
                f[markierung] = 1
        except (TypeError, ValueError, RuntimeError):
            pass
    _fein(bm, radius * 0.3)
    # winzige Unebenheiten
    for v in bm.verts:
        v.co += Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))) * radius * 0.008
    return bm


def _fein(bm, kante):
    """Große Flächen gleichmäßig in kleinere Dreiecke teilen: die Form bleibt facettiert, aber
    Farben (Adern, Moos, Schichten) können feiner verlaufen als ganze Riesenflächen."""
    bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
    for _ in range(2):
        # Nur wirklich große Flächen teilen – schmale Fasenstreifen bleiben, wie sie sind.
        gross = [f for f in bm.faces if f.calc_area() > kante * kante * 0.8]
        lang = list({e for f in gross for e in f.edges if e.calc_length() > kante * 0.8})
        if not lang:
            break
        bmesh.ops.subdivide_edges(bm, edges=lang, cuts=1, use_grid_fill=True)
        bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
    bmesh.ops.beautify_fill(bm, faces=bm.faces[:], edges=[e for e in bm.edges if len(e.link_faces) == 2 and e.calc_face_angle(1.0) < 0.01])


def _setzen(bm, ort, drehung_z, kippen=(0.0, 0.0)):
    rot = (Matrix.Rotation(drehung_z, 4, "Z") @ Matrix.Rotation(kippen[1], 4, "Y") @ Matrix.Rotation(kippen[0], 4, "X"))
    bmesh.ops.transform(bm, matrix=Matrix.Translation(Vector(ort)) @ rot, verts=bm.verts)
    return bm



def _steinfarbe(zufall, dunkel, mittel, hell, kante, schicht=18.0, moos=None, moos_rauschen=None):
    """Farbe je Fläche: oben hell, unten dunkel, waagerechte Schichten, Fasen heller, Moos oben."""
    def von(poly, fase=False):
        n, z = poly.normal, poly.center.z
        oben = max(0.0, n.z)
        # Seiten mittelhell, nur deutlich nach unten zeigende Flächen dunkler
        c = dunkel.lerp(mittel, min(1.0, max(0.0, 0.8 + n.z * 0.6))).lerp(hell, oben ** 1.5 * 0.75)
        c = c * (1.0 + 0.07 * math.sin(z * schicht + poly.center.x * 2.0)) * zufall.uniform(0.93, 1.06)
        if fase:
            c = c.lerp(kante, 0.5)  # Fasen: schmale, helle Lichtkanten
        if moos is not None and n.z > 0.55 and moos_rauschen(poly.center) > 0.55:
            c = moos * zufall.uniform(0.85, 1.1)
        return c
    return von


def _rauschen(seed):
    """Weiches Rauschen 0..1 im Raum (für Moosflecken, Adern)."""
    zufall = random.Random(seed)
    wellen = [(Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))).normalized() * zufall.uniform(3, 7),
               zufall.uniform(0, math.tau)) for _ in range(4)]

    def wert(p):
        return 0.5 + sum(math.sin(p.dot(k) + phase) for k, phase in wellen) / (2 * len(wellen))
    return wert


def _gruppe(zufall, farbe_von, groesse=1.0, schnitte=(9, 13)):
    """Hauptbrocken und 2–4 kleinere drumherum, leicht gekippt und in den Boden gesunken."""
    teile = []
    anzahl = zufall.randint(3, 5)
    for i in range(anzahl):
        haupt = i == 0
        r = (0.6 if haupt else zufall.uniform(0.27, 0.4)) * groesse
        winkel = math.tau * i / anzahl + zufall.uniform(-0.4, 0.4)
        abstand = 0.0 if haupt else zufall.uniform(0.5, 0.75) * groesse
        streckung = (zufall.uniform(0.95, 1.25), zufall.uniform(0.85, 1.05), zufall.uniform(0.95, 1.3) if haupt else zufall.uniform(0.7, 1.0))
        bm = _brocken_bm(zufall, r, streckung, zufall.randint(*schnitte))
        kippen = (zufall.uniform(-0.25, 0.25), zufall.uniform(-0.25, 0.25))
        _setzen(bm, (math.cos(winkel) * abstand, math.sin(winkel) * abstand, r * (0.5 if haupt else 0.38)), zufall.uniform(0, math.tau), kippen)
        teile.append(_objekt(f"Brocken{i}", bm, farbe_von))
    return teile


def _splitter(zufall, farbe_von, anzahl, weite, groesse=(0.05, 0.12)):
    """Kantige Splitter und Kiesel am Boden rund um das Vorkommen."""
    teile = []
    for i in range(anzahl):
        winkel = zufall.uniform(0, math.tau)
        abstand = zufall.uniform(0.7, weite)
        r = zufall.uniform(*groesse)
        bm = bmesh.new()
        bmesh.ops.create_icosphere(bm, subdivisions=1, radius=r)
        bmesh.ops.transform(bm, matrix=Matrix.Diagonal((zufall.uniform(1.0, 1.6), 1.0, zufall.uniform(0.45, 0.7), 1.0)), verts=bm.verts)
        for _ in range(3):
            n = Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.3, 1))).normalized()
            bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=n * r * 0.6, plane_no=n, clear_outer=True)
            rand = [e for e in bm.edges if e.is_boundary]
            if rand:
                bmesh.ops.holes_fill(bm, edges=rand, sides=0)
        bmesh.ops.triangulate(bm, faces=bm.faces[:])
        _setzen(bm, (math.cos(winkel) * abstand, math.sin(winkel) * abstand, r * 0.2), zufall.uniform(0, math.tau), (zufall.uniform(-0.4, 0.4), 0))
        teile.append(_objekt(f"Splitter{i}", bm, farbe_von))
    return teile


def _abschliessen(name, teile):
    obj = vereinen(name, teile)
    _material(obj)
    boden_abflachen(obj, 0.0)
    ursprung_unten(obj)
    dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"VORKOMMEN {name}: {dreiecke} Dreiecke")
    return obj


# ---------------------------------------------------------------------------
# Vorkommen
# ---------------------------------------------------------------------------
def steinvorkommen(seed):
    zufall = random.Random(seed)
    moos = _rauschen(seed + 100)
    fels = _steinfarbe(zufall, farbe("#6A655E"), farbe("#8F8A80"), farbe("#BDB6A8"), farbe("#DAD3C6"),
                       moos=farbe("#5A6A3C"), moos_rauschen=moos)
    teile = _gruppe(zufall, fels)
    # Ein, zwei flache Platten, schräg angelehnt
    for i in range(zufall.randint(1, 2)):
        r = zufall.uniform(0.28, 0.36)
        bm = _brocken_bm(zufall, r, (1.25, 1.0, 0.32), zufall.randint(6, 8), tiefe=(0.6, 0.9), oben=0.2)
        winkel = zufall.uniform(0, math.tau)
        _setzen(bm, (math.cos(winkel) * 0.55, math.sin(winkel) * 0.55, r * 0.45), winkel + math.pi / 2, (zufall.uniform(0.5, 0.8), 0))
        teile.append(_objekt(f"Platte{i}", bm, fels))
    geroell = _steinfarbe(zufall, farbe("#5E5A54"), farbe("#7E796F"), farbe("#A39C90"), farbe("#C2BBAF"))
    teile += _splitter(zufall, geroell, zufall.randint(8, 12), 1.2)
    return _abschliessen("Steinvorkommen", teile)


def erzvorkommen(seed):
    zufall = random.Random(seed)
    basalt = _steinfarbe(zufall, farbe("#34333C"), farbe("#4C4B57"), farbe("#686774"), farbe("#8A8996"), schicht=24.0)
    # Erzadern: schmale Bänder quer durch alle Brocken (Abstand zu ein paar schrägen Ebenen)
    adern = [(Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.4, 0.4))).normalized(), zufall.uniform(-0.25, 0.4))
             for _ in range(4)]
    rost, rost_hell, rost_dunkel = farbe("#B8662F"), farbe("#D9894A"), farbe("#6E3519")
    metall = farbe("#B9C3D0")

    def fels(poly, fase=False):
        c = basalt(poly, fase)
        p = Vector(poly.center)
        for normale, abstand in adern:
            d = abs(p.dot(normale) - abstand)
            if d < 0.028:
                # Im Kern der Ader blitzt hier und da blankes Eisen
                if zufall.random() < 0.18:
                    return metall * zufall.uniform(0.9, 1.1)
                return rost_hell * zufall.uniform(0.9, 1.1) if poly.normal.z > 0.3 else rost * zufall.uniform(0.9, 1.1)
            if d < 0.06:
                c = c.lerp(rost_dunkel, 0.6)
        return c

    teile = _gruppe(zufall, fels, groesse=0.95, schnitte=(10, 14))
    # Rostflecken und Erzbrocken am Boden
    geroell = _steinfarbe(zufall, farbe("#2A2930"), farbe("#45444E"), farbe("#63626E"), farbe("#7E7D8A"))
    rost_geroell = _steinfarbe(zufall, rost_dunkel, rost, rost_hell, farbe("#F0B07A"), schicht=40.0)
    teile += _splitter(zufall, geroell, zufall.randint(5, 7), 1.15)
    teile += _splitter(zufall, rost_geroell, zufall.randint(4, 6), 1.05, groesse=(0.05, 0.09))
    return _abschliessen("Erzvorkommen", teile)


# ---------------------------------------------------------------------------
# Kristallvorkommen (magisch, leuchtend – noch nicht abbaubar)
# ---------------------------------------------------------------------------
def _kristall_mid(laenge, radius, zufall, seiten=6):
    """Mid-Poly-Kristall entlang +Z (Fuß im Ursprung): leicht verjüngtes, leicht verdrehtes
    Sechseckprisma aus drei Abschnitten, darauf eine Spitze mit abgeschrägten Facetten."""
    bm = bmesh.new()
    drall = zufall.uniform(-0.15, 0.15)
    ringe = []
    for hoehe, breite in ((0.0, 1.0), (0.35, 0.97), (0.68, 0.93), (0.78, 0.8)):
        ring = []
        for k in range(seiten):
            w = math.tau * k / seiten + drall * hoehe
            # leicht unregelmäßiger Querschnitt: nicht jede Seite gleich breit
            r = radius * breite * (1.0 + 0.08 * math.sin(k * 2.3 + laenge * 7))
            ring.append(bm.verts.new((math.cos(w) * r, math.sin(w) * r, laenge * hoehe)))
        ringe.append(ring)
    spitze = bm.verts.new((radius * zufall.uniform(-0.15, 0.15), radius * zufall.uniform(-0.15, 0.15), laenge))
    for a, b in zip(ringe, ringe[1:]):
        for k in range(seiten):
            bm.faces.new((a[k], a[(k + 1) % seiten], b[(k + 1) % seiten], b[k]))
    oben = ringe[-1]
    for k in range(seiten):
        bm.faces.new((oben[k], oben[(k + 1) % seiten], spitze))
    bm.faces.new(list(reversed(ringe[0])))
    return bm


def _kristall_farbe(zufall, fuss, mitte, spitze, laenge):
    """Unten tiefblau, nach oben heller bis fast weiß an der Spitze; Facetten im Wechsel."""
    def von(poly, fase=False):
        h = max(0.0, min(1.0, poly.center.z / max(laenge, 0.01)))
        c = fuss.lerp(mitte, min(1.0, h * 1.4)).lerp(spitze, max(0.0, (h - 0.7) / 0.3))
        return c * (1.12 if poly.index % 2 else 0.92) * zufall.uniform(0.95, 1.05)
    return von


def _kristallbueschel_mid(zufall, basis, achse, groesse, farben):
    """Ein Büschel: großer Hauptkristall, darum 2–5 kleinere, fächerförmig nach außen."""
    teile = []
    quer = achse.orthogonal().normalized()
    anzahl = zufall.randint(3, 6)
    for i in range(anzahl):
        haupt = i == 0
        laenge = groesse * (zufall.uniform(0.95, 1.15) if haupt else zufall.uniform(0.35, 0.7))
        radius = laenge * zufall.uniform(0.13, 0.17)
        bm = _kristall_mid(laenge, radius, zufall)
        if haupt:
            richtung = achse
        else:
            w = math.tau * i / (anzahl - 1) + zufall.uniform(-0.3, 0.3)
            seitlich = quer * math.cos(w) + achse.cross(quer) * math.sin(w)
            richtung = (achse + seitlich * zufall.uniform(0.35, 0.8)).normalized()
        dreh = Vector((0, 0, 1)).rotation_difference(richtung).to_matrix().to_4x4() @ Matrix.Rotation(zufall.uniform(0, math.tau), 4, "Z")
        fuss = basis + (richtung - achse) * radius * 1.5 - richtung * laenge * 0.12
        # Farben in lokalen Koordinaten berechnen (vor dem Drehen), dann an ihren Platz setzen
        obj = _objekt(f"Kristall{i}", bm, _kristall_farbe(zufall, *farben, laenge))
        obj.data.transform(Matrix.Translation(fuss) @ dreh)
        teile.append(obj)
    return teile


def _kristall_material():
    """Leuchtendes Material: Grundfarbe aus den Vertexfarben, dazu Emission (die Engine lässt
    Teile mit Emission nachts glühen)."""
    mat = bpy.data.materials.get("Kristall")
    if mat is None:
        mat = bpy.data.materials.new("Kristall")
        try:
            mat.use_nodes = True
        except (AttributeError, TypeError):
            pass
        knoten, links = mat.node_tree.nodes, mat.node_tree.links
        bsdf = next(k for k in knoten if k.type == "BSDF_PRINCIPLED")
        vc = knoten.new("ShaderNodeVertexColor")
        vc.layer_name = "Farbe"
        links.new(vc.outputs["Color"], bsdf.inputs["Base Color"])
        bsdf.inputs["Roughness"].default_value = 0.2
        if "Emission Color" in bsdf.inputs:
            bsdf.inputs["Emission Color"].default_value = srgb_zu_linear("#7CCBFF")
            bsdf.inputs["Emission Strength"].default_value = 1.5
    return mat


def kristallvorkommen(seed):
    zufall = random.Random(seed)
    # Sockel: dunkler, bläulicher Schiefer mit leuchtend blauen Adern
    schiefer = _steinfarbe(zufall, farbe("#232838"), farbe("#343B50"), farbe("#4C5670"), farbe("#6A7690"), schicht=22.0)
    adern = [(Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.4, 0.4))).normalized(), zufall.uniform(-0.2, 0.3))
             for _ in range(3)]
    ader_farbe, ader_rand = farbe("#4FB4FF"), farbe("#2B5C9E")

    def fels(poly, fase=False):
        c = schiefer(poly, fase)
        for normale, abstand in adern:
            d = abs(Vector(poly.center).dot(normale) - abstand)
            if d < 0.022:
                return ader_farbe * zufall.uniform(0.9, 1.15)
            if d < 0.05:
                c = c.lerp(ader_rand, 0.5)
        return c

    felsen = _gruppe(zufall, fels, groesse=0.75, schnitte=(9, 12))
    felsen += _splitter(zufall, _steinfarbe(zufall, farbe("#262B3A"), farbe("#3A4156"), farbe("#525C76"), farbe("#6A7690")),
                        zufall.randint(5, 8), 1.1)

    # Kristallbüschel: eines groß in der Mitte, dazu ein bis zwei kleinere am Rand
    farben = (farbe("#1D4FB8"), farbe("#3C9CFF"), farbe("#D2F1FF"))
    kristalle = _kristallbueschel_mid(zufall, Vector((0, 0, 0.35)), Vector((zufall.uniform(-0.1, 0.1), zufall.uniform(-0.1, 0.1), 1)).normalized(),
                                      zufall.uniform(1.25, 1.55), farben)
    for i in range(zufall.randint(1, 2)):
        w = zufall.uniform(0, math.tau)
        basis = Vector((math.cos(w) * 0.55, math.sin(w) * 0.55, 0.12))
        achse = (Vector((math.cos(w), math.sin(w), 0)) * 0.6 + Vector((0, 0, 1))).normalized()
        kristalle += _kristallbueschel_mid(zufall, basis, achse, zufall.uniform(0.55, 0.85), farben)
    # Abgebrochene Splitter am Boden
    for i in range(zufall.randint(3, 5)):
        w = zufall.uniform(0, math.tau)
        laenge = zufall.uniform(0.15, 0.28)
        bm = _kristall_mid(laenge, laenge * 0.16, zufall)
        obj = _objekt(f"Splitter{i}", bm, _kristall_farbe(zufall, *farben, laenge))
        liegen = Matrix.Rotation(math.radians(zufall.uniform(70, 85)), 4, "X")
        obj.data.transform(Matrix.Translation((math.cos(w) * zufall.uniform(0.8, 1.15), math.sin(w) * zufall.uniform(0.8, 1.15), 0.03))
                           @ Matrix.Rotation(zufall.uniform(0, math.tau), 4, "Z") @ liegen)
        kristalle.append(obj)

    fels_obj = vereinen("Sockel", felsen)
    _material(fels_obj)
    kristall_obj = vereinen("Kristalle", kristalle)
    kristall_obj.data.materials.clear()
    kristall_obj.data.materials.append(_kristall_material())
    for poly in kristall_obj.data.polygons:
        poly.material_index = 0
    obj = vereinen("Kristallvorkommen", [fels_obj, kristall_obj])
    boden_abflachen(obj, 0.0)
    ursprung_unten(obj)
    dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"VORKOMMEN Kristallvorkommen: {dreiecke} Dreiecke")
    return obj
