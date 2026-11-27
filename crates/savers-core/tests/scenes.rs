// Golden frames and the burn-in rule for every scene, rendered small on the host.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A golden is an FNV-1a hash of a 160 × 90 frame at fixed times, for each detail level. The hashes are for the host build only
//! (SAV-D6). After a deliberate change of look, run with `SAVERS_UPDATE_GOLDEN=1` and say so in the commit.

use std::fmt::Write;
use std::path::PathBuf;

use savers_core::colour::luma;
use savers_core::saver::{Saver, render_size};
use savers_core::scenes::build;
use savers_core::settings::{Key, SaverKind, Settings};

/// The scenes built so far.
const SCENES: &[SaverKind] = &[
	SaverKind::Ink,
	SaverKind::Twilight,
	SaverKind::Cracktro,
	SaverKind::Maze,
	SaverKind::Collection,
	SaverKind::Ghostwriter,
];

fn saver(kind: SaverKind, detail: u32, w: usize, h: usize) -> Saver {
	let mut s = Settings::defaults(kind);
	s.set(Key::Detail, detail);
	s.set(Key::RenderScale, 100);
	s.set(Key::Grain, 0);
	let (rw, rh) = render_size(&s, w, h, false);
	let scene = build(kind, &s, 7, rw, rh);
	Saver::new(s, scene, w, h, false)
}

fn fnv(px: &[u32]) -> u64 {
	let mut h = 0xCBF2_9CE4_8422_2325u64;
	for &p in px {
		for b in p.to_le_bytes() {
			h ^= b as u64;
			h = h.wrapping_mul(0x0100_0000_01B3);
		}
	}
	h
}

#[test]
fn golden_frames() {
	let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
	let update = std::env::var_os("SAVERS_UPDATE_GOLDEN").is_some();
	for &kind in SCENES {
		let mut got = String::new();
		for detail in 0..4 {
			let mut sv = saver(kind, detail, 160, 90);
			let mut t = 0.0;
			for stop in [2.0, 15.0, 40.0] {
				while t < stop {
					sv.at(t);
					t += 1.0 / 15.0;
				}
				let f = sv.at(stop);
				writeln!(got, "detail {detail} t {stop}: {:016x}", fnv(&f.px)).unwrap();
			}
		}
		let path = dir.join(format!("{}.txt", kind.registry_name().to_lowercase()));
		if update {
			std::fs::write(&path, &got).unwrap();
		} else {
			let want = std::fs::read_to_string(&path).unwrap_or_default();
			assert_eq!(
				got, want,
				"{kind:?} frames changed; look at them with savers-preview, then SAVERS_UPDATE_GOLDEN=1"
			);
		}
	}
}

/// SAV-D12: nothing bright holds still, and the frame is mostly dark.
#[test]
fn burn_in_rule() {
	for &kind in SCENES {
		let mut sv = saver(kind, 1, 160, 90);
		let mut t = 0.0;
		let mut frames: Vec<Vec<u32>> = Vec::new();
		for stop in [20.0, 28.0, 36.0, 44.0, 52.0] {
			while t < stop {
				sv.at(t);
				t += 1.0 / 15.0;
			}
			frames.push(sv.at(stop).px.clone());
		}
		for pair in frames.windows(2) {
			let n = pair[0].len();
			let stuck = (0..n)
				.filter(|&i| luma(pair[0][i]) > 102 && luma(pair[0][i]).abs_diff(luma(pair[1][i])) < 6)
				.count();
			assert!(
				stuck * 100 < n * 3,
				"{kind:?}: {stuck} of {n} bright pixels held still for 8 s"
			);
		}
		for f in &frames {
			let mean = f.iter().map(|&c| luma(c) as u64).sum::<u64>() / f.len() as u64;
			assert!(mean < 100, "{kind:?}: mean luma {mean} is too bright");
		}
	}
}

#[test]
fn every_render_size_works() {
	for &kind in SCENES {
		for (w, h) in [(16, 9), (152, 112), (33, 17)] {
			let mut sv = saver(kind, 0, w, h);
			sv.at(0.0);
			let f = sv.at(0.1);
			assert_eq!((f.w, f.h), (w, h));
		}
		let _ = render_size(&Settings::defaults(kind), 1, 1, true);
	}
}

/// The Maze in its looping mode is a true loop: the frame after one walk is the frame at the start, and the portal's crossing is no
/// bigger a step than any other frame of the walk.
#[test]
fn the_maze_loops_through_its_portal() {
	let make = || {
		let mut s = Settings::defaults(SaverKind::Maze);
		s.set(Key::Detail, 2);
		s.set(Key::RenderScale, 100);
		s.set(Key::Grain, 0);
		s.set(Key::LoopMaze, 1);
		let (rw, rh) = render_size(&s, 160, 90, false);
		let scene = build(SaverKind::Maze, &s, 7, rw, rh);
		Saver::new(s, scene, 160, 90, false)
	};
	let mut sv = make();
	let length = sv.loop_length().expect("a looping maze reports its length");
	let step = 1.0 / 30.0;
	let mut t = 0.0;
	let at = |sv: &mut Saver, when: f64, t: &mut f64| {
		while *t + step < when {
			sv.at(*t);
			*t += step;
		}
		*t = when;
		sv.at(when).px.clone()
	};
	let first = at(&mut sv, 5.0, &mut t);
	let ordinary = (at(&mut sv, 20.0, &mut t), at(&mut sv, 20.0 + 2.0 * step, &mut t));
	let before = at(&mut sv, length - step, &mut t);
	let after = at(&mut sv, length + step, &mut t);
	let again = at(&mut sv, length + 5.0, &mut t);
	let diff = |a: &[u32], b: &[u32]| {
		a.iter()
			.zip(b)
			.map(|(&x, &y)| (luma(x) as i64 - luma(y) as i64).unsigned_abs())
			.sum::<u64>() as f64
			/ a.len() as f64
	};
	assert_eq!(first, again, "one walk later the frame is the same");
	let seam = diff(&before, &after);
	let walk = diff(&ordinary.0, &ordinary.1);
	assert!(
		seam <= walk * 2.0 + 1.0,
		"the crossing changes {seam:.2} levels a pixel, an ordinary step {walk:.2}"
	);
}
