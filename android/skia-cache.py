"""Skia pour l'APK, sans rien faire à la main (appelé par build-apk.ps1).

skia-bindings télécharge Skia déjà compilé depuis GitHub, mais seulement pour certaines cibles
(pas armeabi-v7a). Sinon il compile Skia depuis ses sources, rangées dans le dossier « skia » de
la crate (cache Cargo). Sous Windows, il échoue à les extraire : l'archive contient des liens
symboliques, que Windows refuse sans le mode développeur (os error 1314).

  prepare <dossier target> <dossier cache> <cibles…>
      Si chaque cible a déjà sa Skia compilée dans le cache : affiche l'adresse file:// à donner à
      SKIA_BINARIES_URL (rien n'est recompilé, même après un « cargo clean »). Sinon, prépare les
      sources (téléchargées et extraites, liens remplacés par des copies) et n'affiche rien.
  save <dossier target> <dossier cache>
      Range dans le cache chaque Skia compilée (ou téléchargée) trouvée dans le dossier target.
"""
import glob, io, os, posixpath, re, shutil, subprocess, sys, tarfile, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))


def bindings_version():
    lock = io.open(os.path.join(HERE, 'Cargo.lock'), encoding='utf-8').read()
    m = re.search(r'name = "skia-bindings"\s*\nversion = "([^"]+)"', lock)
    if not m:
        sys.exit('skia-cache : skia-bindings absent de android/Cargo.lock')
    return m.group(1)


def cache_dir(root):
    d = os.path.join(root, bindings_version())
    os.makedirs(d, exist_ok=True)
    return d


def cached(root, triple):
    return glob.glob(os.path.join(cache_dir(root), f'skia-binaries-*-{triple}-*.tar.gz'))


def crate_dir():
    home = os.environ.get('CARGO_HOME') or os.path.join(os.path.expanduser('~'), '.cargo')
    pattern = os.path.join(home, 'registry', 'src', '*', 'skia-bindings-' + bindings_version())
    found = glob.glob(pattern)
    if not found:
        # Crate pas encore téléchargée (cache Cargo vidé) : cargo fetch la remet.
        subprocess.run(['cargo', 'fetch', '--manifest-path', os.path.join(HERE, 'Cargo.toml')], check=True)
        found = glob.glob(pattern)
    if not found:
        sys.exit('skia-cache : crate skia-bindings introuvable dans ' + home)
    return found[0]


def extract(archive, dest):
    """Extrait l'archive de Skia : sans infra/ (chemins trop longs), liens remplacés par des copies."""
    links = []
    with tarfile.open(archive) as t:
        for m in t:
            parts = m.name.split('/', 1)
            if len(parts) < 2 or not parts[1]:
                continue
            rel = parts[1].split('/')
            if rel[0] == 'infra' or '..' in rel:
                continue
            out = os.path.join(dest, *rel)
            if m.isdir():
                os.makedirs(out, exist_ok=True)
            elif m.issym():
                links.append(('/'.join(rel), m.linkname))
            elif m.isfile():
                os.makedirs(os.path.dirname(out), exist_ok=True)
                with t.extractfile(m) as f, open(out, 'wb') as o:
                    shutil.copyfileobj(f, o)
    root = os.path.abspath(dest)
    for _ in range(5):  # un lien peut viser un autre lien
        rest = []
        for rel, target in links:
            src = posixpath.normpath(posixpath.join(posixpath.dirname(rel), target))
            s, d = os.path.join(dest, *src.split('/')), os.path.join(dest, *rel.split('/'))
            if not os.path.abspath(s).startswith(root):
                continue
            if os.path.isdir(s):
                shutil.copytree(s, d, dirs_exist_ok=True)
            elif os.path.isfile(s):
                os.makedirs(os.path.dirname(d), exist_ok=True)
                shutil.copyfile(s, d)
            else:
                rest.append((rel, target))
        links = rest


def ensure_sources():
    crate = crate_dir()
    skia = os.path.join(crate, 'skia')
    if os.path.isdir(skia) and os.listdir(skia):
        return
    toml = io.open(os.path.join(crate, 'Cargo.toml'), encoding='utf-8').read()
    m = re.search(r'\[package\.metadata\]\s*\nskia = "([^"]+)"', toml)
    if not m:
        sys.exit('skia-cache : version des sources de Skia introuvable')
    tag = m.group(1)
    # Restes d'une extraction ratée par skia-bindings lui-même.
    for d in glob.glob(os.path.join(crate, 'skia-*')):
        shutil.rmtree(d, ignore_errors=True)
    archive = os.path.join(os.environ.get('TEMP', crate), f'skia-{tag}.tar.gz')
    url = f'https://codeload.github.com/rust-skia/skia/tar.gz/{tag}'
    print(f'skia-cache : sources de Skia {tag} ({url})', file=sys.stderr)
    urllib.request.urlretrieve(url, archive)
    tmp = skia + '.partiel'
    shutil.rmtree(tmp, ignore_errors=True)
    extract(archive, tmp)
    os.replace(tmp, skia)
    os.remove(archive)


def prepare(target, root, triples):
    if all(cached(root, t) for t in triples):
        d = cache_dir(root).replace('\\', '/')
        print(f'file://{d}/skia-binaries-{{key}}.tar.gz')
        return
    ensure_sources()


# Contenu d'une archive de binaires (comme celles de GitHub) : bibliothèques, liaisons, clé, version.
LIBS = ('libskia.a', 'libskia-bindings.a', 'bindings.rs')


def key_and_tag(out):
    """Clé et version de la Skia d'un dossier de build : key.txt / tag.txt pour une Skia téléchargée ;
    pour une Skia compilée depuis ses sources, la clé qu'elle a d'abord essayé de télécharger
    (sortie du script de build, gardée par Cargo dans le fichier « output »)."""
    key_file, tag_file = os.path.join(out, 'key.txt'), os.path.join(out, 'tag.txt')
    if os.path.isfile(key_file) and os.path.isfile(tag_file):
        return open(key_file, encoding='utf-8').read().strip(), open(tag_file, encoding='utf-8').read().strip()
    log = os.path.join(os.path.dirname(os.path.dirname(out)), 'output')
    if os.path.isfile(log):
        m = re.search(r'TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: ([^/\s]+)/(\S+)', open(log, encoding='utf-8', errors='replace').read())
        if m:
            return m.group(2), m.group(1)
    return None, None


def save(target, root):
    version = bindings_version()
    best = {}
    for out in glob.glob(os.path.join(target, '*', 'release', 'build', 'skia-bindings-*', 'out', 'skia')):
        if not all(os.path.isfile(os.path.join(out, f)) for f in LIBS):
            continue
        key, tag = key_and_tag(out)
        if not key or tag != version:
            continue
        t = os.path.getmtime(os.path.join(out, 'libskia.a'))
        if key not in best or t > best[key][0]:
            best[key] = (t, out)
    for key, (_, out) in best.items():
        dest = os.path.join(cache_dir(root), f'skia-binaries-{key}.tar.gz')
        if os.path.exists(dest):
            continue
        with tarfile.open(dest + '.partiel', 'w:gz') as t:
            for name in LIBS:
                t.add(os.path.join(out, name), arcname='skia-binaries/' + name)
            for name, text in (('key.txt', key), ('tag.txt', version)):
                data = text.encode('utf-8')
                info = tarfile.TarInfo('skia-binaries/' + name)
                info.size = len(data)
                t.addfile(info, io.BytesIO(data))
        os.replace(dest + '.partiel', dest)
        print('skia-cache : gardé', os.path.basename(dest), file=sys.stderr)


if __name__ == '__main__':
    if len(sys.argv) >= 4 and sys.argv[1] == 'prepare':
        prepare(sys.argv[2], sys.argv[3], sys.argv[4:])
    elif len(sys.argv) == 4 and sys.argv[1] == 'save':
        save(sys.argv[2], sys.argv[3])
    else:
        sys.exit(__doc__)
