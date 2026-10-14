"""Sources Cargo pour Flatpak : packaging/flatpak/cargo-sources.json, tiré de Cargo.lock.

Flatpak compile sans réseau : chaque crate de Cargo.lock y est déclarée (adresse sur crates.io et
empreinte SHA-256, déjà présente dans Cargo.lock), puis rangée dans un dossier « vendor » que cargo
utilise à la place de crates.io. À relancer après chaque changement de Cargo.lock.

Usage : python tools/flatpak-cargo-sources.py   (Python 3.11 ou plus, rien d'autre à installer)
"""
import json
import pathlib
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"

lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
sources = []
for pkg in lock["package"]:
    source = pkg.get("source")
    if source is None:
        continue  # Turtlefin lui-même
    if source != REGISTRY:
        raise SystemExit(f"{pkg['name']} {pkg['version']} : source non prise en charge ({source})")
    name, version, checksum = pkg["name"], pkg["version"], pkg["checksum"]
    dest = f"cargo/vendor/{name}-{version}"
    sources.append({
        "type": "archive",
        "archive-type": "tar-gzip",
        "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
        "sha256": checksum,
        "dest": dest,
    })
    sources.append({
        "type": "inline",
        "contents": json.dumps({"package": checksum, "files": {}}),
        "dest": dest,
        "dest-filename": ".cargo-checksum.json",
    })
sources.append({
    "type": "inline",
    "contents": '[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n'
                '[source.crates-io]\nreplace-with = "vendored-sources"\n',
    "dest": "cargo",
    "dest-filename": "config.toml",
})
out = ROOT / "packaging" / "flatpak" / "cargo-sources.json"
out.write_text(json.dumps(sources, indent=2) + "\n", encoding="utf-8", newline="\n")
print(f"{out.relative_to(ROOT)} : {len(sources) // 2} crates")
