#!/usr/bin/env bash
# Renders a scene's score alone and puts it on an existing preview video, without rendering the picture again (SAV-D36).
# Usage: preview-audio.sh SCENE VIDEO.mp4 GAIN_DB [savers-preview audio options]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The score is rendered at 48 kHz (savers-preview audio, with the options the video was rendered with: --seed, --set, --from, and
# --loop or --seconds), lifted by GAIN_DB to a listening level, written as a FLAC master beside the video (SCENE-audio.flac), and
# muxed onto a copy of the video as VIDEO-audio.mp4: the picture is stream-copied, the sound encoded as AAC at 256 kbps. The silent
# video is kept. Needs ffmpeg.

set -euo pipefail
scene="${1:?usage: preview-audio.sh SCENE VIDEO.mp4 GAIN_DB [options]}"
video="${2:?usage: preview-audio.sh SCENE VIDEO.mp4 GAIN_DB [options]}"
gain="${3:?usage: preview-audio.sh SCENE VIDEO.mp4 GAIN_DB [options]}"
shift 3
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dir="$(cd "$(dirname "$video")" && pwd)"
base="$(basename "$video" .mp4)"
mkdir -p "$HOME/.cache/afterglow-savers"
work="$(mktemp -d "$HOME/.cache/afterglow-savers/audio.XXXXXX")"
trap 'rm -rf "$work"' EXIT
cargo build -q --release --locked -p savers-preview --manifest-path "$root/Cargo.toml"
"$root/target/release/savers-preview" audio --scene "$scene" --out "$work/score.wav" "$@"
ffmpeg -loglevel error -y -i "$work/score.wav" -af "volume=${gain}dB" -c:a flac "$dir/$scene-audio.flac"
ffmpeg -loglevel error -y -i "$video" -i "$dir/$scene-audio.flac" -map 0:v -map 1:a -c:v copy -c:a aac -b:a 256k -shortest \
	-movflags +faststart "$dir/$base-audio.mp4"
echo "preview-audio: $dir/$base-audio.mp4 and $dir/$scene-audio.flac"
