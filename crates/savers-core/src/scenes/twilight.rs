// Afterglow Twilight: a tiny painted planet turning in a luminous atmosphere, with moons, satellites and a slow camera.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The planet is two equirectangular maps made once from the seed: the ground (domain-warped noise cut into stepped terraces and
//! coloured as soft fields of the palette, with a brush-stroke grain and night lights) and the clouds (banded noise). Each frame, every
//! pixel of the disc finds its normal, turns it into the planet's frame (the camera's view of the axis), and reads latitude and
//! longitude; the spin is an offset in longitude. Light wraps softly round the terminator, which is tinted through the glow hues, and
//! the atmosphere glows inside the limb (a fresnel term) and outside it (a falloff), brightest on the day side.
//!
//! The camera, the parameters, the maps and the ramps live in [`crate::twilight_shared`], which the graphics-card edition shares.

use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{self, mix, mixf, scale, screen};
use crate::frame::Frame;
use crate::math::{PI, TAU, V3, atan2, clamp, cos, fract, powf, round_i, saturate, sin, sqrt};
use crate::noise::noise3;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Detail, Settings};
use crate::twilight_shared::{Shared, camera};

pub struct Twilight {
	w: usize,
	h: usize,
	shared: Shared,
	/// Arcsine table: index by y in [-1, 1] over 1024 steps, value is latitude as a map row in 16.16.
	asin_v: Vec<i32>,
	seed: u32,
}

impl Twilight {
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> Twilight {
		let shared = Shared::new(settings, seed, w, h);
		let mh = shared.map_height;
		let mut asin_v = vec![0i32; 1025];
		for (i, slot) in asin_v.iter_mut().enumerate() {
			let y = i as f32 / 512.0 - 1.0;
			let lat = crate::math::asin(y);
			*slot = ((lat / PI + 0.5) * (mh as f32 - 1.0) * 65_536.0) as i32;
		}
		Twilight {
			w,
			h,
			shared,
			asin_v,
			seed,
		}
	}
}

impl Scene for Twilight {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		let cam = camera(t);
		let shot = cam.shot;
		let (w, h) = (self.w as f32, self.h as f32);
		let detail = cx.detail;
		// Background: the nebula, panned against the planet's drift for parallax, then the stars.
		let pan_x = (cam.centre_x - 0.5) * -0.2 + sin(t * 0.01) * 0.03;
		let pan_y = (cam.centre_y - 0.5) * -0.2;
		let nb = &self.shared.nebula;
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
		for s in &self.shared.stars {
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
		let pcx = cam.centre_x * w;
		let pcy = cam.centre_y * h;
		let r = cam.radius * h;
		let tilt = cam.tilt;
		let roll = cam.roll;
		let la = cam.light_angle + t * 0.004;
		let light = V3::new(cos(la), -0.3, sin(la)).norm();
		let spin = t * self.shared.spin_rate;
		let cloud_spin = t * self.shared.spin_rate * 1.35;
		// Moons and satellites, placed first, so those behind the planet are drawn before it.
		let mut bodies: Vec<(f32, f32, f32, f32, u32, bool)> = Vec::new();
		for (i, m) in self.shared.moons.iter().enumerate() {
			let a = m.phase + t * m.speed;
			let p = V3::new(
				cos(a) * m.orbit,
				sin(a) * m.orbit * sin(m.incline),
				sin(a) * m.orbit * cos(m.incline),
			);
			let p = rotate_view(p, tilt * 0.3, roll);
			bodies.push((pcx + p.x * r, pcy + p.y * r, p.z, m.radius * r, i as u32, false));
		}
		for (i, s) in self.shared.satellites.iter().enumerate() {
			let a = s.phase + t * s.speed * if shot == 2 { 1.6 } else { 1.0 };
			let local = V3::new(cos(a) * s.orbit, 0.0, sin(a) * s.orbit);
			let p = V3::new(local.x, local.z * sin(s.incline), local.z * cos(s.incline));
			let p = rotate_view(p, s.node, roll);
			bodies.push((pcx + p.x * r, pcy + p.y * r, p.z, 0.0, i as u32, true));
		}
		// The sun, behind everything but the stars, so the planet hides it as it passes: a small white-hot disc in a soft glow.
		if self.shared.sun && cam.sun_distance > 0.0 {
			let d = sqrt(light.x * light.x + light.y * light.y).max(1e-3);
			let sx = pcx + light.x / d * r * cam.sun_distance;
			let sy = pcy + light.y / d * r * cam.sun_distance;
			self.draw_sun(out, sx, sy);
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

	fn cues(&mut self, t: f64) -> crate::audio::Cues {
		let cam = crate::twilight_shared::camera(t as f32);
		let total = crate::twilight_shared::total_length();
		let loop_t = crate::math::fract(t as f32 / total) * total;
		let start: f32 = crate::twilight_shared::SHOTS[..cam.shot].iter().map(|s| s.len).sum();
		// The sun clears the limb in the night-side drift, where it starts behind the planet.
		let sun = if cam.shot == 3 {
			saturate((cam.sun_distance - 0.95) / 0.15)
		} else {
			0.0
		};
		crate::audio::Cues {
			t,
			fade: cam.fade,
			shot: cam.shot,
			shot_t: loop_t - start,
			sun,
			..crate::audio::Cues::default()
		}
	}

	fn hints(&self, cx: &Ctx) -> Hints {
		let fade = camera(cx.t as f32).fade;
		Hints {
			threshold: 135,
			streak_tint: mixf(self.shared.palette.hue(0), 0xFFFFFF, 0.35),
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
	/// The sun: a core of white through the sun hue, about a fiftieth of the height across, in a glow that falls off over a
	/// sixth of the height. The finishing chain's bloom and streaks do the rest.
	fn draw_sun(&self, out: &mut Frame, sx: f32, sy: f32) {
		let h = self.h as f32;
		let core = (h * 0.014).max(1.5);
		let reach = h * 0.17;
		let glow = mixf(self.shared.palette.sun, self.shared.palette.hue(0), 0.35);
		let hot = mixf(self.shared.palette.sun, 0xFFFFFF, 0.75);
		let y0 = ((sy - reach) as i32).max(0);
		let y1 = ((sy + reach) as i32 + 1).min(self.h as i32);
		let x0 = ((sx - reach) as i32).max(0);
		let x1 = ((sx + reach) as i32 + 1).min(self.w as i32);
		for y in y0..y1 {
			for x in x0..x1 {
				let (dx, dy) = (x as f32 + 0.5 - sx, y as f32 + 0.5 - sy);
				let d = sqrt(dx * dx + dy * dy);
				let i = y as usize * self.w + x as usize;
				let c = if d < core {
					hot
				} else if d < core + 1.0 {
					mixf(scale(glow, 230), hot, core + 1.0 - d)
				} else {
					let k = 1.0 - d / reach;
					if k <= 0.0 {
						continue;
					}
					scale(glow, (k * k * k * 190.0 + (core / d) * 110.0).min(255.0) as u32)
				};
				out.px[i] = screen(out.px[i], c);
			}
		}
	}

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
		let halo = 1.0 + 0.22 * self.shared.atmosphere;
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
		let (mw, mh) = (self.shared.map_width as i32, self.shared.map_height as i32);
		let spin_u = fract(spin) * mw as f32;
		let cloud_u = fract(cloud_spin) * mw as f32;
		let smooth = detail >= Detail::Medium;
		let shadows = detail >= Detail::Medium;
		let lights_on = detail >= Detail::High;
		let sheen = detail >= Detail::Ultra;
		let atmos = self.shared.atmosphere;
		let city = mixf(self.shared.palette.sun, 0xFFFFFF, 0.2);
		let cloud_lit = mixf(self.shared.palette.n(200), self.shared.palette.sun, 0.1);
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
							*pix = screen(*pix, self.shared.atmosphere_ramp.at(k * 0.9));
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
				let ground = sample_map(&self.shared.ground, mw, mh, u16, v16, smooth);
				// Light: a soft wrap round the terminator, which is tinted from the outer glow hue to the inner one.
				let ndl = nx * light.x + ny * light.y + nz * light.z;
				// Light is a colour: warm white by day, orange then rose at the terminator, purple dusk, indigo night.
				let lc = self.shared.light_ramp.c[(saturate((ndl + 0.3) * (1.0 / 1.3)) * 255.0) as usize];
				let mut c = tint(ground, lc);
				if ndl < 0.0 && lights_on {
					let l = sample_lights(&self.shared.lights, mw, mh, u16, v16);
					if l > 0 {
						c = screen(c, scale(city, l as u32 * (saturate(-ndl * 3.0) * 180.0) as u32 >> 8));
					}
				}
				// Clouds, lit like the ground, with their shadow a little towards the dark side.
				let cu = round_i((lon * scale_u + cloud_u) * 256.0) << 8;
				let cloud = sample_cover(&self.shared.clouds, mw, mh, cu, v16);
				if shadows {
					let su = cu + round_i(light.x * -6.0 * 65_536.0);
					let sh = sample_cover(&self.shared.clouds, mw, mh, su, v16 + round_i(light.y * 4.0 * 65_536.0));
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
						c = screen(
							c,
							scale(
								self.shared.palette.sun,
								(powf(saturate((hv - 0.9) * 10.0), 3.0) * 70.0) as u32,
							),
						);
					}
				}
				// The atmosphere inside the limb.
				let rim = 1.0 - nz;
				let k = rim * rim * rim * atmos * saturate(ndl * 1.5 + 0.45) * 1.6;
				if k > 0.003 {
					c = screen(c, self.shared.atmosphere_ramp.at(saturate(k)));
				}
				// Anti-alias the limb over the last pixel, against the background as the halo lights it just outside, so the edge
				// does not show a dark seam between the lit limb and the glow.
				let d = sqrt(d2);
				let edge = (1.0 - d) * r;
				*pix = if edge < 1.0 {
					let side = saturate((nx * light.x + ny * light.y) / d.max(1e-3) * 0.6 + 0.55);
					let k = side * atmos * 1.3;
					let behind = if k > 0.004 {
						screen(*pix, self.shared.atmosphere_ramp.at(k * 0.9))
					} else {
						*pix
					};
					mixf(behind, c, edge)
				} else {
					c
				};
			}
		}
	}

	fn draw_body(&self, out: &mut Frame, b: &(f32, f32, f32, f32, u32, bool), light: V3, t: f32, detail: Detail) {
		let &(bx, by, _z, br, idx, sat) = b;
		if sat {
			// A satellite: a glint and a blinking light.
			let blink = fract(t * 0.8 + idx as f32 * 0.37) < 0.12;
			let c = if blink {
				mixf(self.shared.palette.sun, 0xFFFFFF, 0.6)
			} else {
				self.shared.palette.n(300)
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
		let m = &self.shared.moons[idx as usize];
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
