// Generative audio: a small synthesizer, a reverb, a limiter, and the engine that turns a scene's cues into 16-bit stereo samples.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! SAV-D34, docs/audio.md. No samples: oscillator tables made at start, filters, FM bells, noise and a feedback-delay reverb. A score
//! reads the scene's [`Cues`] once a block and renders that block; the [`Engine`] adds the volume, the opening fade and the limiter.
//! Everything is a function of the seed, the settings and the sample clock, so a render is reproducible.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use crate::math::{TAU, clamp, exp, ln, saturate, sin};

pub mod maze;
pub mod twilight;

/// What a scene tells its score about the moment, sampled once a block.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cues {
	/// Scene time in seconds.
	pub t: f64,
	/// The picture's own fade for cuts, 0 to 1 (1 when the scene has none).
	pub fade: f32,
	/// Twilight: the shot, its time in seconds, and how far the sun is from behind the limb (0 hidden, 1 clear).
	pub shot: usize,
	pub shot_t: f32,
	pub sun: f32,
	/// Maze: how near the portal is (0 far, 1 at its plane), whether the walk is holding before it, the whoosh through it (0 to 1
	/// over the crossing, 0 otherwise), how open the view ahead is (0 a wall close by, 1 a long corridor) and how near a window is.
	pub portal: f32,
	pub hold: bool,
	pub through: f32,
	pub open: f32,
	pub window: f32,
}

/// A score: renders `out.len() / 2` stereo frames (left, right interleaved, in -1 to 1) for the cues of the block's start.
pub trait Score {
	fn render(&mut self, cues: &Cues, out: &mut [f32]);
}

/// Wave tables: sine and a soft, band-limited saw.
pub struct Tables {
	pub sine: Vec<f32>,
	pub soft: Vec<f32>,
}

pub const TABLE: usize = 4096;

impl Tables {
	pub fn new() -> Tables {
		let mut sine = vec![0.0f32; TABLE + 1];
		let mut soft = vec![0.0f32; TABLE + 1];
		for i in 0..=TABLE {
			let x = i as f32 / TABLE as f32 * TAU;
			sine[i] = sin(x);
			// Ten harmonics, each a little softer than a saw's, normalized below.
			let mut s = 0.0;
			let mut amp = 1.0f32;
			for k in 1..=10 {
				s += sin(x * k as f32) * amp / k as f32;
				amp *= 0.82;
			}
			soft[i] = s;
		}
		let peak = soft.iter().fold(0.0f32, |m, &v| m.max(v.abs())).max(1e-6);
		for v in soft.iter_mut() {
			*v /= peak;
		}
		Tables { sine, soft }
	}
}

impl Default for Tables {
	fn default() -> Self {
		Tables::new()
	}
}

/// Reads a table at a phase in [0, 1), interpolating.
#[inline(always)]
pub fn lookup(table: &[f32], phase: f32) -> f32 {
	let x = phase * TABLE as f32;
	let i = x as usize & (TABLE - 1);
	let f = x - (x as usize) as f32;
	table[i] + (table[i + 1] - table[i]) * f
}

/// Advances a phase by `freq / rate`, wrapping into [0, 1).
#[inline(always)]
pub fn step(phase: &mut f32, freq: f32, rate: f32) {
	*phase += freq / rate;
	if *phase >= 1.0 {
		*phase -= (*phase) as i32 as f32;
	}
}

/// A one-pole smoother: moves towards a target with a time constant in seconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct Smooth {
	pub value: f32,
}

impl Smooth {
	#[inline(always)]
	pub fn toward(&mut self, target: f32, seconds: f32, rate: f32) -> f32 {
		let k = 1.0 / (seconds.max(1e-4) * rate);
		self.value += (target - self.value) * k.min(1.0);
		self.value
	}
}

/// A state-variable filter (Chamberlin), giving low-pass and band-pass outputs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
	low: f32,
	band: f32,
}

impl Svf {
	/// Filters one sample with a cutoff in hertz and a damping (2 is gentle, 0.3 resonant). Returns (low-pass, band-pass).
	#[inline(always)]
	pub fn run(&mut self, x: f32, cutoff: f32, damping: f32, rate: f32) -> (f32, f32) {
		let f = 2.0 * sin(core::f32::consts::PI * clamp(cutoff, 10.0, rate * 0.2) / rate);
		self.low += f * self.band;
		let high = x - self.low - damping * self.band;
		self.band += f * high;
		(self.low, self.band)
	}
}

/// White noise from an integer hash, in [-1, 1).
#[derive(Clone, Copy, Debug)]
pub struct Noise {
	state: u32,
}

impl Noise {
	pub fn new(seed: u32) -> Noise {
		Noise { state: seed | 1 }
	}

	#[inline(always)]
	pub fn sample(&mut self) -> f32 {
		self.state ^= self.state << 13;
		self.state ^= self.state >> 17;
		self.state ^= self.state << 5;
		(self.state >> 8) as f32 / 8_388_608.0 - 1.0
	}
}

/// A feedback delay network: eight delay lines, mixed by a Householder matrix, each damped and fed back for a reverberation time.
pub struct Reverb {
	lines: Vec<Vec<f32>>,
	pos: Vec<usize>,
	gain: Vec<f32>,
	damp: Vec<f32>,
	damping: f32,
}

impl Reverb {
	/// `t60` is the time for the tail to fall 60 dB; `size` scales the line lengths (1 is a large hall); `damping` 0 to 1 darkens.
	pub fn new(rate: f32, t60: f32, size: f32, damping: f32) -> Reverb {
		const MS: [f32; 8] = [29.7, 37.1, 41.1, 43.7, 53.3, 59.1, 67.9, 73.3];
		let mut lines = Vec::new();
		let mut gain = Vec::new();
		for ms in MS {
			let len = ((ms * size * 0.001 * rate) as usize).max(8);
			lines.push(vec![0.0; len]);
			// Gain per pass for a 60 dB fall in t60: 10^(-3 len / (t60 rate)).
			gain.push(exp(-3.0 * ln(10.0) * len as f32 / (t60.max(0.1) * rate)));
		}
		Reverb {
			pos: vec![0; 8],
			damp: vec![0.0; 8],
			lines,
			gain,
			damping: clamp(damping, 0.0, 0.95),
		}
	}

	/// One sample in, a stereo pair out.
	#[inline]
	pub fn run(&mut self, input: f32) -> (f32, f32) {
		let mut outs = [0.0f32; 8];
		let mut sum = 0.0;
		for (i, line) in self.lines.iter().enumerate() {
			let v = line[self.pos[i]];
			outs[i] = v;
			sum += v;
		}
		let k = sum * (2.0 / 8.0);
		let (mut left, mut right) = (0.0, 0.0);
		for i in 0..8 {
			let fed = (outs[i] - k) * self.gain[i];
			// Damping: a one-pole low-pass in the loop, so the tail darkens as it decays.
			self.damp[i] += (fed - self.damp[i]) * (1.0 - self.damping);
			let line = &mut self.lines[i];
			// Every line takes the input with the same sign; the lines' lengths alone decorrelate the two sides, so the tail keeps
			// its body when the stereo is folded to mono.
			line[self.pos[i]] = self.damp[i] + input * 0.35;
			self.pos[i] = (self.pos[i] + 1) % line.len();
			if i % 2 == 0 {
				left += outs[i];
			} else {
				right += outs[i];
			}
		}
		(left * 0.5, right * 0.5)
	}
}

/// A soft limiter: transparent below about half scale, approaching ±1 smoothly above it.
#[inline(always)]
pub fn soft_limit(x: f32) -> f32 {
	let x = clamp(x, -3.0, 3.0);
	x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// A frequency `semitones` above `base`.
#[inline]
pub fn note(base: f32, semitones: f32) -> f32 {
	base * exp(semitones * (ln(2.0) / 12.0))
}

/// The tonic and mode for a palette: (root in hertz, scale steps in semitones).
pub fn key_for(palette: crate::palette::PaletteId) -> (f32, [f32; 7]) {
	use crate::palette::PaletteId::*;
	const LYDIAN: [f32; 7] = [0.0, 2.0, 4.0, 6.0, 7.0, 9.0, 11.0];
	const DORIAN: [f32; 7] = [0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 10.0];
	const MIXOLYDIAN: [f32; 7] = [0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 10.0];
	const AEOLIAN: [f32; 7] = [0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0];
	match palette {
		Dusk | Quill => (73.42, LYDIAN),
		Indigo => (58.27, DORIAN),
		Ember => (82.41, MIXOLYDIAN),
		Aurora => (92.50, LYDIAN),
		Graphite => (55.0, AEOLIAN),
	}
}

/// The score for a saver, if it has one yet (SAV-D34).
pub fn score_for(
	kind: crate::settings::SaverKind,
	settings: &crate::settings::Settings,
	seed: u32,
	rate: u32,
) -> Option<Box<dyn Score>> {
	use crate::settings::SaverKind;
	match kind {
		SaverKind::Twilight => Some(Box::new(twilight::TwilightScore::new(settings, seed, rate))),
		SaverKind::Maze => Some(Box::new(maze::MazeScore::new(settings, seed, rate))),
		_ => None,
	}
}

/// Turns a scene's score into 16-bit stereo: volume, the opening fade and the limiter, a block at a time.
pub struct Engine {
	pub rate: u32,
	score: Box<dyn Score>,
	buffer: Vec<f32>,
	frames: u64,
	volume: f32,
	/// Seconds of fade-in from silence at the start (the saver's own fade-up).
	fade_in: f32,
}

/// The run-time sample rate for a detail level (SAV-D35): the Pentium III's budget at Low, full fidelity from High up. Offline renders
/// and the Linux edition always use 48 kHz.
pub fn rate_for(detail: crate::settings::Detail) -> u32 {
	use crate::settings::Detail::*;
	match detail {
		Low => 22_050,
		Medium => 32_000,
		High => 44_100,
		Ultra => 48_000,
	}
}

/// The rate of offline renders and the Linux edition.
pub const FULL_RATE: u32 = 48_000;

/// Frames rendered per cue sample.
pub const BLOCK: usize = 256;

impl Engine {
	pub fn new(rate: u32, score: Box<dyn Score>, volume: f32, fade_in: f32) -> Engine {
		Engine {
			rate,
			score,
			buffer: vec![0.0; BLOCK * 2],
			frames: 0,
			volume: saturate(volume),
			fade_in,
		}
	}

	/// The time in seconds of the next frame to be rendered.
	pub fn time(&self) -> f64 {
		self.frames as f64 / self.rate as f64
	}

	/// Fills `out` (stereo, interleaved) asking `cues` for each block's cues at its start time.
	pub fn render(&mut self, out: &mut [i16], cues: &mut dyn FnMut(f64) -> Cues) {
		for chunk in out.chunks_mut(BLOCK * 2) {
			let frames = chunk.len() / 2;
			let t = self.time();
			let c = cues(t);
			let buf = &mut self.buffer[..frames * 2];
			buf.fill(0.0);
			self.score.render(&c, buf);
			for (i, pair) in chunk.chunks_mut(2).enumerate() {
				let ft = (self.frames + i as u64) as f32 / self.rate as f32;
				let open = if self.fade_in > 0.0 {
					saturate(ft / self.fade_in)
				} else {
					1.0
				};
				let g = self.volume * open * open;
				for (k, s) in pair.iter_mut().enumerate() {
					*s = (soft_limit(buf[i * 2 + k] * g) * 32_000.0) as i16;
				}
			}
			self.frames += frames as u64;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn primitives_behave() {
		let t = Tables::new();
		assert!((lookup(&t.sine, 0.25) - 1.0).abs() < 1e-3);
		assert!(t.soft.iter().all(|v| v.abs() <= 1.0001));
		assert!(soft_limit(10.0) <= 1.0 && soft_limit(0.1) > 0.099);
		assert!((note(110.0, 12.0) - 220.0).abs() < 0.01);
		let mut r = Reverb::new(22_050.0, 2.0, 1.0, 0.3);
		let mut energy = 0.0;
		for i in 0..44_100 {
			let (l, rr) = r.run(if i == 0 { 1.0 } else { 0.0 });
			energy += l * l + rr * rr;
			assert!(l.is_finite() && rr.is_finite());
		}
		assert!(energy > 0.01);
		let mut n = Noise::new(3);
		assert!((0..1000).map(|_| n.sample()).all(|v| (-1.0..1.0).contains(&v)));
	}
}
