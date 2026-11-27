// The parity test: the GPU edition's Twilight against the CPU scene's frames, headless.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Frames are 160 × 90 at the CPU goldens' seed and detail levels, at times that cover every shot (the establishing shot with the sun,
//! the push-in, the satellite pass and the night-side drift with the sun behind the limb). Grain is off in the scene comparison; the
//! finish is compared on its own on a synthetic frame, with the grain on. The test draws on the adapter `AFTERGLOW_GPU_ADAPTER` names,
//! else the first software adapter (lavapipe), else any, and skips with a note when the machine has none.

use savers_core::frame::Frame;
use savers_core::post::{FinishParams, Finisher};
use savers_core::settings::{Key, SaverKind, Settings};
use savers_core::twilight_shared::Shared;
use savers_gpu::compare::{cpu_frames, difference};
use savers_gpu::readback::{read_frame, write_frame};
use savers_gpu::{Finish, Gpu, TwilightRenderer};

const SEED: u32 = 7;
const SIZE: (usize, usize) = (160, 90);
/// One time in each shot: 0 to 25 s, 25 to 50 s, 50 to 70 s and 70 to 90 s.
const TIMES: [f64; 6] = [2.0, 15.0, 40.0, 62.0, 78.0, 85.0];

/// The adapter to test on: the environment's, else lavapipe so CI-like conditions are the default.
fn gpu() -> Option<Gpu> {
	let wanted = std::env::var("AFTERGLOW_GPU_ADAPTER")
		.ok()
		.filter(|name| !name.is_empty())
		.unwrap_or_else(|| "llvmpipe".into());
	match Gpu::new(Some(&wanted)).or_else(|_| Gpu::new(None)) {
		Ok(gpu) => {
			eprintln!("parity: drawing on {}", gpu.describe());
			Some(gpu)
		}
		Err(e) => {
			eprintln!("parity: skipped, {e} (set AFTERGLOW_GPU_ADAPTER, or install a Vulkan driver such as lavapipe)");
			None
		}
	}
}

fn settings(detail: u32) -> Settings {
	let mut s = Settings::defaults(SaverKind::Twilight);
	s.set(Key::Detail, detail);
	s.set(Key::RenderScale, 100);
	s.set(Key::Grain, 0);
	s
}

fn gpu_frames(gpu: &Gpu, settings: &Settings, times: &[f64]) -> Vec<Frame> {
	let shared = Shared::new(settings, SEED, SIZE.0, SIZE.1);
	let mut renderer = TwilightRenderer::new(gpu, settings, shared, SEED, SIZE, SIZE, false);
	times
		.iter()
		.map(|&t| {
			renderer.render(gpu, t, t / settings.speed() as f64);
			renderer.read_back(gpu)
		})
		.collect()
}

fn dump(name: &str, cpu: &Frame, gpu_frame: &Frame) {
	let Some(dir) = std::env::var_os("AFTERGLOW_PARITY_DUMP") else {
		return;
	};
	let dir = std::path::PathBuf::from(dir);
	std::fs::create_dir_all(&dir).expect("the dump folder");
	let mut ppm = format!("P6\n{} {}\n255\n", cpu.w * 2, cpu.h).into_bytes();
	for y in 0..cpu.h {
		for frame in [cpu, gpu_frame] {
			for x in 0..cpu.w {
				let c = frame.px[y * cpu.w + x];
				ppm.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
			}
		}
	}
	std::fs::write(dir.join(format!("{name}.ppm")), ppm).expect("the dump file");
}

#[test]
fn twilight_matches_the_cpu_scene() {
	let Some(gpu) = gpu() else { return };
	let mut worst_mean = 0.0f64;
	let mut worst_far = 0.0f64;
	let mut failures = Vec::new();
	for detail in 0..4 {
		let settings = settings(detail);
		let cpu = cpu_frames(&settings, SEED, SIZE, &TIMES);
		let on_gpu = gpu_frames(&gpu, &settings, &TIMES);
		for (i, (a, b)) in cpu.iter().zip(&on_gpu).enumerate() {
			let d = difference(a, b);
			eprintln!("detail {detail} t {:>4}: {}", TIMES[i], d.describe());
			dump(&format!("detail{detail}-t{}", TIMES[i]), a, b);
			worst_mean = worst_mean.max(d.mean_abs.iter().cloned().fold(0.0, f64::max));
			worst_far = worst_far.max(d.far_share);
			if !d.within_budget() {
				failures.push(format!("detail {detail} t {}: {}", TIMES[i], d.describe()));
			}
		}
	}
	eprintln!(
		"parity: worst mean {worst_mean:.2} levels, worst far share {:.3}%",
		worst_far * 100.0
	);
	assert!(failures.is_empty(), "outside the budget:\n{}", failures.join("\n"));
}

#[test]
fn the_palettes_and_moons_match_too() {
	let Some(gpu) = gpu() else { return };
	for (palette, moons) in [(2u32, 0u32), (3, 1)] {
		let mut settings = settings(3);
		settings.set(Key::Palette, palette);
		settings.set(Key::Moons, moons);
		let cpu = cpu_frames(&settings, SEED, SIZE, &TIMES[..3]);
		let on_gpu = gpu_frames(&gpu, &settings, &TIMES[..3]);
		for (i, (a, b)) in cpu.iter().zip(&on_gpu).enumerate() {
			let d = difference(a, b);
			eprintln!("palette {palette} moons {moons} t {}: {}", TIMES[i], d.describe());
			assert!(
				d.within_budget(),
				"palette {palette} moons {moons} t {}: {}",
				TIMES[i],
				d.describe()
			);
		}
	}
}

/// The finish on its own: a synthetic frame with bright shapes on a mid-grey ramp, through the CPU chain and the GPU chain.
#[test]
fn the_finish_matches_the_cpu_chain() {
	let Some(gpu) = gpu() else { return };
	let (w, h) = (160usize, 90usize);
	let mut frame = Frame::new(w, h);
	for y in 0..h {
		for x in 0..w {
			let level = (x * 255 / w) as u32 / 3;
			frame.set(x, y, level << 16 | (level / 2) << 8 | (level / 3));
		}
	}
	for y in 30..44 {
		for x in 40..60 {
			frame.set(x, y, 0xFFE0A0);
		}
	}
	for y in 60..64 {
		for x in 100..130 {
			frame.set(x, y, 0xA0C0FF);
		}
	}
	let cases = [
		FinishParams {
			grain: 0,
			letterbox: 0.0,
			..FinishParams::default()
		},
		FinishParams {
			grain: 0,
			letterbox: 2.39,
			vignette: 80,
			fade: 0.6,
			bloom: 100,
			streaks: 70,
			..FinishParams::default()
		},
		FinishParams {
			grain: 0,
			coarse: true,
			streak_tint: 0x70B0FF,
			..FinishParams::default()
		},
	];
	for (i, params) in cases.iter().enumerate() {
		let mut cpu_in = frame.clone();
		let mut finisher = Finisher::new();
		finisher.glow(&mut cpu_in, params);
		let cpu_out = finisher.output(&cpu_in, w, h, params, 1).clone();
		let finish = Finish::new(&gpu, (w, h), (w, h), params.coarse);
		write_frame(&gpu, finish.input(), &frame);
		let mut encoder = gpu
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
		finish.encode(&gpu, &mut encoder, params, 1);
		gpu.queue.submit([encoder.finish()]);
		let gpu_out = read_frame(&gpu, finish.output(), w, h);
		let d = difference(&cpu_out, &gpu_out);
		eprintln!("finish case {i}: {}", d.describe());
		assert!(d.within_budget(), "finish case {i}: {}", d.describe());
	}
}

/// The grain moves every pixel between frames and leaves the bars black; the same frame number gives the same grain.
#[test]
fn grain_moves_and_letterbox_is_black() {
	let Some(gpu) = gpu() else { return };
	let (w, h) = (64usize, 64usize);
	let mut frame = Frame::new(w, h);
	frame.fill(0x404040);
	let params = FinishParams {
		letterbox: 2.0,
		vignette: 0,
		bloom: 0,
		streaks: 0,
		..FinishParams::default()
	};
	let finish = Finish::new(&gpu, (w, h), (w, h), false);
	write_frame(&gpu, finish.input(), &frame);
	let draw = |frame_no: u32| {
		let mut encoder = gpu
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
		finish.encode(&gpu, &mut encoder, &params, frame_no);
		gpu.queue.submit([encoder.finish()]);
		read_frame(&gpu, finish.output(), w, h)
	};
	let a = draw(1);
	let b = draw(2);
	let again = draw(1);
	assert!(a.px[0..w].iter().all(|&c| c == 0), "the top bar is black");
	assert!(a.px[(h - 1) * w..].iter().all(|&c| c == 0), "the bottom bar is black");
	let moved = a.px[32 * w..33 * w]
		.iter()
		.zip(&b.px[32 * w..33 * w])
		.filter(|(x, y)| x != y)
		.count();
	assert!(moved > 30, "grain changes most pixels between frames: {moved}");
	assert_eq!(a.px, again.px, "the same frame number gives the same grain");
}
