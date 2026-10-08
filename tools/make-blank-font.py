"""Polices « vides » pour l'effet de changement de langue (src/dust.rs).

Copie de chaque Montserrat de ui/fonts avec les mêmes largeurs, approches et crénages, mais des
lettres sans dessin : l'interface rendue avec elles est identique, sans aucun texte. Une capture
avec et sans texte donne exactement les pixels des textes.

Version modifiée d'une police sous licence OFL : elle porte un autre nom (« Turtlefin Blank »).

Usage : python tools/make-blank-font.py   (demande fonttools : pip install fonttools)
"""
import pathlib
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen

FONTS = pathlib.Path(__file__).resolve().parent.parent / "ui" / "fonts"
FAMILY = "Turtlefin Blank"
# Caractères de l'interface absents de Montserrat (liste tirée des textes de l'app).
SYMBOLS = [0x21E7, 0x2261, 0x232B, 0x2423, 0x2502, 0x25CF, 0x2605, 0x2B07, 0x2713, 0x2665, 0x2022]

for weight in ["Regular", "Medium", "SemiBold", "Bold"]:
    font = TTFont(FONTS / f"Montserrat-{weight}.ttf")
    glyf = font["glyf"]
    empty = TTGlyphPen(None).glyph()
    for name in font.getGlyphOrder():
        glyf[name] = empty
    # Symboles absents de Montserrat (dessinés par une police de secours du système) : ajoutés,
    # vides eux aussi, pour qu'ils disparaissent avec le reste du texte. Largeur proche d'un symbole.
    order = font.getGlyphOrder() + ["tfsymbol"]
    font.setGlyphOrder(order)
    glyf.glyphOrder = order
    glyf["tfsymbol"] = empty
    upm = font["head"].unitsPerEm
    font["hmtx"].metrics["tfsymbol"] = (int(upm * 0.9), 0)
    font["maxp"].numGlyphs = len(order)
    font["hhea"].numberOfHMetrics = len(font["hmtx"].metrics)
    for table in font["cmap"].tables:
        if table.isUnicode():
            for cp in SYMBOLS:
                if table.format != 4 or cp <= 0xFFFF:
                    table.cmap.setdefault(cp, "tfsymbol")
    # Plus de dessin : les tables de hinting et de couleurs n'ont plus d'objet.
    for tag in ["fpgm", "prep", "cvt ", "gasp", "hdmx", "LTSH", "VDMX", "COLR", "CPAL"]:
        if tag in font:
            del font[tag]
    style = "Regular" if weight == "Regular" else weight
    for rec in font["name"].names:
        if rec.nameID in (1, 16):
            rec.string = FAMILY
        elif rec.nameID in (2, 17):
            rec.string = style
        elif rec.nameID == 4:
            rec.string = f"{FAMILY} {style}"
        elif rec.nameID == 6:
            rec.string = f"TurtlefinBlank-{weight}"
    out = FONTS / f"TurtlefinBlank-{weight}.ttf"
    font.save(out)
    print(out.name, out.stat().st_size // 1024, "Ko")
