// Gradient noise in two and three dimensions, tileable, and its fractal sums.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Perlin-style gradient noise with a quintic fade, hashed from the lattice point and a seed, so no permutation table is needed and
//! every seed gives a different field. Values are roughly in [-1, 1]. These run at startup (textures, planets) and on coarse grids;
//! no scene calls them per pixel per frame.

use crate::math::floor_i;
use crate::rng::{hash2, hash3};

#[inline(always)]
fn fade(t: f32) -> f32 {
	t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
fn grad2(h: u32, x: f32, y: f32) -> f32 {
	// Eight directions: the axes and the diagonals.
	match h & 7 {
		0 => x + y,
		1 => x - y,
		2 => -x + y,
		3 => -x - y,
		4 => x * 1.414,
		5 => -x * 1.414,
		6 => y * 1.414,
		_ => -y * 1.414,
	}
}

#[inline(always)]
fn grad3(h: u32, x: f32, y: f32, z: f32) -> f32 {
	// The twelve edge directions of a cube (and four repeats), as Perlin's improved noise.
	match h & 15 {
		0 | 12 => x + y,
		1 | 13 => -x + y,
		2 => x - y,
		3 => -x - y,
		4 => x + z,
		5 => -x + z,
		6 => x - z,
		7 => -x - z,
		8 => y + z,
		9 | 14 => -y + z,
		10 => y - z,
		_ => -y - z,
	}
}

/// 2D gradient noise whose lattice repeats every `period` cells in x and y (a period of 0 means no repeat).
pub fn noise2_tiled(x: f32, y: f32, period: i32, seed: u32) -> f32 {
	let xi = floor_i(x);
	let yi = floor_i(y);
	let fx = x - xi as f32;
	let fy = y - yi as f32;
	let wrap = |v: i32| if period > 0 { v.rem_euclid(period) } else { v };
	let (x0, x1, y0, y1) = (wrap(xi), wrap(xi + 1), wrap(yi), wrap(yi + 1));
	let n00 = grad2(hash2(x0, y0, seed), fx, fy);
	let n10 = grad2(hash2(x1, y0, seed), fx - 1.0, fy);
	let n01 = grad2(hash2(x0, y1, seed), fx, fy - 1.0);
	let n11 = grad2(hash2(x1, y1, seed), fx - 1.0, fy - 1.0);
	let u = fade(fx);
	let v = fade(fy);
	let a = n00 + (n10 - n00) * u;
	let b = n01 + (n11 - n01) * u;
	(a + (b - a) * v) * 0.7
}

/// 2D gradient noise.
#[inline]
pub fn noise2(x: f32, y: f32, seed: u32) -> f32 {
	noise2_tiled(x, y, 0, seed)
}

/// 3D gradient noise.
pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
	let xi = floor_i(x);
	let yi = floor_i(y);
	let zi = floor_i(z);
	let fx = x - xi as f32;
	let fy = y - yi as f32;
	let fz = z - zi as f32;
	let g = |dx: i32, dy: i32, dz: i32| {
		grad3(
			hash3(xi + dx, yi + dy, zi + dz, seed),
			fx - dx as f32,
			fy - dy as f32,
			fz - dz as f32,
		)
	};
	let u = fade(fx);
	let v = fade(fy);
	let w = fade(fz);
	let x00 = g(0, 0, 0) + (g(1, 0, 0) - g(0, 0, 0)) * u;
	let x10 = g(0, 1, 0) + (g(1, 1, 0) - g(0, 1, 0)) * u;
	let x01 = g(0, 0, 1) + (g(1, 0, 1) - g(0, 0, 1)) * u;
	let x11 = g(0, 1, 1) + (g(1, 1, 1) - g(0, 1, 1)) * u;
	let y0 = x00 + (x10 - x00) * v;
	let y1 = x01 + (x11 - x01) * v;
	(y0 + (y1 - y0) * w) * 0.9
}

/// Fractal sum of 2D noise: `octaves` layers, each twice the frequency and `gain` times the amplitude. Normalized to about [-1, 1].
pub fn fbm2(x: f32, y: f32, octaves: u32, gain: f32, seed: u32) -> f32 {
	fbm2_tiled(x, y, 0, octaves, gain, seed)
}

/// Tileable fractal sum: the period doubles with each octave, so the sum repeats every `period` units of the first octave.
pub fn fbm2_tiled(x: f32, y: f32, period: i32, octaves: u32, gain: f32, seed: u32) -> f32 {
	let mut sum = 0.0;
	let mut amp = 1.0;
	let mut norm = 0.0;
	let mut f = 1.0;
	let mut p = period;
	for o in 0..octaves {
		sum += amp * noise2_tiled(x * f, y * f, p, seed.wrapping_add(o.wrapping_mul(0x632B_E5AB)));
		norm += amp;
		amp *= gain;
		f *= 2.0;
		p *= 2;
	}
	sum / norm
}

/// Fractal sum of 3D noise, normalized to about [-1, 1].
pub fn fbm3(x: f32, y: f32, z: f32, octaves: u32, gain: f32, seed: u32) -> f32 {
	let mut sum = 0.0;
	let mut amp = 1.0;
	let mut norm = 0.0;
	let mut f = 1.0;
	for o in 0..octaves {
		sum += amp * noise3(x * f, y * f, z * f, seed.wrapping_add(o.wrapping_mul(0x632B_E5AB)));
		norm += amp;
		amp *= gain;
		f *= 2.0;
	}
	sum / norm
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stays_in_range_and_varies() {
		let (mut lo, mut hi) = (0.0f32, 0.0f32);
		for i in 0..200 {
			for j in 0..200 {
				let v = noise2(i as f32 * 0.137, j as f32 * 0.091, 7);
				let w = noise3(i as f32 * 0.137, j as f32 * 0.091, 0.5, 7);
				lo = lo.min(v).min(w);
				hi = hi.max(v).max(w);
			}
		}
		assert!(lo > -1.05 && hi < 1.05, "{lo} {hi}");
		assert!(lo < -0.5 && hi > 0.5, "{lo} {hi}");
	}

	#[test]
	fn tiles() {
		for i in 0..50 {
			let y = i as f32 * 0.31;
			let a = fbm2_tiled(0.25, y, 8, 4, 0.5, 3);
			let b = fbm2_tiled(8.25, y, 8, 4, 0.5, 3);
			assert!((a - b).abs() < 1e-5);
		}
	}
}
