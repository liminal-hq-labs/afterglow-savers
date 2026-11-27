// The sound feeder: a saver's score turned into 16-bit stereo blocks, from a second tiny scene that only supplies cues (SAV-D34, SAV-D40).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! No Windows in here, so the host tests it. The device side (`waveout`) asks for blocks and queues them.

use alloc::boxed::Box;

use savers_core::audio::{BLOCK, Engine, Score, rate_for};
use savers_core::scene::Scene;
use savers_core::settings::{Key, Settings};

/// The size of the tiny scene that supplies cues. Only its timeline matters, and it never draws, so the audio, which runs well ahead
/// of the picture in the device's queue, never advances the visible scene's state.
pub const CUE_SIZE: (usize, usize) = (32, 18);
/// Seconds of silence-to-full at the start, as the picture's own fade-up.
const FADE_IN: f32 = 1.0;
/// Frames at the start of a block that follows a gap, ramped up so the late block does not click.
const RAMP: usize = 220;

/// Whether a full-screen run plays sound, and at what rate (before the device's own say) and volume (0 to 1).
pub fn plan(settings: &Settings) -> Option<(u32, f32)> {
	let volume = settings.get(Key::Volume);
	if settings.get(Key::Sound) == 0 || volume == 0 {
		return None;
	}
	Some((rate_for(settings.detail()), volume as f32 / 100.0))
}

/// Frames per slice: about 50 ms, a whole number of the engine's blocks so that the cues come at the same times however a render is
/// cut up.
pub fn slice_frames(rate: u32) -> usize {
	(rate as usize / 20 / BLOCK).max(1) * BLOCK
}

pub struct Feeder {
	engine: Engine,
	scene: Box<dyn Scene>,
	speed: f64,
}

impl Feeder {
	/// `scene` is a second instance of the saver's scene, made at `CUE_SIZE` with the same seed as the visible one; `score` is that
	/// saver's own score.
	pub fn new(score: Box<dyn Score>, settings: &Settings, rate: u32, volume: f32, scene: Box<dyn Scene>) -> Feeder {
		Feeder {
			engine: Engine::new(rate, score, volume, FADE_IN),
			scene,
			speed: f64::from(settings.speed()),
		}
	}

	/// Renders `out.len() / 2` frames, a whole number of slices. The score follows scene time, which the Speed setting stretches.
	pub fn render(&mut self, out: &mut [i16]) {
		let speed = self.speed;
		let scene = &mut self.scene;
		self.engine.render(out, &mut |t| scene.cues(t * speed));
	}
}

/// Ramps the first few frames of a block up from silence.
pub fn ramp_in(block: &mut [i16]) {
	for (i, pair) in block.chunks_mut(2).take(RAMP).enumerate() {
		for sample in pair {
			*sample = (i32::from(*sample) * i as i32 / RAMP as i32) as i16;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloc::vec;
	use savers_core::scenes::twilight::Twilight;
	use savers_core::settings::SaverKind;

	fn feeder(volume: f32) -> Feeder {
		let settings = Settings::defaults(SaverKind::Twilight);
		let scene = Box::new(Twilight::new(&settings, 7, CUE_SIZE.0, CUE_SIZE.1));
		let score = savers_core::audio::score_for(SaverKind::Twilight, &settings, 7, 22_050).expect("twilight has a score");
		Feeder::new(score, &settings, 22_050, volume, scene)
	}

	#[test]
	fn defaults_play_quietly_at_the_ladder_rate() {
		let mut settings = Settings::defaults(SaverKind::Maze);
		let (rate, volume) = plan(&settings).expect("sound is on by default");
		assert_eq!(rate, rate_for(settings.detail()));
		assert!((volume - 0.28).abs() < 1e-6);
		settings.set(Key::Sound, 0);
		assert!(plan(&settings).is_none());
		settings.set(Key::Sound, 1);
		settings.set(Key::Volume, 0);
		assert!(plan(&settings).is_none());
	}

	#[test]
	fn cutting_a_render_into_slices_changes_nothing() {
		let slice = slice_frames(22_050);
		assert_eq!(slice % BLOCK, 0);
		let mut whole = vec![0i16; slice * 8 * 2];
		feeder(0.5).render(&mut whole);
		let mut cut = vec![0i16; slice * 8 * 2];
		let mut second = feeder(0.5);
		for part in cut.chunks_mut(slice * 2) {
			second.render(part);
		}
		assert_eq!(whole, cut);
		assert!(whole.iter().any(|&s| s != 0));
	}

	#[test]
	fn the_sound_fades_up_and_stays_clear_of_clipping() {
		let mut f = feeder(1.0);
		let mut block = vec![0i16; 22_050 * 6 * 2];
		f.render(&mut block);
		let early = block[..2_000].iter().map(|&s| i32::from(s).abs()).max().unwrap();
		let peak = block.iter().map(|&s| i32::from(s).abs()).max().unwrap();
		assert!(early < 1_500, "the opening is nearly silent, got {early}");
		assert!(peak < 32_001);
	}

	#[test]
	fn a_ramp_starts_from_silence() {
		let mut block = vec![10_000i16; 1_000];
		ramp_in(&mut block);
		assert_eq!(block[0], 0);
		assert!(block[RAMP * 2 - 2].abs() > 9_000);
		assert_eq!(block[RAMP * 2], 10_000);
	}
}
