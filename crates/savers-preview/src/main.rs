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
  audio    render the scene's score alone to a WAV file (--out FILE.wav), from --from for --seconds

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
  --loop              frames: render exactly one loop of a scene that repeats (Maze with LoopMaze=1), from --from
  --seconds S         audio: how long to render (default 60); with --loop, one loop of the scene
  --rate N            audio: sample rate (default 48000)
  --volume N          audio: level in percent (default 100; the savers' own default is 28)
  --pipe              frames: write raw RGB24 frames to standard output instead of PNG files (for ffmpeg -f rawvideo)
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
	pipe: bool,
	one_loop: bool,
	seconds: f64,
	rate: u32,
	volume: u32,
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
		pipe: false,
		one_loop: false,
		seconds: 60.0,
		rate: savers_core::audio::FULL_RATE,
		volume: 100,
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
			"--pipe" => o.pipe = true,
			"--loop" => o.one_loop = true,
			"--seconds" => o.seconds = val()?.parse().map_err(|_| "bad --seconds")?,
			"--rate" => o.rate = val()?.parse().map_err(|_| "bad --rate")?,
			"--volume" => o.volume = val()?.parse().map_err(|_| "bad --volume")?,
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
	if !o.pipe {
		std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
	}
	let stdout = std::io::stdout();
	let mut pipe = std::io::BufWriter::with_capacity(1 << 20, stdout.lock());
	let mut rgb = Vec::new();
	let mut saver = make(o.kind, &settings, o.seed, o.size);
	let dt = 1.0 / o.fps;
	let mut t = 0.0f64;
	let mut next = o.from;
	let mut to = o.to;
	if o.one_loop {
		let length = saver
			.loop_length()
			.ok_or("--loop: this scene and these settings do not repeat")?;
		// Frames from `from` up to, not including, `from` plus one loop, so the file plays round without a doubled frame.
		to = o.from + length - o.step * 0.5;
		eprintln!("frames: one loop of {length:.2} s");
	}
	let mut n = 0;
	let started = Instant::now();
	while t <= to + 1e-9 {
		let frame = saver.at(t);
		if t + 1e-9 >= next {
			if o.pipe {
				rgb.clear();
				for &c in &frame.px {
					rgb.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
				}
				pipe.write_all(&rgb).map_err(|e| format!("standard output: {e}"))?;
			} else {
				let path = out.join(format!("frame-{n:05}.png"));
				png::write(&path, frame.w, frame.h, &frame.px).map_err(|e| format!("{}: {e}", path.display()))?;
			}
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

/// Renders the score alone: the scene is made at a tiny size only for its timeline (its cues), and nothing is drawn. With `--loop`,
/// one loop is rendered first and thrown away, so the file starts with the tail the loop's end leaves and plays round seamlessly.
fn audio(o: &Opts) -> Result<(), String> {
	let settings = Settings::load(o.kind, &o.store);
	let mut scene = build(o.kind, &settings, o.seed, 32, 18);
	let score = savers_core::audio::score_for(o.kind, &settings, o.seed, o.rate).ok_or("this scene has no score yet")?;
	let (seconds, warm) = if o.one_loop {
		let length = match (scene.loop_length(), o.kind) {
			(Some(length), _) => length,
			(None, SaverKind::Twilight) => savers_core::twilight_shared::total_length() as f64,
			_ => return Err("--loop: this scene and these settings do not repeat".into()),
		};
		(length, length)
	} else {
		(o.seconds, 0.0)
	};
	// A render from the very start fades up over the saver's first second, as the picture does.
	let fade_in = if o.from < 1.0 && !o.one_loop {
		(1.0 - o.from) as f32
	} else {
		0.0
	};
	let mut engine = savers_core::audio::Engine::new(o.rate, score, o.volume as f32 / 100.0, fade_in);
	let from = o.from;
	let mut cues = |t: f64| scene.cues(from + t);
	let started = Instant::now();
	let mut scratch = vec![0i16; 2 * 4096];
	let mut left = (warm * o.rate as f64) as usize;
	while left > 0 {
		let n = left.min(4096);
		engine.render(&mut scratch[..n * 2], &mut cues);
		left -= n;
	}
	let total = (seconds * o.rate as f64).round() as usize;
	// A loop also renders half a second past its end and cross-fades that into its start: the sound one loop on is the same, but
	// oscillator phases are not, and the fade hides the difference.
	let overlap = if o.one_loop { (o.rate as usize / 2).min(total) } else { 0 };
	let mut samples = vec![0i16; (total + overlap) * 2];
	engine.render(&mut samples, &mut cues);
	for i in 0..overlap {
		let k = i as f32 / overlap as f32;
		for c in 0..2 {
			let head = samples[i * 2 + c] as f32;
			let tail = samples[(total + i) * 2 + c] as f32;
			samples[i * 2 + c] = (head * k + tail * (1.0 - k)) as i16;
		}
	}
	samples.truncate(total * 2);
	let out = o
		.out
		.clone()
		.unwrap_or_else(|| PathBuf::from(format!("{}.wav", o.kind.registry_name().to_lowercase())));
	write_wav(&out, o.rate, &samples).map_err(|e| format!("{}: {e}", out.display()))?;
	let peak = samples.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0);
	let rms = (samples.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>() / samples.len().max(1) as f64).sqrt();
	eprintln!(
		"audio: {:.1} s at {} Hz to {} in {:.1} s; peak {:.1} dBFS, RMS {:.1} dBFS",
		seconds,
		o.rate,
		out.display(),
		started.elapsed().as_secs_f64(),
		20.0 * (peak.max(1) as f64 / 32_768.0).log10(),
		20.0 * (rms.max(1.0) / 32_768.0).log10()
	);
	Ok(())
}

/// A 16-bit stereo PCM WAV file.
fn write_wav(path: &Path, rate: u32, samples: &[i16]) -> std::io::Result<()> {
	let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
	let data = (samples.len() * 2) as u32;
	f.write_all(b"RIFF")?;
	f.write_all(&(36 + data).to_le_bytes())?;
	f.write_all(b"WAVEfmt ")?;
	f.write_all(&16u32.to_le_bytes())?;
	f.write_all(&1u16.to_le_bytes())?;
	f.write_all(&2u16.to_le_bytes())?;
	f.write_all(&rate.to_le_bytes())?;
	f.write_all(&(rate * 4).to_le_bytes())?;
	f.write_all(&4u16.to_le_bytes())?;
	f.write_all(&16u16.to_le_bytes())?;
	f.write_all(b"data")?;
	f.write_all(&data.to_le_bytes())?;
	for s in samples {
		f.write_all(&s.to_le_bytes())?;
	}
	f.flush()
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
		"audio" => audio(&o),
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
