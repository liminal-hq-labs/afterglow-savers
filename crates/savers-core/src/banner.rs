// The Afterglow settings dialog's banner: each saver's own scene at a chosen moment and crop, darkened under the title, over a stepped rule.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The banner is the scene itself, rendered once (SAV-D26, SAV-D49): nothing is embedded. A [`Shot`] names the moment, how the
//! scene is framed (the frame is rendered taller than the strip and a band of rows is kept) and how many steps lead up to it, for a
//! scene that evolves rather than being a function of time. The Collection's banner is four panels, one per member, side by side.
//! The platform supplies the scene maker, so a single-scene file links only its own scene, and draws the icon and the title over the
//! result with its own font; [`Inks`] gives the title's colours.

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use crate::colour::{luma, mix};
use crate::palette::Palette;
use crate::saver::{Saver, palette_for};
use crate::scene::Scene;
use crate::settings::{Key, SaverKind, Settings};

/// Makes a scene for settings, a seed and a render size, as the platform links it.
pub type Make<'a> = &'a dyn Fn(&Settings, u32, usize, usize) -> Box<dyn Scene>;

/// How one scene is framed in the banner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
	/// The moment, in seconds of the scene's own time.
	pub t: f32,
	/// The frame is rendered `wide` times the banner's width and `aspect` times as wide as it is tall, then cropped to the strip.
	pub wide: f32,
	pub aspect: f32,
	/// Where the strip's centre sits in that frame, from 0 (top) to 1 (bottom), and from 0 (left) to 1 (right).
	pub centre: f32,
	pub across: f32,
	/// Frames drawn 0.1 s apart up to the moment, for a scene whose picture builds up over frames (Ink's flow); 1 for a scene that
	/// is a function of time.
	pub lead: u16,
	/// The scene's seed for the banner, chosen with the moment, so the banner is the same picture whatever the saver's own seed.
	pub seed: u32,
}

/// The darkened band under the title covers the left part of the banner; the stepped rule is this many pixels tall.
pub const RULE: usize = 2;
/// The Collection's banner shows these members, left to right.
pub const COLLECTION_PANELS: [SaverKind; 4] = [SaverKind::Ink, SaverKind::Twilight, SaverKind::Cracktro, SaverKind::Maze];
/// The Collection starts each member this far into the member's own storyboard (`scenes::collection`, `CROSS` plus `LEAD`).
const COLLECTION_OFFSET: f32 = 4.5;

/// Each scene's banner moment and framing. The Collection's panels are narrower, so some are framed differently there.
pub fn shot(kind: SaverKind, in_collection: bool) -> Shot {
	// (moment, render width, aspect, centre down, centre across, lead, seed)
	let (t, wide, aspect, centre, across, lead, seed) = match (kind, in_collection) {
		// Ink folding into hooks and commas, drawn straight at the strip's own shape; its flow needs frames to build up.
		(SaverKind::Ink, _) => (17.0, 1.0, 9.0, 0.5, 0.5, 30, 2),
		// The push-in: the lit limb, the rose rim of the atmosphere and the terminator across the strip.
		(SaverKind::Twilight, false) => (46.0, 1.0, 2.4, 0.36, 0.5, 1, 1),
		(SaverKind::Twilight, true) => (46.0, 1.0, 1.2, 0.4, 0.5, 1, 1),
		// The copper bars with the stepped suns bobbing over them, above the scroller.
		(SaverKind::Cracktro, false) => (13.0, 1.0, 2.8, 0.35, 0.5, 1, 1),
		(SaverKind::Cracktro, true) => (13.0, 1.0, 1.4, 0.26, 0.5, 1, 1),
		// A corridor of crenellated walls against the dusk sky with a stepped sun window ahead.
		(SaverKind::Maze, false) => (4.5, 1.0, 2.6, 0.42, 0.5, 2, 1),
		(SaverKind::Maze, true) => (6.0, 1.0, 1.3, 0.44, 0.5, 2, 1),
		// The page writing itself, on the right, with the desk under the title.
		(SaverKind::Ghostwriter, _) => (14.0, 1.6, 3.0, 0.5, 0.3, 2, 1),
		(SaverKind::Collection, _) => (0.0, 1.0, 1.0, 0.5, 0.5, 1, 1),
	};
	Shot {
		t,
		wide,
		aspect,
		centre,
		across,
		lead,
		seed,
	}
}

/// The title's colours over the banner: the darkened band, the title, the line under it and the title's shadow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inks {
	pub band: u32,
	pub title: u32,
	pub subtitle: u32,
	pub shadow: u32,
}

/// The title's colours for a palette: the band is the palette's darkest neutral and the title its lightest, by day as by night, so
/// the title always sits light on dark.
pub fn inks(palette: &Palette) -> Inks {
	let (darkest, lightest, second) = if luma(palette.neutral[8]) < luma(palette.neutral[0]) {
		(palette.neutral[8], palette.neutral[0], palette.neutral[1])
	} else {
		(palette.neutral[0], palette.neutral[8], palette.neutral[7])
	};
	Inks {
		band: mix(darkest, 0, 96),
		title: lightest,
		subtitle: second,
		shadow: mix(darkest, 0, 160),
	}
}

/// The stepped rule along the banner's foot: the sun hue, then the glow hues outwards, in steps that narrow to the right; by day the
/// deep hues, which hold their colour against a light dialog face.
pub fn rule_colours(palette: &Palette) -> [u32; 4] {
	if palette.light {
		[palette.deep[0], palette.deep[1], palette.deep[2], palette.deep[0]]
	} else {
		[palette.sun, palette.hue(0), palette.hue(1), palette.hue(2)]
	}
}

/// The rule's steps: where each colour ends, in parts of the width out of 32.
const RULE_STEPS: [usize; 4] = [14, 23, 29, 32];

/// A finished banner, `width` × `height`, 0x00RRGGBB.
pub struct Banner {
	pub width: usize,
	pub height: usize,
	pub pixels: Vec<u32>,
	pub inks: Inks,
}

/// The settings a scene is rendered with for the banner: the saver's own palette, light mode and scene settings, but Low detail,
/// every part on, no letterbox or fades, normal speed, and a light finish.
pub fn banner_settings(settings: &Settings) -> Settings {
	let mut s = settings.clone();
	s.set(Key::Letterbox, 0);
	s.set(Key::Fades, 0);
	s.set(Key::Speed, 100);
	s.set(Key::Grain, 0);
	s.set(Key::Vignette, 10);
	// Low detail, as the preview pane draws: the banner is small, and a scene's start-up costs least at Low.
	s.set(Key::Detail, 0);
	match s.kind {
		SaverKind::Cracktro => {
			s.set(Key::Parts, 31);
			s.set(Key::Shuffle, 0);
		}
		SaverKind::Ghostwriter => s.set(Key::Acts, 63),
		SaverKind::Collection => {
			s.set(Key::Shuffle, 0);
			s.set(Key::Duration, 600);
		}
		_ => {}
	}
	s
}

/// Renders one scene's shot at `width` × `height` and returns the kept rows.
fn render_panel(settings: &Settings, seed: u32, shot: Shot, time: f32, width: usize, height: usize, make: Make) -> Vec<u32> {
	let wide = ((width as f32 * shot.wide.clamp(1.0, 3.0)) as usize).max(width);
	let tall = ((wide as f32 / shot.aspect.max(0.5)) as usize).max(height);
	let scene = make(settings, seed, wide, tall);
	let mut saver = Saver::new(settings.clone(), scene, wide, tall, true);
	let lead = u32::from(shot.lead.max(1));
	for step in 0..lead - 1 {
		saver.at((f64::from(time) - f64::from(lead - 1 - step) * 0.1).max(0.0));
	}
	let frame = saver.at(f64::from(time).max(0.0));
	// The output may be larger than asked only when the render is; it is never smaller.
	let (fw, fh) = (frame.w, frame.h);
	let centre = (shot.centre.clamp(0.0, 1.0) * fh as f32) as usize;
	let top = centre.saturating_sub(height / 2).min(fh.saturating_sub(height));
	let across = (shot.across.clamp(0.0, 1.0) * fw as f32) as usize;
	let left = across.saturating_sub(width / 2).min(fw.saturating_sub(width));
	let mut out = vec![0; width * height];
	for y in 0..height.min(fh) {
		let start = (top + y) * fw + left;
		let row = &frame.px[start..start + (fw - left).min(width)];
		out[y * width..y * width + row.len()].copy_from_slice(row);
	}
	out
}

/// Renders the banner of a saver at `width` × `height` pixels, with its settings.
pub fn render(settings: &Settings, width: usize, height: usize, make: Make) -> Banner {
	render_shot(settings, width, height, make, None)
}

/// As [`render`], with another shot in place of the scene's own (for trying moments in the preview tool); the Collection's panels
/// keep theirs.
pub fn render_shot(settings: &Settings, width: usize, height: usize, make: Make, trial: Option<Shot>) -> Banner {
	let (width, height) = (width.max(16), height.max(8));
	let base = banner_settings(settings);
	let palette = palette_for(&base);
	let inks = inks(&palette);
	let mut pixels = vec![0u32; width * height];
	let picture = height.saturating_sub(RULE).max(1);
	if base.kind == SaverKind::Collection {
		let count = COLLECTION_PANELS.len();
		for (n, &member) in COLLECTION_PANELS.iter().enumerate() {
			let left = width * n / count;
			let right = width * (n + 1) / count;
			let mut s = base.clone();
			let bit = crate::scenes::collection::MEMBERS
				.iter()
				.position(|&m| m == member)
				.unwrap_or(0);
			s.set(Key::Savers, 1 << bit);
			let shot = shot(member, true);
			let panel = render_panel(&s, shot.seed, shot, shot.t - COLLECTION_OFFSET, right - left, picture, make);
			for y in 0..picture {
				pixels[y * width + left..y * width + right].copy_from_slice(&panel[y * (right - left)..(y + 1) * (right - left)]);
			}
			// A seam in the band colour between panels.
			if n > 0 {
				for y in 0..picture {
					pixels[y * width + left] = inks.band;
				}
			}
		}
	} else {
		let shot = trial.unwrap_or_else(|| shot(base.kind, false));
		let panel = render_panel(&base, shot.seed, shot, shot.t, width, picture, make);
		pixels[..width * picture].copy_from_slice(&panel);
	}
	darken(&mut pixels, width, picture, inks.band);
	let colours = rule_colours(&palette);
	for y in picture..height {
		let mut start = 0;
		for (step, &end) in RULE_STEPS.iter().enumerate() {
			let end = width * end / 32;
			pixels[y * width + start..y * width + end].fill(colours[step]);
			start = end;
		}
	}
	Banner {
		width,
		height,
		pixels,
		inks,
	}
}

/// Darkens the picture towards the band colour under the title: 70% at the left edge, easing to nothing by 60% of the width, and a
/// little more towards the foot, where the subtitle sits.
fn darken(pixels: &mut [u32], width: usize, height: usize, band: u32) {
	let reach = (width * 3 / 5).max(1);
	for y in 0..height {
		let foot = (y * 24 / height.max(1)) as u32;
		for x in 0..reach.min(width) {
			let across = (x * 256 / reach) as u32;
			// Smooth: 1 - 3a² + 2a³ of the way across, scaled to 180 of 256 at the edge.
			let a = across * across / 256;
			let ease = 256 - (3 * a - 2 * a * across / 256);
			let amount = (ease * (180 + foot) / 256).min(256);
			let i = y * width + x;
			pixels[i] = mix(pixels[i], band, amount);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::palette::PaletteId;
	use crate::scenes::build;

	#[test]
	fn every_banner_is_drawn_at_its_size_and_is_not_flat() {
		for kind in SaverKind::ALL {
			let settings = Settings::defaults(kind);
			let make = |s: &Settings, seed: u32, w: usize, h: usize| build(kind, s, seed, w, h);
			let banner = render(&settings, 160, 24, &make);
			assert_eq!((banner.width, banner.height, banner.pixels.len()), (160, 24, 160 * 24));
			let picture = &banner.pixels[..160 * (24 - RULE)];
			let right: Vec<u32> = picture.chunks(160).flat_map(|row| row[100..].iter().copied()).collect();
			let lightest = right.iter().map(|&c| luma(c)).max().unwrap_or(0);
			let darkest = right.iter().map(|&c| luma(c)).min().unwrap_or(0);
			assert!(lightest > darkest + 24, "{kind:?}'s banner is flat ({darkest} to {lightest})");
		}
	}

	#[test]
	fn the_title_is_light_on_a_dark_band_by_night_and_by_day() {
		for id in PaletteId::ALL {
			for light in [false, true] {
				let inks = inks(&Palette::new(id, light));
				assert!(luma(inks.title) > luma(inks.band) + 150, "{id:?} light {light}");
				assert!(luma(inks.subtitle) > luma(inks.band) + 120, "{id:?} light {light}");
			}
		}
	}

	#[test]
	fn the_banner_follows_the_palette() {
		let kind = SaverKind::Cracktro;
		let make = |s: &Settings, seed: u32, w: usize, h: usize| build(kind, s, seed, w, h);
		let mut settings = Settings::defaults(kind);
		let dusk = render(&settings, 96, 16, &make);
		settings.set(Key::Palette, PaletteId::Aurora as u32);
		let aurora = render(&settings, 96, 16, &make);
		assert_ne!(dusk.pixels, aurora.pixels);
		assert_eq!(dusk.pixels[96 * 15], Palette::new(PaletteId::Dusk, false).sun);
		assert_eq!(aurora.pixels[96 * 15], Palette::new(PaletteId::Aurora, false).sun);
	}
}
