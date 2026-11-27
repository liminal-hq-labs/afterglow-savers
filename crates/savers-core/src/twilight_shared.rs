// What Twilight's two editions share: the camera, the shot table, the seeded parameters, the painted maps and the colour ramps.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The CPU scene (`scenes::twilight`) is the reference; a graphics-card edition (SAV-D25, `docs/gpu-twilight.md`) draws the same
//! planet from the same data. Everything here is a pure function of the settings, the seed and the render size, so both editions get
//! the same maps, the same moons, satellites and stars, the same ramps and the same camera at any time. A shader uploads the maps
//! and ramps as textures and takes the camera as uniforms.

use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{self, Ramp, mixf};
use crate::frame::Frame;
use crate::math::{PI, TAU, V3, cos, ease, floor_i, fract, lerp, saturate, sin, smoothstep};
use crate::noise::{fbm2, fbm3, noise3};
use crate::palette::Palette;
use crate::rng::{Rng, hash2};
use crate::saver::palette_for;
use crate::settings::{Detail, Key, Settings};

/// One camera shot: where the planet sits, how big, and how the axis and light are turned, at the start and end of the shot.
#[derive(Clone, Copy)]
pub struct Shot {
	pub len: f32,
	/// Planet centre in screen fractions (x of the width, y of the height), and radius in heights.
	pub from: [f32; 7],
	pub to: [f32; 7],
}

// [cx, cy, radius, axis tilt towards the viewer (radians), roll (radians), light angle (radians about the vertical: 0 lights the
// right half, π/2 lights the face, π the left half, more than π the back, for a crescent), the sun's distance from the planet's
// centre on screen in planet radii (0 for no sun in the shot)]
pub const SHOTS: [Shot; 4] = [
	// The establishing shot: small, left third, turning.
	Shot {
		len: 25.0,
		from: [0.36, 0.52, 0.22, 0.35, -0.12, 0.35, 3.0],
		to: [0.40, 0.50, 0.25, 0.30, -0.08, 0.15, 3.3],
	},
	// The push-in towards the terminator.
	Shot {
		len: 25.0,
		from: [0.55, 0.55, 0.30, 0.25, 0.10, 0.1, 0.0],
		to: [0.62, 0.78, 0.75, 0.15, 0.18, -0.15, 0.0],
	},
	// From slightly above, a satellite passes close.
	Shot {
		len: 20.0,
		from: [0.50, 0.58, 0.34, 0.75, 0.0, 0.45, 0.0],
		to: [0.47, 0.56, 0.37, 0.70, -0.05, 0.3, 0.0],
	},
	// The drift along the night side, lit from behind: the sun slides out from behind the limb.
	Shot {
		len: 20.0,
		from: [0.70, 0.42, 0.45, 0.2, -0.25, 3.3, 0.78],
		to: [0.30, 0.47, 0.48, 0.25, -0.15, 3.55, 1.12],
	},
];

pub const FADE: f32 = 1.5;

/// The camera at one moment: the shot's values eased between its ends, and the fade at the cuts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
	/// Planet centre in screen fractions: x of the width, y of the height.
	pub centre_x: f32,
	pub centre_y: f32,
	/// Planet radius in heights.
	pub radius: f32,
	/// The axis's tilt towards the viewer, in radians.
	pub tilt: f32,
	/// The roll about the view axis, in radians.
	pub roll: f32,
	/// The light's angle about the vertical, in radians, before the slow drift the scene adds over time.
	pub light_angle: f32,
	/// The sun's distance from the planet's centre on screen, in planet radii; 0 for no sun in the shot.
	pub sun_distance: f32,
	/// 0 at a cut, 1 mid-shot.
	pub fade: f32,
	/// The index of the shot in [`SHOTS`].
	pub shot: usize,
}

impl Camera {
	fn from_values(v: [f32; 7], fade: f32, shot: usize) -> Camera {
		Camera {
			centre_x: v[0],
			centre_y: v[1],
			radius: v[2],
			tilt: v[3],
			roll: v[4],
			light_angle: v[5],
			sun_distance: v[6],
			fade,
			shot,
		}
	}
}

/// The loop's length in seconds: the shots end to end.
pub fn total_length() -> f32 {
	SHOTS.iter().map(|s| s.len).sum()
}

/// The camera at time `t` seconds, looping over the shot table.
pub fn camera(t: f32) -> Camera {
	let total = total_length();
	let loop_t = fract(t / total) * total;
	let mut start = 0.0;
	for (i, shot) in SHOTS.iter().enumerate() {
		if loop_t < start + shot.len || i == SHOTS.len() - 1 {
			let k = saturate((loop_t - start) / shot.len);
			let e = ease(k) * 0.7 + k * 0.3;
			let mut v = [0.0; 7];
			for (j, slot) in v.iter_mut().enumerate() {
				*slot = lerp(shot.from[j], shot.to[j], e);
			}
			let into = loop_t - start;
			let left = shot.len - into;
			let fade = saturate(into / FADE).min(saturate(left / FADE));
			return Camera::from_values(v, fade, i);
		}
		start += shot.len;
	}
	Camera::from_values(SHOTS[0].from, 1.0, 0)
}

/// A moon: its orbit round the planet (in planet radii), its size and its tint.
pub struct Moon {
	pub orbit: f32,
	pub incline: f32,
	pub speed: f32,
	pub phase: f32,
	pub radius: f32,
	pub tint: u32,
}

/// A satellite: a glint on an inclined orbit that blinks.
pub struct Satellite {
	pub orbit: f32,
	pub incline: f32,
	pub node: f32,
	pub speed: f32,
	pub phase: f32,
}

/// A background star: its place as screen fractions, its brightness, twinkle rate and tint.
pub struct Star {
	pub x: f32,
	pub y: f32,
	pub bright: f32,
	pub twinkle: f32,
	pub tint: u32,
}

/// The equirectangular map size (width, height) for a detail level and a render width. The maps grow with the render width at High
/// and Ultra, so a planet filling a large screen keeps its brush strokes: a 4K render gets 2048 × 1024.
pub fn map_size(detail: Detail, width: usize) -> (usize, usize) {
	let map_width = match detail {
		Detail::Low => 256,
		Detail::Medium => 512,
		Detail::High => (width / 2).next_power_of_two().clamp(512, 1024),
		Detail::Ultra => (width * 3 / 4).next_power_of_two().clamp(512, 2048),
	};
	(map_width, map_width / 2)
}

/// Everything the planet's scene is made of, built once from the settings, the seed and the render size.
pub struct Shared {
	pub palette: Palette,
	/// The map size: `map_width` is a power of two and `map_height` is half of it.
	pub map_width: usize,
	pub map_height: usize,
	/// The ground map, `map_width` × `map_height`, 0x00RRGGBB.
	pub ground: Vec<u32>,
	/// The night lights, 0 to 255, same size.
	pub lights: Vec<u8>,
	/// The cloud cover, 0 to 255, same size.
	pub clouds: Vec<u8>,
	/// The nebula behind the planet, three quarters of the render size.
	pub nebula: Frame,
	pub stars: Vec<Star>,
	pub moons: Vec<Moon>,
	pub satellites: Vec<Satellite>,
	/// The atmosphere's strength, 0 to 1 (the `Atmosphere` setting).
	pub atmosphere: f32,
	/// Whether the sun is shown (the `Sun` setting).
	pub sun: bool,
	/// The atmosphere's colour by intensity (index 0 to 255 for 0 to 1).
	pub atmosphere_ramp: Ramp,
	/// The light's colour by how directly it falls (index: n·l from -0.3 to 1).
	pub light_ramp: Ramp,
	/// The planet's spin in turns per second.
	pub spin_rate: f32,
}

impl Shared {
	/// Builds it all with the maps sized by [`map_size`] for the settings' detail level.
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> Shared {
		let (map_width, map_height) = map_size(settings.detail(), w);
		Shared::with_map_size(settings, seed, w, h, map_width, map_height)
	}

	/// As [`Shared::new`] with the map size chosen by the caller (a graphics card asks for 2048 × 1024 at any detail). The width
	/// must be a power of two and the height half of it.
	pub fn with_map_size(settings: &Settings, seed: u32, w: usize, h: usize, mw: usize, mh: usize) -> Shared {
		let pal = palette_for(settings);
		let mut rng = Rng::new(seed as u64 ^ 0x7E11_6A00_0000);
		let (ground, lights) = paint_ground(&pal, mw, mh, &mut rng.fork(1));
		let clouds = paint_clouds(mw, mh, &mut rng.fork(2));
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
		let satellites = (0..3)
			.map(|i| Satellite {
				orbit: rng.range(1.15, 1.5),
				incline: rng.range(-0.9, 0.9),
				node: rng.f() * TAU,
				speed: rng.range(0.12, 0.2) * if i % 2 == 0 { 1.0 } else { -1.0 },
				phase: rng.f() * TAU,
			})
			.collect();
		let atmos_hue = pal.hue(pal.glow_n - 1);
		let atmosphere_ramp = Ramp::new(&[
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
		Shared {
			palette: pal,
			map_width: mw,
			map_height: mh,
			ground,
			lights,
			clouds,
			nebula,
			stars,
			moons,
			satellites,
			atmosphere: settings.get(Key::Atmosphere) as f32 / 100.0,
			sun: settings.get(Key::Sun) == 1,
			atmosphere_ramp,
			light_ramp,
			spin_rate: rng.range(0.012, 0.02),
		}
	}
}

/// The planet's ground: elevation from domain-warped 3D noise on the sphere, cut into stepped terraces with a lit rim and a dark
/// foot on every step, like contour lines in a painting; dry and wet land from a second field; coastal shelves; brush strokes
/// along the latitude lines; ice at the poles; and lights near the coasts.
pub fn paint_ground(pal: &Palette, mw: usize, mh: usize, rng: &mut Rng) -> (Vec<u32>, Vec<u8>) {
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
pub fn paint_clouds(mw: usize, mh: usize, rng: &mut Rng) -> Vec<u8> {
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
pub fn paint_nebula(pal: &Palette, w: usize, h: usize, rng: &mut Rng) -> Frame {
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
#[cfg(test)]
mod tests {
	use super::*;
	use crate::settings::SaverKind;

	fn settings_at(detail: Detail) -> Settings {
		let mut settings = Settings::defaults(SaverKind::Twilight);
		settings.set(Key::Detail, detail as u32);
		settings
	}

	#[test]
	fn maps_are_the_same_for_the_same_seed() {
		let settings = settings_at(Detail::Low);
		let a = Shared::new(&settings, 7, 160, 90);
		let b = Shared::new(&settings, 7, 160, 90);
		assert_eq!(a.ground, b.ground);
		assert_eq!(a.lights, b.lights);
		assert_eq!(a.clouds, b.clouds);
		assert_eq!(a.nebula.px, b.nebula.px);
		assert_eq!(a.stars.len(), b.stars.len());
		assert_eq!(a.light_ramp.c, b.light_ramp.c);
		assert_eq!(a.atmosphere_ramp.c, b.atmosphere_ramp.c);
	}

	#[test]
	fn a_different_seed_paints_different_maps() {
		let settings = settings_at(Detail::Low);
		let a = Shared::new(&settings, 7, 160, 90);
		let b = Shared::new(&settings, 8, 160, 90);
		assert_ne!(a.ground, b.ground);
		assert_ne!(a.clouds, b.clouds);
	}

	#[test]
	fn map_sizes_follow_the_detail_level() {
		assert_eq!(map_size(Detail::Low, 3840), (256, 128));
		assert_eq!(map_size(Detail::Medium, 3840), (512, 256));
		assert_eq!(map_size(Detail::High, 3840), (1024, 512));
		assert_eq!(map_size(Detail::Ultra, 3840), (2048, 1024));
		assert_eq!(map_size(Detail::Ultra, 160), (512, 256));
		let shared = Shared::with_map_size(&settings_at(Detail::Low), 1, 64, 36, 512, 256);
		assert_eq!((shared.map_width, shared.map_height), (512, 256));
		assert_eq!(shared.ground.len(), 512 * 256);
		assert_eq!(shared.clouds.len(), 512 * 256);
	}

	#[test]
	fn the_sun_setting_is_read() {
		let mut settings = settings_at(Detail::Low);
		settings.set(Key::Sun, 0);
		assert!(!Shared::new(&settings, 1, 64, 36).sun);
		settings.set(Key::Sun, 1);
		assert!(Shared::new(&settings, 1, 64, 36).sun);
	}

	#[test]
	fn shots_start_and_end_at_their_table_values() {
		let mut start = 0.0;
		for (i, shot) in SHOTS.iter().enumerate() {
			let at_start = camera(start + 0.0005);
			assert_eq!(at_start.shot, i);
			assert!((at_start.centre_x - shot.from[0]).abs() < 1e-3);
			assert!((at_start.radius - shot.from[2]).abs() < 1e-3);
			assert!((at_start.sun_distance - shot.from[6]).abs() < 1e-3);
			assert!(at_start.fade < 0.01, "a cut fades from black");
			let at_end = camera(start + shot.len - 0.0005);
			assert_eq!(at_end.shot, i);
			assert!((at_end.centre_y - shot.to[1]).abs() < 1e-3);
			assert!((at_end.light_angle - shot.to[5]).abs() < 1e-3);
			assert!(at_end.fade < 0.01, "a cut fades to black");
			let middle = camera(start + shot.len * 0.5);
			assert_eq!(middle.fade, 1.0);
			start += shot.len;
		}
		assert_eq!(start, total_length());
	}

	#[test]
	fn the_camera_loops() {
		let total = total_length();
		let a = camera(3.0);
		let b = camera(3.0 + total);
		assert_eq!(a.shot, b.shot);
		assert!((a.centre_x - b.centre_x).abs() < 1e-3);
	}
}
