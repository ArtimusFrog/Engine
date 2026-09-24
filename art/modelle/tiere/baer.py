"""Braunbär – stilisiert, mit Skelett und den Animationen Idle, Laufen und Rennen.

Maße: etwa 2,1 m lang, 1,4 m hoch (Schulter ~1,2 m). Schaut nach -Y (Blender „Vorne“).
"""

from werkstatt import (
    animation,
    binden,
    knochen_zuweisen,
    kugel,
    material,
    ruhepose,
    skelett,
    verbeulen,
    vereinen,
    zylinder,
)

fell = material("baer")
fell_dunkel = material("baer_dunkel")
schnauze = material("baer_schnauze")
schwarz = material("schwarz", rau=0.4)

# ---------------------------------------------------------------------------
# Körperteile (jeweils einem Knochen zugeordnet)
# ---------------------------------------------------------------------------
teile = []


def teil(obj, knochen, beule=0.0, seed=1):
    if beule:
        verbeulen(obj, beule, seed)
    knochen_zuweisen(obj, knochen)
    teile.append(obj)
    return obj


# Rumpf: massiger Körper mit Schulterbuckel, hinten etwas schmaler
teil(kugel("Rumpf", 1.0, fell, stufen=2, ort=(0, 0.05, 0.95), groesse=(0.52, 0.95, 0.47)), "Koerper", 0.03, 1)
teil(kugel("Buckel", 0.42, fell, stufen=2, ort=(0, -0.38, 1.18), groesse=(1.0, 1.0, 0.85)), "Koerper", 0.02, 2)
teil(kugel("Hinterteil", 0.46, fell, stufen=2, ort=(0, 0.62, 0.98), groesse=(1.0, 0.9, 0.9)), "Koerper", 0.02, 3)

# Kopf mit Schnauze, Nase, Augen, Ohren
teil(kugel("Kopf", 0.33, fell, stufen=2, ort=(0, -1.02, 1.2), groesse=(1.0, 1.05, 0.92)), "Kopf", 0.015, 4)
teil(kugel("Schnauze", 0.17, schnauze, stufen=2, ort=(0, -1.33, 1.1), groesse=(0.95, 1.3, 0.78)), "Kopf")
teil(kugel("Nase", 0.065, schwarz, stufen=1, ort=(0, -1.54, 1.15), groesse=(1.2, 0.9, 0.8)), "Kopf")
for x in (-1, 1):
    teil(kugel(f"Auge{x}", 0.04, schwarz, stufen=1, ort=(0.14 * x, -1.27, 1.3)), "Kopf")
    teil(kugel(f"Ohr{x}", 0.1, fell, stufen=1, ort=(0.21 * x, -0.95, 1.47), groesse=(1.0, 0.55, 1.0)), "Kopf")
    teil(kugel(f"OhrInnen{x}", 0.06, fell_dunkel, stufen=1, ort=(0.21 * x, -1.0, 1.46), groesse=(1.0, 0.3, 1.0)), "Kopf")

# Beine: oben dick, unten mit dunkler Tatze. (Name, x, y)
for name, x, y in (("VL", 0.3, -0.62), ("VR", -0.3, -0.62), ("HL", 0.3, 0.6), ("HR", -0.3, 0.6)):
    knochen = f"Bein.{name}"
    teil(zylinder(f"Bein{name}", 0.16, 0.8, fell, ecken=8, radius_oben=0.21, ort=(x, y, 0.08)), knochen, 0.012, len(teile))
    teil(kugel(f"Tatze{name}", 0.17, fell_dunkel, stufen=1, ort=(x, y - 0.05, 0.09), groesse=(1.0, 1.25, 0.55)), knochen)

# Stummelschwanz
teil(kugel("Schwanz", 0.1, fell, stufen=1, ort=(0, 1.08, 1.08)), "Schwanz")

baer = vereinen("Baer", teile)

# ---------------------------------------------------------------------------
# Skelett: (Name, Kopf, Ende, Eltern, Oben)
# ---------------------------------------------------------------------------
VORNE = (0, 0, 1)   # Rumpf/Kopf zeigen nach vorne (-Y), lokale Z-Achse nach oben
UNTEN = (0, 1, 0)   # Beine zeigen nach unten, Drehung um X schwingt vor/zurück
armatur = skelett("BaerSkelett", [
    ("Koerper", (0, 0.7, 0.95), (0, -0.7, 0.95), None, VORNE),
    ("Kopf", (0, -0.8, 1.15), (0, -1.45, 1.15), "Koerper", VORNE),
    ("Bein.VL", (0.3, -0.62, 0.85), (0.3, -0.62, 0.08), "Koerper", UNTEN),
    ("Bein.VR", (-0.3, -0.62, 0.85), (-0.3, -0.62, 0.08), "Koerper", UNTEN),
    ("Bein.HL", (0.3, 0.6, 0.85), (0.3, 0.6, 0.08), "Koerper", UNTEN),
    ("Bein.HR", (-0.3, 0.6, 0.85), (-0.3, 0.6, 0.08), "Koerper", UNTEN),
    ("Schwanz", (0, 1.0, 1.05), (0, 1.2, 1.05), "Koerper", VORNE),
])
binden(baer, armatur)

# ---------------------------------------------------------------------------
# Animationen (30 Bilder pro Sekunde, Schleifen: letztes Bild = erstes)
# ---------------------------------------------------------------------------
def rot(bild, knochen, x=0.0, y=0.0, z=0.0):
    return (bild, knochen, "rot", (x, y, z))


def pos(bild, knochen, x=0.0, y=0.0, z=0.0):
    return (bild, knochen, "pos", (x, y, z))


# Idle: ruhiges Atmen, Kopf schaut sich langsam um, Schwanz zuckt.
idle = []
for bild, atmen, blick, nicken in ((0, 0.0, -8, 0), (30, 0.025, 0, -5), (60, 0.0, 10, 0), (90, 0.025, 0, -4), (120, 0.0, -8, 0)):
    idle += [pos(bild, "Koerper", z=atmen), rot(bild, "Kopf", x=nicken, z=blick)]
for bild, wedeln in ((0, 0), (50, 0), (54, 15), (58, -10), (62, 0), (120, 0)):
    idle.append(rot(bild, "Schwanz", z=wedeln))
animation(armatur, "Idle", 120, idle)

# Laufen: gemächlicher Pass – diagonale Beinpaare schwingen gegeneinander, Körper wippt.
SCHRITT = 24
A = 24  # Grad
laufen = []
for bild, vorne in ((0, 1), (SCHRITT // 2, -1), (SCHRITT, 1)):
    laufen += [
        rot(bild, "Bein.VL", x=A * vorne), rot(bild, "Bein.HR", x=A * vorne),
        rot(bild, "Bein.VR", x=-A * vorne), rot(bild, "Bein.HL", x=-A * vorne),
        rot(bild, "Kopf", x=-3 * vorne),
    ]
for bild, hoehe in ((0, 0.0), (SCHRITT // 4, 0.035), (SCHRITT // 2, 0.0), (3 * SCHRITT // 4, 0.035), (SCHRITT, 0.0)):
    laufen.append(pos(bild, "Koerper", z=hoehe))
animation(armatur, "Laufen", SCHRITT, laufen)

# Rennen: Galopp – Vorderbeine zusammen, Hinterbeine zusammen, Körper schaukelt.
SPRUNG = 16
B = 38
rennen = []
for bild, phase in ((0, 1), (SPRUNG // 2, -1), (SPRUNG, 1)):
    rennen += [
        rot(bild, "Bein.VL", x=B * phase), rot(bild, "Bein.VR", x=B * phase * 0.8),
        rot(bild, "Bein.HL", x=-B * phase), rot(bild, "Bein.HR", x=-B * phase * 0.8),
        rot(bild, "Koerper", x=5 * phase), rot(bild, "Kopf", x=-6 * phase),
    ]
for bild, hoehe in ((0, 0.0), (SPRUNG // 4, 0.09), (SPRUNG // 2, 0.0), (3 * SPRUNG // 4, 0.06), (SPRUNG, 0.0)):
    rennen.append(pos(bild, "Koerper", z=hoehe))
animation(armatur, "Rennen", SPRUNG, rennen)

ruhepose(armatur)
