#!/usr/bin/env bash
# Builds QUILLGW.SCR, Quill Ghostwriter for Windows 3.x: Rust to an ELF with rust-lld, then tools/ne_pack.py to an NE executable.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

# Usage: build.sh [--regen]
#
# Writes out/QUILLGW.SCR and out/QUILLGW.MAP. The module description `SCRNSAVE : Quill Ghostwriter` is what Windows 3.x's Desktop
# applet lists in its Screen Saver box. `--regen` rebuilds src/imports.rs from tools/imports.tbl first. Set CARGO_TARGET_DIR to keep the
# build output outside the tree, OUT_DIR to move out/.

cd "$(dirname "${BASH_SOURCE[0]}")"
export RUSTC_BOOTSTRAP=1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
mkdir -p "$CARGO_TARGET_DIR"

if [ "${1:-}" = "--regen" ]; then
	python3 tools/gen_imports.py tools/imports.tbl src/imports.rs
fi

if ! cargo build --release ${W16_FEATURES:+--features "$W16_FEATURES"} --bin quillgw 2>"$CARGO_TARGET_DIR/build.log"; then
	cat "$CARGO_TARGET_DIR/build.log" >&2
	exit 1
fi

out="${OUT_DIR:-out}"
mkdir -p "$out"
python3 tools/ne_pack.py --module QUILLGW --description "SCRNSAVE : Quill Ghostwriter" --stack 0x2000 --heap 0x1000 --shadow 64 \
	--resources res/quillgw.json --segments res/quillgw.segments.json --map "$out/QUILLGW.MAP" \
	"$CARGO_TARGET_DIR/win16/release/quillgw" "$out/QUILLGW.SCR"
