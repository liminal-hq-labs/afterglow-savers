#!/usr/bin/env bash
# Builds the six i586 .scr files from a source tree and writes them, with SHA256SUMS, to a folder. Usage: build-scr.sh OUT_DIR
#
# This is the contract reactos-liminal and Quill build against: it needs no .git (it works in a folder extracted from the source tar
# GitHub serves), builds with platforms/win32/build.sh i586 and copies exactly the six savers, never the test programs. The build output
# goes to `$CARGO_TARGET_DIR/scr-i586` (default `target/` in this tree). It needs rustup with the pinned toolchain, `rust-src` and
# `llvm-tools`. The files are byte-identical in any directory.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

if [ $# -ne 1 ] || [ -z "$1" ]; then
	echo "usage: build-scr.sh OUT_DIR" >&2
	exit 2
fi
out_dir="$1"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"

if ! command -v rustup >/dev/null 2>&1; then
	echo "build-scr: rustup is not installed; the savers build with the toolchain pinned in rust-toolchain.toml" >&2
	exit 1
fi
toolchain="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$root/rust-toolchain.toml")"
toolchain="${toolchain:-1.98.0}"
if ! installed="$(cd "$root" && rustup component list --installed --toolchain "$toolchain" 2>/dev/null)"; then
	echo "build-scr: toolchain $toolchain is missing; run: rustup toolchain install $toolchain --component rust-src,llvm-tools" >&2
	exit 1
fi
for component in rust-src llvm-tools; do
	if ! grep -q "^$component" <<<"$installed"; then
		echo "build-scr: component $component is missing for $toolchain; run: rustup component add $component --toolchain $toolchain" >&2
		exit 1
	fi
done

scr_dir="$target_dir/scr-i586"
CARGO_TARGET_DIR="$target_dir" bash "$root/platforms/win32/build.sh" i586 --out "$scr_dir"

mkdir -p "$out_dir"
names=(AGCOLLCT.SCR AGCRACK.SCR AGINK.SCR AGMAZE.SCR AGQUILL.SCR AGTWILIT.SCR)
for name in "${names[@]}"; do
	cp "$scr_dir/$name" "$out_dir/$name"
done
(cd "$out_dir" && sha256sum "${names[@]}" >SHA256SUMS)
echo "build-scr: wrote ${#names[@]} files and SHA256SUMS to $out_dir"
