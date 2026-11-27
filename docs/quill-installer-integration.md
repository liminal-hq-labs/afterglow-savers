# Quill installer integration

How Quill's installers (built by liminal-installer from Quill's `installer/` project) would ship Quill Ghostwriter (SAV-D22 to SAV-D24). This repository does not edit Quill or liminal-installer; this is the plan for whoever does, read against liminal-installer's `SPEC.md` (§4 Components, install-time placeholders, receipts) and `docs/retro-win16.md` and `docs/retro-win32.md`.

## What ships where

| Installer | Component | File | Destination | Selected by default |
| --- | --- | --- | --- | --- |
| Windows 95 (`windows-x86`, the Win32 `SETUP.EXE`) | `ghostwriter` "Quill Ghostwriter screensaver" | `AGQUILL.SCR` (Win32, i586) | the system folder: `%WINDIR%\SYSTEM` on Windows 9x, `%WINDIR%\System32` on NT | no |
| Windows 95 | `ghostwriter16` "Quill Ghostwriter screensaver, Windows 3.x edition" | `QUILLGW.SCR` (Win16, NE) | the Windows folder, `%WINDIR%` | no |
| Windows 3.x (`win16-x86`) | `ghostwriter16` "Quill Ghostwriter screensaver" | `QUILLGW.SCR` | the Windows folder, `%WINDIR%` | no |

- The Windows 95 installer offers both, so someone can try the old-school and the new-school saver side by side; each is listed in Display Properties by its own description, "Quill Ghostwriter" (the 3.x edition carries "Windows 3.x edition" in its description when the two would otherwise look the same; SAV-D23).
- Neither installer selects Ghostwriter as the active screensaver: no `SCRNSAVE.EXE` in `SYSTEM.INI` (Windows 3.x and 9x) or `HKCU\Control Panel\Desktop` (NT). It sits in the list for people to find.
- Settings need nothing at install time: the Win32 edition reads `HKCU\Software\Liminal HQ\Afterglow Screensavers\Ghostwriter` and the Win16 one `CONTROL.INI` `[Screen Saver.Quill Ghostwriter]`, each with defaults in the program.
- **Sizes:** the Win32 `AGQUILL.SCR` is about 127 KiB (i586, built for speed; the guard is 384 KiB, SAV-D29); the Win16 edition's size goes here once it is built. The Win16 edition is expected well under 64 KiB, so the Windows 3.x floppy has room.

## What liminal-installer needs

The component model in liminal-installer's `SPEC.md` §4 has three gaps for this:

1. **Unticked by default.** A component that is not `required` is offered ticked. Ghostwriter needs a key such as `default = false` (a component offered unticked), honoured by every runtime's components page and by `default_on_later_disk`'s check.
2. **Windows folders.** Install-time placeholders are `{install}`, `{user_data}`, `{system_data}`, `{start_menu}`, `{desktop}` and `{temp}`. A screensaver must go to `{windows}` (`GetWindowsDirectory`) or `{system}` (`GetSystemDirectory`, which is `SYSTEM` on 9x and `System32` on NT). Both runtimes need them, and the Windows 3.x one needs `{windows}`.
3. **Files outside the install folder in the receipt.** The receipts record files with full paths for directories outside `{install}` already; a file outside it needs the same (`F` records by full path in `INSTALL.LIR`), so uninstalling Quill removes the saver. Uninstall should also, when `SCRNSAVE.EXE` names the removed file, leave the setting alone (Windows falls back to none) rather than edit it.

## Wiring the built files into Quill's `installer/`

1. This repository's builds write `AGQUILL.SCR` (`platforms/win32`, i586) and `QUILLGW.SCR` (the Win16 edition). Quill's build copies them into its installer project's `dist/` at fixed paths, from a pinned copy (a vendored `dist/ghostwriter/` with a `PROVENANCE.md` naming this repository's commit and the files' SHA-256 sums), as Quill already vendors other inputs.
2. `components.toml` gains:

   ```toml
   [[component]]
   id      = "ghostwriter"
   name    = "Quill Ghostwriter screensaver"
   default = false
   targets = ["windows-x86"]
   files   = [{ from = "dist/ghostwriter/AGQUILL.SCR", to = "{system}/" }]

   [[component]]
   id      = "ghostwriter16"
   name    = "Quill Ghostwriter screensaver, Windows 3.x edition"
   default = false
   targets = ["windows-x86", "win16-x86"]
   files   = [{ from = "dist/ghostwriter/QUILLGW.SCR", to = "{windows}/" }]
   ```

3. The components page needs nothing new beyond `default = false`: the description line under each check box can say "A screensaver: Display Properties > Screen Saver".

## Checks

- Windows 95 test bed (liminal-installer `docs/win95-testbed.md`): install Quill with both components ticked; Display Properties lists "Quill Ghostwriter" twice (one per edition) and previews both; uninstall removes both files.
- Install with the defaults: neither file is copied.
- Windows 3.11 (where the owner's tree is available): install with `ghostwriter16`; Control Panel > Desktop lists it; Setup writes `CONTROL.INI`.
