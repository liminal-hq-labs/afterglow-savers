# Afterglow Screensavers specification

The source of truth for what the savers do. Why each choice was made is in [docs/decisions.md](docs/decisions.md); what each scene looks like is in [docs/scenes.md](docs/scenes.md).

## 1. The files

| File | Listed as (string 1) | Registry name | Scene |
| --- | --- | --- | --- |
| `AGMAZE.SCR` | Afterglow Maze | `Maze` | A 3D maze walk through dusk architecture |
| `AGCRACK.SCR` | Afterglow Cracktro | `Cracktro` | A demoscene intro with a sine scroller |
| `AGTWILIT.SCR` | Afterglow Twilight | `Twilight` | A tiny painted planet in a glowing atmosphere |
| `AGINK.SCR` | Afterglow Ink | `Ink` | Flowing ink and marbling |
| `AGCOLLCT.SCR` | Afterglow Collection | `Collection` | The four in turn, with cross-fades |
| `AGQUILL.SCR` | Quill Ghostwriter | `Ghostwriter` | An Easter egg: a document that writes itself (not installed by default) |

Each file is a Win32 GUI program (subsystem 4.0) for a Pentium (`i586`, x87 floats, no MMX or SSE) or for x86-64, with no C runtime, and carries string 1, an icon (resource 101) and version information. Each is at most 384 KiB, and the five main files together at most 1.5 MiB (SAV-D29): a guard against a bloated build, not a target; the files are built for speed. They belong in the system folder (`C:\ReactOS\system32`, `C:\Windows\System32`, or `C:\Windows\System` on Windows 9x) or beside the Display applet.

## 2. The command line

The first letter after a `/` or `-`, in either case, picks the mode, as `scrnsave.lib` does ([docs/reactos-scr.md](docs/reactos-scr.md)):

| Argument | Mode |
| --- | --- |
| `/s` | Full screen until input (§4) |
| `/p <hwnd>` or `/p:<hwnd>` | Preview inside that window; exits with code 1 when `<hwnd>` is not a window |
| `/c:<hwnd>` or `/c <hwnd>` | The settings dialog, modal to that window; `/c` alone uses the foreground window |
| `/a` | Ignored (Windows 9x password change); exits 0 |
| nothing, or anything else | The settings dialog with no owner |

## 3. Drawing

- The scene is drawn by the CPU into a 32-bit frame at the **render scale** (100, 75, 50 or 25% of the window in each direction), finished (§6), and shown with GDI `StretchDIBits` (SAV-D2).
- Animation follows elapsed time, scaled by the **speed** setting, so a slow machine shows fewer frames of the same motion. A frame that took longer than 100 ms advances time by at most 100 ms.
- Frames are capped at **Max frames per second** (default 30); the program sleeps for the rest of each frame, so it does not take the whole CPU at idle priority.
- **Threads** (SAV-D51, SAV-D52): the x86-64 build shares the frame's passes between processors, in a full-screen run of at least a million pixels, on up to seven worker threads at idle priority besides the drawing thread (one fewer than the machine's processors). The frame is the same byte for byte with any number of threads. A `Threads` value in the saver's key (not in the dialog) overrides the count: 1 for none, 2 to 8 for that many in all; missing or 0 is automatic. The Pentium build, the preview and the dialog use one thread.
- The same seed, settings, size and time give the same frame (SAV-D6). A seed setting of 0 means a new seed each run, taken from the tick count and the cursor position.

## 4. Full screen (`/s`)

- One borderless topmost window (`WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TOOLWINDOW`) over the whole virtual screen (`SM_XVIRTUALSCREEN` and the rest; the primary screen where those metrics are zero), with no cursor. On several monitors the one picture spans them all.
- **It ends** on any key or mouse button, on losing activation (once it has been active), and on a mouse move once the cursor is more than 10 pixels in x or y from where the first move found it. It also ends when the system sends `WM_CLOSE` or the process is terminated, which Winlogon does on activity.
- The pointer is hidden for the run (`ShowCursor(FALSE)`, restored on exit, with `SetCursor(NULL)` on `WM_SETCURSOR`); the preview pane and the dialogs leave it alone.
- `SC_SCREENSAVE`, `SC_CLOSE`, `SC_NEXTWINDOW` and `SC_PREVWINDOW` system commands are refused while it runs.
- It fades up from black over one second and ends at once when told to (no fade out: the user wants their desktop).
- **Sound** (SAV-D34 to D38, D48): with `Sound` on and `Volume` above 0 it plays the saver's generated score through `waveOut` (16-bit stereo, the rate from `Detail`: 22,050, 32,000, 44,100 or 48,000 Hz, 22,050 Hz if the device refuses it), fading up with the picture and stopping at once when the saver ends. It is silent in `/p` and `/c`, and silent, with no message, when no wave device opens. The Collection follows the member on screen, cross-faded with the picture.
- No password code: NT and ReactOS lock the workstation through Winlogon's `ScreenSaverIsSecure`; Windows 9x passwords are not supported (SAV-D17).

## 5. Preview (`/p`)

- A child window filling the parent's client area. Render scale 25% of a window that small is too coarse, so the preview always draws at the preview's own size with the Low detail level, at most 20 frames a second, and with letterbox off.
- Input never ends it. It ends when the parent window stops existing (checked every frame) or when the process is terminated.

## 6. Finishing (post-processing)

Applied in this order, each optional and adjustable:

| Effect | Setting | Range | Default |
| --- | --- | --- | --- |
| Glow (bloom) from the bright parts | `Bloom` | 0–100 | 60 |
| Anamorphic horizontal streaks from the brightest parts | `Streaks` | 0–100 | 40 |
| Vignette | `Vignette` | 0–100 | 50 |
| Film grain, new every frame, at the output size | `Grain` | 0–100 | 30 |
| Letterbox bars (2.39:1 on wide screens, 1.85:1 on 4:3 ones) | `Letterbox` | 0 or 1 | 0 |
| Slow fades between shots and parts | `Fades` | 0 or 1 | 1 |

## 7. Settings

Each saver keeps its settings under **`HKEY_CURRENT_USER\Software\Liminal HQ\Afterglow Screensavers\<Registry name>`** (SAV-D7). Numbers are `REG_DWORD`, text is `REG_SZ`. A missing or out-of-range value means the default; the program never fails on a bad value, and only the settings dialog writes.

### Common to every saver

| Value | Meaning | Range | Default |
| --- | --- | --- | --- |
| `Sound` | Play the saver's generated soundtrack (never in the preview or the dialog; silent without a sound device) | 0–1 | 1 |
| `Volume` | Its level, percent | 0–100 | 28 |
| `RenderScale` | Percent of the window drawn | 25, 50, 75, 100 | 50 |
| `Detail` | 0 Low, 1 Medium, 2 High, 3 Ultra | 0–3 | 1 |
| `Speed` | Percent of normal speed | 25–200 | 100 |
| `Palette` | 0 Dusk, 1 Indigo, 2 Ember, 3 Aurora, 4 Graphite (Ghostwriter adds 5 Quill, its default) | 0–4 | 0 |
| `Light` | 1 for the palette's light (day) mode, where the scene has one (Ink, Maze and Ghostwriter; the Maze by day keeps its dusk stone under a daytime sky and haze) | 0–1 | 0 |
| `MaxFps` | Frame cap | 10–60 | 30 |
| `Bloom`, `Streaks`, `Vignette`, `Grain`, `Letterbox`, `Fades` | §6 | | |
| `Seed` | The scene's seed; 0 for a new one each run | any | 0 |

### Scene settings

| Saver | Value | Meaning | Range | Default |
| --- | --- | --- | --- | --- |
| Maze | `Floor` | 0 polished stone, 1 water, 2 tiles | 0–2 | 0 |
| Maze | `MazeSize` | Cells along each side | 8–32 | 14 |
| Maze | `LoopMaze` | Through the portal, walk the same maze again (a true loop, as the preview videos use) instead of a new one | 0–1 | 0 |
| Twilight | `Atmosphere` | Strength of the atmosphere's glow | 0–100 | 70 |
| Twilight | `Moons` | Number of moons | 0–2 | 2 |
| Twilight | `Sun` | Show the sun: far off in the establishing shot, and sliding out from behind the limb in the night-side drift | 0–1 | 1 |
| Ink | `Thickness` | Width of the ink bands against the water | 0–100 | 50 |
| Ink | `Turbulence` | Strength and number of the eddies | 0–100 | 50 |
| Cracktro | `Message` (`REG_SZ`) | The scroller's text, at most 2,000 characters | | §8 |
| Cracktro | `Parts` | Bit mask of the parts: 1 copper, 2 plasma, 4 rotozoomer, 8 tunnel, 16 vector bars | 1–31 | 31 |
| Cracktro | `Shuffle` | Shuffle the parts | 0–1 | 0 |
| Collection | `Duration` | Seconds per saver | 30–600 | 90 |
| Collection | `Savers` | Bit mask: 1 Ink, 2 Twilight, 4 Cracktro, 8 Maze, 16 Quill Ghostwriter (played only where `AGQUILL.SCR` is installed beside the Collection; otherwise that turn is Ink's) | 1–31 | 15 |
| Collection | `Shuffle` | Shuffle the order | 0–1 | 0 |
| Ghostwriter | `TextBank` (`REG_SZ`) | The lines it types, one per line; empty means the built-in bank | | |
| Ghostwriter | `Acts` | Bit mask of the six acts | 1–63 | 63 |

The Collection uses its own common settings for all four scenes, and each scene's own scene settings and seed from that scene's key (SAV-D8).

## 8. The cracktro's default message

> Hello, everyone, and welcome to Afterglow for ReactOS. Greetings fly out to the ReactOS developers, the Wine project, everyone keeping a 486 humming in the spare room, the DOS and Amiga sceners, the folks who recap old motherboards, the archivists who keep the old disks readable, and you, for leaving your machine alone long enough to see this. Afterglow for ReactOS is an unofficial remaster, made by Liminal HQ. Not affiliated with the ReactOS project. ... Copper bars courtesy of trigonometry. Plasma courtesy of more trigonometry. Press any key to return to work. ...

## 9. The settings dialog

A modal dialog, "Afterglow Maze Settings" (the saver's name, then "Settings", with the saver's icon), built at run time from common controls ReactOS and Windows 95 both have (SAV-D9). There are two editions of it: the Afterglow one, which is shown when it can be, and the plain one, which is the fallback (SAV-D26, SAV-D39). Both hold the same settings under the same control identifiers, read and write the registry the same way, and answer to the same keys:

- **OK** (Enter) writes every value and ends; **Cancel** (Esc) writes nothing; **Defaults** (Alt+D) puts the defaults in the controls.
- Every label has a mnemonic.

### The Afterglow dialog

A banner across the top (the saver's own scene at a chosen moment, darkened towards the left under the saver's icon and name, over a stepped rule in the palette's hues), then two columns. On the left, **Look** (palette as clickable swatches, light mode, speed), **Quality** (render scale, detail, frame cap) and **Finish** (bloom, streaks, vignette, grain, letterbox, fades); on the right, a **live preview** (the real scene at Low detail, 20 frames a second, following every change), the seed with a **New seed** button, and the scene's own settings with the text editor of the cracktro and Ghostwriter. When the right column would be the taller, Finish and Scene swap columns so the two are balanced.

- **The banner** (SAV-D49) is the scene itself, never an embedded picture: each saver's moment, framing and seed are fixed in `savers_core::banner` (Ink's first folds, Twilight's push-in at the lit limb, the cracktro's copper bars and stepped suns, a maze corridor with a sun window, Quill's page writing itself; the Collection shows Ink, Twilight, the cracktro and the maze side by side). It follows the palette, light mode and the scene's own settings, not the seed, and is rendered at Low detail on the dialog's timer after the dialog shows and after each change of palette, light mode or scene setting, so opening or clicking never waits for it. The title is drawn in the palette's lightest neutral on a band of its darkest, by day as by night.
- **The swatches** are owner-drawn buttons whose window text is the palette's name. Each is a chip drawn from the palette (`savers_core::swatch`): a stepped sky from the ground through the glow hues, the sun half-risen on the horizon in the sun hue with three slats, a strip of ground; by day the day palette. The name (its first word: "Quill" for "Quill navy and gold") is written under the chip in the system's text colour on the face. They are one tab stop (the chosen swatch); the arrow keys move between them and choose, and Space chooses. The chosen one is ringed in the palette's sun hue (its deep hue by day), with a hairline in the system's text colour so it shows on any ground; the focus rectangle goes round the name.
- A change to the palette, light mode, seed or a scene setting rebuilds the preview's scene; finish and speed changes reach the running one at once. The preview stops while the dialog is inactive and drops to 10 frames a second when a frame takes over 150 ms.
- Group titles are drawn in the system's text colour and ruled in the palette's first glow hue (its deeper hue by day).
- Text and controls use the system's colours and the dialog's font, so it reads under any theme, or none.

### The plain dialog

Group boxes hold the controls, in this tab order:

- **Look** (top left): palette (combo box), light mode (check box), speed (trackbar).
- **Quality** (left, under Look): render scale, detail and frame cap (combo boxes).
- **Finish** (right): bloom, streaks, vignette and grain (trackbars), letterbox and fades (check boxes).
- **Scene** (full width, below): the scene's own settings in two runs: trackbars, combo boxes, a seed edit box, the cracktro's part check boxes, the collection's duration and saver check boxes, and for the cracktro (and Ghostwriter) the multi-line text editor. A label too long for its column wraps onto a second line rather than clipping.

### Which one is shown

The plain dialog is shown instead of the Afterglow one when high contrast is on (`SPI_GETHIGHCONTRAST`), when the display has 256 colours or fewer, when the screen is too small for it (the dialog is 564 by about 560 pixels, so 800 by 600 gets the plain one), when the saver's registry key holds a `PlainDialog` value of 1, or when the Afterglow dialog cannot be created or set up (the template fails, or the banner or preview pane has no size).

## 10. Quill Ghostwriter

An Easter egg (SAV-D20, SAV-D22): a real `.scr` that appears in the Screen Saver list as "Quill Ghostwriter" wherever it is installed, but it is never selected by default, and the Collection plays it only when its `Savers` mask asks. It ships only with the Quill remix of the ReactOS image and as an optional, unselected component of Quill's own installers, never in the Afterglow recipes, so its crate and file stay separate. Its scene is in [docs/scenes.md](docs/scenes.md#quill-ghostwriter-an-easter-egg). Its own budget is 384 KiB, outside the five main files' 1.5 MiB.

### Windows 3.x edition (`QUILLGW.SCR`)

A second, much smaller program for Windows 3.0, 3.1 and 3.11 (SAV-D23, SAV-D37): a 16-bit NE executable built with the Win16 toolchain of Quill's Windows 3.x build, not the Win32 shell, in its own workspace `platforms/win16`. It shares no code with `savers-core`. It follows the conventions of the stock savers that Windows 3.1 ships:

- **Listing.** The module description is `SCRNSAVE : Quill Ghostwriter`, so Control Panel's Desktop applet lists the file, from the Windows system folder, as "Quill Ghostwriter". It exports `SCREENSAVERPROC` and `SCREENSAVERCONFIGUREDIALOG` as the stock savers do. The choice is stored by Windows itself: `SCRNSAVE.EXE=` in `SYSTEM.INI`'s `[boot]`, and `ScreenSaveActive` and `ScreenSaveTimeOut` in `WIN.INI`'s `[windows]`.
- **Command line.** Windows 3.x passes `/s` (the Test button and the idle timer) or `/c` (the Setup button), in lower case, with a slash. The saver also takes `-` for `/`, either case, and no argument as `/s`. There is no `/p`: any other switch does nothing and exits.
- **Running.** A topmost popup window of class `WindowsScreenSaverClass` over the whole screen, the cursor hidden, a timer of 60 ms. A key, a button, deactivation, or a mouse move of more than three pixels ends it. A second copy exits at once.
- **Settings.** `CONTROL.INI`, section `[Screen Saver.Quill Ghostwriter]`: `Acts` (a mask numbered as the Windows 95 edition's: `1` is typing with mistakes, `32` the visit from the "Save the changes?" box; default `33`) and `Speed` (`0` slow, `1` normal, `2` fast). `Log=1` makes each start write its command line to `C:\QGWn.LOG` (a diagnostic). Setup is a dialog resource with two check boxes and three radio buttons.
- **The scene.** Reduced for 16-bit integer GDI: a Quill window (navy title bar, menu, B I U buttons, ruler with tab stops, cream page, status bar counting `Pg`, `Ln` and `Col`) drifting on a Lissajous path over a dark desk, drawn into a memory bitmap and copied with `BitBlt`, only when its content changes; typing from a bank of sixteen lines with seven typos, the red squiggle, the suggestion box, the correction, and the "Save the changes to Untitled before Quill closes?" box with a blinking focus rectangle. Where the screen has fewer than 256 colours it uses the sixteen pure colours of the VGA (black desk, navy, dark yellow for the gold, white page); elsewhere Quill's own.
- **Icon.** Icon group 100: a gold nib writing on a navy page, 32 and 16 pixels in the sixteen standard colours, drawn as pixels in the style of Quill's own Windows 3.x icon (`art/gen-quillgw-icon.py`). Program Manager and File Manager show it.
- **Not done in this edition:** the reflow, time machine, font eras and lore acts, the stroke font, the palettes and the Collection. Its ladder has one rung.

## 11. The Linux edition (Twilight on the graphics card)

`afterglow-twilight-gpu` (`platforms/linux`, `docs/linux-edition.md`) draws Twilight on the graphics card with the same scene, palettes, camera and finish as section 7's Twilight, and the same settings by the same names, kept in `~/.config/liminal-hq/afterglow-savers/twilight.toml` instead of the registry. Differences:

- **Defaults:** `RenderScale` 100, `Detail` Ultra and a frame cap of 60 (`FrameCap`: 0 for vsync, or 10 to 240, which replaces `MaxFps`); everything else as on Windows, including `Sound` on at `Volume` 28.
- **Full screen** is borderless on the primary monitor at its native size, cursor hidden, ending as section 4 says (a key, a button, or more than 10 pixels of pointer movement), with `--window WxH` for a window and `--preview` for a small silent one.
- **As an xscreensaver hack** (`-window-id`, `-root`) it draws into xscreensaver's X11 window, ignores input, and is **silent unless `--sound`** is given.
- **Sound** is the same score at 48 kHz through the default ALSA device; with no device it is silent.
