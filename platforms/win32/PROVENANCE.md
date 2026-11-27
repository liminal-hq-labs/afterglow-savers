# Provenance

The Win32 shell reuses the proven no-C-runtime pieces of afterfetch's Win32 platform (`liminal-hq-labs/afterfetch`, `platforms/win32`), itself after Quill's `rt.rs` through the Liminal Installer's Win32 runtime. Both projects are Apache-2.0 OR MIT.

| Here | There | Change |
| --- | --- | --- |
| `src/rt_x86.rs` | `src/rt_x86.rs` | None: the 64-bit division helpers and the 32-bit stack probe |
| `src/rt_probe.rs` | `src/rt_probe.rs` | None: the 64-bit stack probe |
| `src/rt.rs` | `src/rt.rs` | The panic handler exits 70 as there; a `HeapAlloc` global allocator added (SAV-D3) |
| `i586-win32.json` | `i586-win32.json` | `rustc-abi` (x86-softfloat) and `+soft-float`, `-x87` removed: x87 hardware floats, with MMX and SSE still off (SAV-D4) |
| `x86_64-win32.json` | `x86_64-win32.json` | The soft-float features and `rustc-abi` removed: the SSE2 baseline |
| `.cargo/config.toml` | `.cargo/config.toml` | `/SUBSYSTEM:WINDOWS` instead of console; the `aarch64` target dropped |
| `tools/*.sh` | `tools/*.sh` | GUI subsystem, `.rsrc` section, `32` and `64` import markers, `fcmov` and `fcomi` in the Pentium scan |
| `build.sh`, `check.sh`, `../../scripts/repro-win32.sh` | the same | Five programs, size budgets of SAV-D14 |
