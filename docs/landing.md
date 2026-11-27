# How work lands

Afterglow Screensavers has no remote and no pull requests, so work lands locally (SAV-D13).

- `main` is the only long-lived branch. Each work package is a short series of small Conventional Commits (`git commit -F <file>`, markdown body with bold labels, hard-wrapped, `Co-Authored-By` naming the agent's own model). Never `git add -A` or `git commit -a`; files are added by name.
- **One builder per area at a time.** Areas: `core` (`crates/savers-core` outside `scenes/`), one area per scene (`ink`, `twilight`, `cracktro`, `maze`: `crates/savers-core/src/scenes/<scene>`), `win32` (`platforms/win32`), `art` (`art/`, `assets/`), and `repo` (root files, Makefile, scripts, docs, `crates/savers-preview`).
- **When only one builder is active**, it works in the main checkout `~/source/liminal-hq-labs/afterglow-savers` and commits straight onto `main`. **When two or more run at once**, each works in its own worktree `~/source/worktrees/afterglow-savers/<topic>` on a local branch `wp/<topic>`, with build output in that worktree's `target/`.
- **The gate.** `make check` passes in the builder's tree before every commit. The series lands with `git rebase main` and `git -C ~/source/liminal-hq-labs/afterglow-savers merge --ff-only wp/<topic>`, then `git worktree remove`, `git worktree prune` and `git branch -d wp/<topic>`. `make check` is re-run on `main` after each landing.

## Limits

- At most three builders and two designers at once. Heavy tools (cargo, Wine, QEMU) run one at a time per agent, with `CARGO_BUILD_JOBS=4`; check `free -g` before anything large.
- Large files (frame dumps, videos, guest disk copies) live in `~/.cache/afterglow-savers/`. Check `df -h /home` before copying a guest image, and delete copies when done.
- No host desktop capture or key injection: Wine runs on its own Xvfb, QEMU headless with `screendump` through its monitor. Never kill processes by name.
