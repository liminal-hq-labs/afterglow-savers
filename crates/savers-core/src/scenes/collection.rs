// Afterglow Collection: the savers in turn, with cross-fades.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Each saver plays for the `Duration` setting from its own beginning, then the next fades in over two seconds while both are
//! drawn. The next scene is made a few seconds before its fade, so its start-up does not land in the middle of the fade.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::colour::mix;
use crate::frame::Frame;
use crate::math::saturate;
use crate::rng::Rng;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Key, SaverKind, Settings};

const CROSS: f32 = 2.0;
const PREPARE: f32 = 5.0;
/// Each member starts this far into its own storyboard, so its fade-up is over before it is seen.
const LEAD: f32 = 2.5;

/// The savers the Collection can play, in the order of the `Savers` mask's bits.
pub const MEMBERS: [SaverKind; 5] = [
	SaverKind::Ink,
	SaverKind::Twilight,
	SaverKind::Cracktro,
	SaverKind::Maze,
	SaverKind::Ghostwriter,
];

/// Makes a member scene; the Collection is given one so a platform decides which scenes are linked.
pub type Maker = fn(SaverKind, &Settings, u32, usize, usize) -> Box<dyn Scene>;

pub struct Collection {
	w: usize,
	h: usize,
	order: Vec<SaverKind>,
	settings: Vec<Settings>,
	duration: f32,
	make: Maker,
	seed: u32,
	current: Box<dyn Scene>,
	current_index: usize,
	next: Option<Box<dyn Scene>>,
	other: Frame,
	fade: f32,
}

impl Collection {
	/// `members` holds each member's own settings (any order, missing ones take defaults); the Collection's common settings
	/// replace theirs.
	pub fn new(settings: &Settings, members: &[Settings], seed: u32, w: usize, h: usize, make: Maker) -> Collection {
		let mask = settings.get(Key::Savers);
		let mut order: Vec<SaverKind> = MEMBERS
			.iter()
			.enumerate()
			.filter(|(i, _)| mask & (1 << i) != 0)
			.map(|(_, k)| *k)
			.collect();
		if order.is_empty() {
			order = MEMBERS[..4].to_vec();
		}
		if settings.get(Key::Shuffle) == 1 {
			let mut rng = Rng::new(seed as u64 ^ 0xC011_0000_0000);
			for i in (1..order.len()).rev() {
				let j = rng.below(i as u32 + 1) as usize;
				order.swap(i, j);
			}
		}
		let settings_duration = settings.get(Key::Duration) as f32;
		let settings: Vec<Settings> = order
			.iter()
			.map(|&kind| {
				let mut s = members
					.iter()
					.find(|m| m.kind == kind)
					.cloned()
					.unwrap_or_else(|| Settings::defaults(kind));
				s.adopt_common(settings);
				s
			})
			.collect();
		let current = make(order[0], &settings[0], member_seed(seed, &settings[0], 0), w, h);
		Collection {
			w,
			h,
			order,
			settings,
			duration: settings_duration.max(CROSS + PREPARE + 1.0),
			make,
			seed,
			current,
			current_index: 0,
			next: None,
			other: Frame::new(w, h),
			fade: 1.0,
		}
	}
}

impl Collection {
	/// Moves to turn `turn`: the prepared scene becomes the current one.
	fn advance(&mut self, turn: usize) {
		let n = self.order.len();
		while self.current_index < turn {
			self.current_index += 1;
			let i = self.current_index % n;
			self.current = match self.next.take() {
				Some(s) => s,
				None => (self.make)(
					self.order[i],
					&self.settings[i],
					member_seed(self.seed, &self.settings[i], self.current_index),
					self.w,
					self.h,
				),
			};
		}
	}
}

/// A member's seed: its own setting, or one derived from the Collection's seed and its turn.
fn member_seed(seed: u32, s: &Settings, turn: usize) -> u32 {
	match s.get(Key::Seed) {
		0 => seed.wrapping_add((turn as u32).wrapping_mul(0x9E37_79B9)),
		fixed => fixed,
	}
}

impl Scene for Collection {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		let d = self.duration;
		let turn = (t / d) as usize;
		let local = t - turn as f32 * d;
		let n = self.order.len();
		self.advance(turn);
		let start = turn as f32 * d;
		let sub = Ctx {
			t: (t - start + CROSS + LEAD) as f64,
			dt: cx.dt,
			detail: cx.detail,
			pal: cx.pal,
			frame_no: cx.frame_no,
			bands: cx.bands,
		};
		self.current.render(&sub, out);
		self.fade = self.current.hints(&sub).fade;
		if n > 1 && local > d - PREPARE && self.next.is_none() {
			let i = (turn + 1) % n;
			self.next = Some((self.make)(
				self.order[i],
				&self.settings[i],
				member_seed(self.seed, &self.settings[i], turn + 1),
				self.w,
				self.h,
			));
		}
		if n > 1
			&& local > d - CROSS
			&& let Some(next) = self.next.as_mut()
		{
			let into = local - (d - CROSS);
			let nsub = Ctx {
				t: (into + LEAD) as f64,
				dt: cx.dt,
				detail: cx.detail,
				pal: cx.pal,
				frame_no: cx.frame_no,
				bands: cx.bands,
			};
			next.render(&nsub, &mut self.other);
			let k = (saturate(into / CROSS) * 256.0) as u32;
			let other = &self.other.px;
			crate::bands::rows(cx.bands, &mut out.px, out.w, &|band| {
				let base = band.y0 * band.w;
				for (n, o) in band.px.iter_mut().enumerate() {
					*o = mix(*o, other[base + n], k);
				}
			});
			self.fade = 1.0;
		}
	}

	fn cues(&mut self, t: f64) -> crate::audio::Cues {
		let tf = t as f32;
		let d = self.duration;
		let turn = (tf / d) as usize;
		let local = tf - turn as f32 * d;
		self.advance(turn);
		let n = self.order.len();
		let index_of = |kind: SaverKind| MEMBERS.iter().position(|&m| m == kind).unwrap_or(0);
		let mut c = self.current.cues((local + CROSS + LEAD) as f64);
		c.member = index_of(self.order[turn % n]);
		c.next_member = index_of(self.order[(turn + 1) % n]);
		c.cross = if n > 1 { saturate((local - (d - CROSS)) / CROSS) } else { 0.0 };
		c
	}

	fn hints(&self, cx: &Ctx) -> Hints {
		let t = cx.t as f32;
		let start = (t / self.duration) as usize as f32 * self.duration;
		let sub = Ctx {
			t: (t - start + CROSS + LEAD) as f64,
			dt: cx.dt,
			detail: cx.detail,
			pal: cx.pal,
			frame_no: cx.frame_no,
			bands: cx.bands,
		};
		let mut h = self.current.hints(&sub);
		h.fade = self.fade;
		h
	}
}
