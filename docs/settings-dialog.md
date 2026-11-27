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

- **Banner.** A strip across the top: the saver's own scene rendered once at the banner's size, at a fixed, flattering moment (Ink's feathering, Twilight's push-in, the cracktro's copper bars, a maze vista), darkened under the title. The `.scr`'s icon (resource 101) and the saver's name are drawn over it in the dialog font. Nothing is embedded: the banner is the scene, so it costs no bitmap.
- **Palette swatches.** Five (six for Ghostwriter) owner-drawn buttons, each a rounded tile showing the palette's ground and three glow hues as stepped bands, the selected one ringed in the sun hue. Light mode shows the day swatches.
- **Live preview.** A child window that runs the real scene at Low detail and the pane's own size, 20 frames a second on a timer, through the same `Saver` as `/p`. A change to a palette, light mode, a seed or a scene setting rebuilds the scene; finish and speed changes apply to the running one at once.
- **Chrome.** Group labels in sentence case with a thin rule in the palette's first glow hue; the rest is the system's dialog colours and font, so it reads on any theme.

## Staying accessible

- **Every control is a real window.** The swatches are `BS_OWNERDRAW` buttons whose window text is the palette's name ("Dusk"), so a screen reader and `IsDialogMessage` see them. They form one tab stop: the first carries `WS_TABSTOP | WS_GROUP`, and a subclass moves between them with the arrow keys (`WM_GETDLGCODE` returns `DLGC_WANTARROWS`) and selects with Space, as radio buttons do. Focus is drawn with `DrawFocusRect`.
- **Keyboard.** Tab order follows the layout (banner and preview take no focus); Enter is OK, Esc is Cancel, Alt+D is Defaults; every label has an access key.
- **Contrast.** Text is drawn in `COLOR_WINDOWTEXT` on `COLOR_3DFACE`, never on the banner's art except the title, which sits on a darkened band. With high contrast on (`SPI_GETHIGHCONTRAST`), or when the screen has 256 colours or fewer, the dialog falls back to v1.
- **Preview cost.** The preview pauses while the dialog is inactive and stops when it closes; on a slow machine (a frame over 150 ms) it drops to 10 frames a second.

## Size

The scene is already in every `.scr`, so the preview and banner cost only the glue: the owner-drawn swatches, the banner composition, the timer and the rebuild logic, estimated at 8 to 14 KiB per file in the i586 build (the v1 dialog generator stays, as the fallback, about 6 KiB). The files are built for speed with a 384 KiB guard per file (SAV-D29); a scene with its shell is 90 to 130 KiB at i586, so there is ample room.

## Work packages

1. **WP-dialog-v2-shell** (area `win32`; done, SAV-D39): the layout above as a `DLGTEMPLATE` with owner-drawn swatches (subclassed for arrows), the banner, the preview child window on a timer driving a `Saver`, the rebuild on change, the v1 fallback conditions, and Wine screenshots of each saver's dialog.
2. **WP-dialog-v2-art** (area `art`): the swatch drawing rules (stepped bands, ring, light-mode variants) and each scene's banner moment and crop, checked by renders from `savers-preview` (a `--banner` option that writes the banner as PNG).
3. **WP-dialog-v2-check** (area `win32`): the keyboard walk (tab order, arrows, Enter, Esc, Alt+D) scripted under Wine, a high-contrast run, and a ReactOS guest screenshot with the Afterglow theme and with the classic one.

## What the shell package left for the others

- **Placeholders for the art package.** `banner_moment` in `dialog_v2.rs` gives every saver the same scene time (3 seconds); the art package picks the real moments and any crop. The swatch tile is a ground with three stepped glow bands and the name in the strongest neutral; the art package owns the final drawing rules. Neither needs an asset file: the banner is the scene and the swatches are drawn.
- **Fallback.** The plain dialog is shown for high contrast, 256 colours or fewer, `PlainDialog` = 1 in the saver's key, or a failed set-up (SPEC section 9). The Collection's dialog runs its scene through the Collection's own maker, as the saver does.
- **Not yet checked** (the check package): real Windows 95, a ReactOS guest, large fonts and 120 dpi (the controls are laid out in dialog units and the banner and preview take their pixel sizes from the controls, so they should scale; this is not verified), and the full keyboard walk. `scripts/smoke-wine.sh` covers, under Wine: the dialog opens with a banner and a running preview, a swatch click changes the palette and the preview's colours, the arrow keys move the choice, OK writes the registry, Cancel writes nothing, and `PlainDialog` gives the plain dialog.
- **Settings added to the table later** (Sound, Volume) reach the dialog through the table. `section` in `template.rs` files them under Look only once they are named there; until then an unknown key lands in Scene and rebuilds the preview's scene when it changes.
- **Pointer.** A full-screen run hides the pointer (`ShowCursor(FALSE)` at the start, restored on exit, and `SetCursor(NULL)` on `WM_SETCURSOR`); the preview and the dialogs do not.
