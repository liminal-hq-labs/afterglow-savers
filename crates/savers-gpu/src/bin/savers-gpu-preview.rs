// Renders Twilight on the graphics card, headless: PNG frames, CPU-against-GPU comparisons and frame times.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use savers_core::frame::Frame;
use savers_core::saver::{output_size, render_size};
use savers_core::settings::{Key, SaverKind, Settings, fields};
use savers_core::twilight_shared::Shared;
use savers_gpu::compare::{cpu_frames, difference};
use savers_gpu::{Gpu, TwilightRenderer, adapter_names, shared_for_gpu};

// The preview tool's own PNG writer, shared by path so there is one encoder.
#[path = "../../../savers-preview/src/png.rs"]
mod png;

const USAGE: &str = "\
usage: savers-gpu-preview <command> [options]

commands:
  adapters   list the graphics adapters wgpu can use
  frames     render GPU frames to PNG files: OUT/frame-00000.png and on
  compare    render the same frames on the CPU and the GPU: one PNG a time of CPU (left) over GPU (right), with the difference
  bench      time the GPU at 1080p and 4K (or --size)

options:
  --adapter NAME      the adapter to draw on, by part of its name (llvmpipe, radv, Radeon); default the first discrete one
  --size WxH          the window size (default 1920x1080)
  --seed N            the scene seed (default 1)
  --set NAME=VALUE    any Twilight registry value, e.g. --set Detail=3 (repeatable)
  --at S[,S...]       the scene times in seconds (default 12)
  --out PATH          frames: a folder; compare: a file, or with several times a folder
  --frames N          bench: frames timed per size (default 60)
  --stack             compare: stack all the times into one image, a row each
";

#[derive(Clone)]
struct Opts {
	command: String,
	adapter: Option<String>,
	size: (usize, usize),
	seed: u32,
	settings: Settings,
	times: Vec<f64>,
	out: Option<PathBuf>,
	frames: usize,
	size_given: bool,
	stack: bool,
}

fn parse() -> Result<Opts, String> {
	let mut args = std::env::args().skip(1);
	let command = args.next().ok_or("no command")?;
	if command == "--help" || command == "-h" {
		return Err(String::new());
	}
	let mut o = Opts {
		command,
		adapter: None,
		size: (1920, 1080),
		seed: 1,
		settings: Settings::defaults(SaverKind::Twilight),
		times: vec![12.0],
		out: None,
		frames: 60,
		size_given: false,
		stack: false,
	};
	while let Some(a) = args.next() {
		let mut val = || args.next().ok_or(format!("{a} needs a value"));
		match a.as_str() {
			"--adapter" => o.adapter = Some(val()?),
			"--size" => {
				let v = val()?;
				let (w, h) = v.split_once('x').ok_or("size is WxH")?;
				o.size = (w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?);
				o.size_given = true;
			}
			"--seed" => o.seed = val()?.parse().map_err(|_| "bad seed")?,
			"--set" => {
				let v = val()?;
				let (name, value) = v.split_once('=').ok_or("--set NAME=VALUE")?;
				let field = fields(SaverKind::Twilight)
					.find(|f| f.name == name)
					.ok_or(format!("Twilight has no setting {name}"))?;
				let value: u32 = value.parse().map_err(|_| format!("bad value in {v}"))?;
				o.settings.set(field.key, value);
			}
			"--at" => {
				o.times = val()?
					.split(',')
					.map(|s| s.parse().map_err(|_| "bad --at"))
					.collect::<Result<_, _>>()?;
			}
			"--out" => o.out = Some(PathBuf::from(val()?)),
			"--frames" => o.frames = val()?.parse().map_err(|_| "bad --frames")?,
			"--stack" => o.stack = true,
			"--help" | "-h" => return Err(String::new()),
			_ => return Err(format!("unknown option {a}")),
		}
	}
	Ok(o)
}

/// With `cpu_maps` the maps are the CPU scene's, to compare with it; without, the graphics card's own size.
fn renderer(gpu: &Gpu, o: &Opts, size: (usize, usize), cpu_maps: bool) -> TwilightRenderer {
	let (rw, rh) = render_size(&o.settings, size.0, size.1, false);
	let out = output_size(o.settings.detail(), size, (rw, rh));
	let shared = if cpu_maps {
		Shared::new(&o.settings, o.seed, rw, rh)
	} else {
		shared_for_gpu(&o.settings, o.seed, (rw, rh))
	};
	TwilightRenderer::new(gpu, &o.settings, shared, o.seed, (rw, rh), out, false)
}

fn write_png(path: &Path, frame: &Frame) -> Result<(), String> {
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
	}
	png::write(path, frame.w, frame.h, &frame.px).map_err(|e| format!("{}: {e}", path.display()))
}

fn frames(gpu: &Gpu, o: &Opts) -> Result<(), String> {
	let out = o.out.clone().unwrap_or_else(|| PathBuf::from("previews/twilight-gpu"));
	let mut renderer = renderer(gpu, o, o.size, false);
	for (n, &t) in o.times.iter().enumerate() {
		renderer.render(gpu, t, t / o.settings.speed() as f64);
		let frame = renderer.read_back(gpu);
		write_png(&out.join(format!("frame-{n:05}.png")), &frame)?;
	}
	eprintln!("frames: {} written to {}", o.times.len(), out.display());
	Ok(())
}

/// CPU on the left, GPU on the right, side by side.
fn side_by_side(left: &Frame, right: &Frame) -> Frame {
	let mut both = Frame::new(left.w + right.w, left.h);
	for y in 0..left.h {
		both.px[y * both.w..y * both.w + left.w].copy_from_slice(&left.px[y * left.w..(y + 1) * left.w]);
		both.px[y * both.w + left.w..(y + 1) * both.w].copy_from_slice(&right.px[y * right.w..(y + 1) * right.w]);
	}
	both
}

fn compare(gpu: &Gpu, o: &Opts) -> Result<(), String> {
	let settings = &o.settings;
	let cpu = cpu_frames(settings, o.seed, o.size, &o.times);
	let mut renderer = renderer(gpu, o, o.size, true);
	let mut rows = Vec::new();
	for (i, &t) in o.times.iter().enumerate() {
		renderer.render(gpu, t, t / settings.speed() as f64);
		let on_gpu = renderer.read_back(gpu);
		eprintln!("t {t:>5}: {}", difference(&cpu[i], &on_gpu).describe());
		rows.push(side_by_side(&cpu[i], &on_gpu));
	}
	let out = o.out.clone().unwrap_or_else(|| PathBuf::from("gpu-vs-cpu.png"));
	if o.stack || rows.len() == 1 {
		let mut all = Frame::new(rows[0].w, rows[0].h * rows.len());
		for (i, row) in rows.iter().enumerate() {
			all.px[i * row.px.len()..(i + 1) * row.px.len()].copy_from_slice(&row.px);
		}
		write_png(&out, &all)?;
	} else {
		for (i, row) in rows.iter().enumerate() {
			write_png(&out.join(format!("compare-{i:02}.png")), row)?;
		}
	}
	Ok(())
}

fn bench(gpu: &Gpu, o: &Opts) -> Result<(), String> {
	let sizes = if o.size_given {
		vec![o.size]
	} else {
		vec![(1920, 1080), (3840, 2160)]
	};
	let mut settings_for_bench = o.settings.clone();
	settings_for_bench.set(Key::RenderScale, 100);
	println!("adapter: {}", gpu.describe());
	println!(
		"{:<10} {:>6} {:>14} {:>14} {:>14}",
		"size", "detail", "pipelined ms", "one at a time", "setup s"
	);
	for size in sizes {
		let started = Instant::now();
		let bench_opts = Opts {
			settings: settings_for_bench.clone(),
			..o.clone()
		};
		let mut renderer = renderer(gpu, &bench_opts, size, false);
		let setup = started.elapsed().as_secs_f64();
		// Warm up: the first frames compile and upload.
		for i in 0..3 {
			renderer.render(gpu, 10.0 + i as f64 * 0.016, 100.0);
		}
		gpu.device
			.poll(wgpu::PollType::wait_indefinitely())
			.map_err(|e| e.to_string())?;
		// Pipelined: submit every frame, wait once.
		let started = Instant::now();
		for i in 0..o.frames {
			renderer.render(gpu, 12.0 + i as f64 / 60.0, 100.0);
		}
		gpu.device
			.poll(wgpu::PollType::wait_indefinitely())
			.map_err(|e| e.to_string())?;
		let pipelined = started.elapsed().as_secs_f64() * 1000.0 / o.frames as f64;
		// One at a time: submit and wait, the latency of a frame.
		let mut times = Vec::new();
		for i in 0..o.frames.min(30) {
			let started = Instant::now();
			renderer.render(gpu, 40.0 + i as f64 / 60.0, 100.0);
			gpu.device
				.poll(wgpu::PollType::wait_indefinitely())
				.map_err(|e| e.to_string())?;
			times.push(started.elapsed().as_secs_f64() * 1000.0);
		}
		times.sort_by(|a, b| a.partial_cmp(b).unwrap());
		let median = times[times.len() / 2];
		println!(
			"{:<10} {:>6} {:>14.2} {:>14.2} {:>14.1}",
			format!("{}x{}", size.0, size.1),
			settings_for_bench.get(Key::Detail),
			pipelined,
			median,
			setup
		);
	}
	Ok(())
}

fn main() -> ExitCode {
	let o = match parse() {
		Ok(o) => o,
		Err(e) => {
			if !e.is_empty() {
				eprintln!("savers-gpu-preview: {e}\n");
			}
			eprint!("{USAGE}");
			return if e.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(2) };
		}
	};
	if o.command == "adapters" {
		for name in adapter_names() {
			println!("{name}");
		}
		return ExitCode::SUCCESS;
	}
	let gpu = match Gpu::new(o.adapter.as_deref()) {
		Ok(gpu) => gpu,
		Err(e) => {
			eprintln!("savers-gpu-preview: {e}");
			return ExitCode::FAILURE;
		}
	};
	eprintln!("adapter: {}", gpu.describe());
	let result = match o.command.as_str() {
		"frames" => frames(&gpu, &o),
		"compare" => compare(&gpu, &o),
		"bench" => bench(&gpu, &o),
		other => Err(format!("unknown command {other}")),
	};
	match result {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("savers-gpu-preview: {e}");
			ExitCode::FAILURE
		}
	}
}
