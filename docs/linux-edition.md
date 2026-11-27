# The Linux edition

Afterglow Twilight on the graphics card for Linux (SAV-D25, SAV-D42 to SAV-D46): a program, `afterglow-twilight-gpu`, that runs as a fullscreen saver on X11 or Wayland, as an xscreensaver hack, or in a window. It is the `platforms/linux` crate, built over `crates/savers-gpu` (the wgpu renderer) and `savers-core`'s synth; the Win32 shells and the `no_std` CPU path are untouched.

## Install

```sh
bash platforms/linux/install.sh        # builds with cargo, installs under ~/.local; --prefix DIR, --no-build, --uninstall
```

It puts `afterglow-twilight-gpu`, `afterglow-twilight-xscreensaver` and `afterglow-twilight-idle` in `~/.local/bin`, the launcher in `~/.local/share/applications`, the xscreensaver XML in `~/.local/share/afterglow-savers/xscreensaver`, an unenabled systemd user unit in `~/.config/systemd/user`, and writes the settings file. No sudo, no system folders. Make sure `~/.local/bin` is on `PATH`. Needs the Vulkan loader and a driver (Mesa's RADV, ANV or lavapipe) or GL, and libasound for sound; X11 and Wayland libraries are opened when needed, so one binary serves both.

## Modes

| Command | What it does |
| --- | --- |
| `afterglow-twilight-gpu` | The saver: borderless and fullscreen at the primary monitor's native size, cursor hidden, ends on any key, any button or more than 10 pixels of pointer movement |
| `--window 1280x720` | A normal window to test in; a key or closing it ends the run (`--exit-on-input` adds pointer input) |
| `--preview` | A 320 × 180 window, Low detail, no grain, silent, as a settings dialog shows it |
| `--window-id 0x…`, `-root` | A child of an existing X11 window, as xscreensaver runs a hack; ignores input; silent unless `--sound`; always X11, even in a Wayland session |
| `--shot out.png --at 40 --size 3840x2160` | No window: one frame to a PNG |
| `--bench [--size WxH] [--frames N]` | No window: milliseconds a frame at 1080p and 4K (or `--size`) at the settings' render scale |
| `--audio-wav out.wav --seconds 60` | No window, no sound card: the score to a 48 kHz WAV |
| `--adapters`, `--adapter NAME` | List the graphics adapters; choose one by part of its name (`radv`, `llvmpipe`) |
| `--write-config` | Write the commented settings file if there is none (`--force` overwrites) and print its path |

`--frames N` ends after N frames, `--at S` freezes the scene at S seconds (the tests use both), `--verbose` says what was chosen and logs the frame rate every 30 frames.

## Settings

The same table as the Win32 saver (SPEC section 7), by the registry value names, in `$XDG_CONFIG_HOME/liminal-hq/afterglow-savers/twilight.toml` (`~/.config/…` by default). The file is a flat subset of TOML: `Name = value`, `#` comments, a `[twilight]` line ignored, a value a number, a quoted string or a choice's label (`Palette = Aurora`, `Detail = Ultra`, `RenderScale = 75`). A line that does not parse is skipped, and a value out of range is the field's default. `--write-config` writes every setting at its default with the range beside it.

Command-line flags win over the file: `--set NAME=VALUE` for any setting, and `--palette`, `--speed`, `--detail`, `--scale`, `--fps`, `--volume`, `--seed`, `--sun`/`--no-sun`, `--sound`/`--no-sound`. A bad name or value is an error before anything opens.

Two things differ from Windows (SAV-D43):

- **This edition's defaults** are `RenderScale` 100, `Detail` Ultra (3) and a 60 frames a second cap, since the card does what the Pentium cannot. Everything else is the Win32 default, including `Sound` on at `Volume` 28.
- **`FrameCap`** (0, or 10 to 240; 0 follows the display's refresh with vsync) takes the place of `MaxFps`, which the Win32 table limits to 10 to 60. With a cap the swap chain uses Mailbox when it has it (no tearing, no waiting on a slow refresh) and the program sleeps to the next frame; without one it uses Fifo.

**Render scale:** the frame is drawn at `RenderScale` percent of the display and the presenter enlarges it with a bilinear filter; 50% at 4K draws 1920 × 1080. The finish (glow, streaks, vignette, grain) runs at the output size the detail level allows (4096 wide at Ultra), so grain stays fine.

## Ending, and the hack

A saver ends on a key press, a button press, or a pointer more than 10 pixels from where it was when the window had settled (the reports in the first half second only set that place, as some are stale). Closing the window ends any mode. A hack in xscreensaver's window ignores all input, because xscreensaver ends it (SAV-D44).

## xscreensaver

Add this to `programs:` in `~/.xscreensaver` (the installer prints it with the real path):

```text
"Afterglow Twilight"  afterglow-twilight-xscreensaver -root --palette Aurora  \n\
```

xscreensaver runs the command with `-root` and `XSCREENSAVER_WINDOW` set, or with `-window-id ID` in its settings preview; the wrapper script unsets `WAYLAND_DISPLAY` and runs the program, which draws into that window. Options after `-root` are the program's own (`--help`). The hack is silent unless `--sound` is given.

`afterglow-twilight.xml` is the settings dialog. xscreensaver reads its XML from its own config folder (usually `/usr/share/xscreensaver/config/`), which needs administrator rights, so the installer leaves that one copy to you. Copy it as `afterglow-twilight-xscreensaver.xml`: xscreensaver finds a dialog by the command's program name, not the saver's. Without it the hack still runs, just with no dialog. The packages planned in [linux-packages.md](linux-packages.md) install it in place (SAV-D64). The controls map to options as follows:

| Control | Option |
| --- | --- |
| Palette (Dusk, Indigo, Ember, Aurora, Graphite) | `--palette NAME` |
| Speed, 25 to 200 | `--speed N` |
| Detail (Low to Ultra) | `--detail NAME` |
| Draw at full size, 75% or 50% | `--scale N` |
| Frames a second (30, 60, 120, vsync) | `--fps N` |
| Glow | `--set Bloom=N` |
| Film grain | `--set Grain=N` |
| Hide the sun | `--no-sun` |
| Play sound | `--sound` |
| Volume | `--volume N` |

## GNOME, KDE and other desktops

Neither GNOME's lock screen nor KDE's screen locker has an interface for a third-party saver, and neither runs xscreensaver hacks. What works:

- **Any desktop, X11 or Wayland: `afterglow-twilight-idle`**, a small shell loop. It asks how long the session has been idle (GNOME's Mutter IdleMonitor over D-Bus with `gdbus`, else `xprintidle` on X11), skips the saver while something has inhibited idling (a video, a presentation: GNOME's session manager), and past the delay runs the program, which ends itself on input. Run it by hand (`afterglow-twilight-idle 300`), from autostart, or as the installed user unit: `systemctl --user enable --now afterglow-twilight-idle.service`. On GNOME, set Settings > Power > Screen blank and Settings > Privacy > Screen lock to longer than the delay, or the screen goes dark first.
- **sway and other wlroots compositors:** `swayidle -w timeout 300 'afterglow-twilight-gpu' timeout 900 'swaylock'`. The saver needs no `resume` command, since it ends on input.
- **Plain X11 with no desktop:** xscreensaver as above, or `xidlehook --timer 300 'afterglow-twilight-gpu' ''`.
- **KDE Plasma:** the idle helper above on Plasma 6 needs `xprintidle` on X11; on Wayland use `swayidle` if Plasma's idle protocol is on offer (it is on 6), or run `afterglow-twilight-gpu` from a shortcut. A lock-screen wallpaper plug-in is a later package.

## Sound

`savers_core::audio` at 48 kHz, `f32`, stereo, on its own thread, fed by a second, tiny Twilight scene that only supplies the score's cues (nothing is drawn by it); a bounded queue of eight blocks (about 340 ms) sits between it and the card's callback. cpal opens the default ALSA device (PipeWire and PulseAudio both answer to it), asks for 48 kHz 32-bit float, and takes the device's own rate if it will not do 48 kHz (the synth is made for whichever it is). A gap in the queue holds the last frame and fades it over 20 ms, and the end of the run fades over 150 ms, so neither clicks. With no device, or one that will not open, the run is silent and says so with `--verbose` (SAV-D45).

Sound is on by default at `Volume` 28 (SAV-D35), and stays off in `--preview`; **as a hack it is off unless `--sound` is given**, because xscreensaver hacks are expected to be silent. `--no-sound`, `Sound = 0` or `Volume = 0` keep it off anywhere.

## Testing

`make check-gpu` (not part of `make check`: it needs Xvfb, ImageMagick's `import`, `xdotool` and `vulkan-swrast`, and takes about 20 seconds once built) runs five tests, each on its own Xvfb with the lavapipe software Vulkan driver. Nothing touches the desktop session: every command has its own `DISPLAY` and no `WAYLAND_DISPLAY`, the framebuffer is grabbed from that server only, and keys and pointer moves go to that server only. Sound goes to a file through ALSA's file plug-in, and a second run with no ALSA devices at all checks the silent path.

- a 640 × 360 window is not blank and matches the CPU scene's frame at the same time within the parity budget (measured 2.4 levels mean and 0.06% far, against 6 and 2%), and a key ends it with a clean exit;
- fullscreen fills a 1920 × 1080 screen at 50% scale and ends on a key;
- 3 pixels of pointer movement does nothing and 30 ends it;
- the wrapper script draws into a window of xscreensaver's kind, is silent, and ignores a key and movement;
- the captured sound is not silent, has no step over a threshold, and ends at zero.

The unit tests (`make test`) cover the arguments, the settings file and the audio path (levels, clicks, the ramps) with no display at all.

## Numbers

On this machine, a Ryzen 7 7700X (16 threads) and a Radeon RX 7800 XT, release build, Twilight at Ultra, 100% render scale (`--bench`, frames submitted back to back and waited for once):

| Adapter | 1920 × 1080 | 3840 × 2160 |
| --- | --- | --- |
| lavapipe (software Vulkan, 16 threads) | 32.2 ms, 31 fps | 193.6 ms, 5.2 fps |
| lavapipe at 50% render scale | | 36.6 ms, 27 fps |
| Radeon RX 7800 XT (RADV), headless only | 0.30 ms | 1.0 ms |

A 4K fullscreen window on lavapipe at 50% scale held 21 fps, which includes copying every frame to the X server. The stripped binary is 6.5 MB (the unstripped release build, with debug lines, 44 MB; the installer strips). Start-up is the maps' painting on the CPU, about 2 seconds at 4K.

## Not covered

- Only the primary monitor is covered. A second display stays as it was; one window per monitor is planned in [linux-packages.md](linux-packages.md) (SAV-D70 to SAV-D74).
- On Wayland the window mode has no title bar (client-side decorations are left out of winit so a saver does not ship a software-rendered one); fullscreen and the hack do not need one.
- The program has been run on X11 under Xvfb. Wayland, the real Radeon (other than headless), PipeWire, GNOME's idle helper and KDE have not been exercised from here, and nothing was opened on the owner's desktop on purpose.
