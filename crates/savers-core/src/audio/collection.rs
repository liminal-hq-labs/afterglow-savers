// The Collection's sound: each member's own score, cross-faded with the picture.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The member playing gets the cues; during the two-second cross-fade the next member's score starts from its opening (its cues at
//! the same lead-in time the picture uses), and the two are mixed at equal power.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use super::{Cues, Score, score_for};
use crate::math::{HALF_PI, cos, sin};
use crate::scenes::collection::MEMBERS;
use crate::settings::{SaverKind, Settings};

pub struct CollectionScore {
	rate: u32,
	settings: Settings,
	seed: u32,
	current: Option<(usize, Box<dyn Score>)>,
	next: Option<(usize, Box<dyn Score>)>,
	scratch: Vec<f32>,
	/// Seconds into the next member when its fade begins, as the picture's (`LEAD` in the Collection).
	lead: f64,
	cross_start: Option<f64>,
}

impl CollectionScore {
	pub fn new(settings: &Settings, seed: u32, rate: u32) -> CollectionScore {
		CollectionScore {
			rate,
			settings: settings.clone(),
			seed,
			current: None,
			next: None,
			scratch: Vec::new(),
			lead: 2.5,
			cross_start: None,
		}
	}

	fn make(&self, member: usize) -> Box<dyn Score> {
		let kind = MEMBERS.get(member).copied().unwrap_or(SaverKind::Ink);
		let mut s = Settings::defaults(kind);
		s.adopt_common(&self.settings);
		score_for(kind, &s, self.seed.wrapping_add(member as u32), self.rate)
			.unwrap_or_else(|| score_for(SaverKind::Ink, &s, self.seed, self.rate).expect("Ink has a score"))
	}
}

impl Score for CollectionScore {
	fn render(&mut self, cues: &Cues, out: &mut [f32]) {
		// A new member: the one prepared during the fade becomes the current one.
		if self.current.as_ref().is_none_or(|(m, _)| *m != cues.member) {
			self.current = match self.next.take() {
				Some((m, s)) if m == cues.member => Some((m, s)),
				_ => Some((cues.member, self.make(cues.member))),
			};
			self.cross_start = None;
		}
		if let Some((_, score)) = self.current.as_mut() {
			score.render(cues, out);
		}
		if cues.cross > 0.0 {
			if self.next.is_none() {
				self.next = Some((cues.next_member, self.make(cues.next_member)));
				self.cross_start = Some(cues.t);
			}
			let into = cues.t - self.cross_start.unwrap_or(cues.t);
			let next_cues = Cues {
				t: self.lead + into,
				fade: 1.0,
				..Cues::default()
			};
			if self.scratch.len() != out.len() {
				self.scratch = vec![0.0; out.len()];
			}
			self.scratch.fill(0.0);
			if let Some((_, score)) = self.next.as_mut() {
				score.render(&next_cues, &mut self.scratch);
			}
			let (a, b) = (cos(cues.cross * HALF_PI), sin(cues.cross * HALF_PI));
			for (o, &n) in out.iter_mut().zip(&self.scratch) {
				*o = *o * a + n * b;
			}
		}
	}
}
