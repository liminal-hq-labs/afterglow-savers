// The Ghostwriter script: what Quill types, when it slips, and when the "Save the changes?" box visits. No drawing and no Windows calls.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Columns and rows on a page. Every line of the text bank fits one row.
pub const COLS: usize = 46;
pub const ROWS: usize = 8;

/// The text bank. A `~wrong~right~` mark is a word Quill types wrongly, underlines, and then corrects. The whole bank is a few
/// hundred bytes of the data segment; the settings can switch the typos off, and the marks then type the right word straight away.
static BANK: [&str; 16] = [
	"The ~peice~piece~ table keeps every edit.",
	"Quill 0.0.1: it types.",
	"DOS-37: the QUILLDOS.EXE budget is 416 KB.",
	"Undo is a ~tpae~tape~ that rewinds itself.",
	"Nothing is saved; all of it is remembered.",
	"A haiku about piece tables:",
	"  one file, two buffers, and a ~splcie~splice~;",
	"  cut, paste, begin again.",
	"Windows 3.1 at 640 x 480 was ~enogh~enough~.",
	"Ctrl+Z is a time machine with one button.",
	"The ~cusor~cursor~ is a small gold nib.",
	"Spell check ~wnats~wants~ you to be happy.",
	"QUILL(1): edits text and forgets nothing.",
	"v0.0.2: it types and ~apologises~apologizes~.",
	"No Lua was harmed making this DOS build.",
	"The page is only ever as long as you need it.",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
	Start,
	Prefix,
	Wrong,
	Squiggle,
	Suggest,
	Delete,
	Right,
	Suffix,
	EndLine,
	PageEnd,
	Visit,
	Clear,
}

/// The settings `CONTROL.INI` keeps and the config box edits.
#[derive(Clone, Copy)]
pub struct Settings {
	pub typos: bool,
	pub visit: bool,
	/// 0 slow, 1 normal, 2 fast.
	pub speed: u8,
}

/// One line of the bank split at its marks.
#[derive(Clone, Copy)]
struct Line {
	prefix: &'static [u8],
	wrong: &'static [u8],
	right: &'static [u8],
	suffix: &'static [u8],
}

fn split(text: &'static str) -> Line {
	let bytes = text.as_bytes();
	let mut marks = [0usize; 3];
	let mut found = 0;
	for (i, b) in bytes.iter().enumerate() {
		if *b == b'~' && found < 3 {
			marks[found] = i;
			found += 1;
		}
	}
	if found < 3 {
		return Line {
			prefix: bytes,
			wrong: &[],
			right: &[],
			suffix: &[],
		};
	}
	Line {
		prefix: &bytes[..marks[0]],
		wrong: &bytes[marks[0] + 1..marks[1]],
		right: &bytes[marks[1] + 1..marks[2]],
		suffix: &bytes[marks[2] + 1..],
	}
}

pub struct Scene {
	pub settings: Settings,
	rng: u32,
	pub rows: [[u8; COLS]; ROWS],
	pub lens: [u8; ROWS],
	pub row: usize,
	phase: Phase,
	pos: usize,
	wait: u32,
	line: Line,
	next_line: usize,
	pub pages: u32,
	/// A column and a length: the red squiggle under the wrong word, or none when the length is 0.
	pub squiggle_col: usize,
	pub squiggle_len: usize,
	/// The spelling suggestion is open under the squiggle.
	pub suggest: bool,
	/// The "Save the changes?" box is showing.
	pub visit: bool,
	/// Whether Cancel's focus rectangle is drawn right now; it blinks.
	pub focus_on: bool,
	/// The nib is drawn right now; it blinks while Quill waits.
	pub nib_on: bool,
	pub tick: u32,
	pub dirty: bool,
}

impl Scene {
	pub const fn new() -> Scene {
		Scene {
			settings: Settings {
				typos: true,
				visit: true,
				speed: 1,
			},
			rng: 1,
			rows: [[b' '; COLS]; ROWS],
			lens: [0; ROWS],
			row: 0,
			phase: Phase::Start,
			pos: 0,
			wait: 4,
			line: Line {
				prefix: &[],
				wrong: &[],
				right: &[],
				suffix: &[],
			},
			next_line: 0,
			pages: 0,
			squiggle_col: 0,
			squiggle_len: 0,
			suggest: false,
			visit: false,
			focus_on: true,
			nib_on: true,
			tick: 0,
			dirty: true,
		}
	}

	pub fn seed(&mut self, seed: u32) {
		self.rng = seed | 1;
		self.next_line = (seed as usize >> 4) % BANK.len();
	}

	fn random(&mut self) -> u32 {
		self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12_345);
		self.rng >> 16
	}

	/// Scales a pause in ticks by the typing speed: slow takes one and a half times as long, fast half as long.
	fn pause(&self, ticks: u32) -> u32 {
		let scaled = ticks * [3, 2, 1][self.settings.speed.min(2) as usize] / 2;
		scaled.max(1)
	}

	/// How long Quill waits after typing `byte`: a typist's uneven rhythm, longer after a space or a full stop.
	fn rhythm(&mut self, byte: u8) -> u32 {
		let span = [5, 3, 1][self.settings.speed.min(2) as usize];
		let mut ticks = self.random() % span;
		if byte == b' ' || byte == b'.' || byte == b',' || byte == b';' {
			ticks += 1 + self.random() % span;
		}
		if self.settings.speed == 2 {
			ticks / 2
		} else {
			ticks
		}
	}

	fn type_byte(&mut self, byte: u8) {
		let len = self.lens[self.row] as usize;
		if len < COLS {
			self.rows[self.row][len] = byte;
			self.lens[self.row] = (len + 1) as u8;
		}
		self.wait = self.rhythm(byte);
		self.dirty = true;
		self.nib_on = true;
	}

	fn backspace(&mut self) {
		let len = self.lens[self.row] as usize;
		if len > 0 {
			self.rows[self.row][len - 1] = b' ';
			self.lens[self.row] = (len - 1) as u8;
		}
		self.dirty = true;
	}

	/// The word the suggestion box offers.
	pub fn suggestion(&self) -> &'static [u8] {
		self.line.right
	}

	/// Moves the script on by one tick of the timer.
	pub fn step(&mut self) {
		self.tick = self.tick.wrapping_add(1);
		let blink = self.tick / 7;
		let focus = blink.is_multiple_of(2);
		if self.visit && focus != self.focus_on {
			self.focus_on = focus;
			self.dirty = true;
		}
		let nib = self.wait == 0 || (self.tick / 6).is_multiple_of(2);
		if nib != self.nib_on {
			self.nib_on = nib;
			self.dirty = true;
		}
		if self.wait > 0 {
			self.wait -= 1;
			return;
		}
		match self.phase {
			Phase::Start => {
				self.line = split(BANK[self.next_line]);
				self.next_line = (self.next_line + 1) % BANK.len();
				self.phase = Phase::Prefix;
				self.pos = 0;
			}
			Phase::Prefix => {
				if self.pos < self.line.prefix.len() {
					let byte = self.line.prefix[self.pos];
					self.pos += 1;
					self.type_byte(byte);
				} else {
					self.pos = 0;
					self.phase = if self.settings.typos && !self.line.wrong.is_empty() {
						Phase::Wrong
					} else {
						Phase::Right
					};
				}
			}
			Phase::Wrong => {
				if self.pos < self.line.wrong.len() {
					let byte = self.line.wrong[self.pos];
					self.pos += 1;
					self.type_byte(byte);
				} else {
					self.phase = Phase::Squiggle;
					self.wait = self.pause(6);
				}
			}
			Phase::Squiggle => {
				self.squiggle_len = self.line.wrong.len();
				self.squiggle_col = self.lens[self.row] as usize - self.squiggle_len;
				self.phase = Phase::Suggest;
				self.wait = self.pause(8);
				self.dirty = true;
			}
			Phase::Suggest => {
				self.suggest = true;
				self.phase = Phase::Delete;
				self.pos = self.line.wrong.len();
				self.wait = self.pause(16);
				self.dirty = true;
			}
			Phase::Delete => {
				self.suggest = false;
				if self.pos > 0 {
					self.pos -= 1;
					self.backspace();
					self.wait = self.pause(1);
				} else {
					self.squiggle_len = 0;
					self.phase = Phase::Right;
					self.wait = self.pause(3);
					self.dirty = true;
				}
			}
			Phase::Right => {
				if self.pos < self.line.right.len() {
					let byte = self.line.right[self.pos];
					self.pos += 1;
					self.type_byte(byte);
				} else {
					self.pos = 0;
					self.phase = Phase::Suffix;
				}
			}
			Phase::Suffix => {
				if self.pos < self.line.suffix.len() {
					let byte = self.line.suffix[self.pos];
					self.pos += 1;
					self.type_byte(byte);
				} else {
					self.phase = Phase::EndLine;
					self.wait = self.pause(8);
				}
			}
			Phase::EndLine => {
				self.row += 1;
				self.dirty = true;
				if self.row >= ROWS {
					self.row = ROWS - 1;
					self.phase = Phase::PageEnd;
					self.wait = self.pause(24);
				} else {
					self.phase = Phase::Start;
				}
			}
			Phase::PageEnd => {
				if self.settings.visit && self.pages.is_multiple_of(2) {
					self.visit = true;
					self.focus_on = true;
					self.phase = Phase::Visit;
					self.wait = self.pause(70);
				} else {
					self.phase = Phase::Clear;
				}
				self.dirty = true;
			}
			Phase::Visit => {
				self.visit = false;
				self.phase = Phase::Clear;
				self.wait = self.pause(6);
				self.dirty = true;
			}
			Phase::Clear => {
				self.rows = [[b' '; COLS]; ROWS];
				self.lens = [0; ROWS];
				self.row = 0;
				self.pages += 1;
				self.phase = Phase::Start;
				self.wait = self.pause(6);
				self.dirty = true;
			}
		}
	}
}
