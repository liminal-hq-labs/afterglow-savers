# Provenance of platforms/win16's toolchain

The packer, the import generator, the linker script and the runtime in `src/lib.rs` are copies from Quill's Win16 build, `win16/` at `659a038` on Quill's `origin/main` ("Add a zoom box to the Windows 3.x status bar", `~/source/experiments/quill16`, read with `git show origin/main:...`). Quill's copy descends from [`liminal-hq-labs/rust-win16`](https://github.com/liminal-hq-labs/rust-win16), branch `feat/multi-segment`, commit `4c0bac4`. They are copied rather than fetched, so a build needs nothing from the network, and this edition shares nothing else with Quill or with `savers-core`: the scene is 16-bit integer GDI written for it.

## What is copied, and what changed here

| File | From | Changed here |
|---|---|---|
| `win16.json`, `.cargo/config.toml`, `rustfmt.toml` | Quill's `win16/` | No |
| `link.ld` | Quill's `win16/link.ld` | No |
| `src/lib.rs`, `src/probe.rs`, `src/wizard.rs`, `src/fault.rs` | Quill's `win16/src/` | No |
| `tools/ne_pack.py`, `tools/check_segments.py`, `tools/check_cpu.py`, `tools/check_resources.py`, `tools/gen_imports.py` | Quill's `win16/tools/` | No. Quill's `ne_pack.py` is used, not rust-win16's, for its `icons`, `bitmaps` and `dialog_files` |
| `tools/ne_check.py`, `tools/ne_dump.py`, `tools/ne_addr2sym.py`, `tools/rust_demangle.py` | Quill's `win16/tools/` | No |
| `tests/test_ne_pack.py`, `tests/test_segments.py` | liminal-installer's `retro/win16/tests/` | No |
| `tools/imports.tbl`, `src/imports.rs` | Quill's `win16/tools/imports.tbl`, regenerated | Five rows added: `GetPrivateProfileInt` (KERNEL 127), `SetCursor` (USER 69), `ShowCursor` (USER 71), `DrawText` (USER 85) and `CreateWindowEx` (USER 452). The ordinals were read from Windows 3.11's own `KRNL386.EXE`, `USER.EXE` and `GDI.EXE` with `tools/ne_dump.py` |

Everything else (`src/bin/quillgw/`, `res/`, `build.sh`, `check.sh`, `tools/run-wine.sh`, `tools/run-w31.sh`, `tools/guest-reactos.py`) is this project's own.

## Refreshing

Compare the files above with the same paths in Quill's `origin/main`, copy the unchanged ones over, keep the five rows in `tools/imports.tbl`, regenerate `src/imports.rs` with `bash build.sh --regen`, run `bash check.sh`, and record the new commit here.
