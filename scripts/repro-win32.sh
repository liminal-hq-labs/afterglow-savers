#!/usr/bin/env bash
# Builds the Win32 .scr files (Pentium and x86-64) in three different directories and compares the bytes. Usage: repro-win32.sh [arch]...
#
# The three paths differ in depth and length, because a build that depends on the checkout path can hide behind two lucky ones. The copies
# and their build output live in a folder of their own under ~/.cache/afterglow-savers (one per run, so two runs never meet) and are
# removed when the script ends. `platforms/win32/crates` is a symlink to `../../crates`, which `cp -P` keeps, so the core is reached
# under the workspace's root in every copy.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
arches=("$@")
[ ${#arches[@]} -gt 0 ] || arches=(i586 x86_64)
mkdir -p "${HOME}/.cache/afterglow-savers"
work="$(mktemp -d "${HOME}/.cache/afterglow-savers/repro-win32.XXXXXX")"
trap 'rm -rf "$work"' EXIT

copies=(one/afterglow two/second-place three/a/much/deeper/directory/with-a-longer-name)
for copy in "${copies[@]}"; do
	mkdir -p "$work/$copy"
	(cd "$root" && git ls-files --cached --others --exclude-standard -z | xargs -0 -I{} cp -P --parents {} "$work/$copy/")
done
for arch in "${arches[@]}"; do
	for copy in "${copies[@]}"; do
		(cd "$work/$copy" && CARGO_TARGET_DIR="$work/$copy/target" bash platforms/win32/build.sh "$arch" --out "$work/$copy/out/$arch" >/dev/null 2>&1) || {
			echo "FAIL repro win32 $arch: the build in $copy failed" >&2
			exit 1
		}
	done
	for name in AGMAZE AGCRACK AGTWILIT AGINK AGCOLLCT AGQUILL; do
		first="$work/${copies[0]}/out/$arch/$name.SCR"
		for copy in "${copies[@]:1}"; do
			if ! cmp -s "$first" "$work/$copy/out/$arch/$name.SCR"; then
				echo "FAIL repro win32 $arch: $name.SCR in $copy differs from the build in ${copies[0]}" >&2
				exit 1
			fi
		done
	done
	echo "ok repro win32 $arch: six files byte-identical across ${#copies[@]} directories ($(cat "$work/${copies[0]}/out/$arch/"*.SCR | sha256sum | cut -d' ' -f1))"
done
