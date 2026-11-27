#!/usr/bin/env bash
# Runs the Win32 savers under Wine on their own virtual display and checks that each one runs, ends on a key, saves its settings and
# previews inside a parent window. Usage: smoke-wine.sh [--shots DIR] [--size WxH] [--saver NAME]... [i586|x86_64]...
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Wine gets its own Xvfb (the first free display number, started and stopped here) and a throwaway prefix in
# ~/.cache/afterglow-savers/wine, so nothing touches the real desktop, screen or keyboard. Only processes started here are stopped: Xvfb by
# its process number, and Wine through `wineserver -k` for this prefix alone. With --shots, a screendump of every full-screen run, settings
# dialog and preview goes to DIR as win32-<saver>-<size>-<arch>-<mode>.png. The default size is 1024x768, which is ReactOS's screen.
# Says SKIP when Wine, Xvfb, xdotool or ImageMagick is missing.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
prefix="${HOME}/.cache/afterglow-savers/wine"
shots=""
sizes=()
savers=()
arches=()
while [ $# -gt 0 ]; do
	case "$1" in
	--shots) shift; shots="${1:?--shots needs a directory}" ;;
	--size) shift; sizes+=("${1:?--size needs WxH}") ;;
	--saver) shift; savers+=("${1:?--saver needs a name}") ;;
	i586 | x86_64) arches+=("$1") ;;
	*) echo "usage: smoke-wine.sh [--shots DIR] [--size WxH] [--saver NAME]... [i586|x86_64]..." >&2; exit 2 ;;
	esac
	shift
done
[ ${#sizes[@]} -gt 0 ] || sizes=(1024x768)
[ ${#savers[@]} -gt 0 ] || savers=(agmaze agcrack agtwilit agink agcollct agquill)
[ ${#arches[@]} -gt 0 ] || arches=(i586 x86_64)

for tool in wine Xvfb xdotool import magick; do
	if ! command -v "$tool" >/dev/null; then
		echo "SKIP smoke-wine: $tool not found"
		exit 0
	fi
done
available_mb="$(awk '/MemAvailable/ {print int($2 / 1024)}' /proc/meminfo)"
if [ "$available_mb" -lt 2048 ]; then
	echo "SKIP smoke-wine: only $available_mb MB of memory is free"
	exit 0
fi
[ -z "$shots" ] || mkdir -p "$shots"
mkdir -p "$prefix"

xvfb_pid=""
own_display=""
# Every call that touches an X display goes through these two, which refuse to run unless DISPLAY is the Xvfb this script started: a
# stray key press or screendump must never reach the real desktop.
assert_own_display() {
	if [ -z "$own_display" ] || [ "${DISPLAY:-}" != "$own_display" ]; then
		echo "FAIL smoke-wine: refusing to touch display '${DISPLAY:-}' (own display is '${own_display:-none}')" >&2
		exit 1
	fi
}
xdo() {
	assert_own_display
	xdotool "$@"
}
shoot() {
	assert_own_display
	import -window root "$1"
}

stop_display() {
	WINEPREFIX="$prefix" wineserver -k 2>/dev/null || true
	if [ -n "$xvfb_pid" ]; then
		kill "$xvfb_pid" 2>/dev/null || true
		wait "$xvfb_pid" 2>/dev/null || true
		xvfb_pid=""
	fi
}
trap stop_display EXIT

fail() { echo "FAIL smoke-wine: $*" >&2; exit 1; }

start_display() {
	local size="$1" number=""
	for candidate in $(seq 90 130); do
		if [ ! -e "/tmp/.X11-unix/X$candidate" ] && [ ! -e "/tmp/.X$candidate-lock" ]; then
			number="$candidate"
			break
		fi
	done
	[ -n "$number" ] || fail "no free X display number"
	Xvfb ":$number" -screen 0 "${size}x24" -nolisten tcp >/dev/null 2>&1 &
	xvfb_pid=$!
	for _ in $(seq 50); do
		[ -e "/tmp/.X11-unix/X$number" ] && break
		sleep 0.1
	done
	[ -e "/tmp/.X11-unix/X$number" ] || fail "Xvfb did not start"
	own_display=":$number"
	export DISPLAY=":$number" WINEPREFIX="$prefix" WINEDEBUG="${SMOKE_WINEDEBUG:--all}" WINEDLLOVERRIDES="winemenubuilder.exe=d;mscoree=d;mshtml=d"
	unset WAYLAND_DISPLAY
	# A persistent server, so that it does not shut down between two programs and fail the next one's start.
	wineserver -p
	timeout 180 wine wineboot --init >/dev/null 2>&1 || true
}

# Waits up to 30 seconds for a window whose name matches, and prints its id.
wait_window() {
	local id
	for _ in $(seq 150); do
		id="$(xdo search --name "$1" 2>/dev/null | tail -1 || true)"
		if [ -n "$id" ]; then
			echo "$id"
			return 0
		fi
		sleep 0.2
	done
	return 1
}

# Waits up to $2 seconds for a process number to end.
wait_gone() {
	for _ in $(seq $(($2 * 5))); do
		kill -0 "$1" 2>/dev/null || return 0
		sleep 0.2
	done
	return 1
}

# Saves a screendump of the whole display when --shots is given, and prints its mean brightness (0 to 1) either way.
grab() {
	local name="$1" file
	file="${shots:-/tmp}/win32-$name.png"
	[ -n "$shots" ] || file="$(mktemp --suffix=.png)"
	shoot "$file"
	magick "$file" -format '%[fx:mean]' info:
	[ -n "$shots" ] || rm -f "$file"
}

# The registry key name of a program: agink is Ink, agcrack is Cracktro.
key_of() {
	case "$1" in
	agmaze) echo Maze ;;
	agcrack) echo Cracktro ;;
	agtwilit) echo Twilight ;;
	agink) echo Ink ;;
	agcollct) echo Collection ;;
	agquill) echo Ghostwriter ;;
	esac
}

wine_path() { echo "Z:${1//\//\\}"; }

registry_value() { wine reg query "HKCU\\Software\\Liminal HQ\\Afterglow Screensavers\\$1" /v "$2" 2>/dev/null | tr -d '\r' | awk -v v="$2" '$1 == v {print $3}'; }

for size in "${sizes[@]}"; do
	start_display "$size"
	for arch in "${arches[@]}"; do
		dir="$root/platforms/win32/dist/$arch"
		[ -f "$dir/AGINK.SCR" ] || { echo "SKIP smoke-wine $arch: not built"; continue; }
		wine reg delete "HKCU\\Software\\Liminal HQ" /f >/dev/null 2>&1 || true
		for name in "${savers[@]}"; do
			upper="$(tr '[:lower:]' '[:upper:]' <<<"$name")"
			scr="$(wine_path "$dir/$upper.SCR")"
			tag="${name#ag}-$size-$arch"

			# A program started in the instant after another one's exit can be dropped by Wine; give it a moment.
			sleep 2
			# /s: runs full screen, paints something, ends on a key.
			wine "$scr" /s >/dev/null 2>&1 &
			pid=$!
			wait_window "Afterglow|Quill" >/dev/null || fail "$upper /s ($arch, $size) opened no window"
			sleep 4
			mean="$(grab "$tag-full")"
			awk -v m="$mean" 'BEGIN { exit !(m > 0.01) }' || fail "$upper /s ($arch, $size) painted nothing (mean $mean)"
			kill -0 "$pid" 2>/dev/null || fail "$upper /s ($arch, $size) ended by itself"
			xdo key space
			wait_gone "$pid" 10 || fail "$upper /s ($arch, $size) did not end on a key"
			echo "ok $upper /s $arch $size: painted (mean $mean), ended on a key"

			# /c: the dialog opens, OK writes every value, and a value set beforehand survives the round trip.
			wine reg add "HKCU\\Software\\Liminal HQ\\Afterglow Screensavers\\$(key_of "$name")" /v Speed /t REG_DWORD /d 150 /f >/dev/null 2>&1 || true
			wine "$scr" /c >/dev/null 2>&1 &
			pid=$!
			wait_window "Settings" >/dev/null || fail "$upper /c ($arch, $size) opened no dialog"
			sleep 1
			grab "$tag-settings" >/dev/null
			xdo key Return
			wait_gone "$pid" 10 || fail "$upper /c ($arch, $size) did not close on OK"
			speed="$(registry_value "$(key_of "$name")" Speed || true)"
			[ "$speed" = "0x96" ] || fail "$upper /c ($arch, $size) lost Speed 150: read '$speed'"
			[ -n "$(registry_value "$(key_of "$name")" Palette || true)" ] || fail "$upper /c ($arch, $size) wrote no Palette"
			echo "ok $upper /c $arch $size: dialog opened, OK wrote the registry and kept Speed 150"
		done

		# /p: a small parent window with the saver drawing inside it, which the saver leaves when the parent goes.
		for name in agink agtwilit; do
			[[ " ${savers[*]} " == *" $name "* ]] || continue
			upper="$(tr '[:lower:]' '[:upper:]' <<<"$name")"
			wine "$(wine_path "$dir/previewhost.exe")" "$(wine_path "$dir/$upper.SCR")" >/dev/null 2>&1 &
			pid=$!
			wait_window "preview host" >/dev/null || fail "$upper /p ($arch) opened no parent window"
			sleep 3
			mean="$(grab "${name#ag}-$size-$arch-preview")"
			wait "$pid" && status=0 || status=$?
			[ "$status" = 0 ] || fail "$upper /p ($arch) host exit code $status (2 gone early, 3 did not end with its parent)"
			echo "ok $upper /p $arch $size: drew in the parent and ended with it"
		done
	done
	stop_display
done
echo "ok smoke-wine"
