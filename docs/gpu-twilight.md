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

## The renderer

`crates/savers-gpu` (SAV-D32) is the renderer, headless: it draws into textures and reads them back, and a window can present the output texture later. Its parts:

- `Gpu` picks an adapter by name (`AFTERGLOW_GPU_ADAPTER`, or the first discrete one) and makes the device. `TwilightRenderer` takes the settings, a `Shared` (from `savers_core::twilight_shared`), the seed, the render size and the output size, uploads the maps once, and draws a frame for scene time `t` and wall time `wall` (the fade in from black). `shared_for_gpu` makes the `Shared` with 2048 × 1024 maps at any detail level for a large render; the parity test uses `Shared::new` so both editions paint the same maps. `finish_params` is `Saver::draw`'s derivation of the finish for Twilight.
- All textures are `Rgba8Unorm` (the frames) so quantisation matches the CPU, except the glow's blur chain (`Rgba16Float` for the bloom, `R16Float` for the streaks), which is read with `textureLoad` and never filtered.

### Passes

Render size is `W × H`; the coarse size is `W/4 × H/4` rounded up (an eighth at Low). Every pass is a full-screen triangle except the stars. Each shader is a WGSL file in `crates/savers-gpu/src/shaders` and every uniform buffer is a few `vec4`s written each frame (the CPU computes the camera, the bodies' places and the stars' pixels, as the CPU scene does, so the shaders hold no scene logic beyond drawing).

| Pass | Shader | Reads | Writes | Uniforms |
| --- | --- | --- | --- | --- |
| Background | `background.wgsl` | nebula (three quarters of the render size, bilinear) | backdrop `W × H` | the pan, as an origin and a step in nebula texels |
| Stars | `stars.wgsl`, instanced 6 vertices | one instance per lit pixel (a centre, and four arms on the bright ones), at most 3500 | backdrop, with the blend `src + dst × (1 − src)`, which is the screen blend | the render size |
| Scene | `scene.wgsl` | backdrop, ground (RGBA8, 2048 × 1024 at most), clouds (R8), lights (R8), the light and atmosphere ramps (256 × 1), the craters (R8 array, 256 × 256 a moon) | the finishing chain's input `W × H` | the planet's circle, light, view-to-planet matrix, spin, flags (smooth, shadows, lights, sheen from the detail level), the sun, and up to eight bodies |
| Bright | `bright.wgsl` | input | bloom and streak, coarse, two render targets | the threshold and its span |
| Blur | `blur.wgsl`, four on the bloom and four on the streaks | bloom or streak | the other of the pair | the axis and the radius (a bloom radius of `coarse height / 24`, streak rows of `coarse width / 7` three times, then one column of 1) |
| Pack | `pack.wgsl` | bloom, streak | glow, 8-bit, coarse | the gains (`bloom × 3`, `streaks × 5`) and the tint |
| Lay | `lay.wgsl` | input, glow (bilinear) | frame | the render size |
| Output | `output.wgsl` | frame (bilinear) | the output texture, `COPY_SRC` | the output size, the letterbox rows, the vignette, the fade and the grain |

The scene pass is one fragment shader over the whole frame: the sun, the bodies behind the planet, the planet and its halo, then the bodies in front, in the CPU's order; a moon's craters are the core's `noise3` baked into a texture, so the shader has no noise of its own. A texture is read through one sampler only (GL cannot do two), so the unsmoothed ground, the lights and the cloud rows use `textureLoad`.

`savers-gpu-preview` renders frames (`frames`), writes a CPU-over-GPU comparison with the error (`compare`), and times the GPU at 1080p and 4K (`bench`); `adapters` lists what wgpu sees. The timings are the wall time for a frame to be submitted and finished (the GPU timestamp bench belongs to `--bench` in the Linux binary).

## Settings

All of Twilight's settings (SPEC section 7), including `Sun` (the sun disc and glow, drawn behind the planet so it is hidden by the limb, at the shot table's distance), plus two for this edition:

| Setting | Meaning | Default |
| --- | --- | --- |
| `RenderScale` | 25, 50, 75 or 100% of the output, as on the CPU; 4K at 100% is the target | 100 |
| `MaxFps` | frame cap: 30, 60, 120, or 0 for the display's refresh (vsync) | 60 |

Settings come from `~/.config/liminal-hq/afterglow-savers/twilight.toml` (the same value names as the registry; SAV-D43), overridden by command-line flags (`--set Name=value`, `--seed`, `--palette`).

## Linux shells

- **Fullscreen window** (`afterglow-twilight-gpu`): winit, borderless fullscreen on the current monitor at its native size, cursor hidden, the exit rules of SPEC section 4 (any key or button, or 10 pixels of mouse movement), `--window WxH` for a window instead.
- **xscreensaver hack**: xscreensaver runs hacks with `-root` or `--window-id <id>` (from `$XSCREENSAVER_WINDOW`) and kills them on input; the binary accepts `--window-id`, renders into that X window through wgpu's raw-window-handle (Xlib), and ignores input. A `afterglow-twilight.xml` describes the settings for xscreensaver's dialog.
- **GNOME and KDE:** neither runs third-party screensavers any more (GNOME's lock screen and KDE's KScreenLocker have no hack interface; KDE's greeter allows QML wallpapers). Recommended: document xscreensaver (and XScreenSaver's own Wayland caveats) and the fullscreen binary, and leave a Plasma lock-screen wallpaper plugin as a later package.

## The parity test

`savers-gpu`'s tests render Twilight at 160 × 90 at the CPU goldens' times, seeds and detail levels, read the frames back, and compare them with the CPU frames of the same settings (not the hashes: the CPU frames are rendered in the test): mean absolute error per channel at most 6 levels, and at most 2% of pixels more than 24 levels apart, with grain off. The finish is compared separately on a synthetic frame. Headless: wgpu with a software Vulkan driver (Mesa's lavapipe, `vulkan-swrast` on Arch, selected with `VK_ICD_FILENAMES` pointing at `lvp_icd.json` or by `AFTERGLOW_GPU_ADAPTER=llvmpipe`, which is the test's default); the tests skip with a note when no adapter is found. `AFTERGLOW_PARITY_DUMP=<folder>` writes each pair as a side-by-side PPM.

The numbers are the doc's: a mean absolute error per channel of at most 6 levels, and at most 2% of pixels with a channel more than 24 levels apart. The measured worst over every detail level and six times (one in each shot) is under 3 levels and about 0.2% far on lavapipe, the Radeon (RADV) and GL (radeonsi); the finish alone is within 1.3 levels with a worst pixel of 6. The budget keeps twice the measured mean error and ten times the far share in hand because what differs is real and small: the CPU's fixed-point steps (a 1024-step arcsine table, 8-bit mixes and products) against hardware float, a star's arms (the CPU sets them to the centre's colour) and the limb's one-pixel edge. A tolerance any tighter would only test the integer rounding.

## Budgets

- **Speed:** 3840 × 2160 at 60 frames a second on a mid-range GPU at 100% and Ultra; at least 30 on an integrated GPU at 50%. `--bench` prints frame times from GPU timestamps.
- **Dependencies:** wgpu, winit, pollster, bytemuck, raw-window-handle, toml (with serde) and nothing else; `cargo deny` must pass with Apache-2.0, MIT, BSD, Zlib and Unicode licences allowed for this crate only. Release binary under 15 MB stripped.

## Work packages

1. **WP-gpu-share** (area `core`, small): move the shared parts of Twilight into a public module, with the goldens unchanged; add the `map_size` parameter.
2. **WP-gpu-render** (area `gpu`): `savers-gpu` with the planet, background, moons and satellites, and the finish; an offscreen render-to-texture path; the parity test with lavapipe.
3. **WP-gpu-linux** (area `gpu`, after 2; done, `docs/linux-edition.md`): the fullscreen binary, the xscreensaver hack mode and its XML, settings from TOML, the bench; a screenshot mode for previews (`--shot path.png --at 40`).
