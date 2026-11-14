"""Images des télés et des boutiques pour chaque thème intégré : bannière 16:9 (Android TV, lanceur) et
affiche 2:3, dessinées avec Pillow à partir des logos des thèmes (android/res/drawable-nodpi/ic_fg_*.png,
eux-mêmes tirés de theme.rs) et des polices de l'appli.

usage : python packaging/icons/make-tv-art.py        (depuis la racine du dépôt)
Écrit :
  android/res/drawable-xhdpi/banner_<thème>.png     320 x 180 (bannière Android TV)
  android/res/drawable-xxhdpi/banner_<thème>.png    480 x 270
  packaging/tv/<thème>-landscape.png               1920 x 1080
  packaging/tv/<thème>-poster.png                  1000 x 1500
"""
import math, os, random
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FONTS = os.path.join(ROOT, 'ui', 'fonts')

THEMES = {
    # fond (haut, bas), accents, police du nom, ton du sous-titre
    'turtlefin': dict(bg=('#04182a', '#010e18'), a1='#a95bc2', a2='#00a4db', font='Montserrat-Bold.ttf', ink='#ffffff'),
    'aero': dict(bg=('#0b4fa8', '#05214f'), a1='#2a7fd8', a2='#6cc6ff', font='Selawik-Bold.ttf', ink='#ffffff'),
    'nerd': dict(bg=('#00140a', '#000201'), a1='#00b864', a2='#2ef0c8', font='JetBrainsMonoNL-Bold.ttf', ink='#d8ffef'),
}


def rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def mix(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def gradient(w, h, top, bottom):
    img = Image.new('RGB', (w, h))
    d = ImageDraw.Draw(img)
    for y in range(h):
        d.line([(0, y), (w, y)], fill=mix(top, bottom, y / max(1, h - 1)))
    return img


def glow(img, cx, cy, r, color, strength):
    """Halo doux (lumière) autour d'un point."""
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=color + (round(255 * strength),))
    layer = layer.filter(ImageFilter.GaussianBlur(r * 0.45))
    img.alpha_composite(layer)


def decor(img, key, t):
    """Décor propre au thème, discret : hexagones (Turtlefin), reflets et bulles (Aero), pistes (Nerd)."""
    w, h = img.size
    s = min(w, h)
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    rnd = random.Random(7)
    a1, a2 = rgb(t['a1']), rgb(t['a2'])
    if key == 'turtlefin':
        r = s * 0.09
        for row in range(-1, int(h / (r * 1.5)) + 2):
            for col in range(-1, int(w / (r * 1.73)) + 2):
                cx = col * r * 1.732 + (row % 2) * r * 0.866
                cy = row * r * 1.5
                pts = [(cx + r * math.cos(math.radians(60 * k + 30)), cy + r * math.sin(math.radians(60 * k + 30))) for k in range(6)]
                fade = max(0.0, 1 - math.hypot(cx - w * 0.7, cy - h * 0.5) / (max(w, h) * 0.75))
                d.line(pts + pts[:1], fill=mix(a1, a2, cx / w) + (round(70 * fade),), width=max(1, round(s * 0.004)))
    elif key == 'aero':
        for k in range(5):
            y0 = h * (0.15 + k * 0.17)
            d.polygon([(0, y0), (w, y0 - h * 0.35), (w, y0 - h * 0.31), (0, y0 + h * 0.05)], fill=(255, 255, 255, 18 - k * 2))
        for _ in range(26):
            r = s * rnd.uniform(0.01, 0.05)
            cx, cy = rnd.uniform(0, w), rnd.uniform(0, h)
            d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(220, 240, 255, 90), width=max(1, round(r * 0.12)))
            d.ellipse([cx - r * 0.45, cy - r * 0.6, cx - r * 0.05, cy - r * 0.25], fill=(255, 255, 255, 70))
    else:
        step = s * 0.06
        for _ in range(int(w * h / (step * step) * 0.05)):
            x = round(rnd.uniform(0, w) / step) * step
            y = round(rnd.uniform(0, h) / step) * step
            pts = [(x, y)]
            for _ in range(rnd.randint(2, 5)):
                dx, dy = rnd.choice([(1, 0), (0, 1), (1, 1), (-1, 1)])
                n = rnd.randint(1, 4)
                x, y = x + dx * step * n, y + dy * step * n
                pts.append((x, y))
            d.line(pts, fill=a1 + (60,), width=max(1, round(s * 0.005)))
            ex, ey = pts[-1]
            rr = s * 0.008
            d.ellipse([ex - rr, ey - rr, ex + rr, ey + rr], fill=a2 + (110,))
    img.alpha_composite(layer)


def art(key, w, h, portrait=False):
    t = THEMES[key]
    img = gradient(w, h, rgb(t['bg'][0]), rgb(t['bg'][1])).convert('RGBA')
    decor(img, key, t)
    s = min(w, h)
    logo = Image.open(os.path.join(ROOT, 'android', 'res', 'drawable-nodpi', f'ic_fg_{key}.png')).convert('RGBA')
    # Le logo occupe le centre de son image (zone sûre des icônes adaptatives) : on le recadre.
    bbox = logo.getbbox()
    logo = logo.crop(bbox)
    if portrait:
        ls = round(w * 0.56)
        lx, ly = (w - ls) // 2, round(h * 0.25)
    else:
        ls = round(h * 0.58)
        lx, ly = round(w * 0.1), (h - ls) // 2
    glow(img, lx + ls / 2, ly + ls / 2, ls * 0.75, rgb(t['a2']), 0.38)
    img.alpha_composite(logo.resize((ls, round(ls * logo.height / logo.width)), Image.LANCZOS), (lx, ly))
    d = ImageDraw.Draw(img)
    name = 'Turtlefin'
    if portrait:
        size = round(w * 0.15)
        font = ImageFont.truetype(os.path.join(FONTS, t['font']), size)
        tw = d.textlength(name, font=font)
        d.text(((w - tw) / 2, ly + ls + h * 0.06), name, font=font, fill=rgb(t['ink']))
        sub = ImageFont.truetype(os.path.join(FONTS, t['font']), round(size * 0.32))
        tag = 'Jellyfin client'
        sw = d.textlength(tag, font=sub)
        d.text(((w - sw) / 2, ly + ls + h * 0.06 + size * 1.3), tag, font=sub, fill=rgb(t['a2']))
    else:
        size = round(h * 0.2)
        font = ImageFont.truetype(os.path.join(FONTS, t['font']), size)
        tx = lx + ls + w * 0.05
        # Le nom tient dans la largeur restante (réduit au besoin).
        while d.textlength(name, font=font) > w - tx - w * 0.06 and size > 8:
            size -= 1
            font = ImageFont.truetype(os.path.join(FONTS, t['font']), size)
        ty = h / 2 - size * 0.72
        d.text((tx, ty), name, font=font, fill=rgb(t['ink']))
        sub = ImageFont.truetype(os.path.join(FONTS, t['font']), max(6, round(size * 0.34)))
        d.text((tx + size * 0.04, ty + size * 1.22), 'Jellyfin client', font=sub, fill=rgb(t['a2']))
    # Filet d'accent en bas.
    bar = round(s * 0.012)
    for x in range(w):
        d.line([(x, h - bar), (x, h)], fill=mix(rgb(t['a1']), rgb(t['a2']), x / w))
    return img.convert('RGB')


def main():
    out_tv = os.path.join(ROOT, 'packaging', 'tv')
    os.makedirs(out_tv, exist_ok=True)
    for key in THEMES:
        for dpi, (w, h) in (('xhdpi', (320, 180)), ('xxhdpi', (480, 270))):
            d = os.path.join(ROOT, 'android', 'res', f'drawable-{dpi}')
            os.makedirs(d, exist_ok=True)
            art(key, w * 4, h * 4).resize((w, h), Image.LANCZOS).save(os.path.join(d, f'banner_{key}.png'), optimize=True)
        art(key, 1920, 1080).save(os.path.join(out_tv, f'{key}-landscape.png'), optimize=True)
        art(key, 1000, 1500, portrait=True).save(os.path.join(out_tv, f'{key}-poster.png'), optimize=True)
        print(key, 'ok')


if __name__ == '__main__':
    main()
