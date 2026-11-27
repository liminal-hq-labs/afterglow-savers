// What a scene is: something that draws a frame for a time, and tells the finishing chain how to treat it.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::frame::Frame;
use crate::palette::Palette;
use crate::settings::Detail;

/// Everything a scene needs to draw one frame.
pub struct Ctx<'a> {
	/// Scene time in seconds, already scaled by the speed setting. Never goes backwards.
	pub t: f64,
	/// Scene seconds since the last frame (0 for the first), at most 0.1.
	pub dt: f32,
	pub detail: Detail,
	pub pal: &'a Palette,
	/// Counts frames, for anything that should change every frame whatever the time (dither, grain).
	pub frame_no: u32,
}

/// How the finishing chain should treat the current frame.
#[derive(Clone, Copy, Debug)]
pub struct Hints {
	/// Luma above which a pixel glows.
	pub threshold: u32,
	/// The colour of the anamorphic streaks.
	pub streak_tint: u32,
	/// The scene's own fade for a cut between shots, 0 (black) to 1. Applied only when fades are on.
	pub fade: f32,
	/// Multiplies the bloom setting, for scenes that are mostly bright or mostly dark.
	pub bloom_gain: f32,
}

impl Default for Hints {
	fn default() -> Self {
		Hints {
			threshold: 150,
			streak_tint: 0xFFB070,
			fade: 1.0,
			bloom_gain: 1.0,
		}
	}
}

pub trait Scene {
	/// Draws the frame for `cx.t` into `out`, whose size is the render size the scene was made for.
	fn render(&mut self, cx: &Ctx, out: &mut Frame);

	/// The length in seconds after which the scene repeats exactly, when it does: frame(t) equals frame(t + length).
	fn loop_length(&self) -> Option<f64> {
		None
	}

	/// How to finish the frame just drawn.
	fn hints(&self, _cx: &Ctx) -> Hints {
		Hints::default()
	}
}
