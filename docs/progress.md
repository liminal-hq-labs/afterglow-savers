# Progress

The resumable log of where the work stands. Newest state first; each phase lists what is done and what is next.

## Phases

- **P0** repository, research notes (`docs/reactos-scr.md`), storyboard (`docs/scenes.md`). Done.
- **P1** the engine core and the host preview tool. Done.
- **P2** the scenes: Ink, Twilight (with its sun, SAV-D27), Cracktro, Maze, the Collection, and Quill Ghostwriter's scene. Done; previews in `~/Documents/Liminal HQ/Afterglow Savers/previews`, a 4K Twilight video too.
- **P3** the Win32 `.scr` shells: a Sonnet builder has WP-win32 in `~/source/worktrees/afterglow-savers/win32` (branch `wp/win32`). When it lands: `scripts/guest/guest.py --scr <dist>` runs every saver in a copy of the Afterglow ReactOS image and takes screendumps.
- **P4** the ReactOS integration plan: done (`docs/reactos-integration.md`); the Quill installer plan too (`docs/quill-installer-integration.md`).
- **P5** Quill Ghostwriter: the scene is done; its `.scr` comes with WP-win32; the Windows 3.x edition `QUILLGW.SCR` (SAV-D23) is WP-win16, queued for a builder.
- **P6** Twilight on the graphics card (SAV-D25, `docs/gpu-twilight.md`): three work packages, queued for builders.
- **P7** the Afterglow settings dialog (SAV-D26, `docs/settings-dialog.md`), after WP-win32: three work packages.

Builder queue (the orchestrator's order): WP-art, WP-gpu-share, WP-win16, then the dialog v2 packages.

## Measurements

`savers-preview bench` at 800 × 600, frames a second apart across each storyboard; the host's reference loop takes about 0.6 ms, and a Pentium III 800 MHz is estimated 11 to 12 times slower (`savers_core::bench`). Start-up is the scene's painting at the start.

| Saver | 25% Low: host ms, P3 est. fps | 50% Medium: host ms, P3 est. fps | 100% Ultra: host ms, host fps | Start ms (Low / Ultra) |
| --- | --- | --- | --- | --- |
| Ink | 3.7, 23 | 11.0, 7.7 | 24.7, 41 | 50 / 207 |
| Twilight | 2.9, 28 | 6.7, 12 | 14.0, 71 | 49 / 531 (maps grow with the width) |
| Cracktro | 2.3, 37 | 4.2, 20 | 4.6, 218 | 7 / 38 |
| Maze | 2.5, 34 | 4.9, 17 | 9.7, 104 | 9 / 34 |
| Quill Ghostwriter | 2.4, 37 | 5.0, 18 | 8.5, 117 | 1 / 11 |
| Collection | 3.8, 22 | 11.5, 7 | 24.3, 41 | 51 / 207 |

The 25% Low floor (15 fps on a Pentium III) holds with room for every saver. 100% Ultra on one host core: 41 to 218 fps at 800 × 600; at 1280 × 720 Ink drops to about 27 fps, so rendering bands on several threads in the x86_64 build remains a later improvement. The Twilight 4K video took 752 s for 2,701 frames (0.28 s a frame at 3840 × 2160, Ultra).

## Open questions for the owner

- Quill Ghostwriter could be started from Quill's About box (a click on the nib); that is Quill's to decide.
