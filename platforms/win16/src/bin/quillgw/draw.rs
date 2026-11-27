// Quill's window in 16-bit GDI: a bitmap in memory that the saver draws once per change and copies to the screen as it drifts.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::scene::{ROWS, Scene};
use w16::*;

/// The window's size, and the bitmap's: a margin round it that the drift uncovers as the bitmap moves, and room for a shadow.
pub const WINDOW_W: i32 = 416;
pub const WINDOW_H: i32 = 272;
const MARGIN: i32 = 8;
pub const BITMAP_W: i32 = WINDOW_W + MARGIN + 16;
pub const BITMAP_H: i32 = WINDOW_H + MARGIN + 16;

const TRANSPARENT: u32 = 1;
const NULL_BRUSH: u32 = 5;
const ANSI_VAR_FONT: u32 = 12;
const SYSTEM_FONT: u32 = 13;
const SYSTEM_FIXED_FONT: u32 = 16;
const BITSPIXEL: u32 = 12;
const PLANES: u32 = 14;
const SRCCOPY: u32 = 0x00CC_0020;

/// Brush indexes.
const DESK: usize = 0;
const SHADOW: usize = 1;
const FACE: usize = 2;
const WHITE: usize = 3;
const DARK: usize = 4;
const BLACK: usize = 5;
const NAVY: usize = 6;
const GOLD: usize = 7;
const CREAM: usize = 8;
const BRUSHES: usize = 9;

const fn rgb(red: u32, green: u32, blue: u32) -> u32 {
	red | (green << 8) | (blue << 16)
}

/// Quill's navy and gold where the screen has the colours for them, and the sixteen pure colours of a VGA where it has not.
const RICH: [u32; BRUSHES] = [
	rgb(6, 8, 22),
	rgb(2, 3, 10),
	rgb(192, 192, 192),
	rgb(255, 255, 255),
	rgb(128, 128, 128),
	rgb(0, 0, 0),
	rgb(10, 24, 96),
	rgb(240, 168, 48),
	rgb(251, 247, 234),
];
const PURE: [u32; BRUSHES] = [
	rgb(0, 0, 0),
	rgb(128, 128, 128),
	rgb(192, 192, 192),
	rgb(255, 255, 255),
	rgb(128, 128, 128),
	rgb(0, 0, 0),
	rgb(0, 0, 128),
	rgb(128, 128, 0),
	rgb(255, 255, 255),
];

/// Every GDI object the scene uses, made once and deleted when the saver ends.
pub struct Gdi {
	pub memory_dc: u32,
	bitmap: u32,
	old_bitmap: u32,
	brushes: [u32; BRUSHES],
	pen_red: u32,
	pen_dots: u32,
	pub colours: [u32; BRUSHES],
	pub rich: bool,
	cell_w: i32,
}

impl Gdi {
	pub const fn empty() -> Gdi {
		Gdi {
			memory_dc: 0,
			bitmap: 0,
			old_bitmap: 0,
			brushes: [0; BRUSHES],
			pen_red: 0,
			pen_dots: 0,
			colours: RICH,
			rich: true,
			cell_w: 8,
		}
	}

	/// Makes the memory bitmap and the brushes against the screen's DC. Returns false when GDI has no room for them.
	pub fn create(&mut self, screen_dc: u32) -> bool {
		unsafe {
			let depth = GetDeviceCaps(screen_dc, BITSPIXEL) * GetDeviceCaps(screen_dc, PLANES);
			self.rich = depth >= 8;
			self.colours = if self.rich { RICH } else { PURE };
			for i in 0..BRUSHES {
				self.brushes[i] = CreateSolidBrush(self.colours[i]);
			}
			self.pen_red = CreatePen(
				0,
				1,
				if self.rich {
					rgb(220, 24, 24)
				} else {
					rgb(255, 0, 0)
				},
			);
			self.pen_dots = CreatePen(2, 1, 0);
			self.memory_dc = CreateCompatibleDC(screen_dc);
			self.bitmap = CreateCompatibleBitmap(screen_dc, BITMAP_W as u32, BITMAP_H as u32);
			if self.memory_dc == 0
				|| self.bitmap == 0
				|| self.brushes[BRUSHES - 1] == 0
				|| self.pen_dots == 0
			{
				return false;
			}
			self.old_bitmap = SelectObject(self.memory_dc, self.bitmap);
			SetBkMode(self.memory_dc, TRANSPARENT);
			SelectObject(self.memory_dc, GetStockObject(SYSTEM_FIXED_FONT));
			let size = GetTextExtent(self.memory_dc, cs!("M"), 1);
			self.cell_w = (size & 0xFFFF).clamp(6, 12) as i32;
			true
		}
	}

	/// Deletes everything `create` made.
	pub fn destroy(&mut self) {
		unsafe {
			if self.memory_dc != 0 {
				if self.old_bitmap != 0 {
					SelectObject(self.memory_dc, self.old_bitmap);
				}
				DeleteDC(self.memory_dc);
			}
			for brush in self.brushes {
				if brush != 0 {
					DeleteObject(brush);
				}
			}
			for object in [self.bitmap, self.pen_red, self.pen_dots] {
				if object != 0 {
					DeleteObject(object);
				}
			}
		}
		*self = Gdi::empty();
	}

	/// Paints a piece of the screen in the desk colour.
	pub fn desk(&self, screen_dc: u32, left: i32, top: i32, right: i32, bottom: i32) {
		let rect = [left as i16, top as i16, right as i16, bottom as i16];
		unsafe {
			FillRect(screen_dc, rect.as_ptr() as usize as u32, self.brushes[DESK]);
		}
	}

	fn fill(&self, left: i32, top: i32, right: i32, bottom: i32, brush: usize) {
		let rect = [left as i16, top as i16, right as i16, bottom as i16];
		unsafe {
			FillRect(
				self.memory_dc,
				rect.as_ptr() as usize as u32,
				self.brushes[brush],
			);
		}
	}

	/// A window-relative box, which sits MARGIN in from the bitmap's corner.
	fn boxed(&self, left: i32, top: i32, right: i32, bottom: i32, brush: usize) {
		self.fill(
			left + MARGIN,
			top + MARGIN,
			right + MARGIN,
			bottom + MARGIN,
			brush,
		);
	}

	/// A one-pixel bevel inside the box: light on the top and left and dark on the bottom and right, or the other way round.
	fn bevel(&self, left: i32, top: i32, right: i32, bottom: i32, raised: bool) {
		let (light, shade) = if raised { (WHITE, DARK) } else { (DARK, WHITE) };
		self.boxed(left, top, right - 1, top + 1, light);
		self.boxed(left, top, left + 1, bottom - 1, light);
		self.boxed(left, bottom - 1, right, bottom, shade);
		self.boxed(right - 1, top, right, bottom, shade);
	}

	fn font(&self, stock: u32) {
		unsafe {
			SelectObject(self.memory_dc, GetStockObject(stock));
		}
	}

	fn text(&self, x: i32, y: i32, bytes: &[u8], colour: usize) {
		unsafe {
			SetTextColor(self.memory_dc, self.colours[colour]);
			TextOut(
				self.memory_dc,
				(x + MARGIN) as u32,
				(y + MARGIN) as u32,
				bytes.as_ptr() as usize as u32,
				bytes.len() as u32,
			);
		}
	}

	fn width(&self, bytes: &[u8]) -> i32 {
		unsafe {
			(GetTextExtent(
				self.memory_dc,
				bytes.as_ptr() as usize as u32,
				bytes.len() as u32,
			) & 0xFFFF) as i32
		}
	}

	/// A button: black frame, raised face, label centred.
	fn button(&self, left: i32, top: i32, width: i32, height: i32, label: &[u8]) {
		self.boxed(left, top, left + width, top + height, BLACK);
		self.boxed(left + 1, top + 1, left + width - 1, top + height - 1, FACE);
		self.bevel(left + 1, top + 1, left + width - 1, top + height - 1, true);
		let x = left + (width - self.width(label)) / 2;
		self.text(x, top + (height - 13) / 2, label, BLACK);
	}

	/// Draws the whole window, as the scene stands, into the memory bitmap.
	pub fn render(&self, scene: &Scene) {
		self.fill(0, 0, BITMAP_W, BITMAP_H, DESK);
		self.boxed(6, 6, WINDOW_W + 6, WINDOW_H + 6, SHADOW);
		self.boxed(0, 0, WINDOW_W, WINDOW_H, BLACK);
		self.boxed(1, 1, WINDOW_W - 1, WINDOW_H - 1, FACE);
		self.title_bar();
		self.menu_bar();
		self.tool_bar();
		self.ruler();
		self.page(scene);
		self.status_bar(scene);
		if scene.visit {
			self.save_box(scene.focus_on);
		}
	}

	fn title_bar(&self) {
		self.boxed(1, 1, WINDOW_W - 1, 19, NAVY);
		self.boxed(2, 2, 18, 18, FACE);
		self.bevel(2, 2, 18, 18, true);
		self.boxed(5, 9, 15, 12, DARK);
		self.boxed(WINDOW_W - 36, 2, WINDOW_W - 19, 18, FACE);
		self.bevel(WINDOW_W - 36, 2, WINDOW_W - 19, 18, true);
		self.boxed(WINDOW_W - 18, 2, WINDOW_W - 2, 18, FACE);
		self.bevel(WINDOW_W - 18, 2, WINDOW_W - 2, 18, true);
		self.font(SYSTEM_FONT);
		let caption = b"Quill - Untitled";
		self.text((WINDOW_W - self.width(caption)) / 2, 2, caption, WHITE);
	}

	fn menu_bar(&self) {
		self.font(ANSI_VAR_FONT);
		let mut x = 10;
		for item in [
			&b"File"[..],
			b"Edit",
			b"View",
			b"Insert",
			b"Format",
			b"Help",
		] {
			self.text(x, 22, item, BLACK);
			x += self.width(item) + 18;
		}
		self.boxed(1, 37, WINDOW_W - 1, 38, DARK);
	}

	fn tool_bar(&self) {
		self.font(SYSTEM_FONT);
		let mut x = 8;
		for label in [&b"B"[..], b"I", b"U"] {
			self.boxed(x, 41, x + 22, 63, BLACK);
			self.boxed(x + 1, 42, x + 21, 62, FACE);
			self.bevel(x + 1, 42, x + 21, 62, true);
			self.text(x + 7, 45, label, BLACK);
			x += 26;
		}
		self.boxed(x + 6, 43, x + 8, 61, DARK);
		self.boxed(x + 8, 43, x + 9, 61, WHITE);
		self.boxed(x + 14, 48, x + 14 + 38, 56, GOLD);
	}

	fn ruler(&self) {
		self.boxed(1, 66, WINDOW_W - 1, 80, WHITE);
		self.boxed(1, 80, WINDOW_W - 1, 81, DARK);
		let mut x = 16;
		while x < WINDOW_W - 16 {
			let tall = (x - 16) % 64 == 0;
			self.boxed(x, if tall { 70 } else { 73 }, x + 1, 80, BLACK);
			x += 8;
		}
		for stop in [16 + 64, 16 + 160, 16 + 256] {
			unsafe {
				let triangle = [
					(stop - 4 + MARGIN) as i16,
					(66 + MARGIN) as i16,
					(stop + 4 + MARGIN) as i16,
					(66 + MARGIN) as i16,
					(stop + MARGIN) as i16,
					(75 + MARGIN) as i16,
				];
				SelectObject(self.memory_dc, self.brushes[GOLD]);
				SelectObject(self.memory_dc, GetStockObject(7));
				Polygon(self.memory_dc, triangle.as_ptr() as usize as u32, 3);
			}
		}
	}

	fn page(&self, scene: &Scene) {
		let top = 82;
		self.boxed(1, top, WINDOW_W - 1, WINDOW_H - 21, CREAM);
		self.font(SYSTEM_FIXED_FONT);
		let cell = self.cell_w;
		for row in 0..ROWS {
			let len = scene.lens[row] as usize;
			if len > 0 {
				self.text(
					16,
					top + 4 + row as i32 * 19,
					&scene.rows[row][..len],
					BLACK,
				);
			}
		}
		let row_top = top + 4 + scene.row as i32 * 19;
		let caret = 16 + scene.lens[scene.row] as i32 * cell;
		if scene.squiggle_len > 0 {
			self.squiggle(
				16 + scene.squiggle_col as i32 * cell,
				row_top + 16,
				scene.squiggle_len as i32 * cell,
			);
		}
		if scene.suggest {
			self.suggestion(16 + scene.squiggle_col as i32 * cell, row_top, scene);
		}
		if scene.nib_on && !scene.visit {
			self.boxed(caret, row_top - 1, caret + 3, row_top + 16, GOLD);
			self.boxed(caret + 1, row_top + 16, caret + 2, row_top + 18, GOLD);
		}
	}

	fn squiggle(&self, x: i32, y: i32, width: i32) {
		unsafe {
			SelectObject(self.memory_dc, self.pen_red);
			let mut at = x;
			MoveTo(self.memory_dc, (at + MARGIN) as u32, (y + MARGIN) as u32);
			let mut high = false;
			while at < x + width {
				at += 2;
				LineTo(
					self.memory_dc,
					(at + MARGIN) as u32,
					(y + MARGIN + if high { 0 } else { 2 }) as u32,
				);
				high = !high;
			}
		}
	}

	fn suggestion(&self, x: i32, row_top: i32, scene: &Scene) {
		let height = 3 * 15 + 4;
		let top = if scene.row >= 4 {
			row_top - height - 2
		} else {
			row_top + 20
		};
		let width = 104;
		self.boxed(x, top, x + width, top + height, BLACK);
		self.boxed(x + 1, top + 1, x + width - 1, top + height - 1, WHITE);
		self.font(ANSI_VAR_FONT);
		self.boxed(x + 2, top + 2, x + width - 2, top + 17, NAVY);
		self.text(x + 8, top + 3, scene.suggestion(), WHITE);
		self.text(x + 8, top + 18, b"Ignore", BLACK);
		self.text(x + 8, top + 33, b"Add to dictionary", BLACK);
	}

	fn status_bar(&self, scene: &Scene) {
		let top = WINDOW_H - 20;
		self.boxed(1, top, WINDOW_W - 1, WINDOW_H - 1, FACE);
		self.font(ANSI_VAR_FONT);
		let panels = [(6, 150), (160, 300), (308, WINDOW_W - 8)];
		for (left, right) in panels {
			self.boxed(left, top + 3, right, top + 17, FACE);
			self.bevel(left, top + 3, right, top + 17, false);
		}
		let mut line = [0u8; 40];
		let mut len = 0;
		put(&mut line, &mut len, b"Pg ");
		put_number(&mut line, &mut len, scene.pages % 9 + 1);
		put(&mut line, &mut len, b"  Ln ");
		put_number(&mut line, &mut len, scene.row as u32 + 1);
		put(&mut line, &mut len, b"  Col ");
		put_number(&mut line, &mut len, scene.lens[scene.row] as u32 + 1);
		self.text(10, top + 4, &line[..len], BLACK);
		self.text(165, top + 4, b"Times New Roman 12", BLACK);
		self.text(314, top + 4, b"Modified", BLACK);
	}

	/// The "Save the changes?" box that now and then drifts into view; the saver never clicks it.
	fn save_box(&self, focus: bool) {
		let (left, top, width, height) = (76, 94, 264, 92);
		self.boxed(left, top, left + width, top + height, BLACK);
		self.boxed(left + 1, top + 1, left + width - 1, top + height - 1, FACE);
		self.bevel(left + 1, top + 1, left + width - 1, top + height - 1, true);
		self.boxed(left + 3, top + 3, left + width - 3, top + 19, NAVY);
		self.font(SYSTEM_FONT);
		self.text(left + 8, top + 3, b"Quill", WHITE);
		self.font(ANSI_VAR_FONT);
		self.text(left + 16, top + 30, b"Save the changes to Untitled", BLACK);
		self.text(left + 16, top + 44, b"before Quill closes?", BLACK);
		let labels = [&b"Yes"[..], b"No", b"Cancel"];
		for (i, label) in labels.iter().enumerate() {
			let x = left + 16 + i as i32 * 81;
			self.button(x, top + 62, 70, 22, label);
			if i == 2 && focus {
				self.focus_rect(x + 4, top + 66, x + 66, top + 80);
			}
		}
	}

	fn focus_rect(&self, left: i32, top: i32, right: i32, bottom: i32) {
		unsafe {
			SelectObject(self.memory_dc, self.pen_dots);
			SelectObject(self.memory_dc, GetStockObject(NULL_BRUSH));
			Rectangle(
				self.memory_dc,
				(left + MARGIN) as u32,
				(top + MARGIN) as u32,
				(right + MARGIN) as u32,
				(bottom + MARGIN) as u32,
			);
		}
	}

	/// Copies the bitmap to the screen with its top-left corner at `x`, `y`.
	pub fn blit(&self, screen_dc: u32, x: i32, y: i32) {
		unsafe {
			BitBlt(
				screen_dc,
				x as u32,
				y as u32,
				BITMAP_W as u32,
				BITMAP_H as u32,
				self.memory_dc,
				0,
				0,
				SRCCOPY,
			);
		}
	}
}

fn put(buffer: &mut [u8], len: &mut usize, bytes: &[u8]) {
	for byte in bytes {
		if *len < buffer.len() {
			buffer[*len] = *byte;
			*len += 1;
		}
	}
}

fn put_number(buffer: &mut [u8], len: &mut usize, mut value: u32) {
	let mut digits = [0u8; 10];
	let mut count = 0;
	loop {
		digits[count] = b'0' + (value % 10) as u8;
		count += 1;
		value /= 10;
		if value == 0 {
			break;
		}
	}
	while count > 0 {
		count -= 1;
		put(buffer, len, &digits[count..count + 1]);
	}
}
