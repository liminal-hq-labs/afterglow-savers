# The settings dialog

Every `.scr` answers `/c` with a settings dialog (SPEC section 9). There are two editions: the plain one that ships first, and an Afterglow one that replaces it (SAV-D26).

## The plain dialog (v1)

A modal dialog built at run time from a `DLGTEMPLATE` in memory, generated from the settings table (`savers_core::settings::fields`): combo boxes, trackbars, check boxes, edit boxes and the scene's mask check boxes, in the order of the table, with OK, Cancel and Defaults. Standard controls in the system font, so it works on Windows 95, NT, Wine and ReactOS with any theme or none. It is the fallback the v2 dialog keeps.

## The Afterglow dialog (v2)

```text
+---------------------------------------------------------------+
| [banner: the saver's own scene, 72 px tall, icon, title]       |
+-------------------------------+-------------------------------+
| Look                          |  +-------------------------+  |
|  Palette  [■][■][■][■][■]     |  |                         |  |
|  [x] Light (day) mode          |  |   live preview          |  |
|  Speed    ----o------          |  |   (the real scene,      |  |
| Quality                       |  |    Low detail)          |  |
|  Render scale [50%      v]    |  |                         |  |
|  Detail       [Medium   v]    |  +-------------------------+  |
| Finish                        |   Seed [ 0 ]  [New seed]       |
|  Glow ---o--  Streaks --o---   |                               |
|  Vignette -o-  Grain --o----   |  Scene                        |
|  [x] Letterbox [x] Fades       |   (the scene's own settings)  |
+-------------------------------+-------------------------------+
|                         [Defaults]   [OK]   [Cancel]           |
+---------------------------------------------------------------+
```

- **Banner.** A strip across the top: the saver's own scene rendered at the banner's size, at a fixed, flattering moment and framing, darkened under the title, over a 2-pixel stepped rule in the palette's sun and glow hues (its deep hues by day). The `.scr`'s icon (resource 101) and the saver's name are drawn over it in the dialog font. Nothing is embedded: the banner is the scene, so it costs no bitmap. See [The art](#the-art).
- **Palette swatches.** Five (six for Ghostwriter) owner-drawn buttons, each a small dusk chip drawn from the palette (a stepped sky, a slatted sun on the horizon, a strip of ground) with the palette's name under it, the selected one ringed in the sun hue. Light mode shows the day swatches. See [The art](#the-art).
- **Live preview.** A child window that runs the real scene at Low detail and the pane's own size, 20 frames a second on a timer, through the same `Saver` as `/p`. A change to a palette, light mode, a seed or a scene setting rebuilds the scene; finish and speed changes apply to the running one at once.
- **Chrome.** Group labels in sentence case with a thin rule in the palette's first glow hue; the rest is the system's dialog colours and font, so it reads on any theme.

## The art

WP-dialog-v2-art (SAV-D49). Both pieces are drawn by code from the palette tables in `savers-core`, so nothing is embedded, every size comes out crisp, and the host renders exactly what the dialog shows: `savers-preview banner` and `savers-preview swatches` write them as PNG, and `art/dialog-sheet.sh` makes a contact sheet of all six banners and every swatch on Windows 95's face and the ReactOS Afterglow theme's.

**Banners** (`savers_core::banner`). The scene is rendered at Low detail, `wide` times the banner's width and `aspect` times as wide as it is tall, then a strip is cut out of it round (`across`, `centre`); a scene whose picture builds up over frames (Ink's flow) is run for `lead` frames 0.1 s apart up to its moment. Each banner has its own seed, so it is the same chosen picture whatever the saver's seed; it follows the palette, light mode and the scene's settings. Every part, act and member is on for it, with no letterbox, fades or grain and a light vignette.

| Saver | Moment | Framing | Why |
| --- | --- | --- | --- |
| Maze | 4.5 s, seed 1 | 2.6 : 1, centre 0.42 | A corridor of crenellated walls against the dusk sky, a stepped sun window ahead, right of the title |
| Cracktro | 13 s | 2.8 : 1, centre 0.35 | Two copper bars with the stepped suns bobbing over them, above the scroller (whose letters would fight the title) |
| Twilight | 46 s | 2.4 : 1, centre 0.36 | The push-in: the lit limb, the terminator and the rose rim of the atmosphere |
| Ink | 17 s, seed 2, 30 frames of lead | the strip itself | The first folds into hooks and commas; Ink is drawn straight at the strip's shape, which keeps the lead cheap |
| Collection | each member's own moment, in its turn | four panels: Ink, Twilight, Cracktro, Maze | All four in one strip, each framed for a narrow panel (Twilight 1.2 : 1, the cracktro 1.4 : 1, the maze at 6 s, 1.3 : 1) |
| Quill Ghostwriter | 14 s | 1.6 banner widths, 3 : 1, across 0.3 | The page writing itself ("Dear reader, this document is writing itself."), on the right, with the desk under the title |

The left 60% is darkened towards the band colour (70% at the edge, easing to none) under the icon and title; the title is the palette's lightest neutral with a shadow in its darkest, the "Settings" line its second lightest. Rendering takes 3 to 15 ms on the development host for the cracktro, the maze and Ghostwriter, about 65 ms for Twilight (its maps), 95 ms for Ink and 135 ms for the Collection; estimated 0.7 to 1.6 s on a Pentium III for the last three. The dialog therefore opens with the band colour in the banner and renders it on the first timer tick, and again after a change of palette, light mode or a scene setting; the seed does not change it.

**Swatches** (`savers_core::swatch`). The button is drawn whole at its pixel size, each pixel sampled 4 × 4: the face, then a rounded chip inset 3 pixels (radius 14% of its size, 2 to 5 pixels) with a keyline a step lighter than the ground (by day a mid neutral, so a cream chip holds its edge on a grey face); in it four sky bands mixed from the ground towards the glow hues (outermost at the top, 14% to 66%), the sun half-risen on a 1-pixel horizon in the first glow hue (the deep hue by day), its foot in the first glow hue (a deeper sun by day) rising to the sun hue, cut by three slats in the darkened sky, thickest at the foot; a fifth of the chip is ground. Under the chip, a strip of about 30% of the button's height holds the name in the dialog font and `COLOR_BTNTEXT`, unclipped so "Graphite" may spill a pixel into the gap beside a narrow six-swatch button. The chosen chip has a 2-pixel ring in the sun hue (the first deep hue by day) and a 1-pixel hairline in `COLOR_BTNTEXT` round it; the focus rectangle (`DrawFocusRect`) goes round the name, measured with `DT_CALCRECT`.

**Cost.** About 10 KiB per `.scr` at `opt-level = 3` for the banner composition, the swatch drawing and their tables (the swatch's sampling loop is unrolled); every file stays inside its guard.

## Staying accessible

- **Every control is a real window.** The swatches are `BS_OWNERDRAW` buttons whose window text is the palette's name ("Dusk"), so a screen reader and `IsDialogMessage` see them. They form one tab stop: the first carries `WS_TABSTOP | WS_GROUP`, and a subclass moves between them with the arrow keys (`WM_GETDLGCODE` returns `DLGC_WANTARROWS`) and selects with Space, as radio buttons do. Focus is drawn with `DrawFocusRect`.
- **Keyboard.** Tab order follows the layout (banner and preview take no focus); Enter is OK, Esc is Cancel, Alt+D is Defaults; every label has an access key.
- **Contrast.** Text is drawn in `COLOR_WINDOWTEXT` on `COLOR_3DFACE`, never on the banner's art except the title, which sits on a darkened band. With high contrast on (`SPI_GETHIGHCONTRAST`), or when the screen has 256 colours or fewer, the dialog falls back to v1.
- **Preview cost.** The preview pauses while the dialog is inactive and stops when it closes; on a slow machine (a frame over 150 ms) it drops to 10 frames a second.

## Size

The scene is already in every `.scr`, so the preview and banner cost only the glue: the owner-drawn swatches, the banner composition, the timer and the rebuild logic, estimated at 8 to 14 KiB per file in the i586 build (the v1 dialog generator stays, as the fallback, about 6 KiB). The files are built for speed with a 384 KiB guard per file (SAV-D29); a scene with its shell is 90 to 130 KiB at i586, so there is ample room.

## Work packages

1. **WP-dialog-v2-shell** (area `win32`; done, SAV-D39): the layout above as a `DLGTEMPLATE` with owner-drawn swatches (subclassed for arrows), the banner, the preview child window on a timer driving a `Saver`, the rebuild on change, the v1 fallback conditions, and Wine screenshots of each saver's dialog.
2. **WP-dialog-v2-art** (area `art`; done, SAV-D49): the swatch drawing rules (stepped sky, slatted sun, ring, light-mode variants) and each scene's banner moment and crop, checked by renders from `savers-preview banner` and `savers-preview swatches`, a contact sheet (`art/dialog-sheet.sh`) and Wine screendumps of all six dialogs.
3. **WP-dialog-v2-check** (area `win32`): the keyboard walk (tab order, arrows, Enter, Esc, Alt+D) scripted under Wine, a high-contrast run, and a ReactOS guest screenshot with the Afterglow theme and with the classic one.

## What the shell package left for the others

- **Fallback.** The plain dialog is shown for high contrast, 256 colours or fewer, a screen too small for it (800 by 600), `PlainDialog` = 1 in the saver's key, or a failed set-up (SPEC section 9). The Collection's dialog runs its scene through the Collection's own maker, as the saver does.
- **Not yet checked** (the check package): real Windows 95, a ReactOS guest, large fonts and 120 dpi (the controls are laid out in dialog units and the banner and preview take their pixel sizes from the controls, so they should scale; this is not verified), and the full keyboard walk. `scripts/smoke-wine.sh` covers, under Wine: the dialog opens with a banner and a running preview, a swatch click changes the palette and the preview's colours, the arrow keys move the choice, OK writes the registry, Cancel writes nothing, and `PlainDialog` gives the plain dialog.
- **Settings added to the table later** (Sound, Volume) reach the dialog through the table. `section` in `template.rs` files them under Look only once they are named there; until then an unknown key lands in Scene and rebuilds the preview's scene when it changes.
- **Pointer.** A full-screen run hides the pointer (`ShowCursor(FALSE)` at the start, restored on exit, and `SetCursor(NULL)` on `WM_SETCURSOR`); the preview and the dialogs do not.

## The Cracktro's words

The Cracktro's scene group has a check box, "Show the scrolling words" (`Scroller`, off by default), and the text field of its own words (`Message`). Both dialogs grey the field out while the box is clear and enable it when it is ticked (`EnableWindow`, on a click and when the controls are filled). The Afterglow dialog lists the box as the last of the scene's settings, above the field; the plain dialog puts it on the field's label row, left of the label "Scroller text", so the dialog stays within 800 × 600 (the Cracktro's plain dialog is 340 dialog units tall; the limit is 350). The banner and the preview pane follow the setting, so a default Cracktro banner has no words.
