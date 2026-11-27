# ReactOS integration plan

How reactos-liminal would build the Afterglow savers and install them in its images. This repository does not edit reactos-liminal: this is the plan for whoever does, read against reactos-liminal `origin/main` (`recipes/afterglow.toml`, `recipes/afterglow-compact.toml`, `reactos_liminal/rustbuild.py`, `reactos_liminal/recipe.py`, `guest/reg/quiet.reg`, `profiles/base.txt`, `docs/recipes.md`).

## What goes where

| File | Listed as | Afterglow full | Afterglow compact | Quill remix |
| --- | --- | --- | --- | --- |
| `AGMAZE.SCR` | Afterglow Maze | yes | yes (see the open question) | inherited |
| `AGCRACK.SCR` | Afterglow Cracktro | yes | yes | inherited |
| `AGTWILIT.SCR` | Afterglow Twilight | yes | yes | inherited |
| `AGINK.SCR` | Afterglow Ink | yes | yes | inherited |
| `AGCOLLCT.SCR` | Afterglow Collection | yes, the selected saver | yes, the selected saver | inherited |
| `AGQUILL.SCR` | Quill Ghostwriter | no | no | **only here**, never selected (SAV-D22) |

All go to `ReactOS/system32`, where the Display applet's list looks (docs/reactos-scr.md). The i586 builds are the ones for the image; the x86_64 builds are for 64-bit Windows and are not used.

## Building them: today's `[builds]` and what it needs

reactos-liminal's `[builds.NAME]` runs `build.sh OUT_DIR` in a project folder (relative to the recipe), expects exactly `OUT_DIR/NAME.exe`, pins its SHA-256, and the recipe's schema names only `afterfetch`. Five (or six) files from one project do not fit that, so one of these is needed; the first is recommended:

1. **Several outputs from one build (recommended).** Extend the schema: `[builds.afterglow-savers]` with `source`, an `outputs` table of file name to SHA-256, and `[[files]]` sources of the form `built:afterglow-savers/AGMAZE.SCR`. `build_program` runs `build.sh OUT_DIR` once and checks each listed output's sum. `rl lock` measures every output. The contract on this side is a `build.sh` at the project root that takes `OUT_DIR`, builds the i586 `.scr` files and writes them there, over the Win32 shells' own build script.
2. **One build per file.** Keep the schema shape but allow any name, and have the project's `build.sh` take the name (`AGMAZE`) from its output folder and write `NAME.exe`, which `[[files]]` writes as `AGMAZE.SCR`. Simple, but each build gets its own `CARGO_TARGET_DIR`, so the workspace is compiled six times.

**The source.** reactos-liminal builds projects that live in its own tree (`tools/afterfetch`). Until this repository has a remote and releases, the plan is a vendored copy at `tools/afterglow-savers` (the whole repository minus `target/`, with a `PROVENANCE.md` naming this repository's commit), refreshed by a script; later, an `[inputs.afterglow_savers]` release archive pinned by SHA-256, like the Afterglow input. The build needs the same tools as afterfetch: rustup with the pinned 1.98.0, `rust-src` and `rust-lld`, and `RUSTC_BOOTSTRAP=1` (set by `build.sh`); `rl doctor` should name them for the savers too. The output is byte-identical in any directory (this repository's `scripts/repro-win32.sh`), so pinning the sums works.

## Recipe entries (Afterglow full, inherited by compact)

```toml
[builds.afterglow-savers]
source = "../tools/afterglow-savers"
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

; Afterglow Collection after ten minutes, no password. Applied once by apply-settings.cmd, after quiet.reg.

[HKEY_CURRENT_USER\Control Panel\Desktop]
"SCRNSAVE.EXE"="C:\\ReactOS\\system32\\AGCOLLCT.SCR"
"ScreenSaveActive"="1"
"ScreenSaveTimeOut"="600"
"ScreenSaverIsSecure"="0"
```

These are exactly the values the Display applet writes (docs/reactos-scr.md, "Apply"); Winlogon reads `SCRNSAVE.EXE` and runs it with `/s` at idle priority. The savers' own settings need no registry entries: every value has a default in the program, and the defaults (50% render scale, Medium detail) suit the image's QEMU guest. The display power-off values in `quiet.reg` stay as they are.

## The compact recipe

The `base` trim profile removes ReactOS's own eight `.scr` files (`profiles/base.txt`), and the compact recipe inherits the base recipe's `[[files]]`, so it would get the Afterglow savers unless it removes them. The five main files are about 720 KB together at i586 (SAV-D29), so the recommendation is to keep them in the compact image, where they are its only screensavers. If the compact image should have none, add a trim extra to `afterglow-compact.toml` with `f ReactOS/system32/AG*.SCR` lines (one per file; the trim takes no wildcards) and drop `screensaver-afterglow` from its `reg_files`.

## Quill's recipe

Quill's remix recipe (`site/try-reactos/qagrr/qagrr.toml` in the Quill repository) extends the Afterglow recipe and so inherits the five Afterglow savers and the Collection as the selected one. It adds Ghostwriter with one more `[[files]]` entry, `built:afterglow-savers/AGQUILL.SCR` to `ReactOS/system32/AGQUILL.SCR` (the build's `outputs` gains its sum), and nothing in its registry selects it: it shows in the Screen Saver list as "Quill Ghostwriter" for people to find (SAV-D22). The Windows 3.x edition (`QUILLGW.SCR`, SAV-D23) is not placed in `system32` unless the guest shows that the Display applet can start a 16-bit `.SCR`.

## Checks for the integration

- `rl image` with the savers: the five files in `system32` with their pinned sums.
- The guest: Display Properties > Screen Saver lists "Afterglow Collection", "Afterglow Cracktro", "Afterglow Ink", "Afterglow Maze" and "Afterglow Twilight" by their string 1; the preview runs in the monitor; Settings opens each dialog; Preview runs full screen and a key ends it; after the timeout the Collection starts on its own.
- The compact image: the same, or none, by the owner's choice below.

## Open questions

- Should the compact image keep the Afterglow savers (recommended: yes, they are its only ones and are small)?
- Is the Collection the right default saver, and ten minutes the right timeout?
