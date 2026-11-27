#!/usr/bin/env bash
# Writes a review contact sheet of the Afterglow settings dialog's art: the six banners with their icon and title, and the palette
# swatches by night and by day, plain and chosen, on Windows 95's face and on the ReactOS Afterglow theme's.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

# Usage: dialog-sheet.sh [OUT.png]   (default ~/Documents/Liminal HQ/Afterglow Savers/previews/dialog-v2-art-sheet.png)
#
# Everything is shown at twice its size, with hard pixel edges.
# The banners and swatches come from `savers-preview banner` and `savers-preview swatches`, which draw them as the dialog does, at the
# sizes the dialog has at 96 dpi (a 564 × 65 banner, 50 × 49 swatches, 40 × 49 for Quill Ghostwriter's six). The title, the "Settings"
# line and the swatch names are set here in DejaVu Sans as a stand-in for the dialog font, which GDI draws in the real dialog.

cd "$(dirname "${BASH_SOURCE[0]}")/.."
out="${1:-$HOME/Documents/Liminal HQ/Afterglow Savers/previews/dialog-v2-art-sheet.png}"
mkdir -p "$(dirname "$out")"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

cargo build --release --locked -q -p savers-preview
preview=target/release/savers-preview

# scene, icon file, title
savers=(
	"maze agmaze Afterglow Maze"
	"cracktro agcrack Afterglow Cracktro"
	"twilight agtwilit Afterglow Twilight"
	"ink agink Afterglow Ink"
	"collection agcollct Afterglow Collection"
	"ghostwriter agquill Quill Ghostwriter"
)
rows=()
for entry in "${savers[@]}"; do
	read -r scene icon title <<<"$entry"
	inks="$("$preview" banner --scene "$scene" --out "$scratch/$scene.png")"
	title_ink="#$(sed -E 's/.*title=([0-9a-f]+).*/\1/' <<<"$inks")"
	subtitle_ink="#$(sed -E 's/.*subtitle=([0-9a-f]+).*/\1/' <<<"$inks")"
	shadow_ink="#$(sed -E 's/.*shadow=([0-9a-f]+).*/\1/' <<<"$inks")"
	magick "platforms/win32/res/$icon.ico[1]" "$scratch/$scene-icon.png"
	magick "$scratch/$scene.png" "$scratch/$scene-icon.png" -geometry +12+16 -composite \
		-font DejaVu-Sans-Bold -pointsize 17 -fill "$shadow_ink" -annotate +55+38 "$title" \
		-fill "$title_ink" -annotate +54+37 "$title" \
		-font DejaVu-Sans -pointsize 11 -fill "$subtitle_ink" -annotate +54+54 "Settings" "$scratch/$scene-row.png"
	rows+=("$scratch/$scene-row.png")
done
magick "${rows[@]}" -background '#d4d0c8' -splice 0x8 -append -filter point -resize 200% "$scratch/banners.png"

# One row of swatches: $1 the scene (its palette list), $2 the face, $3 the size, $4 the variant (night, night-chosen, day, day-chosen).
swatch_row() {
	local scene="$1" face="$2" size="$3" variant="$4" cells=() line file top name
	mkdir -p "$scratch/sw-$scene-${face#\#}"
	while read -r line; do
		file="${line%% *}"
		case "$file" in *-"$variant".png) ;; *) continue ;; esac
		top="$(awk '{print $3}' <<<"$line")"
		name="$(awk '{print $6}' <<<"$line")"
		magick "$file" -font DejaVu-Sans -pointsize 10 -fill '#000000' -gravity north -annotate "+0+$((top + 1))" "$name" "$file.named.png"
		cells+=("$file.named.png")
	done < <("$preview" swatches --scene "$scene" --size "$size" --face "${face#\#}" --out "$scratch/sw-$scene-${face#\#}")
	magick "${cells[@]}" -background "$face" -splice 3x0 +append "$scratch/row-$scene-${face#\#}-$variant.png"
	echo "$scratch/row-$scene-${face#\#}-$variant.png"
}

panels=()
for face in '#c0c0c0' '#d4d0c8'; do
	for variant in night night-chosen day day-chosen; do
		five="$(swatch_row ink "$face" 50x49 "$variant")"
		six="$(swatch_row ghostwriter "$face" 40x49 "$variant")"
		magick "$five" "$six" -background "$face" -splice 24x0 +append "$scratch/pair-${face#\#}-$variant.png"
	done
	magick "$scratch"/pair-"${face#\#}"-{night,night-chosen,day,day-chosen}.png -background "$face" -splice 0x4 -append \
		-bordercolor "$face" -border 8 "$scratch/panel-${face#\#}.png"
	panels+=("$scratch/panel-${face#\#}.png")
done
magick "${panels[@]}" -append "$scratch/swatches-1x.png"
magick "$scratch/swatches-1x.png" -filter point -resize 200% "$scratch/swatches-2x.png"

magick "$scratch/banners.png" "$scratch/swatches-2x.png" -background '#d4d0c8' -gravity northwest -splice 24x0 +append +repage \
	-bordercolor '#d4d0c8' -border 12 "$out"
echo "dialog-sheet: wrote $out"
