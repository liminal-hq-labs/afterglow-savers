// Afterglow Cracktro: a demoscene intro in the Afterglow colours, with a sine scroller.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Drawn on a small stage (240 to 480 pixels wide by detail) and enlarged with hard pixel edges, as the real thing was. Five parts,
//! each about half a minute, with a transition at every change: copper bars with bobbing stepped suns, a plasma, a rotozoomer, a
//! stepped-ring tunnel with a white-hot mouth, and twisting vector bars. The scroller runs through all of them along the lower third.
//! Every part is a function of time alone, from tables made at the start, so any frame can be drawn on its own.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{Ramp, mix, mixf, scale};
use crate::font::{self, Strip};
use crate::frame::Frame;
use crate::math::{HALF_PI, TAU, atan2, cos, floor_i, fract, round_i, saturate, sin, sqrt};
use crate::palette::Palette;
use crate::rng::Rng;
use crate::saver::palette_for;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Detail, Key, Settings};

/// SPEC section 8.
pub const DEFAULT_MESSAGE: &str = "Hello, everyone, and welcome to Afterglow for ReactOS.     Greetings fly out to the ReactOS developers, \
the Wine project, everyone keeping a 486 humming in the spare room, the DOS and Amiga sceners, the folks who recap old motherboards, \
the archivists who keep the old disks readable, and you, for leaving your machine alone long enough to see this.     Afterglow for \
ReactOS is an unofficial remaster, made by Liminal HQ. Not affiliated with the ReactOS project.     Copper bars courtesy of \
trigonometry. Plasma courtesy of more trigonometry. Press any key to return to work.          ";

const PART_LEN: f32 = 30.0;
const TRANSITION: f32 = 1.4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
	Copper,
	Plasma,
	Roto,
	Tunnel,
	Twister,
}

const PARTS: [Part; 5] = [Part::Copper, Part::Plasma, Part::Roto, Part::Tunnel, Part::Twister];

pub struct Cracktro {
	w: usize,
	h: usize,
	sw: usize,
	sh: usize,
	stage: Frame,
	other: Frame,
	pal: Palette,
	order: Vec<Part>,
	copper: Ramp,
	plasma: Ramp,
	text_ramp: Ramp,
	roto: Vec<u32>,
	tunnel_tex: Vec<u32>,
	/// Tunnel tables over twice the stage in each direction: angle (0 to 255), depth, and the mouth's glow (0 to 255).
	tun_angle: Vec<u8>,
	tun_depth: Vec<u16>,
	tun_glow: Vec<u8>,
	/// The plasma's radial term per stage pixel.
	radial: Vec<u16>,
	sine: Vec<i32>,
	strip: Strip,
	stars: Vec<(f32, f32, f32)>,
	xmap: Vec<u16>,
	reflect: bool,
}

impl Cracktro {
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> Cracktro {
		let detail = settings.detail();
		let pal = palette_for(settings);
		let mut rng = Rng::new(seed as u64 ^ 0xC0FF_EE00_0000);
		let want = match detail {
			Detail::Low => 240,
			Detail::Medium => 320,
			Detail::High => 400,
			Detail::Ultra => 480,
		};
		let sw = want.min(w).max(16);
		let sh = (sw * h / w).max(9);
		let mask = settings.get(Key::Parts);
		let mut order: Vec<Part> = PARTS
			.iter()
			.enumerate()
			.filter(|(i, _)| mask & (1 << i) != 0)
			.map(|(_, p)| *p)
			.collect();
		if order.is_empty() {
			order = PARTS.to_vec();
		}
		if settings.get(Key::Shuffle) == 1 {
			for i in (1..order.len()).rev() {
				let j = rng.below(i as u32 + 1) as usize;
				order.swap(i, j);
			}
		}
		let ground = pal.ground();
		let copper = Ramp::new(&[
			(0.0, ground),
			(0.3, mixf(ground, pal.hue(pal.glow_n - 1), 0.8)),
			(0.6, pal.hue(1)),
			(0.8, pal.hue(0)),
			(0.92, pal.sun),
			(1.0, 0xFFFFFF),
		]);
		// A cyclic ramp for the plasma: from the dark ground out through the hues to a thin band of sun and back, so most of the
		// screen is the deeper colours and the bright parts travel as thin lines.
		let n = pal.glow_n;
		let mut stops: Vec<(f32, u32)> = Vec::new();
		let deepest = mixf(ground, pal.hue(n - 1), 0.25);
		stops.push((0.0, deepest));
		stops.push((0.12, deepest));
		for i in 0..n {
			stops.push((0.12 + 0.3 * (i + 1) as f32 / n as f32, scale(pal.hue(n - 1 - i), 200)));
		}
		stops.push((0.5, pal.sun));
		for i in 0..n {
			stops.push((0.5 + 0.3 * (i + 1) as f32 / n as f32, scale(pal.hue(i), 200)));
		}
		stops.push((0.88, deepest));
		stops.push((1.0, deepest));
		let plasma = Ramp::new(&stops);
		let text_ramp = Ramp::new(&[
			(0.0, mixf(pal.sun, 0xFFFFFF, 0.7)),
			(0.35, pal.sun),
			(0.55, pal.hue(0)),
			(0.8, pal.hue(1)),
			(1.0, pal.hue(n - 1)),
		]);
		let sine = (0..1024).map(|i| round_i(sin(i as f32 * TAU / 1024.0) * 256.0)).collect();
		let roto = paint_tiles(&pal);
		let tunnel_tex = paint_rings(&pal);
		let (tw, th) = (sw * 2, sh * 2);
		let mut tun_angle = vec![0u8; tw * th];
		let mut tun_depth = vec![0u16; tw * th];
		let mut tun_glow = vec![0u8; tw * th];
		let maxd = sqrt((sw * sw + sh * sh) as f32);
		for y in 0..th {
			for x in 0..tw {
				let dx = x as f32 - sw as f32;
				let dy = (y as f32 - sh as f32) * 1.15;
				let d = sqrt(dx * dx + dy * dy).max(1.0);
				let i = y * tw + x;
				tun_angle[i] = ((atan2(dy, dx) / TAU + 0.5) * 256.0) as u8;
				tun_depth[i] = (sh as f32 * 64.0 / d).min(65_535.0) as u16;
				let g = saturate(1.0 - d / (maxd * 0.45));
				tun_glow[i] = (g * g * g * g * 255.0) as u8;
			}
		}
		let mut radial = vec![0u16; sw * sh];
		for y in 0..sh {
			for x in 0..sw {
				let dx = x as f32 - sw as f32 * 0.6;
				let dy = y as f32 - sh as f32 * 0.4;
				radial[y * sw + x] = (sqrt(dx * dx + dy * dy) * 2400.0 / sw as f32) as u16;
			}
		}
		let message: String = if settings.text.trim().is_empty() {
			String::from(DEFAULT_MESSAGE)
		} else {
			let mut m = String::from(settings.text.trim());
			m.push_str("          ");
			m
		};
		// Scrollers shout: the message is set in capitals.
		let strip = font::render(&message.to_uppercase(), (sh / 5).max(12), 0.95);
		let stars = (0..(sw / 3).max(20))
			.map(|_| (rng.f(), rng.f(), rng.range(0.2, 1.0)))
			.collect();
		let xmap = (0..w).map(|x| (x * sw / w) as u16).collect();
		Cracktro {
			w,
			h,
			sw,
			sh,
			stage: Frame::new(sw, sh),
			other: Frame::new(sw, sh),
			pal,
			order,
			copper,
			plasma,
			text_ramp,
			roto,
			tunnel_tex,
			tun_angle,
			tun_depth,
			tun_glow,
			radial,
			sine,
			strip,
			stars,
			xmap,
			reflect: detail >= Detail::High,
		}
	}

	#[inline(always)]
	fn sin8(&self, i: i32) -> i32 {
		self.sine[(i & 1023) as usize]
	}

	fn draw_part(&self, part: Part, t: f32, f: &mut Frame) {
		match part {
			Part::Copper => self.copper_bars(t, f),
			Part::Plasma => self.plasma_part(t, f),
			Part::Roto => self.rotozoom(t, f),
			Part::Tunnel => self.tunnel(t, f),
			Part::Twister => self.twister(t, f),
		}
	}

	fn starfield(&self, t: f32, f: &mut Frame, speed: f32) {
		let (sw, sh) = (self.sw as f32, self.sh as f32);
		for &(x, y, z) in &self.stars {
			let px = fract(x - t * speed * z * 0.08) * sw;
			let c = scale(self.pal.n(200), (z * 230.0) as u32);
			f.put(px as i32, (y * sh * 0.9) as i32, c);
			if z > 0.8 {
				f.put(px as i32 + 1, (y * sh * 0.9) as i32, scale(c, 120));
			}
		}
	}

	fn copper_bars(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw, self.sh);
		// A deep gradient sky.
		for y in 0..sh {
			let c = mixf(
				self.pal.ground(),
				mixf(self.pal.ground(), self.pal.hue(self.pal.glow_n - 1), 0.35),
				y as f32 / sh as f32,
			);
			f.px[y * sw..(y + 1) * sw].fill(c);
		}
		self.starfield(t, f, 1.0);
		// Seven bars on one sine, drawn back to front; the ones behind are darker.
		let mut bars: Vec<(f32, f32)> = (0..7)
			.map(|i| {
				let a = t * 1.3 + i as f32 * 0.9;
				(sh as f32 * (0.38 + 0.3 * sin(a)), cos(a))
			})
			.collect();
		bars.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
		let half = (sh as f32 * 0.05).max(3.0);
		for (cy, z) in bars {
			let bright = (0.55 + 0.45 * z) * 256.0;
			let y0 = (cy - half) as i32;
			let y1 = (cy + half) as i32;
			for y in y0.max(0)..=y1.min(sh as i32 - 1) {
				let d = 1.0 - ((y as f32 - cy) / half).abs();
				let c = scale(self.copper.at(d), bright as u32);
				f.px[y as usize * sw..(y as usize + 1) * sw].fill(c);
			}
		}
		// Bobbing stepped suns on a figure of eight.
		let r = (sh as f32 * 0.05).max(3.0);
		for i in 0..8 {
			let a = t * 0.9 + i as f32 * TAU / 8.0;
			let x = sw as f32 * (0.5 + 0.38 * sin(a));
			let y = sh as f32 * (0.36 + 0.2 * sin(2.0 * a));
			self.sun_bob(f, x, y, r);
		}
	}

	/// A stepped sun: three flat rings, hottest inside.
	fn sun_bob(&self, f: &mut Frame, cx: f32, cy: f32, r: f32) {
		let rings = [
			self.pal.hue(1),
			self.pal.hue(0),
			self.pal.sun,
			mixf(self.pal.sun, 0xFFFFFF, 0.6),
		];
		let ri = r as i32 + 1;
		for dy in -ri..=ri {
			for dx in -ri..=ri {
				let d = sqrt((dx * dx + dy * dy) as f32) / r;
				if d < 1.0 {
					let k = ((1.0 - d) * 4.0) as usize;
					f.put(cx as i32 + dx, cy as i32 + dy, rings[k.min(3)]);
				}
			}
		}
	}

	fn plasma_part(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw, self.sh);
		let k = 1024.0 / sw as f32;
		let tx = (t * 150.0) as i32;
		let ty = (t * -110.0) as i32;
		let td = (t * 80.0) as i32;
		let tr = (t * -260.0) as i32;
		let xs: Vec<i32> = (0..sw).map(|x| self.sin8((x as f32 * k * 2.1) as i32 + tx)).collect();
		let ds: Vec<i32> = (0..sw + sh).map(|d| self.sin8((d as f32 * k * 1.3) as i32 + td)).collect();
		let cycle = (t * 60.0) as i32;
		for y in 0..sh {
			let ys = self.sin8((y as f32 * k * 2.7) as i32 + ty);
			for x in 0..sw {
				let r = self.sin8(self.radial[y * sw + x] as i32 + tr);
				let v = xs[x] + ys + ds[x + y] + r;
				f.px[y * sw + x] = self.plasma.idx((v >> 2) + cycle);
			}
		}
	}

	fn rotozoom(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw, self.sh);
		let a = t * 0.35 + sin(t * 0.21) * 1.2;
		let zoom = (1.4 + 0.9 * sin(t * 0.45)) * 256.0 / sw as f32;
		let du = round_i(cos(a) * zoom * 65_536.0);
		let dv = round_i(sin(a) * zoom * 65_536.0);
		let cu = round_i((128.0 + t * 20.0) * 65_536.0);
		let cv = round_i((128.0 + sin(t * 0.3) * 60.0) * 65_536.0);
		let hx = sw as i32 / 2;
		let hy = sh as i32 / 2;
		for y in 0..sh as i32 {
			let mut u = cu.wrapping_sub(hx.wrapping_mul(du)).wrapping_sub((y - hy).wrapping_mul(dv));
			let mut v = cv.wrapping_sub(hx.wrapping_mul(dv)).wrapping_add((y - hy).wrapping_mul(du));
			let row = &mut f.px[y as usize * sw..(y as usize + 1) * sw];
			for px in row.iter_mut() {
				*px = self.roto[((((v >> 16) & 255) << 8) | ((u >> 16) & 255)) as usize];
				u = u.wrapping_add(du);
				v = v.wrapping_add(dv);
			}
		}
	}

	fn tunnel(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw, self.sh);
		let tw = sw * 2;
		let ox = ((sw as f32 * 0.5) * (1.0 + 0.45 * sin(t * 0.55))) as usize;
		let oy = ((sh as f32 * 0.5) * (1.0 + 0.45 * sin(t * 0.7 + 1.0))) as usize;
		let spin = (t * 24.0) as i32;
		let travel = (t * 90.0) as i32;
		let hot = mixf(self.pal.sun, 0xFFFFFF, 0.7);
		for y in 0..sh {
			let base = (y + oy) * tw + ox;
			for x in 0..sw {
				let i = base + x;
				let u = (self.tun_angle[i] as i32 + spin) & 255;
				let v = (self.tun_depth[i] as i32 + travel) & 255;
				let c = self.tunnel_tex[(v << 8 | u) as usize];
				let g = self.tun_glow[i] as u32;
				// Far away is dark at the walls but white-hot at the mouth: shade by depth, then add the glow.
				let shade = 256 - (g >> 1);
				f.px[y * sw + x] = mix(scale(c, shade), hot, g);
			}
		}
	}

	fn twister(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw, self.sh);
		for y in 0..sh {
			let c = mixf(
				self.pal.ground(),
				mixf(self.pal.ground(), self.pal.hue(0), 0.12),
				y as f32 / sh as f32,
			);
			f.px[y * sw..(y + 1) * sw].fill(c);
		}
		self.starfield(t, f, -0.6);
		let hues = [self.pal.hue(0), self.pal.hue(1), self.pal.hue(2), self.pal.sun];
		for (k, centre) in [0.3f32, 0.7].into_iter().enumerate() {
			let dir = if k == 0 { 1.0 } else { -1.0 };
			for y in 0..sh {
				let yf = y as f32;
				let a = t * 1.5 * dir + sin(yf * 0.021 + t * 0.9 + k as f32) * 1.9;
				let cx = sw as f32 * (centre + 0.06 * sin(yf * 0.013 + t * 0.6 + k as f32 * 2.0));
				let r = sw as f32 * 0.09;
				let xs: [f32; 4] = core::array::from_fn(|i| cx + r * sin(a + i as f32 * HALF_PI));
				for i in 0..4 {
					let (x0, x1) = (xs[i], xs[(i + 1) % 4]);
					if x1 <= x0 {
						continue;
					}
					// A face is brighter the more squarely it faces the viewer, which is how wide it is.
					let facing = (x1 - x0) / (r * 1.414);
					let c = scale(hues[i], (60.0 + 196.0 * facing) as u32);
					let (a0, a1) = (x0 as i32, x1 as i32);
					for x in a0.max(0)..a1.min(sw as i32) {
						let edge = x == a0 || x == a1 - 1;
						f.px[y * sw + x as usize] = if edge { scale(c, 150) } else { c };
					}
				}
			}
		}
		self.cube_ring(t, f);
	}

	/// A ring of six flat-shaded cubes orbiting between the twisters on a tilted circle: back faces culled, faces drawn far to
	/// near, each lit by how squarely it faces a light from the upper left, in the palette's hues.
	fn cube_ring(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw as f32, self.sh as f32);
		let corners: [[f32; 3]; 8] = [
			[-1.0, -1.0, -1.0],
			[1.0, -1.0, -1.0],
			[1.0, 1.0, -1.0],
			[-1.0, 1.0, -1.0],
			[-1.0, -1.0, 1.0],
			[1.0, -1.0, 1.0],
			[1.0, 1.0, 1.0],
			[-1.0, 1.0, 1.0],
		];
		const FACES: [[usize; 4]; 6] = [
			[0, 3, 2, 1],
			[4, 5, 6, 7],
			[0, 1, 5, 4],
			[2, 3, 7, 6],
			[1, 2, 6, 5],
			[0, 4, 7, 3],
		];
		let light = [-0.5f32, -0.6, -0.62];
		let mut quads: Vec<Quad> = Vec::new();
		for k in 0..6 {
			let a = t * 0.7 + k as f32 * TAU / 6.0;
			// The ring: a circle tilted towards the viewer, centred on the screen.
			let (rx, rz) = (cos(a) * 2.6, sin(a) * 2.6);
			let (ry, rz) = (rz * 0.35, rz * 0.94 + 7.0);
			let spin = t * 1.3 + k as f32;
			let (cs, ss) = (cos(spin), sin(spin));
			let (ct, st) = (cos(spin * 0.7), sin(spin * 0.7));
			let hue = self.pal.hue(k % self.pal.glow_n.max(1));
			let world: Vec<[f32; 3]> = corners
				.iter()
				.map(|c| {
					let (x, y, z) = (c[0] * 0.42, c[1] * 0.42, c[2] * 0.42);
					let (x, z) = (x * cs + z * ss, -x * ss + z * cs);
					let (y, z) = (y * ct - z * st, y * st + z * ct);
					[x + rx, y + ry, z + rz]
				})
				.collect();
			for face in FACES {
				let p = |i: usize| world[face[i]];
				let (a3, b3, c3) = (p(0), p(1), p(2));
				let u = [b3[0] - a3[0], b3[1] - a3[1], b3[2] - a3[2]];
				let v = [c3[0] - a3[0], c3[1] - a3[1], c3[2] - a3[2]];
				let n = [
					u[1] * v[2] - u[2] * v[1],
					u[2] * v[0] - u[0] * v[2],
					u[0] * v[1] - u[1] * v[0],
				];
				// Back faces point away from the eye at the origin.
				if n[0] * a3[0] + n[1] * a3[1] + n[2] * a3[2] >= 0.0 {
					continue;
				}
				let len = sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).max(1e-6);
				let lit = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / len).max(0.0);
				let shade = (70.0 + 186.0 * lit) as u32;
				let depth = (0..4).map(|i| p(i)[2]).sum::<f32>();
				let pts = [0, 1, 2, 3].map(|i| {
					let q = p(i);
					(sw * 0.5 + q[0] / q[2] * sh * 0.9, sh * 0.42 + q[1] / q[2] * sh * 0.9)
				});
				quads.push((depth, pts, scale(hue, shade)));
			}
		}
		quads.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(core::cmp::Ordering::Equal));
		for (_, pts, c) in quads {
			fill_triangle(f, pts[0], pts[1], pts[2], c);
			fill_triangle(f, pts[0], pts[2], pts[3], c);
		}
	}

	/// The scroller: the text strip read column by column, each column lifted by a sine, filled with a copper gradient, with a
	/// shadow, and at High a reflection in a strip of water at the bottom.
	fn scroller(&self, t: f32, f: &mut Frame) {
		let (sw, sh) = (self.sw, self.sh);
		let s = &self.strip;
		let period = s.w + sw;
		let scroll = (t * sw as f32 * 0.32) as usize;
		let amp = sh as f32 * 0.06;
		let base = sh as f32 * 0.70;
		let shadow = scale(self.pal.ground(), 128);
		for x in 0..sw {
			let col = (scroll + x) % period;
			if col >= s.w {
				continue;
			}
			let top = (base + amp * sin(x as f32 * 0.04 + t * 2.6)) as i32;
			for r in 0..s.h {
				let a = s.alpha[r * s.w + col] as u32;
				if a == 0 {
					continue;
				}
				let y = top + r as i32;
				// Shadow down and right first, then the face.
				let (sx, sy) = (x as i32 + 1, y + 2);
				if sx < sw as i32 && sy >= 0 && sy < sh as i32 {
					let i = sy as usize * sw + sx as usize;
					f.px[i] = mix(f.px[i], shadow, a);
				}
				if y >= 0 && y < sh as i32 {
					let i = y as usize * sw + x;
					let c = self.text_ramp.at(r as f32 / s.h as f32);
					f.px[i] = mix(f.px[i], c, a);
				}
			}
		}
		if self.reflect {
			let water = (sh as f32 * 0.9) as usize;
			for y in water..sh {
				let d = y - water;
				let src = water.saturating_sub(d * 2 + 1);
				let wob = (sin(y as f32 * 0.9 + t * 5.0) * 1.5) as i32;
				for x in 0..sw {
					let sx = (x as i32 + wob).clamp(0, sw as i32 - 1) as usize;
					let c = f.px[src * sw + sx];
					f.px[y * sw + x] = mix(scale(c, 150), self.pal.hue(self.pal.glow_n - 1), 40);
				}
			}
		}
	}
}

/// The rotozoomer's texture: 256 × 256 of inlaid tiles, each a stepped sun or a diamond, with dark grout.
fn paint_tiles(pal: &Palette) -> Vec<u32> {
	let mut tex = vec![0u32; 256 * 256];
	let n = pal.glow_n;
	for y in 0..256usize {
		for x in 0..256usize {
			let (tx, ty) = (x / 32, y / 32);
			let (lx, ly) = ((x % 32) as f32 - 15.5, (y % 32) as f32 - 15.5);
			let base = if (tx + ty) % 2 == 0 {
				mixf(pal.ground(), pal.hue(n - 1), 0.45)
			} else {
				mixf(pal.ground(), pal.hue(1), 0.35)
			};
			let grout = (x % 32) < 2 || (y % 32) < 2;
			let c = if grout {
				pal.ground()
			} else if (tx * 3 + ty) % 2 == 0 {
				let d = sqrt(lx * lx + ly * ly);
				match d as i32 {
					0..=4 => mixf(pal.sun, 0xFFFFFF, 0.4),
					5..=7 => pal.sun,
					8..=10 => pal.hue(0),
					11..=12 => pal.hue(1 % n),
					_ => base,
				}
			} else {
				let d = lx.abs() + ly.abs();
				match d as i32 {
					0..=5 => pal.hue(0),
					6..=9 => pal.hue(2 % n),
					10..=11 => mixf(pal.sun, 0xFFFFFF, 0.2),
					_ => base,
				}
			};
			tex[y * 256 + x] = c;
		}
	}
	tex
}

/// The tunnel's walls: stepped rings of the palette along the depth, panels around the angle.
fn paint_rings(pal: &Palette) -> Vec<u32> {
	let mut tex = vec![0u32; 256 * 256];
	let n = pal.glow_n;
	let ring_cols: Vec<u32> = (0..8)
		.map(|i| match i % 4 {
			0 => pal.hue(0),
			1 => pal.hue(1 % n),
			2 => pal.hue(2 % n),
			_ => mixf(pal.ground(), pal.hue(n - 1), 0.5),
		})
		.collect();
	for v in 0..256usize {
		for u in 0..256usize {
			let ring = v / 32;
			let within = v % 32;
			let mut c = ring_cols[ring];
			// Each ring is three steps, darker towards its far edge.
			c = scale(c, 256 - (within as u32 / 11) * 60);
			if within < 2 || u % 32 < 1 {
				c = scale(c, 90);
			}
			tex[v * 256 + u] = c;
		}
	}
	tex
}

impl Scene for Cracktro {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		let count = self.order.len();
		let index = floor_i(t / PART_LEN).max(0) as usize;
		let part = self.order[index % count];
		let local = t - index as f32 * PART_LEN;
		let mut stage = core::mem::replace(&mut self.stage, Frame::new(1, 1));
		self.draw_part(part, t, &mut stage);
		if index > 0 && local < TRANSITION && count > 1 {
			let prev = self.order[(index - 1) % count];
			let mut other = core::mem::replace(&mut self.other, Frame::new(1, 1));
			self.draw_part(prev, t, &mut other);
			transition(&mut stage, &other, local / TRANSITION, index % 3, t);
			self.other = other;
		}
		self.scroller(t, &mut stage);
		// Enlarge with hard pixel edges.
		let (sw, sh, h) = (self.sw, self.sh, self.h);
		let (stage_px, xmap) = (&stage.px, &self.xmap);
		crate::bands::rows(cx.bands, &mut out.px, self.w, &|band| {
			for y in band.y0..band.y1 {
				let sy = y * sh / h;
				let src = &stage_px[sy * sw..(sy + 1) * sw];
				for (px, &sx) in band.row(y).iter_mut().zip(xmap) {
					*px = src[sx as usize];
				}
			}
		});
		self.stage = stage;
	}

	fn cues(&mut self, t: f64) -> crate::audio::Cues {
		let tf = t as f32;
		let index = floor_i(tf / PART_LEN).max(0) as usize;
		let part = self.order[index % self.order.len()];
		crate::audio::Cues {
			t,
			fade: 1.0,
			shot: PARTS.iter().position(|&p| p == part).unwrap_or(0),
			shot_t: tf - index as f32 * PART_LEN,
			member: index,
			..crate::audio::Cues::default()
		}
	}

	fn hints(&self, cx: &Ctx) -> Hints {
		let t = cx.t as f32;
		let part = self.order[(floor_i(t / PART_LEN).max(0) as usize) % self.order.len()];
		Hints {
			threshold: if part == Part::Plasma { 200 } else { 160 },
			streak_tint: mixf(self.pal.sun, 0xFFFFFF, 0.3),
			fade: 1.0,
			bloom_gain: if part == Part::Plasma { 0.5 } else { 0.9 },
		}
	}
}

/// A face to draw: its depth (larger is further), its corners on screen, its colour.
type Quad = (f32, [(f32, f32); 4], u32);

/// Fills a triangle with one colour, scanline by scanline, pixel centres inside its edges.
fn fill_triangle(f: &mut Frame, a: (f32, f32), b: (f32, f32), c: (f32, f32), colour: u32) {
	let mut v = [a, b, c];
	v.sort_by(|p, q| p.1.partial_cmp(&q.1).unwrap_or(core::cmp::Ordering::Equal));
	let [(x0, y0), (x1, y1), (x2, y2)] = v;
	let edge = |ya: f32, yb: f32, xa: f32, xb: f32, y: f32| {
		if (yb - ya).abs() < 1e-6 {
			xa
		} else {
			xa + (xb - xa) * (y - ya) / (yb - ya)
		}
	};
	let top = (y0 + 0.5).max(0.0) as i32;
	let bottom = (crate::math::floor_i(y2 - 0.5) + 1).min(f.h as i32 - 1);
	for y in top..=bottom {
		let yc = y as f32 + 0.5;
		if yc < y0 || yc > y2 {
			continue;
		}
		let long = edge(y0, y2, x0, x2, yc);
		let short = if yc < y1 {
			edge(y0, y1, x0, x1, yc)
		} else {
			edge(y1, y2, x1, x2, yc)
		};
		let (l, r) = if long < short { (long, short) } else { (short, long) };
		let (xa, xb) = (((l + 0.5) as i32).max(0), ((r - 0.5) as i32).min(f.w as i32 - 1));
		for x in xa..=xb {
			f.px[y as usize * f.w + x as usize] = colour;
		}
	}
}

/// Mixes the outgoing part `old` into `new` by a pattern: venetian raster lines, an ordered dissolve, or a copper curtain.
fn transition(new: &mut Frame, old: &Frame, k: f32, kind: usize, t: f32) {
	let (w, h) = (new.w, new.h);
	const BAYER: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
	for y in 0..h {
		for x in 0..w {
			let show_new = match kind {
				0 => ((y * 37) % 16) as f32 / 16.0 < k,
				1 => (BAYER[(y & 3) * 4 + (x & 3)] as f32 + 0.5) / 16.0 < k * 1.02,
				_ => (y as f32) < k * (h as f32 * 1.3) - h as f32 * 0.15 + sin(x as f32 * 0.08 + t * 3.0) * h as f32 * 0.08,
			};
			if !show_new {
				new.px[y * w + x] = old.px[y * w + x];
			}
		}
	}
}
