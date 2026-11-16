"""Step 0 of the MakeHuman transfer spike: fetches the MakeHuman data the
spike reads, at a pinned commit, and checks every file it uses against
`makehuman.sha256`.

    python3 fetch_makehuman.py DIR          (plain Python and git; no bpy)
    python3 fetch_makehuman.py DIR --check  (only verify an existing checkout)

It makes a partial, sparse clone of
https://github.com/makehumancommunity/makehuman at commit
a8bc2d54ff0ac92e78ff71431b1023eda42bf482 (master, 2024-06-26) into `DIR`
(a new, empty folder outside the repository: nothing fetched is committed
here), holding only the licence files, the base mesh and the targets
(about 136 MB on disk). The files are data and are only read; none of
MakeHuman's code is fetched or run.

`makehuman.sha256` lists the 33 files the spike reads (the base mesh, 6
ethnic and 18 "universal" macro targets at the young age, 6 regional
targets, and the two licence files); `shapes/LICENCE-RECORD.md` says what makes
them CC0.
"""

import hashlib
import os
import subprocess
import sys

REPOSITORY = "https://github.com/makehumancommunity/makehuman.git"
COMMIT = "a8bc2d54ff0ac92e78ff71431b1023eda42bf482"
PATHS = ["/LICENSE.md", "/LICENSE.ASSETS.md", "/makehuman/data/3dobjs/base.obj", "/makehuman/data/targets/"]
HERE = os.path.dirname(os.path.abspath(__file__))


def git(*args, cwd=None):
    # MSYS_NO_PATHCONV: Git for Windows' shell otherwise rewrites the
    # leading slash of the sparse patterns into a Windows path.
    env = dict(os.environ, MSYS_NO_PATHCONV="1")
    subprocess.run(["git", *args], cwd=cwd, check=True, env=env)


def fetch(target):
    if os.path.exists(target) and os.listdir(target):
        raise SystemExit(f"{target} is not empty")
    git("clone", "--filter=blob:none", "--no-checkout", REPOSITORY, target)
    git("sparse-checkout", "set", "--no-cone", *PATHS, cwd=target)
    git("checkout", "--detach", COMMIT, cwd=target)


def check(target):
    bad = 0
    with open(os.path.join(HERE, "makehuman.sha256"), encoding="utf-8") as f:
        lines = [line.split(None, 1) for line in f if line.strip() and not line.startswith("#")]
    for digest, name in lines:
        path = os.path.join(target, name.strip().lstrip("*"))
        try:
            with open(path, "rb") as g:
                actual = hashlib.sha256(g.read()).hexdigest()
        except OSError:
            actual = "missing"
        if actual != digest:
            print(f"MISMATCH {name.strip()}: {actual}")
            bad += 1
    print(f"{len(lines) - bad} of {len(lines)} files match makehuman.sha256")
    return bad == 0


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        raise SystemExit(__doc__)
    if "--check" not in sys.argv:
        fetch(args[0])
    sys.exit(0 if check(args[0]) else 1)


if __name__ == "__main__":
    main()
