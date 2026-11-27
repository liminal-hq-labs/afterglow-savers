// A test program for the x86-64 worker pool: renders every scene with one thread and with eight, and checks every frame is the same.
// Not a shipped file; `scripts/smoke-wine.sh` runs it. Exit code 0: identical. 2: no pool (one processor, or the build is for the
// Pentium). 10 and up: the first scene whose frames differed, as 10 plus its place in the list.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use afterglow_win32::ExitProcess;

/// The frames of a run, hashed (FNV-1a over every pixel of every frame).
#[cfg(target_arch = "x86_64")]
fn run_hash(kind: savers_core::settings::SaverKind, bands: Option<&'static dyn savers_core::bands::Bands>) -> u64 {
	use savers_core::saver::{Saver, render_size};
	use savers_core::scenes::build;
	use savers_core::settings::{Key, Settings};
	let (w, h) = (257usize, 143usize);
	let mut settings = Settings::defaults(kind);
	settings.set(Key::Detail, 3);
	settings.set(Key::RenderScale, 100);
	settings.set(Key::Grain, 40);
	settings.set(Key::Letterbox, 1);
	let (rw, rh) = render_size(&settings, w, h, false);
	let scene = build(kind, &settings, 11, rw, rh);
	let mut saver = Saver::new(settings, scene, w, h, false);
	if let Some(bands) = bands {
		saver.set_bands(bands);
	}
	let mut hash = 0xCBF2_9CE4_8422_2325u64;
	let mut t = 0.0;
	// Long enough to cross the Ink's aurora, the Cracktro's first transition and the Maze's portal approach.
	while t <= 42.0 {
		for &c in &saver.at(t).px {
			hash = (hash ^ u64::from(c)).wrapping_mul(0x0100_0000_01B3);
		}
		t += 0.25;
	}
	hash
}

#[cfg(target_arch = "x86_64")]
fn check() -> u32 {
	use savers_core::settings::SaverKind;
	let Some(bands) = afterglow_win32::pool::start(8) else {
		return 2;
	};
	let kinds = [
		SaverKind::Ink,
		SaverKind::Twilight,
		SaverKind::Cracktro,
		SaverKind::Maze,
		SaverKind::Ghostwriter,
		SaverKind::Collection,
	];
	for (i, &kind) in kinds.iter().enumerate() {
		if run_hash(kind, None) != run_hash(kind, Some(bands)) {
			return 10 + i as u32;
		}
	}
	0
}

#[cfg(not(target_arch = "x86_64"))]
fn check() -> u32 {
	2
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
	let code = check();
	// SAFETY: ends the process.
	unsafe { ExitProcess(code) }
}
