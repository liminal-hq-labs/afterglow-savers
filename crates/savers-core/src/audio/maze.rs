// The Maze's ambience: a distant hum, wind that opens with the corridors, far chimes, and a rising tone and a whoosh at the portal.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! docs/audio.md, "Maze". No footsteps. Two low sines beating slowly; band-passed wind in each ear, brighter and louder where the view
//! runs long; chimes from the sun windows, softer the further away; near the portal a chord that climbs a fourth over the approach,
//! holds while the walk hesitates, and resolves after a whoosh through the plane; a stone room's reverb.

use alloc::vec::Vec;

use super::{Cues, Noise, Reverb, Score, Smooth, Svf, Tables, key_for, lookup, note, step};
use crate::math::{PI, TAU, exp, floor, saturate, sin, smoothstep};
use crate::palette::PaletteId;
use crate::rng::Rng;
use crate::settings::{Key, Settings};

struct Chime {
	freq: f32,
	carrier: f32,
	modulator: f32,
	age: f32,
	gain: f32,
	pan: f32,
}

pub struct MazeScore {
	rate: f32,
	tables: Tables,
	root: f32,
	scale: [f32; 7],
	rng: Rng,
	hum: [f32; 3],
	hum_filter: Svf,
	wind: [Svf; 2],
	wind_noise: [Noise; 2],
	whoosh: Svf,
	whoosh_noise: Noise,
	lfo: [f32; 2],
	chord_phase: [f32; 3],
	chord_filter: [Svf; 3],
	chord_level: Smooth,
	rise: Smooth,
	open: Smooth,
	chimes: Vec<Chime>,
	next_chime: f32,
	reverb: Reverb,
	master: Smooth,
}

impl MazeScore {
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> MazeScore {
		let (root, _) = key_for(PaletteId::from_u32(settings.get(Key::Palette)));
		// The Maze is in Dorian whatever the palette: stone and evening, a little grave.
		let scale = [0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 10.0];
		let rate = rate as f32;
		MazeScore {
			rate,
			tables: Tables::new(),
			root,
			scale,
			rng: Rng::new(seed as u64 ^ 0x3A2E_A0D1),
			hum: [0.0; 3],
			hum_filter: Svf::default(),
			wind: [Svf::default(); 2],
			wind_noise: [Noise::new(seed ^ 0x11), Noise::new(seed ^ 0x2222)],
			whoosh: Svf::default(),
			whoosh_noise: Noise::new(seed ^ 0x333),
			lfo: [0.0, 0.31],
			chord_phase: [0.0, 0.21, 0.47],
			chord_filter: [Svf::default(); 3],
			chord_level: Smooth { value: 0.0 },
			rise: Smooth { value: 0.0 },
			open: Smooth { value: 0.5 },
			chimes: Vec::new(),
			next_chime: 3.0,
			reverb: Reverb::new(rate, 3.2, 0.6, 0.5),
			master: Smooth { value: 0.0 },
		}
	}
}

impl Score for MazeScore {
	fn render(&mut self, cues: &Cues, out: &mut [f32]) {
		let frames = out.len() / 2;
		let rate = self.rate;
		let t = cues.t as f32;
		// Chimes from the sun windows: more often and louder the nearer one is.
		if t >= self.next_chime {
			if cues.window > 0.05 {
				let s = [0usize, 2, 3, 4, 6][self.rng.below(5) as usize];
				self.chimes.push(Chime {
					freq: note(self.root * 16.0, self.scale[s]),
					carrier: 0.0,
					modulator: 0.0,
					age: 0.0,
					gain: 0.04 * cues.window,
					pan: self.rng.signed() * 0.7,
				});
			}
			self.next_chime = t + self.rng.range(2.5, 6.0) * (1.4 - cues.window);
		}
		self.chimes.retain(|c| c.age < 12.0);
		// The portal chord climbs a fourth over the approach, stays put while the walk holds.
		let approach = smoothstep(0.0, 1.0, cues.portal);
		let tables = &self.tables;
		let base = self.root * 4.0;
		let chord = [1.0f32, 1.4983, 2.2449];
		for i in 0..frames {
			let master = self.master.toward(cues.fade, 1.0, rate);
			let open = self.open.toward(cues.open, 1.5, rate);
			let rise = if cues.hold {
				self.rise.value
			} else {
				self.rise.toward(approach, 1.2, rate)
			};
			let level = self.chord_level.toward(
				approach * approach * (1.0 - cues.through * 0.6),
				if approach > 0.0 { 2.0 } else { 3.5 },
				rate,
			);
			step(&mut self.lfo[0], 1.0 / 11.0, rate);
			step(&mut self.lfo[1], 1.0 / 7.3, rate);
			let l0 = lookup(&tables.sine, self.lfo[0]);
			let l1 = lookup(&tables.sine, self.lfo[1]);
			// The hum: two low sines a hair apart, beating slowly, and a quiet octave.
			step(&mut self.hum[0], self.root, rate);
			step(&mut self.hum[1], self.root * 1.004, rate);
			step(&mut self.hum[2], self.root * 2.0, rate);
			let hum = (lookup(&tables.sine, self.hum[0]) + lookup(&tables.sine, self.hum[1])) * 0.016
				+ lookup(&tables.soft, self.hum[2]) * 0.012;
			let (hum, _) = self.hum_filter.run(hum, 300.0, 1.4, rate);
			// The wind: band-passed noise in each ear, opening with the view.
			let centre = 260.0 + 700.0 * open + 120.0 * l0;
			let (_, wl) = self.wind[0].run(self.wind_noise[0].sample(), centre, 0.9, rate);
			let (_, wr) = self.wind[1].run(self.wind_noise[1].sample(), centre * (1.0 + 0.08 * l1), 0.9, rate);
			let wind_gain = 0.05 + 0.065 * open;
			let (mut left, mut right) = (hum + wl * wind_gain, hum + wr * wind_gain);
			let mut send = (wl + wr) * wind_gain * 0.5 + hum * 0.2;
			if level > 1e-4 {
				let lift = 1.0 + 0.3348 * rise;
				let cut = 300.0 + 1700.0 * level;
				for (k, &interval) in chord.iter().enumerate() {
					step(
						&mut self.chord_phase[k],
						base * interval * lift * (1.0 + 0.0015 * k as f32),
						rate,
					);
					let (v, _) = self.chord_filter[k].run(lookup(&tables.soft, self.chord_phase[k]), cut, 1.2, rate);
					let g = v * 0.06 * level;
					if k == 1 {
						left += g * 0.6;
						right += g * 0.6;
					} else if k == 0 {
						left += g;
					} else {
						right += g;
					}
					send += g * 0.7;
				}
			}
			// The whoosh through the plane: a band of noise sweeping up and away.
			if cues.through > 0.0 {
				let k = cues.through;
				let (_, w) = self.whoosh.run(self.whoosh_noise.sample(), 200.0 + 3200.0 * k * k, 0.7, rate);
				let g = w * 0.12 * sin(PI * k);
				left += g * (1.0 - 0.3 * k);
				right += g * (0.7 + 0.3 * k);
				send += g * 0.5;
			}
			for c in self.chimes.iter_mut() {
				c.age += 1.0 / rate;
				let env = exp(-c.age * 0.7);
				step(&mut c.modulator, c.freq * 3.5, rate);
				let m = lookup(&tables.sine, c.modulator) * 1.6 * env;
				let mut ph = c.carrier + m / TAU;
				ph -= floor(ph);
				step(&mut c.carrier, c.freq, rate);
				let s = lookup(&tables.sine, ph) * env * c.gain * saturate(c.age * 300.0);
				left += s * (1.0 - c.pan) * 0.5;
				right += s * (1.0 + c.pan) * 0.5;
				send += s;
			}
			let (rl, rr) = self.reverb.run(send);
			out[i * 2] = (left * 0.8 + rl * 0.7) * master;
			out[i * 2 + 1] = (right * 0.8 + rr * 0.7) * master;
		}
	}
}
