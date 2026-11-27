#!/usr/bin/env bash
# Builds, or lints, the Win32 screensavers with no C compiler and no C library.
# Usage: build.sh [i586|x86_64] [build|clippy] [--out DIR]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Rust, rust-lld's lld-link and a custom target for each of the Pentium and x86-64 (`<arch>-win32.json`). The programs are copied to DIR,
# default platforms/win32/dist/<arch>, as AGMAZE.SCR and the rest. `core` is built from source (-Zbuild-std) for every arch: the prebuilt
# one for a Pentium assumes SSE2, which a Pentium lacks, and none of them carries the memory routines (`memcpy` and the rest) a program
# with no C library needs. The options are unstable, so RUSTC_BOOTSTRAP=1 lets the pinned stable toolchain accept them. Set
# CARGO_TARGET_DIR to keep the build output outside the tree. The result is reproducible: the same source and toolchain give the same
# files.

set -euo pipefail

arch="i586"
action="build"
out=""
while [ $# -gt 0 ]; do
	case "$1" in
	i586 | x86_64) arch="$1" ;;
	build | clippy) action="$1" ;;
	--out) shift; out="${1:?--out needs a directory}" ;;
	*) echo "usage: build.sh [i586|x86_64] [build|clippy] [--out DIR]" >&2; exit 2 ;;
	esac
	shift
done

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"
out="${out:-$here/dist/$arch}"
export RUSTC_BOOTSTRAP=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
target="${arch}-win32"
cargo_args=(--release --locked --target "$target.json" -Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec)

case "$action" in
clippy)
	cargo clippy "${cargo_args[@]}" -- -D warnings
	;;
build)
	cargo build "${cargo_args[@]}"
	mkdir -p "$out"
	total=0
	for name in agmaze agcrack agtwilit agink agcollct agquill; do
		upper="$(tr '[:lower:]' '[:upper:]' <<<"$name")"
		cp "${CARGO_TARGET_DIR:-$here/target}/$target/release/$name.exe" "$out/$upper.SCR"
		size="$(stat -c %s "$out/$upper.SCR")"
		[ "$name" = agquill ] || total=$((total + size))
		echo "build: $out/$upper.SCR ($size bytes)"
	done
	echo "build: $arch total $total bytes (AGQUILL.SCR is outside the five-file budget)"
	# The preview stand-in the Wine smoke test uses; it is not one of the five and is not checked against the budgets.
	cp "${CARGO_TARGET_DIR:-$here/target}/$target/release/previewhost.exe" "$out/previewhost.exe"
	;;
esac
