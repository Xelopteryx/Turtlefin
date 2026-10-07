"""Dessine le logo de packaging/turtlefin.svg (repère 256) en PNG et ICO. usage: draw.py <dossier de sortie>"""
import sys, os
from PIL import Image, ImageDraw

out = sys.argv[1]
K = 8            # suréchantillonnage
N = 256 * K


def P(x, y):
    return (x * K, y * K)


def stroke(d, pts, w, closed):
    pts = [P(*p) for p in pts]
    segs = list(zip(pts, pts[1:] + pts[:1])) if closed else list(zip(pts, pts[1:]))
    for a, b in segs:
        d.line([a, b], fill=255, width=int(w * K))
    r = w * K / 2
    for x, y in pts:
        d.ellipse([x - r, y - r, x + r, y + r], fill=255)


def logo(background=True):
    mask = Image.new('L', (N, N), 0)
    d = ImageDraw.Draw(mask)
    outer = [(128, 52), (186, 86), (186, 154), (128, 188), (70, 154), (70, 86)]
    inner = [(128, 92), (152, 106), (152, 134), (128, 148), (104, 134), (104, 106)]
    stroke(d, outer, 14, True)
    d.polygon([P(*p) for p in inner], fill=255)
    for a, b in zip(outer, inner):
        stroke(d, [a, b], 8, False)

    # Dégradé #a95bc2 -> #00a4db en diagonale (x1=0,y1=0 -> x2=1,y2=1 de la boîte des formes).
    x0, y0, x1, y1 = 70 * K - 7 * K, 52 * K - 7 * K, 186 * K + 7 * K, 188 * K + 7 * K
    small = Image.new('RGB', (256, 256))
    px = small.load()
    a, b = (0xa9, 0x5b, 0xc2), (0x00, 0xa4, 0xdb)
    for j in range(256):
        for i in range(256):
            u = (i * K - x0) / (x1 - x0)
            v = (j * K - y0) / (y1 - y0)
            t = max(0.0, min(1.0, (u + v) / 2))
            px[i, j] = tuple(round(a[c] + (b[c] - a[c]) * t) for c in range(3))
    grad = small.resize((N, N), Image.BILINEAR)

    img = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    if background:
        bgm = Image.new('L', (N, N), 0)
        ImageDraw.Draw(bgm).rounded_rectangle([0, 0, N - 1, N - 1], radius=56 * K, fill=255)
        img.paste((0x01, 0x0e, 0x18, 255), (0, 0), bgm)
    img.paste(grad, (0, 0), mask)
    return img


big = logo()
os.makedirs(out, exist_ok=True)
for s in (16, 24, 32, 48, 64, 128, 256, 512):
    big.resize((s, s), Image.LANCZOS).save(os.path.join(out, f'turtlefin-{s}.png'))
big.resize((256, 256), Image.LANCZOS).save(
    os.path.join(out, 'turtlefin.ico'), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print('ok')
