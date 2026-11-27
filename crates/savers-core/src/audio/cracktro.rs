// The Cracktro's chiptune: pulse-wave arpeggios and lead, a triangle-ish bass and soft noise drums, generated from the seed.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! docs/audio.md, "Cracktro". 125 beats a minute in sixteenth-note steps. A four-chord progression in the palette's key; a bass on a
//! syncopated pattern; a 25% pulse arpeggio through the chord on every step; a 50% pulse lead with vibrato, its two-bar phrases drawn
//! from the pentatonic by the seeded generator; a kick, a snare and a hat from noise. Each part leans on different voices. Every voice
//! goes through a gentle low-pass, so the square waves are warm rather than harsh, and the mix sits well below full scale.

use super::{Cues, Noise, Reverb, Score, Svf, Tables, key_for, lookup, note, step};
use crate::math::{exp, floor_i, sin};
use crate::palette::PaletteId;
use crate::rng::hash32;
use crate::settings::{Key, Settings};

const STEP: f32 = 60.0 / 125.0 / 4.0;

pub struct CracktroScore {
	rate: f32,
	tables: Tables,
	seed: u32,
	root: f32,
	scale: [f32; 7],
	bass: f32,
	arp: f32,
	lead: f32,
	vibrato: f32,
	kick: f32,
	noise: Noise,
	tone: [Svf; 3],
	hat: Svf,
	snare: Svf,
	reverb: Reverb,
}

impl CracktroScore {
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> CracktroScore {
		let (root, scale) = key_for(PaletteId::from_u32(settings.get(Key::Palette)));
		let rate = rate as f32;
		CracktroScore {
			rate,
			tables: Tables::new(),
			seed,
			root,
			scale,
			bass: 0.0,
			arp: 0.0,
			lead: 0.0,
			vibrato: 0.0,
			kick: 0.0,
			noise: Noise::new(seed ^ 0xC0FE),
			tone: [Svf::default(); 3],
			hat: Svf::default(),
			snare: Svf::default(),
			reverb: Reverb::new(rate, 1.4, 0.4, 0.4),
		}
	}

	/// The frequency of scale degree `d` (any integer) above `base`.
	fn degree(&self, base: f32, d: i32) -> f32 {
		let octave = d.div_euclid(7);
		note(base, self.scale[d.rem_euclid(7) as usize] + 12.0 * octave as f32)
	}
}

#[inline(always)]
fn pulse(phase: f32, duty: f32) -> f32 {
	if phase < duty { 0.7 } else { -0.7 }
}

impl Score for CracktroScore {
	fn render(&mut self, cues: &Cues, out: &mut [f32]) {
		let rate = self.rate;
		// What each part leans on: (bass, arpeggio, lead, drums).
		let mix = match cues.shot {
			0 => [1.0, 0.8, 0.9, 1.0],
			1 => [0.5, 1.0, 0.8, 0.4],
			2 => [1.0, 0.7, 0.5, 1.0],
			3 => [0.8, 0.5, 1.0, 0.9],
			_ => [1.0, 0.9, 1.0, 1.0],
		};
		let frames = out.len() / 2;
		for i in 0..frames {
			let t = cues.t as f32 + i as f32 / rate;
			let s = floor_i(t / STEP);
			let in_step = t - s as f32 * STEP;
			// Every note starts with a 2 ms ramp, so a retrigger never clicks.
			let attack = (in_step * 500.0).min(1.0);
			let bar = s.div_euclid(16);
			let pos = s.rem_euclid(16);
			// i, vi, iv, v: degrees of the mode, a bar each.
			let chord = [0, 5, 3, 4][bar.rem_euclid(4) as usize];
			// Bass: the chord's root on a syncopated pattern, an octave jump on the off-beats.
			let bass_on = matches!(pos, 0 | 3 | 6 | 8 | 11 | 14);
			let bass_f = self.degree(self.root, chord) * if pos == 6 || pos == 14 { 2.0 } else { 1.0 };
			step(&mut self.bass, bass_f, rate);
			let bass_env = if bass_on {
				exp(-in_step * 9.0) * attack
			} else {
				exp(-(in_step + STEP) * 9.0) * 0.3
			};
			let tri = lookup(&self.tables.sine, self.bass) * 0.8 + lookup(&self.tables.soft, self.bass) * 0.2;
			let bass = tri * bass_env * 0.14 * mix[0];
			// Arpeggio: through the chord, one note a step.
			let arp_d = chord + [0, 2, 4, 7][pos.rem_euclid(4) as usize];
			let arp_f = self.degree(self.root * 4.0, arp_d);
			step(&mut self.arp, arp_f, rate);
			let arp = pulse(self.arp, 0.25) * exp(-in_step * 22.0) * attack * 0.045 * mix[1];
			// Lead: a two-bar phrase from the pentatonic, chosen by the seed for this phrase.
			let phrase = bar.div_euclid(2);
			let slot = s.rem_euclid(32);
			let h = hash32(self.seed ^ (phrase as u32).wrapping_mul(0x9E37_79B9) ^ (slot as u32 / 2));
			let plays = (h & 3) != 0;
			let penta = [0, 1, 2, 4, 5];
			let lead_d = chord.rem_euclid(7) + penta[(h >> 4) as usize % 5] + 7;
			step(&mut self.vibrato, 5.5, rate);
			let vib = 1.0 + 0.006 * lookup(&self.tables.sine, self.vibrato) * (in_step / STEP).min(1.0);
			let lead_f = self.degree(self.root * 2.0, lead_d) * vib;
			step(&mut self.lead, lead_f, rate);
			let lead_t = in_step + if slot % 2 == 1 { STEP } else { 0.0 };
			let lead_attack = if slot % 2 == 1 { 1.0 } else { attack };
			let lead = if plays {
				pulse(self.lead, 0.5) * exp(-lead_t * 3.0) * lead_attack * 0.05 * mix[2]
			} else {
				0.0
			};
			// Drums: a kick that falls in pitch, a snare of noise and tone, a short hat.
			let drums = mix[3];
			let mut drum = 0.0;
			if pos == 0 || pos == 8 || (pos == 10 && bar % 2 == 1) {
				if in_step * rate < 1.0 {
					self.kick = 0.0;
				}
				let f = 45.0 + 90.0 * exp(-in_step * 30.0);
				step(&mut self.kick, f, rate);
				drum += lookup(&self.tables.sine, self.kick) * exp(-in_step * 14.0) * attack * 0.16;
			}
			let n = self.noise.sample();
			if pos == 4 || pos == 12 {
				let (_, band) = self.snare.run(n, 1800.0, 0.8, rate);
				drum +=
					(band * 0.35 + sin(in_step * 180.0 * core::f32::consts::TAU) * 0.15) * exp(-in_step * 18.0) * attack * 0.4;
			}
			if pos % 2 == 0 {
				let (low, _) = self.hat.run(n, 7000.0, 0.7, rate);
				drum += (n - low) * exp(-in_step * 60.0) * attack * 0.03;
			}
			drum *= drums;
			// Warm the square waves with a gentle low-pass each.
			let (b, _) = self.tone[0].run(bass, 900.0, 1.2, rate);
			let (a, _) = self.tone[1].run(arp, 3200.0, 1.0, rate);
			let (l, _) = self.tone[2].run(lead, 2600.0, 1.0, rate);
			let (wl, wr) = self.reverb.run((a + l) * 0.5 + drum * 0.1);
			out[i * 2] = b + a * 0.8 + l * 0.6 + drum + wl * 0.5;
			out[i * 2 + 1] = b + a * 0.6 + l * 0.8 + drum + wr * 0.5;
		}
	}
}
