"""Vérifie les traductions : textes du code absents d'un .po, et {} / {n} perdus.

usage : python tools/lang-check.py [lang/xx.po ...]   (par défaut : tous les fichiers de lang/)
Les textes du code sont ceux passés à Tr.t / Tr.f / Tr.p (ui/*.slint) et tr / trf (src/*.rs).
"""
import glob, re, sys

LIT = r'"((?:[^"\\]|\\.)*)"'


def un(s):
    return s.replace('\\"', '"').replace('\\n', '\n').replace('\\\\', '\\')


def code_texts():
    ids = set()
    for f in glob.glob('ui/*.slint'):
        s = open(f, encoding='utf-8').read()
        ids.update(un(m) for m in re.findall(r'Tr\.[tfp]\(Tr\.[lk],\s*' + LIT, s))
    for f in glob.glob('src/*.rs'):
        if not f.endswith('i18n.rs'):
            ids.update(un(m) for m in re.findall(r'(?<![\w])trf?\(\s*' + LIT, open(f, encoding='utf-8').read()))
    return ids


def po(path):
    out = {}
    for block in open(path, encoding='utf-8-sig').read().split('\n\n'):
        m = re.search(r'^msgid ' + LIT, block, re.M)
        if m and m.group(1):
            out[un(m.group(1))] = [un(x) for x in re.findall(r'^msgstr(?:\[\d\])? ' + LIT, block, re.M)]
    return out


ids = code_texts()
bad = 0
for path in sys.argv[1:] or sorted(glob.glob('lang/*.po')):
    cat = po(path)
    missing = sorted(t for t in ids if not any(cat.get(t, [])))
    broken = [(k, v) for k, vs in cat.items() for v in vs
              if v and (v.count('{}') != k.count('{}') or ('{n}' in k) != ('{n}' in v))]
    print('%-14s %3d textes, %3d manquants, %d {} perdus' % (path, len(cat), len(missing), len(broken)))
    for t in missing[:20]:
        print('   manquant :', t)
    for k, v in broken:
        print('   {} :', k, '->', v)
    bad += bool(missing or broken)
sys.exit(1 if bad else 0)
