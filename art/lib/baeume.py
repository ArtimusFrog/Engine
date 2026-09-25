"""Baum-Generator im hellen, „fluffigen“ Stil.

Kronen, Tannenzweige, Palmwedel und Gras bestehen aus Karten (flachen Flächen) mit einer
gemeinsamen Textur, deren durchsichtige Stellen ausgeschnitten werden. Dazu:
- weiche Normalen: Blattkarten werden beleuchtet, als wären sie Teil einer Kugel,
  dadurch wirkt eine Krone rund und weich statt kantig
- Farbverlauf in den Vertexfarben: innen/unten dunkel, außen/oben sonnig und leicht gelblich
- eine Textur für alles; die Farbe kommt aus den Vertexfarben (Grün, Herbst, Violett …)

Aufbau der Textur (Blender-UV, v zeigt nach oben):
    oben links  LAUB    Blattbüschel
    oben rechts NADEL   Tannenzweig (Stamm links, Spitze rechts)
    unten links WEDEL   Palmwedel (oberes Viertel) und GRAS (unteres Viertel)
    unten rechts BIRKE  Birkenrinde (nahtlos kachelbar), am rechten Rand ein weißer Streifen
                        für Rinde, Stämme, Früchte (Farbe nur aus Vertexfarbe)

Jede Art ist eine Funktion (eiche, birke, tanne, palme, zauberbaum), `seed` ergibt die Variante.
Koordinaten wie in der Werkstatt: Z oben, Ursprung am Stammfuß.
"""

import math
import pathlib
import random

import bmesh
import bpy
import numpy as np
from mathutils import Quaternion, Vector

from werkstatt import material, srgb_zu_linear

TEXTUR_NAME = "Laub_Textur"
TEXTUR_GROESSE = 1024
TEXTUR_DATEI = pathlib.Path(__file__).resolve().parents[1] / "texturen" / f"{TEXTUR_NAME}.png"

# Bereiche im Atlas: (u0, v0, u1, v1)
LAUB = (0.0, 0.5, 0.5, 1.0)
NADEL = (0.5, 0.5, 1.0, 1.0)
WEDEL = (0.0, 0.25, 0.5, 0.5)
GRAS = (0.0, 0.0, 0.5, 0.25)
BIRKE = (0.5, 0.0, 0.9, 0.5)
FLECK = (0.95, 0.25)

OBEN = Vector((0, 0, 1))


def farbe(hex_farbe):
    """'#RRGGBB' → lineares RGB als Vector."""
    return Vector(srgb_zu_linear(hex_farbe)[:3])


# ---------------------------------------------------------------------------
# Textur
# ---------------------------------------------------------------------------
def _blatt(bild, gitter, basis, winkel, laenge, breite, hell, form="blatt", rippe=True):
    """Zeichnet ein spitzes Blatt (oder eine Nadel, einen Grashalm) in das Bild.
    Koordinaten 0..1 über das ganze Bild, `winkel` im Bogenmaß."""
    n = bild.shape[0]
    yg, xg = gitter
    d = np.array([math.cos(winkel), math.sin(winkel)])
    q = np.array([-d[1], d[0]])
    basis = np.asarray(basis, dtype=np.float32)
    spitze = basis + d * laenge
    x0 = int(max(0, (min(basis[0], spitze[0]) - breite) * n))
    x1 = int(min(n, (max(basis[0], spitze[0]) + breite) * n + 2))
    y0 = int(max(0, (min(basis[1], spitze[1]) - breite) * n))
    y1 = int(min(n, (max(basis[1], spitze[1]) + breite) * n + 2))
    if x1 <= x0 or y1 <= y0:
        return
    px = xg[y0:y1, x0:x1] - basis[0]
    py = yg[y0:y1, x0:x1] - basis[1]
    s = (px * d[0] + py * d[1]) / laenge
    t = (px * q[0] + py * q[1]) / breite
    sc = np.clip(s, 0, 1)
    if form == "halm":
        umriss = (1 - sc) ** 0.7
    else:
        umriss = np.sin(np.pi * sc) ** 0.75 * (1 - 0.35 * sc)
    maske = (s > 0) & (s < 1) & (np.abs(t) < umriss * 0.5)
    if not maske.any():
        return
    wert = hell * (0.84 + 0.26 * sc)
    wert = wert * np.where(t < 0, 0.9, 1.0)
    if rippe:
        wert = wert * np.where(np.abs(t) < 0.06, 0.82, 1.0)
    feld = bild[y0:y1, x0:x1]
    feld[maske, 0] = np.clip(wert * 1.01, 0, 1)[maske]
    feld[maske, 1] = np.clip(wert, 0, 1)[maske]
    feld[maske, 2] = np.clip(wert * 0.94, 0, 1)[maske]
    feld[maske, 3] = 1.0


def _strich(bild, gitter, a, b, dicke, hell):
    """Gerader, zum Ende dünner werdender Strich (Zweig, Blattrippe) von a nach b."""
    a, b = np.asarray(a, np.float32), np.asarray(b, np.float32)
    laenge = float(np.linalg.norm(b - a))
    if laenge < 1e-6:
        return
    n = bild.shape[0]
    yg, xg = gitter
    d = (b - a) / laenge
    q = np.array([-d[1], d[0]])
    x0, x1 = int(max(0, (min(a[0], b[0]) - dicke) * n)), int(min(n, (max(a[0], b[0]) + dicke) * n + 2))
    y0, y1 = int(max(0, (min(a[1], b[1]) - dicke) * n)), int(min(n, (max(a[1], b[1]) + dicke) * n + 2))
    px, py = xg[y0:y1, x0:x1] - a[0], yg[y0:y1, x0:x1] - a[1]
    s = (px * d[0] + py * d[1]) / laenge
    t = px * q[0] + py * q[1]
    maske = (s >= 0) & (s <= 1) & (np.abs(t) < dicke * (1 - 0.6 * np.clip(s, 0, 1)))
    feld = bild[y0:y1, x0:x1]
    feld[maske, 0:3] = hell
    feld[maske, 3] = 1.0


def _birkenrinde(rng, w, h):
    """Nahtlos kachelbare Birkenrinde (RGB, Breite = einmal um den Stamm): fast weiß mit
    grauen Längsschlieren, feinen schwarzen, linsenförmigen Querstrichen (Lentizellen) in
    Reihen, kleinen Punkten und ein paar Astnarben."""
    y, x = np.mgrid[0:h, 0:w].astype(np.float32)

    def abstand(mitte, laenge):
        d = np.abs(np.arange(laenge, dtype=np.float32) - mitte)
        return np.minimum(d, laenge - d)  # über den Rand hinweg (nahtlos)

    wert = np.full((h, w), 0.94, np.float32)
    for _ in range(45):  # senkrechte Schlieren
        wert += rng.uniform(-0.07, 0.025) * np.exp(-(abstand(rng.uniform(0, w), w) / rng.uniform(2, 14)) ** 2)[None, :]
    for _ in range(14):  # waagerechte, wolkige Bänder
        wert += rng.uniform(-0.05, 0.02) * np.exp(-(abstand(rng.uniform(0, h), h) / rng.uniform(4, 22)) ** 2)[:, None]
    rgb = np.stack([wert, wert * 0.985, wert * 0.955], -1)

    def zeichne(maske, cx, cy, rx, ry, farbe):
        # Mit Wiederholung über die Ränder, damit die Kachel nahtlos bleibt
        for ox in (-w, 0, w):
            for oy in (-h, 0, h):
                bx0, bx1 = int(max(0, cx + ox - rx - 2)), int(min(w, cx + ox + rx + 3))
                by0, by1 = int(max(0, cy + oy - ry - 2)), int(min(h, cy + oy + ry + 3))
                if bx1 <= bx0 or by1 <= by0:
                    continue
                m = maske(x[by0:by1, bx0:bx1] - (cx + ox), y[by0:by1, bx0:bx1] - (cy + oy))
                rgb[by0:by1, bx0:bx1][m] = farbe

    def linse(rx, ry, neigung):
        # in der Mitte am dicksten, zu den Enden spitz
        def maske(px, py):
            s = px / rx
            return (np.abs(s) < 1) & (np.abs(py - neigung * px) < ry * np.clip(1 - s * s, 0, 1) ** 0.7)
        return maske

    # Große Lentizellen in Reihen (1–3 Striche auf fast gleicher Höhe)
    for _ in range(26):
        cy, cx = rng.uniform(0, h), rng.uniform(0, w)
        for _ in range(rng.integers(1, 4)):
            rx, ry = rng.uniform(0.04, 0.16) * w, rng.uniform(1.3, 3.8)
            dunkel = rng.uniform(0.09, 0.2)
            zeichne(linse(rx, ry, rng.uniform(-0.04, 0.04)), cx, cy + rng.uniform(-2, 2), rx, ry, (dunkel * 1.05, dunkel, dunkel * 0.95))
            cx += rx * 2 + rng.uniform(4, 30)
    # Kleine Striche und Punkte
    for _ in range(170):
        rx, ry = rng.uniform(0.008, 0.04) * w, rng.uniform(0.7, 1.6)
        dunkel = rng.uniform(0.18, 0.42)
        zeichne(linse(rx, ry, 0.0), rng.uniform(0, w), rng.uniform(0, h), rx, ry, (dunkel, dunkel, dunkel * 0.97))
    # Astnarben: dunkle, nach unten offene Winkel („Augen“)
    for _ in range(3):
        breite, dicke = rng.uniform(12, 24), rng.uniform(2.5, 4.5)

        def narbe(px, py, breite=breite, dicke=dicke):
            bogen = -0.6 * np.abs(px) + breite * 0.25
            return (np.abs(px) < breite) & (py < bogen + dicke) & (py > bogen - dicke * (1 - np.abs(px) / breite))

        zeichne(narbe, rng.uniform(0, w), rng.uniform(0, h), breite, breite * 0.6, (0.13, 0.12, 0.11))
    return np.clip(rgb, 0, 1)


def blatt_textur():
    """Erzeugt die gemeinsame Textur (einmal pro Blender-Sitzung) und speichert sie als PNG."""
    if TEXTUR_NAME in bpy.data.images:
        return bpy.data.images[TEXTUR_NAME]
    rng = np.random.default_rng(3)
    n = TEXTUR_GROESSE
    bild = np.zeros((n, n, 4), np.float32)
    gitter = tuple(g.astype(np.float32) / n for g in np.mgrid[0:n, 0:n])

    # Laubbüschel: erst innere (dunklere) Blätter, dann äußere darüber
    mitte = np.array([0.25, 0.75])
    blaetter = 320
    for i in range(blaetter):
        w = rng.uniform(0, math.tau)
        basis = mitte + math.sqrt(rng.uniform(0, 1)) * 0.15 * np.array([math.cos(w), math.sin(w)])
        richtung = w + rng.uniform(-1.3, 1.3)
        laenge = rng.uniform(0.038, 0.06)
        spitze = basis + laenge * np.array([math.cos(richtung), math.sin(richtung)])
        if np.linalg.norm(spitze - mitte) > 0.225:
            continue
        hell = 0.64 + 0.36 * (i / blaetter) * rng.uniform(0.75, 1.0)
        _blatt(bild, gitter, basis, richtung, laenge, laenge * rng.uniform(0.36, 0.46), hell)

    # Tannenzweig: Hauptzweig nach rechts, Seitenzweige, dichte Nadeln
    start, ende = np.array([0.505, 0.75]), np.array([0.985, 0.75])
    zweige = [(start, ende, 1.0)]
    for k in range(13):
        s = 0.06 + k * 0.07
        p = start + (ende - start) * s
        laenge = 0.19 * (1 - s) ** 0.85 + 0.03
        for seite in (1, -1):
            w = seite * rng.uniform(0.62, 0.8)
            zweige.append((p, p + laenge * np.array([math.cos(w), math.sin(w)]), 0.8))
    for a, b, dicke in zweige:
        _strich(bild, gitter, a, b, 0.0045 * dicke, 0.42)
    for a, b, _ in zweige:
        d = b - a
        laenge = float(np.linalg.norm(d))
        winkel = math.atan2(d[1], d[0])
        schritte = int(laenge / 0.0055)
        for j in range(schritte):
            s = j / max(schritte, 1)
            p = a + d * s
            nadel = 0.034 * (1 - 0.45 * s) + 0.008
            for seite in (1, -1):
                w = winkel + seite * rng.uniform(0.75, 1.05)
                _blatt(bild, gitter, p, w, nadel, nadel * 0.2, rng.uniform(0.7, 1.0), rippe=False)

    # Palmwedel: Mittelrippe nach rechts, Fiederblätter zur Spitze geneigt
    start, ende = np.array([0.01, 0.375]), np.array([0.49, 0.375])
    for j in range(64):
        s = j / 64
        p = start + (ende - start) * s
        laenge = 0.115 * math.sin(math.pi * (0.1 + 0.88 * s)) ** 0.8 + 0.01
        for seite in (1, -1):
            w = seite * rng.uniform(0.85, 1.05)
            _blatt(bild, gitter, p, w, laenge, laenge * 0.2, rng.uniform(0.72, 1.0))
    _strich(bild, gitter, start, ende, 0.005, 0.55)

    # Grasbüschel: Halme von unten Mitte nach oben, außen kürzer und schräger
    for j in range(60):
        x = 0.25 + rng.normal(0, 0.06)
        neigung = (x - 0.25) * 3.2 + rng.uniform(-0.25, 0.25)
        laenge = rng.uniform(0.13, 0.235) * (1 - abs(x - 0.25) * 2.2)
        if laenge < 0.05:
            continue
        _blatt(bild, gitter, (x, 0.004), math.pi / 2 - neigung, laenge, rng.uniform(0.014, 0.022), rng.uniform(0.7, 1.0), form="halm", rippe=False)

    # Birkenrinde und daneben die weiße Fläche für Rinde & Co.
    x0, x1 = int(BIRKE[0] * n), int(BIRKE[2] * n)
    bild[: n // 2, x0:x1, :3] = _birkenrinde(np.random.default_rng(11), x1 - x0, n // 2)
    bild[: n // 2, x0:x1, 3] = 1.0
    bild[: n // 2, x1:] = 1.0

    img = bpy.data.images.new(TEXTUR_NAME, n, n, alpha=True)
    img.pixels.foreach_set(bild.ravel())
    TEXTUR_DATEI.parent.mkdir(parents=True, exist_ok=True)
    img.filepath_raw = str(TEXTUR_DATEI)
    img.file_format = "PNG"
    img.save()
    return img


def baum_material():
    """Das eine Material aller Bäume: Textur × Vertexfarbe, Ausschnitt (glTF: MASK), beidseitig."""
    if "Baum" in bpy.data.materials:
        return bpy.data.materials["Baum"]
    mat = bpy.data.materials.new("Baum")
    try:
        mat.use_nodes = True
    except (AttributeError, TypeError):
        pass
    knoten, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = next(k for k in knoten if k.type == "BSDF_PRINCIPLED")
    tex = knoten.new("ShaderNodeTexImage")
    tex.image = blatt_textur()
    vc = knoten.new("ShaderNodeVertexColor")
    vc.layer_name = "Farbe"
    mix = knoten.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    mix.blend_type = "MULTIPLY"
    mix.inputs["Factor"].default_value = 1.0
    links.new(tex.outputs["Color"], mix.inputs["A"])
    links.new(vc.outputs["Color"], mix.inputs["B"])
    links.new(mix.outputs["Result"], bsdf.inputs["Base Color"])
    rund = knoten.new("ShaderNodeMath")
    rund.operation = "ROUND"
    links.new(tex.outputs["Alpha"], rund.inputs[0])
    links.new(rund.outputs[0], bsdf.inputs["Alpha"])
    bsdf.inputs["Roughness"].default_value = 0.85
    mat.use_backface_culling = False
    for attr, wert in (("blend_method", "CLIP"), ("surface_render_method", "DITHERED")):
        try:
            setattr(mat, attr, wert)
        except (AttributeError, TypeError):
            pass
    return mat


def _ecken_uv(bereich):
    u0, v0, u1, v1 = bereich
    return [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]


# ---------------------------------------------------------------------------
# Baum
# ---------------------------------------------------------------------------
class Baum:
    """Sammelt die Geometrie eines Baums in einem bmesh, mit UV, Vertexfarbe und eigenen Normalen."""

    def __init__(self, name, seed, hoehe=8.0):
        self.name = name
        self.bm = bmesh.new()
        self.rng = random.Random(seed)
        self.uv = self.bm.loops.layers.uv.new("UV")
        self.farbe = self.bm.loops.layers.float_color.new("Farbe")
        self.normalen = {}      # Vertex → eigene Normale
        self.fertig_gefaerbt = set()  # Flächen mit gesetzter Farbe/UV
        self.rinden = {}        # Farbschlüssel (material_index) → Farbe
        self.leuchtend = set()  # Flächen mit Leuchtmaterial
        self.leucht_farbe = None
        self.holz_uv = {}       # Holzfläche → {Vertex: UV} (z. B. Birkenrinde)
        self.hoehe = hoehe

    def rinde(self, hex_farbe):
        """Farbschlüssel für Rinde (wird als material_index an Rohr-Flächen gemerkt)."""
        schluessel = len(self.rinden)
        self.rinden[schluessel] = farbe(hex_farbe)
        return schluessel

    def zufallsrichtung(self):
        r = self.rng
        return Vector((r.uniform(-1, 1), r.uniform(-1, 1), r.uniform(-1, 1)))

    # --- Holz -------------------------------------------------------------
    def rohr(self, punkte, radien, ecken, mat, spitze=True, streifen=None, muster=None, rauh=1.0, uv=None):
        """Konisches Rohr entlang einer Punktfolge; `streifen`: Farbe für Rindenrillen,
        `muster(ring, ecke)`: Farbe je Fläche, `rauh`: 0 = glatt rund,
        `uv(ring, ecke)`: Texturkoordinaten der Fläche (unten links, unten rechts, oben rechts,
        oben links) – sonst bekommt das Holz nur die weiße Fläche der Textur."""
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
                r = radien[i] * (1 + self.rng.uniform(-0.08, 0.06) * rauh)
                ring.append(bm.verts.new(p + (normale * math.cos(a) + binormale * math.sin(a)) * r))
            ringe.append(ring)
        for i, (a, b) in enumerate(zip(ringe, ringe[1:])):
            for k in range(ecken):
                ecken_f = (a[k], a[(k + 1) % ecken], b[(k + 1) % ecken], b[k])
                f = bm.faces.new(ecken_f)
                f.material_index = muster(i, k) if muster else streifen if k in bahnen else mat
                if uv:
                    # je Ecke gemerkt: `fertig` darf die Fläche noch umdrehen
                    self.holz_uv[f] = dict(zip(ecken_f, uv(i, k)))
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

    # --- Karten und Kugeln ------------------------------------------------
    def flaeche(self, punkte, uvs, farben, normalen):
        """Eine fertig gefärbte Fläche mit eigenen Normalen."""
        verts = [self.bm.verts.new(p) for p in punkte]
        f = self.bm.faces.new(verts)
        for loop, uv, c in zip(f.loops, uvs, farben):
            loop[self.uv].uv = uv
            loop[self.farbe] = (c.x, c.y, c.z, 1.0)
        for v, n in zip(verts, normalen):
            self.normalen[v] = n.normalized()
        self.fertig_gefaerbt.add(f)
        return f

    def kugel(self, mitte, radius, c, stufen=1, platt=1.0, leuchtend=False, dunkel_unten=0.3):
        """Einfarbige, weich schattierte Kugel (Kern einer Krone, Frucht, Nuss)."""
        erg = bmesh.ops.create_icosphere(self.bm, subdivisions=stufen, radius=radius)
        for v in erg["verts"]:
            rel = Vector((v.co.x, v.co.y, v.co.z * platt))
            v.co = mitte + rel
            self.normalen[v] = (rel.normalized() + OBEN * 0.25).normalized()
        for f in {f for v in erg["verts"] for f in v.link_faces}:
            for loop in f.loops:
                loop[self.uv].uv = FLECK
                unten = max(0.0, -(loop.vert.co.z - mitte.z) / radius)
                k = c * (1 - dunkel_unten * unten)
                loop[self.farbe] = (k.x, k.y, k.z, 1.0)
            self.fertig_gefaerbt.add(f)
            if leuchtend:
                self.leuchtend.add(f)
        if leuchtend:
            self.leucht_farbe = c

    def _laubfarbe(self, p, mitte, radius, platt, grund):
        rel = p - mitte
        aussen = min(1.15, rel.length / radius)
        hoehe = max(0.0, min(1.0, p.z / self.hoehe))
        oben = max(-1.0, min(1.0, rel.z / (radius * platt)))
        c = grund * ((0.45 + 0.55 * aussen) * (0.8 + 0.22 * hoehe) * (0.9 + 0.2 * oben))
        # Sonnenseite: heller und gelblicher
        sonne = max(0.0, oben) * min(1.0, aussen) * 0.55
        return c.lerp(Vector((c.x * 1.3 + 0.01, c.y * 1.25 + 0.01, c.z * 0.75)), sonne)

    def krone(self, mitte, radius, grund, dichte=1.0, platt=0.8, karte=1.0, kern=0.72, kern_stufen=1, kern_hell=0.5):
        """Dunkler Kern gegen Durchblicken plus viele Blattkarten auf der Hülle.
        `platt` < 1 ergibt flache Laub-Polster, `karte` skaliert die Blattkarten."""
        r = self.rng
        self.kugel(mitte, radius * kern, grund * kern_hell, stufen=kern_stufen, platt=platt, dunkel_unten=0.4)
        for _ in range(int(dichte * (30 * radius * radius + 16))):
            d = self.zufallsrichtung()
            if d.length < 0.05:
                continue
            d.normalize()
            abstand = radius * (0.45 + 0.55 * r.random() ** 0.6)
            p = mitte + Vector((d.x, d.y, d.z * platt)) * abstand
            aussen = (p - mitte).normalized()
            normale = (aussen + self.zufallsrichtung() * 0.28 + OBEN * 0.2).normalized()
            groesse = radius * r.uniform(0.75, 1.0) * karte
            t = Quaternion(normale, r.uniform(0, math.tau)) @ normale.orthogonal().normalized()
            b = normale.cross(t)
            ecken = [p + (-t - b) * groesse / 2, p + (t - b) * groesse / 2, p + (t + b) * groesse / 2, p + (-t + b) * groesse / 2]
            ton = grund * r.uniform(0.9, 1.1)
            normalen = []
            for e in ecken:
                rel = e - mitte
                rel.z /= platt
                normalen.append(rel.normalized() + OBEN * 0.35)
            self.flaeche(ecken, _ecken_uv(LAUB), [self._laubfarbe(e, mitte, radius, platt, ton) for e in ecken], normalen)

    # --- Fertigstellen ----------------------------------------------------
    def fertig(self, moos=None, rinden_muster=None):
        """Rinde einfärben, Mesh-Objekt mit weichen Normalen erzeugen, nichts unter dem Boden."""
        bm = self.bm
        for v in bm.verts:
            if v.co.z < 0:
                v.co.z = 0.0
        holz = [f for f in bm.faces if f not in self.fertig_gefaerbt]
        bmesh.ops.recalc_face_normals(bm, faces=holz)
        bm.normal_update()
        for f in holz:
            c = self.rinden.get(f.material_index, Vector((0.12, 0.08, 0.05)))
            if rinden_muster:
                c = rinden_muster(self.rng, f, c)
            textur = self.holz_uv.get(f)
            # Einfarbiges Holz bekommt pro Fläche leichte Farbschwankungen; texturierte Rinde
            # kaum, sonst sähe man die Flächen als Kästchen
            c = c * (self.rng.uniform(0.97, 1.02) if textur else self.rng.uniform(0.9, 1.08))
            for loop in f.loops:
                loop[self.uv].uv = textur[loop.vert] if textur else FLECK
                k = c
                if moos is not None:
                    k = k.lerp(moos, max(0.0, 1 - loop.vert.co.z / 1.2) * 0.65)
                loop[self.farbe] = (k.x, k.y, k.z, 1.0)
        for f in bm.faces:
            f.material_index = 1 if f in self.leuchtend else 0
        bm.verts.index_update()
        eigene = {v.index: n for v, n in self.normalen.items() if v.is_valid}
        mesh = bpy.data.meshes.new(self.name)
        bm.to_mesh(mesh)
        bm.free()
        mesh.polygons.foreach_set("use_smooth", [True] * len(mesh.polygons))
        normalen = []
        for loop in mesh.loops:
            n = eigene.get(loop.vertex_index)
            normalen.append(tuple(n) if n is not None else tuple(mesh.vertices[loop.vertex_index].normal))
        mesh.normals_split_custom_set(normalen)
        mesh.materials.append(baum_material())
        if self.leuchtend:
            c = self.leucht_farbe
            hex_farbe = "#%02X%02X%02X" % tuple(int(min(1.0, x) ** (1 / 2.2) * 255) for x in c)
            mesh.materials.append(material(f"{self.name}_Leuchten", farbe=hex_farbe, leuchten=4.0))
        obj = bpy.data.objects.new(self.name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        dreiecke = sum(len(p.vertices) - 2 for p in mesh.polygons)
        print(f"BAUM {self.name}: {dreiecke} Dreiecke")
        return obj


def _drehen(richtung, winkel, azimut):
    """Richtung um `winkel` von `richtung` weg kippen, rundherum um `azimut` gedreht."""
    senkrecht = richtung.orthogonal().normalized()
    senkrecht = Quaternion(richtung, azimut) @ senkrecht
    return (richtung * math.cos(winkel) + senkrecht * math.sin(winkel)).normalized()


def _wurzeln(baum, radius, anzahl, mat, spreizung=1.0, streifen=None):
    """Wurzeln, die schräg aus dem Boden brechen."""
    r = baum.rng
    for k in range(anzahl):
        a = math.tau * k / anzahl + r.uniform(-0.3, 0.3)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        start = aussen * radius * 0.55 + Vector((0, 0, radius * 1.3))
        ende = aussen * radius * (1.7 + r.uniform(0, 0.9)) * spreizung + Vector((0, 0, -0.08))
        mitte = start.lerp(ende, 0.5) + Vector((0, 0, radius * 0.12))
        baum.rohr([start, mitte, ende], [radius * 0.42, radius * 0.28, radius * 0.1], 5, mat, streifen=streifen)


# ---------------------------------------------------------------------------
# Eiche: dicker Stamm, ausladende Äste, breite, sonnige Krone
# ---------------------------------------------------------------------------
def eiche(seed=1, name="Eiche", laub="#5FA83A"):
    baum = Baum(name, seed, hoehe=8.5)
    r = baum.rng
    rinde = baum.rinde("#6B5140")
    rille = baum.rinde("#4A382B")
    gruen = farbe(laub)
    stamm_r = r.uniform(0.42, 0.5)
    stamm = baum.pfad(Vector((0, 0, 0)), OBEN, r.uniform(2.3, 2.8), 5, 0.12, 0.0, 0.05)
    radien = [stamm_r * (1.25 - 0.4 * i / 5) for i in range(6)]
    baum.rohr(stamm, radien, 12, rinde, spitze=True, streifen=rille)
    _wurzeln(baum, stamm_r, 6, rinde, 1.5, streifen=rille)
    enden = []

    def ast(start, richtung, laenge, radius, tiefe):
        schritte = max(3, int(laenge / 0.45))
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.2, 0.02, 0.12)
        rad = [radius * (1 - 0.6 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, rad, 8 if tiefe == 1 else 6, rinde, streifen=rille)
        if tiefe >= 2:
            enden.append((punkte[-1], tiefe))
            return
        enden.append((punkte[len(punkte) * 2 // 3], tiefe + 1))
        for k in range(2):
            i = int(r.uniform(0.6, 0.95) * schritte)
            d = (punkte[min(i + 1, schritte)] - punkte[i - 1]).normalized()
            ast(punkte[i], _drehen(d, r.uniform(0.35, 0.7), math.pi * k + r.uniform(-0.6, 0.6)), laenge * 0.62, rad[i] * 0.7, tiefe + 1)

    haupt = r.randint(4, 5)
    for k in range(haupt):
        richtung = _drehen(OBEN, r.uniform(0.55, 0.95), math.tau * k / haupt + r.uniform(-0.3, 0.3))
        ast(stamm[-2].lerp(stamm[-1], 0.6), richtung, r.uniform(2.6, 3.4), radien[-1] * 0.62, 1)
    ast(stamm[-1], OBEN, 2.6, radien[-1] * 0.6, 1)
    for p, tiefe in enden:
        groesse = r.uniform(1.25, 1.6) if tiefe >= 2 else r.uniform(1.0, 1.3)
        baum.krone(p + Vector((0, 0, 0.35)), groesse, gruen * r.uniform(0.88, 1.12), platt=0.62)
    return baum.fertig(moos=farbe("#6E9A3A"))


# ---------------------------------------------------------------------------
# Schirm-Eiche: alter, mächtiger Stamm mit Brettwurzeln, weit ausladende Äste,
# flache, gestufte Laub-Polster
# ---------------------------------------------------------------------------
def eiche_schirm(seed=1, name="Eiche", laub="#86B83C"):
    baum = Baum(name, seed, hoehe=9.0)
    r = baum.rng
    rinde = baum.rinde("#7A5E48")
    rille = baum.rinde("#5A4434")
    gruen = farbe(laub)
    stamm_r = r.uniform(0.55, 0.65)
    stamm = baum.pfad(Vector((0, 0, 0)), OBEN, r.uniform(2.8, 3.3), 6, 0.1, 0.0, 0.03)
    radien = [stamm_r * (1.35 - 0.5 * i / 6) for i in range(7)]
    baum.rohr(stamm, radien, 12, rinde, spitze=True, streifen=rille)
    # Mächtige Brettwurzeln: setzen hoch am Stamm an und laufen breit in den Boden
    wurzeln = r.randint(5, 6)
    for k in range(wurzeln):
        w = math.tau * k / wurzeln + r.uniform(-0.25, 0.25)
        aussen = Vector((math.cos(w), math.sin(w), 0))
        oben = aussen * stamm_r * 0.45 + Vector((0, 0, r.uniform(1.1, 1.6)))
        mitte = aussen * stamm_r * 1.3 + Vector((0, 0, 0.45))
        unten = aussen * stamm_r * r.uniform(2.2, 2.8) + Vector((0, 0, -0.1))
        baum.rohr([oben, mitte, unten], [stamm_r * 0.5, stamm_r * 0.4, stamm_r * 0.24], 7, rinde, streifen=rille)

    def polster(mitte, breite, c):
        # flaches Laub-Polster mit ein paar Buckeln obenauf
        baum.krone(mitte, breite, c, dichte=0.85, platt=0.34, karte=0.52, kern_hell=0.35)
        for _ in range(r.randint(2, 3)):
            w = r.uniform(0, math.tau)
            p = mitte + Vector((math.cos(w), math.sin(w), 0)) * breite * r.uniform(0.2, 0.55) + Vector((0, 0, breite * 0.2))
            baum.krone(p, breite * r.uniform(0.35, 0.45), c * r.uniform(0.95, 1.1), dichte=0.9, platt=0.65, karte=0.9, kern_stufen=0, kern_hell=0.35)

    def arm(start, richtung, laenge, radius, breite, aufwaerts=0.07):
        schritte = 7
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.1, 0.0, aufwaerts)
        rad = [radius * (1 - 0.6 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, rad, 9, rinde, streifen=rille)
        # zwei kürzere Seitenäste, die das Polster von unten stützen
        for s in (1, -1):
            i = int(r.uniform(0.55, 0.8) * schritte)
            d = (punkte[i + 1] - punkte[i]).normalized()
            seite = d.cross(OBEN).normalized() * s
            zweig = baum.pfad(punkte[i], (d * 0.5 + seite * 0.7 + OBEN * 0.35).normalized(), laenge * r.uniform(0.3, 0.42), 3, 0.15, 0.0, 0.1)
            baum.rohr(zweig, [rad[i] * 0.55, rad[i] * 0.42, rad[i] * 0.3, rad[i] * 0.2], 6, rinde, streifen=rille)
        polster(punkte[-1] + Vector((0, 0, 0.45)), breite, gruen * r.uniform(0.9, 1.1))

    arme = r.randint(3, 4)
    for k in range(arme):
        az = math.tau * k / arme + r.uniform(-0.35, 0.35)
        start = stamm[-3].lerp(stamm[-1], r.uniform(0.2, 1.0))
        # Abwechselnd tiefe, fast waagerechte und höhere, steilere Äste: ergibt die Stufen
        tief = k % 2 == 0
        winkel = r.uniform(1.2, 1.35) if tief else r.uniform(0.6, 0.78)
        laenge = r.uniform(4.6, 5.4) if tief else r.uniform(4.2, 4.8)
        breite = r.uniform(2.2, 2.6) if tief else r.uniform(1.9, 2.3)
        arm(start, _drehen(OBEN, winkel, az), laenge, radien[-1] * 0.62, breite, 0.04 if tief else 0.07)
    # Mittelstamm nach oben: oberstes, größtes Polster
    arm(stamm[-1], _drehen(OBEN, r.uniform(0.1, 0.25), r.uniform(0, math.tau)), r.uniform(4.6, 5.2), radien[-1] * 0.6, r.uniform(2.5, 2.9))
    return baum.fertig(moos=farbe("#5E6A3A"))


# ---------------------------------------------------------------------------
# Birke: ein bis drei schlanke, sanft geschwungene weiße Stämme mit feinen schwarzen
# Querstrichen und dunklem, rissigem Fuß; ovale Krone aus vielen Laub-Lappen an dünnen Ästen
# ---------------------------------------------------------------------------
def birke(seed=1, name="Birke", laub="#EBC23A", staemme=2):
    baum = Baum(name, seed, hoehe=10.5)
    r = baum.rng
    weiss = baum.rinde("#F2EFE8")
    grau = baum.rinde("#D2CCC2")
    fuss = baum.rinde("#4E4842")
    gelb = farbe(laub)
    orange = farbe("#E0862A")
    ecken = 10 if staemme < 3 else 8
    dichte = 1.3 if staemme < 3 else 1.0

    for s in range(staemme):
        az = math.tau * s / staemme + r.uniform(-0.5, 0.5)
        aussen = Vector((math.cos(az), math.sin(az), 0))
        quer = Vector((-aussen.y, aussen.x, 0)) * r.uniform(-1, 1)
        neigung = r.uniform(0.03, 0.07) if staemme > 1 else r.uniform(0.0, 0.03)
        hoehe = r.uniform(8.8, 10.2) * (1.0 - 0.12 * s)
        radius = r.uniform(0.16, 0.19) * (1.0 - 0.1 * s)
        start = aussen * (0.16 if staemme > 1 else 0.0)
        schwung = r.uniform(0.1, 0.25)

        def punkt(t, start=start, aussen=aussen, quer=quer, neigung=neigung, hoehe=hoehe, schwung=schwung):
            # sanfter Bogen nach außen und ein leichtes S – keine zittrigen Knicke
            return (start + OBEN * hoehe * t + aussen * (neigung * hoehe * t + schwung * 0.6 * math.sin(math.pi * t))
                    + quer * schwung * 0.4 * math.sin(math.tau * t))

        # Die Rinde kommt aus der Textur (feine Querstriche, Schlieren, Astnarben); eine Kachel
        # reicht einmal um den Stamm und 1,6 m hoch. Ringe liegen genau auf den Kachelgrenzen,
        # am Fuß enger (für den ausgefransten dunklen Rand), sonst etwa alle 30 cm.
        kachel = 1.6
        zs, grenze = [0.0], kachel
        while zs[-1] < hoehe - 1e-4:
            z = min(hoehe, zs[-1] + (0.09 if zs[-1] < 0.9 else r.uniform(0.26, 0.36)))
            if z > grenze - 0.05 and grenze < hoehe:
                z, grenze = grenze, grenze + kachel
            zs.append(z)
        ts = [z / hoehe for z in zs]
        punkte = [punkt(t) for t in ts]
        radien = [radius * (1 - 0.72 * t) + radius * 0.45 * max(0.0, 1 - t * hoehe / 0.7) ** 2 for t in ts]
        # Jede Kachel um ganze Flächen gedreht, damit sich das Muster nicht erkennbar wiederholt
        drehung = [r.randrange(ecken) for _ in range(int(hoehe / kachel) + 2)]
        bu0, bv0, bu1, bv1 = BIRKE

        def rinde_uv(i, k, zs=zs):
            j = int((zs[i] + 1e-4) // kachel)
            va, vb = (zs[i] - j * kachel) / kachel, (zs[i + 1] - j * kachel) / kachel
            ka = (k + drehung[j]) % ecken
            ua, ub = bu0 + (bu1 - bu0) * ka / ecken, bu0 + (bu1 - bu0) * (ka + 1) / ecken
            va, vb = bv0 + (bv1 - bv0) * va, bv0 + (bv1 - bv0) * vb
            return [(ua, va), (ub, va), (ub, vb), (ua, vb)]

        # Dunkler Fuß: jede Längsbahn endet in anderer Höhe, dazu vereinzelte dunkle Zungen
        fuss_hoehe = [r.uniform(0.2, 0.6) for _ in range(ecken)]
        zungen = {k: r.uniform(0.6, 0.95) for k in range(ecken) if r.random() < 0.3}

        def muster(i, k, zs=zs, fuss_hoehe=fuss_hoehe, zungen=zungen):
            z = zs[i]
            if z < fuss_hoehe[k] or z < zungen.get(k, 0.0):
                return fuss
            return grau if z < fuss_hoehe[k] + 0.2 else weiss

        baum.rohr(punkte, radien, ecken, weiss, spitze=True, muster=muster, rauh=0.15, uv=rinde_uv)

        # Krone: ein paar große, hohe Laubmassen am oberen Stamm …
        for t in ((0.66, 0.81, 0.95) if staemme < 3 else (0.7, 0.92)):
            c = gelb.lerp(orange, r.random() ** 2 * 0.5) * r.uniform(0.95, 1.08)
            groesse = r.uniform(1.3, 1.55) * (0.85 if t < 0.7 else 1.0 if t < 0.9 else 0.85)
            baum.krone(punkt(t) + aussen * 0.2, groesse, c, dichte=1.0 * dichte, platt=1.25, karte=0.9, kern=0.55, kern_hell=0.3)
        # … und dünne Äste, deren kleinere Lappen aus dem Umriss herausragen
        aeste = 4 if staemme < 3 else 3
        lappen = dict(platt=1.0, kern=0.5, kern_stufen=0, kern_hell=0.3, karte=0.9, dichte=dichte)
        for j in range(aeste):
            t = 0.55 + 0.4 * (j + r.uniform(0.1, 0.9)) / aeste
            p = punkt(t)
            w = az + j * 2.4 + r.uniform(-0.4, 0.4)
            d = Vector((math.cos(w), math.sin(w), 0))
            if staemme > 1:
                d = (d + aussen * 0.8).normalized()
            huelle = math.sin(math.pi * min(1.0, (t - 0.3) / 0.75))
            ast = baum.pfad(p, (d * 0.75 + OBEN * r.uniform(0.7, 1.0)).normalized(), 1.2 + 1.2 * huelle * r.uniform(0.8, 1.1), 3, 0.12, 0.05, 0.08)
            ar = radius * (1 - 0.72 * t) * 0.55
            baum.rohr(ast, [ar, ar * 0.7, ar * 0.5, ar * 0.3], 5, weiss, rauh=0.3)
            c = gelb.lerp(orange, r.random() ** 2 * 0.6) * r.uniform(0.92, 1.08)
            baum.krone(ast[-1] + Vector((0, 0, 0.15)), r.uniform(0.75, 0.95) * (0.75 + 0.35 * huelle), c, **lappen)
    return baum.fertig()


# ---------------------------------------------------------------------------
# Tanne: Etagen aus hängenden Nadelzweig-Karten um einen dunklen Kernkegel
# ---------------------------------------------------------------------------
def tanne(seed=1, name="Tanne", nadeln="#2E8A50"):
    hoehe = random.Random(seed).uniform(8.0, 10.0)
    baum = Baum(name, seed, hoehe=hoehe)
    r = baum.rng
    rinde = baum.rinde("#5A4234")
    gruen = farbe(nadeln)
    radius = r.uniform(0.22, 0.27)
    stamm = baum.pfad(Vector((0, 0, 0)), OBEN, hoehe * 0.86, 8, 0.02, 0.0, 0.0)
    baum.rohr(stamm, [radius * (1.15 - i / 8) + 0.02 for i in range(9)], 8, rinde)
    _wurzeln(baum, radius, 5, rinde, 0.85)

    # Kernkegel: verdeckt Lücken zwischen den Zweigen
    kegel = bmesh.ops.create_cone(baum.bm, cap_ends=False, segments=9, radius1=1.05, radius2=0.08, depth=hoehe * 0.78)
    for v in kegel["verts"]:
        v.co.z += hoehe * 0.16 + hoehe * 0.39
        rel = Vector((v.co.x, v.co.y, 0))
        baum.normalen[v] = ((rel.normalized() if rel.length > 0.01 else OBEN) * 0.8 + OBEN * 0.6).normalized()
    for f in {f for v in kegel["verts"] for f in v.link_faces}:
        for loop in f.loops:
            loop[baum.uv].uv = FLECK
            k = gruen * (0.2 + 0.15 * (loop.vert.co.z / hoehe))
            loop[baum.farbe] = (k.x, k.y, k.z, 1.0)
        baum.fertig_gefaerbt.add(f)

    def normale(p):
        rel = Vector((p.x, p.y, 0))
        return (rel.normalized() if rel.length > 0.01 else OBEN) * 0.75 + OBEN * 0.7

    def zweig(start, richtung, laenge, t_hoehe):
        """Zweig-Karte als flaches Dach (zwei Flächen), Nadeln hängen seitlich etwas herab."""
        seite = richtung.cross(OBEN).normalized()
        breite = laenge * 0.95
        unten = Vector((0, 0, -0.18 * breite))
        ende = start + richtung * laenge
        u0, v0, u1, v1 = NADEL
        vm = (v0 + v1) / 2
        links = [start - seite * breite / 2 + unten, ende - seite * breite / 2 + unten]
        rechts = [start + seite * breite / 2 + unten, ende + seite * breite / 2 + unten]
        innen = gruen * (0.55 + 0.35 * t_hoehe)
        aussen = gruen * (1.05 + 0.25 * t_hoehe)
        aussen = aussen.lerp(Vector((aussen.x * 1.25, aussen.y * 1.15, aussen.z * 0.8)), 0.4)
        mitte = [start, ende]
        punkte = [links[0], links[1], mitte[1], mitte[0]]
        baum.flaeche(punkte, [(u0, v0), (u1, v0), (u1, vm), (u0, vm)], [innen * 0.85, aussen * 0.9, aussen, innen], [normale(p) for p in punkte])
        punkte = [mitte[0], mitte[1], rechts[1], rechts[0]]
        baum.flaeche(punkte, [(u0, vm), (u1, vm), (u1, v1), (u0, v1)], [innen, aussen, aussen * 0.9, innen * 0.85], [normale(p) for p in punkte])

    # Etagen von Zweigen, jeder Zweig in zwei Lagen (unten lang und hängend, oben kürzer)
    etagen = 16
    for e in range(etagen):
        t = e / etagen
        z = hoehe * (0.14 + 0.8 * t)
        laenge = (1 - t) ** 0.95 * 2.9 + 0.6
        anzahl = 9 if t < 0.6 else 7 if t < 0.85 else 5
        for k in range(anzahl):
            a = math.tau * k / anzahl + e * 0.55 + r.uniform(-0.25, 0.25)
            aussen = Vector((math.cos(a), math.sin(a), r.uniform(-0.5, -0.22))).normalized()
            zweig(Vector((0, 0, z)), aussen, laenge * r.uniform(0.9, 1.08), t)
            if t < 0.85:
                flacher = Vector((aussen.x, aussen.y, aussen.z * 0.5)).normalized()
                zweig(Vector((0, 0, z + 0.25)), flacher, laenge * r.uniform(0.55, 0.7), min(1.0, t + 0.1))
    # Spitze: drei gekreuzte, nach oben zeigende Zweige
    u0, v0, u1, v1 = NADEL
    for k in range(3):
        a = math.pi * k / 3
        start = Vector((0, 0, hoehe - 1.2))
        oben = start + Vector((0, 0, 1.9))
        seite = Vector((math.cos(a), math.sin(a), 0)) * 0.35
        c0, c1 = gruen * 0.9, gruen * 1.35
        baum.flaeche([start - seite, oben - seite, oben + seite, start + seite], [(u0, v0), (u1, v0), (u1, v1), (u0, v1)],
                     [c0, c1, c1, c0], [OBEN - seite, OBEN - seite, OBEN + seite, OBEN + seite])
    return baum.fertig()


# ---------------------------------------------------------------------------
# Palme: geringelter, geschwungener Stamm, gebogene Wedel-Karten, Kokosnüsse
# ---------------------------------------------------------------------------
def palme(seed=1, name="Palme", laub="#6DB43C"):
    baum = Baum(name, seed, hoehe=7.5)
    r = baum.rng
    hell = baum.rinde("#9A7B5C")
    dunkel = baum.rinde("#735A45")
    gruen = farbe(laub)
    hoehe = r.uniform(6.0, 7.2)
    w = r.uniform(0, math.tau)
    neigung = Vector((math.cos(w), math.sin(w), 0))
    ringe = 14
    punkte = [neigung * ((i / ringe) ** 2 * 1.6) + Vector((0, 0, hoehe * i / ringe)) for i in range(ringe + 1)]
    radien = [0.24 - 0.09 * i / ringe for i in range(ringe + 1)]
    for i in range(ringe):
        # Jeder Ring unten etwas dicker: die typischen Wülste
        baum.rohr([punkte[i], punkte[i + 1]], [radien[i] * 1.12, radien[i + 1] * 0.94], 9, hell if i % 2 == 0 else dunkel, spitze=False)
    krone = punkte[-1]
    baum.kugel(krone, 0.36, gruen * 0.45, platt=0.8)

    def gelb(c, s):
        # Wedelspitzen werden gelblich
        return c.lerp(Vector((c.x * 1.35 + 0.02, c.y * 1.15, c.z * 0.6)), max(0.0, s - 0.7) * 1.6)

    wedel = r.randint(13, 15)
    u0, v0, u1, v1 = WEDEL
    vm = (v0 + v1) / 2
    for k in range(wedel):
        a = math.tau * k / wedel + r.uniform(-0.18, 0.18)
        aussen = Vector((math.cos(a), math.sin(a), 0))
        innen_wedel = k % 3 == 1
        laenge = r.uniform(2.6, 3.3) * (0.7 if innen_wedel else 1.0)
        hang = r.uniform(0.5, 0.85) * (0.5 if innen_wedel else 1.0)
        d = (aussen + Vector((0, 0, r.uniform(0.5, 0.9)))).normalized()
        segmente = 8
        rippe = [krone + d * 0.1]
        for _ in range(segmente):
            d = (d + Vector((0, 0, -hang / 3.2))).normalized()
            rippe.append(rippe[-1] + d * laenge / segmente)
        seite = aussen.cross(OBEN).normalized()
        n = OBEN + aussen * 0.35
        for i in range(segmente):
            s0, s1 = i / segmente, (i + 1) / segmente
            b0 = 0.62 * math.sin(math.pi * (0.1 + 0.85 * s0)) ** 0.7 + 0.05
            b1 = 0.62 * math.sin(math.pi * (0.1 + 0.85 * s1)) ** 0.7 + 0.05
            h0, h1 = Vector((0, 0, -0.3 * b0)), Vector((0, 0, -0.3 * b1))
            m0, m1 = rippe[i], rippe[i + 1]
            l0, l1 = m0 - seite * b0 + h0, m1 - seite * b1 + h1
            r0, r1 = m0 + seite * b0 + h0, m1 + seite * b1 + h1
            ua, ub = u0 + (u1 - u0) * s0, u0 + (u1 - u0) * s1
            c0 = gelb(gruen * (0.6 + 0.6 * s0), s0)
            c1 = gelb(gruen * (0.6 + 0.6 * s1), s1)
            baum.flaeche([l0, l1, m1, m0], [(ua, v0), (ub, v0), (ub, vm), (ua, vm)], [c0 * 0.9, c1 * 0.9, c1, c0], [n] * 4)
            baum.flaeche([m0, m1, r1, r0], [(ua, vm), (ub, vm), (ub, v1), (ua, v1)], [c0, c1, c1 * 0.9, c0 * 0.9], [n] * 4)
    for _ in range(r.randint(3, 5)):
        a = r.uniform(0, math.tau)
        baum.kugel(krone + Vector((math.cos(a) * 0.25, math.sin(a) * 0.25, -0.3)), 0.14, farbe("#5A4028"), platt=1.05)
    return baum.fertig()


# ---------------------------------------------------------------------------
# Zauberbaum: verdrehter violettgrauer Stamm, Laub in Violett und Türkis, Leuchtfrüchte
# ---------------------------------------------------------------------------
def zauberbaum(seed=1, name="Zauberbaum", violett="#9B6BE8", tuerkis="#3CC8C0"):
    baum = Baum(name, seed, hoehe=8.0)
    r = baum.rng
    rinde = baum.rinde("#4E4460")
    rille = baum.rinde("#6A5C80")
    farben = [farbe(violett), farbe(tuerkis)]
    glut = farbe("#E4C2FF")
    fruechte = []

    def ast(start, richtung, laenge, radius, tiefe):
        schritte = max(3, int(laenge / 0.32))
        richtung = richtung.copy()
        richtung.z = max(richtung.z, 0.3)  # kein Ast hängt nach unten
        punkte = baum.pfad(start, richtung, laenge, schritte, 0.4, 0.0, 0.16 + 0.08 * tiefe)
        rad = [radius * (1 - 0.68 * i / schritte) for i in range(schritte + 1)]
        baum.rohr(punkte, rad, max(5, 8 - tiefe), rinde, streifen=rille)
        if tiefe >= 3 or rad[-1] < 0.03:
            c = farben[0].lerp(farben[1], r.random() ** 2)
            baum.krone(punkte[-1] + Vector((0, 0, 0.2)), r.uniform(0.95, 1.25), c, dichte=1.35, platt=0.8, kern=0.6)
            for _ in range(r.randint(1, 2)):
                fruechte.append(punkte[-1] + baum.zufallsrichtung() * 0.5 + Vector((0, 0, -0.6)))
            return
        for k in range(r.randint(2, 3)):
            i = min(schritte, int(r.uniform(0.5, 1.0) * schritte))
            d = (punkte[min(i + 1, schritte)] - punkte[max(i - 1, 0)]).normalized()
            ast(punkte[i], _drehen(d, r.uniform(0.55, 1.0), r.uniform(0, math.tau)), laenge * 0.72, rad[i] * 0.62, tiefe + 1)

    stamm = baum.pfad(Vector((0, 0, 0)), OBEN, r.uniform(2.5, 3.1), 7, 0.28, 0.0, 0.05)
    stamm_radien = [0.34 * (1.25 - 0.45 * i / 7) for i in range(8)]
    baum.rohr(stamm, stamm_radien, 10, rinde, spitze=True, streifen=rille)
    _wurzeln(baum, 0.32, 6, rinde, 1.35, streifen=rille)
    for k in range(3):
        ast(stamm[-2].lerp(stamm[-1], 0.6), _drehen(OBEN, r.uniform(0.5, 0.85), math.tau * k / 3 + r.uniform(-0.3, 0.3)), 2.4, stamm_radien[-1] * 0.72, 1)
    for p in fruechte:
        baum.kugel(p, r.uniform(0.1, 0.14), glut, stufen=1, platt=1.2, leuchtend=True, dunkel_unten=0.0)
    return baum.fertig(moos=farbe("#3F8F86"))
