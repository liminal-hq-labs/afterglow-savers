# ReactOS integration plan

How reactos-liminal would build the Afterglow savers and install them in its images. This repository does not edit reactos-liminal: this is the plan for whoever does, read against reactos-liminal `origin/main` (`recipes/afterglow.toml`, `recipes/afterglow-compact.toml`, `reactos_liminal/rustbuild.py`, `reactos_liminal/recipe.py`, `guest/reg/quiet.reg`, `profiles/base.txt`, `docs/recipes.md`).

## What goes where

| File | Listed as | Afterglow full | Afterglow compact | Quill remix |
| --- | --- | --- | --- | --- |
| `AGMAZE.SCR` | Afterglow Maze | yes | yes (SAV-D55) | inherited |
| `AGCRACK.SCR` | Afterglow Cracktro | yes | yes | inherited |
| `AGTWILIT.SCR` | Afterglow Twilight | yes | yes | inherited |
| `AGINK.SCR` | Afterglow Ink | yes | yes | inherited |
| `AGCOLLCT.SCR` | Afterglow Collection | yes, the selected saver | yes, the selected saver | inherited, not selected (SAV-D57) |
| `AGQUILL.SCR` | Quill Ghostwriter | no | no | **only here**, never selected (SAV-D22) |

All go to `ReactOS/system32`, where the Display applet's list looks (docs/reactos-scr.md). The i586 builds are the ones for the image; the x86_64 builds are for 64-bit Windows and are not used.

## Building them

reactos-liminal builds the savers from this repository at a pinned commit (SAV-D54): `[builds.afterglow-savers]` names `repo`, `commit` and the SHA-256 of the source tar GitHub serves for it, the build script, and an `outputs` table of file name to SHA-256; `[[files]]` takes each file as `built:afterglow-savers/AGMAZE.SCR`. reactos-liminal's `[builds]` does not do this yet (its builds are in-tree projects with one `NAME.exe`); WP-PACK-3 in [packing.md](packing.md) adds it. The work packages, in order, are in that file.

**The contract on this side.** `scripts/build-scr.sh OUT_DIR` builds the six i586 `.scr` files from an extracted source tar (no `.git`; `platforms/win32/crates` is a relative symlink the tar keeps) and writes them and `SHA256SUMS` to `OUT_DIR`. It needs what afterfetch needs: rustup with the pinned 1.98.0, `rust-src` and `llvm-tools` (for `rust-lld`); `RUSTC_BOOTSTRAP=1` is set by the build. The output is byte-identical in any directory (`scripts/repro-win32.sh`, and `scripts/check-archive-build.sh` against a `git archive`), so pinning the sums works, and CI prints the sums for every commit. `AGQUILL.SCR` is built with the others; only Quill's recipe writes it.

## Recipe entries (Afterglow full, inherited by compact)

```toml
[builds.afterglow-savers]
repo = "liminal-hq-labs/afterglow-savers"
commit = "<40 digits>"
tar_sha256 = "<sha256>"
script = "scripts/build-scr.sh"
[builds.afterglow-savers.outputs]
"AGMAZE.SCR" = "<sha256>"
"AGCRACK.SCR" = "<sha256>"
"AGTWILIT.SCR" = "<sha256>"
"AGINK.SCR" = "<sha256>"
"AGCOLLCT.SCR" = "<sha256>"

[[files]]
source = "built:afterglow-savers/AGMAZE.SCR"
destination = "ReactOS/system32/AGMAZE.SCR"
# ... one entry per saver, the same shape, AGQUILL.SCR excluded
```

## Selecting the screensaver

`guest/reg/quiet.reg` turns the screensaver off (`ScreenSaveActive` `"0"`, `ScreenSaveTimeOut` `"0"` under `HKEY_CURRENT_USER\Control Panel\Desktop`). A new named registry file, `screensaver-afterglow`, listed **after** `quiet` in `settings.reg_files` (reg files apply in order), selects the Collection:

```text
Windows Registry Editor Version 5.00

; The Afterglow Collection after ten minutes, no password. Applied once by apply-settings.cmd, after quiet.reg, which it overrides.
;
; (c) Copyright 2026 Liminal HQ, Scott Morris
; SPDX-License-Identifier: Apache-2.0 OR MIT

[HKEY_CURRENT_USER\Control Panel\Desktop]
"SCRNSAVE.EXE"="C:\\ReactOS\\system32\\AGCOLLCT.SCR"
"ScreenSaveActive"="1"
"ScreenSaveTimeOut"="600"
"ScreenSaverIsSecure"="0"
```

These are exactly the values the Display applet writes (docs/reactos-scr.md, "Apply"); Winlogon reads `SCRNSAVE.EXE` and runs it with `/s` at idle priority. The savers' own settings need no registry entries (SAV-D56): every value has a default in the program, and the defaults (`Sound` on at `Volume` 28, the Dusk palette, 50% render scale, Medium detail) suit the image's QEMU guest. The image is 1024 × 768 at 32 bits, so `/c` shows the Afterglow dialog, not the plain fallback. The display power-off values in `quiet.reg` stay as they are.

## The compact recipe

The `base` trim profile removes ReactOS's own eight `.scr` files (`profiles/base.txt`), and the compact recipe inherits the base recipe's `[[files]]`, so it would get the Afterglow savers unless it removes them. The five main files are 1,025,536 bytes together at i586 (`docs/progress.md`), about 0.45 MB in the zstd parts, and the compact image keeps them, where they are its only screensavers (SAV-D55). If the compact image should have none, add a trim extra to `afterglow-compact.toml` with `f ReactOS/system32/AG*.SCR` lines (one per file; the trim takes no wildcards) and drop `screensaver-afterglow` from its `reg_files`.

## Quill's recipe

Quill's remix recipe (`site/try-reactos/qagrr/qagrr.toml` in the Quill repository) extends the compact recipe and so inherits the five Afterglow savers. Its `reg_files` list replaces the base recipe's named files, so it does not inherit `screensaver-afterglow`, and `quiet.reg` keeps the screensaver off in the web demo (SAV-D57). It adds Ghostwriter with one more `[[files]]` entry, `built:afterglow-savers/AGQUILL.SCR` to `ReactOS/system32/AGQUILL.SCR`, and its own `[builds.afterglow-savers.outputs]` line for that file's sum (tables merge key by key); nothing in its registry selects it: it shows in the Screen Saver list as "Quill Ghostwriter" for people to find (SAV-D22). The Windows 3.x edition (`QUILLGW.SCR`, SAV-D23) is not placed in `system32` unless the guest shows that the Display applet can start a 16-bit `.SCR`.

## Checks for the integration

- `rl image` with the savers: the five files in `system32` with their pinned sums.
- The guest: Display Properties > Screen Saver lists "Afterglow Collection", "Afterglow Cracktro", "Afterglow Ink", "Afterglow Maze" and "Afterglow Twilight" by their string 1; the preview runs in the monitor; Settings opens each dialog; Preview runs full screen and a key ends it; after the timeout the Collection starts on its own.
- The compact image: the same, or none, by the owner's choice below.

## Open questions

The owner decisions, each with its default, are in [packing.md](packing.md#owner-decisions).
