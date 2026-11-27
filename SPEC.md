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

Each file is a Win32 GUI program (subsystem 4.0) for a Pentium (`i586`, x87 floats, no MMX or SSE) or for x86-64, with no C runtime, and carries string 1, an icon (resource 101) and version information. Each is at most 192 KiB; the five main files together at most 640 KiB (SAV-D14). They belong in the system folder (`C:\ReactOS\system32`, `C:\Windows\System32`, or `C:\Windows\System` on Windows 9x) or beside the Display applet.

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
- The same seed, settings, size and time give the same frame (SAV-D6). A seed setting of 0 means a new seed each run, taken from the tick count and the cursor position.

## 4. Full screen (`/s`)

- One borderless topmost window (`WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TOOLWINDOW`) over the whole virtual screen (`SM_XVIRTUALSCREEN` and the rest; the primary screen where those metrics are zero), with no cursor. On several monitors the one picture spans them all.
- **It ends** on any key or mouse button, on losing activation, and on a mouse move once the cursor is more than 10 pixels in x or y from where the first move found it. It also ends when the system sends `WM_CLOSE` or the process is terminated, which Winlogon does on activity.
- `SC_SCREENSAVE`, `SC_CLOSE`, `SC_NEXTWINDOW` and `SC_PREVWINDOW` system commands are refused while it runs.
- It fades up from black over one second and ends at once when told to (no fade out: the user wants their desktop).
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
| `RenderScale` | Percent of the window drawn | 25, 50, 75, 100 | 50 |
| `Detail` | 0 Low, 1 Medium, 2 High, 3 Ultra | 0–3 | 1 |
| `Speed` | Percent of normal speed | 25–200 | 100 |
| `Palette` | 0 Dusk, 1 Indigo, 2 Ember, 3 Aurora, 4 Graphite (Ghostwriter adds 5 Quill, its default) | 0–4 | 0 |
| `Light` | 1 for the palette's light (day) mode, where the scene has one (Ink, Maze, Ghostwriter) | 0–1 | 0 |
| `MaxFps` | Frame cap | 10–60 | 30 |
| `Bloom`, `Streaks`, `Vignette`, `Grain`, `Letterbox`, `Fades` | §6 | | |
| `Seed` | The scene's seed; 0 for a new one each run | any | 0 |

### Scene settings

| Saver | Value | Meaning | Range | Default |
| --- | --- | --- | --- | --- |
| Maze | `Floor` | 0 polished stone, 1 water, 2 tiles | 0–2 | 0 |
| Maze | `MazeSize` | Cells along each side | 8–32 | 14 |
| Twilight | `Atmosphere` | Strength of the atmosphere's glow | 0–100 | 70 |
| Twilight | `Moons` | Number of moons | 0–2 | 2 |
| Ink | `Thickness` | Width of the ink bands against the water | 0–100 | 50 |
| Ink | `Turbulence` | Strength and number of the eddies | 0–100 | 50 |
| Cracktro | `Message` (`REG_SZ`) | The scroller's text, at most 2,000 characters | | §8 |
| Cracktro | `Parts` | Bit mask of the parts: 1 copper, 2 plasma, 4 rotozoomer, 8 tunnel, 16 vector bars | 1–31 | 31 |
| Cracktro | `Shuffle` | Shuffle the parts | 0–1 | 0 |
| Collection | `Duration` | Seconds per saver | 30–600 | 90 |
| Collection | `Savers` | Bit mask: 1 Ink, 2 Twilight, 4 Cracktro, 8 Maze, 16 Quill Ghostwriter | 1–31 | 15 |
| Collection | `Shuffle` | Shuffle the order | 0–1 | 0 |
| Ghostwriter | `TextBank` (`REG_SZ`) | The lines it types, one per line; empty means the built-in bank | | |
| Ghostwriter | `Acts` | Bit mask of the six acts | 1–63 | 63 |

The Collection uses its own common settings for all four scenes, and each scene's own scene settings and seed from that scene's key (SAV-D8).

## 8. The cracktro's default message

> Hello, everyone, and welcome to Afterglow for ReactOS. Greetings fly out to the ReactOS developers, the Wine project, everyone keeping a 486 humming in the spare room, the DOS and Amiga sceners, the folks who recap old motherboards, the archivists who keep the old disks readable, and you, for leaving your machine alone long enough to see this. Afterglow for ReactOS is an unofficial remaster, made by Liminal HQ. Not affiliated with the ReactOS project. ... Copper bars courtesy of trigonometry. Plasma courtesy of more trigonometry. Press any key to return to work. ...

## 9. The settings dialog

A modal dialog, "Afterglow Maze Settings" (the saver's name), built at run time from common controls ReactOS and Windows 95 both have (SAV-D9):

- **Look:** palette (combo box), light mode (check box), speed (trackbar).
- **Quality:** render scale (combo box), detail (combo box), frame cap (combo box).
- **Finish:** bloom, streaks, vignette and grain (trackbars), letterbox and fades (check boxes).
- **Scene:** the scene's own settings: trackbars, combo boxes, a seed edit box, the cracktro's multi-line message editor and part check boxes, the collection's duration and saver check boxes.
- **OK** writes every value and ends; **Cancel** writes nothing; **Defaults** puts the defaults in the controls.

## 10. Quill Ghostwriter

An Easter egg (SAV-D20, SAV-D22): a real `.scr` that appears in the Screen Saver list as "Quill Ghostwriter" wherever it is installed, but it is never selected by default, and the Collection plays it only when its `Savers` mask asks. It ships only with the Quill remix of the ReactOS image and as an optional, unselected component of Quill's own installers, never in the Afterglow recipes, so its crate and file stay separate. Its scene is in [docs/scenes.md](docs/scenes.md#quill-ghostwriter-an-easter-egg). Its own budget is 192 KiB, outside the five main files' 640 KiB.
