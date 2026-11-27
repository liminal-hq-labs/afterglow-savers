#!/usr/bin/env bash
# Prints the path of llvm-objdump: the toolchain's own (the llvm-tools component), else one on the PATH. Sourced by the check scripts.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

find_objdump() {
	local host objdump
	host="$(rustc -vV | sed -n 's/^host: //p')"
	objdump="$(rustc --print sysroot)/lib/rustlib/$host/bin/llvm-objdump"
	[ -x "$objdump" ] || objdump="$(command -v llvm-objdump llvm-objdump-19 llvm-objdump-18 llvm-objdump-17 2>/dev/null | head -1 || true)"
	[ -n "$objdump" ] || { echo "llvm-objdump not found" >&2; return 1; }
	echo "$objdump"
}
