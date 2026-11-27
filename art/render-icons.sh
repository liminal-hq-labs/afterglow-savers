#!/usr/bin/env bash
# Renders the icon masters in art/masters to 48, 32 and 16 px and packs them into platforms/win32/res/<name>.ico.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

# Usage: render-icons.sh [--check]
#
# Regenerates the masters (art/gen-icons.py), then renders each with rsvg-convert: the 48 px master at
# 48 and 32 px, the hinted 16 px frame at 16. The 48 and 32 px images are quantized to a fixed 255-colour
# palette with a 1-bit mask, the look Windows 95 and 256-colour displays show; the 16 px frame is
# already flat colour. The .ico files hold BMP images (no PNG), the form every Windows since 95 reads.
# With --check the files are rendered into a scratch folder and compared with the committed ones.

cd "$(dirname "${BASH_SOURCE[0]}")/.."

declare -A files=([maze]=agmaze [cracktro]=agcrack [twilight]=agtwilit [ink]=agink [collection]=agcollct [quill]=agquill)
out=platforms/win32/res
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

if [ "${1:-}" = "--check" ]; then
	out="$scratch/ico"
fi
mkdir -p "$out"

python3 art/gen-icons.py

for name in "${!files[@]}"; do
	rsvg-convert -w 48 -h 48 "art/masters/$name.svg" -o "$scratch/$name-48.png"
	rsvg-convert -w 32 -h 32 "art/masters/$name.svg" -o "$scratch/$name-32.png"
	rsvg-convert -w 16 -h 16 "art/masters/$name-16.svg" -o "$scratch/$name-16.png"
	for size in 48 32; do
		magick "$scratch/$name-$size.png" -channel A -threshold 50% +channel -background none -dither None -colors 255 \
			-define png:exclude-chunks=date,time "PNG8:$scratch/$name-$size-q.png"
	done
	cp "$scratch/$name-16.png" "$scratch/$name-16-q.png"
	magick "$scratch/$name-48-q.png" "$scratch/$name-32-q.png" "$scratch/$name-16-q.png" -colors 255 -dither None "$out/${files[$name]}.ico"
done

if [ "${1:-}" = "--check" ]; then
	for name in "${files[@]}"; do
		cmp "$out/$name.ico" "platforms/win32/res/$name.ico" || { echo "render-icons: $name.ico differs from the master" >&2; exit 1; }
	done
	echo "render-icons: ok: the committed icons match the masters"
fi
