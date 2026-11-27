#!/usr/bin/env bash
# Checks the Win16 workspace: format, lints, the packer's tests, then builds QUILLGW.SCR and checks the executable, its segments and its size.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: platforms/win16/check.sh
#
# Nothing here needs Windows, Wine or an emulator. The packer fails when a code segment passes its budget in res/quillgw.segments.json; this
# script adds the whole-file limit (64 KiB, the size of one segment, so the saver stays a small file), the instruction check (386 only) and
# the branch check (every near branch lands on an instruction in its own segment).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

cargo fmt --all -- --check
RUSTC_BOOTSTRAP=1 cargo clippy --release --locked -- -D warnings
python3 -m unittest discover -s tests

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
OUT_DIR="$out" bash build.sh
desc="$(file "$out/QUILLGW.SCR")"
echo "$desc"
if ! grep -Eq 'NE (version [0-9.]+ )?for MS Windows' <<<"$desc"; then
	echo "check: QUILLGW.SCR is not an NE executable for Windows" >&2
	exit 1
fi
size="$(stat -c %s "$out/QUILLGW.SCR")"
if [ "$size" -gt 65536 ]; then
	echo "check: QUILLGW.SCR is $size bytes; the limit is 65536" >&2
	exit 1
fi
python3 tools/check_segments.py "$out/QUILLGW.SCR"
python3 tools/check_cpu.py "$out/QUILLGW.SCR"
python3 tools/ne_dump.py "$out/QUILLGW.SCR" | grep -E '^(resident|non-resident) names' | grep -q 'SCRNSAVE : Quill Ghostwriter'
echo "check: QUILLGW.SCR is $size bytes, an NE executable described as a screen saver"
