"""Erstes Test-Asset: ein bemooster Felsbrocken. Zeigt den ganzen Weg von Blender ins Spiel."""

from werkstatt import boden_abflachen, einfaerben_nach_richtung, kugel, material, ursprung_unten, verbeulen

stein = material("stein")
moos = material("moos")

fels = kugel("Fels", 0.7, stein, stufen=1, ort=(0, 0, 0.35), groesse=(1.3, 1.0, 0.8))
verbeulen(fels, 0.12, seed=7)
boden_abflachen(fels, 0.0)
einfaerben_nach_richtung(fels, moos, grenze=0.75)
ursprung_unten(fels)
