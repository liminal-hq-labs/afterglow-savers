// Ink's score: glassy and watery, a soft pad that brightens with the aurora, drop-like plucks at each pour.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! docs/audio.md, "Ink". A pad of open fifths on the palette's tonic, slowly beating; a pluck (a decaying sine with a falling pitch,
//! like a drop on water) as each pour begins and a few after it; in the aurora phase a high shimmer of the mode's notes and an opened
//! filter; a five-second room.

use alloc::vec::Vec;

use super::{Cues, Reverb, Score, Smooth, Svf, Tables, key_for, lookup, note, step};
use crate::math::{exp, saturate};
use crate::palette::PaletteId;
use crate::rng::Rng;
use crate::settings::{Key, Settings};

struct Drop {
	freq: f32,
	phase: f32,
	age: f32,
	gain: f32,
	pan: f32,
}

pub struct InkScore {
	rate: f32,
	tables: Tables,
	root: f32,
	scale: [f32; 7],
	rng: Rng,
	pad: [f32; 4],
	pad_filter: [Svf; 2],
	shimmer: [f32; 3],
	aurora: Smooth,
	drops: Vec<Drop>,
	last_cycle_t: f32,
	next_drop: f32,
	reverb: Reverb,
}

impl InkScore {
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> InkScore {
		let (root, scale) = key_for(PaletteId::from_u32(settings.get(Key::Palette)));
		let rate = rate as f32;
		InkScore {
			rate,
			tables: Tables::new(),
			root,
			scale,
			rng: Rng::new(seed as u64 ^ 0x1A4B_A0D1),
			pad: [0.0, 0.25, 0.5, 0.75],
			pad_filter: [Svf::default(); 2],
			shimmer: [0.0; 3],
			aurora: Smooth { value: 0.0 },
			drops: Vec::new(),
			last_cycle_t: 1e9,
			next_drop: 0.5,
			reverb: Reverb::new(rate, 5.0, 0.9, 0.35),
		}
	}
}

impl Score for InkScore {
	fn render(&mut self, cues: &Cues, out: &mut [f32]) {
		let rate = self.rate;
		let cyc = cues.shot_t;
		// A new pour: the cycle begins again; drops fall through its first fifteen seconds.
		if cyc < self.last_cycle_t {
			self.next_drop = 0.3;
		}
		self.last_cycle_t = cyc;
		if cyc >= self.next_drop && cyc < 16.0 {
			let s = self.rng.below(7) as usize;
			self.drops.push(Drop {
				freq: note(self.root * 8.0, self.scale[s]),
				phase: 0.0,
				age: 0.0,
				gain: self.rng.range(0.03, 0.05),
				pan: self.rng.signed() * 0.8,
			});
			self.next_drop = cyc + self.rng.range(0.8, 3.0);
		}
		self.drops.retain(|d| d.age < 6.0);
		let tables = &self.tables;
		let (root, fifth) = (self.root * 2.0, self.root * 3.0);
		let shimmer_notes = [
			note(self.root * 16.0, self.scale[4]),
			note(self.root * 16.0, self.scale[1]),
			note(self.root * 32.0, self.scale[0]),
		];
		let frames = out.len() / 2;
		for i in 0..frames {
			let a = self.aurora.toward(cues.sun, 2.5, rate);
			step(&mut self.pad[0], root, rate);
			step(&mut self.pad[1], root * 1.0021, rate);
			step(&mut self.pad[2], fifth, rate);
			step(&mut self.pad[3], fifth * 0.9984, rate);
			let cut = 700.0 + 1800.0 * a;
			let (l, _) = self.pad_filter[0].run(
				lookup(&tables.soft, self.pad[0]) + lookup(&tables.sine, self.pad[2]) * 0.8,
				cut,
				1.4,
				rate,
			);
			let (r, _) = self.pad_filter[1].run(
				lookup(&tables.soft, self.pad[1]) + lookup(&tables.sine, self.pad[3]) * 0.8,
				cut * 1.05,
				1.4,
				rate,
			);
			let (mut left, mut right) = (l * 0.05, r * 0.05);
			let mut send = (l + r) * 0.025;
			if a > 1e-3 {
				for (k, f) in shimmer_notes.iter().enumerate() {
					step(&mut self.shimmer[k], *f, rate);
					let s = lookup(&tables.sine, self.shimmer[k]) * 0.012 * a;
					if k % 2 == 0 {
						left += s;
					} else {
						right += s;
					}
					send += s * 1.2;
				}
			}
			for d in self.drops.iter_mut() {
				d.age += 1.0 / rate;
				// A drop: a quick fall in pitch and a soft decay.
				let f = d.freq * (1.0 + 0.6 * exp(-d.age * 30.0));
				step(&mut d.phase, f, rate);
				let s = lookup(&tables.sine, d.phase) * d.gain * exp(-d.age * 2.2) * saturate(d.age * 400.0);
				left += s * (1.0 - d.pan) * 0.5;
				right += s * (1.0 + d.pan) * 0.5;
				send += s * 0.8;
			}
			let (wl, wr) = self.reverb.run(send);
			out[i * 2] = left * 0.8 + wl * 0.8;
			out[i * 2 + 1] = right * 0.8 + wr * 0.8;
		}
	}
}
