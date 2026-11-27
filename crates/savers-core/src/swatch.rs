// The Afterglow settings dialog's palette swatches: a small dusk chip per palette, a stepped sky over a slatted sun on the horizon.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Drawn from the palette tables at the button's own size, so nothing is embedded and every size and palette comes out crisp
//! (SAV-D49). The chip is a rounded rectangle: the sky in stepped bands from the ground through the glow hues (outermost at the top,
//! innermost at the horizon), the sun half-risen on the horizon in the sun hue and cut by three slats that thicken downwards, and a
//! strip of ground. The chosen chip is ringed in the sun hue (the deep hue by day) inside a hairline in the system's text colour, so it
//! shows on any dialog face. Under the chip the platform writes the palette's name in the dialog font and the system's text colour on
//! the face, as a radio button's label, with its focus rectangle round it. Every edge is antialiased by sampling each pixel 4 × 4
//! times.

use crate::colour::{b, g, mix, r};
use crate::palette::Palette;

/// Where the platform writes the name, under the chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swatch {
	pub label_left: i32,
	pub label_top: i32,
	pub label_right: i32,
	pub label_bottom: i32,
}

/// The height of the name's strip under the chip for a button `height` pixels tall: about one line of the dialog font, which grows
/// with the button, since both are laid out in dialog units.
pub fn label_height(height: usize) -> usize {
	(height * 30 / 100).clamp(11, 22).min(height / 2)
}

/// The ring takes this many pixels on each side; the tile sits inside it.
pub const RING: i32 = 3;
const SAMPLES: i32 = 4;

/// The sky's bands, top to bottom: how far each is mixed from the ground towards its glow hue, in 256ths.
const SKY: [u32; 4] = [36, 72, 116, 168];

/// The colours of one chip.
struct Look {
	ground: u32,
	sky: [u32; 4],
	sun: u32,
	sun_foot: u32,
	horizon: u32,
	keyline: u32,
	ring: u32,
}

fn look(palette: &Palette) -> Look {
	let ground = palette.ground();
	// Outermost glow hue at the top, innermost just above the horizon.
	let count = palette.glow_n.min(4);
	let mut sky = [ground; 4];
	for (band, colour) in sky.iter_mut().enumerate() {
		let hue = palette.hue(count.saturating_sub(1).saturating_sub(band * count / 4));
		*colour = mix(ground, hue, SKY[band]);
	}
	if palette.light {
		Look {
			ground,
			sky,
			sun: palette.sun,
			sun_foot: mix(palette.sun, palette.deep[1], 128),
			horizon: palette.deep[0],
			keyline: palette.n(500),
			ring: palette.deep[0],
		}
	} else {
		Look {
			ground,
			sky,
			sun: mix(palette.sun, 0xFFFFFF, 40),
			sun_foot: palette.hue(0),
			horizon: palette.hue(0),
			keyline: mix(ground, palette.n(400), 140),
			ring: palette.sun,
		}
	}
}

/// Whether a point (in pixels, with fractions) is inside a rounded rectangle.
fn inside_round(x: f32, y: f32, left: f32, top: f32, right: f32, bottom: f32, radius: f32) -> bool {
	if x < left || x >= right || y < top || y >= bottom {
		return false;
	}
	let cx = if x < left + radius {
		left + radius
	} else if x > right - radius {
		right - radius
	} else {
		return true;
	};
	let cy = if y < top + radius {
		top + radius
	} else if y > bottom - radius {
		bottom - radius
	} else {
		return true;
	};
	let (dx, dy) = (x - cx, y - cy);
	dx * dx + dy * dy <= radius * radius
}

/// Draws one swatch into `pixels` (`width` × `height`, 0x00RRGGBB): `face` is the dialog face behind it, `ink` the system's text
/// colour for the ring's hairline, `chosen` whether it is the selected palette. Returns where the name goes.
pub fn draw(pixels: &mut [u32], width: usize, height: usize, palette: &Palette, face: u32, ink: u32, chosen: bool) -> Swatch {
	let look = look(palette);
	let label = label_height(height);
	let (w, h) = (width as f32, (height - label) as f32);
	let ring = RING as f32;
	let (left, top, right, bottom) = (ring, ring, w - ring, h - ring);
	let tile_h = bottom - top;
	// A strip of ground under the horizon, a fifth of the chip.
	let horizon = bottom - (tile_h * 0.2).max(3.0);
	let sky_h = horizon - top;
	let radius = (tile_h.min(right - left) * 0.14).clamp(2.0, 5.0);
	let sun_r = (sky_h * 0.78).min((right - left) * 0.3);
	let sun_x = (left + right) * 0.5;
	// The slats through the lower half of the sun: where each starts above the horizon and how thick it is, as fractions of the
	// radius, thickest at the foot.
	let slats = [(0.08f32, 0.12f32), (0.28, 0.09), (0.45, 0.06)];

	let shade = |x: f32, y: f32| -> u32 {
		let outer = inside_round(x, y, 0.0, 0.0, w, h, radius + ring);
		let tile = inside_round(x, y, left, top, right, bottom, radius);
		if !tile {
			if chosen && outer {
				let hair = !inside_round(x, y, 1.0, 1.0, w - 1.0, h - 1.0, radius + ring - 1.0);
				return if hair { ink } else { look.ring };
			}
			return face;
		}
		// The keyline: the tile's outermost pixel.
		if !inside_round(
			x,
			y,
			left + 1.0,
			top + 1.0,
			right - 1.0,
			bottom - 1.0,
			(radius - 1.0).max(0.5),
		) {
			return look.keyline;
		}
		if y >= horizon {
			return if y < horizon + 1.0 { look.horizon } else { look.ground };
		}
		let (dx, dy) = (x - sun_x, horizon - y);
		if dx * dx + dy * dy <= sun_r * sun_r {
			let up = dy / sun_r;
			let cut = slats.iter().any(|&(at, thick)| up >= at && up < at + thick);
			let band = look.sky[((((y - top) / sky_h) * 4.0) as usize).min(3)];
			return if cut {
				mix(band, look.ground, 110)
			} else {
				mix(look.sun_foot, look.sun, ((up * 1.6).min(1.0) * 256.0) as u32)
			};
		}
		let band = (((y - top) / sky_h) * 4.0) as usize;
		look.sky[band.min(3)]
	};

	for py in 0..height - label {
		for px in 0..width {
			let (mut rs, mut gs, mut bs) = (0u32, 0u32, 0u32);
			for sy in 0..SAMPLES {
				for sx in 0..SAMPLES {
					let x = px as f32 + (sx as f32 + 0.5) / SAMPLES as f32;
					let y = py as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
					let c = shade(x, y);
					rs += r(c);
					gs += g(c);
					bs += b(c);
				}
			}
			let n = (SAMPLES * SAMPLES) as u32;
			pixels[py * width + px] = ((rs / n) << 16) | ((gs / n) << 8) | (bs / n);
		}
	}
	pixels[(height - label) * width..height * width].fill(face);
	Swatch {
		label_left: 0,
		label_top: (height - label) as i32,
		label_right: width as i32,
		label_bottom: height as i32,
	}
}

/// The short name a swatch shows: the palette's name up to its first space ("Quill" for "Quill navy and gold"); the button's window
/// text keeps the whole name for a screen reader.
pub fn short_name(name: &str) -> &str {
	name.split(' ').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::colour::luma;
	use crate::palette::PaletteId;

	#[test]
	fn every_swatch_draws_its_sun_sky_and_ground_and_rings_the_chosen_one() {
		let (width, height) = (50, 49);
		for id in PaletteId::ALL {
			for light in [false, true] {
				let palette = Palette::new(id, light);
				let mut plain = alloc::vec![0; width * height];
				let mut ringed = plain.clone();
				let label = draw(&mut plain, width, height, &palette, 0xC0C0C0, 0, false);
				draw(&mut ringed, width, height, &palette, 0xC0C0C0, 0, true);
				// The corners show the face unless chosen, and the name's strip is the face.
				assert_eq!(plain[0], 0xC0C0C0);
				assert_ne!(ringed[width + width / 2], 0xC0C0C0, "{id:?}: no ring");
				assert_eq!(plain[width + width / 2], 0xC0C0C0);
				assert_eq!((label.label_top, label.label_bottom), (35, 49));
				assert!(plain[label.label_top as usize * width..].iter().all(|&c| c == 0xC0C0C0));
				// The ground strip is the palette's ground, and the sun stands over it.
				let ground_y = label.label_top as usize - RING as usize - 2;
				assert_eq!(plain[ground_y * width + width / 2], palette.ground(), "{id:?} light {light}");
				let sun_y = ground_y - 12;
				assert_ne!(plain[sun_y * width + width / 2], plain[sun_y * width + 6], "{id:?}: no sun");
				assert!(luma(plain[sun_y * width + width / 2]) > 100, "{id:?}: a dim sun");
			}
		}
	}

	#[test]
	fn short_names() {
		assert_eq!(short_name("Quill navy and gold"), "Quill");
		assert_eq!(short_name("Dusk"), "Dusk");
	}
}
