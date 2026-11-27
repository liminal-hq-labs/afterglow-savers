// Packed 0x00RRGGBB colours, fast mixing, and 256-step ramps.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A pixel is a `u32` holding 0x00RRGGBB, which is also the byte order of a 32-bit bottom-up DIB on little-endian Windows, so a frame
//! goes to `StretchDIBits` as it is.

use crate::math::{clamp, saturate};

#[inline(always)]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
	((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

#[inline(always)]
pub const fn r(c: u32) -> u32 {
	(c >> 16) & 0xFF
}

#[inline(always)]
pub const fn g(c: u32) -> u32 {
	(c >> 8) & 0xFF
}

#[inline(always)]
pub const fn b(c: u32) -> u32 {
	c & 0xFF
}

/// Mixes two colours by `t` in [0, 256] (256 is all of `b`), two channels at a time.
#[inline(always)]
pub fn mix(a: u32, b: u32, t: u32) -> u32 {
	let t = t.min(256);
	let it = 256 - t;
	let rb = ((a & 0xFF00FF) * it + (b & 0xFF00FF) * t) >> 8;
	let g = ((a & 0x00FF00) * it + (b & 0x00FF00) * t) >> 8;
	(rb & 0xFF00FF) | (g & 0x00FF00)
}

/// Mixes by a float in [0, 1].
#[inline]
pub fn mixf(a: u32, b: u32, t: f32) -> u32 {
	mix(a, b, (saturate(t) * 256.0) as u32)
}

/// Scales a colour by `s` in [0, 256].
#[inline(always)]
pub fn scale(c: u32, s: u32) -> u32 {
	let s = s.min(256);
	let rb = ((c & 0xFF00FF) * s) >> 8;
	let g = ((c & 0x00FF00) * s) >> 8;
	(rb & 0xFF00FF) | (g & 0x00FF00)
}

/// Adds two colours, saturating each channel at 255.
#[inline(always)]
pub fn add(a: u32, b: u32) -> u32 {
	let rr = (r(a) + r(b)).min(255);
	let gg = (g(a) + g(b)).min(255);
	let bb = (self::b(a) + self::b(b)).min(255);
	(rr << 16) | (gg << 8) | bb
}

/// Screen blend: 1 - (1 - a)(1 - b) per channel. Brightens without clipping hard.
#[inline(always)]
pub fn screen(a: u32, b: u32) -> u32 {
	let ch = |x: u32, y: u32| x + y - ((x * y + 127) / 255);
	(ch(r(a), r(b)) << 16) | (ch(g(a), g(b)) << 8) | ch(self::b(a), self::b(b))
}

/// Rec. 601 luma, 0 to 255.
#[inline(always)]
pub fn luma(c: u32) -> u32 {
	(r(c) * 77 + g(c) * 150 + b(c) * 29) >> 8
}

/// A colour from float channels in [0, 1].
#[inline]
pub fn from_f(rf: f32, gf: f32, bf: f32) -> u32 {
	let q = |v: f32| (clamp(v, 0.0, 1.0) * 255.0 + 0.5) as u32;
	(q(rf) << 16) | (q(gf) << 8) | q(bf)
}

/// Float channels in [0, 1].
#[inline]
pub fn to_f(c: u32) -> [f32; 3] {
	[r(c) as f32 / 255.0, g(c) as f32 / 255.0, b(c) as f32 / 255.0]
}

/// Lifts a colour towards white (t > 0) or darkens it towards black (t < 0), t in [-1, 1].
#[inline]
pub fn shade(c: u32, t: f32) -> u32 {
	if t >= 0.0 { mixf(c, 0xFFFFFF, t) } else { mixf(c, 0, -t) }
}

/// 256 colours along a path of stops, for mapping an intensity to a colour.
#[derive(Clone)]
pub struct Ramp {
	pub c: [u32; 256],
}

impl Ramp {
	/// Stops are (position in [0, 1], colour), in increasing position. Between stops the colours mix linearly in sRGB.
	pub fn new(stops: &[(f32, u32)]) -> Self {
		let mut c = [0u32; 256];
		for (i, slot) in c.iter_mut().enumerate() {
			let x = i as f32 / 255.0;
			*slot = sample_stops(stops, x);
		}
		Ramp { c }
	}

	/// The colour at `x` in [0, 1].
	#[inline(always)]
	pub fn at(&self, x: f32) -> u32 {
		self.c[(saturate(x) * 255.0) as usize]
	}

	/// The colour at an index, wrapped.
	#[inline(always)]
	pub fn idx(&self, i: i32) -> u32 {
		self.c[(i & 255) as usize]
	}
}

/// The colour along a path of stops at `x`.
pub fn sample_stops(stops: &[(f32, u32)], x: f32) -> u32 {
	match stops {
		[] => 0,
		[(_, only)] => *only,
		_ => {
			if x <= stops[0].0 {
				return stops[0].1;
			}
			for w in stops.windows(2) {
				let (p0, c0) = w[0];
				let (p1, c1) = w[1];
				if x <= p1 {
					let t = if p1 > p0 { (x - p0) / (p1 - p0) } else { 1.0 };
					return mixf(c0, c1, t);
				}
			}
			stops[stops.len() - 1].1
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mixing() {
		assert_eq!(mix(0x000000, 0xFFFFFF, 256), 0xFFFFFF);
		assert_eq!(mix(0x102030, 0xFFFFFF, 0), 0x102030);
		assert_eq!(mix(0x000000, 0xFF8040, 128), 0x7F4020);
		assert_eq!(add(0xF0F0F0, 0x202020), 0xFFFFFF);
		assert_eq!(screen(0x000000, 0x804020), 0x804020);
		assert_eq!(screen(0xFFFFFF, 0x804020), 0xFFFFFF);
		assert_eq!(scale(0xFF8040, 128), 0x7F4020);
	}

	#[test]
	fn ramps_hit_their_stops() {
		let ramp = Ramp::new(&[(0.0, 0x000000), (0.5, 0xFF0000), (1.0, 0xFFFFFF)]);
		assert_eq!(ramp.c[0], 0);
		assert_eq!(ramp.c[255], 0xFFFFFF);
		assert_eq!(r(ramp.c[127]), 253);
	}
}
