// An original stroke font: each glyph is a few polylines on a 4 × 6 grid, drawn as thick round-capped strokes at any size.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! SAV-D15. Capitals, digits and common punctuation; lower case is drawn as capitals, and anything else as `?`. A text is drawn
//! once into an alpha strip, anti-aliased by the distance from each pixel to the nearest stroke.

use alloc::vec;
use alloc::vec::Vec;

use crate::math::{saturate, sqrt};

/// A glyph: its advance in grid units and its strokes, as "x,y x,y|x,y x,y" (y down, 0 to 6; a lone point is a dot).
fn glyph(c: char) -> (f32, &'static str) {
	match c.to_ascii_uppercase() {
		'A' => (5.0, "0,6 2,0 4,6|0.7,4 3.3,4"),
		'B' => (5.0, "0,0 0,6 3,6 4,5 4,4 3,3 0,3|0,0 3,0 4,1 4,2 3,3"),
		'C' => (5.0, "4,1 3,0 1,0 0,1 0,5 1,6 3,6 4,5"),
		'D' => (5.0, "0,0 0,6 3,6 4,5 4,1 3,0 0,0"),
		'E' => (5.0, "4,0 0,0 0,6 4,6|0,3 3,3"),
		'F' => (5.0, "4,0 0,0 0,6|0,3 3,3"),
		'G' => (5.0, "4,1 3,0 1,0 0,1 0,5 1,6 3,6 4,5 4,3 2,3"),
		'H' => (5.0, "0,0 0,6|4,0 4,6|0,3 4,3"),
		'I' => (4.0, "0.5,0 2.5,0|1.5,0 1.5,6|0.5,6 2.5,6"),
		'J' => (5.0, "4,0 4,5 3,6 1,6 0,5"),
		'K' => (5.0, "0,0 0,6|4,0 0,3.5|1.5,2.5 4,6"),
		'L' => (5.0, "0,0 0,6 4,6"),
		'M' => (5.5, "0,6 0,0 2.25,3.5 4.5,0 4.5,6"),
		'N' => (5.0, "0,6 0,0 4,6 4,0"),
		'O' => (5.0, "1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,1 1,0"),
		'P' => (5.0, "0,6 0,0 3,0 4,1 4,2 3,3 0,3"),
		'Q' => (5.0, "1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,1 1,0|2.5,4.5 4,6"),
		'R' => (5.0, "0,6 0,0 3,0 4,1 4,2 3,3 0,3|2,3 4,6"),
		'S' => (5.0, "4,1 3,0 1,0 0,1 0,2 1,3 3,3 4,4 4,5 3,6 1,6 0,5"),
		'T' => (5.0, "0,0 4,0|2,0 2,6"),
		'U' => (5.0, "0,0 0,5 1,6 3,6 4,5 4,0"),
		'V' => (5.0, "0,0 2,6 4,0"),
		'W' => (5.5, "0,0 1.1,6 2.25,2.5 3.4,6 4.5,0"),
		'X' => (5.0, "0,0 4,6|4,0 0,6"),
		'Y' => (5.0, "0,0 2,3 4,0|2,3 2,6"),
		'Z' => (5.0, "0,0 4,0 0,6 4,6"),
		'0' => (5.0, "1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,1 1,0|1,4.5 3,1.5"),
		'1' => (4.0, "0.5,1 1.5,0 1.5,6|0.5,6 2.5,6"),
		'2' => (5.0, "0,1 1,0 3,0 4,1 4,2 0,6 4,6"),
		'3' => (5.0, "0,1 1,0 3,0 4,1 4,2 3,3 4,4 4,5 3,6 1,6 0,5|1.5,3 3,3"),
		'4' => (5.0, "3,6 3,0 0,4 4,4"),
		'5' => (5.0, "4,0 0,0 0,3 3,3 4,4 4,5 3,6 0,6"),
		'6' => (5.0, "4,1 3,0 1,0 0,1 0,5 1,6 3,6 4,5 4,4 3,3 0,3"),
		'7' => (5.0, "0,0 4,0 1.5,6"),
		'8' => (5.0, "1,0 3,0 4,1 4,2 3,3 1,3 0,4 0,5 1,6 3,6 4,5 4,4 3,3|1,3 0,2 0,1 1,0"),
		'9' => (5.0, "4,3 1,3 0,2 0,1 1,0 3,0 4,1 4,5 3,6 1,6 0,5"),
		'.' => (2.0, "0.5,6"),
		',' => (2.0, "0.8,5.6 0.2,7"),
		'!' => (2.0, "0.5,0 0.5,4|0.5,6"),
		'?' => (5.0, "0,1 1,0 3,0 4,1 4,2 2,3.5 2,4|2,6"),
		'\'' => (2.0, "0.5,0 0.5,1.5"),
		'"' => (3.0, "0.3,0 0.3,1.5|1.7,0 1.7,1.5"),
		'-' => (4.0, "0.3,3 2.7,3"),
		'_' => (5.0, "0,6.5 4,6.5"),
		':' => (2.0, "0.5,2|0.5,5.5"),
		';' => (2.0, "0.5,2|0.6,5.4 0.1,6.6"),
		'(' => (3.0, "1.8,0 0.6,1.5 0.6,4.5 1.8,6"),
		')' => (3.0, "0.2,0 1.4,1.5 1.4,4.5 0.2,6"),
		'/' => (4.0, "3,0 0,6"),
		'&' => (5.0, "4,6 1,2 1,1 2,0 3,1 3,2 0,4 0,5 1,6 2,6 4,4"),
		'+' => (5.0, "2,1.5 2,4.5|0.5,3 3.5,3"),
		'=' => (5.0, "0.5,2 3.5,2|0.5,4 3.5,4"),
		'*' => (5.0, "2,1 2,5|0.5,2 3.5,4|0.5,4 3.5,2"),
		'#' => (5.0, "1.2,0.5 1.2,5.5|2.8,0.5 2.8,5.5|0,2 4,2|0,4 4,4"),
		'%' => (5.0, "4,0 0,6|0.5,0.5|3.5,5.5"),
		'<' => (4.0, "3,1 0,3 3,5"),
		'>' => (4.0, "0,1 3,3 0,5"),
		' ' => (3.0, ""),
		_ => (5.0, "0,1 1,0 3,0 4,1 4,2 2,3.5 2,4|2,6"),
	}
}

/// The strokes of a glyph as segments (x0, y0, x1, y1) in grid units.
fn segments(spec: &str, out: &mut Vec<[f32; 4]>) {
	out.clear();
	for stroke in spec.split('|') {
		let mut prev: Option<(f32, f32)> = None;
		let mut count = 0;
		for pt in stroke.split(' ').filter(|s| !s.is_empty()) {
			let mut it = pt.split(',');
			let x = it.next().and_then(parse).unwrap_or(0.0);
			let y = it.next().and_then(parse).unwrap_or(0.0);
			if let Some((px, py)) = prev {
				out.push([px, py, x, y]);
			}
			prev = Some((x, y));
			count += 1;
		}
		if count == 1
			&& let Some((x, y)) = prev
		{
			out.push([x, y, x, y]);
		}
	}
}

/// Parses a short decimal like "3.25" or "-0.5" without `core::str::parse` for floats (which pulls in a large formatter).
fn parse(s: &str) -> Option<f32> {
	let (neg, s) = match s.strip_prefix('-') {
		Some(rest) => (true, rest),
		None => (false, s),
	};
	let mut v = 0.0f32;
	let mut scale = 0.0f32;
	for b in s.bytes() {
		match b {
			b'0'..=b'9' => {
				let d = (b - b'0') as f32;
				if scale == 0.0 {
					v = v * 10.0 + d;
				} else {
					v += d * scale;
					scale *= 0.1;
				}
			}
			b'.' => scale = 0.1,
			_ => return None,
		}
	}
	Some(if neg { -v } else { v })
}

/// A line of text drawn into alpha values, `w` × `h`, 0 to 255.
pub struct Strip {
	pub w: usize,
	pub h: usize,
	pub alpha: Vec<u8>,
}

/// The width in pixels a text takes at a height of `h` pixels.
pub fn measure(text: &str, h: usize) -> usize {
	let unit = h as f32 / 8.0;
	let mut w = 0.0;
	for c in text.chars() {
		w += (glyph(c).0 + 0.6) * unit;
	}
	w as usize + 1
}

/// Draws `text` at a height of `h` pixels (a cap height of three quarters of it), strokes `weight` grid units thick (0.9 is bold).
pub fn render(text: &str, h: usize, weight: f32) -> Strip {
	let unit = h as f32 / 8.0;
	let w = measure(text, h);
	let mut alpha = vec![0u8; w * h];
	let mut segs = Vec::new();
	let radius = weight * 0.5;
	let top = unit * 0.7;
	let mut pen = unit * 0.3;
	for c in text.chars() {
		let (adv, spec) = glyph(c);
		segments(spec, &mut segs);
		if !segs.is_empty() {
			let x0 = (pen - unit) as i32;
			let x1 = (pen + (adv + 0.5) * unit) as i32 + 1;
			for py in 0..h {
				let gy = (py as f32 + 0.5 - top) / unit;
				for px in x0.max(0)..x1.min(w as i32) {
					let gx = (px as f32 + 0.5 - pen) / unit;
					let mut best = f32::MAX;
					for s in &segs {
						best = best.min(dist2(gx, gy, s));
					}
					// Coverage over about one pixel at the stroke's edge.
					let d = (sqrt(best) - radius) * unit;
					let a = saturate(0.5 - d);
					if a > 0.0 {
						let i = py * w + px as usize;
						alpha[i] = alpha[i].max((a * 255.0) as u8);
					}
				}
			}
		}
		pen += (adv + 0.6) * unit;
	}
	Strip { w, h, alpha }
}

/// The squared distance from a point to a segment.
fn dist2(x: f32, y: f32, s: &[f32; 4]) -> f32 {
	let (dx, dy) = (s[2] - s[0], s[3] - s[1]);
	let len2 = dx * dx + dy * dy;
	let t = if len2 > 0.0 {
		saturate(((x - s[0]) * dx + (y - s[1]) * dy) / len2)
	} else {
		0.0
	};
	let (ex, ey) = (x - s[0] - t * dx, y - s[1] - t * dy);
	ex * ex + ey * ey
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn draws_and_measures() {
		let s = render("Hi, ReactOS!", 32, 0.9);
		assert_eq!(s.w, measure("Hi, ReactOS!", 32));
		let inked = s.alpha.iter().filter(|&&a| a > 128).count();
		assert!(inked > 200, "{inked}");
		// The first column of the H is solid down its height.
		let col = (32.0 / 8.0 * 0.3 + 0.5) as usize;
		assert!(s.alpha[16 * s.w + col] > 128);
		assert_eq!(parse("3.25"), Some(3.25));
		assert_eq!(parse("-0.5"), Some(-0.5));
		assert_eq!(render("", 16, 0.9).alpha.len(), 16);
	}
}
