// Renders the savers on the host: PNG frames for looking and for video, and frame times for every rung of the quality ladder.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use savers_core::bands::Bands;
use savers_core::banner::{self, Shot};
use savers_core::bench::{P3_REFERENCE_MS, reference};
use savers_core::palette::{Palette, PaletteId};
use savers_core::saver::{Saver, render_size};
use savers_core::scenes::build;
use savers_core::settings::{Control, Key, SaverKind, Settings, SettingsStore, fields};
use savers_core::swatch;

mod png;
mod pool;

const USAGE: &str = "\
usage: savers-preview <command> [options]

commands:
  frames   render frames to PNG files: OUT/frame-00000.png and on
  bench    time every render scale and detail level, with a Pentium III estimate
  threads  time 100% Ultra at --size with each count of --counts threads (default 1,2,4,8), for the scene or --scene all
  audio    render the scene's score alone to a WAV file (--out FILE.wav), from --from for --seconds
  banner   render the settings dialog's banner as the dialog does (--out FILE.png, --size default 564x65); prints the title's colours
  swatches render the dialog's palette swatches, by night and by day, plain and chosen, into --out DIR (--size default 50x49);
           prints each file with the box its name goes in

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
  --threads N         share the heavy passes between N threads (default 1); the frames are the same byte for byte
  --counts LIST      threads: the thread counts to time, comma separated (default 1,2,4,8)
  --pipe              frames: write raw RGB24 frames to standard output instead of PNG files (for ffmpeg -f rawvideo)
  --shot T,A,C,L[,W,X] banner: try a moment T, render aspect A, crop centre C, lead L, render width W times the banner's and
                      crop centre across X (with --seed) instead of the scene's own
  --face RRGGBB       swatches: the dialog face behind them (default c0c0c0, Windows 95's)
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
	threads: usize,
	counts: Vec<usize>,
	all: bool,
	shot: Option<Shot>,
	face: u32,
	size_given: bool,
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
		threads: 1,
		counts: vec![1, 2, 4, 8],
		all: false,
		shot: None,
		face: 0xC0C0C0,
		size_given: false,
	};
	while let Some(a) = args.next() {
		let mut val = || args.next().ok_or(format!("{a} needs a value"));
		match a.as_str() {
			"--scene" => {
				let v = val()?;
				if v == "all" {
					o.all = true;
				} else {
					o.kind = SaverKind::from_name(&v).ok_or(format!("unknown scene {v}"))?;
				}
			}
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
			"--threads" => o.threads = val()?.parse().map_err(|_| "bad --threads")?,
			"--counts" => {
				o.counts = val()?
					.split(',')
					.map(|n| n.trim().parse().map_err(|_| "bad --counts".to_string()))
					.collect::<Result<_, _>>()?;
			}
			"--shot" => {
				let v = val()?;
				let parts: Vec<f32> = v.split(',').filter_map(|p| p.parse().ok()).collect();
				let (t, aspect, centre, lead, wide, across) = match parts[..] {
					[t, aspect, centre, lead] => (t, aspect, centre, lead, 1.0, 0.5),
					[t, aspect, centre, lead, wide, across] => (t, aspect, centre, lead, wide, across),
					_ => return Err("--shot T,ASPECT,CENTRE,LEAD[,WIDE,ACROSS]".into()),
				};
				o.shot = Some(Shot {
					t,
					wide,
					aspect,
					centre,
					across,
					lead: lead as u16,
					seed: 0,
				});
			}
			"--face" => o.face = u32::from_str_radix(&val()?, 16).map_err(|_| "--face RRGGBB")?,
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

/// The bands for `threads` threads: a pool kept for the rest of the run, or none for one.
fn bands_for(threads: usize) -> Option<&'static dyn Bands> {
	if threads <= 1 {
		return None;
	}
	let pool: &'static pool::Pool = Box::leak(Box::new(pool::Pool::new(threads)));
	Some(pool)
}

fn make(kind: SaverKind, settings: &Settings, seed: u32, size: (usize, usize), bands: Option<&'static dyn Bands>) -> Saver {
	let (rw, rh) = render_size(settings, size.0, size.1, false);
	let scene = build(kind, settings, seed, rw, rh);
	let mut saver = Saver::new(settings.clone(), scene, size.0, size.1, false);
	if let Some(b) = bands {
		saver.set_bands(b);
	}
	saver
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
	let mut saver = make(o.kind, &settings, o.seed, o.size, bands_for(o.threads));
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
	let bands = bands_for(o.threads);
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
			let mut saver = make(o.kind, &settings, o.seed, o.size, bands);
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

/// Times 100% Ultra at the window size for each thread count: frame time and frames a second, the mean of `--frames` frames a
/// second apart in scene time.
fn thread_bench(o: &Opts) -> Result<(), String> {
	let kinds: Vec<SaverKind> = if o.all {
		vec![
			SaverKind::Cracktro,
			SaverKind::Ghostwriter,
			SaverKind::Maze,
			SaverKind::Twilight,
			SaverKind::Ink,
		]
	} else {
		vec![o.kind]
	};
	let mut stdout = std::io::stdout();
	let _ = writeln!(stdout, "100% Ultra at {}x{}, {} frames each", o.size.0, o.size.1, o.frames);
	let _ = writeln!(stdout, "| Saver | Threads | ms per frame | fps |");
	let _ = writeln!(stdout, "| --- | --- | --- | --- |");
	for kind in kinds {
		let mut store = MapStore {
			dwords: o.store.dwords.clone(),
			texts: o.store.texts.clone(),
		};
		store.set_dword("RenderScale", 100);
		store.set_dword("Detail", 3);
		let settings = Settings::load(kind, &store);
		for &n in &o.counts {
			let mut saver = make(kind, &settings, o.seed, o.size, bands_for(n));
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
			let _ = writeln!(stdout, "| {} | {n} | {ms:.1} | {:.0} |", kind.title(), 1000.0 / ms);
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

/// The settings dialog's banner, as `dialog_v2` renders it before drawing the icon and the title over it.
fn banner_png(o: &Opts) -> Result<(), String> {
	let settings = Settings::load(o.kind, &o.store);
	let size = if o.size_given { o.size } else { (564, 65) };
	let kind = o.kind;
	let make = |s: &Settings, seed: u32, w: usize, h: usize| build(kind, s, seed, w, h);
	let started = Instant::now();
	let trial = o.shot.map(|shot| Shot { seed: o.seed, ..shot });
	let made = banner::render_shot(&settings, size.0, size.1, &make, trial);
	let ms = started.elapsed().as_secs_f64() * 1000.0;
	let out = o
		.out
		.clone()
		.unwrap_or_else(|| PathBuf::from(format!("banner-{}.png", kind.registry_name().to_lowercase())));
	png::write(&out, made.width, made.height, &made.pixels).map_err(|e| format!("{}: {e}", out.display()))?;
	println!(
		"{} title={:06x} subtitle={:06x} shadow={:06x} ms={ms:.0}",
		out.display(),
		made.inks.title,
		made.inks.subtitle,
		made.inks.shadow
	);
	Ok(())
}

/// Every palette swatch of the scene's palette field, by night and by day, plain and chosen, one PNG each.
fn swatches(o: &Opts) -> Result<(), String> {
	let size = if o.size_given { o.size } else { (50, 49) };
	let out = o.out.clone().unwrap_or_else(|| PathBuf::from("swatches"));
	std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
	let options = fields(o.kind)
		.find(|f| f.key == Key::Palette)
		.and_then(|f| match f.control {
			Control::Choice(options) => Some(options),
			_ => None,
		})
		.ok_or("this saver has no palette field")?;
	for &(name, value) in options {
		for light in [false, true] {
			for chosen in [false, true] {
				let palette = Palette::new(PaletteId::from_u32(value), light);
				let mut pixels = vec![0u32; size.0 * size.1];
				let label = swatch::draw(&mut pixels, size.0, size.1, &palette, o.face, 0, chosen);
				let file = out.join(format!(
					"swatch-{}-{}{}.png",
					swatch::short_name(name).to_lowercase(),
					if light { "day" } else { "night" },
					if chosen { "-chosen" } else { "" }
				));
				png::write(&file, size.0, size.1, &pixels).map_err(|e| format!("{}: {e}", file.display()))?;
				println!(
					"{} {} {} {} {} {}",
					file.display(),
					label.label_left,
					label.label_top,
					label.label_right,
					label.label_bottom,
					swatch::short_name(name)
				);
			}
		}
	}
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
		"threads" => thread_bench(&o),
		"audio" => audio(&o),
		"banner" => banner_png(&o),
		"swatches" => swatches(&o),
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
