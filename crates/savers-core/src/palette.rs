// The Afterglow palettes, from afterglow-themes' palette and wallpaper roles, and Quill's navy and gold.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Copied from `afterglow-themes` `scripts/palette.mjs` (brand, deep and neutral ramps) and `scripts/build-wallpapers.mjs`
//! (`WALLPAPER_PALETTES`: which neutral, sun and glow hues each palette uses at dusk and by day). See `docs/art.md`.

use crate::colour::{Ramp, mixf};

pub const ORANGE: u32 = 0xFFAA40;
pub const RED: u32 = 0xF43F5E;
pub const PURPLE: u32 = 0xA78BFA;
pub const CYAN: u32 = 0x22D3EE;
pub const BLUE: u32 = 0x60A5FA;
pub const GREEN: u32 = 0x2EC66A;
pub const YELLOW: u32 = 0xFBBF24;
pub const GRAY: u32 = 0x94A3B8;

pub const DEEP_ORANGE: u32 = 0xB45309;
pub const DEEP_RED: u32 = 0xBE123C;
pub const DEEP_PURPLE: u32 = 0x5E44CC;
pub const DEEP_CYAN: u32 = 0x0E7490;
pub const DEEP_BLUE: u32 = 0x2563EB;
pub const DEEP_GREEN: u32 = 0x15803D;
pub const DEEP_YELLOW: u32 = 0xA16207;
pub const DEEP_GRAY: u32 = 0x5A6A80;

/// Neutral ramps, index 0 = step 100 (strongest contrast) to index 8 = step 900 (the background).
pub const VOID: [u32; 9] = [
	0xF4F4F8, 0xE0E0E0, 0xB4BFCE, 0x8D97A8, 0x5A6273, 0x333847, 0x1F2230, 0x12121A, 0x050507,
];
pub const INDIGO: [u32; 9] = [
	0xF1EEE6, 0xE4E2EC, 0xB8B6C8, 0x8F8DA6, 0x5F5D78, 0x3A384F, 0x2A2840, 0x1A1828, 0x0F0E1A,
];
pub const PAPER: [u32; 9] = [
	0x1C1B20, 0x2B2A33, 0x484554, 0x6B6877, 0x8F8C98, 0xC9C5D0, 0xE6E0EB, 0xF2EFF4, 0xFBFAF6,
];

/// Quill's navy and gold, from Quill's hero and icon (`quill16` `assets/hero.svg`): navy `#0a1230` and `#14142a`, gold `#f0a830`,
/// cream `#fbf7ea`.
pub const QUILL_NAVY: [u32; 9] = [
	0xFBF7EA, 0xFFF4D6, 0xC4C6D0, 0x8A90A8, 0x5B6684, 0x2B2F4A, 0x202030, 0x14142A, 0x0A1230,
];
pub const QUILL_GOLD: u32 = 0xF0A830;

/// The palette ids stored in the `Palette` setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteId {
	Dusk = 0,
	Indigo = 1,
	Ember = 2,
	Aurora = 3,
	Graphite = 4,
	Quill = 5,
}

impl PaletteId {
	pub const ALL: [PaletteId; 6] = [
		PaletteId::Dusk,
		PaletteId::Indigo,
		PaletteId::Ember,
		PaletteId::Aurora,
		PaletteId::Graphite,
		PaletteId::Quill,
	];

	pub fn from_u32(v: u32) -> PaletteId {
		*PaletteId::ALL.get(v as usize).unwrap_or(&PaletteId::Dusk)
	}

	pub fn name(self) -> &'static str {
		match self {
			PaletteId::Dusk => "Dusk",
			PaletteId::Indigo => "Indigo",
			PaletteId::Ember => "Ember",
			PaletteId::Aurora => "Aurora",
			PaletteId::Graphite => "Graphite",
			PaletteId::Quill => "Quill navy and gold",
		}
	}
}

/// A resolved palette: what a scene paints with.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
	pub id: PaletteId,
	pub light: bool,
	/// The neutral ramp, 0 = strongest contrast, 8 = the ground.
	pub neutral: [u32; 9],
	/// The sun hue: the hottest light in the scene.
	pub sun: u32,
	/// Glow hues outwards from the sun; `glow_n` of them are real, the rest repeat the last.
	pub glow: [u32; 4],
	pub glow_n: usize,
	/// Deeper hues that keep their colour on a light ground (ink on paper, hills by day).
	pub deep: [u32; 3],
}

impl Palette {
	pub fn new(id: PaletteId, light: bool) -> Palette {
		let (neutral, sun, glow, deep): ([u32; 9], u32, &[u32], [u32; 3]) = match (id, light) {
			(PaletteId::Dusk, false) => (VOID, YELLOW, &[ORANGE, RED, PURPLE], [DEEP_ORANGE, DEEP_RED, DEEP_PURPLE]),
			(PaletteId::Dusk, true) => (PAPER, YELLOW, &[YELLOW, ORANGE], [DEEP_PURPLE, DEEP_ORANGE, DEEP_RED]),
			(PaletteId::Indigo, false) => (INDIGO, ORANGE, &[PURPLE, BLUE], [DEEP_PURPLE, DEEP_BLUE, DEEP_ORANGE]),
			(PaletteId::Indigo, true) => (PAPER, ORANGE, &[ORANGE, PURPLE], [DEEP_BLUE, DEEP_PURPLE, DEEP_ORANGE]),
			(PaletteId::Ember, false) => (VOID, YELLOW, &[YELLOW, ORANGE, RED], [DEEP_YELLOW, DEEP_ORANGE, DEEP_RED]),
			(PaletteId::Ember, true) => (PAPER, YELLOW, &[YELLOW, ORANGE], [DEEP_RED, DEEP_ORANGE, DEEP_YELLOW]),
			(PaletteId::Aurora, false) => (INDIGO, CYAN, &[GREEN, CYAN, BLUE, PURPLE], [DEEP_GREEN, DEEP_CYAN, DEEP_BLUE]),
			(PaletteId::Aurora, true) => (PAPER, YELLOW, &[CYAN, BLUE], [DEEP_GREEN, DEEP_CYAN, DEEP_BLUE]),
			(PaletteId::Graphite, false) => (VOID, ORANGE, &[ORANGE, GRAY, GRAY], [DEEP_GRAY, DEEP_ORANGE, DEEP_GRAY]),
			(PaletteId::Graphite, true) => (PAPER, ORANGE, &[ORANGE, GRAY], [DEEP_GRAY, DEEP_ORANGE, DEEP_GRAY]),
			(PaletteId::Quill, false) => (
				QUILL_NAVY,
				QUILL_GOLD,
				&[QUILL_GOLD, ORANGE, BLUE],
				[DEEP_YELLOW, DEEP_BLUE, DEEP_ORANGE],
			),
			(PaletteId::Quill, true) => (PAPER, QUILL_GOLD, &[QUILL_GOLD, BLUE], [0x22304F, DEEP_YELLOW, DEEP_BLUE]),
		};
		let mut g = [glow[glow.len() - 1]; 4];
		g[..glow.len()].copy_from_slice(glow);
		Palette {
			id,
			light,
			neutral,
			sun,
			glow: g,
			glow_n: glow.len(),
			deep,
		}
	}

	/// The ground colour (neutral 900).
	#[inline]
	pub fn ground(&self) -> u32 {
		self.neutral[8]
	}

	/// A neutral by its step (100 to 900).
	#[inline]
	pub fn n(&self, step: u32) -> u32 {
		self.neutral[((step / 100).clamp(1, 9) - 1) as usize]
	}

	/// Glow hue `i`, wrapping over the real ones.
	#[inline]
	pub fn hue(&self, i: usize) -> u32 {
		self.glow[i % self.glow_n]
	}

	/// The palette's light ramp: ground, then the glow hues from the outermost inwards, then the sun, then near white.
	pub fn light_ramp(&self) -> Ramp {
		let mut stops = [(0.0f32, 0u32); 8];
		let mut n = 0;
		stops[n] = (0.0, self.ground());
		n += 1;
		let count = self.glow_n;
		for i in 0..count {
			let hue = self.glow[count - 1 - i];
			let pos = 0.18 + 0.6 * (i as f32 + 1.0) / (count as f32 + 1.0);
			let start = if i == 0 { mixf(self.ground(), hue, 0.55) } else { hue };
			stops[n] = (pos, start);
			n += 1;
		}
		stops[n] = (0.86, self.sun);
		n += 1;
		stops[n] = (1.0, mixf(self.sun, 0xFFFFFF, 0.75));
		n += 1;
		Ramp::new(&stops[..n])
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_palette_resolves() {
		for id in PaletteId::ALL {
			for light in [false, true] {
				let p = Palette::new(id, light);
				assert!(p.glow_n >= 2);
				let ramp = p.light_ramp();
				assert_eq!(ramp.c[0], p.ground());
			}
		}
		assert_eq!(PaletteId::from_u32(99), PaletteId::Dusk);
	}
}
