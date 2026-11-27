# CLAUDE.md

A summary of [AGENTS.md](AGENTS.md), which is canonical. Keep the two in sync.

## What this is

Afterglow Screensavers is a Liminal HQ Labs experiment: Afterglow Maze, Cracktro, Twilight and Ink, plus Afterglow Collection, as `.scr` files for ReactOS and Windows, drawn by a software renderer in `no_std` Rust. Behaviour lives in `SPEC.md`, decisions in `docs/decisions.md`, the storyboard in `docs/scenes.md`, where the work stands in `docs/progress.md`.

## Rules to remember

- **Canadian English.** Real em dashes. Sentence case. No hard-wrapped markdown prose. Ship the result, not the journey.
- **Conventional Commits.** Body in markdown with bold labels, hard-wrapped, written to a file and committed with `git commit -F`. End with a `Co-Authored-By` trailer naming your own model.
- **Git.** Local only, no remote: never push. Land straight onto `main` after `make check`; parallel builders use worktrees under `~/source/worktrees/afterglow-savers/<topic>` and fast-forward. Never `git add -A` or `git commit -a`.
- **Formatting.** Tabs everywhere except YAML. rustfmt with hard tabs.
- **Headers.** SPDX header on every source, script, TOML and SVG file: summary line, blank, "(c) Copyright 2026 Liminal HQ, Scott Morris", `SPDX-License-Identifier: Apache-2.0 OR MIT`.
- **Architecture.** `savers-core` is `no_std` + `alloc`, no libm, fixed point in inner loops, procedural textures, deterministic for a seed and a time. The Win32 shell has no C runtime, ANSI imports on `imports.allow`, x87 floats and no MMX or SSE on i586. Built for speed; a guard of 384 KiB per `.scr`, 1.5 MiB for all five (SAV-D29). Byte-identical builds in any directory.
- **Machines.** `CARGO_BUILD_JOBS=4`, one heavy tool at a time, large scratch in `~/.cache/afterglow-savers`, Wine and QEMU headless on their own displays, kill only your own PIDs.

## Commands

```sh
make check                       # the gate: fmt, clippy, tests, headers, Win32 builds and checks, sizes, repro
make check-fast                  # fmt, clippy, tests, headers
make check-gpu                   # the Linux edition on its own Xvfb with lavapipe (needs Xvfb, import, xdotool)
cargo run -p savers-preview --release -- --help
bash scripts/smoke-wine.sh
```
