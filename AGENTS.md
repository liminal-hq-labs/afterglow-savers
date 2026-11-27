# AGENTS.md

## Table of contents

- [Project status](#project-status)
- [Spelling and writing](#spelling-and-writing)
- [Markdown formatting](#markdown-formatting)
- [Authoring voice](#authoring-voice)
- [Commit messages](#commit-messages)
- [Git workflow](#git-workflow)
- [Areas and landing](#areas-and-landing)
- [Local tooling](#local-tooling)
- [Licence headers](#licence-headers)
- [Architecture rules](#architecture-rules)
- [Testing](#testing)
- [Documentation](#documentation)
- [Repository layout](#repository-layout)

## Project status

Afterglow Screensavers is a [Liminal HQ Labs](https://github.com/liminal-hq-labs) experiment: four screensavers and a collection for the Afterglow-themed ReactOS and any Windows that runs `.scr` files, drawn by a software renderer in `no_std` Rust. It is **experimental**, so settings, registry values and looks may change between any two commits. `SPEC.md` is the source of truth for behaviour, and `docs/decisions.md` records why. The [Labs charter](https://github.com/liminal-hq-labs/.github/blob/main/docs/labs-charter.md) covers what that status means.

## Spelling and writing

**REQUIREMENT:** Use **Canadian English** in UI strings, comments, commit messages and documentation: `colour`, `centre`, `behaviour`, `cancelled`, `licence` (noun) and `license` (verb), and `-ize` endings such as `customize`. Keep American spellings where an API requires them (`COLORREF`, `color`).

- Use a real em dash (`—`) in prose, never `--`. A command-line flag such as `--scene` is not prose.
- Sentence case in the UI and in headings. Title Case for window titles.
- Write tight. No padding, no restating the heading.

## Markdown formatting

**REQUIREMENT:** Do not hard-wrap markdown prose. Write each paragraph or bullet as one line. Commit message bodies are the one exception: hard-wrap them at roughly 72 characters. Code comments wrap normally.

## Authoring voice

**REQUIREMENT:** Ship the result, not how it was arrived at. Write comments, identifiers and docs as the author of the artifact, for the person who meets it later. No reviewer names or commit hashes in comments or docs. Commit messages are where the why and the how-it-was-checked go.

## Commit messages

**Format:** Conventional Commits (`feat`, `fix`, `docs`, `test`, `build`, `chore`, `refactor`, `perf`, `style`), with an optional scope (`core`, `ink`, `twilight`, `cracktro`, `maze`, `post`, `win32`, `preview`, `art`, `repo`).

- The body explains what and why. Use markdown with **bold labels** and no headings. Hard-wrap.
- Backtick every code-level reference. User-facing strings use quotes.
- End with a `Co-Authored-By:` trailer naming the model that wrote the change (Opus work signs as Opus, Sonnet as Sonnet).
- Write the message to a file with a single-quoted heredoc and commit with `git commit -F <file>`. Check the result with `git log -1 --pretty=fuller`.

## Git workflow

- `main` is the only long-lived branch. There is **no remote**: never add one, never push, tag or publish.
- **Never** run `git add -A` or `git commit -a`. Stage explicit paths.
- Planning files such as `PLAN.md` stay out of git. `docs/progress.md` is the resumable log of where the work stands.

## Areas and landing

[`docs/landing.md`](docs/landing.md) is canonical. In short: one builder per area at a time; work lands straight onto `main` with `make check` passing before every commit; two or more builders at once each use a worktree at `~/source/worktrees/afterglow-savers/<topic>` and land by `git rebase main` and `git merge --ff-only`.

## Local tooling

- **Editor settings:** `.editorconfig` is authoritative: tabs everywhere except YAML.
- **Rust** is formatted with rustfmt using hard tabs (`rustfmt.toml`). `cargo clippy -- -D warnings` must pass.
- **Toolchain:** Rust 1.98.0 (`rust-toolchain.toml`), `cargo nextest`. Set `CARGO_BUILD_JOBS=4`, and run one heavy tool (cargo, Wine, QEMU) at a time.
- **The gate:** `make check` before every commit; `make check-fast` is fmt, clippy, tests and headers.
- **Scratch:** small, short-lived files in `/tmp`; large ones (frames, videos, guest images) in `~/.cache/afterglow-savers/`. Previews worth keeping go to `~/Documents/Liminal HQ/Afterglow Savers/`.
- **Guests:** Wine and QEMU run on their own Xvfb or headless, with no host desktop capture or key injection. Kill only a PID you started; never `pkill` by name.

## Licence headers

Every source, script, YAML, TOML and SVG file starts with a one-line summary, a blank comment line and the copyright block:

```rust
// One-line summary of the file.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
```

Use `#` comments for shell, Python, TOML and Makefiles, and an XML comment for SVG. Markdown, JSON and lockfiles don't need one. `scripts/check-headers.sh` enforces it. The project is licensed `Apache-2.0 OR MIT`.

## Architecture rules

- **Core first.** `savers-core` is `no_std` with `alloc`, knows nothing about Windows, never panics on any setting value, and draws every scene into a 32-bit frame buffer. A platform supplies time, a seed, a settings store and a place to show the frame.
- **No C runtime, no GPU.** The Win32 shell is `no_std` and `no_main`, imports ANSI names from `platforms/win32/imports.allow`, and blits a DIB with GDI. Nothing uses OpenGL or Direct3D.
- **No libm.** `sin`, `cos`, `sqrt`, `exp` and noise are the core's own, deterministic and fast; inner loops are fixed point. The Pentium build uses x87 floats and no MMX or SSE.
- **Procedural art.** Textures, fonts and planets are generated at startup from a seed. No bitmap bigger than an icon is embedded.
- **Vector masters.** Every icon and the hero are SVG masters in `art/`, rendered down by script; generated files are never edited by hand.
- **Budgets.** Built for speed; each `.scr` at most 384 KiB (the Collection 448 KiB, SAV-D47) and all five at most 1.5 MiB as a guard against accidents (SAV-D29), checked by `make check`. Builds are byte-identical in any directory.
- **SPEC first.** A change to behaviour, settings or defaults updates `SPEC.md` in the same change.

## Testing

- `make check` — the gate; `make check-fast` — fmt, clippy, tests, headers
- `cargo run -p savers-preview --release -- --help` — render any scene to PNG frames, a contact sheet, a GIF or an MP4, and time the quality ladder
- `make check-gpu` — the Linux edition on its own Xvfb with lavapipe (needs Xvfb, `import`, `xdotool`); never against the desktop session, and `WAYLAND_DISPLAY` is cleared
- `bash scripts/smoke-wine.sh` — the `.scr` files under Wine on their own Xvfb
- `docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2:v0.17.2 "**/*.md"` — markdown

State exactly what was and was not run when reporting results. [`docs/testing.md`](docs/testing.md) has the details.

## Documentation

- Update `SPEC.md` when behaviour changes, and add or amend a `docs/decisions.md` entry (`SAV-D<n>`) when a decision is made.
- `docs/scenes.md` is the storyboard; `docs/art.md` is the look; `docs/reactos-scr.md` is how ReactOS runs a screensaver.
- `CLAUDE.md` is a summary of this file. Keep them in sync.

## Repository layout

```text
README.md SPEC.md AGENTS.md CLAUDE.md LICENSE-APACHE LICENSE-MIT Makefile Cargo.toml
crates/        savers-core (the renderer, scenes and settings), savers-gpu (the wgpu Twilight) and savers-preview (host-only previews, never shipped)
platforms/     win32: the .scr shells for i586 and x86_64, their own workspace; linux: the fullscreen GPU saver, xscreensaver hack and installer (docs/linux-edition.md)
art/           SVG masters for the icons and the scripts that render them
assets/        hero banner
docs/          decisions, scenes, art, testing, ReactOS notes, landing, progress
scripts/       header check, sizes, reproducibility, Wine smoke, guest tools
```
