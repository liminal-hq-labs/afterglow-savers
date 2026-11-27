// Quill Ghostwriter: a document that writes itself, in a Quill window drifting over a dark desk.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! An Easter egg for Quill (SAV-D20 to SAV-D22, docs/scenes.md). Each act is a script of edits against a real piece table (an
//! original buffer, an add buffer, and pieces pointing into them), replayed up to the act's current time, so any frame can be drawn
//! on its own. The window is drawn into its own frame each frame (chrome, ruler, the laid-out page, caret, squiggle, popup, piece
//! bars, dialog) and then placed on the desk with a slow rotation, zoom and drift, so it never parks.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{self, mix, mixf, scale};
use crate::font::{Atlas, Look};
use crate::frame::Frame;
use crate::math::{cos, fract, saturate, sin, smoothstep, sqrt};
use crate::palette::Palette;
use crate::rng::{Rng, hash32};
use crate::saver::palette_for;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Detail, Key, Settings};

/// The built-in text bank: what the document types when the `TextBank` setting is empty.
pub const DEFAULT_BANK: &str = "\
Dear reader, this document is writing itself.
DOS-37: the QUILLDOS.EXE budget is 416 KB.
v0.0.1: it types.
v0.0.2: it types, and now and then it spells.
QUILL(1) NAME quill - one word processor for DOS, Windows 3.1 and Windows 95.
SYNOPSIS quill [document.qld]
BUGS The caret is a nib. This is not a bug.
Two buffers, one text; pieces split where your words land; undo is a list.
It was a dark and stormy night, and the margins were justified.
Sphinx of black quartz, judge my vow.
Ctrl+S is a love language.
Cross-compiling to the past, one segment at a time.
Every paragraph deserves a second draft and a third margin.
The quick brown fox saved the changes to Untitled.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
	Typing,
	Reflow,
	TimeMachine,
	Eras,
	Lore,
	Dialog,
}

const ACTS: [(Act, f32); 6] = [
	(Act::Typing, 32.0),
	(Act::Reflow, 24.0),
	(Act::TimeMachine, 26.0),
	(Act::Eras, 27.0),
	(Act::Lore, 30.0),
	(Act::Dialog, 14.0),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Era {
	Dos,
	Win3,
	Modern,
}

/// One edit in a script.
#[derive(Clone, Copy, Debug)]
enum Edit {
	Type(char),
	Back,
}

/// A typo moment: the word's span (in characters of the typed text) and when its squiggle, popup and fix happen.
#[derive(Clone, Debug)]
struct Typo {
	start: usize,
	len: usize,
	squiggle: f32,
	popup: f32,
	fixed: f32,
	suggestions: [String; 3],
}

/// The piece table: text is the pieces in order, each a run of the original or the add buffer.
struct Pieces {
	orig: Vec<char>,
	add: Vec<char>,
	pieces: Vec<(bool, usize, usize)>,
}

impl Pieces {
	fn new(original: &str) -> Pieces {
		let orig: Vec<char> = original.chars().collect();
		let n = orig.len();
		Pieces {
			orig,
			add: Vec::new(),
			pieces: if n > 0 { vec![(false, 0, n)] } else { Vec::new() },
		}
	}

	fn len(&self) -> usize {
		self.pieces.iter().map(|p| p.2).sum()
	}

	/// Inserts at the end (the caret follows the typing), growing the last add piece when it is contiguous.
	fn push(&mut self, c: char) {
		self.add.push(c);
		let at = self.add.len() - 1;
		if let Some(last) = self.pieces.last_mut()
			&& last.0 && last.1 + last.2 == at
		{
			last.2 += 1;
			return;
		}
		self.pieces.push((true, at, 1));
	}

	/// Deletes the last character: the last piece shrinks, and goes when it is empty.
	fn pop(&mut self) {
		if let Some(last) = self.pieces.last_mut() {
			last.2 -= 1;
			if last.2 == 0 {
				self.pieces.pop();
			}
		}
	}

	fn text(&self) -> Vec<char> {
		let mut out = Vec::with_capacity(self.len());
		for &(add, start, len) in &self.pieces {
			let buf = if add { &self.add } else { &self.orig };
			out.extend_from_slice(&buf[start..start + len]);
		}
		out
	}
}

/// A script: edits with their times, typed onto a base text, and its typo, if any.
struct Script {
	base: String,
	edits: Vec<(f32, Edit)>,
	typo: Option<Typo>,
	end: f32,
}

impl Script {
	/// Types `lines` after `base` with a typist's rhythm; when `typo` is set, the first long word of the first line goes wrong,
	/// is squiggled, offered suggestions and fixed.
	fn typing(base: &str, lines: &[&str], start: f32, pace: f32, typo: bool, salt: u32) -> Script {
		let mut edits = Vec::new();
		let mut t = start;
		let mut typed = 0usize;
		let mut typo_moment = None;
		let mut planted = !typo;
		for (li, line) in lines.iter().enumerate() {
			let prefix = if base.is_empty() && li == 0 { "" } else { "\n" };
			for c in prefix.chars() {
				edits.push((t, Edit::Type(c)));
				typed += 1;
				t += 0.35 * pace;
			}
			let words: Vec<&str> = line.split(' ').collect();
			for (wi, word) in words.iter().enumerate() {
				let letters: Vec<char> = word.chars().collect();
				let wrong = !planted && letters.len() >= 5 && letters.iter().all(|c| c.is_ascii_alphabetic()) && wi > 0;
				let start_of_word = typed;
				let mut spelled = letters.clone();
				if wrong {
					// Two neighbouring letters swapped, as fast fingers do.
					let k = 1 + (hash32(salt ^ wi as u32) as usize % (letters.len() - 2));
					spelled.swap(k, k + 1);
				}
				for (ci, &c) in spelled.iter().enumerate() {
					edits.push((t, Edit::Type(c)));
					typed += 1;
					let h = hash32(salt ^ (typed as u32).wrapping_mul(0x9E37_79B9) ^ ci as u32);
					t += pace * (0.055 + (h & 255) as f32 / 255.0 * 0.11);
				}
				if wi + 1 < words.len() {
					edits.push((t, Edit::Type(' ')));
					typed += 1;
					t += pace * 0.14;
				}
				if wrong {
					planted = true;
					let squiggle = t + 0.2;
					let popup = squiggle + 1.3;
					let fixed = popup + 1.8;
					let right: String = letters.iter().collect();
					let mut near = right.clone();
					near.push('s');
					let swapped: String = spelled.iter().collect();
					typo_moment = Some(Typo {
						start: start_of_word,
						len: letters.len(),
						squiggle,
						popup,
						fixed,
						suggestions: [right.clone(), near, swapped],
					});
					// The fix: back over the rest of the line typed so far and the word, then the right word, quickly.
					let back = typed - start_of_word;
					let mut ft = fixed;
					for _ in 0..back {
						edits.push((ft, Edit::Back));
						ft += 0.012;
					}
					for c in right.chars() {
						edits.push((ft, Edit::Type(c)));
						ft += 0.012;
					}
					if wi + 1 < words.len() {
						edits.push((ft, Edit::Type(' ')));
					}
					typed = start_of_word + right.chars().count() + usize::from(wi + 1 < words.len());
					t = ft + 0.5;
				}
				// Now and then the typist pauses to think.
				if hash32(salt ^ (wi as u32 * 31 + li as u32)).is_multiple_of(9) {
					t += pace * 0.7;
				}
			}
			t += pace * 0.6;
		}
		Script {
			base: String::from(base),
			edits,
			typo: typo_moment,
			end: t,
		}
	}

	/// The piece table after every edit up to `t`, and how many edits that was.
	fn at(&self, t: f32) -> (Pieces, usize) {
		let mut p = Pieces::new(&self.base);
		let mut n = 0;
		for &(at, e) in &self.edits {
			if at > t {
				break;
			}
			match e {
				Edit::Type(c) => p.push(c),
				Edit::Back => p.pop(),
			}
			n += 1;
		}
		(p, n)
	}
}

pub struct Ghostwriter {
	w: usize,
	h: usize,
	pal: Palette,
	bank: Vec<String>,
	acts: Vec<(Act, f32)>,
	cycle: f32,
	desk: Frame,
	win: Frame,
	other: Frame,
	smooth: Atlas,
	bitmap: Atlas,
	cells: Atlas,
	small: Atlas,
	tilt: bool,
	shadow: bool,
	seed: u32,
}

impl Ghostwriter {
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> Ghostwriter {
		let detail = settings.detail();
		let pal = palette_for(settings);
		let text = if settings.text.trim().is_empty() {
			DEFAULT_BANK
		} else {
			settings.text.as_str()
		};
		let mut bank: Vec<String> = text
			.lines()
			.map(|l| String::from(l.trim()))
			.filter(|l| !l.is_empty())
			.collect();
		if bank.is_empty() {
			bank.push(String::from("Quill"));
		}
		let mask = settings.get(Key::Acts);
		let mut acts: Vec<(Act, f32)> = ACTS
			.iter()
			.enumerate()
			.filter(|(i, _)| mask & (1 << i) != 0)
			.map(|(_, a)| *a)
			.collect();
		if acts.is_empty() {
			acts = ACTS.to_vec();
		}
		let cycle = acts.iter().map(|a| a.1).sum();
		let (ww, wh) = window_size(w, h);
		let text_h = (wh / 22).max(7);
		let mut desk = Frame::new(w, h);
		paint_desk(&mut desk, &pal, &mut Rng::new(seed as u64 ^ 0xDE5C));
		Ghostwriter {
			w,
			h,
			pal,
			bank,
			acts,
			cycle,
			desk,
			win: Frame::new(ww, wh),
			other: Frame::new(ww, wh),
			smooth: Atlas::new(text_h, 0.78, Look::Smooth),
			bitmap: Atlas::new(text_h, 0.7, Look::Bitmap),
			cells: Atlas::new(text_h, 0.75, Look::Cells),
			small: Atlas::new((text_h * 4 / 5).max(6), 0.62, Look::Smooth),
			tilt: detail >= Detail::Medium,
			shadow: detail >= Detail::High,
			seed,
		}
	}

	fn line(&self, i: usize) -> &str {
		&self.bank[i % self.bank.len()]
	}

	/// The act playing at time t, its local time, and which pass through the acts this is.
	fn act_at(&self, t: f32) -> (Act, f32, f32, usize) {
		let pass = (t / self.cycle) as usize;
		let mut local = t - pass as f32 * self.cycle;
		for &(act, len) in &self.acts {
			if local < len {
				return (act, local, len, pass);
			}
			local -= len;
		}
		let (act, len) = self.acts[self.acts.len() - 1];
		(act, len, len, pass)
	}
}

/// The window's size in the render: most of the height, a little wider than tall.
fn window_size(w: usize, h: usize) -> (usize, usize) {
	let wh = (h * 4 / 5).max(24);
	let ww = ((wh * 4 / 3).min(w * 9 / 10)).max(32);
	(ww, wh)
}

/// A dark desk: a deep gradient with a warm lamp pool in one corner and a faint grain.
fn paint_desk(f: &mut Frame, pal: &Palette, rng: &mut Rng) {
	let (w, h) = (f.w, f.h);
	let seed = rng.next_u32();
	let base = mixf(pal.n(900), pal.n(800), 0.5);
	let lamp = mixf(pal.n(800), pal.sun, 0.22);
	for y in 0..h {
		for x in 0..w {
			let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
			let d = sqrt((u - 0.15) * (u - 0.15) + (v - 0.1) * (v - 0.1));
			let grain = crate::noise::noise2(u * 3.0, v * 60.0, seed) * 0.04;
			let c = mixf(base, lamp, saturate(1.0 - d * 1.3) * 0.8);
			f.set(x, y, colour::shade(c, grain));
		}
	}
}

/// Colours for one era of the window.
struct Skin {
	frame: u32,
	title: u32,
	title_text: u32,
	menu: u32,
	menu_text: u32,
	page: u32,
	ink: u32,
	surround: u32,
	status: u32,
	status_text: u32,
	accent: u32,
}

fn skin(era: Era, pal: &Palette) -> Skin {
	let gold = pal.sun;
	match era {
		Era::Dos => Skin {
			frame: 0x0A1A8A,
			title: 0x2A3AAA,
			title_text: 0xFFFFFF,
			menu: 0x8A90A8,
			menu_text: 0x0A1230,
			page: 0x0A1A8A,
			ink: 0xE0E0E0,
			surround: 0x0A1A8A,
			status: 0x00A0A0,
			status_text: 0x0A1230,
			accent: 0xFFFF55,
		},
		Era::Win3 => Skin {
			frame: 0xC0C0C0,
			title: 0x000080,
			title_text: 0xFFFFFF,
			menu: 0xFFFFFF,
			menu_text: 0x000000,
			page: 0xFFFFFF,
			ink: 0x000000,
			surround: 0x808080,
			status: 0xC0C0C0,
			status_text: 0x000000,
			accent: 0x000080,
		},
		Era::Modern => {
			// The window keeps its look by day: a light palette's neutral ramp runs the other way, so its steps are mirrored.
			let n = |step: u32| if pal.light { pal.n(1000 - step) } else { pal.n(step) };
			Skin {
				frame: n(700),
				title: n(900),
				title_text: n(100),
				menu: n(300),
				menu_text: n(900),
				page: n(100),
				ink: mixf(n(900), 0x000000, 0.3),
				surround: n(500),
				status: n(300),
				status_text: n(900),
				accent: gold,
			}
		}
	}
}

/// What the page shows this frame.
struct PageState {
	text: Vec<char>,
	/// Characters of the base text before the typed ones, so the typo's span can be found.
	base_len: usize,
	typo: Option<(Typo, f32)>,
	modified: bool,
	caret_on: bool,
	margins: (f32, f32),
	justify: bool,
	pieces: Option<Vec<(bool, usize)>>,
	ghosts: Vec<(char, f32)>,
	dialog: f32,
	t: f32,
}

impl Ghostwriter {
	/// Draws the window into `win` for one era.
	fn draw_window(&self, win: &mut Frame, era: Era, st: &PageState) {
		let s = skin(era, &self.pal);
		let (ww, wh) = (win.w as i32, win.h as i32);
		let atlas = match era {
			Era::Dos => &self.cells,
			Era::Win3 => &self.bitmap,
			Era::Modern => &self.smooth,
		};
		let line_h = atlas.h as i32 + atlas.h as i32 / 3;
		win.fill(s.frame);
		// Title bar, with the Quill mark: a gold ring with a nib.
		let th = line_h + 4;
		fill(win, 2, 2, ww - 4, th, s.title);
		if era == Era::Modern {
			for y in 2..2 + th {
				let k = (y - 2) as f32 / th as f32;
				let c = mixf(s.title, mixf(s.title, s.accent, 0.12), 1.0 - k);
				fill(win, 2, y, ww - 4, 1, c);
			}
		}
		let mark = th / 2 - 1;
		ring(win, 4 + mark + 2, 2 + th / 2, mark as f32, s.accent);
		let title = match era {
			Era::Dos => "QUILL - UNTITLED.QLD",
			_ => "Quill - Untitled",
		};
		let tw = atlas.width(title) as i32;
		atlas.text(win, (ww - tw) / 2, 2 + (th - atlas.h as i32) / 2, title, s.title_text);
		// Menu bar.
		let my = 2 + th + 1;
		fill(win, 2, my, ww - 4, line_h, s.menu);
		let mut mx = 8;
		for item in ["File", "Edit", "View", "Insert", "Format", "Tools", "Help"] {
			mx += atlas.text(win, mx, my + (line_h - atlas.h as i32) / 2, item, s.menu_text) as i32 + atlas.h as i32;
		}
		// Toolbar: bold, italic and underline buttons (not in DOS), then the ruler.
		let mut ty = my + line_h + 1;
		if era != Era::Dos {
			fill(win, 2, ty, ww - 4, line_h + 4, s.frame);
			let b = line_h;
			for (i, label) in ["B", "I", "U"].iter().enumerate() {
				let bx = 8 + i as i32 * (b + 4);
				bevel(win, bx, ty + 2, b, b, s.menu, era);
				atlas.text(
					win,
					bx + (b - atlas.width(label) as i32) / 2,
					ty + 2 + (b - atlas.h as i32) / 2,
					label,
					s.menu_text,
				);
			}
			ty += line_h + 5;
		}
		let page_x = 2 + (ww / 14);
		let page_w = ww - 2 * page_x;
		let ruler_h = (line_h * 2 / 3).max(5);
		fill(
			win,
			2,
			ty,
			ww - 4,
			ruler_h,
			if era == Era::Dos { s.menu } else { mixf(s.menu, s.page, 0.5) },
		);
		let (ml, mr) = st.margins;
		let left = page_x + (page_w as f32 * ml) as i32;
		let right = page_x + page_w - (page_w as f32 * mr) as i32;
		let mut x = page_x;
		while x < page_x + page_w {
			let tick = if (x - page_x) % (page_w / 8).max(1) == 0 {
				ruler_h / 2
			} else {
				ruler_h / 5
			};
			fill(win, x, ty + ruler_h - tick, 1, tick, s.menu_text);
			x += (page_w / 40).max(2);
		}
		tri(win, left, ty + ruler_h - 1, ruler_h / 2 + 1, s.accent);
		tri(win, right, ty + ruler_h - 1, ruler_h / 2 + 1, s.accent);
		// The page.
		let status_h = line_h + 2;
		let py0 = ty + ruler_h + 2;
		let py1 = wh - 2 - status_h - 1;
		fill(win, 2, py0, ww - 4, py1 - py0, s.surround);
		fill(win, page_x, py0 + 4, page_w, py1 - py0 - 4, s.page);
		let (caret, ln, col) = self.set_text(
			win,
			atlas,
			&s,
			st,
			left + 6,
			right - 6,
			py0 + 4 + line_h / 2,
			py1 - line_h,
			line_h,
			era,
		);
		// The piece table, as bars along the foot of the page.
		if let Some(pieces) = &st.pieces {
			let total: usize = pieces.iter().map(|p| p.1).sum::<usize>().max(1);
			let by = py1 - line_h - 4;
			let mut bx = page_x + 6;
			let span = page_w - 12;
			fill(win, page_x + 4, by - 2, page_w - 8, line_h - 2, mixf(s.page, s.ink, 0.08));
			for (i, &(add, len)) in pieces.iter().enumerate() {
				let bw = ((len * span as usize) / total).max(2) as i32;
				let c = if add {
					self.pal.sun
				} else {
					mixf(self.pal.n(800), self.pal.hue(2 % self.pal.glow_n), 0.3)
				};
				fill(
					win,
					bx,
					by,
					bw - 1,
					line_h / 2,
					if i % 2 == 0 { c } else { colour::shade(c, 0.15) },
				);
				bx += bw;
			}
			self.small.text(win, page_x + 6, by + line_h / 2 + 1, "original", s.ink);
			let label = "add";
			let lw = self.small.width(label) as i32;
			self.small
				.text(win, page_x + page_w - 6 - lw, by + line_h / 2 + 1, label, s.accent);
		}
		// The caret: a gold nib on a stem, blinking when idle.
		if st.caret_on {
			let (cx, cy) = caret;
			let c = if era == Era::Modern { self.pal.sun } else { s.ink };
			fill(win, cx, cy, 1.max(atlas.h as i32 / 10), atlas.h as i32, c);
			if era == Era::Modern {
				let nib = (atlas.h as i32 / 3).max(2);
				for k in 0..nib {
					fill(win, cx - (nib - k) / 2, cy + atlas.h as i32 + k, (nib - k).max(1), 1, c);
				}
			}
		}
		// Letters flying back into the caret during the rewind.
		for &(c, age) in &st.ghosts {
			let k = saturate(age / 0.45);
			let gx = caret.0 + ((1.0 - k) * atlas.h as f32 * 6.0) as i32;
			let gy = caret.1 - ((1.0 - k) * (1.0 - k) * atlas.h as f32 * 2.0) as i32;
			atlas.draw(win, gx, gy, c, mixf(s.accent, s.page, k));
		}
		// The status bar.
		let sy = wh - 2 - status_h;
		fill(win, 2, sy, ww - 4, status_h, s.status);
		let font = match era {
			Era::Dos => "",
			Era::Win3 => "Tms Rmn 12",
			Era::Modern => "Times New Roman 12",
		};
		let mut status = String::new();
		push_fmt(&mut status, "Pg 1  Ln ", ln + 1);
		push_fmt(&mut status, "  Col ", col + 1);
		if !font.is_empty() {
			status.push_str("     ");
			status.push_str(font);
		}
		status.push_str(if st.modified { "     Modified" } else { "" });
		if era == Era::Dos {
			status.push_str("     INS");
		}
		atlas.text(win, 8, sy + (status_h - atlas.h as i32) / 2, &status, s.status_text);
		if st.dialog > 0.0 {
			self.draw_dialog(win, atlas, &s, era, st.dialog, st.t);
		}
	}

	/// Lays out and draws the text between `left` and `right`; returns the caret's position and its line and column.
	#[allow(clippy::too_many_arguments)]
	fn set_text(
		&self,
		win: &mut Frame,
		atlas: &Atlas,
		s: &Skin,
		st: &PageState,
		left: i32,
		right: i32,
		top: i32,
		bottom: i32,
		line_h: i32,
		era: Era,
	) -> ((i32, i32), usize, usize) {
		let text = &st.text;
		let width = (right - left).max(atlas.h as i32 * 4);
		let space = atlas.advance(' ') as i32;
		// Break into lines: words fill a line; a word that would overflow is hyphenated when it is long, or moved down.
		let mut lines: Vec<(Vec<Word>, bool)> = Vec::new();
		let mut current: Vec<Word> = Vec::new();
		let mut used = 0;
		let mut i = 0;
		while i < text.len() {
			if text[i] == '\n' {
				lines.push((core::mem::take(&mut current), true));
				used = 0;
				i += 1;
				continue;
			}
			if text[i] == ' ' {
				i += 1;
				continue;
			}
			let start = i;
			while i < text.len() && text[i] != ' ' && text[i] != '\n' {
				i += 1;
			}
			let ww: i32 = text[start..i].iter().map(|&c| atlas.advance(c) as i32).sum();
			let need = if current.is_empty() { ww } else { used + space + ww };
			if need <= width || current.is_empty() && ww > width {
				used = need;
				current.push((start, i, false));
				continue;
			}
			// Hyphenate a long word whose first part fits.
			let room = width - used - space;
			let len = i - start;
			if st.justify && len >= 7 && room > atlas.h as i32 * 2 {
				let mut cut = start;
				let mut acc = atlas.advance('-') as i32;
				while cut < i - 3 && acc + (atlas.advance(text[cut]) as i32) < room {
					acc += atlas.advance(text[cut]) as i32;
					cut += 1;
				}
				if cut - start >= 3 {
					current.push((start, cut, true));
					lines.push((core::mem::take(&mut current), false));
					let rest: i32 = text[cut..i].iter().map(|&c| atlas.advance(c) as i32).sum();
					current.push((cut, i, false));
					used = rest;
					continue;
				}
			}
			lines.push((core::mem::take(&mut current), false));
			current.push((start, i, false));
			used = ww;
		}
		lines.push((current, true));
		// Keep the last lines in view: scroll up when the page is full.
		let fits = ((bottom - top) / line_h).max(1) as usize;
		let first = lines.len().saturating_sub(fits);
		let mut caret = (left, top);
		let mut caret_line = 0;
		let mut caret_col = 0;
		let typo_span = st
			.typo
			.as_ref()
			.map(|(ty, _)| (st.base_len + ty.start, st.base_len + ty.start + ty.len));
		let mut typo_box: Option<(i32, i32, i32)> = None;
		for (li, (words, hard)) in lines.iter().enumerate() {
			let y = top + (li as i32 - first as i32) * line_h;
			let natural: i32 = words
				.iter()
				.map(|&(a, b, hy)| {
					text[a..b].iter().map(|&c| atlas.advance(c) as i32).sum::<i32>()
						+ if hy { atlas.advance('-') as i32 } else { 0 }
				})
				.sum::<i32>()
				+ space * (words.len() as i32 - 1).max(0);
			let gaps = (words.len() as i32 - 1).max(1);
			let extra = if st.justify && !hard && words.len() > 1 {
				(width - natural).max(0)
			} else {
				0
			};
			let mut x = left;
			for (wi, &(a, b, hy)) in words.iter().enumerate() {
				let word_x = x;
				for &c in &text[a..b] {
					if li >= first {
						atlas.draw(win, x, y, c, s.ink);
					}
					x += atlas.advance(c) as i32;
				}
				if hy {
					if li >= first {
						atlas.draw(win, x, y, '-', s.ink);
					}
					x += atlas.advance('-') as i32;
				}
				if let Some((ts, te)) = typo_span
					&& a <= ts && ts < b.max(a + 1)
					&& te <= b
				{
					typo_box = Some((word_x, x, y));
				}
				caret = (x, y);
				caret_line = li;
				caret_col = b - words[0].0;
				x += space
					+ if wi < words.len() - 1 {
						extra * (wi as i32 + 1) / gaps - extra * wi as i32 / gaps
					} else {
						0
					};
			}
			if words.is_empty() {
				caret = (left, y);
				caret_line = li;
				caret_col = 0;
			}
		}
		if !text.is_empty() && text[text.len() - 1] == ' ' {
			caret.0 += space;
		}
		// The squiggle under the typo, and its suggestion popup.
		if let (Some((ty, t)), Some((x0, x1, y))) = (&st.typo, typo_box)
			&& *t >= ty.squiggle
			&& *t < ty.fixed
		{
			let base = y + atlas.h as i32 + 1;
			let grow = saturate((t - ty.squiggle) / 0.4);
			let end = x0 + ((x1 - x0) as f32 * grow) as i32;
			for x in x0..end {
				let wob = (sin(x as f32 * 1.3 + t * 7.0) * 1.4) as i32;
				fill(win, x, base + wob, 1, 1, 0xE02040);
			}
			if *t >= ty.popup && era != Era::Dos {
				let show = saturate((t - ty.popup) / 0.25);
				let rows = 3;
				let pw = ty.suggestions.iter().map(|w| atlas.width(w) as i32).max().unwrap_or(10) + 16;
				let (px, py) = (x0, base + 4);
				let ph = rows * line_h + 6;
				fill(win, px + 3, py + 3, pw, ph, scale(s.ink, 60));
				fill(win, px, py, pw, ph, mixf(s.page, s.frame, 0.25));
				for (i, word) in ty.suggestions.iter().enumerate() {
					let ry = py + 3 + i as i32 * line_h;
					let selected = i == 0 && *t > ty.popup + 0.9;
					if selected {
						fill(win, px + 2, ry, pw - 4, line_h, s.title);
					}
					let col = if selected { s.title_text } else { s.ink };
					atlas.text(win, px + 8, ry + (line_h - atlas.h as i32) / 2, word, mixf(s.page, col, show));
				}
			}
		}
		(caret, caret_line, caret_col)
	}

	/// The visit from the dialog: "Save the changes to Untitled before Quill closes?" with Yes, No and Cancel; Cancel's focus
	/// rectangle blinks; nothing is ever clicked.
	fn draw_dialog(&self, win: &mut Frame, atlas: &Atlas, s: &Skin, era: Era, k: f32, t: f32) {
		let (ww, wh) = (win.w as i32, win.h as i32);
		let line_h = atlas.h as i32 + atlas.h as i32 / 3;
		let msg = "Save the changes to Untitled before Quill closes?";
		let dw = (atlas.width(msg) as i32 + line_h * 3).min(ww - 8);
		let dh = line_h * 6;
		let (dx, dy) = ((ww - dw) / 2, (wh - dh) / 2);
		let face = if era == Era::Modern { self.pal.n(200) } else { 0xC0C0C0 };
		let alpha = (saturate(k) * 256.0) as u32;
		blend_fill(win, dx + 4, dy + 5, dw, dh, 0x000000, alpha / 3);
		blend_fill(win, dx, dy, dw, dh, face, alpha);
		blend_fill(win, dx, dy, dw, line_h + 2, s.title, alpha);
		if k > 0.6 {
			atlas.text(win, dx + 8, dy + 1 + (line_h + 2 - atlas.h as i32) / 2, "Quill", s.title_text);
			atlas.text(win, dx + line_h, dy + line_h * 2, msg, 0x000000);
			let bw = atlas.width("Cancel") as i32 + line_h;
			let by = dy + dh - line_h * 2;
			let gap = line_h / 2;
			let total = bw * 3 + gap * 2;
			let mut bx = dx + (dw - total) / 2;
			for (i, label) in ["Yes", "No", "Cancel"].iter().enumerate() {
				bevel(win, bx, by, bw, line_h + 2, face, era);
				atlas.text(
					win,
					bx + (bw - atlas.width(label) as i32) / 2,
					by + 1 + (line_h + 2 - atlas.h as i32) / 2,
					label,
					0x000000,
				);
				if i == 2 && fract(t * 1.1) < 0.6 {
					dotted(win, bx + 3, by + 3, bw - 6, line_h - 4);
				}
				bx += bw + gap;
			}
		}
	}

	/// The page for an act at its local time.
	fn page(&self, act: Act, lt: f32, len: f32, pass: usize, t: f32) -> PageState {
		let lines_before = 3 + pass % 3;
		let mut base = String::new();
		for i in 0..lines_before {
			if i > 0 {
				base.push('\n');
			}
			base.push_str(self.line(pass * 2 + i));
		}
		let next = [self.line(pass * 2 + lines_before), self.line(pass * 2 + lines_before + 1)];
		let mut st = PageState {
			text: Vec::new(),
			base_len: base.chars().count(),
			typo: None,
			modified: false,
			caret_on: fract(t * 0.9) < 0.55,
			margins: (0.06, 0.06),
			justify: false,
			pieces: None,
			ghosts: Vec::new(),
			dialog: 0.0,
			t: lt,
		};
		match act {
			Act::Typing | Act::Lore => {
				let lore = [self.line(pass * 3 + 7), self.line(pass * 3 + 8), self.line(pass * 3 + 9)];
				let (lines, pace, typo): (&[&str], f32, bool) = if act == Act::Typing {
					(&next, 1.0, true)
				} else {
					(&lore, 0.7, false)
				};
				let script = Script::typing(&base, lines, 1.0, pace, typo, self.seed ^ pass as u32);
				let (p, n) = script.at(lt);
				st.text = p.text();
				st.modified = n > 0;
				st.typo = script.typo.clone().map(|ty| (ty, lt));
				// The caret holds steady while the typist is at work, and blinks once the script is done.
				st.caret_on = lt < script.end || st.caret_on;
			}
			Act::Reflow => {
				let script = Script::typing(&base, &next, 0.0, 0.0, false, 0);
				st.text = script.at(f32::MAX).0.text();
				st.modified = true;
				st.justify = true;
				let a = smoothstep(0.0, 3.0, lt) * (1.0 - smoothstep(len - 3.0, len, lt));
				st.margins = (
					0.06 + a * (0.12 + 0.1 * sin(lt * 0.55)),
					0.06 + a * (0.14 + 0.12 * sin(lt * 0.43 + 1.7)),
				);
			}
			Act::TimeMachine => {
				// Typed text rewinds to the base (faster and faster, like tape), holds, then runs forward again.
				let script = Script::typing(&base, &next, 0.0, 0.25, false, self.seed ^ 77);
				let n = script.edits.len();
				let rewind_len = (len * 0.4).max(1.0);
				let back = |u: f32| (u * u * n as f32) as usize;
				let done = if lt < 2.0 {
					n
				} else if lt < 2.0 + rewind_len {
					n - back(saturate((lt - 2.0) / rewind_len)).min(n)
				} else if lt < 4.0 + rewind_len {
					0
				} else {
					(saturate((lt - 4.0 - rewind_len) / (len - 6.0 - rewind_len).max(1.0)) * n as f32) as usize
				};
				let mut p = Pieces::new(&script.base);
				for &(_, e) in &script.edits[..done] {
					match e {
						Edit::Type(c) => p.push(c),
						Edit::Back => p.pop(),
					}
				}
				// The letters just removed fly back into the caret.
				if lt >= 2.0 && lt < 2.0 + rewind_len {
					for g in 1..6 {
						let u = saturate((lt - 2.0 - g as f32 * 0.07) / rewind_len);
						let k = n - back(u).min(n);
						if k < n
							&& k >= done && let Edit::Type(c) = script.edits[k].1
						{
							st.ghosts.push((c, g as f32 * 0.07));
						}
					}
				}
				st.text = p.text();
				st.modified = done > 0;
				st.pieces = Some(p.pieces.iter().map(|&(add, _, len)| (add, len)).collect());
				st.caret_on = true;
			}
			Act::Eras | Act::Dialog => {
				let script = Script::typing(&base, &next, 0.0, 0.0, false, 0);
				st.text = script.at(f32::MAX).0.text();
				st.modified = true;
				if act == Act::Dialog {
					st.dialog = smoothstep(1.5, 2.3, lt) * (1.0 - smoothstep(len - 1.5, len - 0.5, lt));
				}
			}
		}
		st
	}
}

/// A laid-out word: its first character, the one after its last, and whether it ends in a hyphen.
type Word = (usize, usize, bool);

fn push_fmt(s: &mut String, label: &str, n: usize) {
	s.push_str(label);
	let mut digits = [0u8; 10];
	let mut k = 0;
	let mut v = n;
	loop {
		digits[k] = b'0' + (v % 10) as u8;
		k += 1;
		v /= 10;
		if v == 0 {
			break;
		}
	}
	for i in (0..k).rev() {
		s.push(digits[i] as char);
	}
}

fn fill(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, c: u32) {
	let (x0, y0) = (x.max(0), y.max(0));
	let (x1, y1) = ((x + w).min(f.w as i32), (y + h).min(f.h as i32));
	for yy in y0..y1 {
		let row = yy as usize * f.w;
		f.px[row + x0 as usize..row + x1.max(x0) as usize].fill(c);
	}
}

fn blend_fill(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, c: u32, a: u32) {
	let (x0, y0) = (x.max(0), y.max(0));
	let (x1, y1) = ((x + w).min(f.w as i32), (y + h).min(f.h as i32));
	for yy in y0..y1 {
		for xx in x0..x1 {
			let i = yy as usize * f.w + xx as usize;
			f.px[i] = mix(f.px[i], c, a);
		}
	}
}

/// A raised button face: light top-left edge, dark bottom-right edge.
fn bevel(f: &mut Frame, x: i32, y: i32, w: i32, h: i32, face: u32, era: Era) {
	fill(f, x, y, w, h, face);
	let (hi, lo) = if era == Era::Dos {
		(face, face)
	} else {
		(colour::shade(face, 0.6), colour::shade(face, -0.5))
	};
	fill(f, x, y, w, 1, hi);
	fill(f, x, y, 1, h, hi);
	fill(f, x, y + h - 1, w, 1, lo);
	fill(f, x + w - 1, y, 1, h, lo);
}

fn dotted(f: &mut Frame, x: i32, y: i32, w: i32, h: i32) {
	for i in (0..w).step_by(2) {
		fill(f, x + i, y, 1, 1, 0x000000);
		fill(f, x + i, y + h - 1, 1, 1, 0x000000);
	}
	for i in (0..h).step_by(2) {
		fill(f, x, y + i, 1, 1, 0x000000);
		fill(f, x + w - 1, y + i, 1, 1, 0x000000);
	}
}

/// A small upward triangle (a tab stop or margin marker) with its tip at (x, y - h).
fn tri(f: &mut Frame, x: i32, y: i32, h: i32, c: u32) {
	for k in 0..h {
		fill(f, x - k / 2, y - h + k, k.max(1), 1, c);
	}
}

/// A ring: the Q of Quill's mark.
fn ring(f: &mut Frame, cx: i32, cy: i32, r: f32, c: u32) {
	let ri = r as i32 + 1;
	for dy in -ri..=ri {
		for dx in -ri..=ri {
			let d = sqrt((dx * dx + dy * dy) as f32);
			let k = saturate(1.0 - (d - r * 0.72).abs() / (r * 0.28));
			if k > 0.0 {
				let px = cx + dx;
				let py = cy + dy;
				if px >= 0 && py >= 0 && (px as usize) < f.w && (py as usize) < f.h {
					let i = py as usize * f.w + px as usize;
					f.px[i] = mixf(f.px[i], c, k);
				}
			}
		}
	}
}

impl Scene for Ghostwriter {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		let (act, lt, len, pass) = self.act_at(t);
		let st = self.page(act, lt, len, pass, t);
		// Font eras: DOS, then Windows 3.x, then modern, cross-fading; other acts are modern.
		let mut win = core::mem::replace(&mut self.win, Frame::new(1, 1));
		if act == Act::Eras {
			let phase = lt / (len / 3.0);
			let era_of = |i: usize| [Era::Dos, Era::Win3, Era::Modern][i.min(2)];
			let i = phase as usize;
			self.draw_window(&mut win, era_of(i), &st);
			let into = fract(phase) * (len / 3.0);
			if i < 2 && into > len / 3.0 - 0.8 {
				let mut other = core::mem::replace(&mut self.other, Frame::new(1, 1));
				self.draw_window(&mut other, era_of(i + 1), &st);
				let k = (saturate((into - (len / 3.0 - 0.8)) / 0.8) * 256.0) as u32;
				for (a, &b) in win.px.iter_mut().zip(&other.px) {
					*a = mix(*a, b, k);
				}
				self.other = other;
			}
		} else {
			self.draw_window(&mut win, Era::Modern, &st);
		}
		// The desk, then the window placed on it: drifting, slowly turning and breathing in size.
		out.px.copy_from_slice(&self.desk.px);
		let (w, h) = (self.w as f32, self.h as f32);
		let ang = if self.tilt {
			0.035 * sin(t * 0.09) + 0.02 * sin(t * 0.23)
		} else {
			0.0
		};
		let zoom = 0.93 + 0.07 * sin(t * 0.071);
		let cxw = w * (0.5 + 0.05 * sin(t * 0.047));
		let cyw = h * (0.5 + 0.04 * sin(t * 0.061 + 1.0));
		let (ca, sa) = (cos(ang) / zoom, sin(ang) / zoom);
		let (hw, hh) = (win.w as f32 * 0.5, win.h as f32 * 0.5);
		let reach = sqrt(hw * hw + hh * hh) * zoom + 2.0;
		let y0 = ((cyw - reach) as i32).max(0) as usize;
		let y1 = ((cyw + reach) as i32 + 1).min(self.h as i32).max(0) as usize;
		let x0 = ((cxw - reach) as i32).max(0) as usize;
		let x1 = ((cxw + reach) as i32 + 1).min(self.w as i32).max(0) as usize;
		let shadow = self.shadow;
		let (sdx, sdy) = (h * 0.018, h * 0.026);
		let (inv_ww, inv_wh) = (1.0 / win.w as f32, 1.0 / win.h as f32);
		let (lamp_u, lamp_v) = (0.5 + 0.55 * sin(t * 0.11), 0.4 + 0.45 * sin(t * 0.083 + 2.0));
		for y in y0..y1 {
			for x in x0..x1 {
				let (dx, dy) = (x as f32 + 0.5 - cxw, y as f32 + 0.5 - cyw);
				let u = dx * ca + dy * sa + hw;
				let v = -dx * sa + dy * ca + hh;
				let i = y * self.w + x;
				if u >= 0.0 && v >= 0.0 && u < win.w as f32 - 1.0 && v < win.h as f32 - 1.0 {
					// A desk lamp's pool wanders slowly over the window, so even the plain page is never still (SAV-D12).
					let (lu, lv) = (u * inv_ww - lamp_u, v * inv_wh - lamp_v);
					let k = 0.6 + 0.34 * saturate(1.0 - (lu * lu + lv * lv) * 1.6);
					out.px[i] = scale(win.sample((u * 65_536.0) as i32, (v * 65_536.0) as i32), (k * 256.0) as u32);
				} else if shadow {
					// A soft shadow down and to the right of the window.
					let (su, sv) = (u - sdx, v - sdy);
					let out_u = (-su).max(su - win.w as f32).max(0.0);
					let out_v = (-sv).max(sv - win.h as f32).max(0.0);
					let d = sqrt(out_u * out_u + out_v * out_v);
					let k = 1.0 - saturate(d / (h * 0.03));
					if k > 0.0 {
						out.px[i] = scale(out.px[i], 256 - (k * k * 150.0) as u32);
					}
				}
			}
		}
		self.win = win;
	}

	fn cues(&mut self, t: f64) -> crate::audio::Cues {
		let tf = t as f32;
		let (act, lt, len, pass) = self.act_at(tf);
		let st = self.page(act, lt, len, pass, tf);
		crate::audio::Cues {
			t,
			fade: 1.0,
			shot: ACTS.iter().position(|a| a.0 == act).unwrap_or(0),
			shot_t: lt,
			keys: st.text.len() as u32,
			dialog: st.dialog,
			hold: st
				.typo
				.as_ref()
				.is_some_and(|(ty, now)| *now >= ty.popup + 0.9 && *now < ty.popup + 1.0),
			..crate::audio::Cues::default()
		}
	}

	fn hints(&self, _cx: &Ctx) -> Hints {
		Hints {
			threshold: 225,
			streak_tint: mixf(self.pal.sun, 0xFFFFFF, 0.3),
			fade: 1.0,
			bloom_gain: 0.5,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_piece_table_splits_and_merges() {
		let mut p = Pieces::new("Hello");
		p.push(',');
		p.push(' ');
		assert_eq!(p.pieces.len(), 2);
		p.pop();
		p.pop();
		assert_eq!(p.pieces.len(), 1);
		p.push('!');
		assert_eq!(p.text().into_iter().collect::<String>(), "Hello!");
		// A new add piece after a deletion, since the add buffer has moved on.
		assert_eq!(p.pieces, vec![(false, 0, 5), (true, 2, 1)]);
	}

	#[test]
	fn a_typo_is_made_and_fixed() {
		let s = Script::typing("Dear reader,", &["the margins were justified"], 0.0, 1.0, true, 3);
		let ty = s.typo.clone().expect("a typo");
		let during: String = s.at(ty.squiggle).0.text().into_iter().collect();
		let after: String = s.at(s.end).0.text().into_iter().collect();
		assert_ne!(during.trim_end(), "Dear reader,\nthe margins were justified");
		assert_eq!(after, "Dear reader,\nthe margins were justified");
	}
}
