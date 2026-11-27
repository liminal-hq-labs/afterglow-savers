# Linux packages, one window per monitor, and the Cracktro's words

The build-ordered plan for three things the owner asked for on 2026-11-27, planned together because they share the Linux edition, its settings file and its xscreensaver dialog:

1. **GPU Twilight on every monitor** (SAV-D62): one window per monitor, each with its own view, with options to have the same view everywhere, different views, or the primary monitor only, and a seed and a speed per monitor; on the graphics card.
2. **Real Linux packages** (SAV-D61): a `.deb`, an `.rpm` and an Arch `PKGBUILD`, so the saver is an installed choice with its xscreensaver dialog in place and no sudo helper; with what each desktop can and cannot do, sound at 48 kHz, and the graphics and sound dependencies.
3. **The Cracktro's words off by default** (SAV-D60): a setting to turn them on and a field for your own text, in the Win32 dialog and the shared settings table.

The decisions are SAV-D63 to SAV-D76 in [decisions.md](decisions.md), reserved by this plan and refined by the packages that build them. What the Linux edition is today is in [linux-edition.md](linux-edition.md); the renderer is in [gpu-twilight.md](gpu-twilight.md).

## The finish line

- `make packages` builds `afterglow-twilight-gpu_<version>-1_amd64.deb`, `afterglow-twilight-gpu-<version>-1.<dist>.x86_64.rpm` and `afterglow-twilight-gpu-<version>-1-x86_64.pkg.tar.zst` from a `git archive` of the commit, each in its own distribution container, twice, with the same SHA-256 both times; CI does the same on every pull request and keeps the files as workflow artifacts for seven days. Nothing is released, tagged or uploaded anywhere else.
- Installing a package needs no other step for the program, the launcher, the xscreensaver settings dialog, the XFCE and MATE screensaver entry, the idle helper and its (unenabled) user unit; uninstalling removes every file it installed and nothing in anyone's home folder.
- xscreensaver lists "Afterglow Twilight" with its settings dialog: on Fedora by itself; on Debian, Ubuntu and Arch after one per-user command, `afterglow-twilight-setup --xscreensaver`, which needs no administrator rights (SAV-D66).
- `afterglow-twilight-gpu` covers every monitor, each window with its own view by default (`Monitors = Different`), or the same view, or the primary only, with `[monitor.NAME]` tables for a seed, a speed and the look per monitor; on X11 and Wayland; input on any monitor ends it; one monitor draws exactly as today.
- The Cracktro shows no words unless `Scroller` is 1; the Win32 Afterglow and plain dialogs have the check box and the text field, enabled together; the Collection follows the Cracktro's key.
- Every claim about a desktop below is backed by a run in a container on a display of its own, with screenshots in `~/Documents/Liminal HQ/Afterglow Savers/linux/`.

## What works on which desktop

GNOME and KDE have no interface for a third-party screensaver: their lock screens draw their own picture, and neither runs xscreensaver hacks. What a package can make work, and how (SAV-D69):

| Desktop | Session | How the saver starts | Settings | Monitors | Locking |
| --- | --- | --- | --- | --- | --- |
| xscreensaver (LXQt, i3, Openbox, any X11 window manager) | X11 | xscreensaver runs the hack, listed as "Afterglow Twilight": by itself on Fedora (`hacks.conf.d`), after `afterglow-twilight-setup --xscreensaver` on Debian, Ubuntu and Arch | xscreensaver's dialog (the installed XML) and `twilight.toml` | xscreensaver runs one hack per monitor; each picks its own view from where its window sits (SAV-D74) | xscreensaver's own |
| XFCE (`xfce4-screensaver`), MATE (`mate-screensaver`) | X11 | Listed in the screensaver preferences from `/usr/share/applications/screensavers/afterglow-twilight.desktop` | `twilight.toml` only (these have no per-theme dialog) | One process per monitor, as with xscreensaver | Their own |
| GNOME | Wayland (GNOME 49 and later ship without an X11 session by default) | `afterglow-twilight-idle` as a user service: Mutter's IdleMonitor over D-Bus, skipping while the session is inhibited (a video, a presentation) | `twilight.toml` | One window per monitor, fullscreen on each output | GNOME's, at its own timeout, over the saver; set Screen blank and Screen lock longer than the idle delay |
| KDE Plasma 6 | Wayland or X11 | `afterglow-twilight-idle`: `org.freedesktop.ScreenSaver.GetSessionIdleTime`, else `swayidle` (KWin has `ext-idle-notify-v1`), else `xprintidle` on X11 | `twilight.toml` | One window per monitor | KScreenLocker's, at its own timeout |
| sway, Hyprland, niri, river, other wlroots-style compositors | Wayland | `swayidle -w timeout 300 afterglow-twilight-gpu` (or `hypridle`'s `on-timeout`); no resume command, the saver ends on input | `twilight.toml` | One window per output | `swaylock` or the compositor's, on a later timeout |
| Cinnamon, Budgie | X11 or Wayland | `afterglow-twilight-idle` (`xprintidle` on X11); their screensavers run no hacks | `twilight.toml` | One window per monitor | Their own |
| xscreensaver under Wayland | Wayland | Not supported: xscreensaver cannot cover a Wayland session; use the idle helper or `swayidle` | | | |
| Any desktop, by hand | Either | The launcher entry, or `afterglow-twilight-gpu` from a shortcut | `twilight.toml` | One window per monitor | None |

Sound is the same everywhere: the score at 48 kHz, `f32` stereo, through cpal's default ALSA device, which PipeWire answers through `pipewire-alsa` and PulseAudio through the ALSA `pulse` plug-in (SAV-D45). It is on at `Volume` 28 when the saver runs by itself or from the idle helper, off in `--preview`, and off as a hack (xscreensaver, XFCE, MATE) unless ticked or given `--sound`. With several monitors there is one score, the primary view's (SAV-D73).

## Decisions

Each is a row in [decisions.md](decisions.md); the package that builds it refines the row and may change the status. The defaults named here are what the builders take; the swaps are listed under [Owner decisions](#owner-decisions).

- **SAV-D63 — packages and names.** One binary package, `afterglow-twilight-gpu`, version from the workspace (`0.1.0`), release `1`; the `.deb` built on Ubuntu 24.04 (glibc 2.39, `libasound2t64`) so that it installs on Ubuntu 24.04 and later and Debian 13 and later, the `.rpm` on the current Fedora release, the Arch package from a `PKGBUILD`. `x86_64` only; `aarch64` is a later swap.
- **SAV-D64 — the installed files.** Listed under [Install layout](#install-layout). The xscreensaver XML is installed as `afterglow-twilight-xscreensaver.xml`: xscreensaver's dialog looks for `<program>.xml` by the command's first word, so today's `afterglow-twilight.xml` would never be found even when copied by hand. The source file is renamed to match, and `install.sh` follows.
- **SAV-D65 — the build pipeline.** Each package is built from a `git archive` of the commit (the same tree GitHub's source tar holds), in a container of its distribution, with Rust 1.98.0 from rustup (checked by SHA-256), `cargo fetch --locked` then an `--offline` build, `SOURCE_DATE_EPOCH` from the commit's time, the source and registry paths remapped, no debug-info package, and a stripped binary. A build is done twice in fresh folders and the two packages must be byte-identical. CI keeps the packages as artifacts; nothing is published.
- **SAV-D66 — xscreensaver's list.** A package must not edit another package's configuration file, and xscreensaver has no folder for third-party hacks except Fedora's `hacks.conf.d` (with `update-xscreensaver-hacks`). So Fedora's package drops `afterglow-twilight.conf` there and runs the tool in `%post`, `%postun` and a trigger on `xscreensaver-base`; Debian, Ubuntu and Arch get `afterglow-twilight-setup --xscreensaver`, a per-user command that adds the one `programs:` line to `~/.xscreensaver` (creating it from the system list when absent) and `--remove` takes it out. No maintainer script touches a home folder.
- **SAV-D67 — dependencies.** The program links only `libc`, `libgcc_s`, `libm` and `libasound`; it opens the Vulkan loader, EGL, X11, XCB, Xcursor, Xrandr, Xi, xkbcommon and Wayland libraries at run time, which no shared-library scan sees. So the packages depend on them by name (table under [Dependencies](#dependencies)), recommend a software Vulkan driver (lavapipe) and the PipeWire ALSA plug-in, and suggest xscreensaver, xfce4-screensaver, swayidle and xprintidle.
- **SAV-D68 — on a software driver.** When the chosen adapter is a CPU device (lavapipe, llvmpipe) and the settings file and flags leave them unset, the program draws at `RenderScale` 50, `Detail` High and a `FrameCap` of 30, and says so with `--verbose`; anything the user sets wins. lavapipe at 4K Ultra is 5 frames a second (linux-edition.md); at these settings 1080p holds about 30, and two monitors share the CPU.
- **SAV-D69 — desktops.** The table above. The XFCE and MATE entry is a `.desktop` file in `/usr/share/applications/screensavers/` running the hack wrapper with `-root`; the idle helper's user unit is installed and never enabled by a package (`systemctl --user enable --now afterglow-twilight-idle.service`, or `afterglow-twilight-setup --idle 300`).
- **SAV-D70 — one window per monitor.** A Linux-only `Monitors` setting: `0` Same (one view everywhere), `1` Different (the default), `2` Primary (today's behaviour). `--monitors same|different|primary` overrides it. Replaces SAV-D44's primary-only line.
- **SAV-D71 — different views.** The primary monitor is view 0 and the rest follow left to right, then top to bottom. View 0 has the saver's seed and no offset; view `i` has `seed ⊕ splitmix(i)` (a new planet, moons and satellites) and starts `i × total_length() / n` seconds into the shot table (another shot, so another camera path). `[monitor.NAME]` (the output's name, as `--list-monitors` prints it, such as `DP-1`) or `[monitor.N]` (the view's number) may set `Seed`, `Speed`, `ViewOffset` (seconds), `Palette`, `Light`, `Detail`, `RenderScale`, `Sun` and `Moons` for one monitor. In Same mode every window has view 0's seed and time, drawn at its own size.
- **SAV-D72 — the settings file grows sections.** `[twilight]` (or no section) holds the shared values; `[monitor.NAME]` and `[monitor.N]` override them. Quoted strings may hold `#` and the escapes `\"`, `\\` and `\n`. A section the program does not know is skipped, as an unknown line is today. Flags: `--monitors MODE`, `--monitor NAME:KEY=VALUE` (repeatable), `--list-monitors`.
- **SAV-D73 — one program, many windows.** One wgpu device for every window (the adapter the first surface accepts), a surface, renderer and presenter per window, and the maps painted for each view on its own thread before the first frame. One score, view 0's. Any key or button in any window ends the saver; pointer movement is measured in screen coordinates where the window system gives them (X11) and per window where it does not (Wayland). With more than one window the swap chains use Mailbox where offered, else Fifo, and the frame cap paces them. On X11 the monitors come from RandR's monitor list through x11rb (winit lists CRTCs only, so it misses RandR 1.5 monitors, tiled displays and Xvfb's), and each window is placed and sized on its monitor explicitly; on Wayland they come from winit and each window asks to be fullscreen on its output.
- **SAV-D74 — the hack on several monitors.** xscreensaver, XFCE and MATE run one hack process per monitor, each in its own window. In Different mode the hack finds which RandR monitor holds its window's centre and takes that view's seed, offset and `[monitor.…]` table, so the monitors differ as they do in the fullscreen saver; Same mode gives every process view 0; Primary has no meaning there (xscreensaver blanks the others itself) and behaves as Different. Only the process on the primary monitor plays sound, when sound is on.
- **SAV-D75 — the Cracktro's words.** `Scroller` (`REG_DWORD`, 0 or 1, default 0), a Cracktro scene setting. Off, the scroller strip is neither rendered nor drawn; on, `Message` scrolls, the built-in text when empty. The Win32 dialogs (Afterglow and plain) show a "Show the scrolling words" check box above the text field, and the field is enabled only while it is ticked (`EnableWindow`, added to `imports.allow`: Windows 95 exports it). The Collection's Cracktro part reads the Cracktro's key (SAV-D8), and the dialog's banner follows the setting (SAV-D49), so by default neither shows words. There is no Cracktro on Linux: the Linux side is the key in the shared table, `savers-preview --set Scroller=1`, and the settings file's quoted strings (SAV-D72) ready for a Linux Cracktro.
- **SAV-D76 — test displays.** X11 tests run on an Xvfb of their own with two RandR monitors made by `xrandr --setmonitor` on that display; Wayland tests run inside a container on a headless sway with two outputs (`WLR_BACKENDS=headless`, `WLR_HEADLESS_OUTPUTS=2`), captured with `grim`; xscreensaver, xfce4-screensaver and the packages are tested in containers on an Xvfb inside the container. Every command on the host has `DISPLAY` set explicitly and `WAYLAND_DISPLAY` unset; inside a container `WAYLAND_DISPLAY` names only that container's own sway socket. No host X or Wayland socket is ever mounted into a container, and nothing is sent to `:0`.

## Install layout

The same on all three, staged by one script (`platforms/linux/packaging/stage.sh DESTDIR`) that the three recipes call, so the file list is written once:

| Path | What |
| --- | --- |
| `/usr/bin/afterglow-twilight-gpu` | The program, stripped |
| `/usr/bin/afterglow-twilight-xscreensaver` | The hack wrapper (on `PATH`, so xscreensaver, XFCE and MATE find it without a hack folder) |
| `/usr/bin/afterglow-twilight-idle` | The idle helper |
| `/usr/bin/afterglow-twilight-setup` | The per-user setup: `--xscreensaver`, `--idle [SECONDS]`, `--remove`, `--status` |
| `/usr/share/xscreensaver/config/afterglow-twilight-xscreensaver.xml` | xscreensaver's settings dialog |
| `/usr/share/xscreensaver/hacks.conf.d/afterglow-twilight.conf` | Fedora only: the `programs:` line for `update-xscreensaver-hacks` |
| `/usr/share/applications/afterglow-twilight.desktop` | The launcher (`Icon=afterglow-twilight`, actions: window, preview, write the settings file) |
| `/usr/share/applications/screensavers/afterglow-twilight.desktop` | XFCE and MATE: `Exec=/usr/bin/afterglow-twilight-xscreensaver -root`, `Categories=Screensaver;`, `OnlyShowIn=XFCE;MATE;` |
| `/usr/share/icons/hicolor/scalable/apps/afterglow-twilight.svg` | From `art/masters/twilight.svg` |
| `/usr/lib/systemd/user/afterglow-twilight-idle.service` | The idle helper's unit, not enabled |
| `/usr/share/man/man1/afterglow-twilight-gpu.1.gz` (and `-idle`, `-setup`, `-xscreensaver` as `.so` links) | Manual pages, written in roff under `platforms/linux/packaging/man` |
| `/usr/share/doc/afterglow-twilight-gpu/` | `README.Linux` (from linux-edition.md's install and desktop sections), Debian's `copyright`, `changelog.Debian.gz` |
| `/usr/share/licenses/afterglow-twilight-gpu/` | `LICENSE-APACHE`, `LICENSE-MIT`, `THIRD-PARTY.txt` (rpm `%license`, Arch; the `.deb` puts them in `copyright`) |

`THIRD-PARTY.txt` lists every crate linked into the binary with its version and licence expression and carries each licence text from the crate's own files: written by `platforms/linux/packaging/third-party.sh` from `cargo metadata --locked` and the vendored sources, with no new tool, and checked against `deny.toml`'s allowed list.

**Uninstall.** Removing the package removes every file above; Fedora's `%postun` reruns `update-xscreensaver-hacks`, so the line leaves xscreensaver's list. What the package never wrote stays: `~/.config/liminal-hq/afterglow-savers/twilight.toml`, the line `afterglow-twilight-setup --xscreensaver` added (xscreensaver shows a hack whose program is missing as not installed) and a user's enabled unit link (systemd ignores a link to a missing unit). `afterglow-twilight-setup --remove` before uninstalling takes out the two the setup made; the manual page and `README.Linux` say so.

## Dependencies

| | Debian and Ubuntu | Fedora | Arch |
| --- | --- | --- | --- |
| Linked | `libc6`, `libgcc-s1`, `libasound2t64` (from `dpkg-shlibdeps`) | `alsa-lib` (automatic) | `glibc`, `gcc-libs`, `alsa-lib` |
| Opened at run time | `libvulkan1`, `libegl1`, `libx11-6`, `libx11-xcb1`, `libxcursor1`, `libxrandr2`, `libxi6`, `libxkbcommon0`, `libxkbcommon-x11-0`, `libwayland-client0`, `libwayland-cursor0` | `vulkan-loader`, `libglvnd-egl`, `libX11`, `libX11-xcb`, `libXcursor`, `libXrandr`, `libXi`, `libxkbcommon`, `libxkbcommon-x11`, `libwayland-client`, `libwayland-cursor` | `vulkan-icd-loader`, `libglvnd`, `libx11`, `libxcursor`, `libxrandr`, `libxi`, `libxkbcommon`, `libxkbcommon-x11`, `wayland` |
| Scripts | `dbus-bin` (for `gdbus`: `libglib2.0-bin`), `procps` | `glib2`, `procps-ng` | `glib2`, `procps-ng` |
| Recommends | `mesa-vulkan-drivers` (RADV, ANV and lavapipe), `pipewire-alsa \| libasound2-plugins` | `mesa-vulkan-drivers`, `pipewire-alsa` | optdepends: `vulkan-driver`, `vulkan-swrast` (lavapipe), `pipewire-alsa` |
| Suggests | `xscreensaver`, `xfce4-screensaver`, `mate-screensaver`, `swayidle`, `xprintidle` | `xscreensaver-base`, `xfce4-screensaver`, `swayidle`, `xprintidle` | optdepends: `xscreensaver`, `xfce4-screensaver`, `swayidle`, `xprintidle` |

The exact names of the run-time libraries are measured, not guessed: the package builder runs the installed program under `LD_DEBUG=libs` on the container's Xvfb and with `--shot` on lavapipe, and every library opened is in the list. The table is the starting point. NVIDIA's own driver brings its own Vulkan ICD, which satisfies `vulkan-driver` on Arch; on Debian and Fedora the recommended Mesa package is harmless beside it.

## Build pipeline

```text
git archive HEAD ──► afterglow-savers-<version>.tar.gz          (SOURCE_DATE_EPOCH = the commit's time)
      │
      ├─► ubuntu:24.04 (by digest)    rustup 1.98.0 ─► dpkg-buildpackage -b   ─► .deb   ┐
      ├─► fedora:<current> (digest)   rustup 1.98.0 ─► rpmbuild -bb           ─► .rpm   ├─ each twice, compared; then installed,
      └─► archlinux:base-devel (dig.) rustup 1.98.0 ─► makepkg (as `builder`)  ─► .pkg   ┘  tested and removed in a clean container
```

- **Files.** `platforms/linux/packaging/` holds `debian/` (`control`, `rules`, `changelog.in`, `copyright`, `source/format`, `lintian-overrides`), `rpm/afterglow-twilight-gpu.spec.in`, `arch/PKGBUILD` (pointing at GitHub's tar for a commit, with `update.sh COMMIT` filling `_commit` and the sum) and `arch/.SRCINFO`, `stage.sh`, `third-party.sh`, `build.sh DISTRO OUT_DIR` (runs inside the distribution, in CI's container job or under `docker.sh`), `docker.sh DISTRO OUT_DIR` (the local wrapper), `test.sh DISTRO PACKAGE` (install, check, remove, check again, inside a clean container), `xscreensaver.conf`, `screensavers.desktop`, `man/`.
- **Source.** `build.sh` takes the tar, never the working tree: the container mounts the worktree read-only, `git archive` writes the tar into the output folder, and the build extracts it into `/build/afterglow-savers-<version>`. The Arch build uses a generated `PKGBUILD.local` whose `source` is that tar and whose `sha256sums` is its sum; the committed `PKGBUILD` is the GitHub form.
- **Rust.** `rustup-init` at a pinned version, checked by SHA-256, installs 1.98.0 with the minimal profile; `cargo fetch --locked`, then `cargo build --release --locked --offline -p afterglow-twilight-gpu` with `RUSTFLAGS=--remap-path-prefix=/build/afterglow-savers-<version>=/usr/src/afterglow-savers --remap-path-prefix=$CARGO_HOME=/cargo`, `CARGO_BUILD_JOBS=4`. Cargo's network is used in the fetch step only.
- **Debian.** `debhelper-compat (= 13)`; `override_dh_auto_build` runs cargo; `override_dh_auto_install` runs `stage.sh`; `override_dh_installsystemduser:` is empty (debhelper would otherwise enable the user unit for every user, SAV-D69); `override_dh_dwz:` empty; `dh_strip --no-automatic-dbgsym`. `changelog` is generated from `changelog.in` with the version and `SOURCE_DATE_EPOCH`'s date. `dpkg-buildpackage -b -us -uc -d` (cargo is from rustup, not a build dependency). `lintian` must show no error that is not in the overrides file, each override with its reason.
- **Fedora.** `%global debug_package %{nil}`, `%define _build_id_links none`, `--define "_buildhost reproducible"`, `%clamp_mtime_to_source_date_epoch 1`, `%use_source_date_epoch_as_buildtime 1`; `%post` and `%postun` run `update-xscreensaver-hacks` if it is installed, `%triggerin -- xscreensaver-base` runs it when xscreensaver arrives later; `%systemd_user_post` and `%systemd_user_preun` for the unit (presets leave it off). `rpmlint` reported in the pull request.
- **Arch.** `makepkg` refuses root, so the container makes a `builder` user; `SOURCE_DATE_EPOCH` and `BUILDDIR` fixed; `options=(!debug strip)`; `namcap` on the `PKGBUILD` and the package, reported.
- **Reproducibility.** `build.sh` builds into `OUT_DIR/a` and `OUT_DIR/b` with separate target folders and compares the packages' SHA-256; a difference fails with `diffoscope` output if it is installed, else the differing member list. `make packages-repro` runs it for all three; CI does it in each job.
- **Make.** `make packages` (all three, one after another, through `docker.sh`), `make package-deb`, `package-rpm`, `package-arch`, `make packages-test` (`test.sh` for each). Not part of `make check`: they need docker and take minutes.
- **CI.** Jobs `pkg-deb` "Debian package", `pkg-rpm` "Fedora package", `pkg-arch` "Arch package", each with `container:` the distribution image by digest, running `build.sh` (both builds and the comparison), then `test.sh` in a second, clean container step (a `docker run` is not available in a container job, so the test is a separate job `pkg-*-test` with `needs:` and the artifact, in a fresh container of the same image), `lintian`, `rpmlint` or `namcap` output and the sums in the job summary, and `actions/upload-artifact` with seven days' retention. No release, no tag, no package registry.

**Scratch on this machine.** `docker.sh` keeps each distribution's cargo registry in `~/.cache/afterglow-savers/pkg/<distro>/cargo` and its target folders in `~/.cache/afterglow-savers/pkg/<distro>/build`, builds one distribution at a time, checks `df -h /home` before each (stop below 20 GB free: the disk had 22 GB free when this plan was written) and removes that distribution's `build` folder with one literal `rm -r` once its packages are copied out. Images are pulled once and kept; nothing goes in `/tmp` but small files.

## Testing

**Packages** (`test.sh`, in a clean container of each distribution, on that container's own Xvfb at `:99` with `DISPLAY=:99` set on every command):

- install with the distribution's tool (`apt-get install ./…deb`, `dnf install ./…rpm`, `pacman -U`), which must pull every dependency; `afterglow-twilight-gpu --version` matches the workspace version, `--help` and `--adapters` (lavapipe listed);
- `--shot /out/shot.png --at 40 --size 1280x720` on lavapipe is not blank, and `--audio-wav /out/score.wav --seconds 10` is 48 kHz stereo and not silent;
- the file list (`dpkg -c`, `rpm -qlp`, `pacman -Qlp`) equals `stage.sh --list`, and no file is outside `/usr`;
- `LD_DEBUG=libs` of a 60-frame `--window` run on the Xvfb and of the shot: every library opened is provided by a declared dependency;
- the hack: xscreensaver started on that Xvfb, the hack listed (Fedora by itself, the others after `afterglow-twilight-setup --xscreensaver` as an ordinary user), `xscreensaver-command -activate` starts `afterglow-twilight-gpu` (seen in `ps`), and a frame of the Xvfb is not blank; `xscreensaver-settings` opened on the hack shows the Afterglow controls (screenshot of that Xvfb), proving the XML is found;
- XFCE on Debian: `xfce4-screensaver-preferences` on the Xvfb lists "Afterglow Twilight" (screenshot);
- remove the package: no file of it is left, Fedora's xscreensaver list no longer has the line, and `afterglow-twilight-setup --remove` (run before) restored `~/.xscreensaver`.

**Monitors** (WP-GPU-MONITORS on the host, `make check-gpu`, and WP-LINUX-DESKTOP in a container):

- X11: an own Xvfb at 3200 × 1080 with two RandR monitors (`DISPLAY=:NN xrandr --setmonitor A 1920/510x1080/290+0+0 none` and `B 1280/340x720/190+1920+0 none`): two windows, each covering its monitor exactly, different frames in Different mode, the same picture (scaled) in Same mode, one window in Primary mode; a key, a button and 30 pixels of movement each end both; 3 pixels do nothing; one monitor gives a frame byte-identical to today's at the same `--at`.
- Wayland: an Arch container with sway, lavapipe and grim; `WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=2 WLR_RENDERER=pixman sway` with `XDG_RUNTIME_DIR=/run/afterglow` inside the container; the program with `WAYLAND_DISPLAY` set to that sway's socket and `DISPLAY` unset; `grim -o HEADLESS-1` and `-o HEADLESS-2` differ in Different mode; `swaymsg` reports both windows fullscreen on their outputs.
- The hack: two xscreensaver-style parent windows on the two-monitor Xvfb (made by the test with x11rb, as `xvfb.rs` does today), two hack processes, different frames in Different mode, the same in Same.
- Unit tests: the settings file's sections, quoted strings and escapes; the view derivation (seeds and offsets for one to four monitors, ordering by position); the per-monitor merge; flags over the file.

**Scroller** (WP-SCROLL): `make check`; a unit test that with `Scroller` 0 the frame does not depend on `Message` and with 1 it does; that `Scroller` and `Message` round-trip through the store; the goldens that move (`cracktro.txt`, `collection.txt`, and the banner tests if they hash the Cracktro), each named in the commit with why; Wine screendumps of both dialogs with the box off (field greyed) and on, on an own Xvfb; the Pentium and size checks.

**Evidence** in `~/Documents/Liminal HQ/Afterglow Savers/linux/`: `packages/` (the three file lists, the sums of both builds, lintian, rpmlint and namcap output, `shot-<distro>.png`, `xscreensaver-list-<distro>.png`, `xscreensaver-dialog-<distro>.png`, `xfce-list.png`), `monitors/` (`x11-different.png`, `x11-same.png`, `x11-primary.png`, `wayland-HEADLESS-1.png`, `wayland-HEADLESS-2.png`, `hack-two-monitors.png`), `scroller/` (`dialog-v2-off.png`, `dialog-v2-on.png`, `dialog-plain-off.png`, `dialog-plain-on.png`, `cracktro-off.png`, `cracktro-on.png`).

## Risks

| Risk | What happens | Answer |
| --- | --- | --- |
| xscreensaver's list on Debian and Arch | No package-level folder for third-party hacks; a missing `~/.xscreensaver` means the system list, and a file with only one `programs:` entry would hide every stock hack | `afterglow-twilight-setup` inserts into an existing list and, when there is none, writes the system list plus ours; the package test checks both cases (SAV-D66) |
| XML not found | xscreensaver looks for `<program>.xml`; the file today is named for the saver, not the program | Renamed on install and in the source (SAV-D64); the dialog screenshot proves it |
| winit cannot see Xvfb's monitors | winit's X11 back end lists CRTCs; Xvfb has one CRTC, and RandR 1.5 monitors are invisible to it | The program reads RandR's monitor list through x11rb on X11 and places windows itself (SAV-D73); this also covers tiled 5K displays and `xrandr --setmonitor` setups |
| Several swap chains | Fifo on two monitors of different refresh rates can block the loop at the slower one | Mailbox where offered, else Fifo with the frame cap pacing the loop; the test measures both windows' frame rates |
| Cost | Two 4K monitors double the work; the maps take about 2 s each at 4K to paint on the CPU | Maps painted on a thread per view; lavapipe falls back to SAV-D68's settings; `--bench --monitors` reports per window |
| Wayland compositors | A compositor may ignore a fullscreen request for a given output or show the window elsewhere first | Each window asks for its output; the test checks with `swaymsg`; GNOME and KDE are checked by hand by the owner, not from here |
| Lock screens | GNOME and KDE lock over the saver at their own timeout; the saver is never the lock screen | Documented; the idle helper's delay must be shorter than the lock's |
| glibc floor of the `.deb` | Built on a newer system, it would refuse Ubuntu 24.04 | Built on Ubuntu 24.04 (SAV-D63), tested on Ubuntu 24.04 and Debian 13 |
| Reproducibility | Timestamps, build ids, the build host, the registry path and archive order can differ between builds | `SOURCE_DATE_EPOCH`, path remaps, rpm's clamp macros and fixed build host, makepkg's own support, and a two-build comparison in every CI job |
| Static Rust and policy linters | lintian and rpmlint complain about embedded crates and a missing debug package | Overrides with reasons; the third-party licence file carries every crate's licence |
| Disk on this machine | 22 GB free when planned; three release LTO builds and images | One distribution at a time, target folders removed after each, `df -h /home` checked first; stop below 20 GB |
| Dialog height (WP-SCROLL) | One more row may push the Afterglow dialog past 600 pixels | The check box shares the text field's row label; the fallback rule for 800 × 600 stays and is retested |

## Order

```text
WP-SCROLL         core, cracktro, win32         ── on its own, any time
WP-LINUX-PKG      linux packaging, ci           ─┐  in parallel
WP-GPU-MONITORS   linux program                 ─┤
WP-LINUX-DESKTOP  desktops, dialog, evidence    ◄┘  needs both merged
```

At most three builders at once: WP-SCROLL, WP-LINUX-PKG and WP-GPU-MONITORS together, then WP-LINUX-DESKTOP. WP-LINUX-PKG packages the program as it is on `main`; WP-LINUX-DESKTOP rebuilds the packages with the monitors and adds their dialog control, so neither of the first two waits for the other. Each package owns its SAV-D rows in `decisions.md` and its own sections of `SPEC.md` and `linux-edition.md`, so the three in parallel touch different lines.

## WP-LINUX-PKG — the deb, the rpm and the PKGBUILD (afterglow-savers, area `linux`)

**Finish line.** `make packages` builds the three packages of today's program from a `git archive` of HEAD in their containers, twice each with equal sums; `make packages-test` installs each in a clean container, passes every package check above, and removes it cleanly; CI does both on every pull request and uploads the packages for seven days; the XML is found by xscreensaver's dialog; Fedora lists the hack by itself and Debian and Arch after `afterglow-twilight-setup --xscreensaver`.

**Files owned.** `platforms/linux/packaging/` (new), `platforms/linux/xscreensaver/` (the XML renamed to `afterglow-twilight-xscreensaver.xml`, the wrapper, the idle helper unchanged apart from paths), `platforms/linux/setup/afterglow-twilight-setup` (new), `platforms/linux/desktop/` (icon line, the screensavers entry), `platforms/linux/systemd/`, `platforms/linux/install.sh` (the renamed XML), `Makefile` (the package targets), `.github/workflows/ci.yml` (the six package jobs), `docs/linux-edition.md` (Install: a Packages section; the XML paragraph), `docs/testing.md`, `docs/progress.md`, `docs/decisions.md` (rows SAV-D63 to SAV-D67 and SAV-D69), `SPEC.md` section 11 (installed paths, a line).

**Must not touch.** `platforms/linux/src` and `platforms/linux/tests` (WP-GPU-MONITORS), `crates/`, `platforms/win32`, `platforms/win16`, the defaults, the existing CI jobs' steps.

**Tests.** `make check-fast`; `make packages`, `make packages-repro`, `make packages-test`; `bash scripts/check-headers.sh`; shellcheck on every new script (`docker run --rm -v "$PWD:/mnt" koalaman/shellcheck:v0.10.0 …`); markdownlint; actionlint; CI green.

## WP-GPU-MONITORS — one window per monitor (afterglow-savers, area `linux`)

**Finish line.** As in the finish line above: `Monitors` Same, Different (default) and Primary; the view derivation of SAV-D71; `[monitor.…]` tables and the three flags; RandR monitors on X11 and winit's on Wayland; the hack's per-monitor view (SAV-D74); SAV-D68's software fallback; `make check-gpu` grows the two-monitor X11 tests; one monitor is byte-identical to today.

**Files owned.** `platforms/linux/src` (`config.rs` sections and strings, `args.rs` flags, `app.rs` windows, a new `monitors.rs`, `render.rs` per-view builds, `audio.rs` only to take view 0), `platforms/linux/tests/xvfb.rs`, `crates/savers-gpu` only if a per-view parameter needs it (a time offset is the caller's, so expected none), `docs/linux-edition.md` (Modes, Settings, Not covered), `docs/gpu-twilight.md` (Linux shells), `SPEC.md` section 11, `docs/decisions.md` (rows SAV-D68, SAV-D70 to SAV-D74, the SAV-D44 note).

**Must not touch.** `platforms/linux/packaging`, `platforms/linux/xscreensaver` (the XML control comes in WP-LINUX-DESKTOP), `crates/savers-core`, the parity budget, the Win32 and Win16 workspaces.

**Tests.** `make check`; `make check-gpu` (the new tests, on own Xvfbs); `cargo nextest run -p afterglow-twilight-gpu` for the unit tests; `--bench` with two monitors in the pull request.

## WP-LINUX-DESKTOP — the desktops, the dialog and the evidence (afterglow-savers, area `linux`)

**Finish line.** The xscreensaver XML gains a "Monitors" choice (same view, different views; `--monitors`); the idle helper gains KDE's `org.freedesktop.ScreenSaver.GetSessionIdleTime` and a `swayidle` path; the packages are rebuilt with the monitors; the Wayland two-output test runs in the Arch container on headless sway; every row of the desktop table that can be run in a container is run and screenshotted (xscreensaver on all three distributions, XFCE's list on Debian, sway's two outputs), and the rows that cannot (GNOME, KDE) say so in `linux-edition.md`'s Not covered.

**Files owned.** `platforms/linux/xscreensaver/`, `platforms/linux/packaging/` (fixes and the Wayland test script `packaging/wayland-test.sh`), `platforms/linux/setup/`, `docs/linux-edition.md` (GNOME, KDE and other desktops: the table; Not covered), `docs/testing.md`, `docs/progress.md`, `docs/decisions.md` (SAV-D69, SAV-D76).

**Must not touch.** `platforms/linux/src` except a fix the desktop runs show (one fix, one commit, with its test), `crates/`, the Win32 and Win16 workspaces.

**Tests.** `make check-fast`; `make packages`, `make packages-test`; `bash platforms/linux/packaging/wayland-test.sh`; markdownlint; CI green.

## WP-SCROLL — the Cracktro's words, off by default (afterglow-savers, areas `core`, `cracktro` and `win32`)

**Finish line.** SAV-D75 built: `Key::Scroller` in the Cracktro's scene fields (default 0), the scene skipping the strip and the scroller when off, the two Win32 dialogs' check box and enabled-together text field, `EnableWindow` in `imports.allow`, the Collection and the banner following, `savers-preview --set Scroller=1`, SPEC section 7 and section 9, the goldens that move.

**Files owned.** `crates/savers-core/src/settings.rs`, `crates/savers-core/src/scenes/cracktro.rs`, `crates/savers-core/tests/golden/` (only the files that move), `crates/savers-preview/src/main.rs` (`--set`, if it lacks one), `platforms/win32/src/dialog.rs`, `dialog_v2.rs`, `template.rs`, `template_v2.rs`, `platforms/win32/imports.allow`, `docs/settings-dialog.md`, `docs/scenes.md`, `SPEC.md` sections 7 and 9, `docs/decisions.md` (SAV-D60 and SAV-D75).

**Must not touch.** `platforms/linux`, `platforms/win16`, any other scene, the score, the size guards (they must still pass), the other defaults.

**Tests.** `make check`; the unit tests named above; `bash scripts/smoke-wine.sh`; Wine screendumps of both dialogs off and on on an own Xvfb.

## Owner decisions

The default is in place; the file named swaps it.

| Question | Default taken | Where to swap |
| --- | --- | --- |
| The package's name | `afterglow-twilight-gpu`, as the program | `Package:` in `packaging/debian/control`, `Name:` in the spec, `pkgname` in the `PKGBUILD`, the doc folder in `stage.sh` |
| Where the `.deb` is built | Ubuntu 24.04, so one file installs on Ubuntu 24.04+ and Debian 13+ | The image digest in `packaging/build.sh` and the `pkg-deb` job |
| Listing in xscreensaver on Debian, Ubuntu and Arch | A per-user `afterglow-twilight-setup --xscreensaver`; no package edits xscreensaver's own configuration (Fedora lists it by itself) | A `dpkg` trigger rewriting `/etc/X11/app-defaults/XScreenSaver` (against Debian policy, not recommended) |
| The hack wrapper's folder | `/usr/bin`, so every screensaver program finds it on `PATH` | `stage.sh` (Debian and Fedora `/usr/libexec/xscreensaver`, Arch `/usr/lib/xscreensaver`) |
| The idle helper enabled by the package | No: installed, never enabled | A user preset file `/usr/lib/systemd/user-preset/80-afterglow.preset` with `enable afterglow-twilight-idle.service` in `stage.sh` |
| Several monitors by default | Different views | The `Monitors` default in `platforms/linux/src/config.rs` |
| Sound with several monitors | One score, the primary view's | `app.rs` (`audio::start` for view 0) |
| Quality on a software driver | `RenderScale` 50, `Detail` High, `FrameCap` 30 unless set | `Config::for_adapter` in `platforms/linux/src/config.rs` |
| A software Vulkan driver pulled in | Recommended (deb, rpm), optional (Arch) | `Recommends:` in `control`, `Recommends:` in the spec, `optdepends` in the `PKGBUILD` |
| A graphical settings window on Linux | None: xscreensaver's dialog and `twilight.toml` | A later package in `platforms/linux` |
| The Cracktro on Linux | Not built; WP-SCROLL's Linux side is the shared key and the file's quoted strings | A later package bringing the CPU scenes to `platforms/linux` through a CPU-frame presenter |
| The banner when the words are off | Follows the setting: no words in the banner by default | `banner::settings` in `savers-core` sets `Scroller` 1 for the Cracktro |
| `aarch64` packages | Not built | A second architecture in each CI job's matrix |

## Sonnet briefs

### WP-LINUX-PKG (in full)

```text
You are a Sonnet builder on afterglow-savers (remote liminal-hq-labs/afterglow-savers; local checkout
~/source/liminal-hq-labs/afterglow-savers, which you never edit).
Package: WP-LINUX-PKG in docs/linux-packages.md. Read it whole first, then AGENTS.md, docs/landing.md, docs/linux-edition.md,
docs/testing.md, platforms/linux/install.sh, platforms/linux/xscreensaver/*, platforms/linux/desktop/*, platforms/linux/systemd/*,
.github/workflows/ci.yml and the decisions SAV-D61 to SAV-D69.

Set up:
- git -C ~/source/liminal-hq-labs/afterglow-savers fetch
- git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/linux-packages ~/source/worktrees/afterglow-savers/linux-packages origin/main
- Work only in ~/source/worktrees/afterglow-savers/linux-packages.
- df -h /home before every container build; stop and report if it is under 20 GB free.

Do, in this order, one commit per step where it makes sense:
1. Rename platforms/linux/xscreensaver/afterglow-twilight.xml to afterglow-twilight-xscreensaver.xml (xscreensaver finds
   <program>.xml), update install.sh, linux-edition.md and the XML's own comment.
2. platforms/linux/packaging/stage.sh DESTDIR [--list]: the single file list of docs/linux-packages.md "Install layout"
   (--list prints it), with --distro fedora adding hacks.conf.d/afterglow-twilight.conf. Paths are literal; no globs.
   The launcher and unit get /usr/bin in place of @BIN@. The icon from art/masters/twilight.svg. Man pages in
   packaging/man (roff, hand-written, SPDX header as a roff comment). packaging/third-party.sh writes THIRD-PARTY.txt from
   cargo metadata --locked and each crate's licence files.
3. platforms/linux/setup/afterglow-twilight-setup (POSIX sh): --xscreensaver adds the line
   "Afterglow Twilight"  afterglow-twilight-xscreensaver -root  \n\  to programs: in ~/.xscreensaver, idempotently;
   when there is no ~/.xscreensaver, first write the system list (from xscreensaver's app-defaults file) plus ours.
   --idle [SECONDS] enables the user unit (systemctl --user, with a drop-in for the delay), --remove undoes both,
   --status says what is set. It never needs root and never touches anything outside the user's home.
4. packaging/screensavers.desktop for /usr/share/applications/screensavers/ (Exec=/usr/bin/afterglow-twilight-xscreensaver
   -root, Categories=Screensaver;, OnlyShowIn=XFCE;MATE;).
5. packaging/debian/ (debhelper-compat 13; override_dh_auto_build runs cargo; override_dh_auto_install runs stage.sh;
   override_dh_installsystemduser and override_dh_dwz empty; dh_strip --no-automatic-dbgsym; changelog from changelog.in
   dated by SOURCE_DATE_EPOCH; machine-readable copyright; lintian-overrides with a reason on each line).
   packaging/rpm/afterglow-twilight-gpu.spec.in (debug_package nil, _build_id_links none, clamp and buildtime macros,
   %post/%postun update-xscreensaver-hacks if present, %triggerin -- xscreensaver-base, %systemd_user_post/preun).
   packaging/arch/PKGBUILD (GitHub tar at _commit, sha256sums) with update.sh COMMIT and .SRCINFO.
   Dependencies exactly as the table in docs/linux-packages.md, then corrected by what step 7 measures.
6. packaging/build.sh DISTRO OUT_DIR (runs inside ubuntu:24.04, fedora:<current> or archlinux:base-devel, each pinned by
   digest): git archive tar, SOURCE_DATE_EPOCH from the commit, rustup-init pinned and checked by SHA-256, Rust 1.98.0
   minimal, cargo fetch --locked then --offline build with the remap RUSTFLAGS and CARGO_BUILD_JOBS=4, the package built
   twice into OUT_DIR/a and OUT_DIR/b with separate target folders, sums compared (fail with the differing members).
   Arch builds as a non-root builder user from a generated PKGBUILD.local whose source is the local tar.
   packaging/docker.sh DISTRO OUT_DIR: the local wrapper; worktree mounted read-only; cargo registry in
   ~/.cache/afterglow-savers/pkg/<distro>/cargo, build folders in ~/.cache/afterglow-savers/pkg/<distro>/build; never mounts
   /tmp/.X11-unix or any Wayland socket; after copying the packages out, the build folder is removed by the caller with one
   rm -r of its literal path.
7. packaging/test.sh DISTRO PACKAGE, in a clean container: every "Packages" check in docs/linux-packages.md Testing,
   on an Xvfb inside the container with DISPLAY=:99 written on every command and WAYLAND_DISPLAY unset; xdotool and import
   only against :99 inside the container. Measure the run-time libraries with LD_DEBUG=libs and fix the Depends lists.
   Check xscreensaver lists the hack (Fedora by itself; Ubuntu, Debian 13 and Arch after the setup command as a normal
   user, with and without an existing ~/.xscreensaver), that xscreensaver-settings shows the Afterglow controls, that
   xfce4-screensaver-preferences lists it on Debian, and that removal leaves no file. Test the .deb on ubuntu:24.04 and
   debian:trixie.
8. Makefile: packages, package-deb, package-rpm, package-arch, packages-repro, packages-test (not in check).
9. ci.yml: pkg-deb, pkg-rpm, pkg-arch (container: the image by digest; build.sh; sums and linter output to the job summary;
   upload-artifact, 7 days, if-no-files-found: error) and pkg-deb-test, pkg-rpm-test, pkg-arch-test (needs:, download the
   artifact, test.sh in a fresh container of the same image). Actions pinned as the file pins them; persist-credentials:
   false. No release, tag or registry step.
10. Docs: linux-edition.md (a Packages section; the XML paragraph; the desktop table points at linux-packages.md),
    testing.md, progress.md, SPEC.md section 11 (the installed paths in one line), decisions.md rows SAV-D63 to SAV-D67 and
    SAV-D69 (refine, keep the numbers; Assumed unless the owner said).

Rules: Canadian English, real em dashes, sentence case, no hard-wrapped markdown, ship the result not the journey. Tabs
everywhere except YAML. SPDX header on every script, spec, PKGBUILD, desktop, unit, XML and YAML file (# comments, XML
comment, roff comment). No sudo and no installs on the host: containers via docker only. Never pkill or killall; stop only
a PID you started. Never push tags, publish, release or dispatch workflows. One heavy tool at a time.

Gate: make check-fast; make packages; make packages-repro; make packages-test; bash scripts/check-headers.sh; shellcheck
(docker run --rm -v "$PWD:/mnt" -w /mnt koalaman/shellcheck:v0.10.0 <each new script>); markdownlint
(docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2:v0.17.2 "**/*.md"); actionlint
(docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.7).

Evidence to ~/Documents/Liminal HQ/Afterglow Savers/linux/packages/ as listed in docs/linux-packages.md.

Commit: small Conventional Commits (build(linux):, feat(linux):, ci:, docs:), message written to a file with a
single-quoted heredoc and committed with git commit -F <file>; markdown body with bold labels and no headings,
hard-wrapped at about 72; trailer Co-Authored-By: Claude Sonnet <your exact model> <noreply@anthropic.com>. Stage files by
name; never git add -A or git commit -a.

Push only feat/linux-packages and open a pull request (gh pr create, labels enhancement, build, linux): title capitalized
imperative, e.g. "Package the GPU Twilight as a deb, an rpm and a PKGBUILD"; body ## Summary with bold-led bullets,
## Test plan with [x]/[ ] items naming commands, last item "- [ ] Not verified: …". Do not merge. Wait for CI; fix
failures on the branch.

Cleanup when told the branch is merged, one command each with literal paths:
rm -r /home/scott/.cache/afterglow-savers/pkg
git -C /home/scott/source/liminal-hq-labs/afterglow-savers worktree remove /home/scott/source/worktrees/afterglow-savers/linux-packages
git -C /home/scott/source/liminal-hq-labs/afterglow-savers worktree prune
git -C /home/scott/source/liminal-hq-labs/afterglow-savers branch -d feat/linux-packages

Report: the PR URL; the three packages' sums from both builds locally and from CI; installed sizes; the measured
dependency lists; what lintian, rpmlint and namcap said; which desktop rows were run; df -h /home at the end; what was
not run.
```

### WP-GPU-MONITORS

```text
Sonnet builder, afterglow-savers. Package WP-GPU-MONITORS in docs/linux-packages.md (read it whole, AGENTS.md,
docs/linux-edition.md, docs/gpu-twilight.md, platforms/linux/src, platforms/linux/tests/xvfb.rs, crates/savers-gpu/src,
decisions SAV-D44, SAV-D62, SAV-D68, SAV-D70 to SAV-D74).
Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers fetch, then
git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/gpu-monitors ~/source/worktrees/afterglow-savers/gpu-monitors origin/main.
Build: the Monitors setting (Same, Different default, Primary) and --monitors; the sectioned settings file ([twilight],
[monitor.NAME], [monitor.N], quoted strings with \" \\ \n) and --monitor NAME:KEY=VALUE and --list-monitors; monitors from
RandR's monitor list through x11rb on X11 (winit lists CRTCs only) with windows placed and sized explicitly, and winit's
monitors on Wayland with fullscreen per output; one wgpu device, a surface, renderer and presenter per window, maps painted
per view on threads; the view derivation of SAV-D71 (view 0 the primary, others by position; seed xor splitmix(i); offset
i * total_length() / n); one score from view 0; any input on any window ends the run, movement in screen coordinates on X11;
Mailbox where offered with several windows; the hack's per-monitor view from its window's centre (SAV-D74), sound only on
the primary's process; the software-driver defaults of SAV-D68 (Config::for_adapter). One monitor must be byte-identical
to today at the same --at. Tests: unit tests for the file, the flags and the derivation; make check-gpu grows two-monitor
tests on an own Xvfb with xrandr --setmonitor (see Testing in the doc), and a two-parent hack test. Every command
has DISPLAY=:NN set explicitly and WAYLAND_DISPLAY unset; never :0; xdotool and import only against your own display;
never pkill or killall. Update SPEC section 11, linux-edition.md, gpu-twilight.md and the decision rows you own.
Gate: make check; make check-gpu. Evidence monitors/x11-*.png and hack-two-monitors.png to
~/Documents/Liminal HQ/Afterglow Savers/linux/monitors/; --bench with two monitors in the PR.
Labels: enhancement, linux. Commits, PR, cleanup (worktree /home/scott/source/worktrees/afterglow-savers/gpu-monitors,
branch feat/gpu-monitors) and report as in WP-LINUX-PKG's brief; report the PR URL, the frame times per window and what
was not run.
```

### WP-LINUX-DESKTOP

```text
Sonnet builder, afterglow-savers. Package WP-LINUX-DESKTOP in docs/linux-packages.md. Starts after WP-LINUX-PKG and
WP-GPU-MONITORS are both merged. Read the doc, AGENTS.md, docs/linux-edition.md, platforms/linux/packaging,
platforms/linux/xscreensaver, platforms/linux/setup, SAV-D69 and SAV-D76.
Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers fetch, then
git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/linux-desktops ~/source/worktrees/afterglow-savers/linux-desktops origin/main.
Do: the XML's Monitors select (same, different; --monitors); the idle helper's KDE path
(org.freedesktop.ScreenSaver.GetSessionIdleTime over gdbus) and swayidle path; packaging/wayland-test.sh: an Arch container
with sway, grim, lavapipe and the Arch package, WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=2 WLR_RENDERER=pixman,
XDG_RUNTIME_DIR=/run/afterglow inside the container, WAYLAND_DISPLAY set only to that sway's socket and DISPLAY unset,
grim per output, swaymsg -t get_tree for the fullscreen state; rebuild and retest the three packages; run every desktop row
that runs in a container and screenshot it; write the desktop table into linux-edition.md and say in Not covered what was
not run (GNOME and KDE on real sessions). Never mount a host X or Wayland socket into a container; on the host, DISPLAY set
explicitly and WAYLAND_DISPLAY unset on every command; never :0; never pkill or killall. df -h /home before each container
build; stop under 20 GB.
Gate: make check-fast; make packages; make packages-test; bash platforms/linux/packaging/wayland-test.sh; shellcheck;
markdownlint; CI green. Evidence to ~/Documents/Liminal HQ/Afterglow Savers/linux/ (packages/ and monitors/wayland-*.png).
Labels: enhancement, linux. Commits, PR, cleanup (worktree /home/scott/source/worktrees/afterglow-savers/linux-desktops,
branch feat/linux-desktops) and report as in WP-LINUX-PKG's brief; report the PR URL and the desktop table with
run / not run per row.
```

### WP-SCROLL

```text
Sonnet builder, afterglow-savers. Package WP-SCROLL in docs/linux-packages.md (read it, AGENTS.md, SPEC.md sections 7 to 9,
docs/settings-dialog.md, docs/scenes.md, crates/savers-core/src/settings.rs, crates/savers-core/src/scenes/cracktro.rs,
crates/savers-core/src/banner.rs, platforms/win32/src/dialog.rs, dialog_v2.rs, template.rs, template_v2.rs,
platforms/win32/imports.allow, decisions SAV-D8, SAV-D49, SAV-D60, SAV-D75).
Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers fetch, then
git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/scroller-off ~/source/worktrees/afterglow-savers/scroller-off origin/main.
Do: add Key::Scroller (append it after Volume and move KEY_COUNT to the new last key) as a Cracktro scene field, Check,
0 to 1, default 0, label "Show the scrolling words"; in the scene, build the strip and draw the scroller only when it is 1
(Message, or the built-in text when empty); the plain and Afterglow dialogs show the check box just above the text field
and enable the field only while it is ticked (EnableWindow on BN_CLICKED and at start; add user32.dll!EnableWindow to
imports.allow); the Collection and the banner follow the setting; savers-preview takes --set Scroller=1 if it has no such
flag. Unit tests: Scroller 0 makes the frame independent of Message, 1 does not; round-trip through the store. Update the
goldens that move (SAVERS_UPDATE_GOLDEN=1 after looking at the frames with savers-preview), naming each changed file and
why in the commit. Keep every other default; the size guards must still pass; check the Afterglow dialog still fits and
the 800 x 600 fallback rule still holds. SPEC sections 7 and 9, settings-dialog.md, scenes.md, decision rows SAV-D60 and
SAV-D75.
Gate: make check; bash scripts/smoke-wine.sh; Wine screendumps of both dialogs with the box off and on, on your own Xvfb
(DISPLAY=:NN on every command, WAYLAND_DISPLAY unset, never :0, xdotool only against your display, never pkill/killall).
Evidence to ~/Documents/Liminal HQ/Afterglow Savers/linux/scroller/ as listed in the doc.
Labels: enhancement, win32. Commits, PR, cleanup (worktree /home/scott/source/worktrees/afterglow-savers/scroller-off,
branch feat/scroller-off) and report as in WP-LINUX-PKG's brief; report the PR URL, the goldens that moved, the size
change per file and what was not run.
```
