// Holds a GPU frame to a CPU frame: the numbers the parity test and the comparison tool both report.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use savers_core::frame::Frame;
use savers_core::saver::{Saver, render_size};
use savers_core::scenes::build;
use savers_core::settings::{SaverKind, Settings};

/// How far apart two frames of the same size are.
#[derive(Clone, Copy, Debug)]
pub struct Difference {
	/// The mean absolute difference per channel (red, green, blue), in levels of 255.
	pub mean_abs: [f64; 3],
	/// The share of pixels where any channel is more than [`FAR`] levels apart, 0 to 1.
	pub far_share: f64,
	/// The largest single-channel difference.
	pub worst: u32,
}

/// A pixel is "far" when a channel differs by more than this many levels.
pub const FAR: u32 = 24;

/// The parity budget of `docs/gpu-twilight.md`: the mean absolute error per channel, and the share of far pixels.
pub const MAX_MEAN_ABS: f64 = 6.0;
pub const MAX_FAR_SHARE: f64 = 0.02;

impl Difference {
	pub fn within_budget(&self) -> bool {
		self.mean_abs.iter().all(|&m| m <= MAX_MEAN_ABS) && self.far_share <= MAX_FAR_SHARE
	}

	pub fn describe(&self) -> String {
		format!(
			"mean abs error r {:.2} g {:.2} b {:.2} levels, {:.3}% of pixels more than {FAR} apart, worst {}",
			self.mean_abs[0],
			self.mean_abs[1],
			self.mean_abs[2],
			self.far_share * 100.0,
			self.worst
		)
	}
}

/// Compares two frames of the same size.
pub fn difference(a: &Frame, b: &Frame) -> Difference {
	assert_eq!((a.w, a.h), (b.w, b.h), "frames to compare must be the same size");
	let mut sums = [0u64; 3];
	let mut far = 0usize;
	let mut worst = 0u32;
	for (&x, &y) in a.px.iter().zip(&b.px) {
		let mut is_far = false;
		for (channel, shift) in [16u32, 8, 0].into_iter().enumerate() {
			let d = ((x >> shift) & 0xFF).abs_diff((y >> shift) & 0xFF);
			sums[channel] += d as u64;
			worst = worst.max(d);
			is_far |= d > FAR;
		}
		far += is_far as usize;
	}
	let n = a.px.len() as f64;
	Difference {
		mean_abs: [sums[0] as f64 / n, sums[1] as f64 / n, sums[2] as f64 / n],
		far_share: far as f64 / n,
		worst,
	}
}

/// The CPU scene's frames, to compare with: a Twilight saver for `settings` and `seed` at `window` size, asked for the scene
/// times in `times` in order. Returns each frame.
pub fn cpu_frames(settings: &Settings, seed: u32, window: (usize, usize), times: &[f64]) -> Vec<Frame> {
	let (rw, rh) = render_size(settings, window.0, window.1, false);
	let scene = build(SaverKind::Twilight, settings, seed, rw, rh);
	let mut saver = Saver::new(settings.clone(), scene, window.0, window.1, false);
	times.iter().map(|&t| saver.at(t).clone()).collect()
}
