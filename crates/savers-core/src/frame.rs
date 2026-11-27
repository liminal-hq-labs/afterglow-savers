// A 32-bit frame buffer, top row first.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::vec;
use alloc::vec::Vec;

use crate::colour::mix;

/// Pixels as 0x00RRGGBB, `w` per row, top row first.
#[derive(Clone)]
pub struct Frame {
	pub w: usize,
	pub h: usize,
	pub px: Vec<u32>,
}

impl Frame {
	pub fn new(w: usize, h: usize) -> Frame {
		let w = w.max(1);
		let h = h.max(1);
		Frame {
			w,
			h,
			px: vec![0; w * h],
		}
	}

	/// Changes the size, keeping the allocation when it is big enough. The contents are undefined afterwards.
	pub fn resize(&mut self, w: usize, h: usize) {
		self.w = w.max(1);
		self.h = h.max(1);
		self.px.resize(self.w * self.h, 0);
	}

	pub fn fill(&mut self, c: u32) {
		self.px.fill(c);
	}

	#[inline(always)]
	pub fn get(&self, x: usize, y: usize) -> u32 {
		self.px[y * self.w + x]
	}

	#[inline(always)]
	pub fn set(&mut self, x: usize, y: usize, c: u32) {
		let i = y * self.w + x;
		self.px[i] = c;
	}

	/// Sets a pixel when it is inside the frame.
	#[inline]
	pub fn put(&mut self, x: i32, y: i32, c: u32) {
		if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
			let i = y as usize * self.w + x as usize;
			self.px[i] = c;
		}
	}

	/// Bilinear sample at 16.16 coordinates, clamped to the edges.
	#[inline]
	pub fn sample(&self, sx: i32, sy: i32) -> u32 {
		let maxx = self.w as i32 - 1;
		let maxy = self.h as i32 - 1;
		let x0 = (sx >> 16).clamp(0, maxx);
		let y0 = (sy >> 16).clamp(0, maxy);
		let x1 = (x0 + 1).min(maxx);
		let y1 = (y0 + 1).min(maxy);
		let fx = ((sx >> 8) & 0xFF) as u32;
		let fy = ((sy >> 8) & 0xFF) as u32;
		let row0 = y0 as usize * self.w;
		let row1 = y1 as usize * self.w;
		let top = mix(self.px[row0 + x0 as usize], self.px[row0 + x1 as usize], fx);
		let bot = mix(self.px[row1 + x0 as usize], self.px[row1 + x1 as usize], fx);
		mix(top, bot, fy)
	}

	/// Copies `src` scaled to this frame's size, bilinear.
	pub fn scale_from(&mut self, src: &Frame) {
		let sx_step = ((src.w as i64) << 16) / self.w as i64;
		let sy_step = ((src.h as i64) << 16) / self.h as i64;
		for y in 0..self.h {
			let sy = (y as i64 * sy_step + sy_step / 2 - 32_768) as i32;
			let mut sx = (sx_step / 2 - 32_768) as i32;
			for x in 0..self.w {
				self.px[y * self.w + x] = src.sample(sx.max(0), sy.max(0));
				sx += sx_step as i32;
			}
		}
	}
}
