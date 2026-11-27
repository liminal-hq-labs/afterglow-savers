// The Afterglow settings dialog as an in-memory `DLGTEMPLATE` (SAV-D26, SAV-D39): a banner, palette swatches, a live preview pane and
// the same fields as the plain dialog, with the same control identifiers so one reader and one writer serve both. Nothing here calls
// Windows, so the host tests the layout.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::vec::Vec;

use savers_core::settings::{Control, Field, Key, SaverKind, fields, text_field};

use crate::template::{
	BS_DEFPUSHBUTTON, BS_PUSHBUTTON, BUTTON_WIDTH, Builder, CANCEL_ID, Class, DEFAULTS_ID, ES_AUTOHSCROLL, ES_AUTOVSCROLL,
	ES_MULTILINE, ES_NUMBER, ES_WANTRETURN, GAP, HALF, HEIGHT, MARGIN, OK_ID, ROW, TEXT_ID, Template, WS_BORDER, WS_GROUP,
	WS_TABSTOP, WS_VSCROLL, field_id, mask_id, place, run_height, section,
};

/// The owner-drawn banner across the top and the preview pane's identifiers.
pub const BANNER_ID: u16 = 903;
pub const PREVIEW_ID: u16 = 902;
/// The "New seed" button.
pub const NEW_SEED_ID: u16 = 101;
/// The owner-drawn group titles: `HEADER_BASE` for Look, then Quality, Finish and Scene.
pub const HEADER_BASE: u16 = 910;
pub const HEADERS: [&str; 4] = ["Look", "Quality", "Finish", "Scene"];
/// The banner's height, in dialog units (about 64 pixels), and the preview pane's.
pub const BANNER_HEIGHT: i16 = 40;
const PREVIEW_HEIGHT: i16 = 84;
/// A group title's height and a palette swatch's.
const HEADER_HEIGHT: i16 = 12;
const SWATCH_HEIGHT: i16 = 30;
const SWATCH_GAP: i16 = 2;
const TEXT_HEIGHT: i16 = 26;
const SS_OWNERDRAW: u32 = 0x0D;
const BS_OWNERDRAW: u32 = 0x0B;

/// The identifier of palette swatch `n` of the palette field at `index`.
pub fn swatch_id(index: usize, n: usize) -> u16 {
	mask_id(index, n)
}

/// A group title: an owner-drawn static carrying its words as its window text.
fn header(b: &mut Builder, n: usize, x: i16, y: i16, w: i16) -> i16 {
	b.item(
		Class::Static,
		SS_OWNERDRAW,
		HEADERS[n],
		HEADER_BASE + n as u16,
		x,
		y,
		w,
		HEADER_HEIGHT,
	);
	y + HEADER_HEIGHT
}

/// The Finish group at (`x`, `y`); returns where it ends.
fn put_finish(b: &mut Builder, finish: &[(usize, Field)], x: i16, y: i16) -> i16 {
	let y = header(b, 2, x, y, HALF);
	place(b, finish, x, y, HALF);
	y + run_height(finish, HALF)
}

/// The scene's own group at (`x`, `y`), with its text editor; returns where it ends.
fn put_scene(b: &mut Builder, scene: &[(usize, Field)], text: Option<(&str, &str, usize)>, present: bool, x: i16, y: i16) -> i16 {
	if !present {
		return y;
	}
	let mut y = header(b, 3, x, y, HALF);
	place(b, scene, x, y, HALF);
	y += run_height(scene, HALF);
	if let Some((_, label, _)) = text {
		b.label(label, x, y, HALF);
		b.item(
			Class::Edit,
			WS_TABSTOP | WS_BORDER | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN,
			"",
			TEXT_ID,
			x,
			y + ROW,
			HALF,
			TEXT_HEIGHT,
		);
		y += ROW + TEXT_HEIGHT;
	}
	y
}

/// The dialog's size in dialog units: (width, height).
pub fn size(kind: SaverKind) -> (i16, i16) {
	let t = build(kind);
	(
		i16::from_le_bytes([t.bytes[14], t.bytes[15]]),
		i16::from_le_bytes([t.bytes[16], t.bytes[17]]),
	)
}

/// Builds the Afterglow settings dialog of a saver.
pub fn build(kind: SaverKind) -> Template {
	let all: Vec<(usize, Field)> = fields(kind).enumerate().collect();
	let pick = |n: u8| -> Vec<(usize, Field)> {
		all.iter()
			.copied()
			.filter(|(_, f)| section(f.key) == n && f.key != Key::Palette && f.key != Key::Seed)
			.collect()
	};
	let (look, quality, finish, scene) = (pick(0), pick(1), pick(2), pick(3));
	let palette = all.iter().copied().find(|(_, f)| f.key == Key::Palette);
	let seed = all.iter().copied().find(|(_, f)| f.key == Key::Seed);
	let text = text_field(kind);

	let width = MARGIN * 2 + HALF * 2 + GAP;
	let left_x = MARGIN;
	let right_x = MARGIN + HALF + GAP;
	let top = BANNER_HEIGHT + 6;

	let mut b = Builder {
		bytes: Vec::new(),
		items: 0,
		ids: Vec::new(),
		taken: 0,
		next_group: false,
	};
	// Defaults owns D and New seed owns N.
	b.taken |= 1u64 << 13;
	let new_seed = b.mnemonic("New seed");

	// The height is patched in once the columns are laid out; the header only needs its place in the template.
	let header_at = b.bytes.len();
	b.dialog_header(kind, width, 0);

	b.item(Class::Static, SS_OWNERDRAW, "", BANNER_ID, 0, 0, width, BANNER_HEIGHT);

	// Left: Look, Quality, and Finish or Scene, whichever balances the columns. Right: the preview, the seed, and the other one.
	let finish_height = HEADER_HEIGHT + run_height(&finish, HALF);
	let has_scene = !scene.is_empty() || text.is_some();
	let scene_height = if has_scene {
		HEADER_HEIGHT + run_height(&scene, HALF) + if text.is_some() { ROW + TEXT_HEIGHT } else { 0 }
	} else {
		0
	};
	let seed_height = if seed.is_some() { ROW + HEIGHT + 6 } else { 0 };
	let left_base = HEADER_HEIGHT
		+ palette.map_or(0, |_| ROW + SWATCH_HEIGHT + 2)
		+ run_height(&look, HALF)
		+ 6 + HEADER_HEIGHT
		+ run_height(&quality, HALF)
		+ 6;
	let right_base = PREVIEW_HEIGHT + 4 + seed_height;
	let finish_left =
		(left_base + finish_height).max(right_base + scene_height) <= (left_base + scene_height).max(right_base + finish_height);

	let mut y = top;
	y = header(&mut b, 0, left_x, y, HALF);
	if let Some((index, field)) = palette
		&& let Control::Choice(options) = field.control
	{
		b.label(field.label, left_x, y, HALF);
		y += ROW;
		let count = options.len() as i16;
		let tile = (HALF - SWATCH_GAP * (count - 1)) / count;
		for (n, (name, _)) in options.iter().enumerate() {
			let style = BS_OWNERDRAW | if n == 0 { WS_TABSTOP | WS_GROUP } else { 0 };
			b.item(
				Class::Button,
				style,
				name,
				swatch_id(index, n),
				left_x + n as i16 * (tile + SWATCH_GAP),
				y,
				tile,
				SWATCH_HEIGHT,
			);
		}
		y += SWATCH_HEIGHT + 2;
	}
	b.next_group = true;
	place(&mut b, &look, left_x, y, HALF);
	y += run_height(&look, HALF) + 6;
	y = header(&mut b, 1, left_x, y, HALF);
	place(&mut b, &quality, left_x, y, HALF);
	y += run_height(&quality, HALF) + 6;
	let left_end = if finish_left {
		put_finish(&mut b, &finish, left_x, y)
	} else {
		put_scene(&mut b, &scene, text, has_scene, left_x, y)
	};

	let mut y = top;
	b.item(Class::Static, SS_OWNERDRAW, "", PREVIEW_ID, right_x, y, HALF, PREVIEW_HEIGHT);
	y += PREVIEW_HEIGHT + 4;
	if let Some((index, field)) = seed {
		b.label(field.label, right_x, y, HALF);
		let edit_y = y + ROW;
		b.item(
			Class::Edit,
			WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL | ES_NUMBER,
			"",
			field_id(index),
			right_x,
			edit_y,
			70,
			HEIGHT,
		);
		b.item(
			Class::Button,
			WS_TABSTOP | BS_PUSHBUTTON,
			&new_seed,
			NEW_SEED_ID,
			right_x + 76,
			edit_y - 1,
			60,
			HEIGHT + 2,
		);
		y = edit_y + HEIGHT + 6;
	}
	let right_end = if finish_left {
		put_scene(&mut b, &scene, text, has_scene, right_x, y)
	} else {
		put_finish(&mut b, &finish, right_x, y)
	};

	let buttons_y = left_end.max(right_end) + 10;
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
	let template_height = buttons_y + HEIGHT + 2 + MARGIN;

	// The height is the 16-bit word at byte 16 of the header: style, extended style, count, x, y, cx, cy.
	let at = header_at + 16;
	b.bytes[at..at + 2].copy_from_slice(&(template_height as u16).to_le_bytes());
	b.finish()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn height_of(bytes: &[u8]) -> i16 {
		i16::from_le_bytes([bytes[16], bytes[17]])
	}

	#[test]
	fn every_saver_gets_a_template_with_unique_ids_and_every_field() {
		for kind in SaverKind::ALL {
			let t = build(kind);
			let mut ids = t.ids.clone();
			ids.sort_unstable();
			let before = ids.len();
			ids.dedup();
			assert_eq!(before, ids.len(), "{kind:?} repeats a control identifier");
			for (index, field) in fields(kind).enumerate() {
				match field.control {
					Control::Choice(options) if field.key == Key::Palette => {
						for n in 0..options.len() {
							assert!(t.ids.contains(&swatch_id(index, n)), "{kind:?} lacks swatch {n}");
						}
						assert!(!t.ids.contains(&field_id(index)), "{kind:?} keeps the palette combo box");
					}
					Control::Mask(bits) => {
						for bit in 0..bits.len() {
							assert!(t.ids.contains(&mask_id(index, bit)));
						}
					}
					_ => assert!(t.ids.contains(&field_id(index)), "{kind:?} lacks {}", field.name),
				}
			}
			for id in [
				BANNER_ID,
				PREVIEW_ID,
				NEW_SEED_ID,
				DEFAULTS_ID,
				OK_ID,
				CANCEL_ID,
				HEADER_BASE,
				HEADER_BASE + 1,
				HEADER_BASE + 2,
			] {
				assert!(t.ids.contains(&id), "{kind:?} lacks control {id}");
			}
			assert_eq!(t.ids.contains(&TEXT_ID), text_field(kind).is_some());
			let count = u16::from_le_bytes([t.bytes[8], t.bytes[9]]) as usize;
			assert!(count >= t.ids.len(), "the header counts every item");
		}
	}

	#[test]
	fn the_dialog_fits_a_1024_by_768_screen() {
		for kind in SaverKind::ALL {
			let t = build(kind);
			let width = i16::from_le_bytes([t.bytes[14], t.bytes[15]]);
			let height = height_of(&t.bytes);
			assert!(i32::from(width) * 3 / 2 <= 580, "{kind:?} is {width} wide");
			// The Afterglow dialog is for screens of 1024 by 768 and up; a smaller one gets the plain dialog (`dialog_v2::wanted`).
			assert!(i32::from(height) * 8 / 5 <= 680, "{kind:?} is {height} tall");
		}
	}
}
