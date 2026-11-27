// Quill Ghostwriter's sound: a hushed room, soft key ticks as the document types, a small bell when a suggestion is chosen, and a
// quiet chord when the "Save the changes?" dialog visits.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! docs/audio.md, "Quill Ghostwriter". Ticks are short band-passed noise bursts, varied per key; each new character in the cues makes
//! one. They are quiet by design, as the owner asked.

use alloc::vec::Vec;

use super::{Cues, Noise, Reverb, Score, Smooth, Svf, Tables, key_for, lookup, note, step};
use crate::math::exp;
use crate::palette::PaletteId;
use crate::rng::Rng;
use crate::settings::{Key, Settings};

struct Tick {
	age: f32,
	centre: f32,
	gain: f32,
	filter: Svf,
}

pub struct GhostwriterScore {
	rate: f32,
	tables: Tables,
	root: f32,
	scale: [f32; 7],
	rng: Rng,
	noise: Noise,
	room: Svf,
	ticks: Vec<Tick>,
	last_keys: u32,
	bell: Option<(f32, f32)>,
	was_holding: bool,
	chord: [f32; 3],
	dialog: Smooth,
	reverb: Reverb,
}

impl GhostwriterScore {
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> GhostwriterScore {
		let (root, scale) = key_for(PaletteId::from_u32(settings.get(Key::Palette)));
		let rate = rate as f32;
		GhostwriterScore {
			rate,
			tables: Tables::new(),
			root,
			scale,
			rng: Rng::new(seed as u64 ^ 0x9A11),
			noise: Noise::new(seed ^ 0x51),
			room: Svf::default(),
			ticks: Vec::new(),
			last_keys: 0,
			bell: None,
			was_holding: false,
			chord: [0.0; 3],
			dialog: Smooth { value: 0.0 },
			reverb: Reverb::new(rate, 0.9, 0.3, 0.5),
		}
	}
}

impl Score for GhostwriterScore {
	fn render(&mut self, cues: &Cues, out: &mut [f32]) {
		let rate = self.rate;
		// A tick for each new character (a block holds at most a few); a fall in the count is a new act or the rewind, which is silent.
		if cues.keys > self.last_keys {
			for _ in 0..(cues.keys - self.last_keys).min(4) {
				self.ticks.push(Tick {
					age: -self.rng.range(0.0, 0.004),
					centre: self.rng.range(1800.0, 3400.0),
					gain: self.rng.range(0.05, 0.09),
					filter: Svf::default(),
				});
			}
		}
		self.last_keys = cues.keys;
		if cues.hold && !self.was_holding {
			self.bell = Some((note(self.root * 16.0, self.scale[4]), 0.0));
		}
		self.was_holding = cues.hold;
		self.ticks.retain(|k| k.age < 0.08);
		let tables = &self.tables;
		let chord = [
			note(self.root * 4.0, self.scale[0]),
			note(self.root * 4.0, self.scale[2]),
			note(self.root * 4.0, self.scale[4]),
		];
		let frames = out.len() / 2;
		for i in 0..frames {
			let n = self.noise.sample();
			// The room: very quiet low noise, a hush rather than a hiss.
			let (room, _) = self.room.run(n, 220.0, 1.6, rate);
			let (mut left, mut right) = (room * 0.004, room * 0.004);
			let mut send = 0.0;
			for k in self.ticks.iter_mut() {
				k.age += 1.0 / rate;
				if k.age < 0.0 {
					continue;
				}
				let (_, band) = k.filter.run(n, k.centre, 0.6, rate);
				let s = band * k.gain * exp(-k.age * 140.0);
				left += s;
				right += s * 0.85;
				send += s * 0.3;
			}
			if let Some((f, age)) = self.bell.as_mut() {
				*age += 1.0 / rate;
				let s = lookup(&tables.sine, (*f * *age) % 1.0) * 0.02 * exp(-*age * 2.5);
				left += s;
				right += s;
				send += s;
			}
			let d = self.dialog.toward(cues.dialog, 0.6, rate);
			if d > 1e-3 {
				for (k, f) in chord.iter().enumerate() {
					step(&mut self.chord[k], *f, rate);
					let s = lookup(&tables.sine, self.chord[k]) * 0.012 * d;
					left += s;
					right += s;
					send += s;
				}
			}
			let (wl, wr) = self.reverb.run(send);
			out[i * 2] = left + wl * 0.5;
			out[i * 2 + 1] = right + wr * 0.5;
		}
		if let Some((_, age)) = self.bell
			&& age > 3.0
		{
			self.bell = None;
		}
	}
}
