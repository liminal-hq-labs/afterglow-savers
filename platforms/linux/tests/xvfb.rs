// Runs the binary on its own Xvfb with the lavapipe software Vulkan driver: the window, fullscreen at the screen's size, the exit
// rules, xscreensaver's window, and sound through ALSA's file plug-in. Never touches the desktop session: every command gets its own
// DISPLAY and no WAYLAND_DISPLAY, and sound goes to a file. `make check-gpu` runs these (they are `#[ignore]` for `make check`).

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use savers_core::frame::Frame;
use savers_core::settings::{Key, SaverKind, Settings};
use savers_gpu::compare::{cpu_frames, difference};

const BINARY: &str = env!("CARGO_BIN_EXE_afterglow-twilight-gpu");
const LAVAPIPE: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";

fn tool_exists(name: &str) -> bool {
	Command::new("sh")
		.args(["-c", &format!("command -v {name}")])
		.stdout(Stdio::null())
		.status()
		.is_ok_and(|status| status.success())
}

/// A private X server. Killed (by its own PID) when dropped.
struct Xvfb {
	child: Child,
	display: String,
}

impl Xvfb {
	/// `None`, with a note on stderr, when the tools the test needs are missing.
	fn start(width: u32, height: u32) -> Option<Xvfb> {
		for tool in ["Xvfb", "import", "xdotool"] {
			if !tool_exists(tool) {
				eprintln!("SKIP: {tool} is not installed");
				return None;
			}
		}
		if !Path::new(LAVAPIPE).exists() {
			eprintln!("SKIP: no lavapipe at {LAVAPIPE}");
			return None;
		}
		let mut child = Command::new("Xvfb")
			.args([
				"-displayfd",
				"1",
				"-nolisten",
				"tcp",
				"-screen",
				"0",
				&format!("{width}x{height}x24"),
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()
			.expect("Xvfb starts");
		let mut line = String::new();
		BufReader::new(child.stdout.take().unwrap())
			.read_line(&mut line)
			.expect("Xvfb reports its display");
		Some(Xvfb {
			child,
			display: format!(":{}", line.trim()),
		})
	}

	/// A command for this server only: its display, no Wayland, the software Vulkan driver, and no real sound device.
	fn command(&self, program: &str, no_sound_config: &Path) -> Command {
		let mut command = Command::new(program);
		command
			.env_remove("WAYLAND_DISPLAY")
			.env("DISPLAY", &self.display)
			.env("XDG_SESSION_TYPE", "x11")
			.env("VK_ICD_FILENAMES", LAVAPIPE)
			.env("AFTERGLOW_GPU_ADAPTER", "llvmpipe")
			.env("ALSA_CONFIG_PATH", no_sound_config);
		command
	}

	fn grab(&self, crop: Option<(u32, u32, u32, u32)>) -> Frame {
		let mut command = Command::new("import");
		command
			.env("DISPLAY", &self.display)
			.env_remove("WAYLAND_DISPLAY")
			.args(["-window", "root"]);
		if let Some((width, height, x, y)) = crop {
			command.args(["-crop", &format!("{width}x{height}+{x}+{y}"), "+repage"]);
		}
		let output = command.args(["-depth", "8"]).arg("ppm:-").output().expect("import runs");
		assert!(output.status.success(), "import failed");
		parse_ppm(&output.stdout)
	}

	/// Gives the program's window the keyboard, as a window manager would; this X server has none.
	fn focus_window(&self, name: &str) {
		let found = Command::new("xdotool")
			.env("DISPLAY", &self.display)
			.env_remove("WAYLAND_DISPLAY")
			.args(["search", "--sync", "--name", name])
			.output()
			.unwrap();
		let id = String::from_utf8_lossy(&found.stdout)
			.lines()
			.next()
			.unwrap_or("")
			.trim()
			.to_string();
		assert!(!id.is_empty(), "no window named {name}");
		Command::new("xdotool")
			.env("DISPLAY", &self.display)
			.env_remove("WAYLAND_DISPLAY")
			.args(["windowfocus", &id])
			.status()
			.unwrap();
	}

	fn key_and_move(&self, moves: &[(i32, i32)], key: Option<&str>) {
		for &(x, y) in moves {
			let status = Command::new("xdotool")
				.env("DISPLAY", &self.display)
				.env_remove("WAYLAND_DISPLAY")
				.args(["mousemove", &x.to_string(), &y.to_string()])
				.status()
				.unwrap();
			assert!(status.success());
			std::thread::sleep(Duration::from_millis(400));
		}
		if let Some(key) = key {
			let status = Command::new("xdotool")
				.env("DISPLAY", &self.display)
				.env_remove("WAYLAND_DISPLAY")
				.args(["key", key])
				.status()
				.unwrap();
			assert!(status.success());
		}
	}
}

impl Drop for Xvfb {
	fn drop(&mut self) {
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}

fn parse_ppm(bytes: &[u8]) -> Frame {
	assert_eq!(&bytes[..2], b"P6");
	let mut fields = Vec::new();
	let mut position = 2;
	while fields.len() < 3 {
		while bytes[position].is_ascii_whitespace() {
			position += 1;
		}
		let start = position;
		while !bytes[position].is_ascii_whitespace() {
			position += 1;
		}
		fields.push(
			std::str::from_utf8(&bytes[start..position])
				.unwrap()
				.parse::<usize>()
				.unwrap(),
		);
	}
	position += 1;
	let (width, height) = (fields[0], fields[1]);
	let mut frame = Frame::new(width, height);
	for (index, rgb) in bytes[position..].as_chunks::<3>().0.iter().take(width * height).enumerate() {
		frame.px[index] = (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
	}
	frame
}

/// A running program with its stderr watched line by line.
struct Program {
	child: Child,
	lines: Receiver<String>,
	log: String,
}

impl Program {
	fn start(mut command: Command) -> Program {
		let mut child = command
			.stdout(Stdio::null())
			.stderr(Stdio::piped())
			.spawn()
			.expect("the program starts");
		let stderr = child.stderr.take().unwrap();
		let (sender, lines) = channel();
		std::thread::spawn(move || {
			for line in BufReader::new(stderr).lines().map_while(Result::ok) {
				if sender.send(line).is_err() {
					break;
				}
			}
		});
		Program {
			child,
			lines,
			log: String::new(),
		}
	}

	/// Waits until a stderr line contains `marker`; panics with the log so far after `seconds`.
	fn wait_for(&mut self, marker: &str, seconds: u64) {
		let deadline = Instant::now() + Duration::from_secs(seconds);
		while Instant::now() < deadline {
			if let Ok(line) = self.lines.recv_timeout(Duration::from_millis(100)) {
				self.log.push_str(&line);
				self.log.push('\n');
				if line.contains(marker) {
					return;
				}
			}
		}
		panic!("never saw \"{marker}\"; stderr so far:\n{}", self.log);
	}

	/// Whether it has exited within `seconds`, and with success.
	fn exits_within(&mut self, seconds: u64) -> Option<bool> {
		let deadline = Instant::now() + Duration::from_secs(seconds);
		while Instant::now() < deadline {
			if let Some(status) = self.child.try_wait().unwrap() {
				return Some(status.success());
			}
			std::thread::sleep(Duration::from_millis(50));
		}
		None
	}

	fn is_running(&mut self) -> bool {
		self.child.try_wait().unwrap().is_none()
	}

	/// Everything the program has written since the last look, appended to the log and returned.
	fn log_now(&mut self) -> &str {
		std::thread::sleep(Duration::from_millis(100));
		while let Ok(line) = self.lines.try_recv() {
			self.log.push_str(&line);
			self.log.push('\n');
		}
		&self.log
	}
}

impl Drop for Program {
	fn drop(&mut self) {
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}

/// A scratch folder for one test, removed afterwards.
struct Scratch(PathBuf);

impl Scratch {
	fn new(name: &str) -> Scratch {
		let path = std::env::temp_dir().join(format!("afterglow-linux-{name}-{}", std::process::id()));
		std::fs::create_dir_all(&path).unwrap();
		Scratch(path)
	}

	/// An ALSA configuration with no devices at all, so a test can never reach the real sound card.
	fn empty_alsa(&self) -> PathBuf {
		let path = self.0.join("empty-alsa.conf");
		std::fs::write(&path, "").unwrap();
		path
	}
}

impl Drop for Scratch {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.0);
	}
}

fn to_ppm(frame: &Frame) -> Vec<u8> {
	let mut bytes = format!("P6\n{} {}\n255\n", frame.w, frame.h).into_bytes();
	for &pixel in &frame.px {
		bytes.extend_from_slice(&[(pixel >> 16) as u8, (pixel >> 8) as u8, pixel as u8]);
	}
	bytes
}

fn is_not_blank(frame: &Frame) -> bool {
	let mut lit = 0;
	let mut distinct = std::collections::BTreeSet::new();
	for &pixel in &frame.px {
		if pixel & 0xFFFFFF != 0 {
			lit += 1;
		}
		distinct.insert(pixel >> 3);
	}
	lit > frame.px.len() / 20 && distinct.len() > 200
}

#[test]
#[ignore = "needs Xvfb, lavapipe, ImageMagick and xdotool: `make check-gpu`"]
fn a_window_draws_the_scene_and_matches_the_cpu_within_budget() {
	let Some(xvfb) = Xvfb::start(1280, 720) else { return };
	let scratch = Scratch::new("window");
	let mut command = xvfb.command(BINARY, &scratch.empty_alsa());
	command.args([
		"--window",
		"640x360",
		"--at",
		"40",
		"--seed",
		"1",
		"--no-sound",
		"--fps",
		"60",
		"--set",
		"Grain=0",
		"--set",
		"Detail=1",
		"--verbose",
	]);
	let mut program = Program::start(command);
	program.wait_for("frame 90", 60);
	let frame = xvfb.grab(Some((640, 360, 0, 0)));
	if let Some(dump) = std::env::var_os("AFTERGLOW_TEST_DUMP") {
		std::fs::write(Path::new(&dump).join("window.ppm"), to_ppm(&frame)).unwrap();
	}
	assert!(is_not_blank(&frame), "the window is blank");

	let mut settings = Settings::defaults(SaverKind::Twilight);
	settings.set(Key::RenderScale, 100);
	settings.set(Key::Detail, 1);
	settings.set(Key::Grain, 0);
	let cpu = &cpu_frames(&settings, 1, (640, 360), &[40.0])[0];
	let difference = difference(cpu, &frame);
	eprintln!("window against the CPU: {}", difference.describe());
	// The parity budget of docs/gpu-twilight.md, through a window system and the larger maps of this edition.
	assert!(difference.within_budget(), "{}", difference.describe());

	// A key ends a window with a clean exit.
	xvfb.focus_window("Afterglow Twilight");
	xvfb.key_and_move(&[(100, 100)], Some("a"));
	assert_eq!(
		program.exits_within(5),
		Some(true),
		"the window did not end on a key:\n{}",
		program.log_now()
	);
}

#[test]
#[ignore = "needs Xvfb, lavapipe, ImageMagick and xdotool: `make check-gpu`"]
fn fullscreen_covers_the_screen_and_ends_on_a_key() {
	let Some(xvfb) = Xvfb::start(1920, 1080) else { return };
	let scratch = Scratch::new("fullscreen");
	let mut command = xvfb.command(BINARY, &scratch.empty_alsa());
	command.args(["--no-sound", "--scale", "50", "--fps", "30", "--verbose"]);
	let mut program = Program::start(command);
	program.wait_for("frame 60", 60);
	assert!(program.log.contains("display 1920x1080, drawn at 960x540"), "{}", program.log);
	let frame = xvfb.grab(None);
	assert_eq!((frame.w, frame.h), (1920, 1080));
	assert!(is_not_blank(&frame), "the screen is blank");
	// Every corner of the screen is painted by the saver (the sky is dark, but the window covers it): the window's own pixels are
	// neither the root's default black everywhere nor unchanged between frames of a moving scene.
	xvfb.key_and_move(&[(500, 500)], Some("space"));
	assert_eq!(program.exits_within(5), Some(true), "no exit on a key:\n{}", program.log);
}

#[test]
#[ignore = "needs Xvfb, lavapipe, ImageMagick and xdotool: `make check-gpu`"]
fn fullscreen_ends_on_ten_pixels_of_movement_but_not_on_two() {
	let Some(xvfb) = Xvfb::start(1280, 720) else { return };
	let scratch = Scratch::new("movement");
	let mut command = xvfb.command(BINARY, &scratch.empty_alsa());
	command.args(["--no-sound", "--fps", "30", "--verbose"]);
	let mut program = Program::start(command);
	program.wait_for("frame 30", 60);
	// The first report after the window settles is where movement is measured from; a few pixels from it does nothing.
	xvfb.key_and_move(&[(300, 300), (303, 301), (299, 297)], None);
	assert!(program.is_running(), "ended on 3 pixels of movement:\n{}", program.log_now());
	xvfb.key_and_move(&[(330, 330)], None);
	assert_eq!(
		program.exits_within(5),
		Some(true),
		"no exit on 30 pixels of movement:\n{}",
		program.log
	);
}

#[test]
#[ignore = "needs Xvfb, lavapipe, ImageMagick and xdotool: `make check-gpu`"]
fn a_hack_in_xscreensavers_window_draws_there_and_ignores_input() {
	use x11rb::connection::Connection;
	use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass};
	let Some(xvfb) = Xvfb::start(1280, 720) else { return };
	let scratch = Scratch::new("hack");
	// A window of our own stands in for the one xscreensaver makes.
	let (connection, screen_number) = x11rb::rust_connection::RustConnection::connect(Some(&xvfb.display)).unwrap();
	let screen = &connection.setup().roots[screen_number];
	let parent = connection.generate_id().unwrap();
	connection
		.create_window(
			screen.root_depth,
			parent,
			screen.root,
			100,
			50,
			480,
			270,
			0,
			WindowClass::INPUT_OUTPUT,
			0,
			&CreateWindowAux::new().background_pixel(screen.black_pixel),
		)
		.unwrap();
	connection.map_window(parent).unwrap();
	connection.flush().unwrap();

	// Through the wrapper script that goes in xscreensaver's `programs:` list, which finds the program on the PATH here.
	let wrapper = concat!(env!("CARGO_MANIFEST_DIR"), "/xscreensaver/afterglow-twilight-xscreensaver");
	let program_folder = Path::new(BINARY).parent().unwrap();
	let mut command = xvfb.command(wrapper, &scratch.empty_alsa());
	command.env(
		"PATH",
		format!("{}:{}", program_folder.display(), std::env::var("PATH").unwrap_or_default()),
	);
	command.args(["-window-id", &format!("0x{parent:x}"), "--fps", "30", "--verbose"]);
	let mut program = Program::start(command);
	program.wait_for("frame 60", 60);
	assert!(program.log.contains("display 480x270"), "{}", program.log);
	assert!(
		program.log.contains("sound off"),
		"a hack must be silent unless asked:\n{}",
		program.log
	);
	let frame = xvfb.grab(Some((480, 270, 100, 50)));
	assert!(is_not_blank(&frame), "the parent window is blank");
	// A key or movement does nothing: xscreensaver ends a hack, not the hack itself.
	xvfb.key_and_move(&[(200, 100), (260, 160)], Some("a"));
	std::thread::sleep(Duration::from_millis(500));
	assert!(program.is_running(), "the hack ended on input:\n{}", program.log_now());
}

#[test]
#[ignore = "needs Xvfb, lavapipe, ImageMagick and xdotool: `make check-gpu`"]
fn sound_reaches_a_device_through_cpal_and_a_missing_device_is_silence() {
	let Some(xvfb) = Xvfb::start(1280, 720) else { return };
	let scratch = Scratch::new("sound");
	let captured = scratch.0.join("captured.wav");
	// ALSA's default device is a file: cpal opens it as any card, and what it plays is written there.
	let alsa = scratch.0.join("alsa.conf");
	std::fs::write(
		&alsa,
		format!(
			"pcm.sink {{\n\ttype null\n}}\npcm.!default {{\n\ttype plug\n\tslave.pcm {{\n\t\ttype file\n\t\tslave.pcm sink\n\t\tfile \"{}\"\n\t\tformat wav\n\t}}\n}}\n",
			captured.display()
		),
	)
	.unwrap();
	let mut command = xvfb.command(BINARY, &alsa);
	command.args([
		"--window",
		"320x180",
		"--sound",
		"--fps",
		"30",
		"--verbose",
		"--set",
		"Volume=60",
	]);
	let mut program = Program::start(command);
	program.wait_for("frame 120", 60);
	assert!(
		program.log.contains("sound on"),
		"no sound device was opened:\n{}",
		program.log
	);
	xvfb.focus_window("Afterglow Twilight");
	xvfb.key_and_move(&[(50, 50)], Some("a"));
	assert_eq!(program.exits_within(5), Some(true), "{}", program.log_now());
	let mut bytes = Vec::new();
	std::fs::File::open(&captured)
		.expect("the file device was written")
		.read_to_end(&mut bytes)
		.unwrap();
	// The file plug-in keeps the stream's own format: 32-bit floats, as cpal wrote them, under a PCM-looking header.
	let bits = u16::from_le_bytes([bytes[34], bytes[35]]);
	assert!(bits == 16 || bits == 32, "{bits}-bit WAV");
	let samples: Vec<i16> = if bits == 32 {
		bytes[44..]
			.as_chunks::<4>()
			.0
			.iter()
			.map(|quad| (f32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]) * 32_768.0).clamp(-32_768.0, 32_767.0) as i16)
			.collect()
	} else {
		bytes[44..]
			.as_chunks::<2>()
			.0
			.iter()
			.map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
			.collect()
	};
	assert!(samples.len() > 48_000, "only {} samples", samples.len());
	let peak = samples.iter().map(|&s| (s as i32).abs()).max().unwrap();
	assert!(peak > 100, "the captured sound is silent (peak {peak})");
	// The synth does not click, and neither does the fade at the end of the run: no step between samples of one channel's
	// neighbours over a threshold (the file holds the ramp that ended the run).
	let largest_step = samples
		.as_chunks::<2>()
		.0
		.iter()
		.collect::<Vec<_>>()
		.windows(2)
		.map(|pair| (pair[1][0] as i32 - pair[0][0] as i32).abs())
		.max()
		.unwrap();
	assert!(largest_step < 3_000, "a step of {largest_step} between samples is a click");
	assert!(samples[samples.len() - 1].abs() < 8, "the sound did not fade out at the end");

	// With no device at all it carries on in silence.
	let mut command = xvfb.command(BINARY, &scratch.empty_alsa());
	command.args(["--window", "320x180", "--sound", "--fps", "30", "--verbose"]);
	let mut program = Program::start(command);
	program.wait_for("frame 60", 60);
	assert!(program.log.contains("no sound"), "{}", program.log);
	assert!(program.is_running());
}
