// AGCOLLCT.SCR: the Afterglow Collection screensaver.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

use savers_core::scene::Scene;
use savers_core::scenes::collection::Collection;
use savers_core::scenes::{cracktro::Cracktro, ink::Ink, maze::MazeScene, twilight::Twilight};
use savers_core::settings::{SaverKind, Settings};

/// The scenes the Collection links. Ghostwriter is left out: it ships with the Quill remix as its own `AGQUILL.SCR` (SAV-D22), so a
/// Collection asked for it plays Ink in its turn rather than carrying a fifth scene.
fn make(kind: SaverKind, settings: &Settings, seed: u32, w: usize, h: usize) -> Box<dyn Scene> {
	match kind {
		SaverKind::Cracktro => Box::new(Cracktro::new(settings, seed, w, h)),
		SaverKind::Maze => Box::new(MazeScene::new(settings, seed, w, h)),
		SaverKind::Twilight => Box::new(Twilight::new(settings, seed, w, h)),
		_ => Box::new(Ink::new(settings, seed, w, h)),
	}
}

// The Collection takes each member's own settings from that member's key.
afterglow_win32::screensaver_main!(SaverKind::Collection, |settings, seed, w, h| Box::new(Collection::new(
	settings,
	&afterglow_win32::member_settings(),
	seed,
	w,
	h,
	make
)));
