# Threads on modern machines

Landed (SAV-D51, SAV-D52): the design below, with the differences listed under "As built". The results are at the end.

The brief asks for at least 30 frames a second at full quality on a modern virtual machine. On one core of the development host at 1920 × 1080, 100% and Ultra, the savers measure (SAV-D41):

| Saver | ms a frame | fps |
| --- | --- | --- |
| Cracktro | 18.7 | 54 |
| Quill Ghostwriter | 29.1 | 34 |
| Maze | 36.0 | 28 |
| Twilight | 40.9 | 24 |
| Ink | 84.0 | 12 |

At 1280 × 720 and below every saver but Ink meets 30; at 1080p and up they need more than one core. The Pentium III path is single-threaded and stays so (ReactOS's guests have one CPU); the x86-64 build uses threads when the machine has more than one.

## Where the time goes

Every frame makes up to three passes over the full picture besides the scene: the scene's own drawing, the glow pass (a bright pass at a quarter of the size, the blurs, and a bilinear screen-blend over the frame), and the output pass (enlargement, vignette, fade, letterbox, grain). Each is a loop over rows in which a row depends only on its inputs, so each splits into bands of rows with no change to the result.

## The design

- **Core (`savers-core`, still `no_std`):** a `Bands` interface the platform implements: `fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync))`, which calls `job` for disjoint row ranges covering `0..rows`, in parallel or not. The default runs one range in the calling thread. `Finisher::glow` and `Finisher::output` take a `&dyn Bands` and split their per-row loops (the bright pass and the blurs at a quarter size stay serial; they are small). Scenes gain an optional `render_bands`: Ink (the advection and the colour pass are per pixel), Twilight (per row of the disc and the background), Maze (pass 1 is per column, pass 2 per row: split columns, then rows), Ghostwriter (the placement of the window on the desk) and Cracktro (the enlargement). Determinism holds: the bands write disjoint rows and read only shared, unchanging inputs, so the frame is the same byte for byte with any number of threads, which a test checks (one band against eight).
- **Win32 shell (x86-64 only):** a small pool of `GetSystemInfo().dwNumberOfProcessors - 1` worker threads, at most seven, made with `CreateThread`, each waiting on its own auto-reset event; a pass sets their ranges, signals them, works its own range, and waits on a counter that the last worker sets an event for (`WaitForSingleObject`). The pool's threads run at the process's idle priority, so a busy machine is not slowed. Imports for the 64-bit build only (`CreateThread`, `CreateEventA`, `SetEvent`, `WaitForSingleObject`, `InterlockedDecrement` is an intrinsic), tagged `64` in `imports.allow`. The i586 build keeps the serial default and no new imports.
- **Linux GPU edition:** not needed (the GPU does it).
- **Preview tool:** a `--threads N` option using `std::thread::scope`, so frame times can be measured with 1, 2, 4 and 8 bands.

Expected: with four cores, Ink at 1080p Ultra about 25 ms (40 fps), the others 10 to 15 ms.

## Work package

**WP-threads** (areas `core` and `win32`, Sonnet with an Opus review of the core change): the `Bands` interface and its use in the finishing chain and the five scenes; the determinism test; the x86-64 pool; `savers-preview --threads`; frame times at 1080p and 1440p with 1 to 8 threads recorded in `docs/progress.md`. Done.

## As built

- **The interface** is `savers_core::bands::Bands` with `run(rows, job)` as designed, and `Ctx::bands` (a field, in place of a `render_bands` method on each scene) hands it to the scenes. `Saver::set_bands` takes a `&'static dyn Bands`; the default is `SERIAL`, one range on the calling thread. `Shared` and `rows` (in the same module) are the only unsafe code in the core.
- **What is banded.** The finishing chain: the bright pass, the blurs (one pass at a time over all four planes, rows then columns, with the packing riding on the last column pass), the glow overlay and the output pass. Ink: the flow grid, the advection of both layers, the blend, the aurora (by column) and the relief. Twilight: the nebula, the sun and the moons, then the planet and the front moons, with the satellites (a few pixels, across band edges) drawn serially between. Maze: the column cast (by column), the floor (by row), the light shafts. Ghostwriter: the desk and the window placed on it, and the era cross-fade. Cracktro: the enlargement of the stage. The Collection: the cross-fade. Left serial because they are small: the scenes' own set-up, the sheet painting, the stage drawing of the Cracktro and the window drawing of Ghostwriter (at their own small sizes), the stars.
- **Order does not matter.** A pass that reads a neighbour another band writes works from a copy of the frame taken before it (Ink's aurora reads the pixel to the right, its relief the pixel up and to the left). The Maze's brightest window is found after the columns, in column order. The test `bands_do_not_change_a_byte` runs every scene with 1, 2, 3 and 8 bands, one after another last band first, and on real threads, against the serial frames at 203 × 117 over the scenes' changes of act, and the goldens are unchanged.
- **The pool** (`platforms/win32/src/pool.rs`) cuts a pass into four chunks for each thread and the caller and the workers take them from one counter, so a thread the scheduler is slow to wake takes fewer. If a pass has waited 12 ms for a worker, the workers go to normal priority until it ends (an idle-priority thread on a machine with every processor busy would otherwise hold the frame). Workers are one fewer than the processors, at most seven, so eight threads at most; only a full-screen run of a million pixels or more starts a pool (the passes at the preview's and the dialog's sizes are too short to repay waking a thread), and the Pentium build has no pool and no new import. A `Threads` registry value in the saver's key overrides the count (1 for none, 2 to 8 in all); it is not in the settings table or the dialog (SAV-D52).
- **Tests.** The host test above; `bandcheck.exe` (built beside `previewhost.exe`, not shipped) renders Ink, Twilight, Cracktro, Maze, Ghostwriter and the Collection with one thread and with eight through the real pool, and `scripts/smoke-wine.sh` runs it for x86-64 (exit 0 is identical frames).
- **Cost.** The x86-64 files grew by 25 to 30 KiB each and the Collection by 70 KiB (430,080 bytes of its 458,752); the Pentium files grew too, because the banded loops are the only loops now, and stay within their budgets (`platforms/win32/check.sh`).

## Results

`savers-preview threads --scene all --size WxH --frames 20` on the development host (AMD Ryzen 7 7700X, eight cores and sixteen threads, other work running at the same time, so a few per cent of noise): 100% Ultra, frames a second apart across each storyboard, whole frame (scene, glow and output) in milliseconds, and frames a second. One thread is the serial path.

| Saver | Size | 1 thread | 2 threads | 4 threads | 8 threads |
| --- | --- | --- | --- | --- | --- |
| Cracktro | 1920 × 1080 | 21.9 ms, 46 | 10.4, 96 | 5.4, 185 | 3.6, 277 |
| Quill Ghostwriter | 1920 × 1080 | 32.2, 31 | 16.7, 60 | 8.8, 113 | 5.2, 193 |
| Maze | 1920 × 1080 | 38.5, 26 | 20.2, 50 | 11.0, 91 | 6.6, 151 |
| Twilight | 1920 × 1080 | 39.9, 25 | 20.9, 48 | 10.9, 92 | 7.2, 139 |
| Ink | 1920 × 1080 | 86.8, 12 | 46.7, 21 | 25.7, 39 | 15.4, 65 |
| Cracktro | 2560 × 1440 | 34.4, 29 | 17.8, 56 | 9.4, 106 | 4.9, 204 |
| Quill Ghostwriter | 2560 × 1440 | 57.2, 17 | 29.8, 34 | 15.6, 64 | 8.5, 117 |
| Maze | 2560 × 1440 | 67.7, 15 | 35.6, 28 | 18.7, 54 | 9.7, 103 |
| Twilight | 2560 × 1440 | 71.3, 14 | 36.9, 27 | 19.6, 51 | 11.1, 90 |
| Ink | 2560 × 1440 | 153.1, 7 | 80.3, 12 | 43.3, 23 | 25.5, 39 |
| Cracktro | 3840 × 2160 | 77.5, 13 | 40.3, 25 | 21.1, 47 | 11.0, 91 |
| Quill Ghostwriter | 3840 × 2160 | 128.7, 8 | 67.1, 15 | 35.7, 28 | 20.3, 49 |
| Maze | 3840 × 2160 | 152.7, 7 | 80.7, 12 | 42.3, 24 | 23.0, 44 |
| Twilight | 3840 × 2160 | 159.5, 6 | 82.9, 12 | 43.3, 23 | 22.7, 44 |
| Ink | 3840 × 2160 | 339.7, 3 | 174.7, 6 | 92.7, 11 | 59.8, 17 |

At 1080p every saver but Ink passes 30 fps with two threads (48 to 96 fps), and Ink needs four (39 fps). At 1440p Cracktro and Ghostwriter pass with two, Maze and Twilight with four, and Ink with eight (39 fps). At 4K the four lighter savers give 23 to 47 fps with four threads (only Cracktro passes 30) and 44 to 91 with eight, and Ink gives 17. The scaling is 1.9 to 2.1 for two threads, 3.4 to 4 for four and 5.5 to 6.2 for eight (the host has eight cores; the serial rest is small). So the target of 30 fps at full quality on a modern virtual machine holds at 1080p with two processors for four savers and with four for Ink.
