# Quill Ghostwriter for Windows 3.x

The brief for `QUILLGW.SCR`, the Windows 3.x edition of Quill Ghostwriter (SAV-D23), built with the same Win16 toolchain as Quill. Facts are from Quill's `origin/main` (`659a038`; the local `~/source/experiments/quill16` checkout is on a stale branch, so read Quill with `git show origin/main:...`), liminal-installer's `retro/win16`, rust-win16, and the stock savers in the Windows 3.1 test tree.

## The toolchain (copy, with provenance)

- **Build.** As Quill's `win16/build.sh`: `RUSTC_BOOTSTRAP=1 cargo build --release` for the target `win16.json` (`i386-unknown-none-code16`, soft float, panic abort), `core` and `alloc` from source, linked with Quill's `link.ld`, then packed by `tools/ne_pack.py --module QUILLGW --description "SCRNSAVE : Quill Ghostwriter" ... out/QUILLGW.SCR`, then `check_segments.py`, `check_cpu.py` and `check_resources.py`.
- **Copy Quill's `origin/main` packer**, not rust-win16's: only Quill's adds `icons` (RT_GROUP_ICON), `bitmaps` and `dialog_files`. Both descend from rust-win16 `feat/multi-segment` `4c0bac4`; this edition gets its own `platforms/win16/PROVENANCE.md` naming Quill's commit and listing what changed.
- **Its own workspace** at `platforms/win16`, with its own `rust-toolchain.toml` if Quill pins a different one, sharing nothing with `savers-core` (which is 32-bit and floats freely); the scene is re-done in 16-bit integer GDI.
- **Imports** are rows in `tools/imports.tbl` (`MODULE ORDINAL Name ARGS RET`), turned into `src/imports.rs` by `gen_imports.py`. Quill's table has `SetTimer`, `KillTimer`, `GetPrivateProfileString` (KERNEL 128) and `WritePrivateProfileString` (KERNEL 129). Add, with their ordinals from the Windows 3.1 SDK's import libraries: `ShowCursor`, `SetCapture`, `GetCursorPos`, `SetWindowPos`, `GetSystemMetrics`, `PostMessage`, `DefWindowProc` if missing, and any GDI calls the scene needs (`PatBlt`, `CreateSolidBrush`, `SelectObject`, `TextOut`, `SetTextColor`, `SetBkMode`, `Rectangle`, `MoveTo`, `LineTo`).
- **Entry and window procedure** as Quill's `main.rs`: `#![no_std] #![no_main]`, `w16_main(hinst, hprev, show)`, the procedure exported with `export_proc!` into segment 1 (the packer's rule), the plain `GetMessage` loop.
- **Segments.** One code segment is enough for a small saver; budget it in `segments.json` (aim under 48 KB of code so the file stays well under 64 KiB with resources).

## Windows 3.1 screensaver conventions

What the stock savers show (in the test tree): `SSSTARS.SCR`'s nonresident name 0 is `SCRNSAVE : Starfield Simulation`, `SCRNSAVE.SCR`'s is `SCRNSAVE : Blank Screen`, and they export `SCREENSAVERPROC`, `SCREENSAVERCONFIGUREDIALOG` and the password dialogs. So:

- **The description** (nonresident name 0) is `SCRNSAVE : Quill Ghostwriter`; Control Panel's Desktop dialog lists `*.SCR` in the Windows folder by the text after the colon.
- **Exports:** export `SCREENSAVERPROC` (the window procedure) and `SCREENSAVERCONFIGUREDIALOG` (the setup dialog procedure) by those names, as the stock savers do, in case Control Panel looks for them.
- **To verify first** (the repositories do not record them): the exact arguments Control Panel passes for Setup and Test (expected `/c` and `/s`, either case, `-` or `/`), and where the choice is stored (expected `SYSTEM.INI` `[boot]` `SCRNSAVE.EXE=`, with `WIN.INI` `[windows]` `ScreenSaveActive` and `ScreenSaveTimeOut`; the tree's `WIN.INI` has `ScreenSaveActive=0` and `ScreenSaveTimeOut=120`). The first spike is a saver that writes its command line to a file and is picked in Control Panel. Windows 3.1 has no preview window (no `/p`).
- **Running:** a topmost full-screen popup (`WS_POPUP`, the screen size from `GetSystemMetrics`), cursor hidden with `ShowCursor(FALSE)`, a `SetTimer` tick (55 ms at best) driving the scene, and the same exit rules as SPEC §4: keys, buttons, deactivation, and mouse moves beyond a few pixels.
- **Settings** in `CONTROL.INI` `[Screen Saver.Quill Ghostwriter]` through `GetPrivateProfileString` and `WritePrivateProfileString`: `Acts` (the same bit mask as the Win32 edition) and `Speed`. The setup dialog is a packer `dialog` resource.

## The scene, reduced

At 640 × 480 in 16 colours (exact RGB where the driver has 256): the Quill window (navy title bar, menu, the page) drifting a few pixels a tick over a dark desk, drawn with GDI rectangles and `TextOut` in the system font; typing from a small embedded bank (a dozen lines, an `rcdata` resource), a swapped-letter typo, a red squiggle drawn with `LineTo`, a suggestion box, the fix, the status bar counting `Ln` and `Col`; now and then the "Save the changes to Untitled before Quill closes?" box with Yes, No and Cancel and a blinking focus rectangle. Redraw only what changes (the line being typed, the caret, the status bar), so a 386 keeps up; the window's drift moves the whole picture by `ScrollDC` or a full repaint every few seconds, so nothing stays put (SAV-D12).

## Testing

- **Wine** on its own Xvfb, as the CI smoke: `/s` for a few seconds and a screenshot, `/c` and the dialog.
- **otvdm**, and on ReactOS a check whether the Display applet can start it at all: ReactOS's own 16-bit hand-off delivers an empty command line (the PSP length byte is 0; Quill's `site/try-reactos/README.md` and SITE-12), so a 16-bit `.SCR` started by the applet may not know it was asked for `/s`. Run through `otvdmw.exe <file> /s` it arrives whole. If the applet cannot run it, the edition is described as "Windows 3.x edition" and kept out of ReactOS's `system32` (SAV-D23).
- **Windows 3.11 in DOSBox-X**, with Quill's harness (`win16/tools/run-w31.sh` and `e2e-*-w31.sh` on Quill's `origin/main`): DOSBox-X in Docker (`quill16-dosbox-x:local`, present) on Xvfb, the program on D:, started from `WIN.INI` `run=`. The tree is `W31_TREE`, default `~/.cache/quill16-play/w31-base`, which **exists** on the development machine (built from `~/Downloads/win31.zip`). For a saver: copy it into the tree's `WINDOWS`, select it in `SYSTEM.INI`, set a one-minute timeout, and screenshot it starting on its own; then Control Panel's Desktop dialog listing it and its Setup dialog.

## Work package

**WP-win16** (area `win16`, Sonnet with an Opus review): `platforms/win16` with the copied toolchain and provenance; the command-line spike under Windows 3.11; the saver and its setup dialog; Wine, otvdm, DOSBox-X Windows 3.11 and ReactOS checks with screenshots to `~/Documents/Liminal HQ/Afterglow Savers/previews/win16-*.png`; `SPEC.md` gains a Windows 3.x section; size under 64 KiB checked by `make check`.
