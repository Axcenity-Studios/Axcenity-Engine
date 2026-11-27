#!/usr/bin/env bash
# GUI test harness for the QA audit (docs/research/qa-audit/README.md).
#
# Runs the real editor in a virtual X display with Mesa's software Vulkan
# (lavapipe) and drives it with xdotool, so a check is a real launch with
# real input, seen through screenshots. Slow (software rendering), but the
# same code paths a user hits.
#
#   source docs/research/qa-audit/gui-harness.sh
#   qa_start [editor args...]   # fresh HOME each time unless QA_HOME is set
#   qa_shot name                # screenshot to $QA_SHOTS/name.png
#   qa_click x y [button]       # click at window coordinates
#   qa_key ctrl+s               # xdotool key names
#   qa_type "text"
#   qa_stop                     # closes the editor (window close, then kill)
#
# Settings live under $QA_HOME (eframe storage in ~/.local/share), so a
# restart with the same QA_HOME tests persistence and a new one tests a
# first launch.

QA_ROOT=${QA_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}
QA_BIN=${QA_BIN:-$QA_ROOT/target/debug/axcenity-editor}
QA_SHOTS=${QA_SHOTS:-/home/user/axtmp/shots}
QA_DISPLAY=${QA_DISPLAY:-:99}
QA_SIZE=${QA_SIZE:-1600x900}
mkdir -p "$QA_SHOTS"
export DISPLAY=$QA_DISPLAY
# Reattach to an editor started by an earlier shell.
QA_PID=${QA_PID:-$(pgrep -f "$QA_BIN" | head -1)}
QA_WIN=${QA_WIN:-$( [ -n "$QA_PID" ] && xdotool search --pid "$QA_PID" --onlyvisible 2>/dev/null | head -1)}

qa_xvfb() {
    if ! xdpyinfo -display "$QA_DISPLAY" >/dev/null 2>&1; then
        Xvfb "$QA_DISPLAY" -screen 0 "${QA_SIZE}x24" -nolisten tcp >/dev/null 2>&1 &
        sleep 1
    fi
    export DISPLAY=$QA_DISPLAY
}

qa_start() {
    qa_xvfb
    if [ -z "${QA_HOME:-}" ]; then
        QA_HOME=$(mktemp -d /home/user/axtmp/home.XXXX)
    fi
    mkdir -p "$QA_HOME"
    export QA_HOME
    HOME=$QA_HOME XDG_DATA_HOME=$QA_HOME/.local/share XDG_CONFIG_HOME=$QA_HOME/.config \
        WGPU_BACKEND=vulkan RUST_BACKTRACE=1 \
        "$QA_BIN" "$@" >"$QA_SHOTS/editor.log" 2>&1 &
    QA_PID=$!
    export QA_PID
    # Wait for the window.
    for _ in $(seq 1 120); do
        QA_WIN=$(xdotool search --pid "$QA_PID" --onlyvisible 2>/dev/null | head -1)
        [ -n "$QA_WIN" ] && break
        if ! kill -0 "$QA_PID" 2>/dev/null; then
            echo "editor exited early"; tail -20 "$QA_SHOTS/editor.log"; return 1
        fi
        sleep 1
    done
    export QA_WIN
    xdotool windowmove "$QA_WIN" 0 0 windowsize "$QA_WIN" ${QA_SIZE%x*} ${QA_SIZE#*x} 2>/dev/null
    echo "editor pid $QA_PID window $QA_WIN home $QA_HOME"
}

qa_alive() { kill -0 "$QA_PID" 2>/dev/null && echo alive || { echo "DEAD"; tail -30 "$QA_SHOTS/editor.log"; }; }
qa_wait() { sleep "${1:-2}"; }
qa_shot() { import -display "$QA_DISPLAY" -window root "$QA_SHOTS/$1.png"; echo "$QA_SHOTS/$1.png"; }
qa_move() { xdotool mousemove --sync "$1" "$2"; }
qa_click() { xdotool mousemove --sync "$1" "$2"; sleep 0.3; xdotool click "${3:-1}"; }
qa_dclick() { xdotool mousemove --sync "$1" "$2"; sleep 0.3; xdotool click --repeat 2 --delay 80 1; }
qa_drag() { xdotool mousemove --sync "$1" "$2" mousedown 1; sleep 0.3; xdotool mousemove --sync "$3" "$4"; sleep 0.3; xdotool mousemove --sync "$3" "$4" mouseup 1; }
qa_key() { xdotool key --delay 250 "$@"; }
qa_type() { xdotool type --delay 90 "$1"; }
qa_stop() {
    [ -n "${QA_PID:-}" ] || return 0
    # The window's own close button (no window manager here, and
    # `xdotool windowclose` destroys the window under winit, which panics).
    xdotool mousemove --sync $(( ${QA_SIZE%x*} - 23 )) 16 click 1 2>/dev/null
    for _ in $(seq 1 10); do kill -0 "$QA_PID" 2>/dev/null || break; sleep 1; done
    kill "$QA_PID" 2>/dev/null
    unset QA_PID QA_WIN
}
