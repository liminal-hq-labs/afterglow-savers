// Sound out: `waveOut` playback in three 300 ms buffers, refilled from the frame loop with no thread (SAV-D40, docs/audio.md).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;

use savers_core::scene::Scene;
use savers_core::settings::Settings;

use crate::saver::{MakeScene, MakeScore};
use crate::sound::{self, CUE_SIZE, Feeder};
use crate::sys::{self, Handle};

const BUFFERS: usize = 3;
/// A buffer is this many slices (about 50 ms each), so about 300 ms.
const SLICES_PER_BUFFER: usize = 6;
/// The most slices rendered in one frame, so that a slow frame never turns into a long one.
const MOST_SLICES: usize = 6;
/// The rate a device that refuses the Detail ladder's is asked for next.
/// How far ahead of real time the audio may be rendered, in seconds: more than the three buffers hold, so a device that keeps time
/// is limited by its buffers, never by this.
const MOST_AHEAD: f64 = 1.2;
const FALLBACK_RATE: u32 = 22_050;

pub struct Player {
	device: Handle,
	feeder: Feeder,
	rate: u32,
	headers: Box<[sys::WaveHdr; BUFFERS]>,
	data: [Vec<i16>; BUFFERS],
	queued: [bool; BUFFERS],
	/// The buffer being filled and how many of its slices are done.
	filling: Option<(usize, usize)>,
	slice: usize,
	/// Whether the next buffer follows a gap (the device ran dry), so it opens on a short ramp.
	ramp_next: bool,
	started: bool,
	/// Seconds of frame loop so far and frames rendered so far: audio is never rendered further ahead of the first than `MOST_AHEAD`,
	/// so a device that does not keep time (a null driver) cannot make the saver render faster than real time.
	clock: f64,
	rendered: u64,
}

fn open_device(rate: u32) -> Option<Handle> {
	let format = sys::WaveFormatEx {
		format_tag: sys::WAVE_FORMAT_PCM,
		channels: 2,
		samples_per_second: rate,
		bytes_per_second: rate * 4,
		block_align: 4,
		bits_per_sample: 16,
		extra_size: 0,
	};
	let mut device: Handle = sys::NULL;
	// SAFETY: valid out pointer and format; no callback.
	let result = unsafe { sys::waveOutOpen(&mut device, sys::WAVE_MAPPER, &format, 0, 0, 0) };
	if result == sys::MMSYSERR_NOERROR && !device.is_null() {
		Some(device)
	} else {
		None
	}
}

impl Player {
	/// Opens the device for a full-screen run, or returns `None` when sound is off, the saver has no score or there is no device
	/// that will take 16-bit stereo (no dialog, no error: the saver runs silently).
	pub fn start(settings: &Settings, seed: u32, make: MakeScene, score: MakeScore) -> Option<Player> {
		let (wanted, volume) = sound::plan(settings)?;
		let (device, rate) = match open_device(wanted) {
			Some(device) => (device, wanted),
			None if wanted != FALLBACK_RATE => (open_device(FALLBACK_RATE)?, FALLBACK_RATE),
			None => return None,
		};
		let scene: Box<dyn Scene> = make(settings, seed, CUE_SIZE.0, CUE_SIZE.1);
		let Some(score) = score(settings, seed, rate) else {
			// SAFETY: the device was just opened.
			unsafe { sys::waveOutClose(device) };
			return None;
		};
		let feeder = Feeder::new(score, settings, rate, volume, scene);
		let slice = sound::slice_frames(rate);
		let empty = || sys::WaveHdr {
			data: core::ptr::null_mut(),
			buffer_length: 0,
			bytes_recorded: 0,
			user: 0,
			flags: 0,
			loops: 0,
			next: core::ptr::null_mut(),
			reserved: 0,
		};
		let mut player = Player {
			device,
			feeder,
			rate,
			headers: Box::new([empty(), empty(), empty()]),
			data: [
				vec![0; slice * SLICES_PER_BUFFER * 2],
				vec![0; slice * SLICES_PER_BUFFER * 2],
				vec![0; slice * SLICES_PER_BUFFER * 2],
			],
			queued: [false; BUFFERS],
			filling: None,
			slice,
			ramp_next: false,
			started: false,
			clock: 0.0,
			rendered: 0,
		};
		for i in 0..BUFFERS {
			player.headers[i].data = player.data[i].as_mut_ptr() as *mut u8;
			player.headers[i].buffer_length = (player.data[i].len() * 2) as u32;
			// SAFETY: the header and its data live in the player, which unprepares them before they go.
			let result = unsafe { sys::waveOutPrepareHeader(device, &mut player.headers[i], size_of::<sys::WaveHdr>() as u32) };
			if result != sys::MMSYSERR_NOERROR {
				// A header that did not prepare is never written; dropping the player closes the device.
				player.queued = [true; BUFFERS];
				return None;
			}
		}
		Some(player)
	}

	/// Called once a frame with the seconds since the last call: marks the buffers the device has finished, and renders and queues as
	/// much as the time that has passed asks for (at least one slice), more when the queue is nearly dry.
	pub fn pump(&mut self, elapsed: f64) {
		self.clock += elapsed.max(0.0);
		for i in 0..BUFFERS {
			if self.queued[i] {
				// SAFETY: the driver sets the flag from its own thread, so it is read volatile.
				let flags = unsafe { core::ptr::read_volatile(&self.headers[i].flags) };
				if flags & sys::WHDR_DONE != 0 {
					self.queued[i] = false;
				}
			}
		}
		let waiting = self.queued.iter().filter(|&&q| q).count();
		let wanted = (elapsed * f64::from(self.rate) * 1.5 / self.slice as f64) as usize + 1;
		let mut budget = wanted.clamp(1, MOST_SLICES);
		if waiting <= 1 {
			budget = MOST_SLICES;
		}
		while budget > 0 {
			if (self.rendered + self.slice as u64) as f64 > (self.clock + MOST_AHEAD) * f64::from(self.rate) {
				return;
			}
			let (index, done) = match self.filling {
				Some(state) => state,
				None => {
					let Some(free) = self.queued.iter().position(|&q| !q) else {
						return;
					};
					// Nothing left in the device and it had started: the next block follows silence.
					self.ramp_next = self.started && waiting == 0;
					(free, 0)
				}
			};
			let frames = self.slice;
			let from = done * frames * 2;
			let part = &mut self.data[index][from..from + frames * 2];
			self.feeder.render(part);
			if done == 0 && self.ramp_next {
				sound::ramp_in(part);
				self.ramp_next = false;
			}
			budget -= 1;
			self.rendered += frames as u64;
			if done + 1 == SLICES_PER_BUFFER {
				self.filling = None;
				// SAFETY: a prepared header whose data is complete.
				let result =
					unsafe { sys::waveOutWrite(self.device, &mut self.headers[index], size_of::<sys::WaveHdr>() as u32) };
				self.queued[index] = result == sys::MMSYSERR_NOERROR;
				self.started = true;
			} else {
				self.filling = Some((index, done + 1));
			}
		}
	}
}

impl Drop for Player {
	fn drop(&mut self) {
		// SAFETY: stops the sound at once, hands every buffer back, then releases the device.
		unsafe {
			sys::waveOutReset(self.device);
			for header in self.headers.iter_mut() {
				sys::waveOutUnprepareHeader(self.device, header, size_of::<sys::WaveHdr>() as u32);
			}
			sys::waveOutClose(self.device);
		}
	}
}
