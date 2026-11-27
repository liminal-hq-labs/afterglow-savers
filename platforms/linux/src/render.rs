// Builds the renderer for a display size and the settings, and the seed and time conventions the window and the headless modes share.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use savers_core::saver::{output_size, render_size};
use savers_core::settings::{Key, Settings};
use savers_core::twilight_shared::Shared;
use savers_gpu::{Gpu, TwilightRenderer, shared_for_gpu};

/// The sizes a renderer was made for, for a log line.
#[derive(Clone, Copy, Debug)]
pub struct Sizes {
	pub display: (u32, u32),
	pub render: (usize, usize),
	pub output: (usize, usize),
}

/// A renderer for a display of `display` pixels. The frame is drawn at the render scale and finished at the detail level's output
/// size; the presenter enlarges it to the display. A preview uses the small maps of the CPU scene, which paint much faster.
pub fn build(gpu: &Gpu, settings: &Settings, seed: u32, display: (u32, u32), preview: bool) -> (TwilightRenderer, Sizes) {
	let window = (display.0 as usize, display.1 as usize);
	let render = render_size(settings, window.0, window.1, preview);
	let output = if preview {
		render
	} else {
		output_size(settings.detail(), window, render)
	};
	let shared = if preview {
		Shared::new(settings, seed, render.0, render.1)
	} else {
		shared_for_gpu(settings, seed, render)
	};
	let renderer = TwilightRenderer::new(gpu, settings, shared, seed, render, output, preview);
	(renderer, Sizes { display, render, output })
}

/// The seed to use: the command line's, else the setting's, else (for 0) one from the clock, so each run is a new planet.
pub fn choose_seed(given: Option<u32>, settings: &Settings) -> u32 {
	let wanted = given.unwrap_or_else(|| settings.get(Key::Seed));
	if wanted != 0 {
		return wanted;
	}
	let nanos = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.subsec_nanos() ^ (d.as_secs() as u32).wrapping_mul(2_654_435_761))
		.unwrap_or(1);
	nanos.max(1)
}
