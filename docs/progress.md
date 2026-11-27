# Progress

The resumable log of where the work stands. Newest state first; each phase lists what is done and what is next.

## Phases

- **P0** repository, research notes (`docs/reactos-scr.md`), storyboard (`docs/scenes.md`). Done.
- **P1** the engine core: maths, noise, colour and palettes, frame buffer, finishing (bloom, streaks, vignette, grain, letterbox, fades, scaling), settings, the host preview tool and tests.
- **P2** the scenes, in order of risk: Ink, Twilight, Cracktro, Maze, each with its quality ladder measured, preview GIFs and a contact sheet.
- **P3** the Win32 `.scr` shells (`/s /p /c`, registry, dialog, exit rules, Collection), reproducible i586 and x86_64 builds, icons, Wine, then the real ReactOS guest.
- **P4** the ReactOS integration plan (`docs/reactos-integration.md`).
- **P6** Twilight on the graphics card (SAV-D25, `docs/gpu-twilight.md`): three work packages, for builders.
- **P5** Quill Ghostwriter (Win32, SAV-D20 to SAV-D22), then its Windows 3.x edition `QUILLGW.SCR` (SAV-D23), and `docs/quill-installer-integration.md` (SAV-D24).

## Measurements

**Ink** (`savers-preview bench`, host reference loop 0.66 ms, a Pentium III estimated 11.3 times slower):

| Window | Rung | Host ms | Host fps | P3 est. fps |
| --- | --- | --- | --- | --- |
| 800 × 600 | 25% Low | 3.8 | 263 | 23 |
| 800 × 600 | 100% Ultra | 24.5 | 41 | 3.6 |
| 1280 × 720 | 25% Low | 3.3 | 304 | 25 |
| 1280 × 720 | 100% Ultra | 36 to 39 | 26 to 28 | 2.3 |

The 25% Low floor holds. 100% Ultra misses 30 fps at 1280 × 720 on one host core; the output pass (enlargement, vignette and grain at the full size) and the two-layer advection dominate. Rendering bands on several threads in the x86_64 build is the planned fix.

**Twilight** and **Ink** at 800 × 600, frames sampled a second apart across the storyboard (host reference loop about 0.62 ms; a Pentium III estimated about 12 times slower):

| Scene | Rung | Start ms | Host ms | Host fps | P3 est. fps |
| --- | --- | --- | --- | --- | --- |
| Twilight | 25% Low | 51 | 3.2 | 313 | 26 |
| Twilight | 100% Ultra | 212 | 15.3 | 65 | 5 |
| Ink | 25% Low | 53 | 3.8 | 266 | 22 |
| Ink | 100% Ultra | 203 | 21.7 | 46 | 4 |

Ink paints its second sheet a few rows a frame while it is invisible, which halves its start-up.

## Open questions for the owner

- Quill Ghostwriter could be started from Quill's About box (a click on the nib); that is Quill's to decide.
