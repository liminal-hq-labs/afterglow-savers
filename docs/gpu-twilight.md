# Twilight on the graphics card

A design for a GPU edition of Afterglow Twilight that runs at 4K in real time, first on Linux (SAV-D25). The CPU scene in `savers-core` stays the reference: the GPU edition draws the same planet, with the same settings, palettes, camera and finish, and a parity test holds it to the CPU frames.

## What it is

- **A new host-only area, `gpu`:** a crate `crates/savers-gpu` (library: the wgpu renderer) and a binary crate `platforms/linux` (the shells). Both are `std`, in the host workspace, never linked into a `.scr`. `savers-core` is not changed except to expose what both editions must share (below), and stays `no_std`.
- **wgpu** (Vulkan first; Metal, DX12 and GL come with wgpu for later platforms), with WGSL shaders. Fragment shaders for the planet, halo and background; compute or fragment passes for the finish. `wgpu` 30.0.1 is already in the local Cargo cache.

## What it shares with the CPU scene

The GPU edition must not re-invent the scene, so the builder first moves these out of `scenes/twilight.rs` into a small public module in `savers-core` (pure data and functions, no behaviour change, the goldens must not move):

- the shot table and `camera(t)` (centre, radius, tilt, roll, light angle, fade);
- the moon, satellite and star parameters made from the seed;
- the map painters (`paint_ground`, `paint_clouds`, `paint_nebula`): the GPU edition calls them once on the CPU and uploads the results as textures (ground RGBA8, clouds and lights R8, nebula RGBA8), at 2048 × 1024 for 4K (a `map_size` parameter; the CPU scene keeps 256 and 512);
- the light and atmosphere ramps, uploaded as 256 × 1 textures;
- `Settings`, `palette_for` and the `FinishParams` derivation from `saver.rs`, so the same registry-style settings drive both.

The shader is then a transcription of `draw_planet`, `draw_body` and the background, using hardware `atan2`, `asin` and `sqrt` (the CPU's own versions exist only because there is no libm). Twilight is the right first port because it is a pure function of time: no simulation state to keep in step.

## The finish on the GPU

The same chain as `post.rs`, in the same order and with the same strengths: bright pass at a quarter size, two box blurs each way (or a separable Gaussian of equal variance), three long horizontal blurs for the streaks, screen blend; then the vignette, fade, letterbox and grain at the output size. Grain uses the same triangular distribution, hashed per pixel and frame.

## Settings

All of Twilight's settings (SPEC section 7), including `Sun` (the sun disc and glow, drawn behind the planet so it is hidden by the limb, at the shot table's distance), plus two for this edition:

| Setting | Meaning | Default |
| --- | --- | --- |
| `RenderScale` | 25, 50, 75 or 100% of the output, as on the CPU; 4K at 100% is the target | 100 |
| `MaxFps` | frame cap: 30, 60, 120, or 0 for the display's refresh (vsync) | 60 |

Settings come from `~/.config/afterglow-savers/twilight.toml` (the same value names as the registry), overridden by command-line flags (`--set Name=value`, `--seed`, `--palette`).

## Linux shells

- **Fullscreen window** (`afterglow-twilight-gpu`): winit, borderless fullscreen on the current monitor at its native size, cursor hidden, the exit rules of SPEC section 4 (any key or button, or 10 pixels of mouse movement), `--window WxH` for a window instead.
- **xscreensaver hack**: xscreensaver runs hacks with `-root` or `--window-id <id>` (from `$XSCREENSAVER_WINDOW`) and kills them on input; the binary accepts `--window-id`, renders into that X window through wgpu's raw-window-handle (Xlib), and ignores input. A `afterglow-twilight.xml` describes the settings for xscreensaver's dialog.
- **GNOME and KDE:** neither runs third-party screensavers any more (GNOME's lock screen and KDE's KScreenLocker have no hack interface; KDE's greeter allows QML wallpapers). Recommended: document xscreensaver (and XScreenSaver's own Wayland caveats) and the fullscreen binary, and leave a Plasma lock-screen wallpaper plugin as a later package.

## The parity test

`savers-gpu`'s tests render Twilight at 160 × 90 at the CPU goldens' times, seeds and detail levels, read the frames back, and compare them with the CPU frames of the same settings (not the hashes: the CPU frames are rendered in the test): mean absolute error per channel at most 6 levels, and at most 2% of pixels more than 24 levels apart, with grain off. The finish is compared separately on a synthetic frame. Headless: wgpu with a software Vulkan driver (Mesa's lavapipe, `VK_ICD_FILENAMES` pointing at `lvp_icd.json`); on this machine only the Radeon driver is installed, so lavapipe (`vulkan-swrast` on Arch) is needed for CI, and the tests skip with a note when no adapter is found.

## Budgets

- **Speed:** 3840 × 2160 at 60 frames a second on a mid-range GPU at 100% and Ultra; at least 30 on an integrated GPU at 50%. `--bench` prints frame times from GPU timestamps.
- **Dependencies:** wgpu, winit, pollster, bytemuck, raw-window-handle, toml (with serde) and nothing else; `cargo deny` must pass with Apache-2.0, MIT, BSD, Zlib and Unicode licences allowed for this crate only. Release binary under 15 MB stripped.

## Work packages

1. **WP-gpu-share** (area `core`, small): move the shared parts of Twilight into a public module, with the goldens unchanged; add the `map_size` parameter.
2. **WP-gpu-render** (area `gpu`): `savers-gpu` with the planet, background, moons and satellites, and the finish; an offscreen render-to-texture path; the parity test with lavapipe.
3. **WP-gpu-linux** (area `gpu`, after 2): the fullscreen binary, the xscreensaver hack mode and its XML, settings from TOML, the bench; a screenshot mode for previews (`--shot path.png --at 40`).
