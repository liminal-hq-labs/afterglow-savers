# Testing

## The gate

`make check` runs before every commit:

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` on the host workspace and the Win32 workspace
- `cargo nextest run --workspace` on the host: the core's maths, noise, colour and post-processing tests, the settings parser, and the golden frames
- `scripts/check-headers.sh`
- the Win32 builds for i586 and x86_64, their structure, import and Pentium checks, and the size budgets (`scripts/check-sizes.sh`)
- `scripts/repro-win32.sh`: three builds in three directories, compared byte for byte

- `scripts/check-archive-build.sh` (`make check-archive`): a `git archive` of HEAD, extracted and built with `scripts/build-scr.sh` in a folder of its own, must give the same six SHA-256 sums as the working tree. `make build-scr` writes the files and `SHA256SUMS` to `target/scr-out`.

`make check-fast` is the first four. CI runs one job for each of these: the Win32 builds for i586 and x86-64 with their checks, the Win32 clippy and tests, the image build contract (which uploads `scr-i586` and prints the sums in the job summary), the Windows 3.x build (which uploads `QUILLGW.SCR`) and the reproducible build. The Wine smoke (`scripts/smoke-wine.sh`) and the ReactOS guest run (`scripts/guest/`) are separate, because they take minutes and need Xvfb, Wine or QEMU.

`make check-gpu` is separate too: the Linux edition (`platforms/linux`) on its own Xvfb with the lavapipe software Vulkan driver, five tests for the window, fullscreen, the exit rules, the xscreensaver hack and the sound (captured through ALSA's file plug-in). It needs Xvfb, ImageMagick's `import`, `xdotool` and `vulkan-swrast`, and says SKIP when one is missing. See `docs/linux-edition.md`.

## Golden frames

Each scene has golden frame checksums: a small frame (160 × 90) at fixed times, seeds and detail levels, hashed with FNV-1a. A change that alters a scene's look changes its hash; update them with `SAVERS_UPDATE_GOLDEN=1 cargo nextest run` after looking at the new frames with the preview tool, and say so in the commit. The goldens are for the host build only (SAV-D6).

## The preview tool

`crates/savers-preview` is a host program that renders any scene without Windows:

```sh
cargo run -p savers-preview --release -- frames --scene ink --size 640x360 --scale 50 --detail high --from 0 --to 20 --step 0.5 --out ~/.cache/afterglow-savers/frames/ink
cargo run -p savers-preview --release -- sheet --scene twilight --size 640x360 --times 0,10,20,30,40,50 --out sheet.png
cargo run -p savers-preview --release -- bench --scene maze --size 800x600     # every rung of the ladder
bash scripts/preview-video.sh ink                                              # frames to GIF and MP4 with ffmpeg
cargo run -p savers-preview --release -- banner --scene maze --out banner.png  # the settings dialog's banner, as the dialog draws it
cargo run -p savers-preview --release -- swatches --scene ink --out swatches     # every palette swatch, night and day, plain and chosen
bash art/dialog-sheet.sh                                                       # a contact sheet of the six banners and the swatches
```

## Frame times and the Pentium III estimate

`bench` times every render scale and detail level on the host, in milliseconds per frame, and estimates a Pentium III from a reference loop: a fixed mix of integer, fixed-point, float and memory work (`savers_core::bench::reference`) that is timed on the host and whose time on a Pentium III 800 MHz is taken as known (see the comment on that function for how it was calibrated). The estimate is the host time times the ratio, a rough guide that is checked against the real guest in QEMU.

## Wine and ReactOS

- `scripts/wine-audio.sh` runs each `.scr` under Wine with its sound recorded from a private virtual sink and checks it is present and unbroken (and absent with `Sound` off); it needs `pactl`, `parec`, Xvfb, xdotool and numpy. `scripts/guest/guest.py --audio out.wav` records a ReactOS guest's AC97 output.
- `scripts/smoke-wine.sh` runs each `.scr` under Wine on its own Xvfb with `/s` for a few seconds, captures the screen from that Xvfb, checks the picture is not blank, and runs `/c` and `/p` against a host window.
- `scripts/guest/` copies the Afterglow ReactOS image to `~/.cache/afterglow-savers/guest`, adds the `.scr` files and the registry settings, boots it headless in QEMU and takes `screendump`s through the monitor.
- `scripts/guest/check.py --scr DIR --base IMG --out DIR` (WP-PACK-2, SAV-D84) is the ReactOS check of all six `.scr` files: it reflink-copies a disk image (the Quill remix's `qagrr.img` is the usual one), writes the files and `fpscheck.exe` into it, boots it on a copy-on-write overlay in a private headless QEMU (`scripts/guest/vm.py`: `-display none`, a QMP socket in its work folder, no network) and drives Display Properties by absolute pointer clicks and keys: the list, each saver's monitor preview, Settings, Preview ended by a key and by the pointer, the Cracktro's words setting, and the Collection starting after the idle timeout; then powers the guest off and reads the task-list log and the frame rates from its disk. It needs KVM, qemu-system-i386, mtools, ImageMagick and a reactos-liminal checkout (only its `mtools.py` is imported). `tabs.png` is the template that finds Display Properties on the screen, because the applet opens in a different place on each boot. About 9 minutes; 12 GB free on `/home` is the floor. `fpscheck.exe` (built beside the savers, never shipped) renders each scene flat out for five seconds and writes `C:\FPS.TXT`. The older `guest.py` runs the savers from a start-up script and takes timed screendumps; it needs no pointer.
