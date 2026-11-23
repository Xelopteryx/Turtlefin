"""Police « Turtlefin Symbols » : les seuls symboles de l'interface qui manquent à Montserrat (★ ✓ ✕ ⋯ ● ⌫ ␣ ⇧…),
tirés de DejaVu Sans (licence Bitstream Vera / DejaVu : redistribution permise, nom changé après modification).

Sous Windows et Linux, ces caractères venaient des polices du système ; Android n'en fournit pas à l'appli,
et ils s'affichaient en carrés vides (étoile des notes absente des cartes, vu le 23 novembre 2026).

usage : python tools/make-symbols-font.py <DejaVuSans.ttf> <DejaVuSans-Bold.ttf>
Écrit ui/fonts/TurtlefinSymbols-Regular.ttf et -Bold.ttf (licence : ui/fonts/DejaVu-LICENSE.txt).
"""
import sys
from fontTools import subset
from fontTools.ttLib import TTFont

SYMBOLS = "★✓✕⋯●⌫␣⇧≡⟳✎▏⏭⬇→←↑↓−▶≈"


def build(src, dst, style):
    font = TTFont(src)
    opts = subset.Options()
    opts.layout_features = []
    opts.name_IDs = ["*"]
    opts.notdef_outline = True
    sub = subset.Subsetter(opts)
    sub.populate(text=SYMBOLS)
    sub.subset(font)
    missing = [c for c in SYMBOLS if ord(c) not in font.getBestCmap()]
    if missing:
        print("absents de DejaVu Sans :", "".join(missing))
    family = "Turtlefin Symbols"
    full = f"{family} {style}" if style != "Regular" else family
    ps = f"TurtlefinSymbols-{style}"
    for rec in font["name"].names:
        nid = rec.nameID
        if nid in (1, 16):
            rec.string = family
        elif nid in (2, 17):
            rec.string = style
        elif nid == 3:
            rec.string = f"{ps};DejaVu-derived"
        elif nid == 4:
            rec.string = full
        elif nid == 6:
            rec.string = ps
    font.save(dst)
    print(dst, len(font.getGlyphOrder()), "glyphes")


build(sys.argv[1], "ui/fonts/TurtlefinSymbols-Regular.ttf", "Regular")
build(sys.argv[2], "ui/fonts/TurtlefinSymbols-Bold.ttf", "Bold")
