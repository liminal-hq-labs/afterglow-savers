# Local gate for Afterglow Screensavers. `make check` is the gate before every commit.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

export CARGO_BUILD_JOBS ?= 4

.PHONY: check check-fast headers fmt clippy test preview

# Each later phase adds its builds, size checks and reproducibility check to `check`.
check: check-fast

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
