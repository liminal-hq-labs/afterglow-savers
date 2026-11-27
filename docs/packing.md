# Packing the savers

The build-ordered work packages that put the Afterglow savers into the Afterglow ReactOS images and Quill Ghostwriter into the Quill remix and Quill's own installers. What goes where is in [reactos-integration.md](reactos-integration.md) and [quill-installer-integration.md](quill-installer-integration.md); the reasons are SAV-D54 to SAV-D58. reactos-liminal carries its half of the plan in its own `docs/plans/afterglow-savers.md`, which repeats WP-PACK-3 to WP-PACK-5 for its builders.

## The finish line

- The Afterglow full image (`recipes/afterglow.toml`) and the compact image (`recipes/afterglow-compact.toml`) list "Afterglow Collection", "Afterglow Cracktro", "Afterglow Ink", "Afterglow Maze" and "Afterglow Twilight" in Display Properties > Screen Saver, with no saver selected, so the list shows "(None)" until someone picks one; each one previews in the monitor, opens the Afterglow settings dialog from Settings, runs full screen from Preview and plays its score at `Volume` 28.
- Neither image carries Quill Ghostwriter. The full image keeps ReactOS's own screensavers beside the Afterglow five; the compact image has the five and no others.
- The Quill remix lists the same five plus "Quill Ghostwriter", none of them selected, and its web demo's parts and saved state are rebuilt over the new disk.
- Quill's Windows 95 installer offers "Quill Ghostwriter screensaver" and "Quill Ghostwriter screensaver, Windows 3.x edition", and its Windows 3.x installer the second, all unticked; a default install copies neither. Until liminal-installer has `{windows}` and `{system}` placeholders, neither shows in Display Properties (WP-PACK-7's dependency); with them, both do.
- Every saver is built from source at a pinned commit of this repository, and its output is pinned by SHA-256 wherever it lands.

## Order

```text
WP-PACK-1  afterglow-savers  build contract and CI            ─┐
WP-PACK-2  afterglow-savers  ReactOS guest check of dialog     ├─ savers commit S (merge of WP-PACK-2)
WP-PACK-3  reactos-liminal   multi-output builds from a pin    ─┤  (in parallel with 1 and 2)
WP-PACK-4  reactos-liminal   recipe wiring at S                ◄┘  needs S and WP-PACK-3
WP-PACK-5  reactos-liminal   images booted, sizes, evidence    on WP-PACK-4's branch, same pull request
WP-PACK-6  quill16           remix: Ghostwriter, parts, state  needs WP-PACK-4 merged
WP-PACK-7  quill16           installer components              needs S only (in parallel with 3 to 6); Display Properties listing needs the installer's {windows} and {system}
```

Three packages stand apart from that chain and need none of it: WP-SCROLL (the Cracktro's scroller words off by default, with a setting), WP-LINUX-PKG (deb, rpm and PKGBUILD for the GPU Twilight) and WP-GPU-MONITORS (one GPU Twilight window per monitor). They are briefed at the end of this file and are not part of this build-ordered plan.

At most three builders at once: 1 → 2 in this repository while 3 runs in reactos-liminal; then 4 and 7 together; then 5 and 6.

## House rules for every package

- **Worktrees** under `~/source/worktrees/<repo>/<topic>`, never in the main checkouts. Quill's checkout `~/source/experiments/quill16` is touched only by `git fetch` and `git worktree add` or `remove`.
- **Git.** A topic branch, small Conventional Commits written to a file and committed with `git commit -F`, a bold-label markdown body hard-wrapped at about 72 columns, ending `Co-Authored-By:` with the builder's own model. Files staged by name; never `git add -A` or `git commit -a`. Push only the package's own branch, open the pull request, never merge.
- **Pull request.** Capitalized imperative title; `## Summary` of bold-led bullets; `## Test plan` of `- [x]` and `- [ ]` items naming the commands, ending with an unchecked `- [ ] Not verified: …`.
- **Displays.** Wine and DOSBox-X run on an Xvfb of the package's own, with `DISPLAY=:NN` set explicitly on every command that needs one and `WAYLAND_DISPLAY` unset; QEMU runs with `-display none` and its own monitor socket. Never capture the host screen, never send keys or pointer moves to `:0`, never `pkill` or `killall`: stop only a PID you started.
- **Cleanup.** One removal per command with a full literal path: `rm -r /home/scott/.cache/afterglow-savers/pack-guest`, `git worktree remove /home/scott/source/worktrees/afterglow-savers/pack-ci`, `git worktree prune`, `git branch -d feat/pack-build`. No globs, `$(...)`, variables or loops in a removal.
- **Scratch.** Guest disk copies and build output in `~/.cache/<repo>/…` (check `df -h /home` and `free -g` first); evidence worth keeping in `~/Documents/Liminal HQ/Afterglow/Savers/images/`.
- **Heavy tools** one at a time, `CARGO_BUILD_JOBS=4`.

## WP-PACK-1 — the image build contract and CI (afterglow-savers, area `repo`)

**Finish line.** `scripts/build-scr.sh OUT_DIR` builds the six i586 `.scr` files from a clean source tree (a `git archive` of the commit, as GitHub serves it) into `OUT_DIR`, with `OUT_DIR/SHA256SUMS`, byte-identical to `platforms/win32/build.sh i586`; CI builds, checks and publishes those sums on every pull request.

**Files owned.** `scripts/build-scr.sh` (new), `scripts/check-archive-build.sh` (new), `.github/workflows/ci.yml`, `Makefile` (`build-scr` and `check-archive` targets, and `check` calling the archive check), `docs/testing.md`, `docs/reactos-integration.md` (the contract section), `docs/progress.md`.

**Must not touch.** `crates/`, `platforms/win32/src`, `platforms/win16/src`, the size guards, any scene's look or golden.

**The contract.**

- `bash scripts/build-scr.sh OUT_DIR`: runs `platforms/win32/build.sh i586 --out "$CARGO_TARGET_DIR/scr-i586"` (the target folder is the caller's, default `target/`), then copies exactly `AGMAZE.SCR`, `AGCRACK.SCR`, `AGTWILIT.SCR`, `AGINK.SCR`, `AGCOLLCT.SCR` and `AGQUILL.SCR` into `OUT_DIR` and writes `OUT_DIR/SHA256SUMS` (`sha256sum` format, sorted by name). No test programs (`previewhost.exe`, `bandcheck.exe`) in `OUT_DIR`. Exits non-zero with a clear line when the toolchain is missing (rustup, 1.98.0, `rust-src`, `llvm-tools`).
- It must work from an extracted GitHub tar: `platforms/win32/crates` is a relative symlink, which the archive keeps; the script must not depend on `.git` (no `git describe`, no `git ls-files`).
- `scripts/check-archive-build.sh` makes `git archive --format=tar HEAD` into a fresh folder under `~/.cache/afterglow-savers/`, extracts it, runs `build-scr.sh` there with its own `CARGO_TARGET_DIR`, and compares the sums with a build in the working tree. It removes its folder with one literal `rm -r` in its `trap`.

**CI** (one job per check, the shared `ci-rust` image pinned by the digest already in `ci.yml`, `rustup toolchain install 1.98.0` with `rust-src` and `llvm-tools`):

- `win32-i586` "Win32 build (Pentium)": `bash platforms/win32/build.sh i586`, `bash platforms/win32/check.sh i586` (structure, imports, Pentium, sizes).
- `win32-x86_64` "Win32 build (x86-64)": the same for x86-64.
- `win32-clippy` "Win32 clippy": `make win32-fmt win32-clippy win32-test`.
- `scr-contract` "Image build contract": `bash scripts/check-archive-build.sh`, then upload `scr-i586` (the six files and `SHA256SUMS`, seven days) and print the sums into the job summary, so reactos-liminal's pin can be compared against CI.
- `win16` "Windows 3.x build": `bash platforms/win16/check.sh` (needs `python3`), and upload `QUILLGW.SCR` with its sum.
- `repro` "Reproducible build": `bash scripts/repro-win32.sh i586`.
- Wine smoke and the guest run stay local (`make check`, `scripts/guest/`).

**Tests.** `make check` in the worktree; `bash scripts/check-archive-build.sh`; `docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2:v0.17.2 "**/*.md"`; `docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint`; CI green on the pull request.

**Evidence.** The six sums from the local build and from CI's job summary, equal, quoted in the pull request.

## WP-PACK-2 — the savers in a ReactOS guest, dialog included (afterglow-savers, areas `repo` and `win32`)

**Finish line.** `scripts/guest/guest.py` drives, in a copy of the Afterglow image under QEMU: the Screen Saver list, each saver's monitor preview, its Settings (the Afterglow dialog, not the plain fallback), Cancel, OK with one change, Preview full screen and a key to end it, and the Collection starting by itself after a one-minute timeout that the check sets in its own copy (the images select no saver, so the run writes `SCRNSAVE.EXE` and `ScreenSaveTimeOut` 60 itself); it records the AC97 output and checks each score is present and quiet. A `--in-image` mode runs the same checks over the `.scr` files an image already carries, without writing any, for WP-PACK-5 and WP-PACK-6. Anything that fails in ReactOS is fixed in the `.scr` in this package, before the commit is pinned.

**Files owned.** `scripts/guest/guest.py`, `scripts/guest/` (new helpers), `platforms/win32/src` only for fixes the guest run shows (one fix, one commit, with its own test where the host can test it), `docs/settings-dialog.md` ("Not yet checked"), `docs/reactos-scr.md` (anything learned), `docs/progress.md`, `SPEC.md` (only if behaviour changes).

**Must not touch.** `crates/savers-core` scenes and scores, `platforms/linux`, `platforms/win16`, the defaults (`Sound` 1, `Volume` 28, palette Dusk, `RenderScale` 50, Detail Medium), the size guards.

**What the guest run checks** (the image's settings: 1024 × 768, 32 bits, the Afterglow visual style):

- The list holds the five names by their string 1 (and the stock ReactOS savers in the full image, none in the compact), with "(None)" selected in an image as built; `AGQUILL.SCR`, when written, adds "Quill Ghostwriter".
- Settings opens the Afterglow dialog owned by Display Properties (`/c:<hwnd>`), with its banner drawn on the first tick, the swatches, the live preview, and Sound and Volume at the top of Look reading on and 28. Cancel writes nothing under `HKCU\Software\Liminal HQ\Afterglow Screensavers`; OK writes the saver's key; Display Properties restarts the monitor preview with the change.
- Preview runs full screen with no pointer, a key ends it, and the monitor preview comes back.
- The run selects the Collection and sets `ScreenSaveActive` 1 and `ScreenSaveTimeOut` 60 in the copy (the image has no saver selected, SAV-D59); Winlogon starts the Collection by itself, and a key ends it.
- The AC97 capture: each saver's score present, with an RMS well below full scale (the same check `scripts/wine-audio.sh` makes), and silence from the preview and the dialog.

**Tests.** `make check`; `python3 -I scripts/guest/guest.py --scr platforms/win32/dist/i586 --shots ~/.cache/afterglow-savers/pack-guest/shots --audio ~/.cache/afterglow-savers/pack-guest/guest.wav` (KVM, no display); `bash scripts/smoke-wine.sh` and `bash scripts/wine-audio.sh` under the package's own Xvfb.

**Evidence** in `~/Documents/Liminal HQ/Afterglow/Savers/images/`: `guest-list.png`, `guest-dialog-<saver>.png` (six), `guest-preview-<saver>.png`, `guest-fullscreen-<saver>.png`, `guest-idle-collection.png`, and the audio levels table in the pull request.

**Its merge commit is S**, the savers commit reactos-liminal and Quill pin.

## WP-PACK-3 — builds from a pinned commit with several outputs (reactos-liminal)

**Finish line.** A recipe can declare `[builds.NAME]` with either an in-tree `source` (afterfetch, unchanged) or `repo`, `commit` and `tar_sha256` (a GitHub source tar, fetched and checked like `[inputs.afterglow]`), an optional `script` (default `build.sh`, relative to the project root), and either `sha256` (one `NAME.exe`) or an `outputs` table of file name to SHA-256. `built:NAME/FILE` names one output. `rl fetch`, `rl lock`, `rl verify`, `rl pin`, `rl key`, `rl doctor`, `rl manifest` and `rl licences` know the new form; `rl build RECIPE NAME` builds one build and checks its sums.

**Files owned.** `reactos_liminal/recipe.py`, `reactos_liminal/rustbuild.py`, `reactos_liminal/fetch.py`, `reactos_liminal/lock.py`, `reactos_liminal/pin.py`, `reactos_liminal/doctor.py`, `reactos_liminal/cli.py`, `reactos_liminal/manifest.py`, `reactos_liminal/licences.py`, `reactos_liminal/image.py` (only the `built:` lookup), `tests/` (new tests), `SPEC.md`, `docs/recipes.md`, `docs/pipeline.md`, `docs/decisions.md` (RL-63 and sub-decisions), `AGENTS.md` and `CLAUDE.md` (the Rust rule), `NOTICE.md`.

**Must not touch.** `recipes/` (WP-PACK-4), `tools/afterfetch` and its pin, the patchers, `guest/`, `profiles/`, `.github/workflows/build.yml` and `release.yml`.

**Details.**

- **Extraction.** The tar is extracted under `WORK/builds/NAME/src-<commit>` with `tarfile`'s `data` filter (relative symlinks inside the tree allowed, nothing outside it), once per commit; the build runs there with `CARGO_TARGET_DIR=WORK/builds/NAME/target`.
- **Outputs.** Each listed output must exist in `OUT_DIR` and match its sum; extra files are ignored. An output's sum may be declared by a later recipe in the chain (Quill's recipe adds `AGQUILL.SCR`): tables already merge key by key, so `declaring_file` becomes per output, and `rl lock` and `rl pin` write each sum to the recipe that declares that output.
- **Rule change.** AGENTS.md's "Rust is for `afterfetch.exe` alone" becomes: Rust in this repository is `afterfetch.exe` alone; a build may also compile a Rust project from another repository at a pinned commit (the Afterglow savers), whose output is pinned by SHA-256 and never committed.
- **Tests** with synthetic fixtures only: a tar made at test time holding a `build.sh` that writes two small files; a wrong tar sum, a missing output, a wrong output sum and an unsafe tar member are each refused; `built:NAME/FILE` for an undeclared output is refused at load; a child recipe's extra output merges and is locked in the child's file; afterfetch's existing tests pass unchanged.

**Tests.** `ruff format --check`, `ruff check`, `python3 -m pytest`, `bash scripts/check-headers.sh`, markdownlint, actionlint, and `./rl explain recipes/afterglow.toml` unchanged.

## WP-PACK-4 — the recipe wiring at S (reactos-liminal)

**Finish line.** `recipes/afterglow.toml` builds the savers from S and writes the five files to `ReactOS/system32`; no registry file selects a saver, so both images list the five in Display Properties with none chosen; the compact recipe inherits the files and its trim removes ReactOS's own savers; both locks are current; CI builds the savers on every pull request.

**Files owned.** `recipes/afterglow.toml`, `recipes/afterglow.lock.json`, `recipes/afterglow-compact.lock.json`, `.github/workflows/ci.yml` (a job), `.github/workflows/build.yml` (the savers tar's cache and `--require`), `.github/workflows/release.yml` (the savers source in the corresponding source, if its source job lists inputs), `SPEC.md`, `docs/recipes.md`, `docs/decisions.md`, `NOTICE.md`, `tests/` (the trim tests).

**Must not touch.** The `[builds]` code (WP-PACK-3), `tools/afterfetch`, the Afterglow pin, `profiles/base.txt`, `guest/reg/` (no new registry file: SAV-D59).

**The recipe.**

```toml
[builds.afterglow-savers]
repo = "liminal-hq-labs/afterglow-savers"
commit = "<S, 40 digits>"
tar_sha256 = "<measured by ./rl pin>"
script = "scripts/build-scr.sh"
[builds.afterglow-savers.outputs]
"AGMAZE.SCR" = "<sha256>"
"AGCRACK.SCR" = "<sha256>"
"AGTWILIT.SCR" = "<sha256>"
"AGINK.SCR" = "<sha256>"
"AGCOLLCT.SCR" = "<sha256>"

[[files]]
source = "built:afterglow-savers/AGMAZE.SCR"
destination = "ReactOS/system32/AGMAZE.SCR"
# and one entry each for AGCRACK, AGTWILIT, AGINK and AGCOLLCT; AGQUILL.SCR is built but never written by these recipes
```

**No saver is selected** (SAV-D59). The recipes add no registry file: `quiet.reg` stays the only screensaver setting, so `ScreenSaveActive` is `"0"` and `SCRNSAVE.EXE` is absent, and the Screen Saver list shows "(None)" until someone picks one. The savers' own settings get no registry entries either: `Sound` 1, `Volume` 28, palette Dusk and the rest are the program's defaults (SAV-D56).

**What each image keeps.** The `base` trim profile removes ReactOS's own eight `.scr` files (`profiles/base.txt`), which suits the compact image. The full image keeps them beside the Afterglow five, so its recipe needs a trim that spares the stock files (the full image removes nothing, SAV-D55); the compact image inherits the five Afterglow files and keeps them, and a test checks that no trim profile names any `AG*.SCR` file.

**CI.** A job "Afterglow savers" in `ci.yml` (the `ci-rust` image by digest, `rustup toolchain install 1.98.0 --component rust-src,llvm-tools`): `./rl build recipes/afterglow.toml afterglow-savers --work "$RUNNER_TEMP/rl-work"`, which fetches the tar at S, builds and checks the five sums. `build.yml` caches the savers tar by commit and sum, as it caches Afterglow's, and verifies it with `--require`.

**Tests.** `./rl pin recipes/afterglow.toml` (measures the tar and outputs), `./rl lock` for both recipes, `./rl verify` for both, `./rl build recipes/afterglow.toml afterglow-savers`, `./rl image recipes/afterglow-compact.toml --dry-run`, the Python checks, markdownlint, actionlint, shellcheck. The sums must equal WP-PACK-1's CI sums for S.

## WP-PACK-5 — the images booted, sizes and evidence (reactos-liminal, on WP-PACK-4's branch)

**Finish line.** The full and compact images are built locally with the savers, booted in QEMU, and every check of WP-PACK-2 passes in each, and the full image still lists ReactOS's own savers where the compact one does not; the size cost is measured and written into `docs/recipes.md`; the evidence is in the pull request.

**Files owned.** `docs/recipes.md` (a short "Screensavers" section with the sizes), the pull request text. Fixes found here go back to this repository (a `fix:` pull request, then a new S) or to WP-PACK-4's files, never patched in place.

**Must not touch.** Anything outside those files; no new recipe keys.

**Steps.**

1. `./rl install recipes/afterglow.toml --work ~/.cache/reactos-liminal` (or the cached golden disk), `./rl image recipes/afterglow.toml`, `./rl image recipes/afterglow-compact.toml --golden <the base golden disk>`, `./rl audit` on both.
2. `./rl parts` for both, before (main) and after (the branch), and the disk's used bytes before and after: report the five files' bytes (about 1.0 MB, `docs/progress.md` sizes), the change in total compressed parts (each file compresses to about 45% at zstd 19; expect about 0.45 MB) and the number of parts that changed.
3. Boot each image with afterglow-savers' `scripts/guest/guest.py --in-image --source <the image>` (WP-PACK-2), headless with KVM: the list, the six dialogs' worth of checks for the five savers, Preview, the idle start (the copy is given the Collection and a one-minute timeout, since no image selects one), and the AC97 capture.

**Evidence** in `~/Documents/Liminal HQ/Afterglow/Savers/images/`: `full-list.png`, `full-dialog-<saver>.png`, `full-fullscreen-<saver>.png`, `full-idle-collection.png`, `compact-list.png`, `compact-dialog-collection.png`, and a sizes table in the pull request.

## WP-PACK-6 — the Quill remix (quill16)

**Finish line.** The remix (`site/try-reactos/qagrr/qagrr.toml`) pins reactos-liminal at WP-PACK-4's merge, inherits the five savers, adds `AGQUILL.SCR`, selects none (as the images do, SAV-D59); its parts and saved state are rebuilt and checked; the page still boots to Quill with the demo letter.

**Files owned.** `site/try-reactos/qagrr/qagrr.toml`, `site/try-reactos/qagrr/reactos-liminal.pin`, `site/try-reactos/README.md`, the remix's docs and decisions, `.github/workflows/reactos-qagrr.yml` only if the savers build needs the toolchain components there (`rust-src`, `llvm-tools`).

**Must not touch.** Quill's programs, `installer/` (WP-PACK-7), the trim lists, anything in reactos-liminal or this repository.

**The recipe.**

```toml
[builds.afterglow-savers.outputs]
"AGQUILL.SCR" = "<sha256 at S>"

[[files]]
source = "built:afterglow-savers/AGQUILL.SCR"
destination = "ReactOS/system32/AGQUILL.SCR"
```

No registry file selects a saver, and `quiet.reg` keeps the screensaver off: the web demo never starts a saver by itself (SAV-D57). The Windows 3.x `QUILLGW.SCR` is not added (SAV-D37).

**Tests.** `build-with-rl.sh --dry-run`, `--check-trim`, a full build, `build-state.sh DIST`, `check-state.sh DIST`; a QEMU boot of the remix disk with `guest.py --in-image` showing "Quill Ghostwriter" in the list, unselected, and its dialog; the parts' size change in the manifest.

**Evidence** in `~/Documents/Liminal HQ/Afterglow/Savers/images/`: `remix-list.png`, `remix-dialog-ghostwriter.png`, `remix-fullscreen-ghostwriter.png`, and the state check's canvas shots.

## WP-PACK-7 — Quill's installers (quill16)

**Finish line.** Quill's Windows 95 installer offers the two Ghostwriter components and the Windows 3.x installer one, all unticked, built from S with the sums checked; a ticked install copies the file and adds a "Quill Ghostwriter" shortcut that runs it; a default install copies nothing of it; uninstall removes what was installed. No liminal-installer change in this package.

**Dependency: Display Properties needs a new installer feature.** The owner wants Ghostwriter to show in Display Properties > Screen Saver, which needs the file in the Windows or system folder. That waits for liminal-installer to gain `{windows}` and `{system}` path placeholders in every runtime, its receipts and its uninstall (SAV-D58); this package ships with `{install}` and the shortcut, and a follow-up one-line change to `to = "{system}/"` (and `"{windows}/"` for the Windows 3.x edition) lands once that feature does. The follow-up does not start until the liminal-installer feature is merged.

**Files owned.** `installer/components.toml`, `installer/ghostwriter.pin` (new: S, the tar's sum, the sums of `AGQUILL.SCR` and `QUILLGW.SCR`), `installer/tools/fetch-ghostwriter.sh` (new), `installer/tools/stage.sh`, `installer/README.TXT`, `installer/tools/e2e-w95.sh` and `e2e-w31.sh` (a ticked run), Quill's installer docs.

**Must not touch.** liminal-installer, the installer's flows and themes, the other components, the remix.

**The components** (today's installer: `default = false` exists; install-time paths are `{install}` only in the retro runtimes, and a shortcut has no arguments):

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

On Windows 95 a shortcut to a `.scr` opens it with the `scrfile` verb (`"%1" /S`), so it plays; `QUILLGW.SCR` with no argument runs as `/s`. Program Manager may refuse an item whose file is not a program by `WIN.INI`'s `Programs=`; if the Windows 3.11 run shows that, drop the 3.x shortcut on `win16-x86` (keep the file) and say in `README.TXT` how to list it: copy `QUILLGW.SCR` to the Windows folder. Neither shows in Display Properties or Control Panel > Desktop from the install folder; that waits for the `{windows}` and `{system}` placeholders in liminal-installer (SAV-D58), then `to` changes.

**Build.** `fetch-ghostwriter.sh` downloads the tar at S from GitHub into `~/.cache/quill16-installer-build/ghostwriter`, checks its sum, runs `scripts/build-scr.sh` and `platforms/win16/build.sh` there, and checks both outputs' sums from the pin. `stage.sh --build` calls it; plain `stage.sh` copies from `GHOSTWRITER_DIR`; `--stand-ins` writes placeholders for both. No `.SCR` is committed.

**Tests.** `installer/tools/check.sh`, `build-disks.sh`, `verify-disks.sh` (the Windows 95 floppy must still fit: `AGQUILL.SCR` is 171 KB, `QUILLGW.SCR` 18 KB; if it does not, the components go on the zip only and the owner is told), `e2e-w95.sh` with the defaults and with both ticked, `e2e-w31.sh` with `ghostwriter16` ticked, each on its own Xvfb with `DISPLAY` set on every command.

**Evidence** in `~/Documents/Liminal HQ/Afterglow/Savers/images/`: `w95-components.png` (both unticked), `w95-start-menu.png`, `w95-ghostwriter.png`, `w95-ghostwriter16.png`, `w31-components.png`, `w31-ghostwriter.png`.

## Owner decisions

The owner's answers are in place; the file named swaps each one.

| Question | Answer | Where to swap |
| --- | --- | --- |
| Ghostwriter in Display Properties from Quill's installers | Wanted, but waits for a new liminal-installer feature (`{windows}` and `{system}` placeholders); until then it installs to `{install}` with a Start menu shortcut | A liminal-installer package adding `{windows}` and `{system}` to every runtime and the receipts, then `to = "{system}/"` and `"{windows}/"` in Quill's `installer/components.toml` |
| The compact image keeps the savers | Yes, the five; ReactOS's own savers go in the compact image and stay in the full one | `recipes/afterglow-compact.toml`: a trim extra with one `f ReactOS/system32/AG….SCR` line per file |
| A default saver and timeout in the images | None: no saver is selected in any image (SAV-D59) | A named registry file in reactos-liminal (the earlier plan's `screensaver-afterglow.reg`: `SCRNSAVE.EXE`, `ScreenSaveActive` 1, `ScreenSaveTimeOut` 600) listed after `quiet` in `reg_files` |
| A saver selected in the Quill remix | None | A named registry file as above in `qagrr.toml`'s `reg_files` |
| The savers' settings written into the images | None: the program defaults (`Sound` on, `Volume` 28, Dusk) | A recipe-level `.reg` file under `HKCU\Software\Liminal HQ\Afterglow Screensavers\<Saver>`, or the defaults in `savers_core::settings` |
| ReactOS's own savers beside Afterglow's in the full image | Kept in the full image, removed from the compact | A trim extra in `recipes/afterglow.toml` naming the eight stock `.scr` files |
| Both Ghostwriter editions on Windows 95 | Both offered, unticked | `targets` of `ghostwriter16` in `installer/components.toml` |
| The Cracktro's scroller words | Off by default; a setting turns them on and a text field takes your own words (WP-SCROLL) | `savers_core::settings` defaults |
| Linux packages and one window per monitor for the GPU Twilight | Queued (WP-LINUX-PKG, WP-GPU-MONITORS) | The two briefs below |

## Sonnet briefs

### WP-PACK-1 (in full)

```text
You are a Sonnet builder on afterglow-savers (~/source/liminal-hq-labs/afterglow-savers, remote liminal-hq-labs/afterglow-savers).
Package: WP-PACK-1 in docs/packing.md — read it, AGENTS.md, docs/landing.md, docs/testing.md and docs/reactos-integration.md first.

Set up:
- git -C ~/source/liminal-hq-labs/afterglow-savers fetch
- git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/pack-build ~/source/worktrees/afterglow-savers/pack-build origin/main
- Work only in ~/source/worktrees/afterglow-savers/pack-build. Never edit the main checkout.

Do:
1. scripts/build-scr.sh OUT_DIR: build the i586 savers with platforms/win32/build.sh i586 --out "${CARGO_TARGET_DIR:-$root/target}/scr-i586",
   copy exactly AGMAZE.SCR AGCRACK.SCR AGTWILIT.SCR AGINK.SCR AGCOLLCT.SCR AGQUILL.SCR to OUT_DIR, write OUT_DIR/SHA256SUMS
   (sha256sum format, sorted). No .git use. A clear error when rustup, 1.98.0, rust-src or llvm-tools is missing. SPDX header.
2. scripts/check-archive-build.sh: git archive HEAD into a fresh folder under ~/.cache/afterglow-savers/, extract, run build-scr.sh
   there with its own CARGO_TARGET_DIR, build again in the worktree, compare SHA256SUMS; FAIL with the differing names. The trap
   removes its folder with one rm -r of that folder's path.
3. Makefile: a build-scr target; add the archive check to check.
4. .github/workflows/ci.yml: jobs win32-i586, win32-x86_64, win32-clippy, scr-contract (uploads artifact scr-i586 with the six files
   and SHA256SUMS, 7 days, and writes the sums to the job summary), win16 (platforms/win16/check.sh, uploads QUILLGW.SCR and its sum),
   repro (scripts/repro-win32.sh i586). The ci-rust image by the digest already used; rustup toolchain install from rust-toolchain.toml;
   actions pinned as the file already pins them; persist-credentials: false.
5. docs: testing.md (the new scripts and jobs), reactos-integration.md (the contract), progress.md (WP-PACK-1 done, the sums).

Rules: Canadian English, real em dashes, no hard-wrapped markdown. Tabs except YAML. CARGO_BUILD_JOBS=4, one heavy tool at a time.
Do not touch crates/, platforms/*/src, the size guards or any golden. Wine smoke runs on its own Xvfb: set DISPLAY=:NN explicitly on
every command, unset WAYLAND_DISPLAY, never touch :0, never pkill/killall (stop only your own PID).

Gate: make check; bash scripts/check-archive-build.sh; markdownlint
(docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2:v0.17.2 "**/*.md"); actionlint
(docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint).

Commit: small Conventional Commits (ci:, build:, docs:), message written to a file with a single-quoted heredoc and committed with
git commit -F <file>; bold-label markdown body hard-wrapped at about 72; trailer
Co-Authored-By: Claude Sonnet <your exact model> <noreply@anthropic.com>. Stage files by name; never git add -A or git commit -a.

Push only feat/pack-build and open a pull request (gh pr create): title capitalized imperative, e.g. "Build the image savers from a
source archive and in CI"; body ## Summary with bold-led bullets, ## Test plan with [x]/[ ] items naming commands, last item
"- [ ] Not verified: …". Do not merge. Wait for CI; fix failures on the branch.

Cleanup when told the branch is merged: git worktree remove /home/scott/source/worktrees/afterglow-savers/pack-build, git worktree
prune, git branch -d ci/pack-build — one command each, literal paths.

Report: the PR URL, the six sums locally and from CI (equal or not), the CI build time of scr-contract, what was not run.
```

### WP-PACK-2

```text
Sonnet builder, afterglow-savers. Package WP-PACK-2 in docs/packing.md (read it, AGENTS.md, docs/reactos-scr.md,
docs/settings-dialog.md, docs/testing.md, scripts/guest/guest.py). Starts after WP-PACK-1 is merged.
Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b test/pack-guest
~/source/worktrees/afterglow-savers/pack-guest origin/main.
Extend scripts/guest/guest.py to check, in a copy of the Afterglow image (source image from a reactos-liminal build under
~/.cache/reactos-liminal/images; copy to ~/.cache/afterglow-savers/pack-guest, check df -h /home first): the list, each monitor
preview, Settings showing the Afterglow dialog (banner, swatches, live preview, Sound on and Volume 28), Cancel writing nothing, OK
writing the saver's key, Preview full screen and a key ending it, the Collection starting after a one-minute timeout, and AC97 levels
(score present, quiet; silence in preview and dialog), and an --in-image mode that checks the .scr files an image already
carries without writing any. QEMU only with -display none, its own monitor socket and pid; keys go to the
guest through the monitor's sendkey, never to the host. Fix any .scr bug the guest shows in platforms/win32/src, one fix per commit.
Do not change defaults, scenes, scores or size guards.
Gate: make check, the guest run, scripts/smoke-wine.sh and scripts/wine-audio.sh on your own Xvfb (DISPLAY=:NN on every command,
WAYLAND_DISPLAY unset, never :0, never pkill/killall).
Evidence to ~/Documents/Liminal HQ/Afterglow/Savers/images/ (guest-*.png as named in the package).
Commits, PR, cleanup and report as in WP-PACK-1's brief (branch test/pack-guest); report the PR URL and that its merge is S.
```

### WP-PACK-3

```text
Sonnet builder, reactos-liminal (~/source/liminal-hq-labs/reactos-liminal). Package WP-PACK-3 in afterglow-savers docs/packing.md
and reactos-liminal docs/plans/afterglow-savers.md (read both, AGENTS.md, SPEC.md, docs/recipes.md, reactos_liminal/rustbuild.py,
recipe.py, lock.py, pin.py, fetch.py).
Worktree: git -C ~/source/liminal-hq-labs/reactos-liminal worktree add -b feat/pinned-builds
~/source/worktrees/reactos-liminal/pinned-builds origin/main.
Add the pinned-commit source (repo, commit, tar_sha256), script, outputs and built:NAME/FILE to [builds]; per-output declaring file
for rl lock and rl pin; rl build RECIPE NAME; fetch, verify, key, doctor, manifest and licences support. Record RL-63 in
docs/decisions.md, amend the Rust rule in AGENTS.md and CLAUDE.md, update SPEC.md and docs/recipes.md in the same commits.
Synthetic fixtures only (a tar made at test time); no recipe change. Python 3.12 stdlib, tabs, ruff, readable names, python3 -I.
Gate: ruff format --check, ruff check, python3 -m pytest, scripts/check-headers.sh, markdownlint, actionlint, shellcheck.
PR labels: enhancement, recipe. Commits, PR, cleanup and report as in WP-PACK-1's brief (branch feat/pinned-builds; worktree path
/home/scott/source/worktrees/reactos-liminal/pinned-builds).
```

### WP-PACK-4 and WP-PACK-5

```text
Sonnet builder, reactos-liminal. Packages WP-PACK-4 then WP-PACK-5 (same branch, same PR) in docs/plans/afterglow-savers.md.
Needs WP-PACK-3 merged and S (the afterglow-savers commit from WP-PACK-2's merge).
Worktree: git -C ~/source/liminal-hq-labs/reactos-liminal worktree add -b feat/afterglow-savers
~/source/worktrees/reactos-liminal/afterglow-savers origin/main.
WP-PACK-4: [builds.afterglow-savers] at S with script scripts/build-scr.sh and five outputs, five [[files]] to ReactOS/system32,
no registry file and no saver selected (SAV-D59: quiet.reg stays the only screensaver setting), the full image keeping ReactOS's own
savers and the compact image trimming them, a test that no trim profile names an AG*.SCR file, ./rl pin then ./rl lock and ./rl verify for both recipes, ci.yml job "Afterglow savers", build.yml cache and
--require for the savers tar, NOTICE.md, SPEC.md, docs. The sums must equal afterglow-savers CI's for S.
WP-PACK-5: rl install/image/audit/parts for full and compact in ~/.cache/reactos-liminal (check df -h /home first), sizes before
and after, then boot both with afterglow-savers scripts/guest/guest.py --in-image headless with KVM (-display none, own monitor socket, keys via
the monitor only), every check of WP-PACK-2, evidence to ~/Documents/Liminal HQ/Afterglow/Savers/images/ (full-*.png, compact-*.png),
a Screensavers section with the sizes in docs/recipes.md. Never pkill/killall; stop only your own QEMU PID. Remove guest copies with
one rm -r per literal path.
PR labels: enhancement, recipe, afterglow. Commits, PR, cleanup and report as in WP-PACK-1's brief (branch feat/afterglow-savers).
```

### WP-PACK-6

```text
Sonnet builder, Quill (~/source/experiments/quill16: never edit that checkout; only git fetch and git worktree add/remove there).
Package WP-PACK-6 in afterglow-savers docs/packing.md. Needs WP-PACK-4 merged in reactos-liminal.
Worktree: git -C ~/source/experiments/quill16 worktree add -b feat/remix-savers ~/source/worktrees/quill16/remix-savers origin/main.
Read the repository's AGENTS.md and site/try-reactos/README.md first and follow its rules and decision prefix.
Pin reactos-liminal.pin to WP-PACK-4's merge commit; add AGQUILL.SCR's output sum and its [[files]] entry to qagrr.toml; select no
saver. build-with-rl.sh --check-trim, --dry-run, a full build (RL_QEMU_LOCK set), build-state.sh,
check-state.sh; boot the disk with afterglow-savers scripts/guest/guest.py --in-image (QEMU -display none). Chromium runs
headless from the scripts only. Evidence remix-*.png to ~/Documents/Liminal HQ/Afterglow/Savers/images/; parts size change in the PR.
Commits, PR, cleanup and report as in WP-PACK-1's brief (branch feat/remix-savers).
```

### WP-PACK-7

```text
Sonnet builder, Quill (~/source/experiments/quill16: never edit that checkout; only git fetch and git worktree add/remove there).
Package WP-PACK-7 in afterglow-savers docs/packing.md. Needs S. No liminal-installer change of any kind here; showing in Display
Properties waits for liminal-installer's {windows} and {system} placeholders, so leave `to = "{install}/"` and note the dependency.
Worktree: git -C ~/source/experiments/quill16 worktree add -b feat/installer-ghostwriter
~/source/worktrees/quill16/installer-ghostwriter origin/main. Read the repository's AGENTS.md and installer docs first.
Add installer/ghostwriter.pin, installer/tools/fetch-ghostwriter.sh (download the GitHub tar at S to
~/.cache/quill16-installer-build/ghostwriter, check its sum, build AGQUILL.SCR with scripts/build-scr.sh and QUILLGW.SCR with
platforms/win16/build.sh, check both sums), stage.sh support (--build, GHOSTWRITER_DIR, --stand-ins), the two default = false
components with their Start menu shortcuts, README.TXT. check.sh, build-disks.sh, verify-disks.sh (report if the Windows 95 floppy
no longer fits), e2e-w95.sh with defaults and with both ticked, e2e-w31.sh with ghostwriter16 ticked: each on its own Xvfb with
DISPLAY=:NN on every command and WAYLAND_DISPLAY unset; xdotool only against that display, never :0; never pkill/killall.
If Program Manager refuses the QUILLGW.SCR item, drop that shortcut on win16-x86 and document copying it to the Windows folder.
Evidence w95-*.png and w31-*.png to ~/Documents/Liminal HQ/Afterglow/Savers/images/.
Commits, PR, cleanup and report as in WP-PACK-1's brief (branch feat/installer-ghostwriter).
```

## WP-SCROLL — the Cracktro's scroller, off by default (afterglow-savers, areas `core` and `win32`)

Small and separate; WP-PACK-1 does not need it. Today the Cracktro scrolls the Afterglow message along its lower third (`Message`, 2,000 characters, SPEC section 7). The owner wants the words off until asked for.

**Finish line.** A new setting (`Scroller`, `REG_DWORD`, 0 off or 1 on, default 0) decides whether the sine scroller shows words. Off, the scene plays as now without the scroller (the copper wave motion stays where the design wants it, with no letters); on, it scrolls `Message`. `Message` is a text field in the Afterglow dialog, enabled only when the scroller is on, and its default text stays the current Afterglow message (the defaults table in SPEC). The Collection's Cracktro part follows the same setting. Windows, Linux and the host preview agree.

**Files owned.** `crates/savers-core` (`settings.rs`, the Cracktro scene, goldens that change because the default is off, say which), `platforms/win32` (the dialog's Look page: a checkbox and a text field, string table), `docs/settings-dialog.md`, `docs/scenes.md`, `SPEC.md`, `docs/decisions.md`.

**Tests.** `make check`; goldens updated with a note in the commit of every frame that changed and why; a unit test that `Message` round-trips and is used only when `Scroller` is 1; Wine screendumps of the dialog with the box off and on.

**Brief.**

```text
Sonnet builder, afterglow-savers. Package WP-SCROLL in docs/packing.md (read it, SPEC.md section 7 and 8, docs/settings-dialog.md,
docs/scenes.md, crates/savers-core/src/scenes/cracktro.rs). Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers worktree
add -b feat/scroller-off ~/source/worktrees/afterglow-savers/scroller-off origin/main. Add the Scroller setting (default off) and the
dialog's checkbox and Message text field (enabled only when on); keep the built-in default text. Update SPEC, a decision, goldens
that change. Gate: make check; Wine screendumps under your own Xvfb (DISPLAY set on every command, WAYLAND_DISPLAY unset, never :0,
no pkill). Commits, PR and report as WP-PACK-1's brief.
```

## WP-LINUX-PKG — packages for the GPU Twilight (afterglow-savers, area `linux`)

**Finish line.** `make packages` (a script under `platforms/linux/packaging/`) builds a `.deb`, an `.rpm` and an Arch `PKGBUILD` (with a `.SRCINFO`) for `afterglow-twilight-gpu`, installing the program, `afterglow-twilight-xscreensaver`, the desktop launcher and `afterglow-twilight.xml` into xscreensaver's own config folder (`/usr/share/xscreensaver/config/`), so the hack shows its settings dialog without the administrator step `docs/linux-edition.md` mentions today. The packages depend on the graphics and sound libraries the program links (Vulkan loader, ALSA) and suggest xscreensaver; they put nothing under the user's home.

**Files owned.** `platforms/linux/packaging/` (new), `docs/linux-edition.md` (Packages section), `docs/testing.md`, `.github/workflows/ci.yml` (one job that builds the three and lints them), `docs/decisions.md`.

**Tests.** Build in a Debian and a Fedora container and an Arch container (docker), install each package there, and check `afterglow-twilight-gpu --help`, the XML in place and `dpkg -c`, `rpm -qlp` and `pacman -Ql` file lists; `lintian` and `rpmlint` output reported; the packages' versions come from the crate version.

**Brief.**

```text
Sonnet builder, afterglow-savers. Package WP-LINUX-PKG in docs/packing.md (read it, docs/linux-edition.md, platforms/linux/install.sh,
platforms/linux/xscreensaver). Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/linux-packages
~/source/worktrees/afterglow-savers/linux-packages origin/main. Build deb, rpm and PKGBUILD for the GPU Twilight, installing the
xscreensaver XML to /usr/share/xscreensaver/config/. Containers via docker only; no sudo on the host, no installs on the host.
Never push tags or publish. Gate: make check-fast, packaging tests in the containers, markdownlint. Commits, PR and report as
WP-PACK-1's brief.
```

## WP-GPU-MONITORS — one window per monitor (afterglow-savers, area `linux`)

**Finish line.** `afterglow-twilight-gpu` opens one borderless fullscreen window on every monitor (today only the primary monitor is covered, `docs/linux-edition.md`), each drawing its own view of the scene (a different seed, camera path and time offset, so the monitors never repeat each other) with options that can differ per monitor in the settings file (`[monitor.<name>]` tables overriding the shared values: palette, speed, render scale, sun) and, where they apply, on the command line (`--monitor NAME:KEY=VALUE`). Input on any monitor ends the saver. One monitor behaves exactly as now. Wayland and X11 both; `--monitors primary` restores the old behaviour.

**Files owned.** `platforms/linux/src`, `crates/savers-gpu` (only what per-view parameters need), `docs/linux-edition.md`, `docs/gpu-twilight.md`, `SPEC.md` (the Linux settings), `docs/decisions.md`.

**Tests.** `make check-gpu` extended on its own Xvfb with two virtual screens (Xinerama or two RandR monitors): two windows, different frames, the exit rules; a unit test for the settings merge; a single-monitor run byte-identical to today's.

**Brief.**

```text
Sonnet builder, afterglow-savers. Package WP-GPU-MONITORS in docs/packing.md (read it, docs/linux-edition.md, docs/gpu-twilight.md,
platforms/linux/src, crates/savers-gpu). Worktree: git -C ~/source/liminal-hq-labs/afterglow-savers worktree add -b feat/gpu-monitors
~/source/worktrees/afterglow-savers/gpu-monitors origin/main. One window per monitor, a different view per window, per-monitor
option tables in the settings file, a flag to restore the primary-only behaviour. Test on your own Xvfb with two RandR monitors
(DISPLAY set on every command, WAYLAND_DISPLAY unset, never :0, no pkill). Gate: make check, make check-gpu. Commits, PR and report
as WP-PACK-1's brief.
```
