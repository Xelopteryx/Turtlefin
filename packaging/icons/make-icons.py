"""Icônes de Turtlefin (PNG de plusieurs tailles et ICO Windows), dessinées avec Pillow.

Même géométrie que packaging/turtlefin.svg et l'animation de démarrage (ui/boot.slint) : repère 256,
centre (128, 128), hexagone de rayon 68 (trait 14), rayons (trait 8), hexagone central plein,
bouts et angles arrondis, dégradé #a95bc2 -> #00a4db à 135° sur le carré [53, 203] ; le tout agrandi
x1,4 autour du centre sur un carré sombre aux coins arrondis.

usage : python make-icons.py <dossier de sortie>
Produit aussi les images de l'installeur Windows (wizard-*.bmp, à copier dans packaging/windows/).
"""
import sys, os
from PIL import Image, ImageDraw

out = sys.argv[1]
K = 8              # suréchantillonnage
N = 256 * K
SCALE = 1.4        # agrandissement du logo dans l'icône

OUTER = [(128, 60), (187, 94), (187, 162), (128, 196), (69, 162), (69, 94)]
INNER = [(128, 98), (154, 113), (154, 143), (128, 158), (102, 143), (102, 113)]


def P(x, y):
    """Repère du logo -> pixels de l'image (agrandissement autour du centre)."""
    return ((128 + (x - 128) * SCALE) * K, (128 + (y - 128) * SCALE) * K)


def stroke(d, pts, w, closed):
    pts = [P(*p) for p in pts]
    w = w * SCALE * K
    segs = list(zip(pts, pts[1:] + pts[:1])) if closed else list(zip(pts, pts[1:]))
    for a, b in segs:
        d.line([a, b], fill=255, width=round(w))
    r = w / 2
    for x, y in pts:
        d.ellipse([x - r, y - r, x + r, y + r], fill=255)


def logo():
    mask = Image.new('L', (N, N), 0)
    d = ImageDraw.Draw(mask)
    stroke(d, OUTER, 14, True)
    for a, b in zip(OUTER, INNER):
        stroke(d, [a, b], 8, False)
    d.polygon([P(*p) for p in INNER], fill=255)

    # Dégradé : t = 0 en (53, 53), 1 en (203, 203) du repère du logo.
    small = Image.new('RGB', (256, 256))
    px = small.load()
    a, b = (0xa9, 0x5b, 0xc2), (0x00, 0xa4, 0xdb)
    for j in range(256):
        for i in range(256):
            x = 128 + (i + 0.5 - 128) / SCALE
            y = 128 + (j + 0.5 - 128) / SCALE
            t = max(0.0, min(1.0, (x - 53 + y - 53) / 300))
            px[i, j] = tuple(round(a[c] + (b[c] - a[c]) * t) for c in range(3))
    grad = small.resize((N, N), Image.BILINEAR)

    img = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    bg = Image.new('L', (N, N), 0)
    ImageDraw.Draw(bg).rounded_rectangle([0, 0, N - 1, N - 1], radius=56 * K, fill=255)
    img.paste((0x01, 0x0e, 0x18, 255), (0, 0), bg)
    img.paste(grad, (0, 0), mask)
    return img


big = logo()
os.makedirs(out, exist_ok=True)
for s in (16, 24, 32, 48, 64, 128, 256, 512):
    big.resize((s, s), Image.LANCZOS).save(os.path.join(out, f'turtlefin-{s}.png'))
big.resize((256, 256), Image.LANCZOS).save(
    os.path.join(out, 'turtlefin.ico'), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

# Installeur (Inno Setup) : petite image du bandeau (55 x 58, et x2) et grande image de côté
# (164 x 314, et x2), en BMP sans transparence, fond sombre du thème.
here = os.path.dirname(os.path.abspath(__file__))
font_path = os.path.join(here, '..', '..', 'ui', 'fonts', 'Montserrat-Bold.ttf')
for scale in (1, 2):
    w, h = 55 * scale, 58 * scale
    img = Image.new('RGB', (w, h), (0x01, 0x0e, 0x18))
    logo_img = big.resize((min(w, h), min(w, h)), Image.LANCZOS)
    img.paste(logo_img, ((w - logo_img.width) // 2, (h - logo_img.height) // 2), logo_img)
    img.save(os.path.join(out, f'wizard-small-{scale}x.bmp'))

    w, h = 164 * scale, 314 * scale
    img = Image.new('RGB', (w, h), (0x01, 0x0e, 0x18))
    side = int(w * 0.8)
    logo_img = big.resize((side, side), Image.LANCZOS)
    img.paste(logo_img, ((w - side) // 2, int(h * 0.22)), logo_img)
    from PIL import ImageFont
    d = ImageDraw.Draw(img)
    f = ImageFont.truetype(font_path, 22 * scale)
    tw = d.textlength('Turtlefin', font=f)
    d.text(((w - tw) / 2, int(h * 0.22) + side + 14 * scale), 'Turtlefin', font=f, fill=(0x00, 0xa4, 0xdb))
    img.save(os.path.join(out, f'wizard-large-{scale}x.bmp'))
print('ok')
