// AGCOLLCT.SCR: the Afterglow Collection screensaver.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

use savers_core::audio::collection::CollectionScore;
use savers_core::scene::Scene;
use savers_core::scenes::collection::Collection;
use savers_core::scenes::{cracktro::Cracktro, ghostwriter::Ghostwriter, ink::Ink, maze::MazeScene, twilight::Twilight};
use savers_core::settings::{SaverKind, Settings};

/// The scenes the Collection plays. Quill Ghostwriter is opt-in (the `Savers` mask) and plays only where Quill has installed
/// `AGQUILL.SCR` beside this file, so it stays absent on a system without Quill (SAV-D22, SAV-D29); otherwise its turn is Ink's.
fn make(kind: SaverKind, settings: &Settings, seed: u32, w: usize, h: usize) -> Box<dyn Scene> {
	match kind {
		SaverKind::Ghostwriter if afterglow_win32::installed_beside("AGQUILL.SCR") => {
			Box::new(Ghostwriter::new(settings, seed, w, h))
		}
		SaverKind::Cracktro => Box::new(Cracktro::new(settings, seed, w, h)),
		SaverKind::Maze => Box::new(MazeScene::new(settings, seed, w, h)),
		SaverKind::Twilight => Box::new(Twilight::new(settings, seed, w, h)),
		_ => Box::new(Ink::new(settings, seed, w, h)),
	}
}

// The Collection takes each member's own settings from that member's key.
afterglow_win32::screensaver_main!(
	SaverKind::Collection,
	|settings, seed, w, h| Box::new(Collection::new(
		settings,
		&afterglow_win32::member_settings(),
		seed,
		w,
		h,
		make
	)),
	|settings, seed, rate| Some(Box::new(CollectionScore::new(settings, seed, rate)))
);
