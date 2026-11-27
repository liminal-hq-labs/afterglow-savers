#!/usr/bin/env bash
# Runs the Win32 savers under Wine with their sound captured from a private virtual sink, and checks the sound: on and unbroken with Sound
# on, absent with Sound off, the process ending cleanly on a key either way. Usage: wine-audio.sh [--saver NAME]... [--seconds N] [i586|x86_64]...
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Wine gets its own Xvfb and a throwaway prefix, as in smoke-wine.sh, and its sound goes to a null sink this script creates in PulseAudio
# or PipeWire (`pactl load-module module-null-sink`), so nothing reaches the speakers; `parec` records the sink's monitor, and only the
# module this script loaded is unloaded. A virtual sink keeps time as a card does, which a plain ALSA null device does not (it takes data
# at once, so a saver feeding it renders faster than real time). Says SKIP when a tool is missing.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
seconds=12
savers=()
arches=()
while [ $# -gt 0 ]; do
	case "$1" in
	--saver) shift; savers+=("${1:?--saver needs a name}") ;;
	--seconds) shift; seconds="${1:?--seconds needs a number}" ;;
	i586 | x86_64) arches+=("$1") ;;
	*) echo "usage: wine-audio.sh [--saver NAME]... [--seconds N] [i586|x86_64]..." >&2; exit 2 ;;
	esac
	shift
done
[ ${#savers[@]} -gt 0 ] || savers=(agmaze agcrack agtwilit agink agcollct agquill)
[ ${#arches[@]} -gt 0 ] || arches=(i586 x86_64)

for tool in wine Xvfb xdotool pactl parec python3; do
	if ! command -v "$tool" >/dev/null; then
		echo "SKIP wine-audio: $tool not found"
		exit 0
	fi
done
python3 -c 'import numpy' 2>/dev/null || { echo "SKIP wine-audio: python3 numpy not found"; exit 0; }

mkdir -p "${HOME}/.cache/afterglow-savers"
prefix="$(mktemp -d "${HOME}/.cache/afterglow-savers/wine-audio.XXXXXX")"
work="$(mktemp -d "${HOME}/.cache/afterglow-savers/wine-audio-out.XXXXXX")"
xvfb_pid=""
sink_module=""
recorder=""
own_display=""

cleanup() {
	[ -z "$recorder" ] || { kill "$recorder" 2>/dev/null || true; wait "$recorder" 2>/dev/null || true; }
	WINEPREFIX="$prefix" wineserver -k 2>/dev/null || true
	[ -z "$xvfb_pid" ] || { kill "$xvfb_pid" 2>/dev/null || true; wait "$xvfb_pid" 2>/dev/null || true; }
	[ -z "$sink_module" ] || pactl unload-module "$sink_module" 2>/dev/null || true
	rm -rf "$prefix" "$work"
}
trap cleanup EXIT
fail() { echo "FAIL wine-audio: $*" >&2; exit 1; }

number=""
for candidate in $(seq 150 190); do
	if [ ! -e "/tmp/.X11-unix/X$candidate" ] && [ ! -e "/tmp/.X$candidate-lock" ]; then
		number="$candidate"
		break
	fi
done
[ -n "$number" ] || fail "no free X display number"
Xvfb ":$number" -screen 0 1024x768x24 -nolisten tcp >/dev/null 2>&1 &
xvfb_pid=$!
for _ in $(seq 50); do [ -e "/tmp/.X11-unix/X$number" ] && break; sleep 0.1; done
[ -e "/tmp/.X11-unix/X$number" ] || fail "Xvfb did not start"
own_display=":$number"
export DISPLAY="$own_display" WINEPREFIX="$prefix" WINEDEBUG=-all WINEDLLOVERRIDES="winemenubuilder.exe=d;mscoree=d;mshtml=d"
unset WAYLAND_DISPLAY
xdo() {
	[ "${DISPLAY:-}" = "$own_display" ] || fail "refusing to touch display '${DISPLAY:-}'"
	xdotool "$@"
}

sink="afterglow_test_$$"
sink_module="$(pactl load-module module-null-sink "sink_name=$sink" format=s16le rate=48000 channels=2)"
export PULSE_SINK="$sink"
wineserver -p
timeout 180 wine wineboot --init >/dev/null 2>&1 || true
wine reg add 'HKCU\Software\Wine\Drivers' /v Audio /t REG_SZ /d pulse /f >/dev/null 2>&1

key_of() {
	case "$1" in
	agmaze) echo Maze ;; agcrack) echo Cracktro ;; agtwilit) echo Twilight ;; agink) echo Ink ;; agcollct) echo Collection ;; agquill) echo Ghostwriter ;;
	esac
}

# Runs one saver for $seconds with the given Sound and Detail, and checks the recording.
run_case() {
	local arch="$1" name="$2" sound="$3" detail="$4" upper key base pid
	upper="$(tr '[:lower:]' '[:upper:]' <<<"$name")"
	key="$(key_of "$name")"
	base="HKCU\\Software\\Liminal HQ\\Afterglow Screensavers\\$key"
	wine reg add "$base" /v Sound /t REG_DWORD /d "$sound" /f >/dev/null 2>&1
	wine reg add "$base" /v Detail /t REG_DWORD /d "$detail" /f >/dev/null 2>&1
	# The Collection plays the Quill turn only when asked, so its sound is the other four.
	rm -f "$work/capture.raw"
	parec -d "$sink.monitor" --format=s16le --rate=48000 --channels=2 --raw >"$work/capture.raw" 2>/dev/null &
	recorder=$!
	sleep 0.5
	wine "Z:${root//\//\\}\\platforms\\win32\\dist\\$arch\\$upper.SCR" /s >/dev/null 2>&1 &
	pid=$!
	sleep "$seconds"
	kill -0 "$pid" 2>/dev/null || fail "$upper ($arch, Sound $sound, Detail $detail) ended by itself"
	xdo key space
	for _ in $(seq 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
	kill -0 "$pid" 2>/dev/null && fail "$upper ($arch, Sound $sound, Detail $detail) did not end on a key"
	sleep 0.3
	kill "$recorder" 2>/dev/null || true
	wait "$recorder" 2>/dev/null || true
	recorder=""
	python3 - "$work/capture.raw" "$sound" "$upper $arch Detail $detail" <<'PY'
import sys, numpy as np
path, sound, label = sys.argv[1], int(sys.argv[2]), sys.argv[3]
rate = 48000
data = np.fromfile(path, dtype="<i2")
data = data[: len(data) // 2 * 2].reshape(-1, 2).astype(np.float64)
peak = int(np.abs(data).max()) if len(data) else 0
if sound == 0:
	if peak != 0:
		sys.exit(f"FAIL wine-audio: {label}: Sound 0 but the sink got sound (peak {peak})")
	print(f"ok {label}: Sound off, nothing reached the sink")
	sys.exit(0)
loud = np.nonzero(np.abs(data).max(axis=1) > 0)[0]
if len(loud) == 0:
	sys.exit(f"FAIL wine-audio: {label}: Sound on but the sink got silence")
body = data[loud[0]:]
# A dropout is 100 ms or more of exact zeros inside the sound (a starved buffer is 300 ms); a quiet score may round to zero for a moment.
zero = np.abs(body).max(axis=1) == 0
longest = run = 0
for z in zero:
	run = run + 1 if z else 0
	longest = max(longest, run)
step = int(np.abs(np.diff(body, axis=0)).max())
rms = 20 * np.log10(np.sqrt((body**2).mean()) / 32768 + 1e-12)
seconds = len(body) / rate
if longest > rate * 0.1:
	sys.exit(f"FAIL wine-audio: {label}: a dropout of {longest / rate * 1000:.0f} ms")
if step > 6000:
	sys.exit(f"FAIL wine-audio: {label}: a step of {step} between samples (a click)")
if rms > -20:
	sys.exit(f"FAIL wine-audio: {label}: too loud ({rms:.1f} dBFS RMS)")
print(f"ok {label}: {seconds:.1f} s of sound, RMS {rms:.1f} dBFS, peak {int(np.abs(body).max())}, largest step {step}, no dropout")
PY
}

for arch in "${arches[@]}"; do
	[ -f "$root/platforms/win32/dist/$arch/AGINK.SCR" ] || { echo "SKIP wine-audio $arch: not built"; continue; }
	for name in "${savers[@]}"; do
		sleep 2
		run_case "$arch" "$name" 1 1
		sleep 2
		run_case "$arch" "$name" 0 1
	done
done
echo "ok wine-audio"
