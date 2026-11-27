// Afterglow Twilight on the graphics card for Linux: the fullscreen saver, the xscreensaver hack, a window to test in, and the
// headless screenshot, bench and sound renders.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod app;
mod args;
mod audio;
mod config;
mod headless;
mod render;

use std::process::ExitCode;

use args::{Mode, Options};
use savers_core::settings::Key;
use savers_gpu::{Gpu, adapter_names};

fn fail(message: impl std::fmt::Display) -> ExitCode {
	eprintln!("afterglow-twilight-gpu: {message}");
	ExitCode::FAILURE
}

/// The settings file, then the command line over it.
fn configure(options: &Options) -> Result<config::Config, String> {
	let mut config = config::load(options.config.as_ref())?;
	for (name, value) in &options.overrides {
		config.set_named(name, value)?;
	}
	Ok(config)
}

fn main() -> ExitCode {
	let options = match args::parse(std::env::args().skip(1)) {
		Ok(options) => options,
		Err(why) => {
			eprintln!("afterglow-twilight-gpu: {why}\n");
			eprint!("{}", args::USAGE);
			return ExitCode::from(2);
		}
	};
	match options.mode {
		Mode::Help => {
			print!("{}", args::USAGE);
			return ExitCode::SUCCESS;
		}
		Mode::Version => {
			println!("afterglow-twilight-gpu {}", env!("CARGO_PKG_VERSION"));
			return ExitCode::SUCCESS;
		}
		Mode::Adapters => {
			for name in adapter_names() {
				println!("{name}");
			}
			return ExitCode::SUCCESS;
		}
		Mode::WriteConfig => return write_config(&options),
		_ => {}
	}

	let mut config = match configure(&options) {
		Ok(config) => config,
		Err(why) => return fail(why),
	};
	// Sound: the command line wins; otherwise the file; but a hack in xscreensaver's window is silent unless asked.
	let is_hack = matches!(options.mode, Mode::Embed(_) | Mode::EmbedFromEnvironment);
	let sound = match options.sound {
		Some(wanted) => wanted,
		None => config.settings.get(Key::Sound) == 1 && !is_hack,
	};
	config.settings.set(Key::Sound, sound as u32);
	let seed = render::choose_seed(options.seed, &config.settings);
	if options.verbose {
		eprintln!(
			"afterglow-twilight-gpu: seed {seed}, sound {}",
			if sound { "requested" } else { "off" }
		);
	}

	match &options.mode {
		Mode::AudioWav(path) => {
			// With sound off in the settings the file would be empty: this mode is always for the score.
			config.settings.set(Key::Sound, 1);
			match audio::render_wav(path, &config.settings, seed, options.seconds.unwrap_or(30.0)) {
				Ok(note) => {
					eprintln!("afterglow-twilight-gpu: audio {note}");
					ExitCode::SUCCESS
				}
				Err(why) => fail(why),
			}
		}
		Mode::Shot(path) => {
			let gpu = match Gpu::new(options.adapter.as_deref()) {
				Ok(gpu) => gpu,
				Err(why) => return fail(why),
			};
			match headless::shot(&gpu, &config.settings, seed, options.size, options.at, path) {
				Ok(note) => {
					eprintln!("afterglow-twilight-gpu: wrote {note}");
					ExitCode::SUCCESS
				}
				Err(why) => fail(why),
			}
		}
		Mode::Bench => {
			let gpu = match Gpu::new(options.adapter.as_deref()) {
				Ok(gpu) => gpu,
				Err(why) => return fail(why),
			};
			match headless::bench(&gpu, &config.settings, seed, options.size, options.frames.unwrap_or(60)) {
				Ok(report) => {
					print!("{report}");
					ExitCode::SUCCESS
				}
				Err(why) => fail(why),
			}
		}
		_ => {
			let plan = app::Plan {
				options: options.clone(),
				settings: config.settings.clone(),
				seed,
				frame_cap: config.frame_cap,
				sound,
			};
			match app::run(plan) {
				app::Outcome::Finished => ExitCode::SUCCESS,
				app::Outcome::Failed(why) => fail(why),
			}
		}
	}
}

fn write_config(options: &Options) -> ExitCode {
	let Some(path) = options.config.clone().or_else(config::default_path) else {
		return fail("no home folder to put the settings file in (set HOME or XDG_CONFIG_HOME)");
	};
	if path.exists() && !options.force {
		println!("{} (already there; --force overwrites it)", path.display());
		return ExitCode::SUCCESS;
	}
	if let Some(parent) = path.parent()
		&& let Err(error) = std::fs::create_dir_all(parent)
	{
		return fail(format!("{}: {error}", parent.display()));
	}
	match std::fs::write(&path, config::template()) {
		Ok(()) => {
			println!("{}", path.display());
			ExitCode::SUCCESS
		}
		Err(error) => fail(format!("{}: {error}", path.display())),
	}
}
