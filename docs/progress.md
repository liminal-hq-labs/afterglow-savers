# Progress

The resumable log of where the work stands. Newest state first; each phase lists what is done and what is next.

## Phases

- **P0** repository, research notes (`docs/reactos-scr.md`), storyboard (`docs/scenes.md`). Done.
- **P1** the engine core and the host preview tool. Done.
- **P2** the scenes: Ink, Twilight (with its sun, SAV-D27), Cracktro, Maze, the Collection, and Quill Ghostwriter's scene. Done; previews in `~/Documents/Liminal HQ/Afterglow Savers/previews`, a 4K Twilight video too.
- **P3** the Win32 `.scr` shells (`platforms/win32`): `/s`, `/p`, `/c`, registry, dialog, resources, six programs for i586 (x87, no MMX or SSE) and x86-64, checks, byte-identical builds and the Wine run. Done. Sizes (`-Os`, i586): AGMAZE 79 KiB, AGCRACK 79 KiB, AGTWILIT 81 KiB, AGINK 67 KiB, AGCOLLCT 167 KiB (without Ghostwriter), together 473 KiB of 640; AGQUILL 86 KiB. Next: `scripts/guest/guest.py --scr platforms/win32/dist/<arch>` in the ReactOS image.
- **P4** the ReactOS integration plan: done (`docs/reactos-integration.md`); the Quill installer plan too (`docs/quill-installer-integration.md`).
- **P5** Quill Ghostwriter: the scene and `AGQUILL.SCR` are done; the Windows 3.x edition `QUILLGW.SCR` (SAV-D23) is WP-win16, queued for a builder.
- **P6** Twilight on the graphics card (SAV-D25, `docs/gpu-twilight.md`): WP-gpu-share and WP-gpu-render are done (`crates/savers-gpu`, SAV-D32 and SAV-D33: a headless renderer held to the CPU frames, `savers-gpu-preview` for frames, comparisons and timings); WP-gpu-linux (the fullscreen binary and the xscreensaver hack) is next.
- **P7** the Afterglow settings dialog (SAV-D26, `docs/settings-dialog.md`), after WP-win32: three work packages. The shell package is done (SAV-D39, SAV-D40): banner, swatches, live preview, fallback, Wine checks; placeholder banner moments await the art package, and the check package is next.

Builder queue (the orchestrator's order): WP-art, WP-gpu-share, WP-win16, then the dialog v2 packages.

The Win32 shell is built with `opt-level = "s"`: at `2` the Collection is 204 KiB and over its budget. The Collection's own maker leaves Ghostwriter out (a Collection asked for it plays Ink), which `AGQUILL.SCR` carries instead.

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

**Win32 sizes** at `opt-level = 3` (SAV-D29), in bytes:

| File | i586 | x86_64 |
| --- | --- | --- |
| `AGCOLLCT.SCR` (all five scenes) | 295,424 | 328,192 |
| `AGMAZE.SCR` | 114,688 | 129,536 |
| `AGCRACK.SCR` | 108,544 | 125,440 |
| `AGTWILIT.SCR` | 110,080 | 121,344 |
| `AGINK.SCR` | 90,624 | 106,496 |
| `AGQUILL.SCR` | 129,536 | 155,136 |
| The five main files | 719,360 | 811,008 |

Against `opt-level = 2` on the host bench, `3` makes Ink at 100% Ultra about 25% faster (29.6 to 22.7 ms at 800 × 600) and leaves Twilight and Maze within noise.

## Open questions for the owner

- Quill Ghostwriter could be started from Quill's About box (a click on the nib); that is Quill's to decide.
