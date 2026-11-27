// Afterglow Twilight: a tiny painted planet turning in a luminous atmosphere, with moons, satellites and a slow camera.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The planet is two equirectangular maps made once from the seed: the ground (domain-warped noise cut into stepped terraces and
//! coloured as soft fields of the palette, with a brush-stroke grain and night lights) and the clouds (banded noise). Each frame, every
//! pixel of the disc finds its normal, turns it into the planet's frame (the camera's view of the axis), and reads latitude and
//! longitude; the spin is an offset in longitude. Light wraps softly round the terminator, which is tinted through the glow hues, and
//! the atmosphere glows inside the limb (a fresnel term) and outside it (a falloff), brightest on the day side.

use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{self, Ramp, mix, mixf, scale, screen};
use crate::frame::Frame;
use crate::math::{PI, TAU, V3, atan2, clamp, cos, ease, floor_i, fract, lerp, powf, round_i, saturate, sin, smoothstep, sqrt};
use crate::noise::{fbm2, fbm3, noise3};
use crate::palette::Palette;
use crate::rng::{Rng, hash2};
use crate::saver::palette_for;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Detail, Key, Settings};

/// One camera shot: where the planet sits, how big, and how the axis and light are turned, at the start and end of the shot.
#[derive(Clone, Copy)]
struct Shot {
	len: f32,
	/// Planet centre in screen fractions (x of the width, y of the height), and radius in heights.
	from: [f32; 6],
	to: [f32; 6],
}

// [cx, cy, radius, axis tilt towards the viewer (radians), roll (radians), light angle (radians about the vertical: 0 lights the
// right half, π/2 lights the face, π the left half, more than π the back, for a crescent)]
const SHOTS: [Shot; 4] = [
	// The establishing shot: small, left third, turning.
	Shot {
		len: 25.0,
		from: [0.36, 0.52, 0.22, 0.35, -0.12, 0.35],
		to: [0.40, 0.50, 0.25, 0.30, -0.08, 0.15],
	},
	// The push-in towards the terminator.
	Shot {
		len: 25.0,
		from: [0.55, 0.55, 0.30, 0.25, 0.10, 0.1],
		to: [0.62, 0.78, 0.75, 0.15, 0.18, -0.15],
	},
	// From slightly above, a satellite passes close.
	Shot {
		len: 20.0,
		from: [0.50, 0.58, 0.34, 0.75, 0.0, 0.45],
		to: [0.47, 0.56, 0.37, 0.70, -0.05, 0.3],
	},
	// The drift along the night side.
	Shot {
		len: 20.0,
		from: [0.70, 0.42, 0.45, 0.2, -0.25, 2.75],
		to: [0.30, 0.47, 0.48, 0.25, -0.15, 2.95],
	},
];

const FADE: f32 = 1.5;

struct Moon {
	orbit: f32,
	incline: f32,
	speed: f32,
	phase: f32,
	radius: f32,
	tint: u32,
}

struct Satellite {
	orbit: f32,
	incline: f32,
	node: f32,
	speed: f32,
	phase: f32,
}

struct Star {
	x: f32,
	y: f32,
	bright: f32,
	twinkle: f32,
	tint: u32,
}

pub struct Twilight {
	w: usize,
	h: usize,
	pal: Palette,
	/// Ground map, `mw` × `mh`, and its night lights (0 to 255).
	ground: Vec<u32>,
	lights: Vec<u8>,
	clouds: Vec<u8>,
	mw: usize,
	mh: usize,
	/// Arcsine table: index by y in [-1, 1] over 1024 steps, value is latitude as a map row in 16.16.
	asin_v: Vec<i32>,
	nebula: Frame,
	stars: Vec<Star>,
	moons: Vec<Moon>,
	sats: Vec<Satellite>,
	atmos: f32,
	atmos_ramp: Ramp,
	/// The light's colour by how directly it falls (index: n·l from -0.3 to 1).
	light_ramp: Ramp,
	total: f32,
	spin_rate: f32,
	seed: u32,
}

impl Twilight {
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> Twilight {
		let detail = settings.detail();
		let pal = palette_for(settings);
		let mut rng = Rng::new(seed as u64 ^ 0x7E11_6A00_0000);
		// The maps grow with the render width at High and Ultra, so a planet filling a large screen keeps its brush strokes.
		let mw = match detail {
			Detail::Low => 256,
			Detail::Medium => 512,
			Detail::High => (w / 2).next_power_of_two().clamp(512, 1024),
			Detail::Ultra => (w * 3 / 4).next_power_of_two().clamp(512, 2048),
		};
		let mh = mw / 2;
		let (ground, lights) = paint_ground(&pal, mw, mh, &mut rng.fork(1));
		let clouds = paint_clouds(mw, mh, &mut rng.fork(2));
		let mut asin_v = vec![0i32; 1025];
		for (i, slot) in asin_v.iter_mut().enumerate() {
			let y = i as f32 / 512.0 - 1.0;
			let lat = crate::math::asin(y);
			*slot = ((lat / PI + 0.5) * (mh as f32 - 1.0) * 65_536.0) as i32;
		}
		let nebula = paint_nebula(&pal, w, h, &mut rng.fork(3));
		let mut stars = Vec::new();
		let mut srng = rng.fork(4);
		let count = (w * h / 900).clamp(60, 700);
		for _ in 0..count {
			let b = srng.f();
			stars.push(Star {
				x: srng.f(),
				y: srng.f(),
				bright: b * b * b,
				twinkle: srng.range(0.5, 3.0),
				tint: if srng.f() < 0.3 {
					pal.hue(srng.below(4) as usize)
				} else {
					pal.n(100)
				},
			});
		}
		let moon_count = settings.get(Key::Moons);
		let mut moons = Vec::new();
		for i in 0..moon_count {
			moons.push(Moon {
				orbit: rng.range(1.7, 2.6) + i as f32 * 0.7,
				incline: rng.range(-0.35, 0.35),
				speed: rng.range(0.025, 0.05) * if i == 1 { -1.0 } else { 1.0 },
				phase: rng.f() * TAU,
				radius: rng.range(0.11, 0.2) - i as f32 * 0.03,
				tint: mixf(pal.n(300), pal.hue(i as usize + 1), 0.25),
			});
		}
		let sats = (0..3)
			.map(|i| Satellite {
				orbit: rng.range(1.15, 1.5),
				incline: rng.range(-0.9, 0.9),
				node: rng.f() * TAU,
				speed: rng.range(0.12, 0.2) * if i % 2 == 0 { 1.0 } else { -1.0 },
				phase: rng.f() * TAU,
			})
			.collect();
		let atmos_hue = pal.hue(pal.glow_n - 1);
		let atmos_ramp = Ramp::new(&[
			(0.0, 0x000000),
			(0.25, mixf(0, atmos_hue, 0.6)),
			(0.55, mixf(atmos_hue, pal.hue(1), 0.5)),
			(0.8, pal.hue(0)),
			(1.0, mixf(pal.sun, 0xFFFFFF, 0.5)),
		]);
		let cool = pal.hue(pal.glow_n - 1);
		let at = |ndl: f32| (ndl + 0.3) / 1.3;
		let light_ramp = Ramp::new(&[
			(at(-0.3), mixf(0x000000, cool, 0.14)),
			(at(-0.14), mixf(0x000000, cool, 0.32)),
			(at(-0.04), mixf(cool, pal.hue(1), 0.45)),
			(at(0.04), pal.hue(1)),
			(at(0.13), mixf(pal.hue(0), pal.hue(1), 0.3)),
			(at(0.24), mixf(pal.hue(0), pal.sun, 0.5)),
			(at(0.42), mixf(pal.sun, 0xFFFFFF, 0.6)),
			(1.0, 0xFFF6EE),
		]);
		Twilight {
			w,
			h,
			light_ramp,
			pal,
			ground,
			lights,
			clouds,
			mw,
			mh,
			asin_v,
			nebula,
			stars,
			moons,
			sats,
			atmos: settings.get(Key::Atmosphere) as f32 / 100.0,
			atmos_ramp,
			total: SHOTS.iter().map(|s| s.len).sum(),
			spin_rate: rng.range(0.012, 0.02),
			seed,
		}
	}

	/// The camera at time t: the shot's values eased between its ends, and the fade at the cuts.
	fn camera(&self, t: f32) -> ([f32; 6], f32, usize) {
		let loop_t = fract(t / self.total) * self.total;
		let mut start = 0.0;
		for (i, shot) in SHOTS.iter().enumerate() {
			if loop_t < start + shot.len || i == SHOTS.len() - 1 {
				let k = saturate((loop_t - start) / shot.len);
				let e = ease(k) * 0.7 + k * 0.3;
				let mut v = [0.0; 6];
				for (j, slot) in v.iter_mut().enumerate() {
					*slot = lerp(shot.from[j], shot.to[j], e);
				}
				let into = loop_t - start;
				let left = shot.len - into;
				let fade = saturate(into / FADE).min(saturate(left / FADE));
				return (v, fade, i);
			}
			start += shot.len;
		}
		(SHOTS[0].from, 1.0, 0)
	}
}

/// The planet's ground: elevation from domain-warped 3D noise on the sphere, cut into stepped terraces with a lit rim and a dark
/// foot on every step, like contour lines in a painting; dry and wet land from a second field; coastal shelves; brush strokes
/// along the latitude lines; ice at the poles; and lights near the coasts.
fn paint_ground(pal: &Palette, mw: usize, mh: usize, rng: &mut Rng) -> (Vec<u32>, Vec<u8>) {
	let seed = rng.next_u32();
	let sea_level = rng.range(-0.02, 0.1);
	let cool = pal.hue(pal.glow_n - 1);
	let deep = mixf(pal.n(700), cool, 0.32);
	let shelf = mixf(pal.n(500), cool, 0.5);
	let beach = mixf(pal.n(300), pal.sun, 0.25);
	let dry = Ramp::new(&[
		(0.0, mixf(pal.n(500), pal.hue(0), 0.5)),
		(0.5, mixf(pal.n(400), pal.hue(1), 0.4)),
		(1.0, mixf(pal.n(200), pal.hue(0), 0.2)),
	]);
	let wet = Ramp::new(&[
		(0.0, mixf(pal.n(600), cool, 0.35)),
		(0.5, mixf(pal.n(500), pal.hue(1), 0.3)),
		(1.0, mixf(pal.n(300), cool, 0.2)),
	]);
	let ice = mixf(pal.n(300), cool, 0.12);
	let mut ground = vec![0u32; mw * mh];
	let mut lights = vec![0u8; mw * mh];
	for v in 0..mh {
		let lat = (v as f32 + 0.5) / mh as f32 * PI - PI * 0.5;
		let (cl, sl) = (cos(lat), sin(lat));
		for u in 0..mw {
			let lon = (u as f32 + 0.5) / mw as f32 * TAU;
			let p = V3::new(cl * sin(lon), sl, cl * cos(lon));
			let warp = fbm3(p.x * 1.5, p.y * 1.5, p.z * 1.5, 3, 0.5, seed ^ 0x55);
			let q = p * 2.0 + V3::new(warp, -warp, warp * 0.6) * 0.9;
			let e = fbm3(q.x, q.y, q.z, 6, 0.5, seed);
			let moist = fbm3(p.x * 2.5 + 3.0, p.y * 2.5, p.z * 2.5, 3, 0.5, seed ^ 0x99);
			let i = v * mw + u;
			// Brush strokes: noise stretched along the latitude lines.
			let stroke = fbm2(lon * 24.0, lat * 120.0, 2, 0.5, seed ^ 0x77) * 0.13;
			let polar = smoothstep(0.84, 0.93, sl.abs() + e * 0.12);
			let c = if e < sea_level {
				let d = saturate((sea_level - e) * 6.0);
				mixf(shelf, deep, smoothstep(0.0, 0.5, d))
			} else {
				let h = saturate((e - sea_level) * 2.4);
				let steps = h * 6.0;
				let base = floor_i(steps) as f32;
				let within = steps - base;
				let terrace = base / 6.0;
				let land = mixf(dry.at(terrace), wet.at(terrace), smoothstep(-0.15, 0.25, moist));
				// The rim of each step catches light; its foot is in shadow.
				let lit_rim = smoothstep(0.8, 0.97, within) * 0.18;
				let foot = (1.0 - smoothstep(0.0, 0.12, within)) * 0.16 * if base > 0.0 { 1.0 } else { 0.0 };
				let mut c = colour::shade(land, lit_rim - foot);
				let coast = 1.0 - smoothstep(0.0, 0.025, e - sea_level);
				c = mixf(c, beach, coast * 0.5);
				if coast > 0.0 && rng_hash(u, v, seed) < (coast * 90.0) as u32 {
					lights[i] = 120 + (rng_hash(u + 7, v, seed) & 127) as u8;
				}
				if h < 0.3 && rng_hash(u, v + 3, seed) < 3 {
					lights[i] = 200;
				}
				c
			};
			ground[i] = mixf(colour::shade(c, stroke), ice, polar);
		}
	}
	(ground, lights)
}

#[inline]
fn rng_hash(u: usize, v: usize, seed: u32) -> u32 {
	hash2(u as i32, v as i32, seed) & 0xFFFF
}

/// Cloud cover, 0 to 255: bands along the latitude lines, broken by noise and swirled.
fn paint_clouds(mw: usize, mh: usize, rng: &mut Rng) -> Vec<u8> {
	let seed = rng.next_u32();
	let mut out = vec![0u8; mw * mh];
	for v in 0..mh {
		let lat = (v as f32 + 0.5) / mh as f32 * PI - PI * 0.5;
		let (cl, sl) = (cos(lat), sin(lat));
		for u in 0..mw {
			let lon = (u as f32 + 0.5) / mw as f32 * TAU;
			let p = V3::new(cl * sin(lon), sl, cl * cos(lon));
			let swirl = noise3(p.x * 2.0, p.y * 2.0, p.z * 2.0, seed ^ 0x11) * 0.8;
			let band = sin(lat * 7.0 + swirl * 2.0) * 0.35;
			let n = fbm3(p.x * 3.0 + swirl, p.y * 9.0, p.z * 3.0 - swirl, 4, 0.55, seed) + band;
			out[v * mw + u] = (smoothstep(0.12, 0.5, n) * 215.0) as u8;
		}
	}
	out
}

/// A painterly nebula behind the planet: domain-warped noise in the palette's glow hues, dim, at the render size plus a margin for
/// the camera's drift.
fn paint_nebula(pal: &Palette, w: usize, h: usize, rng: &mut Rng) -> Frame {
	let seed = rng.next_u32();
	let nw = (w * 3 / 4).max(8);
	let nh = (h * 3 / 4).max(8);
	let mut f = Frame::new(nw, nh);
	let a = mixf(pal.ground(), pal.hue(pal.glow_n - 1), 0.42);
	let b = mixf(pal.ground(), pal.hue(1), 0.32);
	let c = mixf(pal.ground(), pal.hue(0), 0.24);
	for y in 0..nh {
		for x in 0..nw {
			let fx = x as f32 / nh as f32 * 2.2;
			let fy = y as f32 / nh as f32 * 2.2;
			let wx = fbm2(fx, fy, 3, 0.5, seed ^ 3);
			let wy = fbm2(fx + 5.2, fy + 1.3, 3, 0.5, seed ^ 4);
			let n = fbm2(fx + wx * 1.5, fy + wy * 1.5, 5, 0.55, seed);
			let m = fbm2(fx * 0.7 + wy, fy * 0.7 - wx, 3, 0.5, seed ^ 9);
			let cloud = smoothstep(-0.1, 0.6, n);
			let mut col = mixf(pal.ground(), a, cloud);
			col = mixf(col, b, smoothstep(0.0, 0.7, m) * cloud);
			col = mixf(col, c, smoothstep(0.35, 0.8, n * m * 2.0));
			// Dark dust lanes.
			let dust = smoothstep(0.1, 0.4, fbm2(fx * 2.0 - wx, fy * 2.0, 3, 0.5, seed ^ 7));
			f.set(x, y, mixf(col, pal.ground(), dust * 0.7));
		}
	}
	f
}

impl Scene for Twilight {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		let (cam, _fade, shot) = self.camera(t);
		let (w, h) = (self.w as f32, self.h as f32);
		let detail = cx.detail;
		// Background: the nebula, panned against the planet's drift for parallax, then the stars.
		let pan_x = (cam[0] - 0.5) * -0.2 + sin(t * 0.01) * 0.03;
		let pan_y = (cam[1] - 0.5) * -0.2;
		let nb = &self.nebula;
		let sx_step = ((nb.w as f32 * 0.8 / w) * 65_536.0) as i32;
		let sy_step = ((nb.h as f32 * 0.8 / h) * 65_536.0) as i32;
		let ox = (((0.1 + pan_x * 0.5) * nb.w as f32) * 65_536.0) as i32;
		let oy = (((0.1 + pan_y * 0.5) * nb.h as f32) * 65_536.0) as i32;
		for y in 0..self.h {
			let sy = oy + y as i32 * sy_step;
			let row = &mut out.px[y * self.w..(y + 1) * self.w];
			let mut sx = ox;
			for px in row.iter_mut() {
				*px = nb.sample(sx, sy);
				sx += sx_step;
			}
		}
		for s in &self.stars {
			let x = fract(s.x + pan_x * 0.7) * w;
			let y = fract(s.y + pan_y * 0.7) * h;
			let tw = 0.65 + 0.35 * sin(t * s.twinkle * 2.0 + s.x * 40.0);
			let k = (s.bright * tw * 256.0) as u32;
			let (xi, yi) = (x as i32, y as i32);
			if xi >= 0 && yi >= 0 && (xi as usize) < self.w && (yi as usize) < self.h {
				let i = yi as usize * self.w + xi as usize;
				out.px[i] = screen(out.px[i], scale(s.tint, k));
				if s.bright > 0.5 {
					for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
						out.put(xi + dx, yi + dy, screen(out.px[i], scale(s.tint, k / 3)));
					}
				}
			}
		}
		// The planet's frame: the light direction in view space, and the axis turned towards the viewer and rolled.
		let pcx = cam[0] * w;
		let pcy = cam[1] * h;
		let r = cam[2] * h;
		let tilt = cam[3];
		let roll = cam[4];
		let la = cam[5] + t * 0.004;
		let light = V3::new(cos(la), -0.3, sin(la)).norm();
		let spin = t * self.spin_rate;
		let cloud_spin = t * self.spin_rate * 1.35;
		// Moons and satellites, placed first, so those behind the planet are drawn before it.
		let mut bodies: Vec<(f32, f32, f32, f32, u32, bool)> = Vec::new();
		for (i, m) in self.moons.iter().enumerate() {
			let a = m.phase + t * m.speed;
			let p = V3::new(
				cos(a) * m.orbit,
				sin(a) * m.orbit * sin(m.incline),
				sin(a) * m.orbit * cos(m.incline),
			);
			let p = rotate_view(p, tilt * 0.3, roll);
			bodies.push((pcx + p.x * r, pcy + p.y * r, p.z, m.radius * r, i as u32, false));
		}
		for (i, s) in self.sats.iter().enumerate() {
			let a = s.phase + t * s.speed * if shot == 2 { 1.6 } else { 1.0 };
			let local = V3::new(cos(a) * s.orbit, 0.0, sin(a) * s.orbit);
			let p = V3::new(local.x, local.z * sin(s.incline), local.z * cos(s.incline));
			let p = rotate_view(p, s.node, roll);
			bodies.push((pcx + p.x * r, pcy + p.y * r, p.z, 0.0, i as u32, true));
		}
		for b in bodies.iter().filter(|b| b.2 < 0.0) {
			self.draw_body(out, b, light, t, detail);
		}
		self.draw_planet(out, pcx, pcy, r, tilt, roll, light, spin, cloud_spin, detail);
		for b in bodies.iter().filter(|b| b.2 >= 0.0) {
			self.draw_body(out, b, light, t, detail);
		}
		let _ = self.seed;
	}

	fn hints(&self, cx: &Ctx) -> Hints {
		let (_, fade, _) = self.camera(cx.t as f32);
		Hints {
			threshold: 135,
			streak_tint: mixf(self.pal.hue(0), 0xFFFFFF, 0.35),
			fade,
			bloom_gain: 1.2,
		}
	}
}

/// Turns a planet-centred point into view space: a tilt about x, then a roll about z.
fn rotate_view(p: V3, tilt: f32, roll: f32) -> V3 {
	let (ct, st) = (cos(tilt), sin(tilt));
	let q = V3::new(p.x, p.y * ct - p.z * st, p.y * st + p.z * ct);
	let (cr, sr) = (cos(roll), sin(roll));
	V3::new(q.x * cr - q.y * sr, q.x * sr + q.y * cr, q.z)
}

impl Twilight {
	#[allow(clippy::too_many_arguments)]
	fn draw_planet(
		&self,
		out: &mut Frame,
		pcx: f32,
		pcy: f32,
		r: f32,
		tilt: f32,
		roll: f32,
		light: V3,
		spin: f32,
		cloud_spin: f32,
		detail: Detail,
	) {
		let (w, h) = (self.w as i32, self.h as i32);
		let halo = 1.0 + 0.22 * self.atmos;
		let reach = r * halo;
		let y0 = ((pcy - reach) as i32).max(0);
		let y1 = ((pcy + reach) as i32 + 1).min(h);
		let x0 = ((pcx - reach) as i32).max(0);
		let x1 = ((pcx + reach) as i32 + 1).min(w);
		let inv_r = 1.0 / r;
		// View to planet: undo the roll, then the tilt. Rows of the matrix, applied to (nx, ny, nz).
		let (cr, sr) = (cos(roll), sin(roll));
		let (ct, st) = (cos(tilt), sin(tilt));
		let m = [[cr, sr, 0.0], [-sr * ct, cr * ct, st], [sr * st, -cr * st, ct]];
		let (mw, mh) = (self.mw as i32, self.mh as i32);
		let spin_u = fract(spin) * mw as f32;
		let cloud_u = fract(cloud_spin) * mw as f32;
		let smooth = detail >= Detail::Medium;
		let shadows = detail >= Detail::Medium;
		let lights_on = detail >= Detail::High;
		let sheen = detail >= Detail::Ultra;
		let atmos = self.atmos;
		let city = mixf(self.pal.sun, 0xFFFFFF, 0.2);
		let cloud_lit = mixf(self.pal.n(200), self.pal.sun, 0.1);
		let scale_u = mw as f32 / TAU;
		for py in y0..y1 {
			let ny = (py as f32 + 0.5 - pcy) * inv_r;
			let row = &mut out.px[py as usize * self.w..(py as usize + 1) * self.w];
			for px in x0..x1 {
				let nx = (px as f32 + 0.5 - pcx) * inv_r;
				let d2 = nx * nx + ny * ny;
				let pix = &mut row[px as usize];
				if d2 >= 1.0 {
					// Outside the limb: the atmosphere's halo, brightest towards the light.
					let d = sqrt(d2);
					if d < halo {
						let falloff = 1.0 - (d - 1.0) / (halo - 1.0);
						let side = saturate((nx * light.x + ny * light.y) / d * 0.6 + 0.55);
						let k = falloff * falloff * falloff * falloff * side * atmos * 1.3;
						if k > 0.004 {
							*pix = screen(*pix, self.atmos_ramp.at(k * 0.9));
						}
					}
					continue;
				}
				let nz = sqrt(1.0 - d2);
				// Into the planet's frame: y is the axis, so latitude is asin(p.y) and longitude atan2(p.x, p.z).
				let p0 = m[0][0] * nx + m[0][1] * ny + m[0][2] * nz;
				let p1 = m[1][0] * nx + m[1][1] * ny + m[1][2] * nz;
				let p2 = m[2][0] * nx + m[2][1] * ny + m[2][2] * nz;
				let lat_i = clamp(-p1 * 512.0 + 512.0, 0.0, 1024.0) as usize;
				let v16 = self.asin_v[lat_i];
				let lon = atan2(p0, p2);
				let u = lon * scale_u + spin_u;
				let u16 = round_i(u * 65_536.0 / 256.0) << 8;
				let ground = sample_map(&self.ground, mw, mh, u16, v16, smooth);
				// Light: a soft wrap round the terminator, which is tinted from the outer glow hue to the inner one.
				let ndl = nx * light.x + ny * light.y + nz * light.z;
				// Light is a colour: warm white by day, orange then rose at the terminator, purple dusk, indigo night.
				let lc = self.light_ramp.c[(saturate((ndl + 0.3) * (1.0 / 1.3)) * 255.0) as usize];
				let mut c = tint(ground, lc);
				if ndl < 0.0 && lights_on {
					let l = sample_lights(&self.lights, mw, mh, u16, v16);
					if l > 0 {
						c = screen(c, scale(city, l as u32 * (saturate(-ndl * 3.0) * 180.0) as u32 >> 8));
					}
				}
				// Clouds, lit like the ground, with their shadow a little towards the dark side.
				let cu = round_i((lon * scale_u + cloud_u) * 256.0) << 8;
				let cloud = sample_cover(&self.clouds, mw, mh, cu, v16);
				if shadows {
					let su = cu + round_i(light.x * -6.0 * 65_536.0);
					let sh = sample_cover(&self.clouds, mw, mh, su, v16 + round_i(light.y * 4.0 * 65_536.0));
					c = scale(c, 256 - (sh as u32 * 90 >> 8));
				}
				if cloud > 0 {
					c = mix(c, tint(cloud_lit, lc), cloud as u32);
				}
				// The sea's sheen: a soft highlight where the light reflects towards the viewer.
				if sheen && ground & 0xFF > (ground >> 16) & 0xFF {
					let hz = light.z + 1.0;
					let hv = (nx * light.x + ny * light.y + nz * hz) / sqrt(light.x * light.x + light.y * light.y + hz * hz);
					if hv > 0.9 {
						c = screen(c, scale(self.pal.sun, (powf(saturate((hv - 0.9) * 10.0), 3.0) * 70.0) as u32));
					}
				}
				// The atmosphere inside the limb.
				let rim = 1.0 - nz;
				let k = rim * rim * rim * atmos * saturate(ndl * 1.5 + 0.45) * 1.6;
				if k > 0.003 {
					c = screen(c, self.atmos_ramp.at(saturate(k)));
				}
				// Anti-alias the limb over the last pixel.
				let edge = (1.0 - sqrt(d2)) * r;
				*pix = if edge < 1.0 { mixf(*pix, c, edge) } else { c };
			}
		}
	}

	fn draw_body(&self, out: &mut Frame, b: &(f32, f32, f32, f32, u32, bool), light: V3, t: f32, detail: Detail) {
		let &(bx, by, _z, br, idx, sat) = b;
		if sat {
			// A satellite: a glint and a blinking light.
			let blink = fract(t * 0.8 + idx as f32 * 0.37) < 0.12;
			let c = if blink {
				mixf(self.pal.sun, 0xFFFFFF, 0.6)
			} else {
				self.pal.n(300)
			};
			let (xi, yi) = (bx as i32, by as i32);
			out.put(xi, yi, c);
			if detail >= Detail::Medium {
				out.put(xi + 1, yi, scale(c, 120));
				if blink {
					for (dx, dy) in [(-1, 0), (0, 1), (0, -1), (2, 0), (-2, 0)] {
						out.put(xi + dx, yi + dy, scale(c, 160));
					}
				}
			}
			return;
		}
		let m = &self.moons[idx as usize];
		let y0 = ((by - br) as i32).max(0);
		let y1 = ((by + br) as i32 + 1).min(self.h as i32);
		let x0 = ((bx - br) as i32).max(0);
		let x1 = ((bx + br) as i32 + 1).min(self.w as i32);
		if br < 0.5 {
			return;
		}
		let inv = 1.0 / br;
		for py in y0..y1 {
			let ny = (py as f32 + 0.5 - by) * inv;
			for px in x0..x1 {
				let nx = (px as f32 + 0.5 - bx) * inv;
				let d2 = nx * nx + ny * ny;
				if d2 >= 1.0 {
					continue;
				}
				let nz = sqrt(1.0 - d2);
				let ndl = nx * light.x + ny * light.y + nz * light.z;
				let lit = saturate((ndl + 0.08) / 1.08);
				// Craters: soft noise on the moon's face.
				let crater = noise3(nx * 3.0 + idx as f32 * 9.0, ny * 3.0, nz * 3.0, self.seed ^ 0xC0) * 0.25;
				let c = colour::shade(m.tint, crater);
				let c = scale(c, (lit * 235.0 + 8.0) as u32);
				let edge = (1.0 - sqrt(d2)) * br;
				let i = py as usize * self.w + px as usize;
				out.px[i] = if edge < 1.0 { mixf(out.px[i], c, edge) } else { c };
			}
		}
	}
}

/// A surface colour lit by a light colour: the product of the channels, a little over unity so a white light brightens.
#[inline(always)]
fn tint(c: u32, light: u32) -> u32 {
	let ch = |a: u32, b: u32| ((a * b * 5) >> 10).min(255);
	(ch(colour::r(c), colour::r(light)) << 16) | (ch(colour::g(c), colour::g(light)) << 8) | ch(colour::b(c), colour::b(light))
}

/// A ground colour at 16.16 map coordinates, longitude wrapped, latitude clamped.
#[inline(always)]
fn sample_map(map: &[u32], mw: i32, mh: i32, u: i32, v: i32, smooth: bool) -> u32 {
	let x0 = (u >> 16) & (mw - 1);
	let y0 = (v >> 16).clamp(0, mh - 1);
	if !smooth {
		return map[(y0 * mw + x0) as usize];
	}
	let x1 = (x0 + 1) & (mw - 1);
	let y1 = (y0 + 1).min(mh - 1);
	let fx = ((u >> 8) & 0xFF) as u32;
	let fy = ((v >> 8) & 0xFF) as u32;
	let top = mix(map[(y0 * mw + x0) as usize], map[(y0 * mw + x1) as usize], fx);
	let bot = mix(map[(y1 * mw + x0) as usize], map[(y1 * mw + x1) as usize], fx);
	mix(top, bot, fy)
}

#[inline(always)]
fn sample_cover(map: &[u8], mw: i32, mh: i32, u: i32, v: i32) -> u8 {
	let x0 = (u >> 16) & (mw - 1);
	let y0 = (v >> 16).clamp(0, mh - 1);
	let x1 = (x0 + 1) & (mw - 1);
	let fx = (u >> 8) & 0xFF;
	let a = map[(y0 * mw + x0) as usize] as i32;
	let b = map[(y0 * mw + x1) as usize] as i32;
	(a + ((b - a) * fx >> 8)) as u8
}

#[inline(always)]
fn sample_lights(map: &[u8], mw: i32, mh: i32, u: i32, v: i32) -> u8 {
	let x0 = (u >> 16) & (mw - 1);
	let y0 = (v >> 16).clamp(0, mh - 1);
	map[(y0 * mw + x0) as usize]
}
