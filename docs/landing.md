# How work lands

Afterglow Screensavers lives at `liminal-hq-labs/afterglow-savers` and work lands by pull request (SAV-D53). Everything below about commits, areas, worktrees and the gate holds; the last step is a pull request from the topic branch, merged with a merge commit.

- `main` is the only long-lived branch and is never committed to directly. Each work package is a short series of small Conventional Commits (`git commit -F <file>`, markdown body with bold labels, hard-wrapped, `Co-Authored-By` naming the agent's own model). Never `git add -A` or `git commit -a`; files are added by name.
- **One builder per area at a time.** Areas: `core` (`crates/savers-core` outside `scenes/`), one area per scene (`ink`, `twilight`, `cracktro`, `maze`: `crates/savers-core/src/scenes/<scene>`), `win32` (`platforms/win32`), `art` (`art/`, `assets/`), and `repo` (root files, Makefile, scripts, docs, `crates/savers-preview`).
- Each builder works in its own worktree `~/source/worktrees/afterglow-savers/<topic>` on a topic branch (`feat/`, `fix/`, `docs/`, `chore/`, `ci/`, `build/`), with build output in that worktree's `target/`.
- **The gate.** `make check` passes in the builder's tree before every commit. The branch is kept current with `git rebase main` and lands as a pull request with a merge commit; the branch is deleted on merge, then `git worktree remove`, `git worktree prune` and `git branch -d <branch>` clean up. `make check` is re-run on `main` after each landing.

## Limits

- At most three builders and two designers at once. Heavy tools (cargo, Wine, QEMU) run one at a time per agent, with `CARGO_BUILD_JOBS=4`; check `free -g` before anything large.
- Large files (frame dumps, videos, guest disk copies) live in `~/.cache/afterglow-savers/`. Check `df -h /home` before copying a guest image, and delete copies when done.
- No host desktop capture or key injection: Wine runs on its own Xvfb, QEMU headless with `screendump` through its monitor. Never kill processes by name.
