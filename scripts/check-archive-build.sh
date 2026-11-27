#!/usr/bin/env bash
# Checks that the source archive GitHub serves builds the same six .scr files as this working tree. Usage: check-archive-build.sh
#
# Makes `git archive` of HEAD into a fresh folder under ~/.cache/afterglow-savers, extracts it, builds there with scripts/build-scr.sh and
# a target folder of its own, builds the working tree the same way, and compares the two SHA256SUMS. Uncommitted changes are not in the
# archive, so a dirty tree is compared against HEAD and says so. The folder is removed when the script ends.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$HOME/.cache/afterglow-savers"
work="$(mktemp -d "$HOME/.cache/afterglow-savers/archive-build.XXXXXX")"
trap 'rm -r "$work"' EXIT

if [ -n "$(git -C "$root" status --porcelain --untracked-files=no)" ]; then
	echo "check-archive-build: the tree has uncommitted changes; the archive is HEAD, so the comparison is against HEAD's files only" >&2
fi

mkdir -p "$work/src"
git -C "$root" archive --format=tar HEAD | tar -x -C "$work/src"
(cd "$work/src" && CARGO_TARGET_DIR="$work/archive-target" bash scripts/build-scr.sh "$work/archive-out") >/dev/null
(cd "$root" && CARGO_TARGET_DIR="$work/tree-target" bash scripts/build-scr.sh "$work/tree-out") >/dev/null

if ! diff -u "$work/tree-out/SHA256SUMS" "$work/archive-out/SHA256SUMS" >"$work/diff.txt"; then
	differing="$(diff "$work/tree-out/SHA256SUMS" "$work/archive-out/SHA256SUMS" | sed -n 's/^> [0-9a-f]*  //p' | tr '\n' ' ')"
	echo "FAIL archive build: the archive's files differ from the tree's: $differing" >&2
	cat "$work/diff.txt" >&2
	exit 1
fi
echo "ok   archive build: the six .scr files from a git archive match the working tree"
cat "$work/archive-out/SHA256SUMS"
