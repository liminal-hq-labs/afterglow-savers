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
	tmp: Vec<i32>,
	glow: Vec<u32>,
	row: Vec<u32>,
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
			tmp: Vec::new(),
			glow: Vec::new(),
			row: Vec::new(),
			grain,
			vx: Vec::new(),
			vy: Vec::new(),
			out: Frame::new(1, 1),
		}
	}

	/// Lays the glow over `frame` in place.
	pub fn glow(&mut self, frame: &mut Frame, p: &FinishParams) {
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
			self.tmp.clear();
			self.tmp.resize(qw.max(qh) + 2, 0);
		}
		// The bright pass, at the coarse size: two samples per block, the part of the luma above the threshold, kept in colour.
		let thr = p.threshold.min(250) as i32;
		let span = 255 - thr;
		let half = step / 2;
		for qy in 0..qh {
			let y0 = (qy * step + half / 2).min(frame.h - 1);
			let y1 = (qy * step + half + half / 2).min(frame.h - 1);
			for qx in 0..qw {
				let x0 = (qx * step + half / 2).min(frame.w - 1);
				let x1 = (qx * step + half + half / 2).min(frame.w - 1);
				let c0 = frame.px[y0 * frame.w + x0];
				let c1 = frame.px[y1 * frame.w + x1];
				let (rr, gg, bb) = ((r(c0) + r(c1)) as i32, (g(c0) + g(c1)) as i32, (b(c0) + b(c1)) as i32);
				let l = (rr * 77 + gg * 150 + bb * 29) >> 9;
				let i = qy * qw + qx;
				let k = (l - thr).max(0);
				// Each channel times how far over the threshold the luma is, 0 to 510 per channel.
				self.red[i] = rr * k / span;
				self.green[i] = gg * k / span;
				self.blue[i] = bb * k / span;
				self.streak[i] = (k * k / span) * 2;
			}
		}
		// Bloom: two box blurs each way, radius about 2% of the height.
		let radius = (qh / 24).max(1);
		for plane in [&mut self.red, &mut self.green, &mut self.blue] {
			for _ in 0..2 {
				blur_rows(plane, qw, qh, radius, &mut self.tmp);
				blur_cols(plane, qw, qh, radius, &mut self.tmp);
			}
		}
		// Streaks: three long horizontal blurs, about a quarter of the width each.
		if p.streaks > 0 {
			let long = (qw / 7).max(2);
			for _ in 0..3 {
				blur_rows(&mut self.streak, qw, qh, long, &mut self.tmp);
			}
			blur_cols(&mut self.streak, qw, qh, 1, &mut self.tmp);
		}
		// Pack the glow: bloom in its own colours, streaks in the tint.
		let bk = (p.bloom.min(100) * 3) as i32;
		let sk = (p.streaks.min(100) * 5) as i32;
		let (tr, tg, tb) = (r(p.streak_tint) as i32, g(p.streak_tint) as i32, b(p.streak_tint) as i32);
		for i in 0..qw * qh {
			let s = self.streak[i] * sk / 100;
			let rr = (self.red[i] * bk / 200 + s * tr / 255).clamp(0, 255) as u32;
			let gg = (self.green[i] * bk / 200 + s * tg / 255).clamp(0, 255) as u32;
			let bb = (self.blue[i] * bk / 200 + s * tb / 255).clamp(0, 255) as u32;
			self.glow[i] = (rr << 16) | (gg << 8) | bb;
		}
		// Lay it over the frame: bilinear through a row cache, screen blend.
		self.row.clear();
		self.row.resize(qw, 0);
		let fy_step = ((qh as i64) << 16) / frame.h as i64;
		let fx_step = ((qw as i64) << 16) / frame.w as i64;
		for y in 0..frame.h {
			let sy = (y as i64 * fy_step - 32_768).max(0) as i32;
			let y0 = ((sy >> 16) as usize).min(qh - 1);
			let y1 = (y0 + 1).min(qh - 1);
			let ty = ((sy >> 8) & 0xFF) as u32;
			for x in 0..qw {
				self.row[x] = mix(self.glow[y0 * qw + x], self.glow[y1 * qw + x], ty);
			}
			let line = &mut frame.px[y * frame.w..(y + 1) * frame.w];
			let mut sx = -32_768i32;
			for px in line.iter_mut() {
				let xs = sx.max(0);
				let x0 = ((xs >> 16) as usize).min(qw - 1);
				let x1 = (x0 + 1).min(qw - 1);
				let gl = mix(self.row[x0], self.row[x1], ((xs >> 8) & 0xFF) as u32);
				if gl != 0 {
					*px = screen(*px, gl);
				}
				sx += fx_step as i32;
			}
		}
	}

	/// Enlarges `src` into `self.out` at `ow` × `oh` with the vignette, fade, letterbox and grain. `frame_no` moves the grain.
	pub fn output(&mut self, src: &Frame, ow: usize, oh: usize, p: &FinishParams, frame_no: u32) -> &Frame {
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
		for y in 0..oh {
			let line = &mut self.out.px[y * ow..(y + 1) * ow];
			if y < top || y >= bottom {
				line.fill(0);
				continue;
			}
			let vy = self.vy[y] * fade >> 8;
			let mut gi = hash32(frame_no.wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B)) as usize;
			if same {
				let srow = &src.px[y * ow..(y + 1) * ow];
				for ((px, &c), &vx) in line.iter_mut().zip(srow).zip(&self.vx) {
					let k = vx * vy >> 8;
					let gr = self.grain[gi & mask] as i32 * amp >> 8;
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
				for (px, &vx) in line.iter_mut().zip(&self.vx) {
					let xs = sx.max(0);
					let x0 = ((xs >> 16) as usize).min(src.w - 1);
					let x1 = (x0 + 1).min(src.w - 1);
					let tx = ((xs >> 8) & 0xFF) as u32;
					let c = mix(mix(r0[x0], r0[x1], tx), mix(r1[x0], r1[x1], tx), ty);
					let k = vx * vy >> 8;
					let gr = self.grain[gi & mask] as i32 * amp >> 8;
					gi += 1;
					*px = finish_px(c, k, gr);
					sx += sx_step;
				}
			}
		}
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
	let ch = |v: u32| (v as i32 + d).clamp(0, 255) as u32;
	(ch(r(c)) << 16) | (ch(g(c)) << 8) | ch(b(c))
}

/// Box blur along each row, radius `rad`, through a running sum, with the edges clamped.
fn blur_rows(plane: &mut [i32], w: usize, h: usize, rad: usize, tmp: &mut [i32]) {
	let n = (2 * rad + 1) as i32;
	for y in 0..h {
		let row = &mut plane[y * w..(y + 1) * w];
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

/// Box blur down each column.
fn blur_cols(plane: &mut [i32], w: usize, h: usize, rad: usize, tmp: &mut [i32]) {
	let n = (2 * rad + 1) as i32;
	for x in 0..w {
		for y in 0..h {
			tmp[y] = plane[y * w + x];
		}
		let at = |i: isize, tmp: &[i32]| tmp[i.clamp(0, h as isize - 1) as usize];
		let mut sum: i32 = 0;
		for i in -(rad as isize)..=(rad as isize) {
			sum += at(i, tmp);
		}
		for y in 0..h {
			plane[y * w + x] = sum / n;
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
		fin.glow(&mut f, &p);
		assert!(r(f.get(70, 45)) > 10, "bloom next to the dot");
		assert!(r(f.get(40, 45)) > 0, "streak along the row");
		assert_eq!(f.get(80, 5) & 0xFF, 0, "nothing far above");
		let out = fin.output(&f, 320, 180, &p, 0);
		assert_eq!(out.w, 320);
		assert_eq!(out.get(160, 90), 0xFFFFFF);
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
		let a = fin.output(&f, 64, 64, &p, 1).px.clone();
		let c = fin.output(&f, 64, 64, &p, 2).px.clone();
		assert!(a[0..64].iter().all(|&v| v == 0), "bars");
		let moved = a[32 * 64..33 * 64]
			.iter()
			.zip(&c[32 * 64..33 * 64])
			.filter(|(x, y)| x != y)
			.count();
		assert!(moved > 50, "grain changes most pixels between frames: {moved}");
	}
}
