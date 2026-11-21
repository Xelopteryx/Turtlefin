# Station du logo en orbe (thème Aero) : dessine l'orbe comme l'icône (src/theme.rs, orb_icon) sans
# recompiler Turtlefin, pour essayer des variantes du bord et du reflet blanc.
#   python tools/logo-station/orb.py <dossier de sortie> [taille]
#   python tools/logo-station/orb.py gloss   (réécrit ui/logo/orb_gloss.png, le reflet utilisé par l'appli)
# Écrit une planche (orb_variantes.png) et chaque variante seule (orb_<n>.png).
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont

OUTER = [(128, 60), (187, 94), (187, 162), (128, 196), (69, 162), (69, 94)]
INNER = [(128, 98), (154, 113), (154, 143), (128, 158), (102, 143), (102, 113)]
# Quarts de la carapace (haut clair, bas soutenu) : rouge, vert, bleu, jaune (WIN7_COLORS).
WIN7 = [((255, 143, 94), (226, 64, 28)), ((166, 224, 90), (79, 166, 30)),
        ((92, 192, 255), (20, 119, 214)), ((255, 220, 90), (242, 169, 0))]
DEEP, MID, LIGHT, WHITE = np.array([2, 12, 30.]), np.array([10, 50, 110.]), np.array([40, 140, 230.]), np.array([255, 255, 255.])


def mix(a, b, t):
    t = np.asarray(t)[..., None]
    return a + (b - a) * t


def clamp(v, lo=0.0, hi=1.0):
    return np.clip(v, lo, hi)


def smooth(v):
    v = clamp(v)
    return v * v * (3 - 2 * v)


def seg(px, py, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    h = clamp(((px - a[0]) * dx + (py - a[1]) * dy) / (dx * dx + dy * dy))
    return np.hypot(px - a[0] - dx * h, py - a[1] - dy * h)


def inside(px, py, poly):
    c = np.zeros(px.shape, bool)
    j = len(poly) - 1
    for i in range(len(poly)):
        (ax, ay), (bx, by) = poly[i], poly[j]
        cond = ((ay > py) != (by > py)) & (px < (bx - ax) * (py - ay) / (by - ay + 1e-9) + ax)
        c ^= cond
        j = i
    return c


def render(n, variant):
    ss = 3
    m = n * ss
    px = 256.0 / m
    y, x = np.mgrid[0:m, 0:m].astype(float) + 0.5
    sx, sy = x * px, y * px
    cov = lambda d: clamp(0.5 - d / px)
    r = np.hypot(sx - 128, sy - 128)
    a = cov(r - 122)
    ang = np.arctan2(sy - 128, sx - 128)  # -pi/2 : en haut
    top = clamp(-np.sin(ang))            # 1 en haut du cercle, 0 à l'horizontale et dessous

    # Sphère (inchangée).
    rl = np.hypot(sx - 128, sy - 150) / 130
    c = mix(mix(LIGHT, MID, clamp(rl * 1.4)), DEEP, clamp((rl - 0.55) * 1.6))
    rim = clamp((r - 92) / 28) * clamp((sy - 128) / 110)
    c = mix(c, LIGHT, 0.55 * rim)

    # --- Bord ---
    dark = 0.6 * clamp((r - 112) / 8)
    line = 1 - clamp(np.abs(r - 120.5) / 1.5)
    if variant == 1:
        c = mix(c, DEEP, dark)
        c = mix(c, WHITE, 0.35 * line)
    elif variant == 2:
        # Liseré clair atténué en haut, là où l'ovale le croise.
        c = mix(c, DEEP, dark)
        c = mix(c, WHITE, 0.35 * line * (1 - 0.85 * smooth(top * 1.6)))
    elif variant == 3:
        c = mix(c, DEEP, dark)
        c = mix(c, WHITE, 0.35 * line * (1 - 0.6 * smooth(top * 1.6)))
    elif variant == 4:
        # Liseré qui s'éclaire vers le bas (la lumière traverse le verre), sombre en haut.
        c = mix(c, DEEP, dark * (1 + 0.3 * top))
        c = mix(c, WHITE, line * (0.12 + 0.33 * clamp((sy - 100) / 150)))
    else:
        # Bord épais de verre : ombre intérieure en haut, anneau de lumière cyan en bas, double liseré.
        c = mix(c, DEEP, clamp((r - 106) / 12) * (0.55 + 0.35 * top))
        c = mix(c, np.array([120, 220, 255.]), 0.45 * line * clamp((sy - 110) / 130))
        c = mix(c, WHITE, 0.1 * line)
        inner_line = 1 - clamp(np.abs(r - 113) / 1.2)
        c = mix(c, np.array([150, 220, 255.]), 0.25 * inner_line * clamp((sy - 150) / 90))

    # Carapace (inchangée).
    lx, ly = 128 + (sx - 128) / 1.12, 128 + (sy - 128) / 1.12

    def shell(qx, qy):
        d = np.full(qx.shape, np.inf)
        for i in range(6):
            d = np.minimum(d, seg(qx, qy, OUTER[i], OUTER[(i + 1) % 6]) - 6.5)
            d = np.minimum(d, seg(qx, qy, OUTER[i], INNER[i]) - 4.0)
        return d
    dist = shell(lx, ly)
    c = mix(c, DEEP, 0.5 * cov(shell(lx, ly - 2) * 1.12))
    tx, ty = smooth((lx - 128) / 40 + 0.5), smooth((ly - 128) / 40 + 0.5)
    midc = lambda k: (np.array(WIN7[k][0], float) + np.array(WIN7[k][1], float)) / 2
    topc = mix(midc(0), midc(1), tx)
    botc = mix(midc(2), midc(3), tx)
    sc = topc + (botc - topc) * ty[..., None]
    sc = mix(sc, WHITE, 0.4 * (1 - cov(shell(lx, ly + 2.5) * 1.12)))
    c = c + (sc - c) * cov(dist * 1.12)[..., None]
    dc = np.full(lx.shape, np.inf)
    for i in range(6):
        dc = np.minimum(dc, seg(lx, ly, INNER[i], INNER[(i + 1) % 6]))
    dc = np.where(inside(lx, ly, INNER), -dc, dc)
    g = mix(WHITE, np.array([205, 230, 255.]), clamp((ly - 98) / 60))
    c = c + (g - c) * cov(dc * 1.12)[..., None]

    # --- Reflet blanc ---
    if variant in (1, 2):
        # L'ancien : grande ellipse en haut.
        e = ((sx - 128) / 96) ** 2 + ((sy - 66) / 54) ** 2
        w = np.sqrt(clamp(1 - e)) * (0.62 - 0.5 * clamp((sy - 12) / 108))
        c = mix(c, WHITE, np.where(e < 1, np.maximum(w, 0), 0))
    elif variant == 3:
        # L'ancien, mais il s'efface avant le bord du cercle (ne le touche plus).
        e = ((sx - 128) / 96) ** 2 + ((sy - 66) / 54) ** 2
        w = np.sqrt(clamp(1 - e)) * (0.62 - 0.5 * clamp((sy - 12) / 108))
        w = w * (1 - smooth((r - 104) / 12))
        c = mix(c, WHITE, np.where(e < 1, np.maximum(w, 0), 0))
    else:
        # Croissant façon bouton Démarrer : le haut suit le cercle (en retrait), le bas est une courbe
        # plus plate ; dégradé vertical, bords adoucis.
        inset = 112 if variant == 4 else 108
        in_circle = 1 - smooth((r - (inset - 6)) / 6)
        e = ((sx - 128) / 118) ** 2 + ((sy - 52) / 70) ** 2
        in_ell = 1 - smooth((np.sqrt(e) - 0.96) / 0.06)
        grad = 0.62 - 0.55 * clamp((sy - 16) / 100)
        w = in_circle * in_ell * np.maximum(grad, 0)
        c = mix(c, WHITE, w)
        if variant == 5:
            # Lueur douce sous le croissant, reflet de la pièce en bas, petit éclat.
            caust = np.exp(-(((sx - 128) / 70) ** 2 + ((sy - 205) / 22) ** 2))
            c = mix(c, np.array([150, 225, 255.]), 0.35 * caust * a)
            spark = np.exp(-(((sx - 72) / 6) ** 2 + ((sy - 58) / 6) ** 2))
            c = mix(c, WHITE, 0.7 * spark)

    img = np.dstack([clamp(c / 255), a[..., None]])
    im = Image.fromarray((img * 255).round().astype(np.uint8), "RGBA")
    return im.resize((n, n), Image.LANCZOS)


NAMES = ["1. L'ancienne (référence)", "2. Liseré atténué sous l'ovale", "3. Ovale qui s'efface avant le bord",
         "4. Reflet en croissant, liseré clair en bas", "5. Verre travaillé : croissant, bord épais, lueurs"]

if __name__ == "__main__" and sys.argv[1:2] == ["gloss"]:
    export_gloss(sys.argv[2] if len(sys.argv) > 2 else "ui/logo/orb_gloss.png")
elif __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "."
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 320
    pad, lab = 24, 46
    sheet = Image.new("RGBA", (5 * (n + pad) + pad, n + 2 * pad + lab), (14, 80, 170, 255))
    d = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("segoeui.ttf", 15)
    except OSError:
        font = ImageFont.load_default()
    for k in range(5):
        im = render(n, k + 1)
        im.save(f"{out}/orb_{k + 1}.png")
        x = pad + k * (n + pad)
        sheet.alpha_composite(im, (x, pad))
        d.text((x + n / 2, pad + n + 14), NAMES[k], fill="white", font=font, anchor="ma")
    sheet.save(f"{out}/orb_variantes.png")


def export_gloss(path, n=512):
    """Reflet de la variante 3 seul (blanc, transparence), sur le carré de l'orbe : ui/logo/orb_gloss.png,
    posé par ThemeLogo et le démarrage sur l'orbe (même formule que orb_icon, repère de rayon 122)."""
    y, x = np.mgrid[0:n, 0:n].astype(float) + 0.5
    sx, sy = 128 + (x / n * 2 - 1) * 122, 128 + (y / n * 2 - 1) * 122
    r = np.hypot(sx - 128, sy - 128)
    e = ((sx - 128) / 96) ** 2 + ((sy - 66) / 54) ** 2
    w = np.sqrt(clamp(1 - e)) * (0.62 - 0.5 * clamp((sy - 12) / 108))
    w = np.where(e < 1, np.maximum(w, 0), 0) * (1 - smooth((r - 104) / 12))
    img = np.dstack([np.ones((n, n, 3)), w[..., None]])
    Image.fromarray((img * 255).round().astype(np.uint8), "RGBA").save(path)
