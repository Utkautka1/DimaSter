#!/bin/sh
# Open a DimaSter window from the container image on this machine's display.
# Usage: run.sh [dimaster arguments]
#   DIMASTER_IMAGE  image to use (default: ghcr.io/utkautka1/dimaster:latest)
#   DIMASTER_HOME   host folder kept as the container home (settings, history)
set -eu

IMAGE="${DIMASTER_IMAGE:-ghcr.io/utkautka1/dimaster:latest}"
STORE="${DIMASTER_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/dimaster-docker}"
mkdir -p -- "$STORE"

# options are collected as positional parameters to keep paths with spaces intact
set -- "$IMAGE" "$@"
set -- --user "$(id -u):$(id -g)" -v "$STORE:/home/dimaster" "$@"
[ -d /dev/dri ] && set -- --device /dev/dri "$@"
for group in video render; do
    gid="$(getent group "$group" | cut -d: -f3)"
    [ -n "$gid" ] && set -- --group-add "$gid" "$@"
done

if [ -n "${WAYLAND_DISPLAY:-}" ] && [ -S "${XDG_RUNTIME_DIR:-}/$WAYLAND_DISPLAY" ]; then
    set -- -e XDG_RUNTIME_DIR=/tmp -e "WAYLAND_DISPLAY=$WAYLAND_DISPLAY" \
        -v "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY:/tmp/$WAYLAND_DISPLAY" "$@"
elif [ -n "${DISPLAY:-}" ]; then
    # X11 may also need: xhost +si:localuser:"$(id -un)"
    set -- -e "DISPLAY=$DISPLAY" -v /tmp/.X11-unix:/tmp/.X11-unix:ro "$@"
else
    echo "no Wayland or X11 display found" >&2
    exit 1
fi

exec docker run --rm --init --name "dimaster-$$" "$@"
