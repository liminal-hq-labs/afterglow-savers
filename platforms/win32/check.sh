#!/usr/bin/env bash
# Checks the built .scr files of an architecture: structure, imports, Pentium instructions (i586 only), and the size budgets.
# Usage: check.sh [i586|x86_64] [directory]
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
arch="${1:-i586}"
dir="${2:-$here/dist/$arch}"
case "$arch" in
i586 | x86_64) ;;
*) echo "usage: check.sh [i586|x86_64] [directory]" >&2; exit 2 ;;
esac
# SAV-D14 as revised by SAV-D29: 384 KiB each and 1.5 MiB for the five main files, a guard against accidents rather than a target;
# AGQUILL (SAV-D20) has its own 384 KiB, outside the five.
each=393216
all=1572864
total=0
for name in AGMAZE AGCRACK AGTWILIT AGINK AGCOLLCT AGQUILL; do
	scr="$dir/$name.SCR"
	bash "$here/tools/check-pe.sh" "$arch" "$scr"
	bash "$here/tools/check-imports.sh" "$arch" "$scr"
	[ "$arch" != i586 ] || bash "$here/tools/scan-pentium.sh" "$scr"
	size="$(stat -c %s "$scr")"
	if [ "$size" -gt "$each" ]; then
		echo "FAIL check: $name.SCR is $size bytes, over the $each byte budget" >&2
		exit 1
	fi
	[ "$name" = AGQUILL ] || total=$((total + size))
done
if [ "$total" -gt "$all" ]; then
	echo "FAIL check: the five files are $total bytes, over the $all byte budget" >&2
	exit 1
fi
echo "ok check win32 $arch: five files, $total bytes of $all"
