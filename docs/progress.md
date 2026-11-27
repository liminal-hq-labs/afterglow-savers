# Progress

The resumable log of where the work stands. Newest state first; each phase lists what is done and what is next.

## Phases

- **P0** repository, research notes (`docs/reactos-scr.md`), storyboard (`docs/scenes.md`). Done.
- **P1** the engine core and the host preview tool. Done.
- **P2** the scenes: Ink, Twilight (with its sun, SAV-D27), Cracktro (with a vector-cube ring), Maze (with the portal loop, SAV-D31), the Collection, and Quill Ghostwriter. Done; previews, with and without sound, in `~/Documents/Liminal HQ/Afterglow Savers/previews`, a 4K Twilight too.
- **P3** the Win32 `.scr` shells: done (WP-win32, then the speed build SAV-D29, the Afterglow dialog SAV-D39/D40 with pointer hiding). The ReactOS guest ran all six (`reactos-*.png` in the previews); a re-run with the pointer fix and AC97 sound capture waits for WP-audio-win32.
- **P4** the ReactOS integration plan: done (`docs/reactos-integration.md`); the Quill installer plan too (`docs/quill-installer-integration.md`).
- **P5** Quill Ghostwriter: the Win32 scene and `.scr` done; the Windows 3.x `QUILLGW.SCR` done (SAV-D37: go for Windows 3.x, no-go for ReactOS's list).
- **P6** Twilight on the graphics card: renderer landed (`crates/savers-gpu`, parity 2.7 levels mean); WP-gpu-linux landed (SAV-D42 to D46, `docs/linux-edition.md`): `platforms/linux` runs Twilight fullscreen, in a window, in xscreensaver's window and headless, with settings in a TOML file, a render scale and frame cap, cpal sound, an installer under `~/.local`, and `make check-gpu` (Xvfb with lavapipe: window parity 2.4 levels mean, exit rules, hack, sound). lavapipe: 31 fps at 1080p and 5.2 fps at 4K (27 fps at 4K and 50% scale); the Radeon RX 7800 XT headless 1.0 ms a frame at 4K. Wayland, a real GPU window, PipeWire, GNOME and KDE are not exercised yet.
- **P7** the Afterglow settings dialog: shell landed; WP-dialog-v2-art landed (SAV-D49): each saver's banner moment, framing and seed, the Collection's four-panel banner, and the dusk-chip swatches, drawn by `savers-core` and rendered on the host by `savers-preview banner` and `swatches`; Wine screendumps of all six dialogs before and after (`dialog-v2-art-*.png`) and a contact sheet (`dialog-v2-art-sheet.png`) in the previews. WP-dialog-v2-check (keyboard walk, high contrast, a ReactOS guest) is next.
- **P8** audio: the engine and all six scores done (SAV-D34 to D36, D38); every preview video has sound. WP-audio-win32 done (SAV-D47, D48): every Win32 `.scr` plays its score through `waveOut`; checked under Wine and in the ReactOS guest. Twilight's cost on a Pentium III (an estimated 12% to 26%) is the open watch.
- **P9** threads for modern machines (SAV-D41, D51, D52, `docs/threads.md`): WP-threads done. The core shares the finishing chain and the five scenes between bands of rows with the same bytes at any band count; the x86-64 `.scr` files run a worker pool in full-screen runs of a million pixels or more; `savers-preview threads` times it. At 1080p 100% Ultra two threads give 48 to 96 fps for Cracktro, Ghostwriter, Maze and Twilight and four give Ink 39 fps (`docs/threads.md`, Results). Checked under Wine (`bandcheck.exe`); not yet in a ReactOS guest, on a real Windows machine or on a processor with fewer than eight cores.

- **P10** packing (SAV-D54 to D62, `docs/packing.md`): WP-PACK-1 done in its pull request: `scripts/build-scr.sh OUT_DIR` and `scripts/check-archive-build.sh`, and CI jobs for the Win32 builds (Pentium and x86-64), Win32 clippy, the image build contract (artifact `scr-i586` and the sums in the job summary), the Windows 3.x build and the reproducible build. The six i586 sums at this commit are in the pull request. No saver is selected in any image (SAV-D59); the compact image keeps the five. WP-PACK-2 (the ReactOS guest check with the dialog) is next here; WP-PACK-3 to WP-PACK-5 are in reactos-liminal, WP-PACK-6 and WP-PACK-7 in Quill (WP-PACK-7's Display Properties listing waits for liminal-installer's `{windows}` and `{system}`). Queued apart from the chain: WP-SCROLL (the Cracktro's words off by default), WP-LINUX-PKG (deb, rpm, PKGBUILD) and WP-GPU-MONITORS.

Open small items: the Maze's day look (it ignores Light), WP-art (icons and hero). `QUILLGW.SCR` has its icon (SAV-D50).

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

**Win32 sizes** at `opt-level = 3` (SAV-D29), in bytes, with sound (SAV-D47, D48), the x86-64 worker pool (SAV-D52) and the Afterglow dialog's art (SAV-D49, about 10 KiB a file):

| File | i586 | x86_64 |
| --- | --- | --- |
| `AGCOLLCT.SCR` (all five scenes and scores; its guard is 448 KiB, 458,752) | 400,896 | 441,856 |
| `AGMAZE.SCR` | 164,864 | 186,368 |
| `AGCRACK.SCR` | 162,816 | 191,488 |
| `AGTWILIT.SCR` | 160,768 | 178,688 |
| `AGINK.SCR` | 136,192 | 160,256 |
| `AGQUILL.SCR` | 171,008 | 204,800 |
| The five main files | 1,025,536 | 1,158,656 |

Against `opt-level = 2` on the host bench, `3` makes Ink at 100% Ultra about 25% faster (29.6 to 22.7 ms at 800 × 600) and leaves Twilight and Maze within noise.

## Open questions for the owner

Each has a default in place; the file or setting named swaps it.

- **Quill Ghostwriter from Quill's About box** (a click on the nib): Quill's to decide; nothing here depends on it.
- **The video audio level:** lifted to about -20 dBFS RMS for listening (`scripts/preview-audio.sh`, its `GAIN_DB`); the savers themselves play at `Volume` 28.
- **The Cracktro's music:** a gentle generated chiptune at the quiet default; `audio::score_for` can return none for silence.
- **Ghostwriter's key ticks:** very quiet; their gain is in `audio/ghostwriter.rs`.
- **Packing:** the owner's answers (no saver selected, the compact image keeps the five, ReactOS's own savers stay in the full image, Ghostwriter in Display Properties waits for the installer's `{windows}` and `{system}`) are in `docs/packing.md` (Owner decisions).
- **Linux packages, monitors and the scroller:** planned in `docs/linux-packages.md` (WP-LINUX-PKG, WP-GPU-MONITORS, WP-LINUX-DESKTOP, WP-SCROLL; SAV-D63 to SAV-D76), each default with its swap in that file's owner decisions.
- **Twilight's loop:** stays as it is (a designed fade through black at the 90-second wrap, not pixel-periodic), as the owner decided.
- **Not yet verified:** sound in the ReactOS guest (AC97 capture, with WP-audio-win32), the pointer-hiding fix in the guest, Windows 95 itself, real hardware, and a Pentium III's real frame rate (the estimates come from a reference loop).
