#!/bin/sh
# One feature off at a time, in both engines, to see what each costs:
#
#   ablate.sh [views, default hero,crane] [size, default 1920x1080]
#
# Axcenity's runs change one setting of benchmarks/features/owner-ultra.json;
# Unity's change the look with HomesteadBench's -hs-set. Results go beside
# run.py's, in folders named ablate-<what>-<size>; `ablate.sh table` prints
# them as one table, and `ablate.sh world` times the engine alone with the
# clouds, the wind or the painted foliage taken out of the world.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ENGINE=$(cd "$HERE/../../.." && pwd)
WORK="$ENGINE/target/cross-engine/homestead"
VIEWS=${1:-hero,crane}
SIZE=${2:-1920x1080}

if [ "$VIEWS" = table ]; then
    exec python3 - "$WORK/runs" <<'PY'
import json, sys
from pathlib import Path
runs = Path(sys.argv[1])
rows = {}
for folder in sorted(runs.glob("ablate-*")):
    what = folder.name[len("ablate-"):].rsplit("-", 1)[0]
    for path in folder.glob("*-1.json"):
        if path.name.endswith((".frames.json", ".unity.json")):
            continue
        r = json.loads(path.read_text())
        rows[(r["view"], r["engine"], what)] = r
for view in sorted({k[0] for k in rows}):
    for engine in ("Axcenity", "Unity HDRP"):
        base = rows.get((view, engine, "all-on"))
        if not base:
            continue
        print(f"\n{view}, {engine}: everything on {base['mean_ms']:.2f} ms a frame ({base['fps']:.0f} FPS), GPU {base['gpu_ms']:.2f} ms\n")
        print("| Off | Frame ms | Saved ms | GPU ms | FPS |")
        print("|---|---|---|---|---|")
        for (v, e, what), r in sorted(rows.items()):
            if v == view and e == engine and what != "all-on":
                print(f"| {what} | {r['mean_ms']:.2f} | {base['mean_ms'] - r['mean_ms']:.2f} | {r['gpu_ms']:.2f} | {r['fps']:.0f} |")
PY
fi

if [ "$VIEWS" = world ]; then
    # `ablate.sh world`: the engine alone, with one thing taken out of a copy
    # of the standing-still project (the owner's project is never written).
    STILL="$ENGINE/target/bench-projects/Desert Homestead (still)"
    world() {
        # world <name> <python statement changing the world `w`, or the copy's folder `dst`>
        mkdir -p "$WORK/variants"
        [ -d "$WORK/variants/$1" ] || cp -a --reflink=auto "$STILL" "$WORK/variants/$1"
        python3 - "$STILL/world.json" "$WORK/variants/$1" "$2" <<'PY'
import json, sys
w = json.load(open(sys.argv[1]))
dst = sys.argv[2]
exec(sys.argv[3])
json.dump(w, open(dst + "/world.json", "w"))
PY
        HOMESTEAD_STILL="$WORK/variants/$1" python3 "$HERE/run.py" --engine axcenity --what stills \
            --only hero,windmill,porch --runs 1 --frames 1200 --label "world-$1"
    }
    world no-clouds "[c['properties'].update(Cover=0.0) for c in w['lighting']['children'] if c['class'] == 'Clouds']"
    world no-wind "w['workspace']['properties']['GlobalWind'] = [0, 0, 0]"
    world no-foliage "[json.dump({'instances': []}, open(f'{dst}/assets/terrain/{t}/foliage.json', 'w')) for t in ('Homestead', 'Valley')]"
    exit 0
fi

mkdir -p "$WORK/graphics"
variant() {
    # variant <name> <python expression changing the settings `g`>
    python3 - "$ENGINE/benchmarks/features/owner-ultra.json" "$WORK/graphics/$1.json" "$2" <<'PY'
import json, sys
g = json.load(open(sys.argv[1]))
exec(sys.argv[3])
json.dump(g, open(sys.argv[2], "w"), indent=1)
PY
    python3 "$HERE/run.py" --engine axcenity --what stills --only "$VIEWS" --runs 1 --frames 1200 \
        --size "$SIZE" --label "ablate-$1" --graphics "$WORK/graphics/$1.json"
}
unity() {
    python3 "$HERE/run.py" --engine unity --what stills --only "$VIEWS" --runs 1 --frames 1200 \
        --size "$SIZE" --label "ablate-$1" --unity-set "$2"
}

variant all-on "pass"
variant clouds "g['cloud_quality'] = 'Low'"
variant sun-shadows "g['shadow_quality'] = 'Off'"
variant volumetric-fog "g['volumetric_fog'] = False"
variant ambient-occlusion "g['ambient_occlusion'] = 'Off'"
variant contact-shadows "g['contact_shadows'] = False"
variant anti-aliasing "g['anti_aliasing'] = 'Off'"
variant sun-rays "g['sun_rays'] = False"
variant half-the-pixels "g['render_scale'] = 0.7071"

unity all-on ""
unity clouds "clouds=0"
unity sun-shadows "shadows=0"
unity volumetric-fog "volfog=0"
unity ambient-occlusion "ao=0"
unity contact-shadows "contact=0"
unity anti-aliasing "aa=0"
