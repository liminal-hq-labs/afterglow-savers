# Quill installer integration

How Quill's installers (built by liminal-installer from Quill's `installer/` project) ship Quill Ghostwriter (SAV-D22 to SAV-D24, SAV-D58). This repository does not edit Quill or liminal-installer; this is the plan for whoever does, read against liminal-installer's `SPEC.md` (§4 Components, install-time placeholders, receipts) and `docs/retro-win16.md` and `docs/retro-win32.md`.

## What ships where

| Installer | Component | File | Destination | Selected by default |
| --- | --- | --- | --- | --- |
| Windows 95 (`windows-x86`, the Win32 `SETUP.EXE`) | `ghostwriter` "Quill Ghostwriter screensaver" | `AGQUILL.SCR` (Win32, i586) | `{install}`, with a Start menu shortcut "Quill Ghostwriter" | no |
| Windows 95 | `ghostwriter16` "Quill Ghostwriter screensaver, Windows 3.x edition" | `QUILLGW.SCR` (Win16, NE) | `{install}`, with a shortcut "Quill Ghostwriter (3.x)" | no |
| Windows 3.x (`win16-x86`) | `ghostwriter16` "Quill Ghostwriter screensaver, Windows 3.x edition" | `QUILLGW.SCR` | `{install}`, with a Program Manager item | no |

- **Today's installer only** (SAV-D58). liminal-installer has `default = false` (an optional component offered unticked, honoured by every runtime, with its operations gated on the selection), but its retro runtimes install files below `{install}` only, and a shortcut has no arguments. So the savers go to the install folder, and the shortcut plays them: on Windows 95 a shortcut to a `.scr` opens with the `scrfile` verb (`"%1" /S`), and `QUILLGW.SCR` with no argument runs as `/s`. They are **not** listed in Display Properties or Control Panel > Desktop, which look only in the Windows and system folders; listing them needs `{windows}` and `{system}` in the installer, an owner decision ([packing.md](packing.md#owner-decisions)).
- Program Manager may refuse an item whose file `WIN.INI`'s `Programs=` does not name; if Windows 3.11 does, the Windows 3.x installer keeps the file without the item and `README.TXT` says to copy `QUILLGW.SCR` to the Windows folder to list it.
- Neither installer selects Ghostwriter as the active screensaver: no `SCRNSAVE.EXE` in `SYSTEM.INI` (Windows 3.x and 9x) or `HKCU\Control Panel\Desktop` (NT).
- Settings need nothing at install time: the Win32 edition reads `HKCU\Software\Liminal HQ\Afterglow Screensavers\Ghostwriter` and the Win16 one `CONTROL.INI` `[Screen Saver.Quill Ghostwriter]`, each with defaults in the program.
- **Sizes:** `AGQUILL.SCR` is 171,008 bytes at i586; `QUILLGW.SCR` 17,712. The Windows 95 floppy must still fit both (`verify-disks.sh`); if not, they go on the zip only.
- Uninstall removes the files and the shortcuts, as for any component.

## Wiring the built files into Quill's `installer/`

1. Quill pins this repository in `installer/ghostwriter.pin` (the commit, the source tar's SHA-256, and the sums of `AGQUILL.SCR` and `QUILLGW.SCR`). `installer/tools/fetch-ghostwriter.sh` downloads that tar, checks it, builds `AGQUILL.SCR` with `scripts/build-scr.sh` and `QUILLGW.SCR` with `platforms/win16/build.sh`, and checks both sums; `stage.sh` copies them into `dist/ghostwriter/`. No `.SCR` is committed to Quill.
2. `components.toml` gains:

```toml
[[component]]
id       = "ghostwriter"
name     = "Quill Ghostwriter screensaver"
default  = false
targets  = ["windows-x86"]
files    = [{ from = "dist/ghostwriter/AGQUILL.SCR", to = "{install}/" }]
shortcut = [{ name = "Quill Ghostwriter", target = "{install}/AGQUILL.SCR", in = "start_menu" }]

[[component]]
id       = "ghostwriter16"
name     = "Quill Ghostwriter screensaver, Windows 3.x edition"
default  = false
targets  = ["windows-x86", "win16-x86"]
files    = [{ from = "dist/ghostwriter/QUILLGW.SCR", to = "{install}/" }]
shortcut = [{ name = "Quill Ghostwriter (3.x)", target = "{install}/QUILLGW.SCR", in = "start_menu" }]
```

The work package is WP-PACK-7 in [packing.md](packing.md).

## Checks

- Windows 95 test bed (liminal-installer `docs/win95-testbed.md`): install Quill with both components ticked; both shortcuts play their saver; uninstall removes both files and shortcuts.
- Install with the defaults: neither file is copied.
- Windows 3.11: install with `ghostwriter16`; the Program Manager item plays it (or the README note stands in for it).
