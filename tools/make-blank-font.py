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

for weight in ["Regular", "Medium", "SemiBold", "Bold"]:
    font = TTFont(FONTS / f"Montserrat-{weight}.ttf")
    glyf = font["glyf"]
    empty = TTGlyphPen(None).glyph()
    for name in font.getGlyphOrder():
        glyf[name] = empty
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
