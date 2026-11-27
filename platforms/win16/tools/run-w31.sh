#!/usr/bin/env bash
# Runs QUILLGW.SCR on Windows for Workgroups 3.11 in DOSBox-X (in Docker, on its own Xvfb) and saves screendumps of that display.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: [W31_TREE=DIR] platforms/win16/tools/run-w31.sh [options] QUILLGW.SCR PREFIX SECONDS...
#
#   --vga            plain VGA at 640 x 480 in 16 colours (VGA.DRV) instead of the tree's own driver
#   --ini "K=v ..."  CONTROL.INI keys for [Screen Saver.Quill Ghostwriter]
#   --saver          install it the way the Desktop applet does: the file in the system folder, SCRNSAVE.EXE= in SYSTEM.INI's [boot],
#                    ScreenSaveActive and a one-minute ScreenSaveTimeOut in WIN.INI, so Windows itself starts it with /s when idle
#   --win "CMDLINE"  start Windows with this command line instead of Program Manager: `D:\QUILLGW.EXE /s` runs a copy of the saver named
#                    .EXE on drive D (Windows will not start a file named .SCR from the command line), or `C:\WINDOWS\CONTROL.EXE DESKTOP`
#   --click "x,y;x,y|x,y" after each screendump in turn, click one group of points (separated by `;`) of the display, or press keys (`alt+t`); `|` ends a group
#
# With Log=1 in --ini the saver writes the command line of each start to C:\QGWn.LOG, which is printed at the end.
# W31_TREE (default ~/.cache/quill16-play/w31-base) is a Windows 3.11 installation as a DOS drive (a folder with WINDOWS/). It is copied first and the copy is thrown away, so the
# tree is never changed. A screendump is saved as PREFIX-<s>.png at each number of seconds. The image comes from the Xvfb display that
# only this script and its container use (DISPLAY :97 unless W31_DISPLAY says otherwise); the host's desktop is never captured.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"

vga=0 ini="" saver=0 wincmd="" clicks=""
while [ $# -gt 0 ]; do
	case "$1" in
		--vga) vga=1; shift ;;
		--ini) ini="$2"; shift 2 ;;
		--saver) saver=1; shift ;;
		--win) wincmd="$2"; shift 2 ;;
		--click) clicks="$2"; shift 2 ;;
		*) break ;;
	esac
done
scr="$1" prefix="$2"
shift 2
W31_TREE="${W31_TREE:-$HOME/.cache/quill16-play/w31-base}"

export DISPLAY="${W31_DISPLAY:-:97}"
work="$(mktemp -d "$HOME/.cache/afterglow-savers/w31run.XXXXXX")"
name="qgw-w31-$$"
Xvfb "$DISPLAY" -screen 0 "$([ "$vga" = 1 ] && echo 640x480 || echo 800x600)x24" -nolisten tcp > /dev/null 2>&1 &
xvfb_pid=$!
cleanup() { docker rm -f "$name" > /dev/null 2>&1 || true; kill "$xvfb_pid" 2> /dev/null || true; rm -rf "$work"; }
trap cleanup EXIT
sleep 1

cp -a --reflink=auto "$W31_TREE" "$work/c"
mkdir -p "$work/d" "$work/conf"
win="$work/c/WINDOWS"
cp "$scr" "$work/d/QUILLGW.SCR"
cp "$scr" "$work/d/QUILLGW.EXE"
sed -i 's/^run=.*/run=/' "$win/WIN.INI"
# The owner's tree starts Notepad and Trumpet Winsock from its Startup group; a test wants a quiet desktop.
sed -i -e '/^Group5=/d' -e '/^Group6=/d' "$win/PROGMAN.INI"
crlf() { sed -i 's/\r*$/\r/' "$1"; }

if [ "$vga" = 1 ]; then
	sed -i -e 's/^display\.drv=.*/display.drv=vga.drv/' -e 's/^display=.*/display=*vddvga/' "$win/SYSTEM.INI"
	sed -i 's/^display\.drv=S3 .*/display.drv=VGA/' "$win/SYSTEM.INI"
fi
if [ -n "$ini" ]; then
	{
		printf '\r\n[Screen Saver.Quill Ghostwriter]\r\n'
		for pair in $ini; do printf '%s\r\n' "$pair"; done
	} >> "$win/CONTROL.INI"
fi
if [ "$saver" = 1 ]; then
	cp "$scr" "$win/SYSTEM/QUILLGW.SCR"
	sed -i 's/^shell=progman.exe/shell=progman.exe\r\nSCRNSAVE.EXE=C:\\WINDOWS\\SYSTEM\\QUILLGW.SCR/' "$win/SYSTEM.INI"
	sed -i -e 's/^ScreenSaveActive=.*/ScreenSaveActive=1/' -e 's/^ScreenSaveTimeOut=.*/ScreenSaveTimeOut=60/' "$win/WIN.INI"
fi
crlf "$win/SYSTEM.INI"
crlf "$win/WIN.INI"
crlf "$win/CONTROL.INI"

{
	cat << 'CONF'
[sdl]
output=surface
fullscreen=false
windowresolution=original
autolock=false
mouse_emulation=always
[dosbox]
CONF
	if [ "$vga" = 1 ]; then echo "machine=vgaonly"; else echo "machine=svga_s3"; fi
	cat << 'CONF'
memsize=32
[cpu]
core=normal
cputype=pentium
cycles=max
[dos]
ems=false
xms=true
umb=true
[autoexec]
CONF
	printf '%s\n' '@echo off' 'mount c /c' 'mount d /d' 'c:' 'set PATH=c:\windows;c:\utils' 'cd \windows' "win /3 $wincmd" 'exit'
} > "$work/conf/run.conf"

docker run -d --name "$name" --user "$(id -u):$(id -g)" --network host -v /tmp/.X11-unix:/tmp/.X11-unix -e DISPLAY="$DISPLAY" \
	-e SDL_AUDIODRIVER=dummy -e HOME=/tmp -v "$work/c:/c" -v "$work/d:/d" -v "$work/conf:/conf" --entrypoint dosbox-x \
	"${DOSBOX_IMAGE:-quill16-dosbox-x:local}" -conf /conf/run.conf -nomenu -fastlaunch > /dev/null

# The whole of this script's own display, which the DOSBox-X window fills.
shot() {
	import -window root "$1"
}
start=$(date +%s)
IFS='|' read -ra groups <<< "$clicks"
index=0
for at in "$@"; do
	now=$(( $(date +%s) - start ))
	[ "$at" -gt "$now" ] && sleep $(( at - now ))
	shot "$prefix-$at.png"
	if [ "$index" -lt "${#groups[@]}" ]; then
		IFS=';' read -ra points <<< "${groups[$index]}"
		index=$((index + 1))
		for point in "${points[@]}"; do
			case "$point" in
				*,*) xdotool mousemove "${point%,*}" "${point#*,}" click 1 ;;
				*) xdotool key "$point" ;;
			esac
			sleep 1.5
		done
	fi
done
for log in "$work"/c/QGW?.LOG; do
	[ -f "$log" ] && { printf '%s: ' "$(basename "$log")"; tr -d '\r' < "$log"; }
done
true
