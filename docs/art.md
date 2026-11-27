# Art

The look of the savers and how they use the Afterglow palettes. The palette values come from `afterglow-themes` (`scripts/palette.mjs` and the wallpaper palettes in `scripts/build-wallpapers.mjs`), copied into `crates/savers-core/src/palette.rs`.

## The palettes

Each palette has a neutral ramp (900 is the background, 100 the strongest contrast), a sun hue and two to four glow hues in order outwards from the sun, as the dusk wallpapers use them:

| Palette | Neutral | Sun | Glow hues | Light mode |
| --- | --- | --- | --- | --- |
| Dusk (default) | void | yellow `#fbbf24` | orange `#ffaa40`, rose `#f43f5e`, purple `#a78bfa` | paper, sun yellow, glow yellow and orange, deep purple hills |
| Indigo | indigo | orange | purple, blue `#60a5fa` | paper, glow orange and purple, deep blue |
| Ember | void | yellow | yellow, orange, rose | paper, glow yellow and orange, deep red |
| Aurora | indigo | cyan `#22d3ee` | green `#2ec66a`, cyan, blue, purple | paper, glow cyan and blue, deep green |
| Graphite | void | orange | orange, gray `#94a3b8`, gray | paper, glow orange and gray, deep gray |

Neutral ramps (900 → 100): void `#050507 #12121a #1f2230 #333847 #5a6273 #8d97a8 #b4bfce #e0e0e0 #f4f4f8`; indigo `#0f0e1a #1a1828 #2a2840 #3a384f #5f5d78 #8f8da6 #b8b6c8 #e4e2ec #f1eee6`; paper `#fbfaf6 #f2eff4 #e6e0eb #c9c5d0 #8f8c98 #6b6877 #484554 #2b2a33 #1c1b20`. Deep hues for light mode: orange `#b45309`, red `#be123c`, purple `#5e44cc`, cyan `#0e7490`, blue `#2563eb`, green `#15803d`, yellow `#a16207`, gray `#5a6a80`.

## Rules every scene follows

- **Dark ground.** The neutral 900 is the ground; most of the frame is near it. Glow hues are for light: ink, atmosphere, copper, windows.
- **Ramps, not rainbows.** A scene builds a 256-entry ramp from the ground through the glow hues to the sun hue (and to neutral 100 at the very top), and maps intensity through it. Colours mix in linear light where it matters (the glow), in sRGB elsewhere.
- **Stepped and smooth.** The Afterglow shapes are stepped bands and round suns: stepped coasts on the planet, stepped crenellations in the maze, stepped rings in the tunnel. Smooth gradients get the triangular dither the wallpapers use.
- **The finish** is the same in every scene: bloom from the bright parts, a horizontal streak from the brightest, a vignette, a fine grain. They are tuned so a frame reads as film, not as a filter.

## Icons

Each saver's icon is an SVG master in `art/masters/`, drawn in Afterglow's icon set E language (the `Pen`, materials, profiles, dusk plate and keyline of `desktop/icons/gen_e.py` in `afterglow-themes`), rendered to 48, 32 and 16 pixels by `art/render-icons.sh` into the committed `.ico` files in `platforms/win32/res/`. Explorer shows the icon; the Display applet's list shows string 1.

| Saver | Motif |
| --- | --- |
| Maze | A corridor in one-point perspective ending in a stepped sun window |
| Cracktro | Three copper bars over a sine wave |
| Twilight | A small planet with a lit crescent and a moon |
| Ink | Two marbled swirls folding into each other |
| Collection | The four motifs as four quarter tiles |
| Quill Ghostwriter | A gold nib on a navy page (a placeholder until Quill's own art is used) |

`QUILLGW.SCR`, the Windows 3.x edition, has its own icon in the sixteen standard Windows colours (SAV-D50): a gold nib writing the last line on a navy page with a folded corner, 32 and 16 pixels, drawn as pixels in the style of Quill's `QUILL31.ICO`. `art/gen-quillgw-icon.py` writes its masters (`art/masters/quillgw.svg`, `quillgw-16.svg`) and `platforms/win16/res/quillgw.ico`; `--check` compares them with the committed files.

## The settings dialog

The Afterglow settings dialog's banners and palette swatches are drawn from the palettes by `savers-core` rather than kept as pictures (SAV-D49): each banner is the saver's own scene at a chosen moment, and each swatch a small dusk chip, a stepped sky over a slatted sun. The rules, moments and framings are in [settings-dialog.md](settings-dialog.md#the-art); `art/dialog-sheet.sh` writes a contact sheet of them.
