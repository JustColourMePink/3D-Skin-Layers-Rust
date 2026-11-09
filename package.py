"""Packs skin_layers_3d/ into dist/skin_layers_3d.hnmod (a Horizon native mod file).

Run from anywhere:  python package.py
The archive holds horizon-native.json and mixins/skin_layers_3d/<bundle files>,
with fixed timestamps so the same sources always give the same bytes.
"""

import json
import pathlib
import tomllib
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
BUNDLE = HERE / "skin_layers_3d"
DIST = HERE / "dist"
# horizon-mixins commit the launcher builds this Horizon revision with.
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
GAME = "26.3"


def main() -> None:
    manifest = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text(encoding="utf-8"))
    mod_id = manifest["mod"]["id"]
    native = {
        "game": GAME,
        "horizon": manifest["compatibility"]["horizon_revision"],
        "id": mod_id,
        "mixins": MIXINS,
        "schema": 1,
        "version": manifest["mod"]["version"],
    }
    DIST.mkdir(exist_ok=True)
    out = DIST / f"{mod_id}.hnmod"
    files = sorted(p for p in BUNDLE.rglob("*") if p.is_file())
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        def put(name: str, data: bytes) -> None:
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)

        put("horizon-native.json", (json.dumps(native, indent=2, sort_keys=True) + "\n").encode())
        for path in files:
            rel = path.relative_to(BUNDLE).as_posix()
            put(f"mixins/{mod_id}/{rel}", path.read_bytes())
    print(f"{out} ({len(files)} bundle files)")


if __name__ == "__main__":
    main()
