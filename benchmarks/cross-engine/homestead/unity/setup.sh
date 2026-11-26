#!/bin/sh
# Copies the port's scripts into the Unity project, then imports the dump
# or builds the player with the editor, headless.
#
#   unity/setup.sh import|build|both [project folder]
#
# The project (default ~/Projects/Unity Projects/Desert Homestead HDRP) is
# an HDRP project made from Unity's own template, with Vulkan named as its
# only Linux graphics API (the editor picks OpenGL otherwise, which HDRP
# does not draw with), and the engine's dump in its `Dump` folder.
# The editor runs inside Unity Hub's flatpak, where its license is.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
STEP=${1:-both}
PROJECT=${2:-"$HOME/Projects/Unity Projects/Desert Homestead HDRP"}
UNITY=${UNITY:-"$HOME/Unity/Hub/Editor/6000.6.10f1/Editor/Unity"}
mkdir -p "$PROJECT/Assets/Homestead"
cp -r "$HERE/Assets/Homestead/Runtime" "$HERE/Assets/Homestead/Editor" "$PROJECT/Assets/Homestead/"
run() {
    flatpak run --command="$UNITY" com.unity.UnityHub -batchmode -quit -projectPath "$PROJECT" \
        -executeMethod "$1" -logFile "$PROJECT/Logs/$2.log" || {
        echo "$1 failed: see $PROJECT/Logs/$2.log" >&2
        grep -E 'error CS|Exception|HOMESTEAD' "$PROJECT/Logs/$2.log" | sort -u | head -40 >&2
        exit 1
    }
    grep -E 'HOMESTEAD' "$PROJECT/Logs/$2.log" | sort -u
}
case "$STEP" in
    import) run HomesteadImport.Import import ;;
    build) run HomesteadImport.Build build ;;
    both) run HomesteadImport.Import import; run HomesteadImport.Build build ;;
    *) echo "import, build or both" >&2; exit 1 ;;
esac
