// The saver's settings file: a flat `Name = value` subset of TOML under the XDG config folder, read into the shared settings table.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;
use std::path::PathBuf;

use savers_core::settings::{Control, Key, SaverKind, Settings, SettingsStore, fields};

/// Linux-only values that sit beside the shared settings in the file.
pub const FRAME_CAP: &str = "FrameCap";

/// Everything the program is configured with, apart from the command line.
#[derive(Clone, Debug)]
pub struct Config {
	pub settings: Settings,
	/// Frames a second at most; 0 draws as fast as the display's refresh (vsync).
	pub frame_cap: u32,
}

impl Config {
	/// The defaults of this edition: the same as the Win32 saver's, except the settings a graphics card makes free. It draws at
	/// the display's full size and at Ultra detail, and is capped at 60 frames a second.
	pub fn defaults() -> Config {
		let mut settings = Settings::defaults(SaverKind::Twilight);
		settings.set(Key::RenderScale, 100);
		settings.set(Key::Detail, 3);
		Config { settings, frame_cap: 60 }
	}

	/// The defaults with whatever the store holds laid over them.
	pub fn from_store(store: &Store) -> Config {
		let mut config = Config::defaults();
		for field in fields(SaverKind::Twilight) {
			if let Some(value) = store.get_dword(field.name) {
				config.settings.set(field.key, value);
			}
		}
		if let Some(cap) = store.get_dword(FRAME_CAP) {
			config.frame_cap = valid_frame_cap(cap);
		}
		config
	}

	/// Sets one value by its registry name (`--set Name=value` and the file share this).
	pub fn set_named(&mut self, name: &str, text: &str) -> Result<(), String> {
		if name.eq_ignore_ascii_case(FRAME_CAP) {
			let value = parse_value_for(None, text).ok_or_else(|| format!("{FRAME_CAP}: \"{text}\" is not a number"))?;
			self.frame_cap = valid_frame_cap(value);
			return Ok(());
		}
		let field = fields(SaverKind::Twilight)
			.find(|field| field.name.eq_ignore_ascii_case(name))
			.ok_or_else(|| format!("Twilight has no setting named {name}"))?;
		let value = parse_value_for(Some(&field.control), text)
			.filter(|&value| value >= field.min && value <= field.max)
			.ok_or_else(|| {
				format!(
					"{}: \"{text}\" is not a value from {} to {}",
					field.name, field.min, field.max
				)
			})?;
		self.settings.set(field.key, value);
		Ok(())
	}
}

/// 0 (the display's refresh) or 10 to 240; anything else is the 60 default.
fn valid_frame_cap(value: u32) -> u32 {
	if value == 0 || (10..=240).contains(&value) {
		value
	} else {
		60
	}
}

/// A number, or for a choice field the label of one of its choices ("Aurora", "Ultra", "50%").
fn parse_value_for(control: Option<&Control>, text: &str) -> Option<u32> {
	let text = text.trim();
	if let Ok(number) = text.trim_end_matches('%').parse::<u32>() {
		return Some(number);
	}
	if let Some(Control::Choice(choices)) = control {
		let wanted = text.to_lowercase();
		return choices
			.iter()
			.find(|(label, _)| {
				let label = label.to_lowercase();
				label == wanted || label.split(' ').next() == Some(wanted.as_str())
			})
			.map(|&(_, value)| value);
	}
	match text.to_lowercase().as_str() {
		"true" | "on" | "yes" => Some(1),
		"false" | "off" | "no" => Some(0),
		_ => None,
	}
}

/// The parsed file: names to their text, in a map.
#[derive(Clone, Debug, Default)]
pub struct Store {
	values: BTreeMap<String, String>,
}

impl Store {
	/// Parses the file's text. A line is `Name = value`, with the value a number, a bare word or a quoted string; `#` starts a
	/// comment and a `[section]` line is skipped. Lines that are not understood are ignored.
	pub fn parse(text: &str) -> Store {
		let mut values = BTreeMap::new();
		for line in text.lines() {
			let line = line.split('#').next().unwrap_or("").trim();
			if line.is_empty() || line.starts_with('[') {
				continue;
			}
			if let Some((name, value)) = line.split_once('=') {
				let value = value.trim().trim_matches('"');
				values.insert(name.trim().to_lowercase(), value.to_string());
			}
		}
		Store { values }
	}

	pub fn get(&self, name: &str) -> Option<&String> {
		self.values.get(&name.to_lowercase())
	}
}

impl SettingsStore for Store {
	fn get_dword(&self, name: &str) -> Option<u32> {
		let text = self.get(name)?;
		let control = fields(SaverKind::Twilight)
			.find(|field| field.name.eq_ignore_ascii_case(name))
			.map(|field| field.control);
		parse_value_for(control.as_ref(), text)
	}

	fn get_text(&self, name: &str) -> Option<String> {
		self.get(name).cloned()
	}

	fn set_dword(&mut self, name: &str, value: u32) {
		self.values.insert(name.to_lowercase(), value.to_string());
	}

	fn set_text(&mut self, name: &str, value: &str) {
		self.values.insert(name.to_lowercase(), value.to_string());
	}
}

/// `$XDG_CONFIG_HOME/liminal-hq/afterglow-savers/twilight.toml`, or the same under `~/.config`.
pub fn default_path() -> Option<PathBuf> {
	let base = match std::env::var_os("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
		Some(dir) => PathBuf::from(dir),
		None => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
	};
	Some(base.join("liminal-hq/afterglow-savers/twilight.toml"))
}

/// Reads the settings file (a missing one is the defaults). `path` overrides the default place.
pub fn load(path: Option<&PathBuf>) -> Result<Config, String> {
	let path = match path {
		Some(path) => Some(path.clone()),
		None => default_path(),
	};
	let Some(path) = path else {
		return Ok(Config::defaults());
	};
	match std::fs::read_to_string(&path) {
		Ok(text) => Ok(Config::from_store(&Store::parse(&text))),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::defaults()),
		Err(error) => Err(format!("{}: {error}", path.display())),
	}
}

/// A commented file holding every setting at this edition's default, for `--write-config`.
pub fn template() -> String {
	let defaults = Config::defaults();
	let mut text = String::from(
		"# Afterglow Twilight (Linux, graphics card): settings.\n\
		 #\n\
		 # (c) Copyright 2026 Liminal HQ, Scott Morris\n\
		 # SPDX-License-Identifier: Apache-2.0 OR MIT\n\
		 #\n\
		 # The names are the Win32 saver's registry values. Delete a line to use its default. A choice may be written by its label.\n\
		 # Command-line flags (`--set Name=value`, `--palette`, `--scale`) win over this file.\n\n[twilight]\n",
	);
	for field in fields(SaverKind::Twilight) {
		if matches!(field.key, Key::MaxFps) {
			continue;
		}
		let note = match field.control {
			Control::Choice(choices) => {
				let list: Vec<String> = choices.iter().map(|(label, value)| format!("{value} {label}")).collect();
				format!("one of {}", list.join(", "))
			}
			Control::Check => "0 or 1".to_string(),
			_ => format!("{} to {}", field.min, field.max),
		};
		text.push_str(&format!(
			"# {}: {note}\n{} = {}\n",
			field.label,
			field.name,
			defaults.settings.get(field.key)
		));
	}
	text.push_str(&format!(
		"# Frames a second at most: 0 for the display's refresh (vsync), or 10 to 240\n{FRAME_CAP} = {}\n",
		defaults.frame_cap
	));
	text
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn defaults_draw_at_full_size_and_ultra() {
		let config = Config::defaults();
		assert_eq!(config.settings.get(Key::RenderScale), 100);
		assert_eq!(config.settings.get(Key::Detail), 3);
		assert_eq!(config.frame_cap, 60);
		assert_eq!(config.settings.get(Key::Sound), 1);
		assert_eq!(config.settings.get(Key::Volume), 28);
	}

	#[test]
	fn the_file_overrides_defaults_and_accepts_labels() {
		let store = Store::parse(
			"[twilight]\n# a comment\nPalette = \"Aurora\"\nRenderScale = 75 # three quarters\nspeed=150\nFrameCap = 0\nDetail = Ultra\nBogus = 3\n",
		);
		let config = Config::from_store(&store);
		assert_eq!(config.settings.get(Key::Palette), 3);
		assert_eq!(config.settings.get(Key::RenderScale), 75);
		assert_eq!(config.settings.get(Key::Speed), 150);
		assert_eq!(config.settings.get(Key::Detail), 3);
		assert_eq!(config.frame_cap, 0);
	}

	#[test]
	fn an_out_of_range_value_falls_back_to_the_fields_default() {
		let config = Config::from_store(&Store::parse("Speed = 9000\nFrameCap = 7\n"));
		assert_eq!(config.settings.get(Key::Speed), 100);
		assert_eq!(config.frame_cap, 60);
	}

	#[test]
	fn set_named_checks_names_and_ranges() {
		let mut config = Config::defaults();
		config.set_named("RenderScale", "50").unwrap();
		config.set_named("palette", "ember").unwrap();
		config.set_named("FrameCap", "120").unwrap();
		assert_eq!(config.settings.get(Key::RenderScale), 50);
		assert_eq!(config.settings.get(Key::Palette), 2);
		assert_eq!(config.frame_cap, 120);
		assert!(config.set_named("Nope", "1").is_err());
		assert!(config.set_named("Speed", "5").is_err());
	}

	#[test]
	fn the_template_reads_back_as_the_defaults() {
		let config = Config::from_store(&Store::parse(&template()));
		let defaults = Config::defaults();
		for field in fields(SaverKind::Twilight) {
			assert_eq!(
				config.settings.get(field.key),
				defaults.settings.get(field.key),
				"{}",
				field.name
			);
		}
		assert_eq!(config.frame_cap, defaults.frame_cap);
	}
}
