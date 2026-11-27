#!/usr/bin/env bash
# Checks the structure of an Afterglow .scr for an architecture. Usage: check-pe.sh <i586|x86_64> <scr>
#
# i586: a 32-bit Intel GUI program (subsystem 4.0, so Windows 95 and ReactOS load it), fixed at its base with no relocations, only code,
# read-only data and resources. x86_64: a PE32+ GUI program that keeps its relocations and loads at a random base, with high-entropy
# ASLR and no executable-stack or data-execution opt-out. No C runtime in either, and string 1, an icon group and version information.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

arch="${1:?usage: check-pe.sh <i586|x86_64> <scr>}"
exe="${2:?usage: check-pe.sh <i586|x86_64> <scr>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/objdump.sh"
objdump="$(find_objdump)"

fail() { echo "FAIL check-pe $arch: $*" >&2; exit 1; }
[ -f "$exe" ] || fail "$exe does not exist"

# The machine, the PE flavour, the subsystem version and the sections each architecture must have.
case "$arch" in
i586) format="coff-i386"; magic="PE32"; subsystem="4.0"; sections=".text .rdata .rsrc " ;;
x86_64) format="coff-x86-64"; magic="PE32+"; subsystem="5.2"; sections=".text .rdata .rsrc .reloc " ;;
*) fail "unknown architecture $arch" ;;
esac

[ "$("$objdump" -f "$exe" | awk '/file format/ {print $NF}')" = "$format" ] || fail "the machine is not $format"
header="$("$objdump" -p "$exe")"
field() { awk -v key="$1" '$1 == key {print $2}' <<<"$header" | head -1; }
grep -q "^Magic.*($magic)" <<<"$header" || fail "not a $magic file"
[ "$(field MajorSubsystemVersion).$(field MinorSubsystemVersion)" = "$subsystem" ] || fail "the subsystem version is not $subsystem"
grep -qi "Subsystem *00000002" <<<"$header" || fail "not a GUI program"

if [ "$arch" = i586 ]; then
	grep -q "relocations stripped" <<<"$header" || fail "the executable has relocations (it is built /FIXED)"
else
	grep -q "relocations stripped" <<<"$header" && fail "the executable has no relocations, so it cannot be moved by ASLR"
	grep -q "DYNAMIC_BASE" <<<"$header" || fail "DYNAMIC_BASE is not set"
	grep -q "HIGH_ENTROPY_VA" <<<"$header" || fail "HIGH_ENTROPY_VA is not set"
fi
grep -q "NX_COMPAT" <<<"$header" || fail "NX_COMPAT is not set"

# Sections: code, read-only data and, where the file can move, its relocations. A `.data` section would mean writable data the program
# should not have, and the `/MERGE:.data=.rdata` in .cargo/config.toml would make it fault, so the list must stay exactly this.
found="$("$objdump" -h "$exe" | awk '$1 ~ /^[0-9]+$/ {print $2}' | tr '\n' ' ')"
[ "$found" = "$sections" ] || fail "sections are '$found', expected '$sections'"

# Resources: string 1 (the description), version information and an icon group, which `build.rs` writes into every file.
resources="$(strings -el "$exe" || true)"
grep -q "^Afterglow\|^Quill" <<<"$resources" || fail "string 1 is missing"
grep -q "^VS_VERSION_INFO" <<<"$resources" || fail "the version information is missing"
rsrc_size="$("$objdump" -h "$exe" | awk '$2 == ".rsrc" {print strtonum("0x" $3)}')"
[ "${rsrc_size:-0}" -ge 4096 ] || fail "the resource section is only ${rsrc_size:-0} bytes, so the icon is missing"

dlls="$(awk '/DLL Name:/ {print tolower($3)}' <<<"$header" | sort -u | tr '\n' ' ')"
case "$dlls" in
*msvcrt* | *ucrt* | *vcruntime* | *api-ms-*) fail "imports a C runtime: $dlls" ;;
esac
echo "ok check-pe $arch: $magic GUI, subsystem $subsystem, sections $found, imports from $dlls"
