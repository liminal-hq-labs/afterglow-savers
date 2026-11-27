// Golden frames and the burn-in rule for every scene, rendered small on the host.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A golden is an FNV-1a hash of a 160 × 90 frame at fixed times, for each detail level. The hashes are for the host build only
//! (SAV-D6). After a deliberate change of look, run with `SAVERS_UPDATE_GOLDEN=1` and say so in the commit.

use std::fmt::Write;
use std::ops::Range;
use std::path::PathBuf;

use savers_core::bands::{Bands, chunk};
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

/// Every seed makes a walkable maze: none hangs, panics or paints a blank frame at the start, for each maze size.
#[test]
fn many_mazes_start_and_paint() {
	for seed in 0..300u32 {
		let mut s = Settings::defaults(SaverKind::Maze);
		s.set(Key::RenderScale, 100);
		s.set(Key::Grain, 0);
		s.set(Key::MazeSize, [8, 14, 20, 32][(seed % 4) as usize]);
		let (rw, rh) = render_size(&s, 64, 36, false);
		let scene = build(SaverKind::Maze, &s, seed, rw, rh);
		let mut sv = Saver::new(s, scene, 64, 36, false);
		sv.at(0.0);
		let f = sv.at(1.5);
		let mean = f.px.iter().map(|&c| luma(c) as u64).sum::<u64>() / f.px.len() as u64;
		assert!(mean > 3, "seed {seed}: the first frames are blank (mean luma {mean})");
	}
}

/// The scores are reproducible, stay below full scale with no clicks, and are never silent once faded in.
#[test]
fn scores_are_reproducible_and_gentle() {
	use savers_core::audio::{Engine, score_for};
	for kind in [SaverKind::Twilight, SaverKind::Maze] {
		let render = || {
			let mut s = Settings::defaults(kind);
			s.set(Key::LoopMaze, 1);
			let mut scene = build(kind, &s, 3, 32, 18);
			let score = score_for(kind, &s, 3, 22_050).expect("a score");
			let mut engine = Engine::new(22_050, score, 1.0, 1.0);
			let mut out = vec![0i16; 22_050 * 2 * 8];
			engine.render(&mut out, &mut |t| scene.cues(t));
			out
		};
		let a = render();
		assert_eq!(a, render(), "{kind:?}: the same seed gives the same samples");
		let peak = a.iter().map(|&v| (v as i32).abs()).max().unwrap_or(0);
		assert!(peak < 23_000, "{kind:?}: peak {peak} is within 3 dB of full scale");
		let step = a
			.chunks(2)
			.collect::<Vec<_>>()
			.windows(2)
			.map(|w| (w[0][0] as i32 - w[1][0] as i32).abs())
			.max()
			.unwrap_or(0);
		assert!(step < 4_000, "{kind:?}: a jump of {step} between samples is a click");
		let tail = &a[a.len() / 2..];
		let rms = (tail.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt();
		assert!(rms > 100.0, "{kind:?}: RMS {rms:.0} is near silence");
	}
}

/// Every saver has a score: reproducible, below full scale, and not silent (Ghostwriter's is quiet by design).
#[test]
fn every_saver_has_a_gentle_score() {
	use savers_core::audio::{Engine, score_for};
	for kind in SaverKind::ALL {
		let render = || {
			let mut s = Settings::defaults(kind);
			s.set(Key::Duration, 30);
			let mut scene = build(kind, &s, 5, 32, 18);
			let score = score_for(kind, &s, 5, 22_050).expect("a score");
			let mut engine = Engine::new(22_050, score, 1.0, 0.5);
			let mut out = vec![0i16; 22_050 * 2 * 6];
			engine.render(&mut out, &mut |t| scene.cues(t * 5.0));
			out
		};
		let a = render();
		assert_eq!(a, render(), "{kind:?}: the same seed gives the same samples");
		let peak = a.iter().map(|&v| (v as i32).abs()).max().unwrap_or(0);
		assert!(peak < 23_000, "{kind:?}: peak {peak} is within 3 dB of full scale");
		let rms = (a.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / a.len() as f64).sqrt();
		assert!(rms > 8.0, "{kind:?}: RMS {rms:.1} is silence");
	}
}

/// Cuts a pass into `count` ranges and runs them one after another, last first, so a band that depended on its neighbour having run
/// would show.
struct Split {
	count: usize,
}

impl Bands for Split {
	fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync)) {
		for i in (0..self.count).rev() {
			let r = chunk(rows, self.count, i);
			if !r.is_empty() {
				job(r);
			}
		}
	}
}

/// Cuts a pass into `count` ranges and runs each on its own thread.
struct Threaded {
	count: usize,
}

impl Bands for Threaded {
	fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync)) {
		std::thread::scope(|scope| {
			for i in 0..self.count {
				let r = chunk(rows, self.count, i);
				if !r.is_empty() {
					scope.spawn(move || job(r));
				}
			}
		});
	}
}

/// Hashes every frame of a run of `seconds`, at 10 frames a second, with the grain, the letterbox and every effect on.
fn run_hash(kind: SaverKind, w: usize, h: usize, seconds: f64, bands: Option<&'static dyn Bands>) -> u64 {
	let mut s = Settings::defaults(kind);
	s.set(Key::Detail, 3);
	s.set(Key::RenderScale, 100);
	s.set(Key::Grain, 40);
	s.set(Key::Letterbox, 1);
	let (rw, rh) = render_size(&s, w, h, false);
	let scene = build(kind, &s, 11, rw, rh);
	let mut sv = Saver::new(s, scene, w, h, false);
	if let Some(b) = bands {
		sv.set_bands(b);
	}
	let mut all = 0xCBF2_9CE4_8422_2325u64;
	let mut t = 0.0;
	while t <= seconds {
		let f = sv.at(t);
		all = (all ^ fnv(&f.px)).wrapping_mul(0x0100_0000_01B3);
		t += 0.1;
	}
	all
}

/// SAV-D51: the frame is the same byte for byte with any number of bands, in any order and on any number of threads. The sizes are
/// odd and the times cross the places where the scenes change (the Ink's aurora, the Cracktro's transitions, the Ghostwriter's eras).
#[test]
fn bands_do_not_change_a_byte() {
	for &kind in SCENES {
		let seconds = match kind {
			SaverKind::Ink => 52.0,
			SaverKind::Cracktro => 33.0,
			_ => 30.0,
		};
		let (w, h) = (203, 117);
		let want = run_hash(kind, w, h, seconds, None);
		for count in [1, 2, 3, 8] {
			let sequential: &'static dyn Bands = Box::leak(Box::new(Split { count }));
			assert_eq!(
				run_hash(kind, w, h, seconds, Some(sequential)),
				want,
				"{kind:?} with {count} bands one after another"
			);
			let threaded: &'static dyn Bands = Box::leak(Box::new(Threaded { count }));
			assert_eq!(
				run_hash(kind, w, h, seconds, Some(threaded)),
				want,
				"{kind:?} with {count} bands on threads"
			);
		}
	}
}
