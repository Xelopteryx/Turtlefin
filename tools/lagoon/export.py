# Images et données du décor Aero (lagon), à partir de la démo (lagoon-demo.html) rendue par Edge sans
# fenêtre. Usage : python tools/lagoon/export.py tools/lagoon ui/lagoon
# Les calques sont ensuite recadrés (le récif n'occupe que le bas) et réduits (ce qui est flou), pour la
# mémoire : voir le bas de ce fichier et lagoon.rs.
import io, sys, os, re, json, base64, subprocess
d = sys.argv[1]
out_dir = sys.argv[2]
src = io.open(os.path.join(d, 'lagoon-demo.html'), encoding='utf-8').read()
inject = r"""
  (function () {
    function sprite2(w, h, f) { var c = mk(w, h); f(c.getContext("2d")); return c; }
    var clownR = sprite2(52, 30, function (g) { clownfish(g, 26, 15, 1.1, 1, 0); });
    var clownL = sprite2(52, 30, function (g) { clownfish(g, 26, 15, 1.1, -1, 0); });
    var chromR = sprite2(36, 20, function (g) { chromis(g, 18, 10, 1.1, 1, 0, 0); });
    var chromL = sprite2(36, 20, function (g) { chromis(g, 18, 10, 1.1, -1, 0, 0); });
    var fish = sprite2(32, 14, function (g) { var s = 6; g.fillStyle = "rgba(4,30,70,1)"; g.beginPath(); g.ellipse(20, 7, s * 1.6, s * .6, 0, 0, Math.PI * 2); g.fill(); g.beginPath(); g.moveTo(20 - s * 1.4, 7); g.lineTo(20 - s * 2.4, 7 - s * .7); g.lineTo(20 - s * 2.4, 7 + s * .7); g.fill(); });
    var list = { reef_back: L.reefBack, reef_day: L.reefDay, reef_night: L.reefNight, reef_glow: L.reefGlow, sand: L.sand, rays: L.rays, c1: L.c1, c2: L.c2, win: L.win, aur1: L.aur1, aur2: L.aur2,
      fan_day: L.fanDay, fan_night: L.fanNight, bubble: L.spr, dot: L.dot, jglow: L.jglow, clown_r: clownR, clown_l: clownL, chromis_r: chromR, chromis_l: chromL, fish: fish };
    var o = {}; Object.keys(list).forEach(function (k) { var big = list[k].width > 300; o[k] = list[k].toDataURL(big ? "image/webp" : "image/png", .92); });
    o.data = JSON.stringify({ fans: L.fans, anemones: L.anemones, chromis: L.chromis, jellies: L.jellies.map(function (j) { return { x: j.x, y: j.y, s: j.s, th: j.th, ph: j.ph, side: j.side }; }), cols: L.cols, bub: L.bub, fish: L.fish, plankton: L.plankton });
    var pre = document.createElement("pre"); pre.id = "out"; pre.textContent = JSON.stringify(o); document.body.appendChild(pre);
  })();
"""
i = src.rindex("})();")
page = src[:i] + inject + src[i:]
tmp = os.path.join(d, 'export_lagoon.html')
io.open(tmp, 'w', encoding='utf-8').write(page)
edge = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
url = "file:///" + tmp.replace("\\", "/")
res = subprocess.run([edge, "--headless=new", "--disable-gpu", "--virtual-time-budget=6000", "--dump-dom", url], capture_output=True, text=True, encoding='utf-8')
m = re.search(r'<pre id="out">(.*?)</pre>', res.stdout, re.S)
if not m:
    print("pas de sortie", res.stderr[-500:])
    sys.exit(1)
data = json.loads(m.group(1).replace("&quot;", '"').replace("&amp;", "&"))
os.makedirs(out_dir, exist_ok=True)
io.open(os.path.join(out_dir, "lagoon.json"), "w", encoding="utf-8").write(data.pop("data"))
for k, v in data.items():
    head, b64 = v.split(",", 1)
    ext = "webp" if "webp" in head else "png"
    raw = base64.b64decode(b64)
    io.open(os.path.join(out_dir, k + "." + ext), "wb").write(raw)
    print(k, ext, len(raw))

from PIL import Image
def _save(im, name):
    im.save(os.path.join(out_dir, name), "WEBP", quality=90, method=6)
for k, y in {"reef_back": 480, "reef_day": 480, "reef_night": 480, "reef_glow": 480, "sand": 760}.items():
    _save(Image.open(os.path.join(out_dir, k + ".webp")).convert("RGBA").crop((0, y, 1600, 900)), k + ".webp")
for k, sz in {"rays": (800, 450), "c1": (920, 220), "c2": (920, 220), "aur1": (900, 150), "aur2": (900, 150), "win": (400, 150)}.items():
    _save(Image.open(os.path.join(out_dir, k + ".webp")).convert("RGBA").resize(sz, Image.LANCZOS), k + ".webp")
