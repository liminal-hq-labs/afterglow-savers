// The runner a platform drives: time, render size, the scene, the finishing chain and the output frame.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::boxed::Box;

use crate::frame::Frame;
use crate::math::saturate;
use crate::palette::{Palette, PaletteId};
use crate::post::{FinishParams, Finisher};
use crate::scene::{Ctx, Scene};
use crate::settings::{Detail, Key, Settings};

/// The render size for a window: the render scale of each side, at least 16 × 9.
pub fn render_size(settings: &Settings, win_w: usize, win_h: usize, preview: bool) -> (usize, usize) {
	if preview {
		return (win_w.max(16), win_h.max(9));
	}
	let s = settings.get(Key::RenderScale) as usize;
	((win_w * s / 100).max(16), (win_h * s / 100).max(9))
}

/// The output size: the window, but no wider than the detail level's cap and no smaller than the render size, so a large
/// screen does not cost more than the detail asks; `StretchDIBits` enlarges the rest.
pub fn output_size(detail: Detail, win: (usize, usize), render: (usize, usize)) -> (usize, usize) {
	let cap = match detail {
		Detail::Low => 640,
		Detail::Medium => 1024,
		Detail::High => 1600,
		Detail::Ultra => 4096,
	};
	if win.0 <= cap || win.0 <= render.0 {
		return if render.0 > win.0 { render } else { win };
	}
	let w = cap.max(render.0);
	(w, (win.1 * w / win.0).max(render.1))
}

/// The palette a saver's settings ask for.
pub fn palette_for(settings: &Settings) -> Palette {
	let light = settings.get(Key::Light) == 1 && settings.kind.has_light_mode();
	Palette::new(PaletteId::from_u32(settings.get(Key::Palette)), light)
}

/// One running saver.
pub struct Saver {
	pub settings: Settings,
	scene: Box<dyn Scene>,
	fin: Finisher,
	render: Frame,
	pal: Palette,
	win: (usize, usize),
	preview: bool,
	time: f64,
	wall: f64,
	frame_no: u32,
}

impl Saver {
	/// `scene` must have been made for `render_size(&settings, win_w, win_h, preview)`.
	pub fn new(settings: Settings, scene: Box<dyn Scene>, win_w: usize, win_h: usize, preview: bool) -> Saver {
		let (rw, rh) = render_size(&settings, win_w, win_h, preview);
		let pal = palette_for(&settings);
		Saver {
			settings,
			scene,
			fin: Finisher::new(),
			render: Frame::new(rw, rh),
			pal,
			win: (win_w.max(1), win_h.max(1)),
			preview,
			time: 0.0,
			wall: 0.0,
			frame_no: 0,
		}
	}

	pub fn detail(&self) -> Detail {
		if self.preview { Detail::Low } else { self.settings.detail() }
	}

	/// Advances by `dt` wall-clock seconds and draws the next frame at the output size.
	pub fn step(&mut self, dt: f32) -> &Frame {
		let dt = if self.frame_no == 0 { 0.0 } else { dt.clamp(0.0, 0.1) };
		let sdt = dt * self.settings.speed();
		self.time += sdt as f64;
		self.wall += dt as f64;
		self.draw(sdt)
	}

	/// Draws the frame for scene time `t`, as the preview tool and the tests do. Frames must be asked for in increasing time.
	pub fn at(&mut self, t: f64) -> &Frame {
		let dt = if self.frame_no == 0 {
			0.0
		} else {
			(t - self.time).clamp(0.0, 0.1) as f32
		};
		self.time = t;
		self.wall = t / self.settings.speed() as f64;
		self.draw(dt)
	}

	fn draw(&mut self, dt: f32) -> &Frame {
		let detail = self.detail();
		let cx = Ctx {
			t: self.time,
			dt,
			detail,
			pal: &self.pal,
			frame_no: self.frame_no,
		};
		self.scene.render(&cx, &mut self.render);
		let hints = self.scene.hints(&cx);
		let s = &self.settings;
		let fades = s.get(Key::Fades) == 1;
		let mut fade = saturate(self.wall as f32);
		if fades {
			fade *= saturate(hints.fade);
		}
		let wide = self.win.0 as f32 / self.win.1 as f32;
		let p = FinishParams {
			bloom: ((s.get(Key::Bloom) as f32 * hints.bloom_gain) as u32).min(100),
			streaks: if detail == Detail::Low {
				s.get(Key::Streaks) / 2
			} else {
				s.get(Key::Streaks)
			},
			vignette: s.get(Key::Vignette),
			grain: if self.preview { 0 } else { s.get(Key::Grain) },
			letterbox: if s.get(Key::Letterbox) == 1 && !self.preview {
				if wide > 1.5 { 2.39 } else { 1.85 }
			} else {
				0.0
			},
			fade,
			threshold: hints.threshold,
			streak_tint: hints.streak_tint,
			coarse: detail == Detail::Low,
		};
		self.fin.glow(&mut self.render, &p);
		let (ow, oh) = output_size(detail, self.win, (self.render.w, self.render.h));
		self.frame_no = self.frame_no.wrapping_add(1);
		self.fin.output(&self.render, ow, oh, &p, self.frame_no)
	}

	/// Scene time in seconds.
	pub fn time(&self) -> f64 {
		self.time
	}
}
