# Local gate for Afterglow Screensavers. `make check` is the gate before every commit.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

export CARGO_BUILD_JOBS ?= 4

.PHONY: check check-fast headers fmt clippy test preview win32 win32-fmt win32-clippy win32-test win32-build repro smoke

# `check` is the whole gate: the host checks, the Win32 build and its structure, import, Pentium and size checks, the byte-identical
# rebuild in three directories, and the Wine run (which says SKIP when Wine or Xvfb is missing).
check: check-fast win32 repro smoke

check-fast: headers fmt clippy test

headers:
	bash scripts/check-headers.sh

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --locked -- -D warnings

test:
	cargo nextest run --workspace --locked --no-tests=pass

preview:
	cargo build --release --locked -p savers-preview

# The Win32 shells are their own workspace, so they have their own targets.
win32: win32-fmt win32-clippy win32-test win32-build

win32-fmt:
	cd platforms/win32 && cargo fmt --all -- --check

win32-clippy:
	bash platforms/win32/build.sh i586 clippy
	bash platforms/win32/build.sh x86_64 clippy
	cd platforms/win32 && cargo clippy --lib --tests --locked --target x86_64-unknown-linux-gnu -- -D warnings

win32-test:
	cd platforms/win32 && cargo nextest run --lib --locked --target x86_64-unknown-linux-gnu --no-tests=pass

win32-build:
	bash platforms/win32/build.sh i586
	bash platforms/win32/build.sh x86_64
	bash platforms/win32/check.sh i586
	bash platforms/win32/check.sh x86_64

repro:
	bash scripts/repro-win32.sh

smoke:
	bash scripts/smoke-wine.sh
