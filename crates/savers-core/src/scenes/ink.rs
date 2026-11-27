// Afterglow Ink: marbled ink on dark water, folded by a slowly changing flow.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Two parts (docs/scenes.md, "Ink"):
//!
//! - **The sheet.** A tiling marbling pattern made with the exact marbling transforms: each ink drop of radius r at c pushes every
//!   point p outwards to c + (p - c)·√(1 + r²/|p - c|²), and a comb shifts points along its tines by z·λ^d, d being the distance to
//!   the nearest tine. Evaluated backwards per texel: undo the operations from the last, and the first drop the point falls inside
//!   gives its colour. The pattern is therefore crisp at any size and needs no simulation.
//! - **The flow.** Each screen pixel remembers where on the sheet its ink came from (16.16 sheet coordinates). Every frame a pixel
//!   looks back along the flow (drifting vortices, a shear, and in the aurora phase a vertical lift, all divergence-free) and takes
//!   the sheet position found there, so the ink folds into ever finer filaments without blurring. Two such layers run half a cycle
//!   apart and cross-fade, and each starts afresh on its own sheet while it is invisible.

use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{self, mix, mixf, scale};
use crate::frame::Frame;
use crate::math::{TAU, cos, exp, floor, fract, sin, smoothstep};
use crate::noise::noise2;
use crate::palette::Palette;
use crate::rng::Rng;
use crate::saver::palette_for;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Detail, Key, Settings};

/// One cycle of the storyboard, in scene seconds: a layer lives for two of these, fading in and out.
const CYCLE: f32 = 60.0;
/// Velocity grid spacing in render pixels at Ultra (others use twice this).
const GRID_FINE: usize = 8;

/// A tiling marbling sheet.
struct Sheet {
	size: usize,
	shift: u32,
	px: Vec<u32>,
	/// The marbling operations, kept until every row is painted.
	ops: Vec<Op>,
	/// The next row to paint; `size` when the sheet is done.
	next_row: usize,
}

enum Op {
	Drop {
		cx: f32,
		cy: f32,
		r2: f32,
		colour: u32,
	},
	/// A comb along x (tines are horizontal lines when `vertical` is false) or y.
	Comb {
		vertical: bool,
		spacing: f32,
		offset: f32,
		amp: f32,
		falloff: f32,
	},
	/// A sinusoidal stroke: shifts along x by amp·sin(2π·y/len) (or along y with x).
	Wave {
		vertical: bool,
		len: f32,
		phase: f32,
		amp: f32,
	},
}

impl Sheet {
	fn new(size: usize, pal: &Palette, thickness: f32, rng: &mut Rng) -> Sheet {
		let s = size as f32;
		let mut ops: Vec<Op> = Vec::new();
		// Inks: the palette's glow hues and sun, a pale vein, and the water itself; more water when the ink is thin.
		let mut inks: Vec<u32> = Vec::new();
		for i in 0..pal.glow_n {
			inks.push(if pal.light { pal.deep[i % 3] } else { pal.hue(i) });
		}
		inks.push(if pal.light { pal.deep[0] } else { pal.sun });
		let pale = if pal.light {
			pal.n(300)
		} else {
			mixf(pal.n(200), pal.sun, 0.35)
		};
		let water = |rng: &mut Rng| {
			if pal.light {
				mixf(pal.ground(), pal.n(700), rng.f() * 0.5)
			} else {
				mixf(pal.ground(), pal.n(700), rng.f() * 0.6)
			}
		};
		// Several pours: a cluster of concentric drops at one place, alternating ink and water, like a marbler's stone pattern.
		let pours = 5 + rng.below(4);
		for _ in 0..pours {
			let (cx, cy) = (rng.f() * s, rng.f() * s);
			let rings = 4 + rng.below(5);
			let base = s * rng.range(0.04, 0.09);
			for k in 0..rings {
				let ink = rng.f() < 0.3 + 0.45 * thickness;
				// The sun hue is the hottest ink: rare, and only in thin rings, so it reads as a glint rather than a field.
				let hot = ink && rng.f() < 0.12;
				let colour = if hot {
					inks[inks.len() - 1]
				} else if ink {
					// A glow hue, or a blend towards its neighbour (coral between orange and rose, magenta between rose and purple).
					let i = rng.below(pal.glow_n as u32) as usize;
					let next = inks[(i + 1) % pal.glow_n];
					colour::shade(mixf(inks[i], next, rng.f() * rng.f() * 0.6), rng.range(-0.08, 0.06))
				} else if rng.f() < 0.2 {
					pale
				} else {
					water(rng)
				};
				let size = if hot {
					0.35
				} else if ink {
					0.55 + 0.8 * thickness
				} else {
					0.7 + 0.9 * (1.0 - thickness)
				};
				let r = base * size * rng.range(0.7, 1.2);
				let jitter = s * 0.01 * k as f32;
				ops.push(Op::Drop {
					cx: cx + rng.signed() * jitter,
					cy: cy + rng.signed() * jitter,
					r2: r * r,
					colour,
				});
			}
			// A few scattered small drops between pours.
			for _ in 0..3 {
				let r = s * rng.range(0.01, 0.03);
				let colour = if rng.f() < 0.5 {
					inks[rng.below(inks.len() as u32) as usize]
				} else {
					pale
				};
				ops.push(Op::Drop {
					cx: rng.f() * s,
					cy: rng.f() * s,
					r2: r * r,
					colour,
				});
			}
		}
		// Then the combing: waves and combs along both axes, each a whole number of repeats across the sheet so it tiles.
		let strokes = 2 + rng.below(3);
		for _ in 0..strokes {
			let vertical = rng.f() < 0.5;
			if rng.f() < 0.55 {
				let repeats = 1 + rng.below(3);
				ops.push(Op::Wave {
					vertical,
					len: s / repeats as f32,
					phase: rng.f() * TAU,
					amp: s * rng.range(0.03, 0.08),
				});
			} else {
				let tines = 2 + rng.below(4);
				ops.push(Op::Comb {
					vertical,
					spacing: s / tines as f32,
					offset: rng.f() * s,
					amp: s * rng.range(0.05, 0.12) * if rng.f() < 0.5 { 1.0 } else { -1.0 },
					falloff: rng.range(0.02, 0.06) * 64.0 / s,
				});
			}
		}
		let ground = if pal.light { pal.ground() } else { pal.n(900) };
		Sheet {
			size,
			shift: size.trailing_zeros(),
			px: vec![ground; size * size],
			ops,
			next_row: 0,
		}
	}

	/// Paints up to `rows` more rows: each texel undoes the operations from the last, and the first drop it lands in is its
	/// colour.
	fn paint(&mut self, rows: usize) {
		let size = self.size;
		let s = size as f32;
		let half = s * 0.5;
		let wrap = |d: f32| {
			if d > half {
				d - s
			} else if d < -half {
				d + s
			} else {
				d
			}
		};
		let end = (self.next_row + rows).min(size);
		for i in self.next_row * size..end * size {
			let texel = &mut self.px[i];
			let mut x = (i % size) as f32 + 0.5;
			let mut y = (i / size) as f32 + 0.5;
			for op in self.ops.iter().rev() {
				match *op {
					Op::Drop { cx, cy, r2, colour } => {
						let dx = wrap(x - cx);
						let dy = wrap(y - cy);
						let d2 = dx * dx + dy * dy;
						if d2 < r2 {
							*texel = colour;
							break;
						}
						// √(1 - r²/d²): a series far from the drop, one Newton step from a bit-level guess near it. A part in a
						// thousand is far below a texel.
						let q = r2 / d2;
						let k = if q < 0.06 {
							1.0 - q * (0.5 + 0.125 * q)
						} else {
							let a = 1.0 - q;
							let g = f32::from_bits((a.to_bits() >> 1).wrapping_add(0x1FBD_1DF5));
							0.5 * (g + a / g)
						};
						x = cx + dx * k;
						y = cy + dy * k;
					}
					Op::Comb {
						vertical,
						spacing,
						offset,
						amp,
						falloff,
					} => {
						let across = if vertical { x } else { y };
						let d = (fract((across - offset) / spacing + 0.5) - 0.5).abs() * spacing;
						let fall = d * falloff;
						// e^-x for the comb's falloff, as a cheap rational form; the shape matters, not the exact curve.
						let shift = amp / (1.0 + fall + 0.5 * fall * fall + 0.17 * fall * fall * fall);
						if vertical {
							y -= shift;
						} else {
							x -= shift;
						}
					}
					Op::Wave {
						vertical,
						len,
						phase,
						amp,
					} => {
						if vertical {
							y -= amp * sin(TAU * x / len + phase);
						} else {
							x -= amp * sin(TAU * y / len + phase);
						}
					}
				}
				x -= s * floor(x / s);
				y -= s * floor(y / s);
			}
		}
		self.next_row = end;
		if end == size {
			self.ops = Vec::new();
		}
	}

	/// The colour at 16.16 sheet coordinates, wrapped; bilinear when `smooth`.
	#[inline(always)]
	fn at(&self, u: i32, v: i32, smooth: bool) -> u32 {
		let m = self.size as i32 - 1;
		let x0 = (u >> 16) & m;
		let y0 = (v >> 16) & m;
		if !smooth {
			return self.px[((y0 << self.shift) | x0) as usize];
		}
		let x1 = (x0 + 1) & m;
		let y1 = (y0 + 1) & m;
		let fx = ((u >> 8) & 0xFF) as u32;
		let fy = ((v >> 8) & 0xFF) as u32;
		let r0 = (y0 << self.shift) as usize;
		let r1 = (y1 << self.shift) as usize;
		let top = mix(self.px[r0 + x0 as usize], self.px[r0 + x1 as usize], fx);
		let bot = mix(self.px[r1 + x0 as usize], self.px[r1 + x1 as usize], fx);
		mix(top, bot, fy)
	}
}

/// One advected layer: a map from screen pixels to sheet coordinates, and the sheet it reads.
struct Layer {
	u: Vec<i32>,
	v: Vec<i32>,
	nu: Vec<i32>,
	nv: Vec<i32>,
	sheet: Sheet,
	/// The fresh map: sheet = origin + x·ax + y·ay (16.16 per render pixel).
	origin: (i32, i32),
	ax: (i32, i32),
	ay: (i32, i32),
	/// Which cycle this layer was last reset for.
	epoch: i64,
}

impl Layer {
	fn new(w: usize, h: usize, sheet: Sheet) -> Layer {
		let n = w * h;
		Layer {
			u: vec![0; n],
			v: vec![0; n],
			nu: vec![0; n],
			nv: vec![0; n],
			sheet,
			origin: (0, 0),
			ax: (65_536, 0),
			ay: (0, 65_536),
			epoch: i64::MIN,
		}
	}

	/// Starts afresh: a new place, angle and size on the sheet.
	fn reset(&mut self, w: usize, h: usize, rng: &mut Rng) {
		let s = self.sheet.size as f32;
		let angle = rng.f() * TAU;
		// The sheet spans about 1.1 widths of the screen, so its tiling is hidden by the angle and the folds.
		let k = s / (w as f32 * rng.range(0.9, 1.3));
		let (ca, sa) = (cos(angle) * k, sin(angle) * k);
		self.ax = ((ca * 65_536.0) as i32, (sa * 65_536.0) as i32);
		self.ay = ((-sa * 65_536.0) as i32, (ca * 65_536.0) as i32);
		self.origin = ((rng.f() * s * 65_536.0) as i32, (rng.f() * s * 65_536.0) as i32);
		for y in 0..h {
			for x in 0..w {
				let (u, v) = self.fresh(x as i32, y as i32);
				self.u[y * w + x] = u;
				self.v[y * w + x] = v;
			}
		}
	}

	/// The fresh map at a pixel position.
	#[inline(always)]
	fn fresh(&self, x: i32, y: i32) -> (i32, i32) {
		(
			self.origin
				.0
				.wrapping_add(x.wrapping_mul(self.ax.0))
				.wrapping_add(y.wrapping_mul(self.ay.0)),
			self.origin
				.1
				.wrapping_add(x.wrapping_mul(self.ax.1))
				.wrapping_add(y.wrapping_mul(self.ay.1)),
		)
	}

	/// The fresh map at a 16.16 position.
	#[inline(always)]
	fn fresh_fx(&self, sx: i32, sy: i32) -> (i32, i32) {
		let mulfx = |a: i32, b: i32| ((a as i64 * b as i64) >> 16) as i32;
		(
			self.origin
				.0
				.wrapping_add(mulfx(sx, self.ax.0))
				.wrapping_add(mulfx(sy, self.ay.0)),
			self.origin
				.1
				.wrapping_add(mulfx(sx, self.ax.1))
				.wrapping_add(mulfx(sy, self.ay.1)),
		)
	}
}

struct Vortex {
	/// Lissajous path: centre, radii and rates.
	path: [f32; 6],
	strength: f32,
	core: f32,
}

pub struct Ink {
	w: usize,
	h: usize,
	layers: [Layer; 2],
	rng: Rng,
	vortices: Vec<Vortex>,
	turbulence: f32,
	grid: usize,
	gw: usize,
	gh: usize,
	vx: Vec<i32>,
	vy: Vec<i32>,
	rowx: Vec<i32>,
	rowy: Vec<i32>,
	/// Per-column shimmer of the aurora rays, 0 to 256.
	rays: Vec<u32>,
	seed: u32,
	aurora: f32,
	pal: Palette,
	smooth: bool,
}

impl Ink {
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> Ink {
		let detail = settings.detail();
		let pal = palette_for(settings);
		let thickness = settings.get(Key::Thickness) as f32 / 100.0;
		let turbulence = settings.get(Key::Turbulence) as f32 / 100.0;
		let mut rng = Rng::new(seed as u64 ^ 0x1A4B_0000_0000);
		let size = if detail <= Detail::Low { 256 } else { 512 };
		// Layer B is the one on screen first; layer A's sheet is painted a few rows a frame while its weight is still near zero.
		let s0 = Sheet::new(size, &pal, thickness, &mut rng.fork(1));
		let mut s1 = Sheet::new(size, &pal, thickness, &mut rng.fork(2));
		s1.paint(size);
		let grid = if detail == Detail::Ultra { GRID_FINE } else { GRID_FINE * 2 };
		let gw = w / grid + 2;
		let gh = h / grid + 2;
		let count = 3 + (turbulence * 5.0) as u32;
		let mut vortices = Vec::new();
		for i in 0..count {
			let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
			vortices.push(Vortex {
				path: [
					rng.range(0.2, 0.8),
					rng.range(0.25, 0.75),
					rng.range(0.15, 0.4),
					rng.range(0.15, 0.35),
					rng.range(0.011, 0.03),
					rng.range(0.013, 0.033),
				],
				strength: sign * rng.range(0.6, 1.2),
				core: rng.range(0.08, 0.2),
			});
		}
		let mut ink = Ink {
			w,
			h,
			layers: [Layer::new(w, h, s0), Layer::new(w, h, s1)],
			rng,
			vortices,
			turbulence,
			grid,
			gw,
			gh,
			vx: vec![0; gw * gh],
			vy: vec![0; gw * gh],
			rowx: vec![0; gw],
			rowy: vec![0; gw],
			rays: vec![256; w],
			seed,
			aurora: 0.0,
			pal,
			smooth: detail >= Detail::Medium,
		};
		for i in 0..2 {
			let mut r = ink.rng.fork(10 + i as u32);
			ink.layers[i].reset(w, h, &mut r);
		}
		ink
	}

	/// The flow at time t on the coarse grid, in render pixels per second.
	fn flow(&mut self, t: f32) {
		let (w, h) = (self.w as f32, self.h as f32);
		let unit = h;
		let cyc = fract(t / CYCLE) * CYCLE;
		// The aurora phase: the vortices calm and the flow turns upwards, between 35 and 47 seconds of each cycle.
		let a = smoothstep(33.0, 38.0, cyc) * (1.0 - smoothstep(44.0, 50.0, cyc));
		self.aurora = a;
		// The shear wakes after the pour and is strongest in the feathering phase.
		let shear = (0.35 + 0.65 * smoothstep(12.0, 25.0, cyc)) * (0.5 + self.turbulence) * unit * 0.05 * (1.0 - a);
		let swirl = (0.3 + 0.7 * smoothstep(3.0, 10.0, cyc)) * (0.4 + self.turbulence) * (1.0 - 0.8 * a);
		let lift = a * unit * 0.12;
		let centres: Vec<(f32, f32, f32, f32)> = self
			.vortices
			.iter()
			.map(|v| {
				let p = v.path;
				let cx = (p[0] + p[2] * sin(t * p[4] * TAU + p[1] * 9.0)) * w;
				let cy = (p[1] + p[3] * sin(t * p[5] * TAU + p[0] * 7.0)) * h;
				let core = v.core * unit;
				(cx, cy, v.strength * swirl * unit * unit * 0.05, core * core)
			})
			.collect();
		let k1 = TAU / (h * 0.9);
		let k2 = TAU / (w * 0.7);
		let ph1 = t * 0.05;
		let ph2 = t * 0.037;
		let drift = (unit * 0.01 * sin(t * 0.013), unit * 0.008 * cos(t * 0.011));
		for gy in 0..self.gh {
			let y = (gy * self.grid) as f32;
			for gx in 0..self.gw {
				let x = (gx * self.grid) as f32;
				let mut u = shear * sin(y * k1 + ph1) + drift.0;
				let mut v = shear * 0.6 * sin(x * k2 + ph2) + drift.1;
				for &(cx, cy, s, core2) in &centres {
					let dx = x - cx;
					let dy = y - cy;
					let r2 = dx * dx + dy * dy + 1.0;
					// Lamb-Oseen vortex: tangential speed s/r·(1 - e^(-r²/core²)).
					let f = s * (1.0 - exp(-r2 / core2)) / r2;
					u -= dy * f;
					v += dx * f;
				}
				if lift > 0.0 {
					// Curtains: an upward flow whose speed varies across the screen, so ink stretches into vertical sheets.
					v -= lift * (0.6 + 0.4 * sin(x * k2 * 1.7 + t * 0.21)) * (0.6 + 0.4 * sin(x * k2 * 3.1 - t * 0.13));
				}
				self.vx[gy * self.gw + gx] = (u * 65_536.0) as i32;
				self.vy[gy * self.gw + gx] = (v * 65_536.0) as i32;
			}
		}
		// The rays' shimmer: fine vertical streaks that drift sideways.
		if a > 0.0 {
			for (x, ray) in self.rays.iter_mut().enumerate() {
				let n = noise2(x as f32 * 40.0 / w, t * 0.3, self.seed) * 0.6
					+ noise2(x as f32 * 11.0 / w, t * 0.1, self.seed + 1) * 0.4;
				*ray = (256.0 + 220.0 * n) as u32;
			}
		}
	}

	/// The aurora: light carried upwards from every bright band, column by column, fading with height, each column at its own
	/// shimmering rate, so lit ink throws tall rays like the curtains of an aurora.
	fn curtains(&self, out: &mut Frame) {
		let (w, h) = (out.w, out.h);
		let a = self.aurora;
		// Rays reach about a third of the height at their longest.
		let base = 1.0 - 3.5 / h as f32;
		// The ink dims a little under the curtains, so they glow against it.
		let dim = (256.0 - 90.0 * a) as u32;
		for x in 0..w {
			let shimmer = self.rays[x] as f32 / 256.0;
			let keep = (256.0 * (base + (1.0 - base) * 0.5 * (shimmer - 1.0))).clamp(0.0, 255.0) as u32;
			let gain = (a * 150.0 * shimmer) as u32;
			let mut acc = 0u32;
			for y in (0..h).rev() {
				let i = y * w + x;
				let c = out.px[i];
				// The rays are fed from the pixel blended with its right-hand neighbour (not yet lit, since columns go left to
				// right), so one bright pixel does not draw a scratch. Only bright, saturated ink feeds them; pale veins and the
				// water do not.
				let soft = mix(c, out.px[i + (x + 1 < w) as usize], 110);
				let (r, g, b) = (colour::r(soft), colour::g(soft), colour::b(soft));
				let chroma = r.max(g).max(b) - r.min(g).min(b);
				let feed = if chroma > 70 && colour::luma(soft) > 60 {
					scale(soft, gain.min(256))
				} else {
					0
				};
				// A decaying maximum, not a sum, so a tall band does not pile up into white.
				let kept = scale(acc, keep);
				acc = if colour::luma(feed) > colour::luma(kept) { feed } else { kept };
				out.px[i] = colour::screen(scale(c, dim), acc);
			}
		}
	}

	/// Moves both layers' maps along the flow by `dt` seconds. The grid velocities are first scaled by `dt` into 16.16 pixel
	/// displacements, so the per-pixel work is 32-bit adds, shifts and small multiplies, which a Pentium does quickly.
	fn advect(&mut self, dt: f32) {
		let (w, h) = (self.w, self.h);
		let gs = self.grid as i32;
		for (vx, vy) in self.vx.iter_mut().zip(self.vy.iter_mut()) {
			*vx = (*vx as f32 * dt) as i32;
			*vy = (*vy as f32 * dt) as i32;
		}
		let maxx = (w as i32 - 1) << 16;
		let maxy = (h as i32 - 1) << 16;
		let [la, lb] = &mut self.layers;
		for y in 0..h {
			let gy = y / gs as usize;
			let ty = ((y as i32 % gs) << 8) / gs;
			for gx in 0..self.gw {
				let (i0, i1) = (gy * self.gw + gx, (gy + 1) * self.gw + gx);
				self.rowx[gx] = self.vx[i0] + ((self.vx[i1] - self.vx[i0]) * ty >> 8);
				self.rowy[gx] = self.vy[i0] + ((self.vy[i1] - self.vy[i0]) * ty >> 8);
			}
			for x in 0..w {
				let gx = x / gs as usize;
				let tx = ((x as i32 % gs) << 8) / gs;
				let dx = self.rowx[gx] + ((self.rowx[gx + 1] - self.rowx[gx]) * tx >> 8);
				let dy = self.rowy[gx] + ((self.rowy[gx + 1] - self.rowy[gx]) * tx >> 8);
				let sx = ((x as i32) << 16) - dx;
				let sy = ((y as i32) << 16) - dy;
				let i = y * w + x;
				if sx < 0 || sy < 0 || sx > maxx || sy > maxy {
					// Ink flowing in from beyond the edge is fresh sheet.
					(la.nu[i], la.nv[i]) = la.fresh_fx(sx, sy);
					(lb.nu[i], lb.nv[i]) = lb.fresh_fx(sx, sy);
					continue;
				}
				let x0 = (sx >> 16) as usize;
				let y0 = (sy >> 16) as usize;
				let x1 = (x0 + 1).min(w - 1);
				let y1 = (y0 + 1).min(h - 1);
				let fx = (sx >> 8) & 0xFF;
				let fy = (sy >> 8) & 0xFF;
				// Map values wrap (they are positions on a tiling sheet), so differences are taken with wrapping arithmetic and
				// shifted before the multiply to stay in 32 bits.
				let lerp = |a: i32, b: i32, t: i32| a.wrapping_add((b.wrapping_sub(a) >> 8).wrapping_mul(t));
				let (i00, i10, i01, i11) = (y0 * w + x0, y0 * w + x1, y1 * w + x0, y1 * w + x1);
				for layer in [&mut *la, &mut *lb] {
					let top = lerp(layer.u[i00], layer.u[i10], fx);
					let bot = lerp(layer.u[i01], layer.u[i11], fx);
					layer.nu[i] = lerp(top, bot, fy);
					let top = lerp(layer.v[i00], layer.v[i10], fx);
					let bot = lerp(layer.v[i01], layer.v[i11], fx);
					layer.nv[i] = lerp(top, bot, fy);
				}
			}
		}
		for layer in [la, lb] {
			core::mem::swap(&mut layer.u, &mut layer.nu);
			core::mem::swap(&mut layer.v, &mut layer.nv);
		}
	}
}

impl Scene for Ink {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		// Each layer lives for two cycles, offset by one, and is reset at the moment its weight is zero.
		let phase = t / CYCLE;
		for which in 0..2 {
			let local = phase + which as f32;
			let epoch = crate::math::floor_i(local * 0.5) as i64;
			if self.layers[which].epoch != epoch {
				let first = self.layers[which].epoch == i64::MIN;
				self.layers[which].epoch = epoch;
				if !first {
					let mut r = self.rng.fork(epoch as u32 ^ (which as u32) << 16);
					let w = self.w;
					let h = self.h;
					self.layers[which].reset(w, h, &mut r);
				}
			}
		}
		let sheet = &mut self.layers[0].sheet;
		if sheet.next_row < sheet.size {
			let rows = (sheet.size / 48).max(1);
			sheet.paint(rows);
		}
		self.flow(t);
		if cx.dt > 0.0 {
			self.advect(cx.dt);
		}
		// Layer weights: a triangle over each layer's two-cycle life, so the two always sum to one.
		let wa = 1.0 - (fract(phase * 0.5) * 2.0 - 1.0).abs();
		let wa256 = ((1.0 - wa) * 256.0) as u32;
		let light_sweep = cx.detail >= Detail::High;
		// The sweep: a soft band moving diagonally across once a cycle, lifting what it touches by up to 30%.
		let sweep_pos = fract(t / CYCLE + 0.3) * 2.6 - 0.8;
		let (w, h) = (self.w, self.h);
		let inv = 1.0 / (w + h) as f32;
		let aurora = self.aurora > 0.01 && cx.detail >= Detail::Medium && !self.pal.light;
		let smooth = self.smooth;
		let [la, lb] = &self.layers;
		for y in 0..h {
			let row = &mut out.px[y * w..(y + 1) * w];
			for (x, px) in row.iter_mut().enumerate() {
				let i = y * w + x;
				let ca = la.sheet.at(la.u[i], la.v[i], smooth);
				let cb = lb.sheet.at(lb.u[i], lb.v[i], smooth);
				let mut c = mix(ca, cb, wa256);
				if light_sweep {
					let d = (x + y) as f32 * inv * 1.6 - sweep_pos;
					if d.abs() < 0.35 {
						let k = 1.0 - d.abs() / 0.35;
						c = colour::screen(c, scale(c, (k * k * 90.0) as u32));
					}
				}
				*px = c;
			}
		}
		if aurora {
			self.curtains(out);
		}
		if cx.detail >= Detail::High {
			relief(out, self.pal.light);
		}
	}

	fn hints(&self, _cx: &Ctx) -> Hints {
		Hints {
			threshold: if self.pal.light { 235 } else { 120 },
			streak_tint: mixf(self.pal.sun, 0xFFFFFF, 0.3),
			fade: 1.0,
			bloom_gain: if self.pal.light { 0.3 } else { 1.3 + 0.4 * self.aurora },
		}
	}
}

/// Wet-ink relief: each pixel is lit by how much brighter it is than its upper-left neighbour, so every band of ink reads as a thin
/// raised film catching a light from the top left. Runs from the bottom right so each pixel still sees its neighbour's old value.
fn relief(out: &mut Frame, light: bool) {
	let w = out.w;
	for y in (1..out.h).rev() {
		for x in (1..w).rev() {
			let c = out.px[y * w + x];
			let n = out.px[(y - 1) * w + x - 1];
			let d = colour::luma(c) as i32 - colour::luma(n) as i32;
			out.px[y * w + x] = if d > 2 {
				let k = (d * if light { 3 } else { 5 }).min(200) as u32;
				colour::screen(c, scale(0xFFF4E8, k))
			} else if d < -2 {
				scale(c, (256 + d * 2).max(150) as u32)
			} else {
				c
			};
		}
	}
}
