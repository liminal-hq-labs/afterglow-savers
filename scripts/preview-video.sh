#!/usr/bin/env bash
# Renders a scene's frames with savers-preview and makes a GIF, an MP4 and a contact sheet from them. Usage: preview-video.sh SCENE OUTDIR [savers-preview options]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The frames go to a scratch folder under ~/.cache/afterglow-savers and are removed at the end; the MP4 (H.264 at the render size,
# every frame), the GIF (360 wide, 10 fps, the first 20 seconds, to stay a few megabytes) and the contact sheet (one frame every two
# seconds, 4 across) go to OUTDIR. Needs ffmpeg and ImageMagick's `magick`.

set -euo pipefail
scene="${1:?usage: preview-video.sh SCENE OUTDIR [options]}"
outdir="${2:?usage: preview-video.sh SCENE OUTDIR [options]}"
shift 2
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$HOME/.cache/afterglow-savers" "$outdir"
work="$(mktemp -d "$HOME/.cache/afterglow-savers/video.XXXXXX")"
trap 'rm -rf "$work"' EXIT
cargo build -q --release --locked -p savers-preview --manifest-path "$root/Cargo.toml"
"$root/target/release/savers-preview" frames --scene "$scene" --fps 30 --step 0.0666 --out "$work" "$@"
ffmpeg -loglevel error -y -framerate 15 -i "$work/frame-%05d.png" -c:v libx264 -pix_fmt yuv420p -crf 18 "$outdir/$scene.mp4"
ffmpeg -loglevel error -y -framerate 15 -t 20 -i "$work/frame-%05d.png" \
	-vf "fps=10,scale=360:-2:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3" "$outdir/$scene.gif"
mapfile -t picks < <(find "$work" -name 'frame-*.png' | sort | awk 'NR % 30 == 1')
magick montage "${picks[@]}" -tile 4x -geometry 320x+3+3 -background '#050507' -quality 88 "$outdir/$scene-sheet.jpg"
echo "preview: $outdir/$scene.gif, $scene.mp4 and $scene-sheet.jpg"
