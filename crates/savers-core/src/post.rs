// The finishing chain every scene shares: bloom, anamorphic streaks, vignette, grain, letterbox, fades and the final scaling.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Two passes. The glow pass works at the render size: the bright parts are taken at a quarter of the size (an eighth at Low),
//! blurred by repeated box filters (bloom) and by long horizontal ones (streaks), packed into one colour buffer and laid over the
//! frame with a screen blend through a bilinear enlargement. The output pass enlarges the frame to the output size and applies the
//! vignette, the fade, the letterbox and the grain in the same loop, so each output pixel is touched once.

use alloc::vec;
use alloc::vec::Vec;

use crate::bands::{Bands, Shared};
use crate::colour::{b, g, mix, r, screen};
use crate::frame::Frame;
use crate::rng::hash32;

/// What the finishing chain does to one frame.
#[derive(Clone, Copy, Debug)]
pub struct FinishParams {
	/// Bloom, streaks, vignette and grain strengths, 0 to 100.
	pub bloom: u32,
	pub streaks: u32,
	pub vignette: u32,
	pub grain: u32,
	/// Letterbox bars to this width-to-height ratio when the output is wider than it is... 0 for none.
	pub letterbox: f32,
	/// The fade: 0 is black, 1 is the picture.
	pub fade: f32,
	/// Luma above which a pixel glows (0 to 255).
	pub threshold: u32,
	/// The colour of the streaks.
	pub streak_tint: u32,
	/// Bloom at an eighth of the render size instead of a quarter.
	pub coarse: bool,
}

impl Default for FinishParams {
	fn default() -> Self {
		FinishParams {
			bloom: 60,
			streaks: 40,
			vignette: 50,
			grain: 30,
			letterbox: 0.0,
			fade: 1.0,
			threshold: 150,
			streak_tint: 0xFFB070,
			coarse: false,
		}
	}
}

/// The buffers the chain keeps between frames.
pub struct Finisher {
	qw: usize,
	qh: usize,
	red: Vec<i32>,
	green: Vec<i32>,
	blue: Vec<i32>,
	streak: Vec<i32>,
	glow: Vec<u32>,
	grain: Vec<i16>,
	vx: Vec<u32>,
	vy: Vec<u32>,
	pub out: Frame,
}

impl Default for Finisher {
	fn default() -> Self {
		Finisher::new()
	}
}

impl Finisher {
	pub fn new() -> Finisher {
		// Triangular grain values in about [-255, 255]: the sum of two uniform ones, so most are small.
		let mut grain = vec![0i16; 8192];
		for (i, v) in grain.iter_mut().enumerate() {
			let a = (hash32(i as u32 * 2 + 1) & 0xFF) as i32;
			let c = (hash32(i as u32 * 2 + 0x9E37) & 0xFF) as i32;
			*v = (a + c - 255) as i16;
		}
		Finisher {
			qw: 0,
			qh: 0,
			red: Vec::new(),
			green: Vec::new(),
			blue: Vec::new(),
			streak: Vec::new(),
			glow: Vec::new(),
			grain,
			vx: Vec::new(),
			vy: Vec::new(),
			out: Frame::new(1, 1),
		}
	}

	/// Lays the glow over `frame` in place. Every pass is shared between the threads of `bands` (SAV-D51); the result does not
	/// depend on how.
	pub fn glow(&mut self, frame: &mut Frame, p: &FinishParams, bands: &dyn Bands) {
		if p.bloom == 0 && p.streaks == 0 {
			return;
		}
		let step = if p.coarse { 8 } else { 4 };
		let qw = frame.w.div_ceil(step);
		let qh = frame.h.div_ceil(step);
		if qw != self.qw || qh != self.qh {
			self.qw = qw;
			self.qh = qh;
			for plane in [&mut self.red, &mut self.green, &mut self.blue, &mut self.streak] {
				plane.clear();
				plane.resize(qw * qh, 0);
			}
			self.glow.clear();
			self.glow.resize(qw * qh, 0);
		}
		let (fw, fh) = (frame.w, frame.h);
		let red = Shared::new(&mut self.red);
		let green = Shared::new(&mut self.green);
		let blue = Shared::new(&mut self.blue);
		let streak = Shared::new(&mut self.streak);
		let glow = Shared::new(&mut self.glow);
		// The bright pass, at the coarse size: two samples per block, the part of the luma above the threshold, kept in colour.
		let thr = p.threshold.min(250) as i32;
		let span = 255 - thr;
		let half = step / 2;
		let src = &frame.px;
		bands.run(qh, &|range| {
			// SAFETY: the bands' row ranges are disjoint, and each plane is written only here in this pass.
			let (red, green, blue, streak) = unsafe {
				let cells = range.start * qw..range.end * qw;
				(
					red.slice(cells.clone()),
					green.slice(cells.clone()),
					blue.slice(cells.clone()),
					streak.slice(cells),
				)
			};
			for qy in range.clone() {
				let y0 = (qy * step + half / 2).min(fh - 1);
				let y1 = (qy * step + half + half / 2).min(fh - 1);
				for qx in 0..qw {
					let x0 = (qx * step + half / 2).min(fw - 1);
					let x1 = (qx * step + half + half / 2).min(fw - 1);
					let c0 = src[y0 * fw + x0];
					let c1 = src[y1 * fw + x1];
					let (rr, gg, bb) = ((r(c0) + r(c1)) as i32, (g(c0) + g(c1)) as i32, (b(c0) + b(c1)) as i32);
					let l = (rr * 77 + gg * 150 + bb * 29) >> 9;
					let i = (qy - range.start) * qw + qx;
					let k = (l - thr).max(0);
					// Each channel times how far over the threshold the luma is, 0 to 510 per channel.
					red[i] = rr * k / span;
					green[i] = gg * k / span;
					blue[i] = bb * k / span;
					streak[i] = (k * k / span) * 2;
				}
			}
		});
		// Bloom: two box blurs each way, radius about 2% of the height. Streaks: three long horizontal blurs, about a quarter of
		// the width each, then a thin vertical one. The planes are independent, so each pass does all of them, rows then columns.
		let radius = (qh / 24).max(1);
		let long = (qw / 7).max(2);
		let streaks = p.streaks > 0;
		let blur_all_rows = |first: bool| {
			bands.run(qh, &|range| {
				let mut tmp = vec![0i32; qw];
				let cells = range.start * qw..range.end * qw;
				// SAFETY: disjoint row ranges again; each plane is touched by this pass alone.
				unsafe {
					for plane in [&red, &green, &blue] {
						blur_rows(plane.slice(cells.clone()), qw, radius, &mut tmp);
					}
					if first && streaks {
						for _ in 0..3 {
							blur_rows(streak.slice(cells.clone()), qw, long, &mut tmp);
						}
					}
				}
			});
		};
		// The packing of the glow rides on the last column pass: bloom in its own colours, streaks in the tint.
		let bk = (p.bloom.min(100) * 3) as i32;
		let sk = (p.streaks.min(100) * 5) as i32;
		let (tr, tg, tb) = (r(p.streak_tint) as i32, g(p.streak_tint) as i32, b(p.streak_tint) as i32);
		let blur_all_cols = |first: bool, last: bool| {
			bands.run(qw, &|range| {
				let mut tmp = vec![0i32; qh];
				// SAFETY: disjoint column ranges; every cell of a column is read and written by its own band only.
				unsafe {
					for plane in [&red, &green, &blue] {
						blur_cols(plane, qw, qh, radius, range.clone(), &mut tmp);
					}
					if first && streaks {
						blur_cols(&streak, qw, qh, 1, range.clone(), &mut tmp);
					}
					if last {
						for y in 0..qh {
							for x in range.clone() {
								let i = y * qw + x;
								let s = streak.get(i) * sk / 100;
								let rr = (red.get(i) * bk / 200 + s * tr / 255).clamp(0, 255) as u32;
								let gg = (green.get(i) * bk / 200 + s * tg / 255).clamp(0, 255) as u32;
								let bb = (blue.get(i) * bk / 200 + s * tb / 255).clamp(0, 255) as u32;
								glow.set(i, (rr << 16) | (gg << 8) | bb);
							}
						}
					}
				}
			});
		};
		blur_all_rows(true);
		blur_all_cols(true, false);
		blur_all_rows(false);
		blur_all_cols(false, true);
		// Lay it over the frame: bilinear through a row cache, screen blend.
		let fy_step = ((qh as i64) << 16) / fh as i64;
		let fx_step = ((qw as i64) << 16) / fw as i64;
		let glow = &self.glow;
		crate::bands::rows(bands, &mut frame.px, fw, &|band| {
			let mut row = vec![0u32; qw];
			for y in band.y0..band.y1 {
				let sy = (y as i64 * fy_step - 32_768).max(0) as i32;
				let y0 = ((sy >> 16) as usize).min(qh - 1);
				let y1 = (y0 + 1).min(qh - 1);
				let ty = ((sy >> 8) & 0xFF) as u32;
				for x in 0..qw {
					row[x] = mix(glow[y0 * qw + x], glow[y1 * qw + x], ty);
				}
				let line = band.row(y);
				let mut sx = -32_768i32;
				for px in line.iter_mut() {
					let xs = sx.max(0);
					let x0 = ((xs >> 16) as usize).min(qw - 1);
					let x1 = (x0 + 1).min(qw - 1);
					let gl = mix(row[x0], row[x1], ((xs >> 8) & 0xFF) as u32);
					if gl != 0 {
						*px = screen(*px, gl);
					}
					sx += fx_step as i32;
				}
			}
		});
	}

	/// Enlarges `src` into `self.out` at `ow` × `oh` with the vignette, fade, letterbox and grain. `frame_no` moves the grain.
	pub fn output(&mut self, src: &Frame, ow: usize, oh: usize, p: &FinishParams, frame_no: u32, bands: &dyn Bands) -> &Frame {
		if self.out.w != ow || self.out.h != oh || self.vx.len() != ow || self.vy.len() != oh {
			self.out.resize(ow, oh);
			self.vx.clear();
			self.vy.clear();
			self.vx.resize(ow, 256);
			self.vy.resize(oh, 256);
		}
		// The vignette as a product of a horizontal and a vertical falloff, wider than tall as a lens is.
		let v = p.vignette.min(100) as f32 / 100.0;
		let aspect = ow as f32 / oh as f32;
		for (x, slot) in self.vx.iter_mut().enumerate() {
			let d = (x as f32 + 0.5) / ow as f32 * 2.0 - 1.0;
			*slot = vfall(d * d * v * 0.55 * crate::math::min(aspect, 1.6) / 1.6);
		}
		for (y, slot) in self.vy.iter_mut().enumerate() {
			let d = (y as f32 + 0.5) / oh as f32 * 2.0 - 1.0;
			*slot = vfall(d * d * v * 0.7);
		}
		let fade = (crate::math::saturate(p.fade) * 256.0) as u32;
		// Letterbox rows, when the output is wider than the ratio asks... the bars take the rest.
		let (top, bottom) = if p.letterbox > 0.0 {
			let want = (ow as f32 / p.letterbox) as usize;
			if want < oh {
				let bar = (oh - want) / 2;
				(bar, oh - bar)
			} else {
				(0, oh)
			}
		} else {
			(0, oh)
		};
		let amp = (p.grain.min(100) * 28 / 100) as i32;
		let same = src.w == ow && src.h == oh;
		let sx_step = (((src.w as i64) << 16) / ow as i64) as i32;
		let sy_step = (((src.h as i64) << 16) / oh as i64) as i32;
		let mask = self.grain.len() - 1;
		let Finisher { out, vx, vy, grain, .. } = self;
		let (vx, vy, grain) = (&vx[..], &vy[..], &grain[..]);
		crate::bands::rows(bands, &mut out.px, ow, &|band| {
			for y in band.y0..band.y1 {
				let line = band.row(y);
				if y < top || y >= bottom {
					line.fill(0);
					continue;
				}
				let vy = vy[y] * fade >> 8;
				let mut gi = hash32(frame_no.wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B)) as usize;
				if same {
					let srow = &src.px[y * ow..(y + 1) * ow];
					for ((px, &c), &vx) in line.iter_mut().zip(srow).zip(vx) {
						let k = vx * vy >> 8;
						let gr = grain[gi & mask] as i32 * amp >> 8;
						gi += 1;
						*px = finish_px(c, k, gr);
					}
				} else {
					let sy = (y as i32 * sy_step + sy_step / 2 - 32_768).max(0);
					let y0 = ((sy >> 16) as usize).min(src.h - 1);
					let y1 = (y0 + 1).min(src.h - 1);
					let ty = ((sy >> 8) & 0xFF) as u32;
					let r0 = &src.px[y0 * src.w..(y0 + 1) * src.w];
					let r1 = &src.px[y1 * src.w..(y1 + 1) * src.w];
					let mut sx = sx_step / 2 - 32_768;
					for (px, &vx) in line.iter_mut().zip(vx) {
						let xs = sx.max(0);
						let x0 = ((xs >> 16) as usize).min(src.w - 1);
						let x1 = (x0 + 1).min(src.w - 1);
						let tx = ((xs >> 8) & 0xFF) as u32;
						let c = mix(mix(r0[x0], r0[x1], tx), mix(r1[x0], r1[x1], tx), ty);
						let k = vx * vy >> 8;
						let gr = grain[gi & mask] as i32 * amp >> 8;
						gi += 1;
						*px = finish_px(c, k, gr);
						sx += sx_step;
					}
				}
			}
		});
		&self.out
	}
}

/// The vignette's falloff for a squared, weighted distance, as 0 to 256.
fn vfall(d: f32) -> u32 {
	let f = 1.0 - d;
	(crate::math::saturate(f * f) * 256.0) as u32
}

/// The grain's weight for a level: 96 in the blacks and whites, up to 256 in the mid-tones.
const GRAIN_WEIGHT: [u8; 256] = {
	let mut t = [0u8; 256];
	let mut l = 0;
	while l < 256 {
		let m = (l * (255 - l)) >> 6;
		t[l] = (96 + if m < 159 { m } else { 159 }) as u8;
		l += 1;
	}
	t
};

/// One finished pixel: scaled by `k` (0 to 256), plus grain `gr`, weighted towards the mid-tones.
#[inline(always)]
fn finish_px(c: u32, k: u32, gr: i32) -> u32 {
	let rb = ((c & 0xFF00FF) * k) >> 8;
	let gg = ((c & 0x00FF00) * k) >> 8;
	let c = (rb & 0xFF00FF) | (gg & 0x00FF00);
	if gr == 0 {
		return c;
	}
	// Grain is strongest in the mid-tones and lighter in the blacks, so the void stays deep but never still. Green stands in
	// for the luma, which saves two multiplies a pixel.
	let d = gr * GRAIN_WEIGHT[g(c) as usize] as i32 >> 8;
	add_saturating(c, d)
}

/// Adds `d` (-255 to 255) to each channel of a colour, saturating at 0 and 255, two channels at a time: red and blue share one
/// word with eight guard bits between them, green another. A carry into a guard bit marks a channel that overflowed; a guard bit
/// borrowed away marks one that underflowed.
#[inline(always)]
fn add_saturating(c: u32, d: i32) -> u32 {
	let (rb, gg) = (c & 0x00FF_00FF, c & 0x0000_FF00);
	if d >= 0 {
		let d = d as u32;
		let rb = rb + d * 0x0001_0001;
		let gg = gg + (d << 8);
		let rb_over = rb & 0x0100_0100;
		let g_over = gg & 0x0001_0000;
		let rb = (rb | (rb_over - (rb_over >> 8))) & 0x00FF_00FF;
		let gg = (gg | (g_over - (g_over >> 8))) & 0x0000_FF00;
		rb | gg
	} else {
		let d = (-d) as u32;
		// Set the guard bits, subtract, and keep a channel only where its guard bit survived.
		let rb = (rb | 0x0100_0100) - d * 0x0001_0001;
		let gg = (gg | 0x0001_0000) - (d << 8);
		let rb_keep = rb & 0x0100_0100;
		let g_keep = gg & 0x0001_0000;
		let rb = rb & (rb_keep - (rb_keep >> 8)) & 0x00FF_00FF;
		let gg = gg & (g_keep - (g_keep >> 8)) & 0x0000_FF00;
		rb | gg
	}
}

/// Box blur along each row of `rows` (whole rows of width `w`), radius `rad`, through a running sum, with the edges clamped.
fn blur_rows(rows: &mut [i32], w: usize, rad: usize, tmp: &mut [i32]) {
	let n = (2 * rad + 1) as i32;
	for row in rows.chunks_exact_mut(w) {
		tmp[..w].copy_from_slice(row);
		let at = |i: isize, tmp: &[i32]| tmp[i.clamp(0, w as isize - 1) as usize];
		let mut sum: i32 = 0;
		for i in -(rad as isize)..=(rad as isize) {
			sum += at(i, tmp);
		}
		for (x, v) in row.iter_mut().enumerate() {
			*v = sum / n;
			sum += at(x as isize + rad as isize + 1, tmp) - at(x as isize - rad as isize, tmp);
		}
	}
}

/// Box blur down the columns `xs` of a `w` × `h` plane.
///
/// # Safety
/// No other thread touches the columns `xs` of `plane` during the call.
unsafe fn blur_cols(plane: &Shared<i32>, w: usize, h: usize, rad: usize, xs: core::ops::Range<usize>, tmp: &mut [i32]) {
	let n = (2 * rad + 1) as i32;
	for x in xs {
		for (y, cell) in tmp.iter_mut().enumerate().take(h) {
			// SAFETY: column `x` is ours alone.
			*cell = unsafe { plane.get(y * w + x) };
		}
		let at = |i: isize, tmp: &[i32]| tmp[i.clamp(0, h as isize - 1) as usize];
		let mut sum: i32 = 0;
		for i in -(rad as isize)..=(rad as isize) {
			sum += at(i, tmp);
		}
		for y in 0..h {
			// SAFETY: as above.
			unsafe { plane.set(y * w + x, sum / n) };
			sum += at(y as isize + rad as isize + 1, tmp) - at(y as isize - rad as isize, tmp);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_bright_dot_glows_and_black_stays_black_without_grain() {
		let mut f = Frame::new(160, 90);
		for y in 40..50 {
			for x in 75..85 {
				f.set(x, y, 0xFFFFFF);
			}
		}
		let mut fin = Finisher::new();
		let p = FinishParams {
			grain: 0,
			vignette: 0,
			..FinishParams::default()
		};
		fin.glow(&mut f, &p, &crate::bands::SERIAL);
		assert!(r(f.get(70, 45)) > 10, "bloom next to the dot");
		assert!(r(f.get(40, 45)) > 0, "streak along the row");
		assert_eq!(f.get(80, 5) & 0xFF, 0, "nothing far above");
		let out = fin.output(&f, 320, 180, &p, 0, &crate::bands::SERIAL);
		assert_eq!(out.w, 320);
		assert_eq!(out.get(160, 90), 0xFFFFFF);
	}

	#[test]
	fn saturating_add_matches_per_channel() {
		for &c in &[
			0x000000u32,
			0xFFFFFF,
			0x80FF01,
			0x01FE80,
			0x7F7F7F,
			0xFF0000,
			0x00FF00,
			0x0000FF,
		] {
			for d in -255..=255 {
				let ch = |v: u32| (v as i32 + d).clamp(0, 255) as u32;
				let want = (ch(r(c)) << 16) | (ch(g(c)) << 8) | ch(b(c));
				assert_eq!(add_saturating(c, d), want, "{c:06x} {d}");
			}
		}
	}

	#[test]
	fn grain_moves_every_pixel_and_letterbox_is_black() {
		let mut f = Frame::new(64, 64);
		f.fill(0x404040);
		let mut fin = Finisher::new();
		let p = FinishParams {
			letterbox: 2.0,
			vignette: 0,
			..FinishParams::default()
		};
		let a = fin.output(&f, 64, 64, &p, 1, &crate::bands::SERIAL).px.clone();
		let c = fin.output(&f, 64, 64, &p, 2, &crate::bands::SERIAL).px.clone();
		assert!(a[0..64].iter().all(|&v| v == 0), "bars");
		let moved = a[32 * 64..33 * 64]
			.iter()
			.zip(&c[32 * 64..33 * 64])
			.filter(|(x, y)| x != y)
			.count();
		assert!(moved > 50, "grain changes most pixels between frames: {moved}");
	}
}
