// The command line: what to run (the saver, a window, xscreensaver's window, a screenshot, a bench) and the settings over the file.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::PathBuf;

pub const USAGE: &str = "\
usage: afterglow-twilight-gpu [options]

Afterglow Twilight on the graphics card. With no mode option it is the screensaver: borderless and fullscreen at the display's native
size, the cursor hidden, ending on any key, button or 10 pixels of mouse movement.

modes:
  --window WxH        a normal window of that size, for testing (a key press or closing the window ends it)
  --preview           a small 320x180 window with no sound and the Low detail level, as a settings dialog shows it
  --window-id ID      draw into an existing X11 window, as xscreensaver does (also -window-id; ID may be 0x...). Silent unless
                      --sound is given; ignores input; follows xscreensaver's kill
  -root               as --window-id with the id in $XSCREENSAVER_WINDOW (xscreensaver's older way)
  --shot FILE.png     no window: render one frame (at --at seconds, default 40) at --size and write a PNG
  --bench             no window: time frames at 1080p and 4K (or --size) and print them
  --audio-wav FILE    no window, no sound device: render the score for --seconds (default 30) to a 48 kHz WAV file
  --adapters          list the graphics adapters wgpu can use
  --write-config      write the commented settings file if there is none (--force to overwrite) and print its path
  --help, --version

settings (all over the file; the names are the Win32 saver's registry values):
  --set NAME=VALUE    any Twilight setting, e.g. --set Bloom=40 (repeatable); FrameCap is one too
  --palette P         Dusk, Indigo, Ember, Aurora, Graphite (or 0 to 4)
  --speed N           25 to 200 (percent)
  --detail D          Low, Medium, High, Ultra (or 0 to 3)
  --scale N           render scale: 25, 50, 75 or 100 (percent of the display; the frame is enlarged to fit)
  --fps N             frames a second at most; 0 for the display's refresh
  --seed N            the scene seed (0 for a new one each run)
  --sun, --no-sun     show or hide the sun
  --sound, --no-sound play the score (the default is on at low volume; off under xscreensaver) or keep it silent
  --volume N          0 to 100 (default 28)
  --config FILE       read this settings file instead of ~/.config/liminal-hq/afterglow-savers/twilight.toml

other:
  --adapter NAME      the graphics adapter, by part of its name (radv, llvmpipe); default the first discrete one
  --size WxH          --shot and --bench: the frame size (default 1920x1080)
  --at S              --shot: the scene time; elsewhere, freeze the scene at this time (for tests)
  --frames N          end after N frames (for tests and the bench)
  --exit-on-input     in a window, end on mouse movement and clicks too
  --verbose           say what was chosen (adapter, size, audio) and log the frame count
";

/// What the program does.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
	/// The screensaver: fullscreen.
	Fullscreen,
	/// A window to test in.
	Window(u32, u32),
	/// A small window with the Low detail level.
	Preview,
	/// A child of an X11 window someone else owns (xscreensaver's).
	Embed(u32),
	/// `-root`: the same, with the id from the environment.
	EmbedFromEnvironment,
	Shot(PathBuf),
	Bench,
	AudioWav(PathBuf),
	Adapters,
	WriteConfig,
	Help,
	Version,
}

#[derive(Clone, Debug)]
pub struct Options {
	pub mode: Mode,
	/// `--set`, `--palette` and the rest, in order, as (name, value).
	pub overrides: Vec<(String, String)>,
	pub seed: Option<u32>,
	pub sound: Option<bool>,
	pub config: Option<PathBuf>,
	pub adapter: Option<String>,
	pub size: Option<(u32, u32)>,
	pub at: Option<f64>,
	pub frames: Option<u64>,
	pub seconds: Option<f64>,
	pub exit_on_input: bool,
	pub force: bool,
	pub verbose: bool,
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
	let (width, height) = text
		.split_once(['x', 'X'])
		.ok_or_else(|| format!("\"{text}\" is not a size like 1920x1080"))?;
	let width: u32 = width.parse().map_err(|_| format!("bad width in \"{text}\""))?;
	let height: u32 = height.parse().map_err(|_| format!("bad height in \"{text}\""))?;
	if !(16..=16384).contains(&width) || !(9..=16384).contains(&height) {
		return Err(format!("\"{text}\" is outside 16x9 to 16384x16384"));
	}
	Ok((width, height))
}

/// xscreensaver gives an id as `0x1c00003` or as decimal.
fn parse_window_id(text: &str) -> Result<u32, String> {
	let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
		Some(hex) => u32::from_str_radix(hex, 16),
		None => text.parse(),
	};
	parsed.map_err(|_| format!("\"{text}\" is not a window id"))
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
	let mut options = Options {
		mode: Mode::Fullscreen,
		overrides: Vec::new(),
		seed: None,
		sound: None,
		config: None,
		adapter: None,
		size: None,
		at: None,
		frames: None,
		seconds: None,
		exit_on_input: false,
		force: false,
		verbose: false,
	};
	let mut args = args.into_iter();
	while let Some(arg) = args.next() {
		let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
		match arg.as_str() {
			"--window" => {
				let (w, h) = parse_size(&value("--window")?)?;
				options.mode = Mode::Window(w, h);
			}
			"--preview" => options.mode = Mode::Preview,
			"--window-id" | "-window-id" => options.mode = Mode::Embed(parse_window_id(&value("--window-id")?)?),
			"-root" | "--root" => options.mode = Mode::EmbedFromEnvironment,
			"--fullscreen" => options.mode = Mode::Fullscreen,
			"--shot" => options.mode = Mode::Shot(PathBuf::from(value("--shot")?)),
			"--bench" => options.mode = Mode::Bench,
			"--audio-wav" => options.mode = Mode::AudioWav(PathBuf::from(value("--audio-wav")?)),
			"--adapters" => options.mode = Mode::Adapters,
			"--write-config" => options.mode = Mode::WriteConfig,
			"--force" => options.force = true,
			"--help" | "-h" => options.mode = Mode::Help,
			"--version" | "-V" => options.mode = Mode::Version,
			"--set" => {
				let pair = value("--set")?;
				let (name, text) = pair.split_once('=').ok_or("--set takes NAME=VALUE")?;
				options.overrides.push((name.to_string(), text.to_string()));
			}
			"--palette" => options.overrides.push(("Palette".into(), value("--palette")?)),
			"--speed" => options.overrides.push(("Speed".into(), value("--speed")?)),
			"--detail" => options.overrides.push(("Detail".into(), value("--detail")?)),
			"--scale" | "--render-scale" => options.overrides.push(("RenderScale".into(), value("--scale")?)),
			"--fps" => options.overrides.push(("FrameCap".into(), value("--fps")?)),
			"--volume" => options.overrides.push(("Volume".into(), value("--volume")?)),
			"--sun" => options.overrides.push(("Sun".into(), "1".into())),
			"--no-sun" => options.overrides.push(("Sun".into(), "0".into())),
			"--seed" => options.seed = Some(value("--seed")?.parse().map_err(|_| "--seed takes a whole number")?),
			"--sound" => options.sound = Some(true),
			"--no-sound" => options.sound = Some(false),
			"--config" => options.config = Some(PathBuf::from(value("--config")?)),
			"--adapter" => options.adapter = Some(value("--adapter")?),
			"--size" => options.size = Some(parse_size(&value("--size")?)?),
			"--at" => options.at = Some(value("--at")?.parse().map_err(|_| "--at takes seconds")?),
			"--frames" => options.frames = Some(value("--frames")?.parse().map_err(|_| "--frames takes a whole number")?),
			"--seconds" => options.seconds = Some(value("--seconds")?.parse().map_err(|_| "--seconds takes a number")?),
			"--exit-on-input" => options.exit_on_input = true,
			"--verbose" | "-v" => options.verbose = true,
			other => return Err(format!("unknown option {other}")),
		}
	}
	Ok(options)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse_line(line: &str) -> Result<Options, String> {
		parse(line.split_whitespace().map(str::to_string))
	}

	#[test]
	fn no_arguments_is_the_fullscreen_saver() {
		assert_eq!(parse_line("").unwrap().mode, Mode::Fullscreen);
	}

	#[test]
	fn xscreensaver_styles_are_understood() {
		assert_eq!(parse_line("-window-id 0x1c00003").unwrap().mode, Mode::Embed(0x1c00003));
		assert_eq!(parse_line("--window-id 4096").unwrap().mode, Mode::Embed(4096));
		assert_eq!(parse_line("-root").unwrap().mode, Mode::EmbedFromEnvironment);
	}

	#[test]
	fn settings_become_overrides_in_order() {
		let options = parse_line("--palette Aurora --scale 50 --set Bloom=40 --no-sun --fps 0 --seed 7 --sound").unwrap();
		assert_eq!(options.overrides[0], ("Palette".into(), "Aurora".into()));
		assert_eq!(options.overrides[1], ("RenderScale".into(), "50".into()));
		assert_eq!(options.overrides[2], ("Bloom".into(), "40".into()));
		assert_eq!(options.overrides[3], ("Sun".into(), "0".into()));
		assert_eq!(options.overrides[4], ("FrameCap".into(), "0".into()));
		assert_eq!(options.seed, Some(7));
		assert_eq!(options.sound, Some(true));
	}

	#[test]
	fn bad_input_is_an_error_not_a_panic() {
		assert!(parse_line("--window 100").is_err());
		assert!(parse_line("--window 4x4").is_err());
		assert!(parse_line("--frobnicate").is_err());
		assert!(parse_line("--set Bloom").is_err());
		assert!(parse_line("--window-id zz").is_err());
		assert!(parse_line("--seed").is_err());
	}

	#[test]
	fn window_and_shot_modes() {
		assert_eq!(parse_line("--window 640x360").unwrap().mode, Mode::Window(640, 360));
		let shot = parse_line("--shot a.png --at 12 --size 800x450").unwrap();
		assert_eq!(shot.mode, Mode::Shot("a.png".into()));
		assert_eq!(shot.at, Some(12.0));
		assert_eq!(shot.size, Some((800, 450)));
	}
}
