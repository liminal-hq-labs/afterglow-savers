# Threads on modern machines

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
- **Win32 shell (x86-64 only):** a small pool of `GetSystemInfo().dwNumberOfProcessors - 1` worker threads, at most seven, made with `CreateThread`, each waiting on its own auto-reset event; a pass sets their ranges, signals them, works its own range, and waits on a counter that the last worker sets an event for (`WaitForSingleObject`). The pool's threads run at the process's idle priority, so a busy machine is not slowed. Imports for the 64-bit build only (`CreateThread`, `CreateEventA`, `SetEvent`, `WaitForSingleObject`, `InterlockedDecrement` is an intrinsic), tagged ` 64` in `imports.allow`. The i586 build keeps the serial default and no new imports.
- **Linux GPU edition:** not needed (the GPU does it).
- **Preview tool:** a `--threads N` option using `std::thread::scope`, so frame times can be measured with 1, 2, 4 and 8 bands.

Expected: with four cores, Ink at 1080p Ultra about 25 ms (40 fps), the others 10 to 15 ms.

## Work package

**WP-threads** (areas `core` and `win32`, Sonnet with an Opus review of the core change): the `Bands` interface and its use in the finishing chain and the five scenes; the determinism test; the x86-64 pool; `savers-preview --threads`; frame times at 1080p and 1440p with 1 to 8 threads recorded in `docs/progress.md`.
