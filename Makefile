# Local gate for Afterglow Screensavers. `make check` is the gate before every commit.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

export CARGO_BUILD_JOBS ?= 4

.PHONY: check check-fast check-gpu headers fmt clippy test deny preview win16 win32 win32-fmt win32-clippy win32-test win32-build build-scr check-archive repro smoke

# `check` is the whole gate: the host checks, the Win32 build and its structure, import, Pentium and size checks, the byte-identical
# rebuild in three directories, and the Wine run (which says SKIP when Wine or Xvfb is missing).
check: check-fast deny win32 win16 check-archive repro smoke

check-fast: headers fmt clippy test

headers:
	bash scripts/check-headers.sh

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --locked -- -D warnings

test:
	cargo nextest run --workspace --locked --no-tests=pass

# Licences, bans and sources of every dependency (`savers-gpu` brings wgpu); the advisory database needs the network, so it is left out.
deny:
	cargo deny --locked check --hide-inclusion-graph licenses bans sources

preview:
	cargo build --release --locked -p savers-preview

# The Linux edition on its own Xvfb with the lavapipe software Vulkan driver (needs Xvfb, ImageMagick's `import`, xdotool and
# `vulkan-swrast`; a test says SKIP when one is missing). Sound goes to a file through ALSA's file plug-in, and nothing touches the
# desktop session: the tests clear WAYLAND_DISPLAY and give every command their own DISPLAY. Left out of `check` because it needs a
# display server and takes about half a minute.
check-gpu:
	env -u WAYLAND_DISPLAY cargo nextest run --release --locked -p afterglow-twilight-gpu --run-ignored only --test-threads 1 --no-tests=pass

# The Windows 3.x edition is a workspace of its own too: format, lints, the packer's tests, the build, and the CPU, branch and size checks.
win16:
	bash platforms/win16/check.sh

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

# The six i586 files and SHA256SUMS in `dist/scr-i586`, as reactos-liminal and Quill build them (`scripts/build-scr.sh`).
build-scr:
	bash scripts/build-scr.sh dist/scr-i586

# A `git archive` of HEAD, built like the GitHub source tar, must match the working tree's files.
check-archive:
	bash scripts/check-archive-build.sh

repro:
	bash scripts/repro-win32.sh

smoke:
	bash scripts/smoke-wine.sh
