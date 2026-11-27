// Renders the savers on the host: PNG frames for looking and for video, and frame times for every rung of the quality ladder.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use savers_core::bench::{P3_REFERENCE_MS, reference};
use savers_core::saver::{Saver, render_size};
use savers_core::scenes::build;
use savers_core::settings::{Key, SaverKind, Settings, SettingsStore, fields};

mod png;

const USAGE: &str = "\
usage: savers-preview <command> [options]

commands:
  frames   render frames to PNG files: OUT/frame-00000.png and on
  bench    time every render scale and detail level, with a Pentium III estimate

options:
  --scene NAME        maze, cracktro, twilight, ink, collection, ghostwriter (default ink)
  --size WxH          the window size (default 640x360)
  --seed N            the scene seed (default 1)
  --set NAME=VALUE    any registry value of the saver, e.g. --set Detail=3 --set RenderScale=100 (repeatable)
  --text TEXT         the saver's text setting (the cracktro's message)
  --from S --to S     the first and last time written, in seconds (default 0 and 10)
  --step S            seconds between written frames (default 1)
  --fps N             simulation steps per second (default 30)
  --out DIR           where frames go (default previews/<scene>)
  --frames N          frames timed per rung in bench (default 60)
";

#[derive(Default)]
struct MapStore {
	dwords: BTreeMap<String, u32>,
	texts: BTreeMap<String, String>,
}

impl SettingsStore for MapStore {
	fn get_dword(&self, name: &str) -> Option<u32> {
		self.dwords.get(name).copied()
	}
	fn get_text(&self, name: &str) -> Option<String> {
		self.texts.get(name).cloned()
	}
	fn set_dword(&mut self, name: &str, value: u32) {
		self.dwords.insert(name.to_string(), value);
	}
	fn set_text(&mut self, name: &str, value: &str) {
		self.texts.insert(name.to_string(), value.to_string());
	}
}

struct Opts {
	command: String,
	kind: SaverKind,
	size: (usize, usize),
	seed: u32,
	store: MapStore,
	from: f64,
	to: f64,
	step: f64,
	fps: f64,
	out: Option<PathBuf>,
	frames: usize,
}

fn parse() -> Result<Opts, String> {
	let mut args = std::env::args().skip(1);
	let command = args.next().ok_or("no command")?;
	if command == "--help" || command == "-h" {
		return Err(String::new());
	}
	let mut o = Opts {
		command,
		kind: SaverKind::Ink,
		size: (640, 360),
		seed: 1,
		store: MapStore::default(),
		from: 0.0,
		to: 10.0,
		step: 1.0,
		fps: 30.0,
		out: None,
		frames: 60,
	};
	while let Some(a) = args.next() {
		let mut val = || args.next().ok_or(format!("{a} needs a value"));
		match a.as_str() {
			"--scene" => {
				let v = val()?;
				o.kind = SaverKind::from_name(&v).ok_or(format!("unknown scene {v}"))?;
			}
			"--size" => {
				let v = val()?;
				let (w, h) = v.split_once('x').ok_or("size is WxH")?;
				o.size = (w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?);
			}
			"--seed" => o.seed = val()?.parse().map_err(|_| "bad seed")?,
			"--set" => {
				let v = val()?;
				let (name, value) = v.split_once('=').ok_or("--set NAME=VALUE")?;
				o.store
					.set_dword(name, value.parse().map_err(|_| format!("bad value in {v}"))?);
			}
			"--text" => {
				let v = val()?;
				o.store.texts.insert("Message".into(), v.clone());
				o.store.texts.insert("TextBank".into(), v);
			}
			"--from" => o.from = val()?.parse().map_err(|_| "bad --from")?,
			"--to" => o.to = val()?.parse().map_err(|_| "bad --to")?,
			"--step" => o.step = val()?.parse().map_err(|_| "bad --step")?,
			"--fps" => o.fps = val()?.parse().map_err(|_| "bad --fps")?,
			"--out" => o.out = Some(PathBuf::from(val()?)),
			"--frames" => o.frames = val()?.parse().map_err(|_| "bad --frames")?,
			"--help" | "-h" => return Err(String::new()),
			_ => return Err(format!("unknown option {a}")),
		}
	}
	for name in o.store.dwords.keys() {
		if !fields(o.kind).any(|f| f.name == name) {
			return Err(format!("{} has no setting {name}", o.kind.registry_name()));
		}
	}
	Ok(o)
}

fn make(kind: SaverKind, settings: &Settings, seed: u32, size: (usize, usize)) -> Saver {
	let (rw, rh) = render_size(settings, size.0, size.1, false);
	let scene = build(kind, settings, seed, rw, rh);
	Saver::new(settings.clone(), scene, size.0, size.1, false)
}

fn frames(o: &Opts) -> Result<(), String> {
	let settings = Settings::load(o.kind, &o.store);
	let out = o
		.out
		.clone()
		.unwrap_or_else(|| Path::new("previews").join(o.kind.registry_name().to_lowercase()));
	std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
	let mut saver = make(o.kind, &settings, o.seed, o.size);
	let dt = 1.0 / o.fps;
	let mut t = 0.0f64;
	let mut next = o.from;
	let mut n = 0;
	let started = Instant::now();
	while t <= o.to + 1e-9 {
		let frame = saver.at(t);
		if t + 1e-9 >= next {
			let path = out.join(format!("frame-{n:05}.png"));
			png::write(&path, frame.w, frame.h, &frame.px).map_err(|e| format!("{}: {e}", path.display()))?;
			n += 1;
			next += o.step;
		}
		t += dt;
	}
	eprintln!(
		"frames: {n} written to {} in {:.1} s",
		out.display(),
		started.elapsed().as_secs_f64()
	);
	Ok(())
}

fn bench(o: &Opts) -> Result<(), String> {
	// The reference loop, best of five, gives the host-to-Pentium III ratio.
	let mut best = f64::MAX;
	for _ in 0..5 {
		let s = Instant::now();
		std::hint::black_box(reference());
		best = best.min(s.elapsed().as_secs_f64() * 1000.0);
	}
	let ratio = P3_REFERENCE_MS / best;
	let mut stdout = std::io::stdout();
	let _ = writeln!(
		stdout,
		"{} at {}x{}: reference loop {best:.2} ms on this host, a Pentium III 800 MHz is taken as {ratio:.1}x slower",
		o.kind.title(),
		o.size.0,
		o.size.1
	);
	let _ = writeln!(
		stdout,
		"| Scale | Detail | Render | ms per frame (host) | fps (host) | ms on a P3 (est.) | fps on a P3 (est.) |"
	);
	let _ = writeln!(stdout, "| --- | --- | --- | --- | --- | --- | --- |");
	for scale in [100u32, 75, 50, 25] {
		for detail in 0..4u32 {
			let mut store = MapStore {
				dwords: o.store.dwords.clone(),
				texts: o.store.texts.clone(),
			};
			store.set_dword("RenderScale", scale);
			store.set_dword("Detail", detail);
			let settings = Settings::load(o.kind, &store);
			let made = Instant::now();
			let mut saver = make(o.kind, &settings, o.seed, o.size);
			let start_ms = made.elapsed().as_secs_f64() * 1000.0;
			let (rw, rh) = render_size(&settings, o.size.0, o.size.1, false);
			// Frames are taken a second apart (each simulated as a 0.1 s step), so the timing covers the storyboard's shots and
			// phases rather than its first moments.
			let mut t = 0.0;
			for _ in 0..3 {
				saver.at(t);
				t += 1.0;
			}
			let s = Instant::now();
			for _ in 0..o.frames {
				std::hint::black_box(saver.at(t));
				t += 1.0;
			}
			let ms = s.elapsed().as_secs_f64() * 1000.0 / o.frames as f64;
			let p3 = ms * ratio;
			let names = ["Low", "Medium", "High", "Ultra"];
			let _ = writeln!(
				stdout,
				"| {scale}% | {} | {rw}x{rh} | {start_ms:.0} | {ms:.2} | {:.0} | {p3:.0} | {:.1} |",
				names[detail as usize],
				1000.0 / ms,
				1000.0 / p3
			);
			let _ = settings.get(Key::Detail);
		}
	}
	Ok(())
}

fn main() -> ExitCode {
	let o = match parse() {
		Ok(o) => o,
		Err(e) => {
			if !e.is_empty() {
				eprintln!("savers-preview: {e}");
			}
			eprint!("{USAGE}");
			return ExitCode::from(2);
		}
	};
	let r = match o.command.as_str() {
		"frames" => frames(&o),
		"bench" => bench(&o),
		other => Err(format!("unknown command {other}")),
	};
	match r {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("savers-preview: {e}");
			ExitCode::FAILURE
		}
	}
}
