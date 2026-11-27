#!/usr/bin/env bash
# Writes review contact sheets of the shipped icons: every .ico frame at 1x and at 4x, on the classic cream window and on the Dusk desktop.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

# Usage: contact-sheet.sh [OUT_DIR]   (default ~/Documents/Liminal HQ/Afterglow Savers/icons)

cd "$(dirname "${BASH_SOURCE[0]}")/.."
out="${1:-$HOME/Documents/Liminal HQ/Afterglow Savers/icons}"
mkdir -p "$out"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

names=(agmaze agcrack agtwilit agink agcollct agquill)
for ground in cream:#d4d0c8:#1c1b20 dusk:#090e2a:#e0e0e0; do
	IFS=: read -r label colour ink <<<"$ground"
	rows=()
	for name in "${names[@]}"; do
		cells=()
		# frames in the file are 48, 32 and 16
		for index in 2 1 0; do
			magick "platforms/win32/res/$name.ico[$index]" -background "$colour" -alpha remove -alpha off "$scratch/$name-$index-1x.png"
			magick "$scratch/$name-$index-1x.png" -filter point -resize 400% "$scratch/$name-$index-4x.png"
		done
		magick -background "$colour" \
			\( "$scratch/$name-2-1x.png" -gravity center -extent 56x56 \) \( "$scratch/$name-1-1x.png" -gravity center -extent 56x56 \) \( "$scratch/$name-0-1x.png" -gravity center -extent 56x56 \) \
			\( "$scratch/$name-2-4x.png" -gravity center -extent 80x80 \) \( "$scratch/$name-1-4x.png" -gravity center -extent 144x144 \) \( "$scratch/$name-0-4x.png" -gravity center -extent 208x208 \) \
			+append -gravity west -extent 680x216 \
			-fill "$ink" -pointsize 14 -gravity southwest -annotate +8+4 "$name" "$scratch/$name-row.png"
		rows+=("$scratch/$name-row.png")
	done
	magick -background "$colour" "${rows[@]}" -append -bordercolor "$colour" -border 16 "$out/icons-$label.png"
done
echo "contact-sheet: wrote $out"
