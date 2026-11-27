// The savers' settings: one table of fields that the registry code, the dialog and the preview tool all read.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! SPEC section 7. Every value has a range and a default; anything missing or out of range reads as the default, so no value a user
//! or another program puts in the registry can upset a scene.

use alloc::string::String;

/// The six savers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaverKind {
	Maze,
	Cracktro,
	Twilight,
	Ink,
	Collection,
	Ghostwriter,
}

impl SaverKind {
	pub const ALL: [SaverKind; 6] = [
		SaverKind::Maze,
		SaverKind::Cracktro,
		SaverKind::Twilight,
		SaverKind::Ink,
		SaverKind::Collection,
		SaverKind::Ghostwriter,
	];

	/// The subkey under `HKCU\Software\Liminal HQ\Afterglow Screensavers`.
	pub fn registry_name(self) -> &'static str {
		match self {
			SaverKind::Maze => "Maze",
			SaverKind::Cracktro => "Cracktro",
			SaverKind::Twilight => "Twilight",
			SaverKind::Ink => "Ink",
			SaverKind::Collection => "Collection",
			SaverKind::Ghostwriter => "Ghostwriter",
		}
	}

	/// String 1: the name in the Screen Saver list.
	pub fn title(self) -> &'static str {
		match self {
			SaverKind::Maze => "Afterglow Maze",
			SaverKind::Cracktro => "Afterglow Cracktro",
			SaverKind::Twilight => "Afterglow Twilight",
			SaverKind::Ink => "Afterglow Ink",
			SaverKind::Collection => "Afterglow Collection",
			SaverKind::Ghostwriter => "Quill Ghostwriter",
		}
	}

	pub fn file_name(self) -> &'static str {
		match self {
			SaverKind::Maze => "AGMAZE.SCR",
			SaverKind::Cracktro => "AGCRACK.SCR",
			SaverKind::Twilight => "AGTWILIT.SCR",
			SaverKind::Ink => "AGINK.SCR",
			SaverKind::Collection => "AGCOLLCT.SCR",
			SaverKind::Ghostwriter => "AGQUILL.SCR",
		}
	}

	pub fn from_name(name: &str) -> Option<SaverKind> {
		SaverKind::ALL
			.into_iter()
			.find(|k| k.registry_name().eq_ignore_ascii_case(name))
	}

	/// Whether the scene has a light (day) mode.
	pub fn has_light_mode(self) -> bool {
		matches!(
			self,
			SaverKind::Ink | SaverKind::Maze | SaverKind::Ghostwriter | SaverKind::Collection
		)
	}
}

/// Every setting, as an index into `Settings::values`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
	RenderScale,
	Detail,
	Speed,
	Palette,
	Light,
	MaxFps,
	Bloom,
	Streaks,
	Vignette,
	Grain,
	Letterbox,
	Fades,
	Seed,
	Floor,
	MazeSize,
	Atmosphere,
	Moons,
	Thickness,
	Turbulence,
	Parts,
	Shuffle,
	Duration,
	Savers,
	Acts,
}

pub const KEY_COUNT: usize = Key::Acts as usize + 1;

/// How the dialog shows a field.
#[derive(Clone, Copy, Debug)]
pub enum Control {
	/// A combo box of (label, value).
	Choice(&'static [(&'static str, u32)]),
	/// A trackbar over the range.
	Slider,
	/// A check box (0 or 1).
	Check,
	/// An edit box for a number.
	Number,
	/// Check boxes, one per bit, labelled in order.
	Mask(&'static [&'static str]),
}

/// One setting: its registry value name, label, range, default and control.
#[derive(Clone, Copy, Debug)]
pub struct Field {
	pub key: Key,
	pub name: &'static str,
	pub label: &'static str,
	pub min: u32,
	pub max: u32,
	pub default: u32,
	pub control: Control,
}

const fn f(key: Key, name: &'static str, label: &'static str, min: u32, max: u32, default: u32, control: Control) -> Field {
	Field {
		key,
		name,
		label,
		min,
		max,
		default,
		control,
	}
}

const SCALES: &[(&str, u32)] = &[("100% (sharpest)", 100), ("75%", 75), ("50%", 50), ("25% (fastest)", 25)];
const DETAILS: &[(&str, u32)] = &[("Low", 0), ("Medium", 1), ("High", 2), ("Ultra", 3)];
const PALETTES: &[(&str, u32)] = &[("Dusk", 0), ("Indigo", 1), ("Ember", 2), ("Aurora", 3), ("Graphite", 4)];
const PALETTES_QUILL: &[(&str, u32)] = &[
	("Quill navy and gold", 5),
	("Dusk", 0),
	("Indigo", 1),
	("Ember", 2),
	("Aurora", 3),
	("Graphite", 4),
];
const FPS: &[(&str, u32)] = &[("15", 15), ("20", 20), ("30", 30), ("45", 45), ("60", 60)];
const FLOORS: &[(&str, u32)] = &[("Polished stone", 0), ("Still water", 1), ("Inlaid tiles", 2)];
const MOONS: &[(&str, u32)] = &[("None", 0), ("One", 1), ("Two", 2)];

/// The settings every saver has (SPEC section 7).
const COMMON: &[Field] = &[
	f(Key::Palette, "Palette", "Palette", 0, 4, 0, Control::Choice(PALETTES)),
	f(Key::Light, "Light", "Light (day) mode", 0, 1, 0, Control::Check),
	f(Key::Speed, "Speed", "Speed", 25, 200, 100, Control::Slider),
	f(
		Key::RenderScale,
		"RenderScale",
		"Render scale",
		25,
		100,
		50,
		Control::Choice(SCALES),
	),
	f(Key::Detail, "Detail", "Detail", 0, 3, 1, Control::Choice(DETAILS)),
	f(
		Key::MaxFps,
		"MaxFps",
		"Frames per second at most",
		10,
		60,
		30,
		Control::Choice(FPS),
	),
	f(Key::Bloom, "Bloom", "Glow", 0, 100, 60, Control::Slider),
	f(Key::Streaks, "Streaks", "Light streaks", 0, 100, 40, Control::Slider),
	f(Key::Vignette, "Vignette", "Vignette", 0, 100, 50, Control::Slider),
	f(Key::Grain, "Grain", "Film grain", 0, 100, 30, Control::Slider),
	f(Key::Letterbox, "Letterbox", "Letterbox bars", 0, 1, 0, Control::Check),
	f(Key::Fades, "Fades", "Slow fades between shots", 0, 1, 1, Control::Check),
	f(
		Key::Seed,
		"Seed",
		"Seed (0 for a new one each time)",
		0,
		u32::MAX,
		0,
		Control::Number,
	),
];

const MAZE: &[Field] = &[
	f(Key::Floor, "Floor", "Floor", 0, 2, 0, Control::Choice(FLOORS)),
	f(Key::MazeSize, "MazeSize", "Maze size", 8, 32, 14, Control::Slider),
];
const TWILIGHT: &[Field] = &[
	f(Key::Atmosphere, "Atmosphere", "Atmosphere", 0, 100, 70, Control::Slider),
	f(Key::Moons, "Moons", "Moons", 0, 2, 2, Control::Choice(MOONS)),
];
const INK: &[Field] = &[
	f(Key::Thickness, "Thickness", "Ink thickness", 0, 100, 50, Control::Slider),
	f(Key::Turbulence, "Turbulence", "Turbulence", 0, 100, 50, Control::Slider),
];
const CRACKTRO: &[Field] = &[
	f(
		Key::Parts,
		"Parts",
		"Parts",
		1,
		31,
		31,
		Control::Mask(&["Copper bars", "Plasma", "Rotozoomer", "Tunnel", "Vector bars"]),
	),
	f(Key::Shuffle, "Shuffle", "Shuffle the parts", 0, 1, 0, Control::Check),
];
const COLLECTION: &[Field] = &[
	f(Key::Duration, "Duration", "Seconds per saver", 30, 600, 90, Control::Number),
	f(
		Key::Savers,
		"Savers",
		"Savers",
		1,
		31,
		15,
		Control::Mask(&["Ink", "Twilight", "Cracktro", "Maze", "Quill Ghostwriter"]),
	),
	f(Key::Shuffle, "Shuffle", "Shuffle the order", 0, 1, 0, Control::Check),
];
const GHOSTWRITER: &[Field] = &[f(
	Key::Acts,
	"Acts",
	"Acts",
	1,
	63,
	63,
	Control::Mask(&["Typing", "Reflow", "Time machine", "Font eras", "Lore", "The dialog"]),
)];

/// The scene's own fields, after the common ones.
pub fn scene_fields(kind: SaverKind) -> &'static [Field] {
	match kind {
		SaverKind::Maze => MAZE,
		SaverKind::Cracktro => CRACKTRO,
		SaverKind::Twilight => TWILIGHT,
		SaverKind::Ink => INK,
		SaverKind::Collection => COLLECTION,
		SaverKind::Ghostwriter => GHOSTWRITER,
	}
}

/// The common fields, as the saver shows them (Ghostwriter's palette list starts with Quill's own).
pub fn common_fields(kind: SaverKind) -> impl Iterator<Item = Field> {
	COMMON.iter().map(move |field| {
		let mut field = *field;
		if kind == SaverKind::Ghostwriter && field.key == Key::Palette {
			field.max = 5;
			field.default = 5;
			field.control = Control::Choice(PALETTES_QUILL);
		}
		field
	})
}

/// Every field of a saver: the common ones, then its own.
pub fn fields(kind: SaverKind) -> impl Iterator<Item = Field> {
	common_fields(kind).chain(scene_fields(kind).iter().copied())
}

/// The saver's text setting, if it has one: (value name, label, longest length).
pub fn text_field(kind: SaverKind) -> Option<(&'static str, &'static str, usize)> {
	match kind {
		SaverKind::Cracktro => Some(("Message", "Scroller text", 2000)),
		SaverKind::Ghostwriter => Some(("TextBank", "Text bank (one line each)", 8000)),
		_ => None,
	}
}

/// Where settings are kept: the registry on Windows, a map in the preview tool and the tests.
pub trait SettingsStore {
	fn get_dword(&self, name: &str) -> Option<u32>;
	fn get_text(&self, name: &str) -> Option<String>;
	fn set_dword(&mut self, name: &str, value: u32);
	fn set_text(&mut self, name: &str, value: &str);
}

/// A saver's settings, always valid.
#[derive(Clone, Debug)]
pub struct Settings {
	pub kind: SaverKind,
	values: [u32; KEY_COUNT],
	/// The cracktro's message or Ghostwriter's text bank; empty means the built-in text.
	pub text: String,
}

impl Settings {
	pub fn defaults(kind: SaverKind) -> Settings {
		let mut values = [0u32; KEY_COUNT];
		// Keys a saver does not use still hold a sensible value, so a scene may read any of them.
		for field in COMMON
			.iter()
			.chain(MAZE)
			.chain(TWILIGHT)
			.chain(INK)
			.chain(CRACKTRO)
			.chain(COLLECTION)
			.chain(GHOSTWRITER)
		{
			values[field.key as usize] = field.default;
		}
		let mut s = Settings {
			kind,
			values,
			text: String::new(),
		};
		for field in fields(kind) {
			s.values[field.key as usize] = field.default;
		}
		s
	}

	#[inline]
	pub fn get(&self, key: Key) -> u32 {
		self.values[key as usize]
	}

	/// Sets a value; one the field does not allow becomes the field's default.
	pub fn set(&mut self, key: Key, value: u32) {
		let field = fields(self.kind).find(|f| f.key == key);
		self.values[key as usize] = match field {
			Some(field) => valid(&field, value),
			None => value,
		};
	}

	/// Reads every field from a store.
	pub fn load(kind: SaverKind, store: &dyn SettingsStore) -> Settings {
		let mut s = Settings::defaults(kind);
		for field in fields(kind) {
			if let Some(v) = store.get_dword(field.name) {
				s.values[field.key as usize] = valid(&field, v);
			}
		}
		if let Some((name, _, longest)) = text_field(kind)
			&& let Some(mut text) = store.get_text(name)
		{
			let mut cut = text.len().min(longest);
			while !text.is_char_boundary(cut) {
				cut -= 1;
			}
			text.truncate(cut);
			s.text = text;
		}
		s
	}

	/// Writes every field to a store.
	pub fn save(&self, store: &mut dyn SettingsStore) {
		for field in fields(self.kind) {
			store.set_dword(field.name, self.values[field.key as usize]);
		}
		if let Some((name, _, _)) = text_field(self.kind) {
			store.set_text(name, &self.text);
		}
	}

	/// The speed as a factor (1.0 is normal).
	pub fn speed(&self) -> f32 {
		self.get(Key::Speed) as f32 / 100.0
	}

	pub fn detail(&self) -> Detail {
		Detail::from_u32(self.get(Key::Detail))
	}
}

/// A value the field allows, or its default.
fn valid(field: &Field, v: u32) -> u32 {
	if v < field.min || v > field.max {
		return field.default;
	}
	match field.control {
		Control::Choice(options) if !options.iter().any(|&(_, o)| o == v) => field.default,
		Control::Mask(_) if v == 0 => field.default,
		_ => v,
	}
}

/// The detail level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Detail {
	Low = 0,
	Medium = 1,
	High = 2,
	Ultra = 3,
}

impl Detail {
	pub fn from_u32(v: u32) -> Detail {
		match v {
			0 => Detail::Low,
			1 => Detail::Medium,
			2 => Detail::High,
			_ => Detail::Ultra,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use alloc::collections::BTreeMap;
	use alloc::string::ToString;

	#[derive(Default)]
	struct Map {
		dwords: BTreeMap<String, u32>,
		texts: BTreeMap<String, String>,
	}

	impl SettingsStore for Map {
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

	#[test]
	fn bad_values_read_as_defaults() {
		let mut m = Map::default();
		m.set_dword("RenderScale", 60);
		m.set_dword("Detail", 7);
		m.set_dword("Speed", 150);
		m.set_dword("Parts", 0);
		m.set_text("Message", &"é".repeat(1500));
		let s = Settings::load(SaverKind::Cracktro, &m);
		assert_eq!(s.get(Key::RenderScale), 50);
		assert_eq!(s.get(Key::Detail), 1);
		assert_eq!(s.get(Key::Speed), 150);
		assert_eq!(s.get(Key::Parts), 31);
		assert!(s.text.len() <= 2000);
	}

	#[test]
	fn round_trips_through_a_store() {
		let mut s = Settings::defaults(SaverKind::Ghostwriter);
		assert_eq!(s.get(Key::Palette), 5);
		s.set(Key::Palette, 3);
		s.text = "line one\nline two".to_string();
		let mut m = Map::default();
		s.save(&mut m);
		let back = Settings::load(SaverKind::Ghostwriter, &m);
		assert_eq!(back.get(Key::Palette), 3);
		assert_eq!(back.text, "line one\nline two");
		assert_eq!(Settings::defaults(SaverKind::Ink).get(Key::Palette), 0);
	}

	#[test]
	fn every_saver_has_unique_value_names() {
		for kind in SaverKind::ALL {
			let names: alloc::vec::Vec<_> = fields(kind).map(|f| f.name).collect();
			for (i, a) in names.iter().enumerate() {
				assert!(!names[i + 1..].contains(a), "{kind:?} repeats {a}");
			}
		}
	}
}
