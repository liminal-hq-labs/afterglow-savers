// Sound: the core's synth on its own thread, handed to the sound card through cpal, or written to a WAV file with no card at all.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use savers_core::audio::{BLOCK, Engine, FULL_RATE, score_for};
use savers_core::scene::Scene;
use savers_core::scenes::build;
use savers_core::settings::{Key, SaverKind, Settings};

/// Frames in each block the synth thread hands to the card's callback.
const CHUNK_FRAMES: usize = BLOCK * 8;
/// Blocks queued ahead: about 340 ms at 48 kHz, which rides out a slow frame on the main thread.
const QUEUE_BLOCKS: usize = 8;
/// The fade out when the saver ends, in seconds.
const FADE_OUT: f32 = 0.15;

/// The Twilight score and the tiny second scene that gives it its cues. Nothing is drawn: the scene only knows the time.
pub struct Synth {
	engine: Engine,
	scene: Box<dyn Scene>,
	speed: f64,
	scratch: Vec<i16>,
}

impl Synth {
	/// Whether the settings ask for any sound.
	pub fn wanted(settings: &Settings) -> bool {
		settings.get(Key::Sound) == 1 && settings.get(Key::Volume) > 0
	}

	/// `None` when the settings ask for no sound. The volume is the setting's, as a fraction; the first second fades up from silence
	/// as the picture does.
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> Option<Synth> {
		if !Synth::wanted(settings) {
			return None;
		}
		let score = score_for(SaverKind::Twilight, settings, seed, rate)?;
		let engine = Engine::new(rate, score, settings.get(Key::Volume) as f32 / 100.0, 1.0);
		Some(Synth {
			engine,
			scene: build(SaverKind::Twilight, settings, seed, 32, 18),
			speed: settings.speed() as f64,
			scratch: Vec::new(),
		})
	}

	/// Renders `frames` stereo frames as 16-bit samples, left and right interleaved.
	pub fn render_i16(&mut self, frames: usize) -> &[i16] {
		self.scratch.resize(frames * 2, 0);
		let scene = &mut self.scene;
		let speed = self.speed;
		self.engine.render(&mut self.scratch, &mut |t| scene.cues(t * speed));
		&self.scratch
	}
}

/// Writes 16-bit stereo PCM as a WAV file.
pub fn write_wav(path: &Path, rate: u32, samples: &[i16]) -> std::io::Result<()> {
	let data_bytes = (samples.len() * 2) as u32;
	let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
	file.write_all(b"RIFF")?;
	file.write_all(&(36 + data_bytes).to_le_bytes())?;
	file.write_all(b"WAVEfmt ")?;
	file.write_all(&16u32.to_le_bytes())?;
	file.write_all(&1u16.to_le_bytes())?;
	file.write_all(&2u16.to_le_bytes())?;
	file.write_all(&rate.to_le_bytes())?;
	file.write_all(&(rate * 4).to_le_bytes())?;
	file.write_all(&4u16.to_le_bytes())?;
	file.write_all(&16u16.to_le_bytes())?;
	file.write_all(b"data")?;
	file.write_all(&data_bytes.to_le_bytes())?;
	for sample in samples {
		file.write_all(&sample.to_le_bytes())?;
	}
	file.flush()
}

/// Renders `seconds` of the score at 48 kHz to a WAV file with no sound card involved. Returns what it wrote, for a log line.
pub fn render_wav(path: &Path, settings: &Settings, seed: u32, seconds: f64) -> Result<String, String> {
	let mut synth = Synth::new(settings, seed, FULL_RATE).ok_or("the settings have Sound or Volume at 0")?;
	let total = (seconds * FULL_RATE as f64) as usize;
	let mut samples = Vec::with_capacity(total * 2);
	let mut left = total;
	while left > 0 {
		let frames = left.min(CHUNK_FRAMES);
		samples.extend_from_slice(synth.render_i16(frames));
		left -= frames;
	}
	write_wav(path, FULL_RATE, &samples).map_err(|error| format!("{}: {error}", path.display()))?;
	let peak = samples.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0);
	Ok(format!(
		"{seconds:.1} s at {FULL_RATE} Hz to {}, peak {:.1} dBFS",
		path.display(),
		20.0 * (peak.max(1) as f64 / 32_768.0).log10()
	))
}

/// A running sound: the synth thread and the card's stream. Dropping it fades the sound out and stops both.
pub struct Playback {
	stream: Option<cpal::Stream>,
	stop: Arc<AtomicBool>,
	thread: Option<JoinHandle<()>>,
	/// Set when the saver ends: the callback fades the sound out.
	closing: Arc<AtomicBool>,
	pub description: String,
}

/// Starts the score on the default output device: 48 kHz, 32-bit float, stereo when the device allows it, or else the device's own
/// rate (the synth is made for whichever it is). `Err` says why there is no sound; the caller then carries on in silence.
pub fn start(settings: &Settings, seed: u32) -> Result<Playback, String> {
	let host = cpal::default_host();
	let device = host.default_output_device().ok_or("no audio output device")?;
	let name = device
		.description()
		.map(|d| d.name().to_string())
		.unwrap_or_else(|_| "unnamed device".into());
	let ranges = device
		.supported_output_configs()
		.map_err(|error| format!("{name}: {error}"))?
		.filter(|range| range.sample_format() == cpal::SampleFormat::F32 && range.channels() >= 2)
		.collect::<Vec<_>>();
	let chosen = ranges
		.iter()
		.find_map(|range| range.try_with_sample_rate(FULL_RATE))
		.or_else(|| ranges.first().map(|range| range.with_max_sample_rate()))
		.ok_or_else(|| format!("{name} has no 32-bit float output"))?;
	let rate = chosen.sample_rate();
	let channels = chosen.channels() as usize;
	if !Synth::wanted(settings) {
		return Err("the settings have Sound or Volume at 0".into());
	}

	let (sender, receiver): (SyncSender<Vec<f32>>, Receiver<Vec<f32>>) = sync_channel(QUEUE_BLOCKS);
	let stop = Arc::new(AtomicBool::new(false));
	let closing = Arc::new(AtomicBool::new(false));
	let thread = {
		let stop = stop.clone();
		let settings = settings.clone();
		std::thread::Builder::new()
			.name("afterglow-synth".into())
			.spawn(move || {
				// The synth is made here: its score and scene are not `Send`.
				let Some(mut synth) = Synth::new(&settings, seed, rate) else {
					return;
				};
				while !stop.load(Ordering::Relaxed) {
					let block: Vec<f32> = synth
						.render_i16(CHUNK_FRAMES)
						.iter()
						.map(|&sample| sample as f32 / 32_768.0)
						.collect();
					// A full queue is the normal state: wait for the card to drain it, but never past a stop.
					let mut block = Some(block);
					while let Some(pending) = block.take() {
						match sender.try_send(pending) {
							Ok(()) => {}
							Err(std::sync::mpsc::TrySendError::Full(back)) => {
								if stop.load(Ordering::Relaxed) {
									return;
								}
								block = Some(back);
								std::thread::sleep(Duration::from_millis(10));
							}
							Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
						}
					}
				}
			})
			.map_err(|error| format!("could not start the synth thread: {error}"))?
	};

	let mut feeder = Feeder::new(receiver, channels, rate, closing.clone());
	let stream = device
		.build_output_stream(
			&chosen.config(),
			move |data: &mut [f32], _| feeder.fill(data),
			|error| eprintln!("afterglow-twilight-gpu: sound stopped: {error}"),
			None,
		)
		.map_err(|error| format!("{name}: {error}"));
	let stream = match stream.and_then(|stream| stream.play().map(|()| stream).map_err(|error| format!("{name}: {error}"))) {
		Ok(stream) => stream,
		Err(why) => {
			stop.store(true, Ordering::Relaxed);
			let _ = thread.join();
			return Err(why);
		}
	};
	Ok(Playback {
		stream: Some(stream),
		stop,
		thread: Some(thread),
		closing,
		description: format!("{name}, {rate} Hz, {channels} channels"),
	})
}

impl Drop for Playback {
	fn drop(&mut self) {
		self.closing.store(true, Ordering::Relaxed);
		// Let the callback play out the fade, then stop.
		std::thread::sleep(Duration::from_millis((FADE_OUT * 1000.0) as u64 + 60));
		self.stop.store(true, Ordering::Relaxed);
		self.stream.take();
		if let Some(thread) = self.thread.take() {
			let _ = thread.join();
		}
	}
}

/// The card's callback side: takes blocks from the synth thread, spreads them over the device's channels, and covers a gap (the
/// thread being late) and the end of the saver with a short ramp, so neither is a click: a gap holds the last frame and fades it
/// over 20 ms, and the end fades the sound over [`FADE_OUT`].
struct Feeder {
	receiver: Receiver<Vec<f32>>,
	channels: usize,
	current: Vec<f32>,
	position: usize,
	last: (f32, f32),
	gain: f32,
	gap_step: f32,
	end_step: f32,
	closing: Arc<AtomicBool>,
}

impl Feeder {
	fn new(receiver: Receiver<Vec<f32>>, channels: usize, rate: u32, closing: Arc<AtomicBool>) -> Feeder {
		Feeder {
			receiver,
			channels,
			current: Vec::new(),
			position: 0,
			last: (0.0, 0.0),
			gain: 1.0,
			gap_step: 1.0 / (0.02 * rate as f32),
			end_step: 1.0 / (FADE_OUT * rate as f32),
			closing,
		}
	}

	fn fill(&mut self, data: &mut [f32]) {
		let closing = self.closing.load(Ordering::Relaxed);
		for frame in data.chunks_mut(self.channels) {
			if self.position >= self.current.len()
				&& let Ok(block) = self.receiver.try_recv()
			{
				self.current = block;
				self.position = 0;
			}
			let have = self.position < self.current.len();
			if have {
				self.last = (self.current[self.position], self.current[self.position + 1]);
				self.position += 2;
			}
			// Towards the wanted gain one step a frame: a gap or the end fades the sound out, and a block arriving after a gap
			// fades it back in.
			let want = if have && !closing { 1.0 } else { 0.0 };
			let step = if closing { self.end_step } else { self.gap_step };
			if self.gain < want {
				self.gain = (self.gain + step).min(want);
			} else if self.gain > want {
				self.gain = (self.gain - step).max(want);
			}
			for (index, out) in frame.iter_mut().enumerate() {
				*out = match index {
					0 => self.last.0 * self.gain,
					1 => self.last.1 * self.gain,
					_ => 0.0,
				};
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn settings(volume: u32) -> Settings {
		let mut settings = Settings::defaults(SaverKind::Twilight);
		settings.set(Key::Volume, volume);
		settings
	}

	#[test]
	fn no_sound_when_the_setting_is_off() {
		let mut off = settings(28);
		off.set(Key::Sound, 0);
		assert!(Synth::new(&off, 1, FULL_RATE).is_none());
		assert!(Synth::new(&settings(0), 1, FULL_RATE).is_none());
	}

	#[test]
	fn the_score_is_audible_and_free_of_clicks() {
		let mut synth = Synth::new(&settings(28), 1, FULL_RATE).unwrap();
		let mut samples = Vec::new();
		for _ in 0..40 {
			samples.extend_from_slice(synth.render_i16(CHUNK_FRAMES));
		}
		let peak = samples.iter().map(|&s| (s as i32).abs()).max().unwrap();
		let tail = &samples[samples.len() / 2..];
		let rms = (tail.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt();
		assert!(peak > 200, "peak {peak} is silence");
		assert!(peak < 24_000, "peak {peak} is too loud for the default volume");
		assert!(rms > 20.0, "rms {rms} is silence");
		let largest_step = samples
			.windows(2)
			.map(|pair| (pair[1] as i32 - pair[0] as i32).abs())
			.max()
			.unwrap();
		assert!(largest_step < 2_500, "a step of {largest_step} between samples is a click");
	}

	#[test]
	fn the_feeder_ramps_through_an_underrun_and_the_end() {
		let (sender, receiver) = sync_channel(2);
		let closing = Arc::new(AtomicBool::new(false));
		let mut feeder = Feeder::new(receiver, 2, FULL_RATE, closing.clone());
		sender.send(vec![0.5; 4000]).unwrap();
		let mut out = vec![0.0f32; 8000];
		feeder.fill(&mut out);
		// The block ran dry after 2000 frames: the tail fades out, then silence, with no jump of more than a ramp's step.
		let largest = out.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0f32, f32::max);
		assert!(largest < 0.01, "step {largest}");
		assert_eq!(out[out.len() - 1], 0.0);
		sender.send(vec![0.5; 4000]).unwrap();
		let mut again = vec![0.0f32; 1000];
		feeder.fill(&mut again);
		assert!(again[0].abs() < 0.01 && again[999] > 0.2, "fades back in");
		closing.store(true, Ordering::Relaxed);
		let mut last = vec![0.0f32; 20_000];
		feeder.fill(&mut last);
		assert_eq!(last[last.len() - 1], 0.0);
	}

	#[test]
	fn a_wav_is_written_with_a_correct_header() {
		let path = std::env::temp_dir().join(format!("afterglow-test-{}.wav", std::process::id()));
		render_wav(&path, &settings(28), 1, 1.0).unwrap();
		let bytes = std::fs::read(&path).unwrap();
		std::fs::remove_file(&path).unwrap();
		assert_eq!(&bytes[0..4], b"RIFF");
		assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), FULL_RATE);
		assert_eq!(bytes.len(), 44 + FULL_RATE as usize * 4);
	}
}
