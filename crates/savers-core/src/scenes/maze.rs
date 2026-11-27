// Afterglow Maze: a walk through dusk architecture, with stepped stone and glass, polished floors and glowing sun windows.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A ray caster in the spirit of the classic 3D Maze (docs/scenes.md, "Maze"):
//!
//! - **The maze** is a recursive backtracker on a grid of cells, stored as blocks (walls between cells are blocks too). Each wall
//!   block has a material: stone, brick, glass or inlaid tiles by region, and a stained-glass sun window where a long corridor ends
//!   or at the exit.
//! - **The walls** are cast one screen column at a time. Their tops are crenellated: the top of each texture is cut away in
//!   merlons, and since every wall is the same height, a farther wall seen through a gap is always lower, so the cut-outs show sky.
//! - **The floor** is cast row by row and mirrors the walls: a floor pixel below a wall's foot shows the wall column mirrored about
//!   the foot, darkened and blended with the floor by a Fresnel term, rippled when it is water.
//! - **The light** is fog towards the sky's horizon colour, warm light pooled round the windows, and light shafts: a radial blur of
//!   the brightest window towards the viewer.
//! - **The camera** glides along a spline through the solution path at a speed that eases into turns, looking a little ahead, and
//!   walks into the exit window's light at the end, after which a new maze fades up.

use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{self, Ramp, mix, mixf, scale, screen};
use crate::frame::Frame;
use crate::math::{PI, TAU, atan2, clamp, cos, floor_i, fract, lerp, round_i, saturate, sin, smoothstep, sqrt};
use crate::noise::{fbm2, fbm2_tiled};
use crate::palette::Palette;
use crate::rng::{Rng, hash2};
use crate::saver::palette_for;
use crate::scene::{Ctx, Hints, Scene};
use crate::settings::{Detail, Key, Settings};

const EMPTY: u8 = 0;
const STONE: u8 = 1;
const BRICK: u8 = 2;
const GLASS: u8 = 3;
const TILES: u8 = 4;
const WINDOW: u8 = 5;
const MATERIALS: usize = 6;

/// Transparent texels (the gaps between merlons) carry this bit.
const CUT: u32 = 0x0100_0000;

/// The eye's height as a fraction of the wall's.
const EYE: f32 = 0.5;
/// Wall height against the corridor's width: low walls let the dusk sky in.
const WALL: f32 = 0.62;
/// Field of view: the half-width of the view plane at distance 1.
const PLANE: f32 = 0.88;
/// Walking speed in blocks a second on the straight.
const WALK: f32 = 0.95;
/// Seconds of fade at each end of a maze.
const FADE: f32 = 2.0;

/// Path samples are this far apart, in blocks.
const STEP: f32 = 0.05;

struct Maze {
	n: usize,
	blocks: Vec<u8>,
	/// Warm light from the windows at each block, 0 to 1.
	glow: Vec<f32>,
	/// The camera path, resampled every `STEP` blocks.
	path: Vec<(f32, f32)>,
	/// Distance along the path at each tenth of a second.
	schedule: Vec<f32>,
	duration: f32,
	start: f32,
}

pub struct MazeScene {
	w: usize,
	h: usize,
	pal: Palette,
	rng: Rng,
	size: usize,
	floor_kind: u32,
	tex: Vec<Vec<u32>>,
	ts: usize,
	floor_tex: Vec<u32>,
	sky: Vec<u32>,
	sky_w: usize,
	sky_h: usize,
	fog: u32,
	maze: Maze,
	maze_no: u32,
	/// Per screen column: wall distance, wall top and foot rows, texture column, material, and light.
	col_dist: Vec<f32>,
	col_top: Vec<i32>,
	col_foot: Vec<i32>,
	col_u: Vec<u16>,
	col_mat: Vec<u8>,
	col_light: Vec<u16>,
	reflect: bool,
	shafts: bool,
	ripple: bool,
	shaft_buf: Vec<u32>,
	/// Down a wall's height in 64 steps: the light (256 is unchanged) and the sky's warm light added at the top.
	wall_shade: Vec<u16>,
	wall_warm: Vec<u32>,
	/// The brightest window on screen this frame: column, row and strength.
	window: Option<(f32, f32, f32)>,
	fade: f32,
}

impl MazeScene {
	pub fn new(settings: &Settings, seed: u32, w: usize, h: usize) -> MazeScene {
		let detail = settings.detail();
		let pal = palette_for(settings);
		let mut rng = Rng::new(seed as u64 ^ 0x3A2E_0000_0000);
		let ts = if detail == Detail::Low { 64 } else { 128 };
		let tex = (0..MATERIALS)
			.map(|m| paint_wall(m as u8, ts, &pal, &mut rng.fork(m as u32 + 1)))
			.collect();
		let floor_kind = settings.get(Key::Floor);
		let floor_tex = paint_floor(floor_kind, ts, &pal, &mut rng.fork(20));
		let (sky, sky_w, sky_h) = paint_sky(&pal, h, &mut rng.fork(21));
		let size = settings.get(Key::MazeSize) as usize;
		let mut first = rng.fork(100);
		let maze = Maze::new(size, &mut first, 0.0);
		let fog = mixf(pal.n(800), pal.hue(pal.glow_n - 1), 0.35);
		MazeScene {
			w,
			h,
			pal,
			rng,
			size,
			floor_kind,
			tex,
			ts,
			floor_tex,
			sky,
			sky_w,
			sky_h,
			fog,
			maze,
			maze_no: 0,
			col_dist: vec![0.0; w],
			col_top: vec![0; w],
			col_foot: vec![0; w],
			col_u: vec![0; w],
			col_mat: vec![0; w],
			col_light: vec![0; w],
			reflect: detail >= Detail::Medium,
			shafts: detail >= Detail::High,
			ripple: detail >= Detail::Ultra || floor_kind == 1,
			shaft_buf: Vec::new(),
			wall_shade: (0..64)
				.map(|i| (270.0 - 140.0 * smoothstep(0.5, 1.0, i as f32 / 63.0)) as u16)
				.collect(),
			wall_warm: (0..64)
				.map(|i| {
					scale(
						mixf(pal.hue(0), pal.hue(1 % pal.glow_n), 0.4),
						(70.0 * (1.0 - smoothstep(0.0, 0.45, i as f32 / 63.0))) as u32,
					)
				})
				.collect(),
			window: None,
			fade: 1.0,
		}
	}
}

impl Maze {
	fn new(n: usize, rng: &mut Rng, start: f32) -> Maze {
		let g = 2 * n + 1;
		let mut blocks = vec![STONE; g * g];
		// The recursive backtracker, with an explicit stack.
		let mut seen = vec![false; n * n];
		let mut stack = vec![(0usize, 0usize)];
		seen[0] = true;
		blocks[g + 1] = EMPTY;
		while let Some(&(cx, cy)) = stack.last() {
			let mut options = [(0usize, 0usize); 4];
			let mut count = 0;
			for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
				let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
				if nx >= 0 && ny >= 0 && (nx as usize) < n && (ny as usize) < n && !seen[ny as usize * n + nx as usize] {
					options[count] = (nx as usize, ny as usize);
					count += 1;
				}
			}
			if count == 0 {
				stack.pop();
				continue;
			}
			// Carry straight on two times in three when that is possible, for long corridors with a view down them.
			let straight = stack.len().checked_sub(2).map(|i| stack[i]).and_then(|(px, py)| {
				let want = (2 * cx as i32 - px as i32, 2 * cy as i32 - py as i32);
				options[..count]
					.iter()
					.copied()
					.find(|&(ox, oy)| (ox as i32, oy as i32) == want)
			});
			let (nx, ny) = match straight {
				Some(next) if rng.below(3) < 2 => next,
				_ => options[rng.below(count as u32) as usize],
			};
			seen[ny * n + nx] = true;
			blocks[(2 * ny + 1) * g + 2 * nx + 1] = EMPTY;
			blocks[(cy + ny + 1) * g + cx + nx + 1] = EMPTY;
			stack.push((nx, ny));
		}
		// Materials by region: a coarse noise picks stone, brick, glass or tiles for each stretch of wall.
		let salt = rng.next_u32();
		for y in 0..g {
			for x in 0..g {
				if blocks[y * g + x] != EMPTY {
					let r = fbm2(x as f32 * 0.18, y as f32 * 0.18, 2, 0.5, salt);
					blocks[y * g + x] = if r < -0.25 {
						BRICK
					} else if r < 0.12 {
						STONE
					} else if r < 0.3 {
						TILES
					} else {
						GLASS
					};
				}
			}
		}
		// The solution: a breadth-first search over cells from the start to the far corner.
		let goal = (n - 1, n - 1);
		let mut prev = vec![usize::MAX; n * n];
		let mut queue = vec![0usize];
		prev[0] = 0;
		let mut head = 0;
		while head < queue.len() {
			let c = queue[head];
			head += 1;
			let (cx, cy) = (c % n, c / n);
			for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
				let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
				if nx < 0 || ny < 0 || nx as usize >= n || ny as usize >= n {
					continue;
				}
				let between = ((cy as i32 * 2 + 1 + dy) as usize) * g + (cx as i32 * 2 + 1 + dx) as usize;
				let next = ny as usize * n + nx as usize;
				if blocks[between] == EMPTY && prev[next] == usize::MAX {
					prev[next] = c;
					queue.push(next);
				}
			}
		}
		let mut cells = Vec::new();
		let mut c = goal.1 * n + goal.0;
		while c != 0 {
			cells.push(c);
			c = prev[c];
		}
		cells.push(0);
		cells.reverse();
		// Sun windows: where a straight run of three or more cells ends in a dead end, its end wall glows.
		for i in 0..n * n {
			let (cx, cy) = (i % n, i / n);
			let open: Vec<(i32, i32)> = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
				.into_iter()
				.filter(|&(dx, dy)| blocks[((cy as i32 * 2 + 1 + dy) as usize) * g + (cx as i32 * 2 + 1 + dx) as usize] == EMPTY)
				.collect();
			if open.len() != 1 {
				continue;
			}
			let (dx, dy) = open[0];
			let mut run = 0;
			let (mut x, mut y) = (cx as i32 * 2 + 1, cy as i32 * 2 + 1);
			while blocks[(y + dy) as usize * g + (x + dx) as usize] == EMPTY {
				x += 2 * dx;
				y += 2 * dy;
				run += 1;
				if x <= 0 || y <= 0 || x as usize >= g - 1 || y as usize >= g - 1 {
					break;
				}
			}
			if run >= 3 {
				blocks[((cy as i32 * 2 + 1 - dy) as usize) * g + (cx as i32 * 2 + 1 - dx) as usize] = WINDOW;
			}
		}
		// The exit: a window in the outer wall beyond the last cell.
		let (ex, ey) = (2 * goal.0 + 1, 2 * goal.1 + 1);
		blocks[ey * g + ex + 1] = WINDOW;
		// Warm light: falls off with distance from each window, through open blocks only.
		let mut glow = vec![0.0f32; g * g];
		for y in 0..g {
			for x in 0..g {
				if blocks[y * g + x] == WINDOW {
					for yy in y.saturating_sub(5)..(y + 6).min(g) {
						for xx in x.saturating_sub(5)..(x + 6).min(g) {
							let (ddx, ddy) = (xx as f32 - x as f32, yy as f32 - y as f32);
							let d = sqrt(ddx * ddx + ddy * ddy);
							let i = yy * g + xx;
							glow[i] = glow[i].max(saturate(1.0 - d / 5.0));
						}
					}
				}
			}
		}
		// The path: cell centres, ending inside the exit window's light, smoothed by a Catmull-Rom spline and resampled evenly.
		// Now and then the walk turns into a short dead-end branch and comes back round in a U-turn arc: in on one side of the
		// corridor, round, and out on the other.
		let centre = |c: usize| ((2 * (c % n) + 1) as f32 + 0.5, (2 * (c / n) + 1) as f32 + 0.5);
		let on_path: Vec<bool> = {
			let mut v = vec![false; n * n];
			for &c in &cells {
				v[c] = true;
			}
			v
		};
		let open = |c: usize, dx: i32, dy: i32| {
			let (cx, cy) = ((c % n) as i32, (c / n) as i32);
			let (nx, ny) = (cx + dx, cy + dy);
			nx >= 0
				&& ny >= 0 && (nx as usize) < n
				&& (ny as usize) < n
				&& blocks[((2 * cy + 1 + dy) as usize) * g + (2 * cx + 1 + dx) as usize] == EMPTY
		};
		let mut pts: Vec<(f32, f32)> = Vec::new();
		let mut detours = 0;
		for (i, &c) in cells.iter().enumerate() {
			pts.push(centre(c));
			if detours >= 2 || i == 0 || i + 1 == cells.len() || rng.below(3) != 0 {
				continue;
			}
			for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
				if !open(c, dx, dy) {
					continue;
				}
				let next = ((c / n) as i32 + dy) as usize * n + ((c % n) as i32 + dx) as usize;
				if on_path[next] {
					continue;
				}
				// Follow the branch while it runs straight on with no side ways; take it if it ends within three cells.
				let mut branch = vec![next];
				let mut cur = next;
				while branch.len() <= 3 && open(cur, dx, dy) {
					let side = open(cur, dy, dx) || open(cur, -dy, -dx);
					if side {
						break;
					}
					cur = ((cur / n) as i32 + dy) as usize * n + ((cur % n) as i32 + dx) as usize;
					branch.push(cur);
				}
				let ends = !open(cur, dx, dy) && !open(cur, dy, dx) && !open(cur, -dy, -dx);
				if !ends || branch.len() > 3 {
					continue;
				}
				let (nx, ny) = (-dy as f32 * 0.22, dx as f32 * 0.22);
				for &b in &branch[..branch.len() - 1] {
					let p = centre(b);
					pts.push((p.0 + nx, p.1 + ny));
				}
				let end = centre(cur);
				pts.push((end.0 + nx - dx as f32 * 0.2, end.1 + ny - dy as f32 * 0.2));
				pts.push((end.0 - nx - dx as f32 * 0.2, end.1 - ny - dy as f32 * 0.2));
				for &b in branch[..branch.len() - 1].iter().rev() {
					let p = centre(b);
					pts.push((p.0 - nx, p.1 - ny));
				}
				pts.push(centre(c));
				detours += 1;
				break;
			}
		}
		// Midpoints between cells keep the spline inside the corridors.
		let mut dense = Vec::new();
		for w in pts.windows(2) {
			dense.push(w[0]);
			dense.push(((w[0].0 + w[1].0) * 0.5, (w[0].1 + w[1].1) * 0.5));
		}
		dense.push(*pts.last().unwrap_or(&(1.5, 1.5)));
		dense.push((ex as f32 + 1.35, ey as f32 + 0.5));
		pts = dense;
		let mut curve = Vec::new();
		for i in 0..pts.len().saturating_sub(1) {
			let p0 = pts[i.saturating_sub(1)];
			let p1 = pts[i];
			let p2 = pts[i + 1];
			let p3 = pts[(i + 2).min(pts.len() - 1)];
			for k in 0..8 {
				let s = k as f32 / 8.0;
				curve.push(catmull(p0, p1, p2, p3, s));
			}
		}
		curve.push(*pts.last().unwrap_or(&(1.5, 1.5)));
		let mut path = vec![curve[0]];
		let mut carry = 0.0;
		for wnd in curve.windows(2) {
			let (a, b) = (wnd[0], wnd[1]);
			let seg = sqrt((b.0 - a.0) * (b.0 - a.0) + (b.1 - a.1) * (b.1 - a.1));
			let mut d = STEP - carry;
			while d <= seg {
				let k = d / seg;
				path.push((lerp(a.0, b.0, k), lerp(a.1, b.1, k)));
				d += STEP;
			}
			carry = seg - (d - STEP);
		}
		// The schedule: slower where the path turns.
		let len = path.len() as f32 * STEP;
		let mut schedule = vec![0.0];
		let mut s = 0.0;
		while s < len - STEP {
			let i = ((s / STEP) as usize).min(path.len() - 1);
			let j = (i + 30).min(path.len() - 1);
			let k = (i + 15).min(path.len() - 1);
			let turn = turn_angle(path[i], path[k], path[j]);
			let speed = WALK * (1.0 - 0.45 * saturate(turn / 1.2));
			s += speed * 0.1;
			schedule.push(s.min(len));
		}
		let duration = schedule.len() as f32 * 0.1;
		Maze {
			n,
			blocks,
			glow,
			path,
			schedule,
			duration,
			start,
		}
	}

	/// Position and heading at maze time `lt`.
	fn camera(&self, lt: f32) -> (f32, f32, f32) {
		let k = clamp(lt * 10.0, 0.0, (self.schedule.len() - 1) as f32);
		let i = k as usize;
		let j = (i + 1).min(self.schedule.len() - 1);
		let s = lerp(self.schedule[i], self.schedule[j], k - i as f32);
		let at = |s: f32| {
			let f = clamp(s / STEP, 0.0, (self.path.len() - 1) as f32);
			let a = f as usize;
			let b = (a + 1).min(self.path.len() - 1);
			let t = f - a as f32;
			(
				lerp(self.path[a].0, self.path[b].0, t),
				lerp(self.path[a].1, self.path[b].1, t),
			)
		};
		let p = at(s);
		// Look ahead along the path, so the camera turns into a corner before reaching it.
		let ahead = at(s + 0.7);
		let behind = at(s - 0.4);
		let heading = atan2(ahead.1 - behind.1, ahead.0 - behind.0);
		(p.0, p.1, heading)
	}
}

fn turn_angle(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
	let h1 = atan2(b.1 - a.1, b.0 - a.0);
	let h2 = atan2(c.1 - b.1, c.0 - b.0);
	let mut d = h2 - h1;
	while d > PI {
		d -= TAU;
	}
	while d < -PI {
		d += TAU;
	}
	d.abs()
}

fn catmull(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32) -> (f32, f32) {
	let t2 = t * t;
	let t3 = t2 * t;
	let f = |a: f32, b: f32, c: f32, d: f32| {
		0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3)
	};
	(f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
}

/// A wall texture, `ts` × `ts`, top row first; texels in the gaps between merlons carry `CUT`.
fn paint_wall(material: u8, ts: usize, pal: &Palette, rng: &mut Rng) -> Vec<u32> {
	let mut tex = vec![0u32; ts * ts];
	let seed = rng.next_u32();
	let s = ts as f32;
	let n = pal.glow_n;
	let stone = mixf(pal.n(600), pal.hue(0), 0.2);
	let mortar = mixf(pal.n(800), pal.hue(n - 1), 0.2);
	let brick = mixf(mixf(pal.n(600), pal.hue(1), 0.5), pal.hue(0), 0.2);
	let glass_a = mixf(pal.n(700), pal.hue(n - 1), 0.55);
	let glass_b = mixf(pal.n(600), pal.hue((n - 1).max(1) - 1), 0.4);
	let lead = pal.n(800);
	for y in 0..ts {
		for x in 0..ts {
			let (u, v) = (x as f32 / s, y as f32 / s);
			let grain = fbm2_tiled(u * 8.0, v * 8.0, 8, 3, 0.5, seed) * 0.12;
			// The crenellated top: four merlons a texture, the gaps cut away above the top eighth.
			let crenel = v < 0.13 && fract(u * 4.0 + 0.25) < 0.45;
			let c = match material {
				BRICK => {
					let row = (v * 12.0) as i32;
					let offset = if row % 2 == 0 { 0.0 } else { 0.5 };
					let bu = fract(u * 4.0 + offset);
					let bv = fract(v * 12.0);
					if bu < 0.06 || bv < 0.12 {
						mortar
					} else {
						let k = (hash2((u * 4.0 + offset) as i32, row, seed) & 255) as f32 / 255.0;
						colour::shade(brick, (k - 0.5) * 0.25 + grain)
					}
				}
				GLASS => {
					// Panes in a lead frame, each pane a slightly different cool tint, brighter at its top.
					let pu = fract(u * 3.0);
					let pv = fract(v * 4.0);
					if pu < 0.08 || pv < 0.08 || v < 0.16 {
						colour::shade(stone, grain)
					} else if pu < 0.12 || pv < 0.12 {
						lead
					} else {
						let k = (hash2((u * 3.0) as i32, (v * 4.0) as i32, seed) & 255) as f32 / 255.0;
						colour::shade(mixf(glass_a, glass_b, k), 0.25 * (1.0 - pv) + grain * 0.5)
					}
				}
				TILES => {
					// Stone with an inlaid band of small mosaic tiles across the middle.
					if v > 0.45 && v < 0.62 {
						let tu = (u * 16.0) as i32;
						let tv = ((v - 0.45) * 16.0 / 0.17 * 0.25) as i32;
						let edge = fract(u * 16.0) < 0.12 || fract((v - 0.45) / 0.17 * 4.0) < 0.12;
						if edge {
							mortar
						} else {
							colour::shade(pal.hue(((tu + tv * 3) as usize) % n), -0.15 + grain)
						}
					} else {
						stone_block(u, v, stone, mortar, grain, seed)
					}
				}
				WINDOW => {
					// A stained-glass sun: stepped rings of the sun and glow hues, lead lines radiating, in a stone frame.
					let (dx, dy) = (u - 0.5, v - 0.55);
					let d = sqrt(dx * dx + dy * dy) * 2.6;
					if d > 1.0 || v < 0.14 {
						stone_block(u, v, stone, mortar, grain, seed)
					} else {
						let ang = atan2(dy, dx);
						let spoke = fract(ang / TAU * 16.0);
						let ring = (d * 4.0) as usize;
						let glass = match ring {
							0 => mixf(pal.sun, 0xFFFFFF, 0.6),
							1 => pal.sun,
							2 => pal.hue(0),
							_ => pal.hue(1 % n),
						};
						if spoke < 0.06 || fract(d * 4.0) < 0.08 { lead } else { glass }
					}
				}
				_ => stone_block(u, v, stone, mortar, grain, seed),
			};
			tex[y * ts + x] = if crenel { CUT } else { c };
		}
	}
	tex
}

/// Stone courses of uneven blocks with mortar.
fn stone_block(u: f32, v: f32, stone: u32, mortar: u32, grain: f32, seed: u32) -> u32 {
	let course = (v * 6.0) as i32;
	let shift = (hash2(course, 7, seed) & 255) as f32 / 255.0;
	let width = 1.5 + ((hash2(course, 9, seed) & 3) as f32) * 0.5;
	let bu = fract(u * width + shift);
	let bv = fract(v * 6.0);
	if bu < 0.04 || bv < 0.07 {
		return mortar;
	}
	let k = (hash2((u * width + shift) as i32, course, seed) & 255) as f32 / 255.0;
	// Each block a little lighter at its top edge, as if bevelled.
	colour::shade(stone, (k - 0.5) * 0.2 + grain + (1.0 - bv) * 0.08)
}

/// The floor: polished stone slabs (0), dark water (1) or inlaid tiles (2), tiling, `ts` × `ts`.
fn paint_floor(kind: u32, ts: usize, pal: &Palette, rng: &mut Rng) -> Vec<u32> {
	let seed = rng.next_u32();
	let s = ts as f32;
	let n = pal.glow_n;
	let mut tex = vec![0u32; ts * ts];
	for y in 0..ts {
		for x in 0..ts {
			let (u, v) = (x as f32 / s, y as f32 / s);
			let grain = fbm2_tiled(u * 6.0, v * 6.0, 6, 3, 0.5, seed) * 0.1;
			tex[y * ts + x] = match kind {
				1 => colour::shade(mixf(pal.n(800), pal.hue(n - 1), 0.25), grain * 0.5),
				2 => {
					let (tu, tv) = ((u * 4.0) as i32, (v * 4.0) as i32);
					if fract(u * 4.0) < 0.05 || fract(v * 4.0) < 0.05 {
						pal.n(800)
					} else if (tu + tv) % 2 == 0 {
						colour::shade(mixf(pal.n(600), pal.hue(1 % n), 0.3), grain)
					} else {
						let d = (fract(u * 4.0) - 0.5).abs() + (fract(v * 4.0) - 0.5).abs();
						if d < 0.25 {
							colour::shade(pal.hue(0), -0.2)
						} else {
							colour::shade(pal.n(500), grain)
						}
					}
				}
				_ => {
					if fract(u * 2.0) < 0.02 || fract(v * 2.0) < 0.02 {
						pal.n(800)
					} else {
						let k = (hash2((u * 2.0) as i32, (v * 2.0) as i32, seed) & 255) as f32 / 255.0;
						colour::shade(mixf(pal.n(600), pal.hue(n - 1), 0.12), (k - 0.5) * 0.12 + grain)
					}
				}
			};
		}
	}
	tex
}

/// The sky as a panorama: angle across (1024 columns), rows down to the horizon; a dusk gradient with clouds and stars.
fn paint_sky(pal: &Palette, h: usize, rng: &mut Rng) -> (Vec<u32>, usize, usize) {
	let seed = rng.next_u32();
	let sw = 1024;
	let sh = (h / 2 + 2).max(8);
	let n = pal.glow_n;
	let ramp = Ramp::new(&[
		(0.0, pal.ground()),
		(0.45, mixf(pal.ground(), pal.hue(n - 1), 0.45)),
		(0.75, mixf(pal.hue(n - 1), pal.hue(1 % n), 0.5)),
		(0.92, mixf(pal.hue(1 % n), pal.hue(0), 0.5)),
		(1.0, mixf(pal.hue(0), pal.sun, 0.4)),
	]);
	let mut sky = vec![0u32; sw * sh];
	for y in 0..sh {
		let k = y as f32 / (sh - 1) as f32;
		let base = ramp.at(k * k * 0.95 + k * 0.05);
		for x in 0..sw {
			let a = x as f32 / sw as f32;
			// Long thin clouds, lit from below by the horizon's colour.
			let c = fbm2_tiled(a * 16.0, k * 6.0, 16, 4, 0.5, seed);
			let cloud = smoothstep(0.15, 0.5, c) * smoothstep(0.2, 0.6, k) * (1.0 - smoothstep(0.85, 1.0, k));
			let mut col = mixf(base, mixf(pal.hue(1 % n), pal.sun, 0.3), cloud * 0.35);
			let star = hash2(x as i32, y as i32, seed) & 0xFFF;
			if k < 0.55 && star < 3 {
				col = screen(col, scale(pal.n(100), (200.0 * (1.0 - k)) as u32));
			}
			sky[y * sw + x] = col;
		}
	}
	(sky, sw, sh)
}

impl Scene for MazeScene {
	fn render(&mut self, cx: &Ctx, out: &mut Frame) {
		let t = cx.t as f32;
		// A new maze when this one has been walked.
		if t > self.maze.start + self.maze.duration + FADE {
			self.maze_no += 1;
			let mut r = self.rng.fork(100 + self.maze_no);
			self.maze = Maze::new(self.size, &mut r, t);
		}
		let lt = t - self.maze.start;
		let (px, py, heading) = self.maze.camera(lt);
		// Fades: up at the start, and at the end a bloom into the exit's light, then down.
		let end = self.maze.duration;
		self.fade = saturate(lt / FADE) * (1.0 - saturate((lt - end) / FADE));
		let white = smoothstep(end - 3.0, end, lt);
		let bob = sin(lt * 3.6) * 0.006;
		let (w, h) = (self.w, self.h);
		let horizon = h as f32 * (0.5 + bob);
		let (dx, dy) = (cos(heading), sin(heading));
		let (plx, ply) = (-dy * PLANE, dx * PLANE);
		let g = 2 * self.maze.n + 1;
		let ts = self.ts as i32;
		self.window = None;
		let mut best_window = 0.0f32;
		// Pass 1: cast each column, draw sky and wall, remember what is there.
		for x in 0..w {
			let camx = 2.0 * (x as f32 + 0.5) / w as f32 - 1.0;
			let (rx, ry) = (dx + plx * camx, dy + ply * camx);
			let (mut mx, mut my) = (floor_i(px), floor_i(py));
			let ddx = if rx == 0.0 { 1e9 } else { (1.0 / rx).abs() };
			let ddy = if ry == 0.0 { 1e9 } else { (1.0 / ry).abs() };
			let (stepx, mut sdx) = if rx < 0.0 {
				(-1, (px - mx as f32) * ddx)
			} else {
				(1, (mx as f32 + 1.0 - px) * ddx)
			};
			let (stepy, mut sdy) = if ry < 0.0 {
				(-1, (py - my as f32) * ddy)
			} else {
				(1, (my as f32 + 1.0 - py) * ddy)
			};
			let mut side = 0;
			let mut mat = STONE;
			for _ in 0..128 {
				if sdx < sdy {
					sdx += ddx;
					mx += stepx;
					side = 0;
				} else {
					sdy += ddy;
					my += stepy;
					side = 1;
				}
				if mx < 0 || my < 0 || mx as usize >= g || my as usize >= g {
					break;
				}
				let b = self.maze.blocks[my as usize * g + mx as usize];
				if b != EMPTY {
					mat = b;
					break;
				}
			}
			let dist = if side == 0 { sdx - ddx } else { sdy - ddy }.max(0.05);
			let hitx = if side == 0 { py + dist * ry } else { px + dist * rx };
			let mut u = fract(hitx);
			if (side == 0 && rx > 0.0) || (side == 1 && ry < 0.0) {
				u = 1.0 - u;
			}
			let wall_h = h as f32 / dist * WALL;
			let top = horizon - wall_h * (1.0 - EYE);
			let foot = horizon + wall_h * EYE;
			// The light at the face: the windows' warm pool, and a little less on one side for shape.
			let (lx, ly) = (
				if side == 0 { mx - stepx } else { mx },
				if side == 1 { my - stepy } else { my },
			);
			let glow = if lx >= 0 && ly >= 0 && (lx as usize) < g && (ly as usize) < g {
				self.maze.glow[ly as usize * g + lx as usize]
			} else {
				0.0
			};
			let light = (if side == 1 { 0.82 } else { 1.0 }) * 256.0;
			self.col_dist[x] = dist;
			self.col_top[x] = top as i32;
			self.col_foot[x] = foot as i32;
			self.col_u[x] = ((u * ts as f32) as i32).clamp(0, ts - 1) as u16;
			self.col_mat[x] = mat;
			self.col_light[x] = (light * (1.0 + glow * 0.5)) as u16;
			if mat == WINDOW {
				let strength = 1.0 / dist;
				if strength > best_window {
					best_window = strength;
					self.window = Some((x as f32, (top + foot) * 0.5, strength));
				}
			}
			// Sky above the horizon from the panorama, by the ray's angle.
			let ray_ang = atan2(ry, rx);
			let sky_x = ((fract(ray_ang / TAU) * self.sky_w as f32) as usize) % self.sky_w;
			let hz = horizon as i32;
			let sky_rows = self.sky_h as i32;
			for y in 0..hz.min(h as i32).max(0) {
				let sy = (sky_rows - (hz - y)).max(0) as usize;
				out.px[y as usize * w + x] = self.sky[sy * self.sky_w + sky_x];
			}
			// The wall slice.
			let tex = &self.tex[mat as usize];
			let fogk = (saturate((dist - 1.0) / 9.0) * 230.0) as u32;
			let fogk = if mat == WINDOW { fogk / 3 } else { fogk };
			let tu = self.col_u[x] as usize;
			let span = foot - top;
			let v_step = ((ts as f32 / span) * 65_536.0) as i32;
			let y0 = (top.max(0.0)) as i32;
			let y1 = (foot as i32).min(h as i32);
			let mut v = ((y0 as f32 - top) / span * ts as f32 * 65_536.0) as i32;
			let lightk = self.col_light[x] as u32;
			for y in y0..y1 {
				let tv = (v >> 16).clamp(0, ts - 1);
				let texel = tex[tv as usize * self.ts + tu];
				v += v_step;
				if texel & CUT != 0 {
					continue;
				}
				// Dusk light from the sky warms the tops of the walls; the feet fall into shadow.
				let k = lightk * self.wall_shade[(tv * 64 / ts) as usize] as u32 >> 8;
				let lit = if mat == WINDOW {
					scale_wide(texel, 330)
				} else {
					screen(scale_wide(texel, k), self.wall_warm[(tv * 64 / ts) as usize])
				};
				out.px[y as usize * w + x] = mix(lit, self.fog, fogk);
			}
		}
		// Pass 2: the floor, row by row, with the walls mirrored in it.
		let (r0x, r0y) = (dx - plx, dy - ply);
		let (r1x, r1y) = (dx + plx, dy + ply);
		let fts = self.ts as i32;
		let water = self.floor_kind == 1;
		let reflectivity = if water {
			0.75
		} else if self.floor_kind == 2 {
			0.25
		} else {
			0.4
		};
		let start = (horizon as i32 + 1).max(0) as usize;
		for y in start..h {
			let p = y as f32 - horizon;
			let row_dist = (EYE * h as f32 * WALL) / p;
			let fx0 = px + row_dist * r0x;
			let fy0 = py + row_dist * r0y;
			let sx = row_dist * (r1x - r0x) / w as f32;
			let sy = row_dist * (r1y - r0y) / w as f32;
			let mut fu = round_i(fx0 * fts as f32 * 256.0);
			let mut fv = round_i(fy0 * fts as f32 * 256.0);
			let su = round_i(sx * fts as f32 * 256.0);
			let sv = round_i(sy * fts as f32 * 256.0);
			let fogk = (saturate((row_dist - 1.0) / 9.0) * 230.0) as u32;
			// Grazing rows reflect more (Fresnel).
			let fres = saturate(reflectivity * (0.55 + 0.6 * (1.0 - saturate(p / (h as f32 * 0.5)))));
			let rk = (fres * 256.0) as u32;
			let wob = if self.ripple {
				sin(row_dist * 9.0 - t * 2.6) * (2.0 + 6.0 / row_dist)
			} else {
				0.0
			};
			for x in 0..w {
				let foot = self.col_foot[x];
				if (y as i32) < foot {
					fu = fu.wrapping_add(su);
					fv = fv.wrapping_add(sv);
					continue;
				}
				let ft = self.floor_tex[(((fv >> 8) & (fts - 1)) * fts + ((fu >> 8) & (fts - 1))) as usize];
				fu = fu.wrapping_add(su);
				fv = fv.wrapping_add(sv);
				let mut c = ft;
				if self.reflect {
					// The wall column mirrored about its foot; above its top, the mirrored sky (darker).
					let my = 2 * foot - y as i32 + wob as i32;
					let top = self.col_top[x];
					let refl = if my >= top && my < foot {
						let span = (foot - self.col_top[x]).max(1);
						let tv = ((my - top) * fts / span).clamp(0, fts - 1) as usize;
						let mat = self.col_mat[x] as usize;
						let texel = self.tex[mat][tv * self.ts + self.col_u[x] as usize];
						if texel & CUT != 0 {
							self.sky[(self.sky_h - 2) * self.sky_w]
						} else if mat == WINDOW as usize {
							texel
						} else {
							scale_wide(texel, self.col_light[x] as u32)
						}
					} else if my >= 0 && my < top {
						out.px[my as usize * w + x]
					} else {
						c
					};
					c = mix(c, scale(refl, 200), rk);
				}
				out.px[y * w + x] = mix(c, self.fog, fogk);
			}
		}
		if self.shafts
			&& let Some((wx, wy, strength)) = self.window
		{
			self.light_shafts(out, wx, wy, strength);
		}
		if white > 0.0 {
			let hot = mixf(self.pal.sun, 0xFFFFFF, 0.6);
			for px in out.px.iter_mut() {
				*px = mixf(*px, hot, white * 0.6);
			}
		}
	}

	fn hints(&self, _cx: &Ctx) -> Hints {
		Hints {
			threshold: 170,
			streak_tint: mixf(self.pal.sun, 0xFFFFFF, 0.25),
			fade: self.fade,
			bloom_gain: 1.1,
		}
	}
}

impl MazeScene {
	/// Light shafts: at a quarter of the size, the bright parts are blurred along lines towards the window's centre, then added.
	fn light_shafts(&mut self, out: &mut Frame, wx: f32, wy: f32, strength: f32) {
		let (w, h) = (out.w, out.h);
		let (qw, qh) = ((w / 4).max(1), (h / 4).max(1));
		self.shaft_buf.clear();
		self.shaft_buf.resize(qw * qh, 0);
		// The bright pass at a quarter of the size.
		let mut bright = vec![0u32; qw * qh];
		for y in 0..qh {
			for x in 0..qw {
				let c = out.px[(y * 4).min(h - 1) * w + (x * 4).min(w - 1)];
				let l = colour::luma(c);
				bright[y * qw + x] = if l > 170 { scale(c, ((l - 170) * 3).min(256)) } else { 0 };
			}
		}
		let (cxq, cyq) = (wx / 4.0, wy / 4.0);
		let k = saturate(strength * 1.2) * 0.55;
		for y in 0..qh {
			for x in 0..qw {
				let (mut sx, mut sy) = (x as f32, y as f32);
				let (stx, sty) = ((cxq - sx) / 14.0, (cyq - sy) / 14.0);
				let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
				let mut weight = 256u32;
				for _ in 0..14 {
					let (ix, iy) = (sx as i32, sy as i32);
					if ix >= 0 && iy >= 0 && (ix as usize) < qw && (iy as usize) < qh {
						let c = bright[iy as usize * qw + ix as usize];
						r += colour::r(c) * weight >> 8;
						g += colour::g(c) * weight >> 8;
						b += colour::b(c) * weight >> 8;
					}
					weight = weight * 240 >> 8;
					sx += stx;
					sy += sty;
				}
				let s = (k * 256.0 / 7.0) as u32;
				let c = ((r * s >> 8).min(255) << 16) | ((g * s >> 8).min(255) << 8) | (b * s >> 8).min(255);
				self.shaft_buf[y * qw + x] = c;
			}
		}
		// Enlarge bilinearly, so the shafts stay soft.
		let mut shafts = Frame::new(qw, qh);
		shafts.px.copy_from_slice(&self.shaft_buf);
		let (sx_step, sy_step) = (((qw as i64) << 16) / w as i64, ((qh as i64) << 16) / h as i64);
		for y in 0..h {
			let sy = (y as i64 * sy_step - 32_768).max(0) as i32;
			let mut sx = -32_768i64;
			for x in 0..w {
				let c = shafts.sample(sx.max(0) as i32, sy);
				if c != 0 {
					out.px[y * w + x] = screen(out.px[y * w + x], c);
				}
				sx += sx_step;
			}
		}
	}
}

/// Scales a colour by `k` in 0 to 511 (256 is unchanged), saturating.
#[inline(always)]
fn scale_wide(c: u32, k: u32) -> u32 {
	if k <= 256 {
		return scale(c, k);
	}
	let ch = |v: u32| (v * k >> 8).min(255);
	(ch(colour::r(c)) << 16) | (ch(colour::g(c)) << 8) | ch(colour::b(c))
}
