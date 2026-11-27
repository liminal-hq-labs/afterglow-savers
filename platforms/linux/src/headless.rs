// The modes with no window: a screenshot, a bench, the adapter list. They draw into textures and read back, so they run with no
// display server at all.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::Path;
use std::time::Instant;

use savers_core::settings::{Key, Settings};
use savers_gpu::Gpu;

use crate::render;

#[path = "../../../crates/savers-preview/src/png.rs"]
mod png;

const DEFAULT_SIZE: (u32, u32) = (1920, 1080);

/// The scene time of a screenshot when none is given: the planet's first push-in is well under way.
pub const DEFAULT_SHOT_TIME: f64 = 40.0;

/// Renders one frame at `size` and `at` seconds of scene time, and writes it as a PNG.
pub fn shot(
	gpu: &Gpu,
	settings: &Settings,
	seed: u32,
	size: Option<(u32, u32)>,
	at: Option<f64>,
	path: &Path,
) -> Result<String, String> {
	let size = size.unwrap_or(DEFAULT_SIZE);
	let at = at.unwrap_or(DEFAULT_SHOT_TIME);
	let started = Instant::now();
	let (mut renderer, sizes) = render::build(gpu, settings, seed, size, false);
	let ready = started.elapsed().as_secs_f64();
	// A wall time of 100 s is long past the opening fade from black.
	renderer.render(gpu, at, 100.0);
	let frame = renderer.read_back(gpu);
	if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
		std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
	}
	png::write(path, frame.w, frame.h, &frame.px).map_err(|error| format!("{}: {error}", path.display()))?;
	Ok(format!(
		"{} at {}x{} (drawn at {}x{}, setup {ready:.1} s) on {}",
		path.display(),
		frame.w,
		frame.h,
		sizes.render.0,
		sizes.render.1,
		gpu.describe()
	))
}

/// Times the frames at 1080p and 4K, or at `size`, as the saver would draw them (at the settings' render scale).
pub fn bench(gpu: &Gpu, settings: &Settings, seed: u32, size: Option<(u32, u32)>, frames: u64) -> Result<String, String> {
	let sizes = match size {
		Some(size) => vec![size],
		None => vec![(1920, 1080), (3840, 2160)],
	};
	let mut report = format!(
		"adapter: {}\nscale {}%, detail {}\n{:<10} {:>12} {:>14} {:>14} {:>8}\n",
		gpu.describe(),
		settings.get(Key::RenderScale),
		settings.get(Key::Detail),
		"display",
		"drawn at",
		"ms a frame",
		"frames a sec",
		"setup s"
	);
	for display in sizes {
		let started = Instant::now();
		let (mut renderer, sizes) = render::build(gpu, settings, seed, display, false);
		let setup = started.elapsed().as_secs_f64();
		for step in 0..3 {
			renderer.render(gpu, 10.0 + step as f64 * 0.016, 100.0);
		}
		gpu.device
			.poll(wgpu::PollType::wait_indefinitely())
			.map_err(|error| error.to_string())?;
		let started = Instant::now();
		for step in 0..frames {
			renderer.render(gpu, 12.0 + step as f64 / 60.0, 100.0);
		}
		gpu.device
			.poll(wgpu::PollType::wait_indefinitely())
			.map_err(|error| error.to_string())?;
		let per_frame = started.elapsed().as_secs_f64() * 1000.0 / frames as f64;
		report.push_str(&format!(
			"{:<10} {:>12} {:>14.2} {:>14.1} {:>8.1}\n",
			format!("{}x{}", display.0, display.1),
			format!("{}x{}", sizes.render.0, sizes.render.1),
			per_frame,
			1000.0 / per_frame,
			setup
		));
	}
	Ok(report)
}
