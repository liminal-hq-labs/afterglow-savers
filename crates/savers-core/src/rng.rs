// Seeded random numbers and integer hashes, the same on every platform.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// A small PCG32 generator: 64 bits of state, 32-bit output.
#[derive(Clone, Debug)]
pub struct Rng {
	state: u64,
}

impl Rng {
	pub fn new(seed: u64) -> Self {
		let mut r = Rng {
			state: seed.wrapping_add(0x853C_49E6_748F_EA9B),
		};
		r.next_u32();
		r
	}

	pub fn next_u32(&mut self) -> u32 {
		let old = self.state;
		self.state = old
			.wrapping_mul(6_364_136_223_846_793_005)
			.wrapping_add(1_442_695_040_888_963_407);
		let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
		let rot = (old >> 59) as u32;
		xorshifted.rotate_right(rot)
	}

	/// A float in [0, 1).
	pub fn f(&mut self) -> f32 {
		(self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
	}

	/// A float in [lo, hi).
	pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
		lo + (hi - lo) * self.f()
	}

	/// An integer in [0, n), or 0 when n is 0.
	pub fn below(&mut self, n: u32) -> u32 {
		if n == 0 {
			0
		} else {
			((self.next_u32() as u64 * n as u64) >> 32) as u32
		}
	}

	/// A float in [-1, 1).
	pub fn signed(&mut self) -> f32 {
		self.f() * 2.0 - 1.0
	}

	/// A child generator for one part of a scene, so adding draws in one part does not shift another.
	pub fn fork(&mut self, salt: u32) -> Rng {
		Rng::new(((self.next_u32() as u64) << 32) ^ salt as u64)
	}
}

/// A 32-bit integer hash with good avalanche (the "lowbias32" mix).
#[inline(always)]
pub fn hash32(mut x: u32) -> u32 {
	x ^= x >> 16;
	x = x.wrapping_mul(0x7FEB_352D);
	x ^= x >> 15;
	x = x.wrapping_mul(0x846C_A68B);
	x ^= x >> 16;
	x
}

/// Hashes two integers with a seed.
#[inline(always)]
pub fn hash2(x: i32, y: i32, seed: u32) -> u32 {
	hash32((x as u32).wrapping_mul(0x9E37_79B1) ^ hash32((y as u32).wrapping_mul(0x85EB_CA77) ^ seed))
}

/// Hashes three integers with a seed.
#[inline(always)]
pub fn hash3(x: i32, y: i32, z: i32, seed: u32) -> u32 {
	hash32((x as u32).wrapping_mul(0x9E37_79B1) ^ hash2(y, z, seed))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn is_deterministic_and_spread() {
		let mut a = Rng::new(42);
		let mut b = Rng::new(42);
		let mut buckets = [0u32; 10];
		for _ in 0..10_000 {
			let x = a.f();
			assert_eq!(x, b.f());
			assert!((0.0..1.0).contains(&x));
			buckets[(x * 10.0) as usize] += 1;
		}
		assert!(buckets.iter().all(|&n| (900..1100).contains(&n)), "{buckets:?}");
		assert!(Rng::new(1).below(7) < 7);
		assert_eq!(Rng::new(1).below(0), 0);
	}
}
