# Quill Ghostwriter for Windows 3.x

The findings of the spike for `QUILLGW.SCR`, the Windows 3.x edition of Quill Ghostwriter (SAV-D23, SAV-D37), and the verdict. The behaviour is in `SPEC.md` §10; the code is `platforms/win16` (its own Cargo workspace, sharing nothing with `savers-core`). Facts about Quill are from its `origin/main` at `659a038`.

## Verdict

**Go, for Windows 3.x. No-go, as things stand, for ReactOS.** A Windows 3.x edition can honestly ship: it is a 16 KB file that Windows 3.11 itself lists, configures, tests and starts when idle, in 64K-colour and in plain 16-colour VGA. It is not shipped to ReactOS's `system32`: the Display applet cannot start a 16-bit program (see [ReactOS](#reactos)), so the Quill remix keeps the Win32 `AGQUILL.SCR` alone and this edition stays a component of Quill's own installers (SAV-D24), labelled "Windows 3.x edition".

## Size and the 64 KB budget

| What | Bytes |
| --- | --- |
| `QUILLGW.SCR` | 17 712 (limit 65 536, checked by `platforms/win16/check.sh`); 16 560 before its icon (SAV-D50), whose two images and group add 1 152 |
| Code, one segment (budget 40 000 in `res/quillgw.segments.json`) | 13 392 |
| Data segment: data | 1 964 |
| Data segment: stack | 8 192 |
| Data segment: local heap | 4 096 |
| Data segment in all, of 65 536 | 14 252 (22 %) |
| Imports | 51 |
| Resources | 5: the setup dialog, string 1, and icon group 100 with its 32 and 16 px images |

The text bank (sixteen lines, seven typos) and the scene state are part of the 1 964 bytes of data. The scene is drawn into a memory bitmap that lives in GDI's own memory, not in the data segment.

## What Windows 3.11 does (verified on the owner's tree, in DOSBox-X)

Everything here was seen, not read: the tree at `~/.cache/quill16-play/w31-base`, copied for each run, `QUILLGW.SCR` set up the way the Desktop applet does it.

- **Listing.** The Desktop applet lists `*.SCR` and shows the text after `SCRNSAVE :` in the module description, which is the first entry of the non-resident name table. `SSSTARS.SCR`'s is `SCRNSAVE : Starfield Simulation`. `QUILLGW.SCR` shows as "Quill Ghostwriter" (`previews/win16-w31-desktop-applet.png`). The files are searched for in the Windows system folder.
- **Arguments.** A saver that logged its command line shows what Windows passes: the Setup button runs `QUILLGW.SCR /c` and the Test button runs `QUILLGW.SCR /s`, as does the idle timer. Both are lower case with a slash; there is no `/p` and no window handle. (`/c:<hwnd>` and `/p <hwnd>`, which ReactOS and Windows 95 pass, are also handled: the first is read as `/c`, the second does nothing.)
- **Where the choice is kept.** `SYSTEM.INI` `[boot]` `SCRNSAVE.EXE=`, and `WIN.INI` `[windows]` `ScreenSaveActive` and `ScreenSaveTimeOut` in seconds. The saver's own settings are in `CONTROL.INI`, `[Screen Saver.Quill Ghostwriter]`. The applet's Delay box counts minutes; the tree's own 120 seconds showed there as 2 and a value of 60 as 1.
- **Idle start.** With those set and a delay of one minute, Windows 3.11 started the saver on its own about a minute after the last input (`previews/win16-w31-idle-start-64k.png`, `-idle-typing-64k.png`).
- **Setup dialog** from the Setup button (`previews/win16-w31-setup-dialog.png`) and the saver from the Test button (`-test-button.png`).
- **16 colours.** On a plain VGA (`VGA.DRV`, 640 × 480) the saver chooses the sixteen pure colours; the squiggle, the suggestion box and the "Save the changes?" box are in `previews/win16-w31-vga16-*.png`.
- **Exports.** `SCREENSAVERPROC` and `SCREENSAVERCONFIGUREDIALOG` are exported by name, as the stock savers do. The packer numbers them in alphabetical order (1 is the configure dialog), where the stock savers have the window procedure first; nothing in Windows 3.11 was seen to care.

## Wine

On its own Xvfb under Wine 11.21 (`tools/run-wine.sh`): `/s` runs the whole scene (typing, squiggle, suggestion, correction, Save box, page change); `/c` shows the dialog, and clicking OK writes `Acts=` and `Speed=` to `CONTROL.INI`; a mouse move or a click ends the saver. Wine runs it through its own `winevdm.exe`. One bug showed only here and is fixed: the deactivation that destroying the window brings called `DestroyWindow` again, and the 16-bit task crashed. The window is now destroyed once.

## ReactOS

From ReactOS 0.4.16's source (`dll/cpl/desk/screensaver.c`) and a run in the Quill remix image under QEMU (`tools/guest-reactos.py`):

- **The applet's list** searches `*.scr` in the system folder, the Windows folder and its own, and loads each with `LoadLibraryEx(..., LOAD_LIBRARY_AS_DATAFILE)` to read string 1 for the name. For a 16-bit file that fails, so the entry is **"Unknown"** (`previews/win16-reactos-display-unknown.png`). `QUILLGW.SCR` carries string 1 as `Quill Ghostwriter` anyway.
- **Starting it.** The applet's Preview runs `file /s`, Settings runs `file /c:<hwnd>`, and the mini preview runs `file /p <hwnd>`, each through `CreateProcess`. For a 16-bit NE file ReactOS's own NTVDM takes over and stops with "NTVDM Subsystem: An unrecoverable failure happened from start-app BOP, exiting DOS" (`previews/win16-reactos-ntvdm-failure.png`); the saver never runs, whether it is named `.SCR` or `.EXE` and started from `cmd` or from the applet. That happens before the command line matters.
- **otvdm.** The remix image carries otvdm in `C:\Program Files\otvdm`. Started from the guest's script as `otvdmw.exe` or `otvdm.exe` with `QUILLGW.EXE /s` (and `.SCR`), nothing appeared on screen and the saver wrote no command-line log, so **it was not shown to run**. The cause was not isolated (the script's `start` may not have launched it, or otvdm may not have loaded the file); Quill's own `QUILL.EXE` does run through `otvdmw.exe` from a shortcut. The saver already survives the hand-off's known fault (`w16::command_line` stops at the PSP length byte, a zero, CR or LF), so if otvdm runs it, `/s` will arrive.
- **So** the applet cannot start a 16-bit `.SCR` on ReactOS, with or without otvdm: it calls `CreateProcess` on the file and has no way to name a different launcher. A shortcut could start it through `otvdmw.exe`, but that is not a screensaver. The edition therefore stays out of ReactOS's `system32` (SAV-D23's fallback).

## Not verified

- Real hardware, and a Windows 3.1 retail install (only Windows for Workgroups 3.11 in DOSBox-X); Windows 3.0.
- A 256-colour display driver.
- Real input ending the saver on Windows 3.11 (the harness sends none to the guest); it was shown under Wine (mouse move, click).
- Why otvdm did not run it, and the speed of the scene on a slow 386 (the scene redraws only what changes and copies one bitmap a frame; frame times were not measured).
- The icon in Program Manager and File Manager on Windows 3.11 itself (the resource is checked by `check.sh` and shown under Wine; see below).

## How to run it

```sh
bash platforms/win16/build.sh            # out/QUILLGW.SCR
bash platforms/win16/check.sh            # fmt, clippy, packer tests, build, CPU, branch, size checks (part of make check)
bash platforms/win16/tools/run-wine.sh --ini "Speed=2" out/QUILLGW.SCR PREFIX 10 30
bash platforms/win16/tools/run-w31.sh --saver --ini "Speed=2 Log=1" out/QUILLGW.SCR PREFIX 80 100   # Windows 3.11 in DOSBox-X
bash platforms/win16/tools/run-w31.sh --vga --win 'D:\QUILLGW.EXE /s' out/QUILLGW.SCR PREFIX 30 60  # plain VGA, 16 colours
python3 -I platforms/win16/tools/guest-reactos.py out/QUILLGW.SCR
```

The Windows 3.11 harness copies the tree (`W31_TREE`, default `~/.cache/quill16-play/w31-base`) for every run and starts DOSBox-X in Quill's Docker image on its own Xvfb; the Wine and ReactOS runs use their own Xvfb and QEMU monitor. Screendumps are in `~/Documents/Liminal HQ/Afterglow Savers/previews/win16-*.png`.
