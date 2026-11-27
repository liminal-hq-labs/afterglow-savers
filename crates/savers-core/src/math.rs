// The core's own maths: trigonometry, roots, exponentials and fast conversions, with no libm.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Every function here is plain arithmetic, so it gives the same answer on every build of one target (SAV-D5, SAV-D6). They are
//! accurate to a few parts in a million over the ranges the scenes use, which is far below what a pixel can show.

use core::ops::{Add, Mul, Neg, Sub};

pub const PI: f32 = core::f32::consts::PI;
pub const TAU: f32 = core::f32::consts::TAU;
pub const HALF_PI: f32 = core::f32::consts::FRAC_PI_2;

/// 1.5 × 2^23: adding it to a float below 2^22 in size leaves the rounded integer in the low mantissa bits.
const MAGIC: f32 = 12_582_912.0;

/// The nearest integer, ties to even, for |x| < 2^22. Much faster than `as i32` on x87, which must change the rounding mode.
#[inline(always)]
pub fn round_i(x: f32) -> i32 {
	((x + MAGIC).to_bits() as i32).wrapping_sub(0x4B40_0000)
}

/// The largest integer not above `x`, for |x| < 2^22.
#[inline(always)]
pub fn floor_i(x: f32) -> i32 {
	let r = round_i(x);
	if r as f32 > x { r - 1 } else { r }
}

#[inline(always)]
pub fn floor(x: f32) -> f32 {
	floor_i(x) as f32
}

/// The fractional part, in [0, 1).
#[inline(always)]
pub fn fract(x: f32) -> f32 {
	x - floor(x)
}

/// `x` as 16.16 fixed point, for |x| < 32 768.
#[inline(always)]
pub fn fx(x: f32) -> i32 {
	round_i(x * 65_536.0)
}

#[inline(always)]
pub fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
	if x < lo {
		lo
	} else if x > hi {
		hi
	} else {
		x
	}
}

#[inline(always)]
pub fn saturate(x: f32) -> f32 {
	clamp(x, 0.0, 1.0)
}

#[inline(always)]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

#[inline(always)]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
	let t = saturate((x - e0) / (e1 - e0));
	t * t * (3.0 - 2.0 * t)
}

#[inline(always)]
pub fn min(a: f32, b: f32) -> f32 {
	if a < b { a } else { b }
}

#[inline(always)]
pub fn max(a: f32, b: f32) -> f32 {
	if a > b { a } else { b }
}

/// `x` folded into [-π, π].
#[inline(always)]
pub fn wrap_pi(x: f32) -> f32 {
	x - TAU * floor(x * (1.0 / TAU) + 0.5)
}

/// Sine, by folding into [-π/2, π/2] and an odd Taylor polynomial to x^11 (error below 6e-8 there).
pub fn sin(x: f32) -> f32 {
	let mut a = wrap_pi(x);
	if a > HALF_PI {
		a = PI - a;
	} else if a < -HALF_PI {
		a = -PI - a;
	}
	let a2 = a * a;
	a * (1.0
		+ a2 * (-1.0 / 6.0 + a2 * (1.0 / 120.0 + a2 * (-1.0 / 5040.0 + a2 * (1.0 / 362_880.0 + a2 * (-1.0 / 39_916_800.0))))))
}

#[inline(always)]
pub fn cos(x: f32) -> f32 {
	sin(x + HALF_PI)
}

/// Square root: a bit-level first guess refined by three Newton steps. Zero and negative inputs give zero.
pub fn sqrt(x: f32) -> f32 {
	if x <= 0.0 {
		return 0.0;
	}
	let mut y = f32::from_bits((x.to_bits() >> 1).wrapping_add(0x1FBD_1DF5));
	y = 0.5 * (y + x / y);
	y = 0.5 * (y + x / y);
	0.5 * (y + x / y)
}

/// 1 / √x, for positive x.
#[inline]
pub fn rsqrt(x: f32) -> f32 {
	1.0 / sqrt(x)
}

/// e^x, by splitting off a power of two and a degree-6 polynomial. Clamped to (0, 2^126].
pub fn exp(x: f32) -> f32 {
	let x = clamp(x, -87.0, 87.0);
	let t = x * core::f32::consts::LOG2_E;
	let n = floor_i(t);
	let f = (t - n as f32) * core::f32::consts::LN_2;
	let p = 1.0
		+ f * (1.0 + f * (0.5 + f * (1.0 / 6.0 + f * (1.0 / 24.0 + f * (1.0 / 120.0 + f * (1.0 / 720.0 + f * (1.0 / 5040.0)))))));
	p * f32::from_bits(((n + 127) as u32) << 23)
}

/// The natural logarithm of a positive number, from the exponent and an atanh series on the mantissa.
pub fn ln(x: f32) -> f32 {
	if x <= 0.0 {
		return -87.0;
	}
	let bits = x.to_bits();
	let e = ((bits >> 23) & 0xFF) as i32 - 127;
	let m = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
	let s = (m - 1.0) / (m + 1.0);
	let s2 = s * s;
	let series = 2.0 * s * (1.0 + s2 * (1.0 / 3.0 + s2 * (1.0 / 5.0 + s2 * (1.0 / 7.0 + s2 * (1.0 / 9.0)))));
	series + e as f32 * core::f32::consts::LN_2
}

/// x^y for positive x.
#[inline]
pub fn powf(x: f32, y: f32) -> f32 {
	if x <= 0.0 { 0.0 } else { exp(y * ln(x)) }
}

/// atan2(y, x) in [-π, π], from a rational approximation of atan on [0, 1] (error about 1e-5).
pub fn atan2(y: f32, x: f32) -> f32 {
	let ax = x.abs();
	let ay = y.abs();
	if ax == 0.0 && ay == 0.0 {
		return 0.0;
	}
	let (num, den, swap) = if ay > ax { (ax, ay, true) } else { (ay, ax, false) };
	let z = num / den;
	let z2 = z * z;
	let mut a = z * (0.999_866 + z2 * (-0.330_299_5 + z2 * (0.180_141 + z2 * (-0.085_133 + z2 * 0.020_835_1))));
	if swap {
		a = HALF_PI - a;
	}
	if x < 0.0 {
		a = PI - a;
	}
	if y < 0.0 { -a } else { a }
}

/// asin(x) for x in [-1, 1].
pub fn asin(x: f32) -> f32 {
	let x = clamp(x, -1.0, 1.0);
	atan2(x, sqrt(1.0 - x * x))
}

/// A 3-vector of floats.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

impl V3 {
	pub const fn new(x: f32, y: f32, z: f32) -> Self {
		V3 { x, y, z }
	}

	#[inline]
	pub fn dot(self, o: V3) -> f32 {
		self.x * o.x + self.y * o.y + self.z * o.z
	}

	#[inline]
	pub fn cross(self, o: V3) -> V3 {
		V3::new(
			self.y * o.z - self.z * o.y,
			self.z * o.x - self.x * o.z,
			self.x * o.y - self.y * o.x,
		)
	}

	#[inline]
	pub fn len(self) -> f32 {
		sqrt(self.dot(self))
	}

	pub fn norm(self) -> V3 {
		let l = self.len();
		if l > 0.0 { self * (1.0 / l) } else { self }
	}
}

impl Add for V3 {
	type Output = V3;
	#[inline]
	fn add(self, o: V3) -> V3 {
		V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
	}
}

impl Sub for V3 {
	type Output = V3;
	#[inline]
	fn sub(self, o: V3) -> V3 {
		V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
	}
}

impl Mul<f32> for V3 {
	type Output = V3;
	#[inline]
	fn mul(self, s: f32) -> V3 {
		V3::new(self.x * s, self.y * s, self.z * s)
	}
}

impl Neg for V3 {
	type Output = V3;
	#[inline]
	fn neg(self) -> V3 {
		V3::new(-self.x, -self.y, -self.z)
	}
}

/// Smooth ease in and out over [0, 1].
#[inline]
pub fn ease(t: f32) -> f32 {
	let t = saturate(t);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn close(a: f32, b: f32, tol: f32) -> bool {
		(a - b).abs() <= tol
	}

	#[test]
	fn trig_matches_std() {
		let mut x = -40.0f32;
		while x < 40.0 {
			assert!(close(sin(x), (x as f64).sin() as f32, 2e-6 + x.abs() * 1.5e-7), "sin {x}");
			assert!(close(cos(x), (x as f64).cos() as f32, 2e-6 + x.abs() * 1.5e-7), "cos {x}");
			x += 0.0137;
		}
	}

	#[test]
	fn roots_and_exponentials_match_std() {
		for i in 1..5000 {
			let x = i as f32 * 0.173;
			assert!(close(sqrt(x), (x as f64).sqrt() as f32, 1e-5 * x.max(1.0)), "sqrt {x}");
			assert!(close(ln(x), (x as f64).ln() as f32, 2e-5), "ln {x}");
		}
		for i in -200..200 {
			let x = i as f32 * 0.05;
			let want = (x as f64).exp() as f32;
			assert!(close(exp(x), want, 4e-6 * want.max(1.0)), "exp {x}");
		}
		assert_eq!(sqrt(0.0), 0.0);
		assert_eq!(sqrt(-1.0), 0.0);
	}

	#[test]
	fn atan2_matches_std() {
		for i in -50..50 {
			for j in -50..50 {
				let (y, x) = (i as f32 * 0.31, j as f32 * 0.27);
				if x == 0.0 && y == 0.0 {
					continue;
				}
				assert!(close(atan2(y, x), (y as f64).atan2(x as f64) as f32, 3e-5), "atan2 {y} {x}");
			}
		}
	}

	#[test]
	fn rounding_and_floor() {
		assert_eq!(round_i(2.4), 2);
		assert_eq!(round_i(-2.6), -3);
		assert_eq!(floor_i(-0.5), -1);
		assert_eq!(floor_i(3.0), 3);
		assert_eq!(floor_i(-3.0), -3);
		assert!(close(fract(-1.25), 0.75, 1e-6));
		assert_eq!(fx(1.5), 98_304);
	}
}
