# How ReactOS runs a screensaver

Notes from the ReactOS 0.4.16 sources (tag commit `822e8647`, read from `~/.cache/reactos-src-0.4.16`), with file and function names so each fact can be checked. Windows NT behaves the same way in every point that matters here; differences are noted.

## What a screensaver is

A `.scr` file is an ordinary GUI executable with another extension. ReactOS's own savers (`base/applications/screensavers/3dtext`, `logon`, and the rosapps ones such as `butterflies` and `blankscr`) are built with `set_module_type(<name> win32gui UNICODE)` and `set_target_properties(... SUFFIX ".scr")`, linked to `scrnsave.lib`, and installed to `reactos/system32` (`add_cd_file(... DESTINATION reactos/system32)`). Nothing registers them: the Display applet finds them by searching folders.

Each one carries three resources:

- **String 1**, the description, shown in the Display applet's list (`IDS_DESCRIPTION` in `scrnsave.h`; 3D Text's is "3D Text ScreenSaver").
- **An icon** (`IDI_ICON`, 101 in 3D Text, `ID_APP` in `scrnsave.h`), shown by Explorer and used as the window class icon.
- **Dialog `DLG_SCRNSAVECONFIGURE`** (2003), the settings dialog `scrnsave.lib` opens for `/c`. A saver with its own entry point (as ours are) may build its dialog any way it likes.

## The command line

`sdk/lib/scrnsave/scrnsave.c`, `_tWinMain`, scans the command line one character at a time and acts on the first letter it knows, so `/s`, `-s`, `s` and `/S` are all the same:

| Argument | Who sends it | What it means |
| --- | --- | --- |
| `/s` | Winlogon (`"%s /s"`), the applet's **Preview** button (`"%s /s"`), Explorer's **Test** | Run full screen |
| `/p <hwnd>` | The applet's monitor preview (`"%s /p %Iu"`: a space, then the decimal window handle) | Run as a child of that window. `scrnsave.lib` returns -1 when the handle is not a window |
| `/c:<hwnd>` | The applet's **Settings** button (`"%s /c:%Iu"`) | Show the settings dialog, owned by that window. Without `:<hwnd>`, owned by the foreground window |
| `/a <hwnd>` | Windows 9x only | Change the Windows 9x screensaver password; ignored on NT |
| nothing | Double-clicking the file, Explorer's **Configure** | The settings dialog with no owner |

## How Winlogon starts one

`base/system/winlogon/screensaver.c`:

- A thread waits for the idle timeout from `SystemParametersInfoW(SPI_GETSCREENSAVETIMEOUT)` (seconds), and only when `SPI_GETSCREENSAVEACTIVE` is true. It measures idleness with `GetLastInputInfo` and posts `WLX_SAS_TYPE_SCRNSVR_TIMEOUT` when the time is up.
- `StartScreenSaver` reads `HKCU\Control Panel\Desktop\SCRNSAVE.EXE` (a `REG_SZ` full path), and runs `"<path> /s"` with `CreateProcessAsUserW`, `STARTF_SCREENSAVER`, on `WinSta0\Default`, at **`IDLE_PRIORITY_CLASS`**. The saver gets only the CPU nothing else wants, so a frame loop must sleep rather than spin and must not assume a steady frame rate.
- It then waits for the process to end **or for user activity**. On activity it calls **`TerminateProcess`**: the saver may be killed at any instant, so it must hold no state that needs saving at exit. Settings are written only by the `/c` dialog.
- `ScreenSaverIsSecure` (`"1"`) locks the workstation when the saver ends; that is Winlogon's job, not the saver's (the "FIXME" notes that the secure desktop is not used yet).

## The Display applet

`dll/cpl/desk/screensaver.c`:

- **The list.** `EnumScreenSavers` adds "(None)", then every `*.scr` in the applet's own folder, then `GetSystemDirectory()` (`C:\ReactOS\system32`), then `GetWindowsDirectory()`, skipping hidden files (`SearchScreenSavers`). For each it calls `LoadLibraryEx(path, NULL, DONT_RESOLVE_DLL_REFERENCES | LOAD_LIBRARY_AS_DATAFILE)` and `LoadString(hModule, 1, ...)`; with no string 1 it shows the file name without `.scr`, and when the file cannot be loaded at all, "Unknown". `MAX_SCREENSAVERS` caps the list.
- **The preview.** Choosing an entry kills the previous preview process with `TerminateProcess`, then runs `"<path> /p <hwnd>"` with `CreateProcess` and no quoting: a path with spaces would break, so the files have 8.3 names in `system32` (SAV-D1). The preview window is the small monitor picture in the dialog (about 152 × 112 pixels at 96 DPI). The process is killed, never asked to close, when the selection changes or the dialog closes.
- **Settings.** `ScreenSaverConfig` runs `"<path> /c:<hwnd>"` and then pumps the applet's own messages until the process ends (`WaitForSettingsDialog`), so the dialog should be modal to that owner and end the process when it closes. It also kills the preview while the settings are open and restarts it afterwards, so a changed setting shows in the preview at once.
- **Apply.** `SetScreenSaver` writes `SCRNSAVE.EXE` (or deletes it for "(None)"), calls `SystemParametersInfo(SPI_SETSCREENSAVEACTIVE, ...)`, writes `ScreenSaverIsSecure` as `"0"` or `"1"`, and sets the timeout with `SPI_SETSCREENSAVETIMEOUT` in seconds (the dialog's minutes × 60, at least 1 minute).

So the registry values a preset image needs are, under `HKCU\Control Panel\Desktop` (or the default user hive): `SCRNSAVE.EXE` = the full path, `ScreenSaveActive` = `"1"`, `ScreenSaveTimeOut` = seconds as a string, `ScreenSaverIsSecure` = `"0"`.

## What `scrnsave.lib` does while it runs

`sdk/lib/scrnsave/scrnsave.c`, which our shell follows (SAV-D10):

- **The window.** Full screen: class `WindowsScreenSaverClass`, `WS_POPUP | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN`, `WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, covering the virtual screen (`SM_XVIRTUALSCREEN`, `SM_YVIRTUALSCREEN`, `SM_CXVIRTUALSCREEN`, `SM_CYVIRTUALSCREEN`), black background brush, no cursor. Preview: `WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN` over the parent's client rectangle.
- **Ending.** `WM_ACTIVATE`, `WM_ACTIVATEAPP` or `WM_NCACTIVATE` with `wParam` false (losing focus), any key (`WM_KEYDOWN`, `WM_KEYUP`, `WM_SYSKEYDOWN`) and any button (`WM_LBUTTONDOWN`, `WM_MBUTTONDOWN`, `WM_RBUTTONDOWN`, `WM_XBUTTONDOWN`) post `WM_CLOSE`. **Mouse moves** close it only once the cursor is more than **10 pixels** in x or y from where it was at the first `WM_MOUSEMOVE`, which absorbs the move Windows sends when the window appears.
- **`WM_SETCURSOR`** sets no cursor.
- **`WM_SYSCOMMAND`** with `SC_CLOSE`, `SC_NEXTWINDOW`, `SC_PREVWINDOW` or `SC_SCREENSAVE` is refused while full screen, so Alt+Tab and a second saver cannot start over it (the key press still ends the saver).
- **Preview mode** skips all of this: input does not end it, and it ends when its process is killed or its parent goes away.
- **No password code.** ReactOS's library has no Windows 9x password support (it says so); NT-style systems lock through Winlogon.

## Consequences for the Afterglow savers

- Run at idle priority and be killable at any instant: nothing to save on exit, no temporary files.
- Keep `/p` cheap: the preview runs in a tiny window beside a dialog the user is working in, so it renders small, at a low frame rate, and checks that its parent still exists.
- Put string 1 and an icon in every file; build the settings dialog at run time (SAV-D9).
- Name the files in 8.3 form with no spaces (SAV-D1).
