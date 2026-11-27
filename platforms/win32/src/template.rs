// The settings dialog as an in-memory `DLGTEMPLATE`, laid out from the core's field table (SAV-D9). Nothing here calls Windows, so the
// host tests it: the layout, the control identifiers and the byte format.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::string::String;
use alloc::vec::Vec;

use savers_core::settings::{Control, Field, Key, SaverKind, fields, text_field};

/// The first identifier of the field controls; field `n` owns `FIELD_BASE + n * FIELD_STEP` and the next `FIELD_STEP - 1`.
pub const FIELD_BASE: u16 = 1000;
pub const FIELD_STEP: u16 = 8;
/// The multi-line text editor.
pub const TEXT_ID: u16 = 900;
/// The Defaults button; OK and Cancel keep the system's 1 and 2.
pub const DEFAULTS_ID: u16 = 100;
pub const OK_ID: u16 = 1;
pub const CANCEL_ID: u16 = 2;

/// The identifier of the control of field number `index`, and of its check box `bit` when it is a mask.
pub fn field_id(index: usize) -> u16 {
	FIELD_BASE + index as u16 * FIELD_STEP
}

pub fn mask_id(index: usize, bit: usize) -> u16 {
	field_id(index) + 1 + bit as u16
}

// Dialog and control styles.
const WS_POPUP: u32 = 0x8000_0000;
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_BORDER: u32 = 0x0080_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_VSCROLL: u32 = 0x0020_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_GROUP: u32 = 0x0002_0000;
const DS_SETFONT: u32 = 0x0040;
const DS_MODALFRAME: u32 = 0x0080;
const DS_CENTER: u32 = 0x0800;
const BS_PUSHBUTTON: u32 = 0;
const BS_DEFPUSHBUTTON: u32 = 1;
const BS_AUTOCHECKBOX: u32 = 3;
const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
const ES_AUTOHSCROLL: u32 = 0x0080;
const ES_WANTRETURN: u32 = 0x1000;
const ES_NUMBER: u32 = 0x2000;
const CBS_DROPDOWNLIST: u32 = 3;
const TBS_NOTICKS: u32 = 0x0010;

#[derive(Clone, Copy)]
enum Class {
	Button,
	Edit,
	Static,
	ComboBox,
	Trackbar,
}

const GROUP_BOX: u32 = 7;

const MARGIN: i16 = 8;
const GAP: i16 = 8;
/// A group box's inner padding and the height of its title line.
const PAD: i16 = 6;
const GROUP_TOP: i16 = 12;
const GROUP_BOTTOM: i16 = 4;
/// Every one-line row is this tall; a label that needs two lines makes its row one line taller.
const ROW: i16 = 14;
const LINE: i16 = 8;
const HEIGHT: i16 = 12;
const BUTTON_WIDTH: i16 = 50;
/// The two groups above the scene's own, and the scene's one, are laid out in these widths (dialog units).
const HALF: i16 = 176;
const FULL: i16 = HALF * 2 + GAP;
/// How wide a character of the dialog font is, with margin for capitals and for larger fonts, so a label is never clipped.
const CHAR_WIDTH_TENTHS: i32 = 46;

struct Builder {
	bytes: Vec<u8>,
	items: u16,
	ids: Vec<u16>,
	/// The letters already taken as mnemonics, one bit per letter or digit.
	taken: u64,
}

impl Builder {
	fn u16(&mut self, v: u16) {
		self.bytes.extend_from_slice(&v.to_le_bytes());
	}

	fn u32(&mut self, v: u32) {
		self.bytes.extend_from_slice(&v.to_le_bytes());
	}

	fn wide(&mut self, text: &str) {
		for c in text.chars() {
			let mut buffer = [0u16; 2];
			for unit in c.encode_utf16(&mut buffer) {
				self.u16(*unit);
			}
		}
		self.u16(0);
	}

	fn align(&mut self) {
		while !self.bytes.len().is_multiple_of(4) {
			self.bytes.push(0);
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn item(&mut self, class: Class, style: u32, text: &str, id: u16, x: i16, y: i16, w: i16, h: i16) {
		self.align();
		self.u32(WS_CHILD | WS_VISIBLE | style);
		self.u32(0);
		for v in [x, y, w, h] {
			self.u16(v as u16);
		}
		self.u16(id);
		match class {
			Class::Button => self.u16s(&[0xFFFF, 0x0080]),
			Class::Edit => self.u16s(&[0xFFFF, 0x0081]),
			Class::Static => self.u16s(&[0xFFFF, 0x0082]),
			Class::ComboBox => self.u16s(&[0xFFFF, 0x0085]),
			Class::Trackbar => self.wide("msctls_trackbar32"),
		}
		self.wide(text);
		self.u16(0);
		self.items += 1;
		if id != 0xFFFF {
			self.ids.push(id);
		}
	}

	fn u16s(&mut self, values: &[u16]) {
		for v in values {
			self.u16(*v);
		}
	}

	/// The label with an `&` before the first letter of it nobody has used yet, preferring the first letter of a word.
	fn mnemonic(&mut self, label: &str) -> String {
		let bit = |c: char| c.to_ascii_lowercase().to_digit(36).map(|d| 1u64 << d);
		let mut previous = ' ';
		let mut choice = None;
		for (at, c) in label.char_indices() {
			if let Some(b) = bit(c)
				&& self.taken & b == 0
				&& !previous.is_alphanumeric()
			{
				choice = Some((at, b));
				break;
			}
			previous = c;
		}
		if choice.is_none() {
			choice = label
				.char_indices()
				.find_map(|(at, c)| bit(c).filter(|b| self.taken & b == 0).map(|b| (at, b)));
		}
		match choice {
			Some((at, b)) => {
				self.taken |= b;
				let mut marked = String::with_capacity(label.len() + 1);
				marked.push_str(&label[..at]);
				marked.push('&');
				marked.push_str(&label[at..]);
				marked
			}
			None => String::from(label),
		}
	}

	fn label(&mut self, text: &str, x: i16, y: i16, w: i16) {
		let lines = lines_for(text, w);
		let text = self.mnemonic(text);
		self.item(Class::Static, 0, &text, 0xFFFF, x, y + 2, w, LINE * lines);
	}

	fn group(&mut self, title: &str, x: i16, y: i16, w: i16, inner_height: i16) {
		self.item(
			Class::Button,
			GROUP_BOX,
			title,
			0xFFFF,
			x,
			y,
			w,
			GROUP_TOP + inner_height + GROUP_BOTTOM,
		);
	}
}

/// How many lines a label needs in a column `width` units wide, breaking at spaces.
fn lines_for(text: &str, width: i16) -> i16 {
	let per_line = (i32::from(width) * 10 / CHAR_WIDTH_TENTHS).max(1) as usize;
	let mut lines = 1;
	let mut used = 0usize;
	for word in text.split(' ') {
		let need = word.chars().count();
		if used == 0 {
			used = need;
		} else if used + 1 + need <= per_line {
			used += 1 + need;
		} else {
			lines += 1;
			used = need;
		}
	}
	lines
}

/// Where a group's label column ends and its control begins, for a group of a given inner width.
fn columns(inner: i16) -> (i16, i16) {
	let label = inner * 45 / 100;
	(label, inner - label - 4)
}

/// The height of one field in a column of the given inner width.
fn field_height(field: &Field, inner: i16) -> i16 {
	let (label, _) = columns(inner);
	match field.control {
		Control::Mask(bits) => ROW * (1 + (bits.len() as i16 + 1) / 2),
		Control::Check => ROW,
		_ => ROW + (lines_for(field.label, label) - 1) * LINE,
	}
}

/// Which group a setting belongs to: 0 Look, 1 Quality, 2 Finish, 3 the scene's own.
fn section(key: Key) -> u8 {
	match key {
		Key::Palette | Key::Light | Key::Speed => 0,
		Key::RenderScale | Key::Detail | Key::MaxFps => 1,
		Key::Bloom | Key::Streaks | Key::Vignette | Key::Grain | Key::Letterbox | Key::Fades => 2,
		_ => 3,
	}
}

/// Splits fields, in order, into a left and a right run with the least tall of the taller.
fn split_columns(fields: &[(usize, Field)], inner: i16) -> usize {
	let heights: Vec<i16> = fields.iter().map(|(_, f)| field_height(f, inner)).collect();
	let total: i16 = heights.iter().sum();
	let mut best = (i16::MAX, 0);
	let mut left = 0;
	for cut in 0..=heights.len() {
		let tallest = left.max(total - left);
		if tallest < best.0 {
			best = (tallest, cut);
		}
		if cut < heights.len() {
			left += heights[cut];
		}
	}
	best.1
}

/// The height of a run of fields.
fn run_height(fields: &[(usize, Field)], inner: i16) -> i16 {
	fields.iter().map(|(_, f)| field_height(f, inner)).sum()
}

/// A finished template and the identifiers of its controls, for the tests.
pub struct Template {
	pub bytes: Vec<u8>,
	pub ids: Vec<u16>,
}

/// The title of the dialog: the saver's name, then "Settings".
pub fn title(kind: SaverKind) -> String {
	let mut text = String::from(kind.title());
	text.push_str(" Settings");
	text
}

/// Puts a run of fields in one column starting at (`x`, `y`), `inner` units wide. Returns nothing: heights come from `run_height`.
fn place(b: &mut Builder, fields: &[(usize, Field)], x: i16, mut y: i16, inner: i16) {
	let (label_w, control_w) = columns(inner);
	let control_x = x + label_w + 4;
	for &(index, field) in fields {
		let id = field_id(index);
		match field.control {
			Control::Choice(_) => {
				b.label(field.label, x, y, label_w);
				b.item(
					Class::ComboBox,
					WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST,
					"",
					id,
					control_x,
					y,
					control_w,
					90,
				);
			}
			Control::Slider => {
				b.label(field.label, x, y, label_w);
				b.item(
					Class::Trackbar,
					WS_TABSTOP | TBS_NOTICKS,
					"",
					id,
					control_x,
					y,
					control_w,
					HEIGHT + 2,
				);
			}
			Control::Check => {
				let text = b.mnemonic(field.label);
				b.item(
					Class::Button,
					WS_TABSTOP | BS_AUTOCHECKBOX,
					&text,
					id,
					x,
					y + 1,
					inner,
					HEIGHT,
				);
			}
			Control::Number => {
				b.label(field.label, x, y, label_w);
				b.item(
					Class::Edit,
					WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL | ES_NUMBER,
					"",
					id,
					control_x,
					y,
					control_w,
					HEIGHT,
				);
			}
			Control::Mask(bits) => {
				b.label(field.label, x, y, inner);
				for (bit, name) in bits.iter().enumerate() {
					let style = WS_TABSTOP | BS_AUTOCHECKBOX | if bit == 0 { WS_GROUP } else { 0 };
					let text = b.mnemonic(name);
					b.item(
						Class::Button,
						style,
						&text,
						mask_id(index, bit),
						x + 10 + (bit as i16 % 2) * (inner / 2),
						y + ROW * (1 + bit as i16 / 2) + 1,
						inner / 2 - 10,
						HEIGHT,
					);
				}
			}
		}
		y += field_height(&field, inner);
	}
}

/// Builds the settings dialog of a saver: Look and Quality on the left, Finish on the right, the scene's own settings under them, then
/// the buttons.
pub fn build(kind: SaverKind) -> Template {
	let all: Vec<(usize, Field)> = fields(kind).enumerate().collect();
	let group_of = |n: u8| -> Vec<(usize, Field)> { all.iter().copied().filter(|(_, f)| section(f.key) == n).collect() };
	let (look, quality, finish, scene) = (group_of(0), group_of(1), group_of(2), group_of(3));
	let inner_half = HALF - PAD * 2;
	let inner_full = FULL - PAD * 2;
	let sub = (inner_full - PAD) / 2;
	let text = text_field(kind);

	let look_h = run_height(&look, inner_half);
	let quality_h = run_height(&quality, inner_half);
	let finish_h = run_height(&finish, inner_half);
	let cut = split_columns(&scene, sub);
	let scene_body = run_height(&scene[..cut], sub).max(run_height(&scene[cut..], sub));
	let text_h = if text.is_some() { ROW + 40 } else { 0 };
	let scene_h = scene_body + text_h;

	let left_h = GROUP_TOP + look_h + GROUP_BOTTOM + 4 + GROUP_TOP + quality_h + GROUP_BOTTOM;
	let right_h = GROUP_TOP + finish_h + GROUP_BOTTOM;
	let top_h = left_h.max(right_h);
	let scene_y = MARGIN + top_h + 6;
	let buttons_y = scene_y + GROUP_TOP + scene_h + GROUP_BOTTOM + 8;
	let width = MARGIN * 2 + FULL;
	let height = buttons_y + HEIGHT + MARGIN;

	let mut b = Builder {
		bytes: Vec::new(),
		items: 0,
		ids: Vec::new(),
		taken: 0,
	};
	// The Defaults button owns D.
	b.taken |= 1u64 << 13;

	b.u32(WS_POPUP | WS_CAPTION | WS_SYSMENU | DS_MODALFRAME | DS_SETFONT | DS_CENTER);
	b.u32(0);
	b.u16(0); // the item count, patched below
	for v in [0, 0, width, height] {
		b.u16(v as u16);
	}
	b.u16(0); // no menu
	b.u16(0); // the default dialog class
	b.wide(&title(kind));
	b.u16(8);
	b.wide("MS Sans Serif");

	// Left: Look, then Quality.
	b.group("Look", MARGIN, MARGIN, HALF, look_h);
	place(&mut b, &look, MARGIN + PAD, MARGIN + GROUP_TOP, inner_half);
	let quality_y = MARGIN + GROUP_TOP + look_h + GROUP_BOTTOM + 4;
	b.group("Quality", MARGIN, quality_y, HALF, quality_h);
	place(&mut b, &quality, MARGIN + PAD, quality_y + GROUP_TOP, inner_half);
	// Right: Finish.
	let right_x = MARGIN + HALF + GAP;
	b.group("Finish", right_x, MARGIN, HALF, finish_h + (top_h - right_h));
	place(&mut b, &finish, right_x + PAD, MARGIN + GROUP_TOP, inner_half);
	// Under both: the scene's own settings in two runs, and the text editor.
	b.group("Scene", MARGIN, scene_y, FULL, scene_h);
	place(&mut b, &scene[..cut], MARGIN + PAD, scene_y + GROUP_TOP, sub);
	place(&mut b, &scene[cut..], MARGIN + PAD + sub + PAD, scene_y + GROUP_TOP, sub);
	if let Some((_, label, _)) = text {
		let y = scene_y + GROUP_TOP + scene_body;
		b.label(label, MARGIN + PAD, y, inner_full);
		b.item(
			Class::Edit,
			WS_TABSTOP | WS_BORDER | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN,
			"",
			TEXT_ID,
			MARGIN + PAD,
			y + ROW,
			inner_full,
			38,
		);
	}

	b.item(
		Class::Button,
		WS_TABSTOP | BS_PUSHBUTTON,
		"&Defaults",
		DEFAULTS_ID,
		MARGIN,
		buttons_y,
		BUTTON_WIDTH,
		HEIGHT + 2,
	);
	b.item(
		Class::Button,
		WS_TABSTOP | BS_DEFPUSHBUTTON,
		"OK",
		OK_ID,
		width - MARGIN - BUTTON_WIDTH * 2 - 6,
		buttons_y,
		BUTTON_WIDTH,
		HEIGHT + 2,
	);
	b.item(
		Class::Button,
		WS_TABSTOP | BS_PUSHBUTTON,
		"Cancel",
		CANCEL_ID,
		width - MARGIN - BUTTON_WIDTH,
		buttons_y,
		BUTTON_WIDTH,
		HEIGHT + 2,
	);

	let count = b.items.to_le_bytes();
	b.bytes[8] = count[0];
	b.bytes[9] = count[1];
	Template {
		bytes: b.bytes,
		ids: b.ids,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_saver_gets_a_template_with_unique_ids() {
		for kind in SaverKind::ALL {
			let t = build(kind);
			let mut ids = t.ids.clone();
			ids.sort_unstable();
			let before = ids.len();
			ids.dedup();
			assert_eq!(before, ids.len(), "{kind:?} repeats a control identifier");
			// One control per field (a mask adds one per bit), the text editor and three buttons.
			let controls: usize = fields(kind)
				.map(|f| match f.control {
					Control::Mask(bits) => bits.len(),
					_ => 1,
				})
				.sum();
			let expected = controls + usize::from(text_field(kind).is_some()) + 3;
			assert_eq!(t.ids.len(), expected);
			let count = u16::from_le_bytes([t.bytes[8], t.bytes[9]]) as usize;
			assert!(count >= expected, "the header counts every item");
		}
	}

	#[test]
	fn the_header_names_the_saver() {
		let t = build(SaverKind::Ink);
		let title_bytes: Vec<u8> = "Afterglow Ink Settings".encode_utf16().flat_map(u16::to_le_bytes).collect();
		assert!(t.bytes.windows(title_bytes.len()).any(|w| w == title_bytes.as_slice()));
		assert_eq!(t.bytes.len() % 2, 0);
	}

	#[test]
	fn the_dialog_fits_an_800_by_600_screen() {
		for kind in SaverKind::ALL {
			let t = build(kind);
			let width = i16::from_le_bytes([t.bytes[14], t.bytes[15]]);
			let height = i16::from_le_bytes([t.bytes[16], t.bytes[17]]);
			// Dialog units are about 1.5 pixels across and 1.6 down at 96 dpi with this font; leave room for the caption and a taskbar.
			assert!(i32::from(width) * 3 / 2 <= 580, "{kind:?} is {width} wide");
			assert!(i32::from(height) * 8 / 5 <= 560, "{kind:?} is {height} tall");
		}
	}
}
