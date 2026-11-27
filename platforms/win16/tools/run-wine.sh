#!/usr/bin/env bash
# Runs QUILLGW.SCR under Wine on its own Xvfb and saves screendumps of that display; nothing touches the host's desktop.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: platforms/win16/tools/run-wine.sh [--ini "Key=value ..."] [--args "/s"] QUILLGW.SCR PREFIX SECONDS...
#
# Starts Xvfb (640 x 480, DISPLAY :98 unless XVFB_DISPLAY says otherwise) and a throwaway Wine prefix under ~/.cache/afterglow-savers,
# writes the CONTROL.INI keys given with --ini into the prefix, starts the saver with the arguments, and saves a PNG named PREFIX-<s>.png
# at each number of seconds. With --input it then moves the mouse on that same Xvfb and reports whether the saver exited.
set -euo pipefail

ini="" args="/s" input=0 depth=24
while [ $# -gt 0 ]; do
	case "$1" in
		--ini) ini="$2"; shift 2 ;;
		--args) args="$2"; shift 2 ;;
		--input) input=1; shift ;;
		--depth) depth="$2"; shift 2 ;;
		*) break ;;
	esac
done
scr="$1" prefix="$2"
shift 2

export DISPLAY="${XVFB_DISPLAY:-:98}"
export WINEPREFIX="${WINE_PREFIX_DIR:-$HOME/.cache/afterglow-savers/wine-win16}" WINEDEBUG=-all WINEDLLOVERRIDES="mscoree,mshtml="
Xvfb "$DISPLAY" -screen 0 "640x480x$depth" -nolisten tcp &
xvfb_pid=$!
wine_pid=""
trap '[ -n "$wine_pid" ] && kill "$wine_pid" 2> /dev/null || true; wineserver -k 2> /dev/null || true; kill "$xvfb_pid" 2> /dev/null || true' EXIT
sleep 1
[ -d "$WINEPREFIX/drive_c" ] || wine wineboot -i > /dev/null 2>&1
win_dir="$WINEPREFIX/drive_c/windows"
cp "$scr" "$win_dir/QUILLGW.SCR"
if [ -n "$ini" ]; then
	{
		printf '[Screen Saver.Quill Ghostwriter]\r\n'
		for pair in $ini; do printf '%s\r\n' "$pair"; done
	} > "$win_dir/control.ini"
else
	rm -f "$win_dir/control.ini"
fi
# shellcheck disable=SC2086
wine "C:\\windows\\QUILLGW.SCR" $args > /dev/null 2>&1 &
wine_pid=$!
start=$(date +%s)
for at in "$@"; do
	now=$(( $(date +%s) - start ))
	[ "$at" -gt "$now" ] && sleep $(( at - now ))
	import -window root "$prefix-$at.png"
done
if [ "$input" = 1 ]; then
	running() { pgrep -f "winevdm.*QUILLGW" > /dev/null; }
	xdotool mousemove 300 200
	sleep 1
	xdotool mousemove 320 230
	sleep 2
	if running; then echo "run-wine: mouse move: still running"; else echo "run-wine: mouse move: exited"; fi
fi
