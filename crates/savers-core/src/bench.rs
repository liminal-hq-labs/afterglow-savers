// A reference workload for estimating frame times on a Pentium III from frame times on the host.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! [`reference`] does a fixed mix of what the scenes do per pixel: packed colour mixing, 16.16 fixed-point texture walks with
//! table reads, a little float maths and streaming writes, over a 640 × 480 buffer. Its time on a Pentium III at 800 MHz is taken as
//! [`P3_REFERENCE_MS`]; the preview tool's `bench` times it on the host and scales every frame time by the ratio. The constant is an
//! estimate from instruction counts (about 5.5 million instructions at an IPC near 0.9 for this mix on a P6 core), to be replaced
//! by a measurement in the guest when one is made (see docs/testing.md).

use alloc::vec;

use crate::colour::mix;
use crate::math::{fract, sin};

/// The reference loop's estimated time on a Pentium III 800 MHz, in milliseconds.
pub const P3_REFERENCE_MS: f64 = 7.5;

/// Runs the reference workload once and returns a checksum so it cannot be optimized away.
pub fn reference() -> u32 {
	let (w, h) = (640usize, 480usize);
	let mut tex = vec![0u32; 256 * 256];
	for (i, t) in tex.iter_mut().enumerate() {
		*t = (i as u32).wrapping_mul(2_654_435_761);
	}
	let mut buf = vec![0u32; w * h];
	let mut sum = 0u32;
	for y in 0..h {
		let a = sin(y as f32 * 0.01);
		let du = (a * 65_536.0) as i32;
		let dv = (fract(a * 3.0) * 65_536.0) as i32;
		let mut u = (y as i32) << 16;
		let mut v = 0i32;
		for x in 0..w {
			let t0 = tex[(((v >> 16) & 255) << 8 | ((u >> 16) & 255)) as usize];
			let t1 = tex[((((v >> 16) + 1) & 255) << 8 | ((u >> 16) & 255)) as usize];
			let c = mix(t0, t1, ((v >> 8) & 255) as u32);
			buf[y * w + x] = c;
			sum = sum.wrapping_add(c);
			u = u.wrapping_add(du);
			v = v.wrapping_add(dv);
		}
	}
	sum
}
