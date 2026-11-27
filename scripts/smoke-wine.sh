#!/usr/bin/env bash
# Runs the Win32 savers under Wine on their own virtual display and checks that each one runs, ends on a key, saves its settings and
# previews inside a parent window, and that the Afterglow settings dialog (SAV-D39) opens, changes its palette from a click and from the
# arrow keys, updates its live preview, writes the registry on OK, writes nothing on Cancel, and gives way to the plain dialog when asked. Usage: smoke-wine.sh [--shots DIR] [--size WxH] [--saver NAME]... [i586|x86_64]...
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
# A prefix of this run's own: two runs sharing one (another agent's, or another worktree's) share a wineserver, and a program can
# then open its window on the other run's display.
mkdir -p "${HOME}/.cache/afterglow-savers"
prefix="$(mktemp -d "${HOME}/.cache/afterglow-savers/wine.XXXXXX")"
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
finish() {
	stop_display
	rm -rf "$prefix"
}
trap finish EXIT

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

# Waits up to a minute for a window whose name matches, and prints its id.
wait_window() {
	local id
	for _ in $(seq 300); do
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

registry_value() { local try out; for try in 1 2 3 4 5; do out="$(registry_value_once "$@")"; [ -z "$out" ] || { echo "$out"; return 0; }; sleep 1; done; return 0; }
registry_value_once() { wine reg query "HKCU\\Software\\Liminal HQ\\Afterglow Screensavers\\$1" /v "$2" 2>/dev/null | tr -d '\r' | awk -v v="$2" '$1 == v {print $3}'; }

# Statistics of a region of a screendump: crop_stat FILE WxH+X+Y FORMAT.
crop_stat() { magick "$1" -crop "$2" +repage -format "$3" info:; }

# The registry value of Palette, or nothing.
palette_of() { registry_value "$(key_of "$1")" Palette || true; }

# The number of the palette swatch that holds a palette value, in the dialog's order (Ghostwriter lists Quill's own first).
swatch_of() {
	if [ "$1" = agquill ]; then
		case "$2" in 5) echo 0 ;; 0) echo 1 ;; 1) echo 2 ;; 2) echo 3 ;; 3) echo 4 ;; 4) echo 5 ;; esac
	else
		echo "$2"
	fi
}

# Opens a saver's settings dialog and waits for it to draw; sets DIALOG_X, DIALOG_Y, DIALOG_W and DIALOG_H and DIALOG_PID.
open_dialog() {
	local id geometry="" attempt
	# Wine drops a program started right after another one's exit now and then; ask again, twice, before failing.
	for attempt in 1 2 3; do
		sleep 2
		wine "$1" /c >/dev/null 2>&1 &
		DIALOG_PID=$!
		id=""
		for _ in $(seq 100); do
			id="$(xdo search --name "Settings" 2>/dev/null | tail -1 || true)"
			if [ -n "$id" ] && geometry="$(xdo getwindowgeometry --shell "$id" 2>/dev/null)" && [ -n "$geometry" ]; then
				break 2
			fi
			id=""
			sleep 0.2
		done
		kill "$DIALOG_PID" 2>/dev/null || true
		wait "$DIALOG_PID" 2>/dev/null || true
	done
	[ -n "$id" ] || fail "$2: opened no dialog"
	sleep 3
	local X Y WIDTH HEIGHT
	eval "$geometry"
	DIALOG_X=$X DIALOG_Y=$Y DIALOG_W=$WIDTH DIALOG_H=$HEIGHT
}

# Clicks the centre of swatch number $1 of a saver's dialog. The layout is in dialog units (376 wide, swatches in the left column 76 down
# from the top and 30 tall); a unit is DIALOG_W / 376 pixels across and 1.625 down in Wine's default font.
click_swatch() {
	local count="$2" tile
	tile=$(((176 - 2 * (count - 1)) / count))
	local ux=$((8 + $1 * (tile + 2) + tile / 2)) x y
	x=$(awk -v w="$DIALOG_W" -v u="$ux" -v o="$DIALOG_X" 'BEGIN { printf "%d", o + u * w / 376 }')
	y=$(awk -v o="$DIALOG_Y" 'BEGIN { printf "%d", o + 91 * 1.625 }')
	xdo mousemove "$x" "$y" click 1
}

# The Afterglow dialog: $1 is the saver, $2 the path of its file, $3 the tag for screendumps.
afterglow_dialog_checks() {
	local name="$1" scr="$2" tag="$3" upper key count before after shot pane
	upper="$(tr '[:lower:]' '[:upper:]' <<<"$name")"
	key="$(key_of "$name")"
	count=5
	[ "$name" != agquill ] || count=6
	shot="$(mktemp --suffix=.png)"
	local base="HKCU\\Software\\Liminal HQ\\Afterglow Screensavers\\$key"

	# It opens: the banner is the scene, so it is far from flat, where the plain dialog's grey is.
	wine reg add "$base" /v Palette /t REG_DWORD /d 4 /f >/dev/null 2>&1 || true
	open_dialog "$scr" "$upper /c ($tag)"
	shoot "$shot"
	local banner
	banner="$(crop_stat "$shot" "$((DIALOG_W - 8))x8+$((DIALOG_X + 4))+$((DIALOG_Y + 1))" '%[fx:standard_deviation]')"
	awk -v v="$banner" 'BEGIN { exit !(v > 0.008) }' || fail "$upper /c ($tag): no Afterglow banner (spread $banner)"
	# The live preview runs: two grabs a moment apart differ.
	local px=$((DIALOG_X + DIALOG_W * 192 / 376)) py=$((DIALOG_Y + 50 * 1625 / 1000)) pw=$((DIALOG_W * 176 / 376)) ph=$((96 * 1625 / 1000))
	pane="${pw}x${ph}+${px}+${py}"
	# A host under load may take a few seconds to deliver the timer ticks that draw it.
	local before_text="0" try
	for try in 1 2 3 4 5 6 7 8; do
		shoot "$shot"
		before_text="$(crop_stat "$shot" "$pane" '%[fx:mean]')"
		awk -v v="$before_text" 'BEGIN { exit !(v > 0.01) }' && break
		sleep 1.5
	done
	awk -v v="$before_text" 'BEGIN { exit !(v > 0.01) }' || fail "$upper /c ($tag): the preview pane is black"
	before="$(crop_stat "$shot" "$pane" '%[fx:mean.g-mean.r]')"

	# A click on the Aurora swatch changes the palette and the preview's colours.
	click_swatch "$(swatch_of "$name" 3)" "$count"
	sleep 2
	shoot "$shot"
	after="$(crop_stat "$shot" "$pane" '%[fx:mean.g-mean.r]')"
	awk -v a="$after" -v b="$before" 'BEGIN { exit !(a - b > 0.008) }' || fail "$upper /c ($tag): a swatch click did not recolour the preview (green less red $before then $after)"
	xdo key Return
	wait_gone "$DIALOG_PID" 10 || fail "$upper /c ($tag): did not close on OK after a swatch click"
	[ "$(palette_of "$name")" = 0x3 ] || fail "$upper /c ($tag): the swatch click wrote Palette '$(palette_of "$name")', not 3"
	echo "ok $upper /c $tag: banner and preview drawn, a swatch click changed the palette and the preview, OK wrote Palette 3"

	# The arrow keys move between the swatches as radio buttons do: Aurora, then Graphite.
	open_dialog "$scr" "$upper /c ($tag)"
	xdo key Right
	sleep 1
	xdo key Return
	wait_gone "$DIALOG_PID" 10 || fail "$upper /c ($tag): did not close on OK after an arrow key"
	[ "$(palette_of "$name")" = 0x4 ] || fail "$upper /c ($tag): Right wrote Palette '$(palette_of "$name")', not 4"

	# Cancel writes nothing, even after a change.
	open_dialog "$scr" "$upper /c ($tag)"
	click_swatch "$(swatch_of "$name" 1)" "$count"
	sleep 1
	xdo key Escape
	wait_gone "$DIALOG_PID" 10 || fail "$upper /c ($tag): did not close on Cancel"
	[ "$(palette_of "$name")" = 0x4 ] || fail "$upper /c ($tag): Cancel wrote Palette '$(palette_of "$name")'"
	echo "ok $upper /c $tag: arrow keys moved the choice, Cancel wrote nothing"
	rm -f "$shot"
}

# The plain dialog stays: with PlainDialog set in the saver's key the dialog has no banner and still reads and writes the same values.
plain_dialog_checks() {
	local name="$1" scr="$2" tag="$3" upper key shot banner
	upper="$(tr '[:lower:]' '[:upper:]' <<<"$name")"
	key="$(key_of "$name")"
	local base="HKCU\\Software\\Liminal HQ\\Afterglow Screensavers\\$key"
	wine reg add "$base" /v PlainDialog /t REG_DWORD /d 1 /f >/dev/null 2>&1 || true
	wine reg add "$base" /v Palette /t REG_DWORD /d 2 /f >/dev/null 2>&1 || true
	open_dialog "$scr" "$upper /c plain ($tag)"
	shot="$(mktemp --suffix=.png)"
	shoot "$shot"
	banner="$(crop_stat "$shot" "$((DIALOG_W - 8))x8+$((DIALOG_X + 4))+$((DIALOG_Y + 1))" '%[fx:standard_deviation]')"
	rm -f "$shot"
	awk -v v="$banner" 'BEGIN { exit !(v < 0.004) }' || fail "$upper /c plain ($tag): PlainDialog did not give the plain dialog (spread $banner)"
	xdo key Return
	wait_gone "$DIALOG_PID" 10 || fail "$upper /c plain ($tag): did not close on OK"
	[ "$(palette_of "$name")" = 0x2 ] || fail "$upper /c plain ($tag): wrote Palette '$(palette_of "$name")', not 2"
	wine reg delete "$base" /v PlainDialog /f >/dev/null 2>&1 || true
	echo "ok $upper /c $tag: PlainDialog gave the plain dialog, which kept Palette 2"
}

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
			sleep 3
			grab "$tag-settings" >/dev/null
			xdo key Return
			wait_gone "$pid" 10 || fail "$upper /c ($arch, $size) did not close on OK"
			speed="$(registry_value "$(key_of "$name")" Speed || true)"
			[ "$speed" = "0x96" ] || fail "$upper /c ($arch, $size) lost Speed 150: read '$speed'"
			[ -n "$(registry_value "$(key_of "$name")" Palette || true)" ] || fail "$upper /c ($arch, $size) wrote no Palette"
			echo "ok $upper /c $arch $size: dialog opened, OK wrote the registry and kept Speed 150"
			sleep 2
			afterglow_dialog_checks "$name" "$scr" "$arch $size"
			if [ "$name" = agink ]; then
				sleep 2
				plain_dialog_checks "$name" "$scr" "$arch $size"
			fi
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
