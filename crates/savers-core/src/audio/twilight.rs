// Twilight's score: vast and slow, with swells that follow the shots and a bright one when the sun clears the limb.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! docs/audio.md, "Twilight". A sub-bass and a filtered drone on the palette's tonic; pad chords from its mode that swell over
//! seconds like aurora curtains; sparse FM glass bells, swept across the stereo field in the satellite pass; a bright high swell as
//! the sun slides out from behind the limb; all through a long hall. Chords and bells are chosen by a generator seeded from the
//! seed and the loop's pass, at times within the 90-second loop, so a render is reproducible.

use alloc::vec::Vec;

use super::{Cues, Noise, Reverb, Score, Smooth, Svf, Tables, key_for, lookup, note, step};
use crate::math::{TAU, exp, floor, saturate, smoothstep};
use crate::palette::PaletteId;
use crate::rng::Rng;
use crate::settings::{Key, Settings};
use crate::twilight_shared::{SHOTS, total_length};

/// One pad note: two detuned oscillators, one each side, through low-pass filters, with a slow swell and fade.
struct Pad {
	freq: f32,
	phase: [f32; 2],
	filter: [Svf; 2],
	level: Smooth,
	target: f32,
	/// Seconds until it starts to fade.
	life: f32,
	gain: f32,
	attack: f32,
	release: f32,
	bright: f32,
}

/// One glass bell: frequency modulation with a falling index, a slow decay, a place in the stereo field.
struct Bell {
	freq: f32,
	carrier: f32,
	modulator: f32,
	age: f32,
	gain: f32,
	pan: f32,
	decay: f32,
}

/// Something the score does at a time within the loop.
#[derive(Clone, Copy)]
enum Event {
	Chord { degree: usize, voices: usize },
	Bell { step: usize, octave: f32 },
}

pub struct TwilightScore {
	rate: f32,
	tables: Tables,
	seed: u32,
	root: f32,
	scale: [f32; 7],
	loop_len: f32,
	pass: i64,
	events: Vec<(f32, Event)>,
	last_loop_t: f32,
	sub_phase: f32,
	drone_phase: [f32; 3],
	drone_filter: [Svf; 2],
	lfo: f32,
	pads: Vec<Pad>,
	bells: Vec<Bell>,
	sun_pad: Option<Pad>,
	reverb: Reverb,
	master: Smooth,
	low: Smooth,
	noise: Noise,
}

impl TwilightScore {
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> TwilightScore {
		let (root, scale) = key_for(PaletteId::from_u32(settings.get(Key::Palette)));
		let rate = rate as f32;
		TwilightScore {
			rate,
			tables: Tables::new(),
			seed,
			root,
			scale,
			loop_len: total_length(),
			pass: -1,
			events: Vec::new(),
			last_loop_t: 0.0,
			sub_phase: 0.0,
			drone_phase: [0.0; 3],
			drone_filter: [Svf::default(); 2],
			lfo: 0.0,
			pads: Vec::new(),
			bells: Vec::new(),
			sun_pad: None,
			reverb: Reverb::new(rate, 9.0, 1.4, 0.45),
			master: Smooth { value: 0.0 },
			low: Smooth { value: 0.0 },
			noise: Noise::new(seed ^ 0x7A11),
		}
	}

	/// The events of one pass through the loop: a chord every 9 to 14 seconds, bells between.
	fn plan(&mut self, pass: i64) {
		let mut rng = Rng::new(((self.seed as u64) << 20) ^ pass as u64 ^ 0x5C0E);
		self.events.clear();
		let mut t = 0.5;
		while t < self.loop_len {
			let degree = [0usize, 0, 4, 3, 5, 1][rng.below(6) as usize];
			self.events.push((
				t,
				Event::Chord {
					degree,
					voices: 3 + rng.below(2) as usize,
				},
			));
			t += rng.range(9.0, 14.0);
		}
		let mut t = rng.range(2.0, 5.0);
		while t < self.loop_len {
			self.events.push((
				t,
				Event::Bell {
					step: [0usize, 1, 2, 4, 5][rng.below(5) as usize],
					octave: if rng.f() < 0.6 { 16.0 } else { 32.0 },
				},
			));
			t += rng.range(3.5, 8.0);
		}
	}

	fn degree(&self, d: usize) -> f32 {
		self.scale[d % 7] + 12.0 * (d / 7) as f32
	}

	fn trigger(&mut self, event: Event, cues: &Cues, rng: &mut Rng) {
		match event {
			Event::Chord { degree, voices } => {
				// The previous chord fades as the new one swells, overlapping by seconds.
				for p in self.pads.iter_mut() {
					p.life = p.life.min(2.0);
				}
				// Thirds stacked from the degree, the lowest an octave down when the push-in gathers the low pad.
				let steps = [degree, degree + 2, degree + 4, degree + 8, degree + 6];
				for (i, &d) in steps.iter().take(voices).enumerate() {
					let base = if i == 0 { 2.0 } else { 4.0 };
					let freq = note(self.root * base, self.degree(d));
					self.pads.push(Pad {
						freq,
						phase: [rng.f(), rng.f()],
						filter: [Svf::default(); 2],
						level: Smooth { value: 0.0 },
						target: 1.0,
						life: 13.0,
						gain: if i == 0 { 0.04 } else { 0.05 },
						attack: rng.range(4.0, 7.0),
						release: rng.range(6.0, 9.0),
						bright: rng.range(0.6, 1.0),
					});
				}
			}
			Event::Bell { step: s, octave } => {
				// In the satellite pass the bells sweep across the stereo field with it; elsewhere they fall where they may.
				let pan = if cues.shot == 2 {
					(cues.shot_t / SHOTS[2].len) * 2.0 - 1.0
				} else {
					rng.signed() * 0.8
				};
				let density = if cues.shot == 0 || cues.shot == 2 { 1.0 } else { 0.55 };
				if rng.f() < density {
					self.bells.push(Bell {
						freq: note(self.root * octave, self.scale[s]),
						carrier: 0.0,
						modulator: 0.0,
						age: 0.0,
						gain: rng.range(0.022, 0.04),
						pan,
						decay: rng.range(3.0, 5.5),
					});
				}
			}
		}
	}
}

impl Score for TwilightScore {
	fn render(&mut self, cues: &Cues, out: &mut [f32]) {
		let t = cues.t as f32;
		let pass = (t / self.loop_len) as i64;
		let loop_t = t - pass as f32 * self.loop_len;
		if pass != self.pass {
			self.pass = pass;
			self.plan(pass);
			self.last_loop_t = -0.001;
		}
		let mut rng = Rng::new(((self.seed as u64) << 24) ^ (cues.t * 100.0) as u64);
		let due: Vec<Event> = self
			.events
			.iter()
			.filter(|(at, _)| *at > self.last_loop_t && *at <= loop_t)
			.map(|(_, e)| *e)
			.collect();
		for e in due {
			self.trigger(e, cues, &mut rng);
		}
		self.last_loop_t = loop_t;
		// The sun's swell: a bright chord high above that blooms as the sun clears the limb.
		if cues.sun > 0.02 && self.sun_pad.is_none() {
			self.sun_pad = Some(Pad {
				freq: note(self.root * 8.0, self.degree(4)),
				phase: [0.0, 0.37],
				filter: [Svf::default(); 2],
				level: Smooth { value: 0.0 },
				target: 1.0,
				life: 1e9,
				gain: 0.045,
				attack: 3.0,
				release: 6.0,
				bright: 1.0,
			});
		}
		let frames = out.len() / 2;
		let rate = self.rate;
		let dt = frames as f32 / rate;
		// Shot shaping: the push-in gathers the low pad and opens the filters; the night-side drift is darker.
		let shot_k = match cues.shot {
			1 => 0.55 + 0.45 * smoothstep(0.0, SHOTS[1].len, cues.shot_t),
			3 => 0.6,
			_ => 0.75,
		};
		let low_target = if cues.shot == 1 || cues.shot == 3 { 1.0 } else { 0.55 };
		for p in self.pads.iter_mut() {
			p.life -= dt;
			if p.life <= 0.0 {
				p.target = 0.0;
			}
		}
		self.pads.retain(|p| p.target > 0.0 || p.level.value > 1e-4);
		self.bells.retain(|b| b.age < b.decay * 3.0);
		let sun_level = saturate(cues.sun);
		let (tables, root) = (&self.tables, self.root);
		for i in 0..frames {
			let master = self.master.toward(0.55 + 0.45 * cues.fade, 0.8, rate);
			let low = self.low.toward(low_target, 4.0, rate);
			// Slow breathing for the drone and the sub.
			step(&mut self.lfo, 1.0 / 17.0, rate);
			let breath = 0.5 + 0.5 * lookup(&tables.sine, self.lfo);
			step(&mut self.sub_phase, root * 0.5, rate);
			let sub = lookup(&tables.sine, self.sub_phase) * (0.022 + 0.02 * low) * (0.8 + 0.2 * breath);
			step(&mut self.drone_phase[0], root, rate);
			step(&mut self.drone_phase[1], root * 1.5 * 1.0008, rate);
			step(&mut self.drone_phase[2], root * 2.0 * 0.9993, rate);
			let cutoff = 260.0 + 420.0 * breath * shot_k;
			let d0 = lookup(&tables.soft, self.drone_phase[0]) * 0.022 + lookup(&tables.soft, self.drone_phase[2]) * 0.022;
			let d1 = lookup(&tables.soft, self.drone_phase[1]) * 0.024;
			let (dl, _) = self.drone_filter[0].run(d0, cutoff, 1.6, rate);
			let (dr, _) = self.drone_filter[1].run(d1, cutoff * 1.1, 1.6, rate);
			let (mut left, mut right) = (sub + dl * 0.8 + dr * 0.4, sub + dr * 0.8 + dl * 0.4);
			let mut send = (dl + dr) * 0.3;
			for p in self.pads.iter_mut().chain(self.sun_pad.iter_mut()) {
				let target = if p.life > 1e8 { sun_level } else { p.target };
				let seconds = if target > p.level.value { p.attack } else { p.release };
				let lv = p.level.toward(target, seconds, rate);
				if lv < 1e-5 {
					continue;
				}
				step(&mut p.phase[0], p.freq * 1.0023, rate);
				step(&mut p.phase[1], p.freq * 0.9977, rate);
				// The filter opens as the note swells, like a curtain catching light.
				let cut = (320.0 + 2800.0 * lv * lv * p.bright) * shot_k;
				let (a, _) = p.filter[0].run(lookup(&tables.soft, p.phase[0]), cut, 1.3, rate);
				let (b, _) = p.filter[1].run(lookup(&tables.soft, p.phase[1]), cut * 1.07, 1.3, rate);
				let g = p.gain * lv * if p.freq < root * 3.0 { low } else { 1.0 };
				left += a * g;
				right += b * g;
				send += (a + b) * g * 0.6;
			}
			for b in self.bells.iter_mut() {
				b.age += 1.0 / rate;
				let env = exp(-b.age / b.decay * 2.3);
				let index = 2.2 * env * env;
				step(&mut b.modulator, b.freq * 2.0, rate);
				let m = lookup(&tables.sine, b.modulator) * index;
				let mut ph = b.carrier + m / TAU;
				ph -= floor(ph);
				step(&mut b.carrier, b.freq, rate);
				let s = lookup(&tables.sine, ph) * env * b.gain * saturate(b.age * 200.0);
				let (gl, gr) = ((1.0 - b.pan) * 0.5, (1.0 + b.pan) * 0.5);
				left += s * gl * 0.5;
				right += s * gr * 0.5;
				send += s * 0.9;
			}
			// A breath of air, barely there, so the hall always has something in it.
			send += self.noise.sample() * 0.0015;
			let (wl, wr) = self.reverb.run(send);
			out[i * 2] = (left * 0.75 + wl * 0.85) * master;
			out[i * 2 + 1] = (right * 0.75 + wr * 0.85) * master;
		}
	}
}
