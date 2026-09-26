"""Siedlung und Wirtschaftsgebäude: Dorfhalle (Langhaus → Rathaus → Burgfried), Holzfäller,
Steinbruch und Erzmine.

Gebaut mit dem Werkzeugkasten der Türme (`tuerme.Werk`): einzeln gesetzte Mauersteine im Verband,
Eckquader, Schindeldächer mit versetzten Reihen, Windbretter und Sparrenköpfe, Banner, Wimpel,
Laternen, Wagen mit Speichenrädern, Zäune, Brunnen und Marktstände. Hell und freundlich wie die
übrige Insel.

Der Ursprung liegt auf dem Boden in der Mitte, die Vorderseite zeigt nach −Y (im Spiel nach +Z).
Sockel reichen `SOCKEL` Meter unter den Boden. Das Spiel lässt die Gebäude beim Bau von unten nach
oben entstehen – deshalb besteht alles aus einzelnen, übereinander liegenden Teilen.

`DREIECKE=1` in der Umgebung druckt, welche Teile wie viele Dreiecke kosten.
"""

import math
import os

import bmesh
from mathutils import Matrix, Vector

from bauten import (SOCKEL, _banner_mast, _brett, _dorf_fenster, _fachwerkwand, _fertig, _giebel, _laterne, _platte,
                    _schild, _stamm)
from lager import _fass_teile, _rad, brett_bm, einfarbig, holzfarbe, objekt, setzen, stamm_bm, stammfarbe, stein_bm
from tuerme import (EISEN, GOLD, TUCH_BLAU, Werk, _kiste, fahne, mauerblock, mauerring, steinfarbe,
                    wandbanner)
from vorkommen import _brocken_bm, _setzen, _splitter, _steinfarbe, farbe

WURZEL2 = math.sqrt(2.0)


# ---------------------------------------------------------------------------
# Werkzeuge
# ---------------------------------------------------------------------------
def fertig(w, name):
    if os.environ.get("DREIECKE"):
        summe = {}
        for o in w.teile + w.licht:
            n = o.name.rstrip("0123456789.")
            summe[n] = summe.get(n, 0) + sum(len(p.vertices) - 2 for p in o.data.polygons)
        for n, d in sorted(summe.items(), key=lambda x: -x[1])[:18]:
            print(f"  {name:12s} {n:18s} {d}")
    return _fertig(name, w.teile, w.licht)


def setze(w, ort, rz, bauen):
    """Baut mit `bauen(werk)` um den Ursprung und setzt das Ergebnis gedreht (Grad) an `ort`."""
    tmp = Werk(0)
    tmp.z = w.z
    bauen(tmp)
    m = Matrix.Translation(Vector((ort[0], ort[1], ort[2] if len(ort) > 2 else 0.0))) @ Matrix.Rotation(math.radians(rz), 4, "Z")
    for o in tmp.teile + tmp.licht:
        o.data.transform(m)
    w.teile += tmp.teile
    w.licht += tmp.licht


def sockel(w, farbe_von, b, d, oben, mitte=(0.0, 0.0), h=0.42):
    """Gemauerter Sockel (Oberkante `oben`) mit Abdeckplatte, darunter ein Kern bis tief in den Boden."""
    quaderblock(w, farbe_von, b - 0.4, d - 0.4, -0.3, oben - 0.12, mitte=mitte, h=h, laenge=1.25, tiefe=0.4, fase=0.03, kern=None)
    w.brett("Sockelkern", farbe_von, (b - 0.3, d - 0.3, SOCKEL), (mitte[0], mitte[1], -0.3 - SOCKEL / 2 + 0.05), fase=0.0)
    w.brett("Sockelplatte", farbe_von, (b + 0.12, d + 0.12, 0.12), (mitte[0], mitte[1], oben - 0.06), fase=0.03)


def quadermauer(w, farbe_von, a, b, z0, z1, tiefe=0.8, h=0.55, laenge=1.1, kern="#57524B", beide=True, fase=0.04):
    """Gerade Mauer von `a` nach `b` (Mittellinie): Steine im Verband auf beiden Seiten, dunkler Kern."""
    a, b = Vector((a[0], a[1], 0)), Vector((b[0], b[1], 0))
    d = b - a
    L = d.length
    richt = d.normalized()
    quer = Vector((-richt.y, richt.x, 0))
    rz = math.degrees(math.atan2(d.y, d.x))
    lagen = max(1, round((z1 - z0) / h))
    h = (z1 - z0) / lagen
    n = max(1, round(L / laenge))
    teil = L / n
    for l in range(lagen):
        z = z0 + (l + 0.5) * h
        for s in ((-1, 1) if beide else (-1,)):
            versatz = 0.5 if (l + (s > 0)) % 2 else 0.0
            for k in range(n + (1 if versatz else 0)):
                u0 = max((k - versatz) * teil, 0.0)
                u1 = min((k - versatz + 1) * teil, L)
                if u1 - u0 < 0.15:
                    continue
                m = a + richt * ((u0 + u1) / 2) + quer * (s * tiefe * 0.27)
                bm = brett_bm((u1 - u0) * w.z.uniform(0.93, 0.99), tiefe * 0.46 * w.z.uniform(0.95, 1.08), h * w.z.uniform(0.88, 0.95), fase=fase)
                setzen(bm, (0, 0, 0), (w.z.uniform(-1, 1), 0, rz))
                setzen(bm, (m.x, m.y, z))
                w.bm("Mauerstein", bm, farbe_von)
    if not kern:
        return
    mitte = (a + b) / 2
    w.brett("Mauerkern", einfarbig(kern, 0.05, w.z), (L - 0.04, tiefe * 0.6, z1 - z0), (mitte.x, mitte.y, (z0 + z1) / 2), (0, 0, rz), fase=0.0)


def eckquader(w, farbe_von, ecke, da, db, z0, z1, h=0.6, lang=0.95, tiefe=0.5, vor=0.05):
    """Eckverband: abwechselnd lange Quader entlang der beiden Wände, die sich in `ecke` treffen."""
    ecke = Vector((ecke[0], ecke[1], 0))
    da = Vector((da[0], da[1], 0)).normalized()
    db = Vector((db[0], db[1], 0)).normalized()
    lagen = max(1, round((z1 - z0) / h))
    h = (z1 - z0) / lagen
    for l in range(lagen):
        a, b = (da, db) if l % 2 == 0 else (db, da)
        laenge = lang * w.z.uniform(0.85, 1.05)
        m = ecke + a * (laenge / 2 - vor) + b * (tiefe / 2 - vor)
        w.brett("Eckquader", farbe_von, (laenge, tiefe, h * 0.9), (m.x, m.y, z0 + (l + 0.5) * h), (0, 0, math.degrees(math.atan2(a.y, a.x))), fase=0.04)


def stufendach(w, farbe_von, r, z, hoehe, ecken=10, reihen=6, ort=(0.0, 0.0), drehung_z=0.0, lippe=0.08):
    """Kegel- oder Pyramidendach (ecken=4, drehung_z=45) aus vorstehenden Schindelreihen."""
    profil = [(r * 0.8, z)]
    for i in range(reihen):
        r0, r1 = r * (1 - i / reihen), r * (1 - (i + 1) / reihen)
        z0, z1 = z + hoehe * i / reihen, z + hoehe * (i + 1) / reihen
        profil.append((r0 + lippe, z0 - lippe * 0.8))
        profil.append((r1 if i < reihen - 1 else 0.0, z1))
    w.dreh("Dach", farbe_von, profil, (ort[0], ort[1], 0.0), ecken=ecken, drehung=(0, 0, drehung_z))


def ziegeldach(w, farbe_von, breite, tiefe, traufe, first, ueberstand, reihen=8, stuecke=4, name="Schindeln", dicke=0.08):
    """Satteldach (First entlang X) aus Schindelreihen, jede zweite Reihe um ein halbes Stück versetzt."""
    halb = tiefe / 2 + ueberstand
    neigung = math.atan2(first - traufe, tiefe / 2)
    z_rand = traufe - ueberstand * math.tan(neigung)
    hang = halb / math.cos(neigung)
    L = breite + 2 * ueberstand
    teil = L / stuecke
    for seite in (-1, 1):
        for r in range(reihen):
            t = (r + 0.5) / reihen
            y = seite * halb * (1 - t)
            z = z_rand + (first - z_rand) * t
            versatz = 0.5 if r % 2 else 0.0
            for s in range(stuecke + (1 if versatz else 0)):
                a = max(-L / 2 + (s - versatz) * teil, -L / 2)
                e = min(-L / 2 + (s - versatz + 1) * teil, L / 2)
                if e - a < 0.2:
                    continue
                w.brett(name, farbe_von, ((e - a) * 0.985, hang / reihen * 1.3, dicke), ((a + e) / 2, y, z + 0.06),
                        (seite * -math.degrees(neigung), 0, w.z.uniform(-0.5, 0.5)), fase=0.02)
    w.brett("First", farbe_von, (L + 0.1, 0.34, 0.24), (0, 0, first + 0.16), fase=0.03)


def windbretter(w, farbe_von, x, tiefe, traufe, first, ueberstand, zier=True):
    """Windbretter an der Giebelkante bei `x`, oben gekreuzt über den First (Pferdeköpfe)."""
    halb = tiefe / 2 + ueberstand
    neigung = math.atan2(first - traufe, tiefe / 2)
    z_rand = traufe - ueberstand * math.tan(neigung)
    hang = halb / math.cos(neigung)
    for seite in (-1, 1):
        w.brett("Windbrett", farbe_von, (0.1, hang + 0.05, 0.3), (x, seite * halb / 2, (z_rand + first) / 2 + 0.12),
                (seite * -math.degrees(neigung), 0, 0), fase=0.02)
        if zier:
            w.brett("Giebelzier", farbe_von, (0.1, 0.8, 0.22), (x, -seite * 0.3, first + 0.3 * math.tan(neigung) + 0.2),
                    (seite * -math.degrees(neigung), 0, 0), fase=0.02)


def sparren(w, farbe_von, breite, tiefe, traufe, first, ueberstand, abstand=1.1):
    """Sparrenköpfe unter dem Dachüberstand der Traufseiten."""
    neigung = math.atan2(first - traufe, tiefe / 2)
    n = max(2, round(breite / abstand))
    for seite in (-1, 1):
        for i in range(n + 1):
            x = -breite / 2 + i * breite / n
            y = seite * (tiefe / 2 + ueberstand / 2 - 0.05)
            z = traufe - (ueberstand / 2 - 0.05) * math.tan(neigung) - 0.06
            w.brett("Sparrenkopf", farbe_von, (0.13, ueberstand + 0.1, 0.15), (x, y, z), (seite * -math.degrees(neigung), 0, 0), fase=0.015)


def steinwand(w, farbe_von, laenge, hoehe, ort, drehung_z, luecken=(), dicke=0.45):
    """Glatte Steinwand entlang ihrer X-Achse (Mitte unten bei `ort`) mit Öffnungen (von, bis, unten, oben)."""
    x0, y0, z0 = ort
    rot = Matrix.Rotation(math.radians(drehung_z), 4, "Z")

    def p(dx, dy, dz):
        v = rot @ Vector((dx, dy, dz))
        return (x0 + v.x, y0 + v.y, z0 + v.z)
    kanten = sorted({-laenge / 2, laenge / 2} | {l[0] for l in luecken} | {l[1] for l in luecken})
    for a, b in zip(kanten, kanten[1:]):
        m = (a + b) / 2
        offen = [l for l in luecken if l[0] <= m <= l[1]]
        for u, o in ([(0.0, hoehe)] if not offen else [(0.0, offen[0][2]), (offen[0][3], hoehe)]):
            if o - u > 0.03:
                w.brett("Steinwand", farbe_von, (b - a + 0.002, dicke, o - u), p(m, 0, (u + o) / 2), (0, 0, drehung_z), fase=0.0)


def pflasterweg(w, farbe_von, x0, x1, y0, y1, groesse=0.55):
    """Weg aus flachen, leicht verdrehten Pflastersteinen."""
    nx = max(1, round((x1 - x0) / groesse))
    ny = max(1, round((y1 - y0) / groesse))
    for i in range(nx):
        for j in range(ny):
            x = x0 + (i + 0.5 + (0.5 if j % 2 else 0.0) * 0.5) * (x1 - x0) / nx
            y = y0 + (j + 0.5) * (y1 - y0) / ny
            if x > x1 - 0.1:
                continue
            s = groesse * w.z.uniform(0.8, 0.95)
            w.brett("Pflaster", farbe_von, (s, s * w.z.uniform(0.8, 1.0), 0.14), (x, y, 0.03), (0, 0, w.z.uniform(-12, 12)), fase=0.0)


def zaun(w, punkte, hoehe=1.0, abstand=1.7):
    """Holzzaun entlang eines Linienzugs: Pfähle und zwei Rundholz-Latten je Feld."""
    pfahl = holzfarbe(w.z, "#7E5530", "#56381E")
    latte = stammfarbe(w.z, "#8A5E36", "#5E3D22", "#D8B078", "#A87C48")
    for i, (a, b) in enumerate(zip(punkte, punkte[1:])):
        a, b = Vector((a[0], a[1], 0)), Vector((b[0], b[1], 0))
        d = b - a
        n = max(1, math.ceil(d.length / abstand))
        rz = math.degrees(math.atan2(d.y, d.x))
        for k in range(0 if i == 0 else 1, n + 1):
            p = a + d * (k / n)
            w.brett("Zaunpfahl", pfahl, (0.13, 0.13, hoehe + 0.15), (p.x, p.y, (hoehe + 0.15) / 2 - 0.05),
                    (w.z.uniform(-3, 3), w.z.uniform(-3, 3), w.z.uniform(0, 90)), fase=0.02)
        for z in (hoehe * 0.42, hoehe * 0.85):
            for k in range(n):
                p = a + d * ((k + 0.5) / n)
                w.stamm("Zaunlatte", latte, 0.05, d.length / n + 0.25, (p.x, p.y, z + w.z.uniform(-0.03, 0.03)), (0, 0, rz), ecken=5)


def tanne(w, ort, hoehe=3.0):
    x, y = ort
    w.saeule("Tannenstamm", stammfarbe(w.z, "#6E4A2A", "#4E321A", "#C49A62", "#96703E"), 0.12, (x, y, -0.2), (x, y, hoehe * 0.35), ecken=6)
    for i, (r, z0, h, f) in enumerate(((0.36, 0.22, 0.42, "#3E7F45"), (0.28, 0.45, 0.36, "#4A9150"), (0.19, 0.66, 0.34, "#57A05B"))):
        w.spitze("Tanne", einfarbig(f, 0.05, w.z), r * hoehe, h * hoehe, (x, y, z0 * hoehe), ecken=7, drehung=(0, 0, w.z.uniform(0, 60)))


def busch(w, ort, groesse=0.6, blueten=None):
    x, y = ort
    gruen = einfarbig("#5FA04E", 0.07, w.z)
    for k in range(3):
        wi = math.tau * k / 3 + w.z.uniform(-0.4, 0.4)
        w.stein("Busch", gruen, groesse * w.z.uniform(0.7, 1.0), (x + math.cos(wi) * groesse * 0.5, y + math.sin(wi) * groesse * 0.5, groesse * 0.35), flach=0.8)
    if blueten:
        for k in range(5):
            wi = w.z.uniform(0, math.tau)
            w.stein("Blüte", einfarbig(blueten, 0.06, w.z), 0.09, (x + math.cos(wi) * groesse * 0.6, y + math.sin(wi) * groesse * 0.6, groesse * 0.75), flach=1.0)


def fass(w, ort, hoehe=0.9):
    w.teile.extend(_fass_teile(w.z, ort, hoehe))


def rad(w, ort, radius=0.6, speichen=10, rz=0.0):
    """Speichenrad (Achse entlang Y, um `rz` Grad gedreht) mit der Nabe bei `ort`."""
    m = Matrix.Translation(Vector(ort)) @ Matrix.Rotation(math.radians(rz), 4, "Z")
    for teil in _rad(w.z, radius, speichen):
        teil.data.transform(m)
        w.teile.append(teil)


def karren_gestell(w, holz, laenge=2.2, breite=1.3, radius=0.6):
    """Zweirädriger Wagen entlang X (Achse bei x=0), Deichsel nach +X auf dem Boden. Gibt die Ladehöhe zurück."""
    oben = radius + 0.2
    for y in (-breite / 2 - 0.08, breite / 2 + 0.08):
        rad(w, (0, y, radius), radius)
    w.saeule("Achse", einfarbig(EISEN, 0.04, w.z), 0.05, (0, -breite / 2 - 0.2, radius), (0, breite / 2 + 0.2, radius), ecken=6)
    for y in (-breite / 2 + 0.1, breite / 2 - 0.1):
        w.brett("Wagenbaum", holz, (laenge, 0.14, 0.16), (0, y, oben - 0.08), fase=0.02)
    for x in (-laenge / 2 + 0.15, laenge / 2 - 0.15):
        w.brett("Querbalken", holz, (0.16, breite, 0.14), (x, 0, oben), fase=0.02)
    w.saeule("Deichsel", holz, 0.06, (laenge / 2 - 0.2, 0, oben - 0.1), (laenge / 2 + 1.8, 0, 0.08), ecken=6)
    w.brett("Ortscheit", holz, (0.1, 0.9, 0.08), (laenge / 2 + 1.7, 0, 0.12), fase=0.01)
    return oben + 0.07


def brunnen(w):
    """Ziehbrunnen: gemauerter Ring, Wasser, Holzgestell mit Walze, Kurbel, Eimer und Schindeldach."""
    stein = steinfarbe(w, moos="#7BA84E")
    holz = holzfarbe(w.z, "#8E6238", "#5E3F22")
    dach = holzfarbe(w.z, "#B4523A", "#8A3A28", maserung=9.0)
    mauerring(w, stein, 0.95, 0.95, -0.1, 0.84, h=0.32, tiefe=0.34)
    w.dreh("Brunnenrand", stein, [(1.1, 0.84), (1.1, 0.98), (0.74, 0.98), (0.74, 0.9)], (0, 0, 0), ecken=14)
    w.dreh("Wasser", einfarbig("#4F8FC0", 0.04, w.z), [(0.78, 0.55), (0.0, 0.55)], (0, 0, 0), ecken=12)
    for x in (-0.92, 0.92):
        w.brett("Brunnenpfosten", holz, (0.15, 0.15, 2.0), (x, 0, 1.75), fase=0.02)
        for s in (-1, 1):
            w.brett("Kopfband", holz, (0.08, 0.5, 0.1), (x, s * 0.2, 2.45), (s * 45, 0, 0), fase=0.01)
    w.stamm("Walze", stammfarbe(w.z, "#8A5E36", "#5E3D22", "#D8B078", "#A87C48"), 0.13, 1.7, (0, 0, 1.75), ecken=8)
    w.brett("Kurbel", einfarbig(EISEN, 0.04, w.z), (0.06, 0.06, 0.45), (1.08, 0, 1.6), fase=0.0)
    w.brett("Kurbelgriff", holz, (0.3, 0.07, 0.07), (1.22, 0, 1.4), fase=0.0)
    w.saeule("Seil", einfarbig("#C8B58A", 0.03, w.z), 0.02, (0.2, 0, 1.65), (0.2, 0, 1.25), ecken=4)
    w.dreh("Eimer", holz, [(0.13, 0.95), (0.17, 1.25), (0.15, 1.25), (0.11, 0.97)], (0.2, 0, 0), ecken=8)
    for s in (-1, 1):
        for r in range(3):
            w.brett("Brunnendach", dach, (2.4, 0.5, 0.07), (0, s * (0.2 + r * 0.36), 3.05 - r * 0.25 + 0.02), (s * -35, 0, 0), fase=0.02)
    w.brett("Brunnenfirst", dach, (2.5, 0.2, 0.16), (0, 0, 3.12), fase=0.02)


def marktstand(w, tuch_a="#D94A3A", tuch_b="#F3E6C8"):
    """Marktstand mit gestreiftem Sonnendach und Ware (Äpfel, Brote, Kürbisse, Körbe)."""
    holz = holzfarbe(w.z, "#A87A48", "#7A5430")
    for x in (-1.15, 1.15):
        w.brett("Standpfosten", holz, (0.12, 0.12, 2.5), (x, 0.55, 1.25), fase=0.015)
        w.brett("Standpfosten", holz, (0.12, 0.12, 2.0), (x, -0.75, 1.0), fase=0.015)
    w.brett("Tisch", holz, (2.4, 1.0, 0.1), (0, -0.1, 0.9), fase=0.02)
    w.brett("Tischblende", holz, (2.4, 0.06, 0.5), (0, -0.6, 0.62), fase=0.01)
    neigung = math.degrees(math.atan2(0.5, 1.4))
    for i in range(6):
        w.brett("Sonnendach", einfarbig(tuch_a if i % 2 else tuch_b, 0.03, w.z), (0.44, 1.65, 0.04), (-1.1 + 0.22 + i * 0.44, -0.12, 2.28), (neigung, 0, 0), fase=0.0)
        w.brett("Volant", einfarbig(tuch_b if i % 2 else tuch_a, 0.03, w.z), (0.42, 0.03, 0.24), (-1.1 + 0.22 + i * 0.44, -0.95, 1.9), fase=0.0)
    ware = [("#D8342C", 0.09), ("#E8B04A", 0.1), ("#8BC34A", 0.09)]
    for j, (x, farbe_ware) in enumerate(((-0.75, 0), (0.0, 1), (0.75, 2))):
        w.brett("Korb", holzfarbe(w.z, "#C49A62", "#96703E"), (0.6, 0.5, 0.2), (x, -0.15, 1.05), fase=0.03)
        hexf, r = ware[farbe_ware]
        for k in range(4):
            w.stein("Ware", einfarbig(hexf, 0.06, w.z), r, (x + w.z.uniform(-0.2, 0.2), -0.15 + w.z.uniform(-0.15, 0.15), 1.2 + w.z.uniform(0, 0.06)), flach=0.9)
    for x in (-0.6, 0.6):
        w.dreh("Kürbis", einfarbig("#E8862A", 0.05, w.z), [(0.0, 0.0), (0.26, 0.08), (0.28, 0.2), (0.12, 0.34), (0.0, 0.34)], (x, -1.05, 0.0), ecken=8)
    _kiste(w, (1.55, 0.2, 0.0), 0.5)


# ---------------------------------------------------------------------------
# Holzfäller: Blockhütte auf Bruchsteinsockel, gemauerter Kamin, Veranda, Brennholzschuppen,
# Stammstapel, Holzwagen, Zaun, Tannen, Sägebock, Hackklotz und Schleifstein
# ---------------------------------------------------------------------------
def holzfaeller(seed=61):
    w = Werk(seed)
    teile, licht, zufall = w.teile, w.licht, w.z
    B, T = 6.2, 4.6
    boden = 0.45
    lage_h, lagen = 0.34, 8
    traufe = boden + lage_h * lagen + 0.05
    first = traufe + 2.0
    ueber = 0.55

    stein = steinfarbe(w, moos="#7BA84E")
    erde = einfarbig("#8C7A58", 0.06, zufall)
    _platte(teile, "Hof", erde, 7.6, 6.4, 0.04, zufall)
    sockel(w, stein, B + 0.7, T + 0.7, boden)

    # Blockwände: Längswände und Querwände abwechselnd um eine halbe Lage versetzt
    rinde = stammfarbe(zufall, "#9A6436", "#6A4222", "#E6BC80", "#B8864E")
    r = lage_h * 0.56
    tuer = (-0.65, 0.65, 6)
    fenster_seite = (-0.7, 0.7, 3, 6)
    fenster_vorne = (1.4, 2.4, 3, 6)
    for lage in range(lagen):
        z = boden + r + lage * lage_h
        for seite in (-1, 1):
            y = seite * T / 2
            lucken = []
            if seite == -1 and lage < tuer[2]:
                lucken.append((tuer[0], tuer[1]))
            if seite == -1 and fenster_vorne[2] <= lage < fenster_vorne[3]:
                lucken.append((fenster_vorne[0], fenster_vorne[1]))
            stuecke, start = [], -B / 2 - 0.35
            for a, b in sorted(lucken):
                stuecke.append((start, a))
                start = b
            stuecke.append((start, B / 2 + 0.35))
            for a, b in stuecke:
                if b - a > 0.2:
                    _stamm(teile, "Blockbalken", rinde, r * zufall.uniform(0.95, 1.05), b - a, ((a + b) / 2, y, z), seed=seed + lage * 13 + int(a * 10))
        if lage == lagen - 1:
            continue
        z2 = z + lage_h / 2
        for seite in (-1, 1):
            x = seite * B / 2
            stuecke = [(-T / 2 - 0.35, T / 2 + 0.35)]
            if fenster_seite[2] <= lage < fenster_seite[3]:
                stuecke = [(-T / 2 - 0.35, fenster_seite[0]), (fenster_seite[1], T / 2 + 0.35)]
            for a, b in stuecke:
                _stamm(teile, "Blockbalken", rinde, r * zufall.uniform(0.95, 1.05), b - a, (x, (a + b) / 2, z2), (0, 0, 90), seed=seed + lage * 17 + int(a * 10) + 5)
    _brett(teile, "Dielen", holzfarbe(zufall, "#A87A4A", "#7E5630"), (B - 0.2, T - 0.2, 0.06), (0, 0, boden + 0.03), fase=0.0)

    # Tür mit Beschlägen, Fenster mit Läden und Blumenkasten
    tuerholz = holzfarbe(zufall, "#7C4E2A", "#5A3618")
    eisen = einfarbig("#35312E", 0.05, zufall)
    tuer_h = lage_h * tuer[2] - 0.05
    for i in range(5):
        _brett(teile, "Türbrett", tuerholz, (0.25, 0.07, tuer_h), (tuer[0] + 0.14 + i * 0.26, -T / 2 - 0.02, boden + tuer_h / 2), fase=0.01)
    for dz in (0.35, tuer_h - 0.35):
        _brett(teile, "Beschlag", eisen, (1.2, 0.03, 0.08), (0, -T / 2 - 0.07, boden + dz), fase=0.0)
    w.brett("Türgriff", eisen, (0.05, 0.08, 0.2), (0.45, -T / 2 - 0.1, boden + 1.0), fase=0.0)
    _brett(teile, "Türsturz", tuerholz, (1.6, 0.3, 0.22), (0, -T / 2, boden + tuer_h + 0.12), fase=0.03)
    laden = einfarbig("#3E7A55", 0.04, zufall)
    rahmen = holzfarbe(zufall, "#C49A62", "#96703E")
    fz0 = boden + lage_h * fenster_vorne[2]
    fz1 = boden + lage_h * fenster_vorne[3]
    for (cx, cy, quer) in (((fenster_vorne[0] + fenster_vorne[1]) / 2, -T / 2, False), (-B / 2, 0.0, True), (B / 2, 0.0, True)):
        breite = 1.0 if not quer else fenster_seite[1] - fenster_seite[0]
        dreh = (0, 0, 90) if quer else (0, 0, 0)
        _brett(teile, "Fensterbank", rahmen, (breite + 0.3, 0.3, 0.08), (cx, cy, fz0), dreh, fase=0.01)
        _brett(teile, "Fenstersturz", rahmen, (breite + 0.3, 0.3, 0.1), (cx, cy, fz1), dreh, fase=0.01)
        w.brett("Scheibe", einfarbig("#FFC878", 0.02, zufall), (breite, 0.04, fz1 - fz0 - 0.05), (cx, cy, (fz0 + fz1) / 2), dreh, fase=0.0, glut=True)
        _brett(teile, "Sprosse", rahmen, (0.05, 0.06, fz1 - fz0), (cx, cy + (0.03 if quer else -0.03), (fz0 + fz1) / 2), dreh, fase=0.0)
        for s in (-1, 1):
            if quer:
                ort = (cx + math.copysign(0.24, cx), cy + s * (breite / 2 + 0.3), (fz0 + fz1) / 2)
            else:
                ort = (cx + s * (breite / 2 + 0.3), cy - 0.24, (fz0 + fz1) / 2)
            _brett(teile, "Laden", laden, (0.5, 0.05, fz1 - fz0 - 0.08), ort, dreh, fase=0.012)
        if not quer:
            w.brett("Blumenkasten", rahmen, (breite + 0.2, 0.28, 0.22), (cx, cy - 0.36, fz0 - 0.16), fase=0.015)
            for k in range(4):
                w.stein("Blüte", einfarbig(("#E24A4A", "#F2C94C", "#E88AD0", "#6FA8E8")[k], 0.05, zufall), 0.11, (cx - 0.4 + k * 0.27, cy - 0.36, fz0 + 0.0), flach=1.0)

    # Dach: Giebel, Schindeln in versetzten Reihen, Windbretter, Sparrenköpfe
    giebelholz = holzfarbe(zufall, "#B07A44", "#86582E")
    for x in (-B / 2, B / 2):
        _giebel(teile, giebelholz, T, x, traufe - 0.05, first - 0.15)
    schindel = holzfarbe(zufall, "#B4523A", "#8A3A28", maserung=9.0)
    ziegeldach(w, schindel, B, T, traufe, first, ueber, reihen=7, stuecke=4)
    dunkel = holzfarbe(zufall, "#6E4A2A", "#4E321A")
    for x in (-B / 2 - ueber, B / 2 + ueber):
        windbretter(w, dunkel, x, T, traufe, first, ueber)
    sparren(w, dunkel, B, T, traufe, first, ueber, abstand=1.05)

    # Gemauerter Kamin außen am linken Giebel, unten breiter, oben schlanker
    kx, ky = -B / 2 - 0.9, 1.3
    mauerblock(w, stein, 1.3, 1.2, -0.3, 2.3, h=0.43, tiefe=0.36, mitte=(kx, ky))
    w.brett("Kaminschulter", stein, (1.38, 1.28, 0.22), (kx, ky, 2.4), (0, -8, 0), fase=0.04)
    quaderblock(w, stein, 0.6, 0.56, 2.5, first + 1.1, mitte=(kx - 0.1, ky), h=0.43, laenge=0.9, tiefe=0.3, fase=0.03)
    w.brett("Kaminkrone", stein, (1.12, 1.08, 0.2), (kx - 0.1, ky, first + 1.2), fase=0.04)
    w.brett("Kaminloch", einfarbig("#1E1A18", 0.02, zufall), (0.5, 0.46, 0.05), (kx - 0.1, ky, first + 1.31), fase=0.0)

    # Veranda: Dielen, zwei Pfosten mit Kopfbändern, Pfette und Schindel-Vordach
    diele = holzfarbe(zufall, "#B8864E", "#8E6334")
    for i in range(6):
        _brett(teile, "Verandadiele", diele, (3.0, 0.24, 0.07), (-0.6, -T / 2 - 0.4 - i * 0.25, boden - 0.05), (0, 0, zufall.uniform(-0.5, 0.5)), fase=0.01)
    for x in (-1.9, 0.7):
        _brett(teile, "Verandapfosten", giebelholz, (0.16, 0.16, 2.2), (x, -T / 2 - 1.6, boden + 1.1), fase=0.02)
        for s in (-1, 1):
            w.brett("Kopfband", giebelholz, (0.55, 0.09, 0.1), (x + s * 0.2, -T / 2 - 1.6, boden + 1.95), (0, s * 45, 0), fase=0.01)
    w.brett("Pfette", giebelholz, (3.2, 0.16, 0.18), (-0.6, -T / 2 - 1.6, boden + 2.25), fase=0.02)
    for i in range(4):
        _brett(teile, "Vordach", schindel, (3.1, 0.42, 0.06), (-0.6, -T / 2 - 0.2 - i * 0.4, boden + 2.6 - i * 0.12), (-17, 0, 0), fase=0.012)
    _laterne(teile, licht, zufall, (1.25, -T / 2 - 1.7, 0.0), hoehe=2.0)
    # Zweimannsäge und Axt an der Wand neben der Tür
    stahl = einfarbig("#B8BEC6", 0.03, zufall)
    w.brett("Wandsäge", stahl, (1.4, 0.02, 0.2), (-2.0, -T / 2 - 0.25, boden + 1.75), fase=0.0)
    for s in (-1, 1):
        w.brett("Sägegriff", tuerholz, (0.06, 0.06, 0.34), (-2.0 + s * 0.74, -T / 2 - 0.26, boden + 1.75), fase=0.01)
    w.brett("Wandaxtstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.05, 0.05, 0.8), (-1.25, -T / 2 - 0.26, boden + 1.2), (0, 20, 0), fase=0.0)
    w.brett("Wandaxt", stahl, (0.24, 0.03, 0.18), (-1.12, -T / 2 - 0.27, boden + 1.55), (0, 20, 0), fase=0.0)

    # Brennholzschuppen rechts an der Hütte
    sx0, sx1 = B / 2 + 0.35, B / 2 + 2.3
    for y in (-T / 2 + 0.2, T / 2 - 0.2):
        _brett(teile, "Schuppenpfosten", giebelholz, (0.14, 0.14, 2.3), (sx1 - 0.1, y, 1.15), fase=0.02)
    w.brett("Schuppenpfette", giebelholz, (0.16, T, 0.16), (sx1 - 0.1, 0, 2.3), fase=0.02)
    for i in range(6):
        y = -T / 2 - 0.1 + i * (T + 0.2) / 5.5
        _brett(teile, "Pultdach", schindel, (2.3, (T + 0.3) / 5.2, 0.06), ((sx0 + sx1) / 2 + 0.1, y + 0.4, 2.5), (0, 12, 0), fase=0.012)
    scheit = stammfarbe(zufall, "#8A5A30", "#5A3A1C", "#E8C48C", "#C49A62")
    for lage in range(6):
        for i in range(9):
            y = -T / 2 + 0.35 + i * 0.48 + (0.24 if lage % 2 else 0)
            if y > T / 2 - 0.2:
                continue
            _stamm(teile, "Scheit", scheit, 0.11 * zufall.uniform(0.9, 1.1), 1.3, ((sx0 + sx1) / 2 - 0.1, y, 0.13 + lage * 0.2), (0, 0, zufall.uniform(-4, 4)), ecken=6, seed=seed + lage * 31 + i)

    # Stammstapel zwischen Pflöcken
    stamm = stammfarbe(zufall, "#7A5230", "#4E3218", "#E0B070", "#B08048")
    for lage, (anzahl, versatz) in enumerate(((4, 0.0), (3, 0.5), (2, 1.0))):
        for i in range(anzahl):
            _stamm(teile, "Stamm", stamm, 0.28, 3.4 + zufall.uniform(-0.2, 0.2),
                   (-B / 2 - 1.9 + (i + versatz * 0.5) * 0.58 - 0.9, -1.2, 0.28 + lage * 0.5), (0, 0, 90 + zufall.uniform(-3, 3)), ecken=11, seed=seed + 200 + lage * 7 + i)
    for y in (-2.9, 0.5):
        _brett(teile, "Keil", giebelholz, (2.2, 0.14, 0.14), (-B / 2 - 2.4, y, 0.07), fase=0.02)
    for x in (-B / 2 - 3.3, -B / 2 - 0.95):
        w.brett("Pflock", giebelholz, (0.12, 0.12, 1.5), (x, -1.2, 0.7), (0, w.z.uniform(-4, 4), 0), fase=0.015)

    # Sägebock mit Stamm und Säge, Hackklotz mit Axt, Spaltscheite
    bock = holzfarbe(zufall, "#9C6E3C", "#724C26")
    for y in (-3.4, -2.4):
        for wi in (35, -35):
            _brett(teile, "Bockbein", bock, (0.1, 0.1, 1.2), (2.9, y, 0.5), (wi, 0, 0), fase=0.01)
    _stamm(teile, "Sägestamm", stamm, 0.22, 2.2, (2.9, -2.9, 0.95), (0, 0, 90), ecken=10, seed=seed + 300)
    _brett(teile, "Sägeblatt", stahl, (0.03, 1.1, 0.16), (2.9, -2.1, 1.15), (0, 0, 8), fase=0.0)
    _brett(teile, "Sägegriff", bock, (0.06, 0.1, 0.3), (2.93, -1.52, 1.2), (0, 0, 8), fase=0.01)
    bm = stamm_bm(0.34, 0.55, ecken=14, seed=seed + 400)
    setzen(bm, (0, 0, 0), (0, -90, 0))
    setzen(bm, (1.6, -4.3, 0))
    teile.append(objekt("Hackklotz", bm, stamm))
    _brett(teile, "Axtstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.75, 0.05, 0.05), (1.82, -4.3, 0.84), (0, -38, 0), fase=0.008)
    _brett(teile, "Axtkopf", einfarbig("#8E959E", 0.04, zufall), (0.16, 0.035, 0.22), (1.55, -4.3, 0.62), (0, -38, 0), fase=0.006)
    for i in range(7):
        _stamm(teile, "Spaltscheit", scheit, 0.09, 0.45, (1.0 + zufall.uniform(-0.6, 0.6), -4.6 + zufall.uniform(-0.5, 0.4), 0.09), (0, 0, zufall.uniform(0, 360)), ecken=6, seed=seed + 500 + i)
    # Baumstumpf mit eingeschlagener Axt
    bm = stamm_bm(0.42, 0.45, ecken=12, seed=seed + 410, knorrig=0.1)
    setzen(bm, (0, 0, 0), (0, -90, 0))
    setzen(bm, (-1.4, -5.1, -0.05))
    teile.append(objekt("Baumstumpf", bm, stamm))
    w.brett("Stumpfaxtstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.7, 0.05, 0.05), (-1.15, -5.1, 0.62), (0, -30, 20), fase=0.008)
    w.brett("Stumpfaxt", einfarbig("#8E959E", 0.04, zufall), (0.2, 0.035, 0.2), (-1.42, -5.2, 0.45), (0, -30, 20), fase=0.006)

    # Schleifstein auf Holzbock mit Wassertrog
    gx, gy = 4.3, -4.3
    for s in (-1, 1):
        w.brett("Schleifbock", bock, (0.12, 0.12, 0.9), (gx, gy + s * 0.3, 0.45), fase=0.015)
    w.saeule("Schleifachse", eisen, 0.03, (gx, gy - 0.4, 0.82), (gx, gy + 0.55, 0.82), ecken=5)
    w.dreh("Schleifstein", _steinfarbe(zufall, farbe("#8E887E"), farbe("#B0A99C"), farbe("#CFC8BA"), farbe("#E0DAD0")),
           [(0.45, -0.09), (0.45, 0.09)], (gx, gy, 0.82), ecken=14, drehung=(90, 0, 0))
    w.brett("Kurbelarm", eisen, (0.04, 0.04, 0.3), (gx, gy + 0.58, 0.7), fase=0.0)
    w.brett("Schleiftrog", bock, (0.9, 0.5, 0.3), (gx, gy, 0.15), fase=0.02)
    fass(w, (-3.4, -2.8, 0.0), 0.95)
    w.dreh("Wasser", einfarbig("#4F8FC0", 0.04, zufall), [(0.3, 0.9), (0.0, 0.9)], (-3.4, -2.8, 0), ecken=10)

    # Holzwagen mit Stämmen hinter der Hütte
    def holzwagen(t):
        oben = karren_gestell(t, holzfarbe(t.z, "#8E6238", "#5E3F22"), laenge=2.2, breite=1.4, radius=0.62)
        for x in (-0.95, 0.95):
            for y in (-0.72, 0.72):
                t.brett("Runge", holzfarbe(t.z, "#8E6238", "#5E3F22"), (0.08, 0.08, 0.8), (x, y, oben + 0.35), fase=0.01)
        for (y, z) in ((-0.4, 0.2), (0.0, 0.2), (0.4, 0.2), (-0.2, 0.55), (0.2, 0.55)):
            t.stamm("Wagenstamm", stamm, 0.2, 3.5 + t.z.uniform(-0.2, 0.2), (t.z.uniform(-0.15, 0.15), y, oben + z), (0, 0, t.z.uniform(-2, 2)), ecken=9)
        t.brett("Kette", einfarbig(EISEN, 0.04, t.z), (0.05, 1.2, 0.05), (0.6, 0, oben + 0.78), fase=0.0)
    setze(w, (1.9, 3.95, 0.0), 0, holzwagen)

    # Zaun hinten links, junge Tannen, Büsche
    zaun(w, [(-6.3, -3.4), (-6.7, 0.8), (-5.9, 3.8), (-3.4, 5.4), (0.2, 5.9)])
    tanne(w, (-1.3, 4.5), 3.6)
    tanne(w, (-3.1, 4.3), 2.7)
    tanne(w, (6.4, -2.4), 3.1)
    busch(w, (6.0, -4.6), 0.55, "#F2C94C")
    busch(w, (-5.6, 3.3), 0.5)

    # Schild: gekreuzte Äxte
    _schild(teile, zufall, (-3.6, -4.9, 0.0), 0, [
        (-0.05, 0.0, 0.5, 0.05, 40, "#7A4A26"), (0.05, 0.0, 0.5, 0.05, -40, "#7A4A26"),
        (-0.2, 0.15, 0.14, 0.12, 40, "#5C636C"), (0.2, 0.15, 0.14, 0.12, -40, "#5C636C"),
    ])
    return fertig(w, "Holzfaeller")


# ---------------------------------------------------------------------------
# Steinbruch: Felswand mit halb gelösten Blöcken, Tretradkran, Stützmauern, Leiter,
# Werkstatt mit Steinwand, Quader auf Paletten, Blockwagen, Säulentrommeln und Mühlstein
# ---------------------------------------------------------------------------
def steinbruch(seed=62):
    w = Werk(seed)
    teile, licht, zufall = w.teile, w.licht, w.z
    fels = _steinfarbe(zufall, farbe("#6E6A63"), farbe("#9A948A"), farbe("#C6BFB2"), farbe("#DDD6C8"))
    block = _steinfarbe(zufall, farbe("#9E978B"), farbe("#BDB6A9"), farbe("#DAD3C5"), farbe("#EDE7DA"), schicht=6.0)
    kies = einfarbig("#A7A092", 0.07, zufall)
    _platte(teile, "Kiesplatz", kies, 8.2, 7.0, 0.05, zufall)

    # Felswand hinten (+Y): große Brocken, vorne senkrecht abgeschnitten
    for i, (x, y, rad_, hoch) in enumerate(((-4.2, 3.6, 2.2, 1.5), (-1.3, 4.3, 2.6, 1.8), (1.9, 4.0, 2.4, 1.6), (4.6, 3.3, 1.9, 1.3), (0.2, 5.4, 2.3, 1.9))):
        bm = _brocken_bm(zufall, rad_, (1.1, 0.9, hoch), 11, fase=0.03)
        bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=Vector((0, -rad_ * 0.35, 0)), plane_no=Vector((0, -1, 0)), clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
        bmesh.ops.triangulate(bm, faces=bm.faces[:])
        _setzen(bm, (x, y, rad_ * hoch * 0.55 - 0.6), zufall.uniform(-0.2, 0.2))
        teile.append(objekt(f"Fels{i}", bm, fels))
    # Stufen im Fels: halb herausgelöste Blöcke, Keile an der Kante
    for i in range(7):
        x = -4.5 + i * 1.5 + zufall.uniform(-0.2, 0.2)
        stufe = i % 3
        _brett(teile, "Rohblock", block, (1.2, 0.9, 0.8), (x, 2.3 + stufe * 0.55, 0.4 + stufe * 0.7), (0, 0, zufall.uniform(-4, 4)), fase=0.05)
    eisen = einfarbig("#3C3936", 0.05, zufall)
    for i in range(5):
        _brett(teile, "Keil", eisen, (0.08, 0.12, 0.2), (-2.4 + i * 0.45, 1.84, 1.35), fase=0.0)
    for i in range(4):
        w.brett("Bohrloch", einfarbig("#2A2624", 0.02, zufall), (0.06, 0.05, 0.5), (1.55 + i * 0.3, 2.83, 1.55), fase=0.0)

    holz = holzfarbe(zufall, "#A6773F", "#7C552A")
    dunkel = holzfarbe(zufall, "#7A5230", "#553820")
    rundholz = stammfarbe(zufall, "#8A5A30", "#5E3D22", "#E0B878", "#B08650")
    seil = einfarbig("#C8B58A", 0.03, zufall)

    # Stützmauern aus Quadern links und rechts der Abbausohle
    quadermauer(w, block, (-6.3, -0.9), (-6.0, 2.7), 0.0, 1.3, tiefe=0.8, h=0.44, laenge=1.2)
    quadermauer(w, block, (5.8, -0.3), (5.5, 2.4), 0.0, 1.1, tiefe=0.8, h=0.37, laenge=1.2)
    for (x, y) in ((-6.3, -0.9), (5.8, -0.3)):
        w.brett("Mauerkopf", block, (1.0, 1.0, 0.25), (x, y, 1.35 if x < 0 else 1.15), (0, 0, zufall.uniform(-5, 5)), fase=0.04)

    # Leiter an der Felswand
    lx = -0.7
    for s in (-0.26, 0.26):
        w.saeule("Leiterholm", holz, 0.045, (lx + s, 1.3, 0.0), (lx + s, 2.4, 3.4), ecken=5)
    for i in range(8):
        t = (i + 0.7) / 9
        w.brett("Sprosse", holz, (0.55, 0.05, 0.05), (lx, 1.3 + 1.1 * t, 3.4 * t), fase=0.0)

    # Tretradkran: Rad in zwei A-Böcken, Mast mit Ausleger, Seil über Rollen zum hängenden Block
    mx, my = 3.3, 0.6
    rr, rz_ = 1.5, 1.75
    for y in (my - 0.5, my + 0.5):
        for k in range(16):
            a = math.tau * (k + 0.5) / 16
            w.brett("Radkranz", holz, (2 * rr * math.sin(math.pi / 16) * 1.04, 0.1, 0.16), (mx + math.cos(a) * rr, y, rz_ + math.sin(a) * rr),
                    (0, 90 - math.degrees(a), 0), fase=0.015)
        for k in range(4):
            w.brett("Radspeiche", dunkel, (2 * rr - 0.1, 0.08, 0.08), (mx, y, rz_), (0, 45 * k, 0), fase=0.01)
    for k in range(16):
        a = math.tau * k / 16
        w.brett("Tritt", holz, (0.24, 1.06, 0.05), (mx + math.cos(a) * (rr - 0.1), my, rz_ + math.sin(a) * (rr - 0.1)), (0, 90 - math.degrees(a), 0), fase=0.01)
    w.saeule("Radwelle", rundholz, 0.12, (mx, my - 0.95, rz_), (mx, my + 0.95, rz_), ecken=8)
    for y in (my - 0.85, my + 0.85):
        for s in (-1, 1):
            w.saeule("Bockbein", dunkel, 0.08, (mx + s * 1.25, y, 0.0), (mx + s * 0.08, y, rz_ + 0.12), ecken=6)
        w.brett("Bockriegel", dunkel, (1.5, 0.1, 0.12), (mx, y, 0.75), fase=0.01)
    for s in (-1, 1):
        w.brett("Schwelle", dunkel, (0.2, 2.0, 0.18), (mx + s * 1.25, my, 0.09), fase=0.02)
    # Mast mit Streben, Ausleger zur Felswand, Rollen
    ax, ay = mx - 2.2, my
    w.stamm("Mast", rundholz, 0.19, 6.0, (ax, ay, 3.0), (0, 90, 0), ecken=9)
    for k in range(4):
        a = math.tau * k / 4 + math.pi / 4
        w.saeule("Maststrebe", holz, 0.06, (ax + math.cos(a) * 1.1, ay + math.sin(a) * 1.1, 0.0), (ax, ay, 1.9), ecken=5)
    spitze = Vector((-1.4, -0.5, 6.4))
    w.saeule("Ausleger", rundholz, 0.14, (ax, ay, 5.3), tuple(spitze), ecken=8, radius_ende=0.1)
    w.saeule("Auslegerstrebe", holz, 0.07, (ax, ay, 3.7), tuple(Vector((ax, ay, 5.3)).lerp(spitze, 0.5)), ecken=5)
    for p in ((ax, ay, 6.05), tuple(spitze)):
        w.dreh("Rolle", eisen, [(0.16, -0.05), (0.16, 0.05)], p, ecken=10, drehung=(90, 0, 0))
    w.saeule("Kranseil", seil, 0.025, (mx, my, rz_ + 0.12), (ax, ay, 6.05), ecken=4)
    w.saeule("Kranseil", seil, 0.025, (ax, ay, 6.05), tuple(spitze), ecken=4)
    w.saeule("Kranseil", seil, 0.025, tuple(spitze), (spitze.x, spitze.y, 3.0), ecken=4)
    w.brett("Kranhaken", eisen, (0.08, 0.08, 0.3), (spitze.x, spitze.y, 2.9), fase=0.0)
    for s in (-1, 1):
        w.saeule("Schlinge", seil, 0.02, (spitze.x, spitze.y, 2.8), (spitze.x + s * 0.38, spitze.y, 2.45), ecken=4)
    _brett(teile, "Hängeblock", block, (0.85, 0.75, 0.75), (spitze.x, spitze.y, 2.05), (0, 0, 20), fase=0.05)

    # Quader auf Paletten
    for (px, py, reihen) in ((-3.8, -1.9, 3), (-1.9, -2.6, 2), (-4.4, -4.1, 2)):
        for i in range(3):
            _brett(teile, "Palette", dunkel, (1.9, 0.18, 0.12), (px, py - 0.55 + i * 0.55, 0.06), fase=0.01)
        for lage in range(reihen):
            for i in range(2):
                for j in range(2):
                    if lage == reihen - 1 and (i + j) % 2 and reihen > 2:
                        continue
                    _brett(teile, "Quader", block, (0.82, 0.55, 0.5), (px - 0.43 + i * 0.86, py - 0.29 + j * 0.58, 0.37 + lage * 0.52), (0, 0, zufall.uniform(-2, 2)), fase=0.04)

    # Werkstatt: Steinwand hinten, Pfosten, Schindel-Pultdach, Werkbank, Werkzeug, Laterne
    wx, wy = 3.4, -3.0
    quadermauer(w, block, (wx - 1.7, wy + 1.25), (wx + 1.7, wy + 1.25), 0.0, 2.6, tiefe=0.5, h=0.52, laenge=1.15)
    for dx in (-1.5, 1.5):
        w.brett("Pfosten", holz, (0.16, 0.16, 2.4), (wx + dx, wy - 1.1, 1.2), fase=0.02)
        w.brett("Kopfband", holz, (0.5, 0.09, 0.1), (wx + dx - math.copysign(0.18, dx), wy - 1.1, 2.15), (0, math.copysign(45, dx), 0), fase=0.01)
    w.brett("Pfette", holz, (3.3, 0.16, 0.18), (wx, wy - 1.1, 2.45), fase=0.02)
    dach = holzfarbe(zufall, "#B4523A", "#8A3A28", maserung=9.0)
    for i in range(6):
        for s in range(2):
            w.brett("Dachschindel", dach, (1.85, 0.52, 0.06), (wx - 0.9 + s * 1.8 + (0.3 if i % 2 else 0) - 0.15, wy - 1.45 + i * 0.5, 2.5 + i * 0.1), (11, 0, zufall.uniform(-1, 1)), fase=0.015)
    _brett(teile, "Werkbank", holz, (2.2, 0.8, 0.12), (wx, wy + 0.5, 0.95), fase=0.02)
    for dx in (-0.95, 0.95):
        for dy in (0.2, 0.8):
            _brett(teile, "Bankbein", dunkel, (0.1, 0.1, 0.9), (wx + dx, wy + dy, 0.45), fase=0.01)
    _brett(teile, "Werkstück", block, (0.6, 0.45, 0.4), (wx - 0.3, wy + 0.5, 1.21), (0, 0, 12), fase=0.04)
    _brett(teile, "Hammerstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.45, 0.04, 0.04), (wx + 0.5, wy + 0.35, 1.04), (0, 0, 30), fase=0.0)
    _brett(teile, "Hammerkopf", eisen, (0.08, 0.16, 0.08), (wx + 0.3, wy + 0.24, 1.06), (0, 0, 30), fase=0.0)
    _brett(teile, "Meißel", eisen, (0.22, 0.03, 0.03), (wx + 0.8, wy + 0.7, 1.03), (0, 0, -20), fase=0.0)
    for i in range(4):
        w.brett("Wandwerkzeug", eisen, (0.05, 0.03, 0.5), (wx - 1.0 + i * 0.3, wy + 0.97, 1.7), (0, 0, 0), fase=0.0)
    _laterne(teile, licht, zufall, (wx - 1.5, wy - 1.4, 0.0), hoehe=2.2)

    # Fertige Werkstücke: Säulentrommeln, Kapitell und ein Mühlstein
    fx, fy = -5.5, -0.4
    for i, (dz, r) in enumerate(((0.0, 0.42), (0.72, 0.42))):
        w.dreh("Säulentrommel", block, [(r, dz), (r, dz + 0.7)], (fx, fy, 0.0), ecken=12, drehung=(0, 0, i * 15))
    w.brett("Kapitell", block, (1.0, 1.0, 0.3), (fx, fy, 1.57), (0, 0, 10), fase=0.05)
    w.dreh("Säulentrommel", block, [(0.42, -0.35), (0.42, 0.35)], (fx + 0.9, fy - 1.3, 0.4), ecken=12, drehung=(90, 0, 70))
    w.dreh("Mühlstein", block, [(0.62, -0.13), (0.62, 0.13), (0.16, 0.13), (0.16, -0.13)], (5.6, -3.6, 0.62), ecken=16, drehung=(80, 0, -60))
    _kiste(w, (5.9, -2.6, 0.0), 0.6)

    # Blockwagen mit großem Quader
    def blockwagen(t):
        oben = karren_gestell(t, dunkel, laenge=2.2, breite=1.2, radius=0.55)
        for i in range(6):
            t.brett("Ladebrett", holz, (0.36, 1.35, 0.06), (-0.95 + i * 0.38, 0, oben), fase=0.01)
        t.brett("Ladung", block, (1.25, 0.95, 0.85), (0.0, 0, oben + 0.45), (0, 0, 4), fase=0.06)
        t.brett("Spanngurt", seil, (0.06, 1.0, 0.9), (0.15, 0, oben + 0.45), (0, 0, 4), fase=0.0)
    setze(w, (0.9, -5.5, 0.0), 180, blockwagen)

    # Schubkarre mit Bruchsteinen
    kx, ky = 0.2, -3.6
    _brett(teile, "Karrenboden", dunkel, (1.1, 0.7, 0.08), (kx, ky, 0.55), (0, -8, 20), fase=0.01)
    for s in (-1, 1):
        _brett(teile, "Karrenwand", holz, (1.1, 0.06, 0.3), (kx, ky + s * 0.33, 0.7), (0, -8, 20), fase=0.01)
        _brett(teile, "Holm", holz, (1.2, 0.06, 0.06), (kx - 0.9, ky + s * 0.28 - 0.3, 0.55), (0, -12, 20), fase=0.0)
    bm = stamm_bm(0.26, 0.08, ecken=12, seed=seed + 9)
    setzen(bm, (-0.04, 0, 0), (0, 0, 110))
    setzen(bm, (kx + 0.62, ky + 0.22, 0.26))
    teile.append(objekt("Rad", bm, dunkel))
    for i in range(6):
        bm = stein_bm(zufall, zufall.uniform(0.12, 0.18))
        setzen(bm, (kx + zufall.uniform(-0.35, 0.35), ky + zufall.uniform(-0.2, 0.2), 0.75))
        teile.append(objekt("Bruchstein", bm, fels))
    teile += _splitter(zufall, fels, 26, 6.5, (0.08, 0.2))
    busch(w, (-6.6, -2.6), 0.5, "#F2C94C")
    busch(w, (6.6, 1.4), 0.45)
    _schild(teile, zufall, (-6.0, -3.6, 0.0), 0, [
        (0.0, 0.05, 0.36, 0.22, 0, "#8E8A82"), (-0.1, 0.05, 0.04, 0.22, 0, "#6A665F"),
        (0.2, -0.1, 0.35, 0.05, 45, "#6E4A2A"), (0.08, 0.02, 0.16, 0.1, 45, "#4A4642"),
    ])
    return fertig(w, "Steinbruch")


# ---------------------------------------------------------------------------
# Erzmine: Felshügel mit gemauertem Stollenportal, Förderturm über einem Schacht,
# Schienen mit Prellbock, Lore voller Erz, Rennofen mit Blasebalg und Eisenbarren
# ---------------------------------------------------------------------------
def erzmine(seed=63):
    w = Werk(seed)
    teile, licht, zufall = w.teile, w.licht, w.z
    fels = _steinfarbe(zufall, farbe("#5A544E"), farbe("#7E766C"), farbe("#A89E90"), farbe("#C4BAAC"),
                       moos=farbe("#6F9A48"), moos_rauschen=lambda p: 0.5 + 0.5 * math.sin(p.x * 1.3 + p.y * 0.7))
    erz = _steinfarbe(zufall, farbe("#6A3322"), farbe("#9A4A2C"), farbe("#C8683A"), farbe("#E08E5A"))
    quader = steinfarbe(w, "#6F6A62", "#9A938A", "#C2BAAE", "#DAD2C6")
    erde = einfarbig("#80705A", 0.06, zufall)
    _platte(teile, "Vorplatz", erde, 7.6, 6.8, 0.05, zufall)

    # Hügel aus Felsbrocken hinter dem Eingang, mit Erzadern
    for i, (x, y, r, hoch) in enumerate(((0.0, 3.6, 3.2, 1.25), (-3.4, 2.8, 2.3, 1.0), (3.3, 3.2, 2.4, 1.05), (-1.8, 5.2, 2.6, 1.2),
                                         (2.2, 5.4, 2.4, 1.1), (-4.8, 4.8, 1.8, 0.9), (4.9, 4.6, 1.7, 0.85))):
        bm = _brocken_bm(zufall, r, (1.05, 0.95, hoch), 10, fase=0.03)
        _setzen(bm, (x, y, r * hoch * 0.5 - 0.7), zufall.uniform(0, math.tau))
        teile.append(objekt(f"Hügel{i}", bm, fels))
    for i in range(8):
        wi = zufall.uniform(-1.2, 1.2)
        bm = stein_bm(zufall, zufall.uniform(0.35, 0.55), flach=0.7)
        _setzen(bm, (math.sin(wi) * 2.9, 3.6 - math.cos(wi) * 2.3, zufall.uniform(1.4, 2.8)), zufall.uniform(0, math.tau))
        teile.append(objekt("Erzader", bm, erz))

    # Stollen: dunkle Öffnung, Holzausbau, davor ein gemauertes Rundbogenportal mit Flügelmauern
    holz = stammfarbe(zufall, "#7C5230", "#523418", "#D8A868", "#A87C48")
    bretter = holzfarbe(zufall, "#8E6238", "#5E3F22")
    eisen = einfarbig("#4A4A4E", 0.04, zufall)
    _brett(teile, "Stollen", einfarbig("#140F0C", 0.02, zufall), (2.5, 1.8, 3.3), (0, 1.75, 1.6), fase=0.0)
    for x in (-1.1, 1.1):
        _stamm(teile, "Stempel", holz, 0.16, 2.8, (x, 1.0, 1.35), (0, -90, 0), ecken=8, seed=seed + int(x * 10))
    _stamm(teile, "Kappe", holz, 0.18, 2.7, (0, 1.0, 2.75), (0, 0, 0), ecken=8, seed=seed + 20)
    pz, pr, py = 2.35, 1.35, 0.55
    for s in (-1, 1):
        eckquader(w, quader, (s * (pr + 0.62), py - 0.42), (-s, 0), (0, 1), 0.0, pz, h=0.47, lang=0.62, tiefe=0.7, vor=0.0)
        w.brett("Kämpfer", quader, (0.8, 0.95, 0.2), (s * (pr + 0.3), py, pz + 0.05), fase=0.03)
        quadermauer(w, quader, (s * (pr + 0.7), py + 0.1), (s * (pr + 2.6), py + 0.7), 0.0, 1.7, tiefe=0.75, h=0.43, laenge=1.0)
        w.brett("Mauerkrone", quader, (2.1, 0.9, 0.18), (s * (pr + 1.65), py + 0.4, 1.78), (0, 0, s * math.degrees(math.atan2(0.6, 1.9))), fase=0.03)
    for k in range(9):
        a = math.pi * (k + 0.5) / 9
        r = pr + 0.3
        schluss = k == 4
        w.brett("Schlussstein" if schluss else "Bogenstein", quader, (r * math.pi / 9 * 0.93, 0.9 if not schluss else 1.0, 0.62 if not schluss else 0.78),
                (math.cos(a) * r, py, pz + 0.12 + math.sin(a) * r), (0, 90 - math.degrees(a), 0), fase=0.04)
    w.brett("Bogenfeld", bretter, (2.3, 0.1, 0.65), (0, py + 0.35, 3.28), fase=0.01)
    w.brett("Stollenschild", einfarbig("#E9D9B0", 0.03, zufall), (1.1, 0.06, 0.34), (0, py + 0.26, 3.2), fase=0.015)
    for s in (-1, 1):
        w.brett("Schlägel", einfarbig("#5A3A20", 0.03, zufall), (0.4, 0.03, 0.06), (s * 0.12, py + 0.22, 3.2), (0, s * 40, 0), fase=0.0)
    w.brett("Wappen", einfarbig(GOLD, 0.03, zufall), (0.36, 0.08, 0.36), (0, py - 0.55, pz + 0.12 + pr + 0.3), (0, 45, 0), fase=0.02)
    for x in (-2.2, 2.2):
        _laterne(teile, licht, zufall, (x, -0.35, 0.0), hoehe=2.3)

    # Schienen mit Schwellen und Prellbock
    for s in (-0.42, 0.42):
        _brett(teile, "Schiene", eisen, (0.07, 6.6, 0.09), (s, -1.6, 0.2), fase=0.0)
    schwelle = holzfarbe(zufall, "#6E4A2A", "#4E321A")
    for i in range(13):
        _brett(teile, "Schwelle", schwelle, (1.3, 0.22, 0.12), (0, 1.5 - i * 0.52, 0.1), (0, 0, zufall.uniform(-3, 3)), fase=0.01)
    for s in (-0.5, 0.5):
        w.brett("Prellbockpfosten", schwelle, (0.2, 0.2, 0.9), (s, -4.95, 0.45), fase=0.02)
    w.brett("Prellbock", bretter, (1.4, 0.24, 0.26), (0, -5.0, 0.72), fase=0.02)
    # Lore voller Erz
    ly = -2.2
    lore = einfarbig("#6B5E52", 0.04, zufall)
    _brett(teile, "Loreboden", lore, (1.0, 1.3, 0.1), (0, ly, 0.52), fase=0.01)
    for s in (-1, 1):
        _brett(teile, "Lorewand", lore, (0.08, 1.4, 0.6), (s * 0.55, ly, 0.82), (0, s * 10, 0), fase=0.01)
        _brett(teile, "Lorestirn", lore, (1.2, 0.08, 0.6), (0, ly + s * 0.68, 0.82), (s * -10, 0, 0), fase=0.01)
        for dy in (-0.4, 0.4):
            bm = stamm_bm(0.17, 0.08, ecken=10, seed=seed + 50)
            setzen(bm, (-0.04, 0, 0))
            setzen(bm, (s * 0.44, ly + dy, 0.4))
            teile.append(objekt("Lorenrad", bm, eisen))
    for dy in (-0.62, 0.62):
        _brett(teile, "Loreband", eisen, (1.25, 0.04, 0.06), (0, ly + dy * 1.1, 1.05), fase=0.0)
    for i in range(9):
        bm = stein_bm(zufall, zufall.uniform(0.2, 0.28), flach=0.8)
        _setzen(bm, (zufall.uniform(-0.3, 0.3), ly + zufall.uniform(-0.45, 0.45), 1.02 + zufall.uniform(0, 0.12)), zufall.uniform(0, math.tau))
        teile.append(objekt("Erz", bm, erz))
    for i in range(4):
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.06, radius2=0.0, depth=0.25)
        setzen(bm, (zufall.uniform(-0.3, 0.3), ly + zufall.uniform(-0.4, 0.4), 1.2), (zufall.uniform(-25, 25), zufall.uniform(-25, 25), 0))
        licht.append(objekt("Kristall", bm, einfarbig("#9FE0FF", 0.05, zufall)))

    # Förderturm über dem Schacht: vier Beine, Streben, Bühne, Seilscheibe, Seil mit Kübel
    fx, fy = 4.6, -0.9
    w.brett("Schacht", einfarbig("#140F0C", 0.02, zufall), (1.3, 1.3, 0.05), (fx, fy, 0.08), fase=0.0)
    for k in range(4):
        rz = 90 * k
        v = Matrix.Rotation(math.radians(rz), 3, "Z") @ Vector((0, -0.75, 0))
        w.stamm("Schachtkranz", holz, 0.14, 1.8, (fx + v.x, fy + v.y, 0.14), (0, 0, rz), ecken=7)
    fuss, kopf, hoehe = 1.05, 0.5, 4.6
    for sx in (-1, 1):
        for sy in (-1, 1):
            w.saeule("Turmbein", holz, 0.11, (fx + sx * fuss, fy + sy * fuss, 0.0), (fx + sx * kopf, fy + sy * kopf, hoehe), ecken=7)
    for zz in (1.6, 3.2):
        t = zz / hoehe
        b = fuss + (kopf - fuss) * t
        for k in range(4):
            rz = 90 * k
            v = Matrix.Rotation(math.radians(rz), 3, "Z") @ Vector((0, -b, zz))
            w.brett("Turmriegel", bretter, (2 * b + 0.2, 0.1, 0.12), (fx + v.x, fy + v.y, v.z), (0, 0, rz), fase=0.01)
    for k in range(4):
        rz = 90 * k
        v0 = Matrix.Rotation(math.radians(rz), 3, "Z") @ Vector((-fuss * 0.95, -fuss * 0.95, 0.1))
        v1 = Matrix.Rotation(math.radians(rz), 3, "Z") @ Vector((0.66, -0.66, 3.2))
        w.saeule("Turmstrebe", bretter, 0.05, (fx + v0.x, fy + v0.y, v0.z), (fx + v1.x, fy + v1.y, v1.z), ecken=5)
    w.brett("Turmbühne", bretter, (1.5, 1.5, 0.12), (fx, fy, hoehe), fase=0.02)
    for s in (-1, 1):
        w.brett("Scheibenlager", bretter, (0.18, 0.18, 0.7), (fx, fy + s * 0.35, hoehe + 0.4), fase=0.02)
    sr, sz = 0.85, hoehe + 0.75
    for k in range(14):
        a = math.tau * (k + 0.5) / 14
        w.brett("Seilscheibe", eisen, (2 * sr * math.sin(math.pi / 14) * 1.05, 0.12, 0.12), (fx + math.cos(a) * sr, fy, sz + math.sin(a) * sr), (0, 90 - math.degrees(a), 0), fase=0.01)
    for k in range(4):
        w.brett("Scheibenspeiche", eisen, (2 * sr - 0.1, 0.05, 0.05), (fx, fy, sz), (0, 45 * k, 0), fase=0.0)
    w.saeule("Scheibenachse", eisen, 0.06, (fx, fy - 0.45, sz), (fx, fy + 0.45, sz), ecken=6)
    seil = einfarbig("#C8B58A", 0.03, zufall)
    w.saeule("Förderseil", seil, 0.025, (fx + sr, fy, sz), (fx + sr, fy, 1.9), ecken=4)
    w.dreh("Kübel", bretter, [(0.28, 1.1), (0.34, 1.7), (0.3, 1.7), (0.24, 1.14)], (fx + sr, fy, 0.0), ecken=10)
    w.saeule("Förderseil", seil, 0.025, (fx - sr, fy, sz), (fx - 2.1, fy - 0.3, 1.05), ecken=4)
    # Haspel (Seilwinde) neben dem Turm
    hx, hy = fx - 2.1, fy - 0.3
    for s in (-1, 1):
        w.brett("Haspelbock", bretter, (0.14, 0.14, 1.0), (hx, hy + s * 0.55, 0.5), fase=0.015)
    w.stamm("Haspelwelle", holz, 0.16, 1.2, (hx, hy, 0.9), (0, 0, 90), ecken=8)
    for s in (-1, 1):
        w.brett("Haspelkurbel", eisen, (0.05, 0.05, 0.5), (hx, hy + s * 0.68, 0.75), (s * 25, 0, 0), fase=0.0)

    # Rennofen: gemauerter Schachtofen mit Glut, Blasebalg, Schlacke
    def rennofen(t):
        ofen = steinfarbe(t, "#5E5850", "#7A736A", "#968E84", "#ADA59A")
        mauerring(t, ofen, 0.95, 0.55, 0.0, 2.3, h=0.46, tiefe=0.4)
        t.dreh("Ofenkrone", ofen, [(0.78, 2.3), (0.78, 2.45), (0.42, 2.45), (0.42, 2.3)], (0, 0, 0), ecken=10)
        t.dreh("Ofenglut", einfarbig("#FF8A2A", 0.05, t.z), [(0.42, 2.36), (0.0, 2.58)], (0, 0, 0), ecken=8, glut=True)
        t.brett("Ofenmund", einfarbig("#FF9A3A", 0.05, t.z), (0.5, 0.1, 0.5), (0, -1.13, 0.4), fase=0.0, glut=True)
        t.brett("Ofensturz", ofen, (1.0, 0.4, 0.22), (0, -1.12, 0.78), fase=0.03)
        for s in (-1, 1):
            t.brett("Ofenwange", ofen, (0.25, 0.4, 0.68), (s * 0.38, -1.12, 0.34), fase=0.03)
        for k in range(5):
            t.stein("Schlacke", einfarbig("#3A3230", 0.06, t.z), t.z.uniform(0.1, 0.16), (t.z.uniform(-0.5, 0.5), -1.55 + t.z.uniform(-0.2, 0.2), 0.04), flach=0.6)
        leder = einfarbig("#6A4A34", 0.05, t.z)
        t.brett("Balgboden", bretter, (1.0, 0.55, 0.07), (-1.55, -0.2, 0.42), fase=0.01)
        t.brett("Balgleder", leder, (0.85, 0.5, 0.16), (-1.6, -0.2, 0.53), (0, 5, 0), fase=0.02)
        t.brett("Balgdeckel", bretter, (1.0, 0.55, 0.07), (-1.58, -0.2, 0.66), (0, 10, 0), fase=0.01)
        t.saeule("Balgdüse", eisen, 0.05, (-1.05, -0.2, 0.5), (-0.75, -0.2, 0.55), ecken=6)
        t.brett("Balgstiel", bretter, (0.9, 0.06, 0.06), (-2.3, -0.2, 0.85), (0, 30, 0), fase=0.0)
        for x in (-1.2, -1.9):
            t.brett("Balgbock", bretter, (0.1, 0.5, 0.38), (x, -0.2, 0.19), fase=0.01)
    setze(w, (-3.6, -1.5, 0.0), 0, rennofen)
    # Eisenbarren, frisch aus dem Ofen
    barren = einfarbig("#5A5E66", 0.05, zufall)
    for lage in range(3):
        for i in range(4 - lage):
            w.brett("Eisenbarren", barren, (0.55, 0.15, 0.12), (-3.3, -3.55 + (i - (3 - lage) / 2) * 0.18, 0.07 + lage * 0.12), (0, 0, zufall.uniform(-3, 3)), fase=0.02)

    # Erzhaufen, Kisten, Fass, Spitzhacke, Schild, Tannen
    for i in range(16):
        wi = zufall.uniform(0, math.tau)
        d = zufall.uniform(0.0, 0.9)
        bm = stein_bm(zufall, zufall.uniform(0.24, 0.36), flach=0.8)
        _setzen(bm, (2.6 + math.cos(wi) * d, -3.0 + math.sin(wi) * d, 0.18 + (0.9 - d) * 0.35), zufall.uniform(0, math.tau))
        teile.append(objekt("Erzhaufen", bm, erz))
    for (x, y, z, g) in ((-1.5, -4.3, 0.0, 0.7), (-1.55, -5.1, 0.0, 0.6), (-1.5, -4.35, 0.7, 0.5)):
        _kiste(w, (x, y, z), g)
    fass(w, (-2.5, -4.6, 0.0), 0.95)
    _brett(teile, "Hackenstiel", holzfarbe(zufall, "#C08A4E", "#94663A"), (0.05, 0.05, 0.9), (3.5, -4.1, 0.45), (0, 25, 30), fase=0.0)
    _brett(teile, "Hackenkopf", eisen, (0.6, 0.05, 0.07), (3.35, -4.2, 0.86), (0, 5, 30), fase=0.0)
    _schild(teile, zufall, (-5.4, -3.3, 0.0), 0, [
        (-0.05, 0.02, 0.45, 0.05, 45, "#6E4A2A"), (0.05, 0.02, 0.45, 0.05, -45, "#6E4A2A"),
        (-0.14, 0.16, 0.26, 0.06, 45, "#4A4642"), (0.14, 0.16, 0.26, 0.06, -45, "#4A4642"),
        (0.0, -0.13, 0.14, 0.1, 0, "#B85A34"),
    ])
    tanne(w, (-6.2, 0.9), 3.2)
    tanne(w, (6.5, 1.8), 2.6)
    busch(w, (-5.9, -1.4), 0.5)
    return fertig(w, "Erzmine")


# ---------------------------------------------------------------------------
# Dorfhalle: Stufe 1 Langhaus mit Vorlaube, Stufe 2 Rathaus (Steingeschoss, Fachwerk, Uhrturm,
# Balkon, Marktstände), Stufe 3 Burgfried dahinter mit Erkertürmen und Mauern mit Rundtürmen
# ---------------------------------------------------------------------------
def quaderblock(w, farbe_von, b, d, z0, z1, mitte=(0.0, 0.0), h=0.7, laenge=1.3, tiefe=0.6, fase=0.0, kern="#57524B"):
    """Rechteckiger Mauerkörper aus Quadern (Außenmaß b + tiefe, d + tiefe), mit Kern."""
    mx, my = mitte
    ecken = [(mx - b / 2, my - d / 2), (mx + b / 2, my - d / 2), (mx + b / 2, my + d / 2), (mx - b / 2, my + d / 2)]
    for i in range(4):
        a, e = Vector(ecken[i]), Vector(ecken[(i + 1) % 4])
        r = (e - a).normalized()
        quadermauer(w, farbe_von, a - r * tiefe / 2, e + r * tiefe / 2, z0, z1, tiefe=tiefe, h=h, laenge=laenge, beide=False, fase=fase, kern=None)
    if kern:
        w.brett("Mauerkern", einfarbig(kern, 0.05, w.z), (b, d, z1 - z0), (mx, my, (z0 + z1) / 2), fase=0.0)


def rundmauer(w, farbe_von, r, z0, z1, ort=(0.0, 0.0), h=0.5, tiefe=0.45, fase=0.0, kern="#57524B"):
    """Runder Turmschaft aus Quadern im Verband (ohne Fase: günstig für große Flächen)."""
    lagen = max(1, round((z1 - z0) / h))
    h = (z1 - z0) / lagen
    n = max(8, round(math.tau * r / 1.0))
    for l in range(lagen):
        for k in range(n):
            wi = math.tau * (k + (0.5 if l % 2 else 0.0)) / n
            laenge = math.tau * r / n * w.z.uniform(0.9, 0.97)
            bm = brett_bm(laenge, tiefe * w.z.uniform(0.9, 1.05), h * w.z.uniform(0.88, 0.95), fase=fase)
            setzen(bm, (0, 0, 0), (0, 0, math.degrees(wi) + 90))
            setzen(bm, (ort[0] + math.cos(wi) * r, ort[1] + math.sin(wi) * r, z0 + (l + 0.5) * h))
            w.bm("Mauerstein", bm, farbe_von)
    w.dreh("Mauerkern", einfarbig(kern, 0.05, w.z), [(r - tiefe * 0.4, z0), (r - tiefe * 0.4, z1)], (ort[0], ort[1], 0), ecken=12)


def gewaende(w, stein, mitte, drehung_z, breite, hoehe):
    """Steinerne Fensterumrahmung: Gewände, Sturz mit Schlussstein."""
    x0, y0, z0 = mitte
    rot = Matrix.Rotation(math.radians(drehung_z), 4, "Z")

    def p(dx, dy, dz):
        v = rot @ Vector((dx, dy, dz))
        return (x0 + v.x, y0 + v.y, z0 + v.z)
    for s in (-1, 1):
        w.brett("Gewände", stein, (0.16, 0.18, hoehe + 0.1), p(s * (breite / 2 + 0.06), -0.02, 0), (0, 0, drehung_z), fase=0.025)
    w.brett("Sturz", stein, (breite + 0.5, 0.22, 0.26), p(0, -0.04, hoehe / 2 + 0.17), (0, 0, drehung_z), fase=0.03)
    w.brett("Schlussstein", stein, (0.26, 0.26, 0.38), p(0, -0.06, hoehe / 2 + 0.2), (0, 0, drehung_z), fase=0.03)


def uhr(w, mitte, s, gold):
    """Zifferblatt mit Goldrand und Zeigern an einer Wand, die nach s·Y schaut."""
    x, y, z = mitte
    w.dreh("Uhrrand", gold, [(0.7, -0.05), (0.7, 0.05)], (x, y, z), ecken=16, drehung=(90, 0, 0))
    w.dreh("Zifferblatt", einfarbig("#F4EEDC", 0.02, w.z), [(0.6, -0.08), (0.6, 0.08)], (x, y, z), ecken=16, drehung=(90, 0, 0))
    for k in range(4):
        a = math.tau * k / 4
        w.brett("Stundenmarke", gold, (0.08, 0.04, 0.14), (x + math.cos(a) * 0.48, y + s * 0.09, z + math.sin(a) * 0.48), (0, 90 - math.degrees(a), 0), fase=0.0)
    w.brett("Stundenzeiger", einfarbig("#2E2B2A", 0.03, w.z), (0.06, 0.03, 0.34), (x + 0.08, y + s * 0.1, z + 0.13), (0, 32, 0), fase=0.0)
    w.brett("Minutenzeiger", einfarbig("#2E2B2A", 0.03, w.z), (0.05, 0.03, 0.5), (x, y + s * 0.11, z + 0.23), (0, -4, 0), fase=0.0)


def dorfhalle(stufe=1, seed=64):
    w = Werk(seed + stufe)
    teile, licht, zufall = w.teile, w.licht, w.z
    B, T = 11.0, 6.4
    boden = 0.5
    geschoss = 3.2
    traufe = boden + geschoss * (2 if stufe >= 2 else 1)
    first = traufe + 3.2
    ueber = 0.65
    extra = 0.4 if stufe >= 2 else 0.0

    stein = steinfarbe(w, moos="#7BA84E")
    quader = steinfarbe(w, "#8E877C", "#B3AB9E", "#D2CABC", "#E6DFD2")
    turmstein = _steinfarbe(zufall, farbe("#7E776C"), farbe("#A8A094"), farbe("#CAC2B4"), farbe("#DDD6C8"))
    steinputz = einfarbig("#E2D8C2", 0.03, zufall)
    erde = einfarbig("#8C7A58", 0.06, zufall)
    putz = einfarbig("#EFE4C8", 0.025, zufall)
    balken = holzfarbe(zufall, "#5E3C22", "#3E2614")
    rahmen = holzfarbe(zufall, "#C49A62", "#96703E")
    laden = einfarbig("#3F6FB5", 0.04, zufall)
    ziegel = holzfarbe(zufall, "#B4523A", "#8A3A28", maserung=9.0)
    gold = einfarbig(GOLD, 0.03, zufall)
    eisen = einfarbig("#35312E", 0.05, zufall)
    torholz = holzfarbe(zufall, "#7C4E2A", "#5A3618")

    _platte(teile, "Dorfplatz", erde, 11.5, 9.5, 0.04, zufall, ecken=22)
    pflasterweg(w, quader, -1.5, 1.5, -9.0, -4.6)
    sockel(w, stein, B + 0.6, T + 0.6, boden)
    for i in range(3):
        w.brett("Stufe", quader, (3.2 - i * 0.2, 0.45, boden - i * 0.16), (0, -T / 2 - 0.5 - i * 0.42, (boden - i * 0.16) / 2), fase=0.03)

    # Erdgeschoss: Fachwerk (Langhaus) oder Stein mit Eckquadern (Rathaus)
    if stufe == 1:
        tor = (-1.0, 1.0, 0.0, 2.7)
        fenster = [(-3.6, -2.4, 1.1, 2.3), (2.4, 3.6, 1.1, 2.3)]
        _fachwerkwand(teile, B, geschoss, (0, -T / 2, boden), 0, putz, balken, zufall, luecken=[tor] + fenster)
        _fachwerkwand(teile, B, geschoss, (0, T / 2, boden), 180, putz, balken, zufall, luecken=[(-0.6, 0.6, 1.1, 2.3)])
        _fachwerkwand(teile, T, geschoss, (-B / 2, 0, boden), 90, putz, balken, zufall, luecken=[(-0.6, 0.6, 1.1, 2.3)])
        _fachwerkwand(teile, T, geschoss, (B / 2, 0, boden), -90, putz, balken, zufall, luecken=[(-0.6, 0.6, 1.1, 2.3)])
        for (a, b, u, o) in fenster:
            _dorf_fenster(teile, licht, zufall, ((a + b) / 2, -T / 2 - 0.12, boden + (u + o) / 2), 0, b - a, o - u, rahmen, laden)
        for (x, y, d) in ((0, T / 2 + 0.12, 180), (-B / 2 - 0.12, 0, 90), (B / 2 + 0.12, 0, -90)):
            _dorf_fenster(teile, licht, zufall, (x, y, boden + 1.7), d, 1.2, 1.2, rahmen, laden)
        for i in range(8):
            w.brett("Torbrett", torholz, (0.24, 0.08, 2.6), (-0.87 + i * 0.25, -T / 2 - 0.02, boden + 1.3), fase=0.01)
        for dz in (0.5, 2.1):
            w.brett("Torband", eisen, (2.0, 0.04, 0.1), (0, -T / 2 - 0.08, boden + dz), fase=0.0)
        for k in range(7):
            a = math.pi * k / 6
            w.brett("Torbogen", balken, (0.5, 0.3, 0.22), (math.cos(a) * 1.05, -T / 2 - 0.08, boden + 2.7 + math.sin(a) * 0.55), (0, -math.degrees(a) + 90, 0), fase=0.02)
    else:
        tor = (-1.0, 1.0, 0.0, 2.5)
        fenster = [(-4.0, -2.6, 0.9, 2.4), (2.6, 4.0, 0.9, 2.4)]
        seitenfenster = [(-0.7, 0.7, 0.9, 2.4)]
        steinwand(w, steinputz, B, geschoss, (0, -T / 2 + 0.1, boden), 0, [tor] + fenster)
        steinwand(w, steinputz, B, geschoss, (0, T / 2 - 0.1, boden), 180, seitenfenster)
        steinwand(w, steinputz, T - 0.6, geschoss, (-B / 2 + 0.12, 0, boden), 90, seitenfenster)
        steinwand(w, steinputz, T - 0.6, geschoss, (B / 2 - 0.12, 0, boden), -90, seitenfenster)
        for sx in (-1, 1):
            for sy in (-1, 1):
                eckquader(w, quader, (sx * (B / 2 + 0.105), sy * (T / 2 + 0.125)), (-sx, 0), (0, -sy), boden, boden + geschoss, h=0.54, lang=0.9, tiefe=0.5, vor=0.05)
        for (a, b, u, o) in fenster:
            m = ((a + b) / 2, -T / 2 - 0.14, boden + (u + o) / 2)
            _dorf_fenster(teile, licht, zufall, m, 0, b - a, o - u, rahmen, laden)
            gewaende(w, quader, m, 0, b - a, o - u)
        for (x, y, d) in ((0, T / 2 + 0.14, 180), (-B / 2 - 0.12, 0, 90), (B / 2 + 0.12, 0, -90)):
            _dorf_fenster(teile, licht, zufall, (x, y, boden + 1.65), d, 1.4, 1.5, rahmen, laden)
            gewaende(w, quader, (x, y, boden + 1.65), d, 1.4, 1.5)
        # Tor: Bohlenflügel, Beschläge, Gewände und scheitrechter Bogen mit Schlussstein
        for i in range(8):
            w.brett("Torbrett", torholz, (0.24, 0.08, 2.5), (-0.87 + i * 0.25, -T / 2 + 0.02, boden + 1.25), fase=0.01)
        for dz in (0.5, 2.0):
            w.brett("Torband", eisen, (2.0, 0.04, 0.1), (0, -T / 2 - 0.04, boden + dz), fase=0.0)
        for s in (-1, 1):
            w.brett("Torring", gold, (0.14, 0.05, 0.14), (s * 0.3, -T / 2 - 0.06, boden + 1.2), (0, 45, 0), fase=0.0)
            w.brett("Torgewände", quader, (0.32, 0.55, 2.5), (s * 1.16, -T / 2 - 0.04, boden + 1.25), fase=0.03)
        for k in range(5):
            x = (k - 2) * 0.47
            hk = 0.75 if k == 2 else 0.6
            w.brett("Bogenstein", quader, (0.45, 0.56, hk), (x * 1.04, -T / 2 - 0.05, boden + 2.5 + hk / 2), (0, math.degrees(math.atan2(x, 2.2)), 0), fase=0.03)

    if stufe >= 2:
        # Obergeschoss: vorkragendes Fachwerk, Balkon über dem Tor, Banner an der Fassade
        z1 = boden + geschoss
        w.brett("Deckenbalken", balken, (B + 0.7, T + 0.7, 0.25), (0, 0, z1), fase=0.02)
        for i in range(12):
            x = -B / 2 + 0.2 + i * (B - 0.4) / 11
            w.brett("Balkenkopf", balken, (0.2, 0.3, 0.22), (x, -T / 2 - 0.4, z1 - 0.12), fase=0.015)
        _fachwerkwand(teile, B + 0.4, geschoss, (0, -T / 2 - 0.2, z1 + 0.12), 0, putz, balken, zufall, luecken=[(-3.6, -2.4, 0.9, 2.2), (-0.8, 0.8, 0.0, 2.4), (2.4, 3.6, 0.9, 2.2)])
        _fachwerkwand(teile, B + 0.4, geschoss, (0, T / 2 + 0.2, z1 + 0.12), 180, putz, balken, zufall, luecken=[(-2.0, -0.8, 0.9, 2.2), (0.8, 2.0, 0.9, 2.2)])
        _fachwerkwand(teile, T + 0.4, geschoss, (-B / 2 - 0.2, 0, z1 + 0.12), 90, putz, balken, zufall)
        _fachwerkwand(teile, T + 0.4, geschoss, (B / 2 + 0.2, 0, z1 + 0.12), -90, putz, balken, zufall)
        for x in (-3.0, 3.0):
            _dorf_fenster(teile, licht, zufall, (x, -T / 2 - 0.32, z1 + 1.67), 0, 1.2, 1.3, rahmen, laden)
        for x in (-1.4, 1.4):
            _dorf_fenster(teile, licht, zufall, (x, T / 2 + 0.32, z1 + 1.67), 180, 1.2, 1.3, rahmen, laden)
        w.brett("Balkontür", einfarbig("#FFC878", 0.02, zufall), (1.4, 0.05, 2.2), (0, -T / 2 - 0.22, z1 + 1.25), fase=0.0, glut=True)
        w.brett("Balkonboden", rahmen, (2.8, 1.2, 0.14), (0, -T / 2 - 0.85, z1 + 0.1), fase=0.02)
        for i in range(8):
            w.brett("Geländerstab", rahmen, (0.07, 0.07, 0.9), (-1.3 + i * 0.37, -T / 2 - 1.4, z1 + 0.6), fase=0.01)
        w.brett("Handlauf", balken, (2.9, 0.12, 0.1), (0, -T / 2 - 1.4, z1 + 1.08), fase=0.01)
        w.brett("Balkonkasten", rahmen, (2.7, 0.24, 0.2), (0, -T / 2 - 1.55, z1 + 0.95), fase=0.015)
        for i in range(7):
            w.stein("Blüte", einfarbig(("#E24A4A", "#F2C94C", "#E88AD0", "#6FA8E8")[i % 4], 0.05, zufall), 0.12, (-1.15 + i * 0.38, -T / 2 - 1.55, z1 + 1.12), flach=1.0)
        for x in (-1.3, 1.3):
            w.brett("Balkonstütze", balken, (0.14, 0.14, 1.2), (x, -T / 2 - 1.3, z1 - 0.5), (35, 0, 0), fase=0.01)
        for x in (-4.75, 4.75):
            wandbanner(w, (x, -T / 2 - 0.45, z1 + 2.85), 0, 0.8, 2.1, TUCH_BLAU)

    # Giebel, Windbretter, Schindeldach mit Sparrenköpfen
    for x in (-B / 2 - extra / 2, B / 2 + extra / 2):
        _giebel(teile, putz, T + extra, x, traufe - 0.05, first - 0.15, bretter=9)
        w.brett("Giebelbalken", balken, (0.14, 0.2, first - traufe), (x * 1.01, 0, (traufe + first) / 2), fase=0.01)
        w.brett("Giebelriegel", balken, (0.14, T * 0.7, 0.18), (x * 1.01, 0, traufe + (first - traufe) * 0.35), fase=0.01)
        w.brett("Giebelfenster", einfarbig("#FFC878", 0.02, zufall), (0.1, 0.8, 0.9), (x * 1.012, 0, traufe + (first - traufe) * 0.55), fase=0.0, glut=True)
        w.brett("Giebelfensterrahmen", rahmen, (0.12, 1.0, 1.1), (x * 1.008, 0, traufe + (first - traufe) * 0.55), fase=0.015)
    ziegeldach(w, ziegel, B + extra, T + extra, traufe, first, ueber, reihen=9, stuecke=4, name="Dachziegel")
    for x in (-(B + extra) / 2 - ueber, (B + extra) / 2 + ueber):
        windbretter(w, balken, x, T + extra, traufe, first, ueber)
    sparren(w, balken, B + extra, T + extra, traufe, first, ueber, abstand=1.2)
    quaderblock(w, stein, 0.65, 0.55, first - 1.9, first + 1.5, mitte=(B / 2 - 2.0, 1.4), h=0.43, laenge=0.9, tiefe=0.3, fase=0.0)
    w.brett("Kaminkrone", stein, (1.15, 1.05, 0.2), (B / 2 - 2.0, 1.4, first + 1.6), fase=0.04)

    if stufe == 1:
        # Vorlaube über dem Tor: Satteldach quer zum Haus auf geschnitzten Pfosten
        def vorlaube(t):
            ziegeldach(t, ziegel, 2.2, 3.4, 3.35, 4.45, 0.25, reihen=4, stuecke=2, name="Vorlaubendach")
            windbretter(t, balken, -1.1 - 0.25, 3.4, 3.35, 4.45, 0.25)
            _giebel(t.teile, putz, 3.3, -1.05, 3.3, 4.4, bretter=7)
            t.brett("Laubenbalken", balken, (0.2, 3.5, 0.22), (-1.0, 0, 3.25), fase=0.02)
            for s in (-1, 1):
                t.brett("Laubenpfosten", balken, (0.2, 0.2, 3.2), (-0.95, s * 1.65, 1.6), fase=0.02)
                t.brett("Kopfband", balken, (0.1, 0.6, 0.1), (-0.95, s * 1.45, 2.95), (s * 45, 0, 0), fase=0.01)
                t.brett("Pfostenfuß", quader, (0.4, 0.4, 0.25), (-0.95, s * 1.65, 0.12), fase=0.03)
            t.brett("Laubenwappen", gold, (0.5, 0.08, 0.5), (-1.12, 0, 3.8), (0, 45, 90), fase=0.02)
        setze(w, (0, -T / 2 - 0.9, 0), 90, vorlaube)
        # Glocke am Holzgalgen
        for y in (-2.2, -1.0):
            w.brett("Galgenpfosten", balken, (0.16, 0.16, 2.8), (-7.8, y, 1.4), fase=0.02)
        w.brett("Galgenbalken", balken, (0.16, 1.5, 0.16), (-7.8, -1.6, 2.8), fase=0.02)
        w.dreh("Glocke", gold, [(0.0, 1.85), (0.3, 1.9), (0.26, 2.2), (0.16, 2.5), (0.0, 2.55)], (-7.8, -1.6, 0), ecken=10)
    if stufe >= 2:
        # Uhrturm auf dem First: Fachwerkkasten mit Uhren, offene Glockenstube, Pyramidendach, Wimpel
        tb = 2.2
        z0 = first - 1.3
        for (x, y, rz, l) in ((0, -tb / 2, 0, tb), (0, tb / 2, 180, tb), (-tb / 2, 0, 90, tb), (tb / 2, 0, -90, tb)):
            _fachwerkwand(teile, l, 2.7, (x, y, z0), rz, putz, balken, zufall, streben=False)
        for s in (-1, 1):
            uhr(w, (0, s * (tb / 2 + 0.16), first + 0.55), s, gold)
        zs = z0 + 2.7
        w.brett("Stubenboden", balken, (tb + 0.4, tb + 0.4, 0.2), (0, 0, zs), fase=0.02)
        for sx in (-1, 1):
            for sy in (-1, 1):
                w.brett("Stubenpfosten", balken, (0.2, 0.2, 1.8), (sx * (tb / 2 - 0.05), sy * (tb / 2 - 0.05), zs + 0.9), fase=0.02)
        for s in range(4):
            v = Matrix.Rotation(math.radians(90 * s), 3, "Z") @ Vector((0, -tb / 2 + 0.05, zs + 0.55))
            w.brett("Brüstung", rahmen, (tb, 0.1, 0.12), tuple(v), (0, 0, 90 * s), fase=0.01)
        w.dreh("Glocke", gold, [(0.0, zs + 0.5), (0.45, zs + 0.55), (0.38, zs + 0.95), (0.22, zs + 1.4), (0.0, zs + 1.48)], (0, 0, 0), ecken=12)
        w.brett("Glockenjoch", balken, (tb, 0.18, 0.2), (0, 0, zs + 1.55), fase=0.02)
        stufendach(w, ziegel, (tb / 2 + 0.45) * WURZEL2, zs + 1.8, 3.2, ecken=4, reihen=6, drehung_z=45)
        w.dreh("Turmknauf", gold, [(0.0, zs + 4.9), (0.14, zs + 5.05), (0.1, zs + 5.3), (0.0, zs + 5.45)], (0, 0, 0), ecken=8)
        fahne(w, (0, 0, zs + 5.4), 1.7, TUCH_BLAU, breite=1.1, tuch_hoehe=0.7)

    if stufe >= 3:
        kx, ky, kb, kh = -5.2, 4.6, 5.6, 14.0

        def burgfried(t):
            tiefe = 0.6
            t.brett("Burgfriedkern", einfarbig("#57524B", 0.05, t.z), (kb - 0.5, kb - 0.5, kh + SOCKEL), (0, 0, (kh - SOCKEL) / 2), fase=0.0)
            t.dreh("Anlauf", turmstein, [((kb / 2 + 0.45) * WURZEL2, -0.3), ((kb / 2 + 0.45) * WURZEL2, 0.5), ((kb / 2) * WURZEL2, 1.3)], (0, 0, 0), ecken=4, drehung=(0, 0, 45))
            quaderblock(t, turmstein, kb - tiefe, kb - tiefe, 1.2, kh, h=0.7, laenge=1.35, tiefe=tiefe, fase=0.0, kern=None)
            for zz in (5.2, 10.2):
                t.brett("Gurtgesims", quader, (kb + 0.22, kb + 0.22, 0.26), (0, 0, zz), fase=0.04)
            for rz in (0, -90, 90, 180):
                rot = Matrix.Rotation(math.radians(rz), 3, "Z")

                def p(x, y, z):
                    return tuple(rot @ Vector((x, y, z)))
                aussen = -kb / 2 - 0.02
                for (x, z) in ((-1.3, 3.2), (1.2, 7.4), (-0.9, 11.6)):
                    if rz == 0 and z < 10:
                        continue
                    t.brett("Scharte", einfarbig("#FFC878", 0.02, t.z), (0.24, 0.1, 0.95), p(x, aussen, z), (0, 0, rz), fase=0.0, glut=True)
                    t.brett("Schartenrahmen", quader, (0.52, 0.12, 1.22), p(x, aussen + 0.03, z), (0, 0, rz), fase=0.03)
                if rz != 0:
                    t.brett("Turmfenster", einfarbig("#FFC878", 0.02, t.z), (0.8, 0.1, 1.3), p(0.9, aussen, 8.6), (0, 0, rz), fase=0.0, glut=True)
                    gewaende(t, quader, p(0.9, aussen, 8.6), rz, 0.8, 1.3)
                for i in range(6):
                    x = -kb / 2 + 0.45 + i * (kb - 0.9) / 5
                    t.brett("Konsole", quader, (0.32, 0.5, 0.55), p(x, -kb / 2 - 0.1, kh - 0.3), (0, 0, rz), fase=0.03)
                t.brett("Brüstung", turmstein, (kb + 0.7, 0.4, 1.1), p(0, -kb / 2 - 0.15, kh + 0.55), (0, 0, rz), fase=0.04)
                for i in range(4):
                    x = -kb / 2 + 0.9 + i * (kb - 1.8) / 3
                    t.brett("Zinne", turmstein, (0.75, 0.45, 0.7), p(x, -kb / 2 - 0.15, kh + 1.45), (0, 0, rz), fase=0.04)
            t.brett("Wehrgang", quader, (kb, kb, 0.2), (0, 0, kh + 0.1), fase=0.02)
            # Banner an den Seiten
            wandbanner(t, (-kb / 2 - 0.08, 0.0, kh - 1.0), -90, 1.5, 3.8, TUCH_BLAU)
            wandbanner(t, (0.0, kb / 2 + 0.08, kh - 1.0), 180, 1.5, 3.8, TUCH_BLAU)
            wandbanner(t, (kb / 2 + 0.08, 0.9, kh - 1.0), 90, 1.3, 3.2, TUCH_BLAU)
            wandbanner(t, (0.9, -kb / 2 - 0.08, kh - 1.0), 0, 1.2, 2.6, TUCH_BLAU)
            # Erkertürmchen an den Ecken mit Kegeldächern
            for sx in (-1, 1):
                for sy in (-1, 1):
                    cx, cy = sx * (kb / 2 + 0.05), sy * (kb / 2 + 0.05)
                    t.dreh("Erkerfuß", turmstein, [(0.0, kh - 2.6), (0.5, kh - 2.0), (0.9, kh - 1.2)], (cx, cy, 0), ecken=10)
                    t.dreh("Erker", turmstein, [(0.9, kh - 1.2), (0.9, kh + 1.75)], (cx, cy, 0), ecken=10)
                    t.dreh("Erkergesims", quader, [(1.0, kh + 1.6), (1.02, kh + 1.8), (0.85, kh + 1.85)], (cx, cy, 0), ecken=10)
                    t.brett("Erkerfenster", einfarbig("#FFC878", 0.02, t.z), (0.22, 0.1, 0.6), (cx + sx * 0.62, cy + sy * 0.62, kh + 0.6),
                            (0, 0, math.degrees(math.atan2(sy, sx)) + 90), fase=0.0, glut=True)
                    stufendach(t, ziegel, 1.2, kh + 1.85, 2.6, ecken=10, reihen=5, ort=(cx, cy))
                    t.dreh("Erkerknauf", gold, [(0.0, kh + 4.35), (0.1, kh + 4.5), (0.0, kh + 4.75)], (cx, cy, 0), ecken=6)
                    if sy < 0:
                        fahne(t, (cx, cy, kh + 4.6), 1.3, "#E8C04A", breite=0.8, tuch_hoehe=0.5)
            # Hohes Pyramidendach mit Goldspitze und großer Fahne
            stufendach(t, ziegel, (kb / 2 - 0.2) * WURZEL2, kh + 0.2, 5.5, ecken=4, reihen=7, drehung_z=45)
            t.dreh("Turmspitze", gold, [(0.0, kh + 5.5), (0.2, kh + 5.75), (0.13, kh + 6.05), (0.0, kh + 6.3)], (0, 0, 0), ecken=8)
            fahne(t, (0, 0, kh + 6.2), 2.4, TUCH_BLAU, breite=1.7, tuch_hoehe=0.95)
        setze(w, (kx, ky, 0.0), 0, burgfried)

        # Mauern links und rechts des Dorfplatzes mit Zinnen, vorne je ein Rundturm
        for sx in (-1, 1):
            x = sx * 8.2
            quadermauer(w, quader, (x, -4.4), (x, 1.6), 0.0, 2.8, tiefe=0.9, h=0.56, laenge=1.2, fase=0.0)
            w.brett("Mauerabdeckung", quader, (1.0, 6.0, 0.16), (x, -1.4, 2.88), fase=0.03)
            for i in range(5):
                w.brett("Zinne", quader, (0.9, 0.7, 0.7), (x, -3.9 + i * 1.25, 3.3), fase=0.04)
            ty = -5.0
            rundmauer(w, turmstein, 1.0, -0.3, 4.2, ort=(x, ty), h=0.52, tiefe=0.45)
            w.dreh("Turmgesims", quader, [(1.2, 4.1), (1.25, 4.35), (1.05, 4.4)], (x, ty, 0), ecken=12)
            stufendach(w, ziegel, 1.3, 4.35, 2.8, ecken=12, reihen=5, ort=(x, ty))
            w.dreh("Turmknauf", gold, [(0.0, 7.1), (0.12, 7.25), (0.0, 7.5)], (x, ty, 0), ecken=6)
            fahne(w, (x, ty, 7.35), 1.4, TUCH_BLAU, breite=0.9, tuch_hoehe=0.55)
            for zz in (1.4, 3.0):
                w.brett("Scharte", einfarbig("#FFC878", 0.02, zufall), (0.2, 0.1, 0.7), (x, ty - 1.02, zz), fase=0.0, glut=True)

    # Dorfplatz: Banner, Feuerstelle, Brunnen, Fässer, Bank, Laternen, Büsche, Marktstände
    _banner_mast(teile, zufall, (-3.6, -T / 2 - 3.2, 0.0), hoehe=6.5 + stufe)
    if stufe >= 2:
        _banner_mast(teile, zufall, (3.6, -T / 2 - 3.2, 0.0), hoehe=6.5 + stufe)
    for i in range(10):
        wi = math.tau * i / 10
        w.stein("Feuerstein", stein, 0.22, (4.8 + math.cos(wi) * 0.55, -T / 2 - 1.3 + math.sin(wi) * 0.55, 0.12), flach=0.8)
    for i in range(3):
        _stamm(teile, "Feuerholz", stammfarbe(zufall, "#8A5A30", "#5A3A1C", "#E8C48C", "#C49A62"), 0.08, 0.8, (4.8, -T / 2 - 1.3, 0.18), (0, 0, i * 60), ecken=6, seed=seed + 70 + i)
    for k in range(3):
        w.spitze("Flamme", einfarbig("#FF9A3A" if k else "#FFD36A", 0.05, zufall), 0.2, 0.6 + 0.2 * k, (4.8 + (k - 1) * 0.12, -T / 2 - 1.3, 0.2), ecken=5, glut=True)
    setze(w, (-5.9, -6.3, 0.0), 20, brunnen)
    for (x, y) in ((B / 2 + 0.9, -1.5), (B / 2 + 0.9, -0.6), (B / 2 + 1.6, -1.05)):
        fass(w, (x, y, 0.0), 0.9)
    _kiste(w, (B / 2 + 1.0, 0.6, 0.0), 0.6)
    _kiste(w, (B / 2 + 1.05, 0.62, 0.6), 0.45)
    w.brett("Bank", rahmen, (2.2, 0.45, 0.08), (-3.9, -T / 2 - 0.8, 0.48), fase=0.01)
    for x in (-4.8, -3.0):
        w.brett("Bankbein", balken, (0.1, 0.4, 0.45), (x, -T / 2 - 0.8, 0.22), fase=0.01)
    _laterne(teile, licht, zufall, (2.0, -T / 2 - 0.9, 0.0), hoehe=2.3)
    _laterne(teile, licht, zufall, (-2.4, -T / 2 - 0.9, 0.0), hoehe=2.3)
    busch(w, (-6.1, -3.7), 0.6, "#E24A4A")
    busch(w, (6.1, -3.9), 0.6, "#F2C94C")
    busch(w, (-6.2, 2.4), 0.55, "#E88AD0")
    if stufe >= 2:
        setze(w, (5.4, -6.9, 0.0), 0, lambda t: marktstand(t, "#D94A3A", "#F3E6C8"))
        setze(w, (-3.1, -8.1, 0.0), 0, lambda t: marktstand(t, "#3F6FB5", "#F3E6C8"))
    return fertig(w, ["Dorfhalle", "Rathaus", "Burgfried"][stufe - 1])
