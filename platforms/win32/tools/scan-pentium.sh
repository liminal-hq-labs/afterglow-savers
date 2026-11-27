#!/usr/bin/env bash
# Scans the code of an Afterglow .scr for an instruction a Pentium lacks: a vector register, a conditional move (integer or x87), or the newer bit tricks.
# Usage: scan-pentium.sh <exe>
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

exe="${1:?usage: scan-pentium.sh <exe>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/objdump.sh"
objdump="$(find_objdump)"

listing="$("$objdump" -d --no-show-raw-insn "$exe")"
[ "$(wc -l <<<"$listing")" -gt 100 ] || { echo "FAIL scan-pentium: no code found in $exe" >&2; exit 1; }
bad="$(grep -Ei '(xmm|ymm|zmm)[0-9]|[[:space:]]%?mm[0-7]([^0-9a-z]|$)|[[:space:]](cmov[a-z]+|fcmov[a-z]*|fcomi[p]?|fucomi[p]?|emms|movbe|popcnt|lzcnt|tzcnt|prefetch[a-z0-9]*|[ls]fence|mfence)[[:space:]]' <<<"$listing" || true)"
if [ -n "$bad" ]; then
	echo "FAIL scan-pentium: instructions a Pentium lacks:" >&2
	head -20 <<<"$bad" >&2
	exit 1
fi
echo "ok scan-pentium: $(wc -l <<<"$listing") lines of code, Pentium instructions only"
