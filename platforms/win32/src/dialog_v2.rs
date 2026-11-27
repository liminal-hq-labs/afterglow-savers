// The Afterglow settings dialog (SAV-D26, SAV-D39): a banner that is the saver's own scene, clickable palette swatches, a live preview
// that runs the real scene at Low detail, and group titles ruled in the palette's first glow hue. Every control is a real window, so the
// keyboard and screen readers see them; the reading and writing of values is the plain dialog's, over the same identifiers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::cell::{Cell, RefCell};

use savers_core::banner::{self, Banner};
use savers_core::palette::{Palette, PaletteId};
use savers_core::saver::{Saver, palette_for, render_size};
use savers_core::settings::{Control, Key, SaverKind, Settings, SettingsStore, fields};
use savers_core::swatch;

use crate::ansi::{number_text, to_ansi};
use crate::clock::Clock;
use crate::dialog::{collect, item, prepare, show};
use crate::registry::RegStore;
use crate::saver::{MakeScene, seed_for, stretch};
use crate::sys::{self, Handle};
use crate::template::{CANCEL_ID, DEFAULTS_ID, OK_ID, section};
use crate::template_v2::{self, BANNER_ID, HEADER_BASE, HEADERS, NEW_SEED_ID, PREVIEW_ID, swatch_id};

/// `DrawText` flags for the swatch names (kept here, beside their one use).
const DT_NOCLIP: u32 = 0x0100;
const DT_CALCRECT: u32 = 0x0400;

/// What the dialog returns when it could not set itself up, so the plain dialog takes over.
const FALLBACK: isize = 99;
const TIMER_ID: usize = 1;
/// The preview's frame interval, and the slower one it drops to when a frame takes longer than `SLOW_FRAME` seconds.
const FRAME_MS: u32 = 50;
const SLOW_MS: u32 = 100;
const SLOW_FRAME: f64 = 0.15;

/// The scene running in the preview pane, and the settings it was last given.
struct Preview {
	saver: Saver,
	current: Settings,
	width: usize,
	height: usize,
}

struct State {
	kind: SaverKind,
	make: MakeScene,
	settings: RefCell<Settings>,
	preview: RefCell<Option<Preview>>,
	/// The banner's picture (`savers_core::banner`), and whether it is out of date: it is rendered on the timer after the dialog
	/// shows and after a change of palette or light mode, so the dialog never waits for it.
	banner: RefCell<Option<Banner>>,
	banner_stale: Cell<bool>,
	clock: Clock,
	last: Cell<f64>,
	/// A seed for a preview whose setting is 0 ("a new one each time"), chosen once so that the preview does not change on every edit.
	fresh_seed: u32,
	dirty: Cell<bool>,
	active: Cell<bool>,
	slow: Cell<bool>,
	light: Cell<bool>,
	/// The palette field's place in the table and its choices.
	palette_index: usize,
	palette_options: &'static [(&'static str, u32)],
	seed_id: u16,
	seed_max: u32,
	icon: Handle,
	/// The system's button procedure, which the swatches' own one calls.
	button_proc: Cell<isize>,
	bold: Cell<Handle>,
	title: Cell<Handle>,
}

fn colour(rgb: u32) -> u32 {
	((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF)
}

fn fill(dc: Handle, left: i32, top: i32, right: i32, bottom: i32, colorref: u32) {
	let rect = sys::Rect {
		left,
		top,
		right,
		bottom,
	};
	// SAFETY: a live context and a brush made and freed here.
	unsafe {
		let brush = sys::CreateSolidBrush(colorref);
		sys::FillRect(dc, &rect, brush);
		sys::DeleteObject(brush);
	}
}

fn draw_text(dc: Handle, text: &str, rect: &sys::Rect, format: u32, colorref: u32, font: Handle) {
	let text = to_ansi(text);
	let mut rect = sys::Rect {
		left: rect.left,
		top: rect.top,
		right: rect.right,
		bottom: rect.bottom,
	};
	// SAFETY: the text ends in a NUL and the rectangle is ours.
	unsafe {
		let old = if font.is_null() {
			sys::NULL
		} else {
			sys::SelectObject(dc, font)
		};
		sys::SetBkMode(dc, sys::TRANSPARENT);
		sys::SetTextColor(dc, colorref);
		sys::DrawTextA(dc, text.as_ptr(), -1, &mut rect, format | sys::DT_NOPREFIX);
		if !old.is_null() {
			sys::SelectObject(dc, old);
		}
	}
}

/// A copy of a font in another weight and scale, or null.
fn derived_font(base: Handle, weight: i32, percent: i32) -> Handle {
	if base.is_null() {
		return sys::NULL;
	}
	let mut face: sys::LogFont = sys::LogFont {
		height: 0,
		width: 0,
		escapement: 0,
		orientation: 0,
		weight: 0,
		italic: 0,
		underline: 0,
		strike_out: 0,
		char_set: 0,
		out_precision: 0,
		clip_precision: 0,
		quality: 0,
		pitch_and_family: 0,
		face: [0; 32],
	};
	// SAFETY: the buffer is as large as the size says.
	unsafe {
		let size = core::mem::size_of::<sys::LogFont>() as i32;
		if sys::GetObjectA(base, size, &mut face as *mut sys::LogFont as *mut core::ffi::c_void) == 0 {
			return sys::NULL;
		}
		face.weight = weight;
		face.width = 0;
		face.height = face.height * percent / 100;
		sys::CreateFontIndirectA(&face)
	}
}

// ---- Palette swatches ----

fn swatch_window(dialog: Handle, index: usize, n: usize) -> Handle {
	item(dialog, swatch_id(index, n))
}

/// Marks swatch `which` of the palette field at `index` as the chosen one: the others are cleared, and only the chosen one is a tab
/// stop. Returns false when the dialog has no swatches (it is the plain one).
pub fn select_swatch(dialog: Handle, index: usize, count: usize, which: usize) -> bool {
	if swatch_window(dialog, index, 0).is_null() {
		return false;
	}
	for n in 0..count {
		let window = swatch_window(dialog, index, n);
		if window.is_null() {
			continue;
		}
		// SAFETY: the window is ours; the style and the user data are plain integers.
		unsafe {
			sys::SetWindowLongPtr(window, sys::GWL_USERDATA, isize::from(n == which));
			let style = sys::GetWindowLongPtr(window, sys::GWL_STYLE) as u32;
			let style = if n == which {
				style | sys::WS_TABSTOP
			} else {
				style & !sys::WS_TABSTOP
			};
			sys::SetWindowLongPtr(window, sys::GWL_STYLE, style as isize);
			sys::InvalidateRect(window, core::ptr::null(), 0);
		}
	}
	true
}

/// The chosen swatch's number, or `None` when the dialog has no swatches.
pub fn chosen_swatch(dialog: Handle, index: usize, count: usize) -> Option<usize> {
	if swatch_window(dialog, index, 0).is_null() {
		return None;
	}
	let chosen = (0..count).find(|&n| {
		let window = swatch_window(dialog, index, n);
		// SAFETY: a plain integer in the window's user data.
		!window.is_null() && unsafe { sys::GetWindowLongPtr(window, sys::GWL_USERDATA) } == 1
	});
	Some(chosen.unwrap_or(0))
}

/// The window procedure the swatches share, over the system's button procedure (kept in the dialog's state, since the image has no
/// writable data section): the arrow keys move between them and choose, as radio buttons do, so the group is one tab stop.
unsafe extern "system" fn swatch_proc(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize {
	// SAFETY: plain window calls on controls of the dialog that owns this one, whose user data is its `State`.
	unsafe {
		let state = sys::GetWindowLongPtr(sys::GetParent(window), sys::GWL_USERDATA) as *const State;
		if state.is_null() {
			return sys::DefWindowProcA(window, message, wparam, lparam);
		}
		let original = (*state).button_proc.get();
		if message == sys::WM_GETDLGCODE {
			return sys::CallWindowProcA(original, window, message, wparam, lparam) | sys::DLGC_WANTARROWS;
		}
		if message == sys::WM_KEYDOWN {
			let step: i32 = match wparam {
				sys::VK_LEFT | sys::VK_UP => -1,
				sys::VK_RIGHT | sys::VK_DOWN => 1,
				_ => 0,
			};
			if step != 0 {
				let parent = sys::GetParent(window);
				let id = sys::GetDlgCtrlID(window);
				let mut target = id + step;
				if sys::GetDlgItem(parent, target).is_null() {
					// Past either end: wrap to the other.
					target = id;
					while !sys::GetDlgItem(parent, target - step).is_null() {
						target -= step;
					}
				}
				let next = sys::GetDlgItem(parent, target);
				if !next.is_null() && target != id {
					sys::SetFocus(next);
					sys::SendMessageA(
						parent,
						sys::WM_COMMAND,
						target as usize | (sys::BN_CLICKED << 16),
						next as isize,
					);
				}
				return 0;
			}
		}
		sys::CallWindowProcA(original, window, message, wparam, lparam)
	}
}

// ---- Drawing ----

fn draw_swatch(state: &State, item: &sys::DrawItemStruct) {
	let dc = item.dc;
	let rect = &item.rect;
	let id = item.control_id as u16;
	let first = swatch_id(state.palette_index, 0);
	let Some(&(name, value)) = state.palette_options.get(usize::from(id.wrapping_sub(first))) else {
		return;
	};
	let palette = Palette::new(PaletteId::from_u32(value), state.light.get());
	// SAFETY: a plain integer in the window's user data.
	let chosen = unsafe { sys::GetWindowLongPtr(item.window, sys::GWL_USERDATA) } == 1;
	// SAFETY: plain system colour queries.
	let (face, ink) = unsafe { (sys::GetSysColor(sys::COLOR_BTNFACE), sys::GetSysColor(sys::COLOR_BTNTEXT)) };
	let (width, height) = ((rect.right - rect.left).max(1), (rect.bottom - rect.top).max(1));
	let mut pixels = alloc::vec![0u32; (width * height) as usize];
	let label = swatch::draw(
		&mut pixels,
		width as usize,
		height as usize,
		&palette,
		colour(face),
		colour(ink),
		chosen,
	);
	stretch(dc, width, height, width as usize, height as usize, &pixels);
	let label_rect = sys::Rect {
		left: rect.left + label.label_left,
		top: rect.top + label.label_top,
		right: rect.left + label.label_right,
		bottom: rect.top + label.label_bottom,
	};
	// SAFETY: a plain message to the dialog for its own font.
	let font = unsafe { sys::SendMessageA(sys::GetParent(item.window), sys::WM_GETFONT, 0, 0) } as Handle;
	let text = swatch::short_name(name);
	// The name may be a pixel or two wider than a narrow swatch; it is centred and left unclipped, so only the button's edge trims it.
	draw_text(
		dc,
		text,
		&label_rect,
		sys::DT_CENTER | sys::DT_VCENTER | sys::DT_SINGLELINE | DT_NOCLIP,
		ink,
		font,
	);
	if item.item_state & sys::ODS_FOCUS != 0 {
		// The name is in the system's text colour on the face, and the focus rectangle goes round it, as round a radio button's label.
		let ansi = to_ansi(text);
		let mut extent = sys::Rect {
			left: 0,
			top: 0,
			right: 0,
			bottom: 0,
		};
		// SAFETY: a live context, the dialog's font selected only for the measurement, and a text that ends in a NUL.
		unsafe {
			let old = if font.is_null() {
				sys::NULL
			} else {
				sys::SelectObject(dc, font)
			};
			sys::DrawTextA(
				dc,
				ansi.as_ptr(),
				-1,
				&mut extent,
				DT_CALCRECT | sys::DT_SINGLELINE | sys::DT_NOPREFIX,
			);
			if !old.is_null() {
				sys::SelectObject(dc, old);
			}
		}
		let half = ((extent.right - extent.left) / 2 + 2).min((label_rect.right - label_rect.left) / 2);
		let middle = (label_rect.left + label_rect.right) / 2;
		let focus = sys::Rect {
			left: middle - half,
			top: label_rect.top,
			right: middle + half,
			bottom: label_rect.bottom,
		};
		// SAFETY: a live context.
		unsafe { sys::DrawFocusRect(dc, &focus) };
	}
}

/// The palette the dialog shows now: the preview's, which follows the swatches and light mode.
fn palette_now(state: &State) -> Palette {
	match state.preview.borrow().as_ref() {
		Some(preview) => palette_for(&preview.current),
		None => palette_for(&state.settings.borrow()),
	}
}

fn draw_header(state: &State, item: &sys::DrawItemStruct, number: usize) {
	let dc = item.dc;
	let rect = &item.rect;
	// SAFETY: plain system colour queries.
	let (face, ink) = unsafe { (sys::GetSysColor(sys::COLOR_BTNFACE), sys::GetSysColor(sys::COLOR_BTNTEXT)) };
	fill(dc, rect.left, rect.top, rect.right, rect.bottom, face);
	draw_text(
		dc,
		HEADERS[number],
		&sys::Rect {
			left: rect.left,
			top: rect.top,
			right: rect.right,
			bottom: rect.bottom - 3,
		},
		sys::DT_LEFT | sys::DT_BOTTOM | sys::DT_SINGLELINE,
		ink,
		state.bold.get(),
	);
	// The rule under it, in the palette's first glow hue (the deeper hue by day, where a light hue would vanish on the face).
	let palette = palette_now(state);
	let hue = if palette.light { palette.deep[0] } else { palette.glow[0] };
	fill(dc, rect.left, rect.bottom - 2, rect.right, rect.bottom, colour(hue));
}

fn draw_banner(state: &State, item: &sys::DrawItemStruct) {
	let dc = item.dc;
	let rect = &item.rect;
	let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
	let inks = banner::inks(&palette_now(state));
	match state.banner.borrow().as_ref() {
		Some(banner) => stretch(dc, width, height, banner.width, banner.height, &banner.pixels),
		None => fill(dc, rect.left, rect.top, rect.right, rect.bottom, colour(inks.band)),
	}
	let text_left = rect.left + 12 + 32 + 10;
	// SAFETY: a live context and the saver's own icon (or null, which draws nothing).
	unsafe {
		if !state.icon.is_null() {
			sys::DrawIcon(dc, rect.left + 12, rect.top + (height - 32) / 2, state.icon);
		}
	}
	let split = rect.top + height * 62 / 100;
	let title = state.kind.title();
	let shadow = |offset: i32| sys::Rect {
		left: text_left + offset,
		top: rect.top + offset,
		right: rect.right,
		bottom: split + offset,
	};
	draw_text(
		dc,
		title,
		&shadow(1),
		sys::DT_LEFT | sys::DT_BOTTOM | sys::DT_SINGLELINE,
		colour(inks.shadow),
		state.title.get(),
	);
	draw_text(
		dc,
		title,
		&shadow(0),
		sys::DT_LEFT | sys::DT_BOTTOM | sys::DT_SINGLELINE,
		colour(inks.title),
		state.title.get(),
	);
	draw_text(
		dc,
		"Settings",
		&sys::Rect {
			left: text_left,
			top: split + 1,
			right: rect.right,
			bottom: rect.bottom,
		},
		sys::DT_LEFT | sys::DT_TOP | sys::DT_SINGLELINE,
		colour(inks.subtitle),
		// SAFETY: the dialog's own font.
		unsafe { sys::SendMessageA(sys::GetParent(item.window), sys::WM_GETFONT, 0, 0) } as Handle,
	);
}

fn draw_item(state: &State, item: &sys::DrawItemStruct) {
	let id = item.control_id as u16;
	let first = swatch_id(state.palette_index, 0);
	if id == BANNER_ID {
		draw_banner(state, item);
	} else if id == PREVIEW_ID {
		// The next tick paints the frame; until then, black.
		fill(item.dc, item.rect.left, item.rect.top, item.rect.right, item.rect.bottom, 0);
	} else if (HEADER_BASE..HEADER_BASE + HEADERS.len() as u16).contains(&id) {
		draw_header(state, item, usize::from(id - HEADER_BASE));
	} else if id >= first && usize::from(id - first) < state.palette_options.len() {
		draw_swatch(state, item);
	}
}

// ---- The scene ----

fn seed_of(state: &State, settings: &Settings) -> u32 {
	match settings.get(Key::Seed) {
		0 => state.fresh_seed,
		set => set,
	}
}

/// The size of a control's client area.
fn client_size(window: Handle) -> (usize, usize) {
	let mut rect = sys::Rect {
		left: 0,
		top: 0,
		right: 0,
		bottom: 0,
	};
	// SAFETY: a valid out pointer.
	unsafe { sys::GetClientRect(window, &mut rect) };
	(
		(rect.right - rect.left).max(0) as usize,
		(rect.bottom - rect.top).max(0) as usize,
	)
}

fn make_saver(state: &State, settings: &Settings, width: usize, height: usize) -> Saver {
	let (render_width, render_height) = render_size(settings, width, height, true);
	let scene = (state.make)(settings, seed_of(state, settings), render_width, render_height);
	Saver::new(settings.clone(), scene, width, height, true)
}

/// Renders the banner for the settings the preview has now, and repaints it.
fn render_banner(dialog: Handle, state: &State) {
	let window = item(dialog, BANNER_ID);
	let (width, height) = client_size(window);
	let settings = match state.preview.borrow().as_ref() {
		Some(preview) => preview.current.clone(),
		None => state.settings.borrow().clone(),
	};
	let make = |s: &Settings, seed: u32, w: usize, h: usize| (state.make)(s, seed, w, h);
	*state.banner.borrow_mut() = Some(banner::render(&settings, width, height, &make));
	// SAFETY: a plain repaint request for a control of the dialog.
	unsafe { sys::InvalidateRect(window, core::ptr::null(), 0) };
}

/// Applies what the controls say now: a palette, light, seed or scene setting rebuilds the scene; the rest reaches the running one.
fn apply_changes(dialog: Handle, state: &State) {
	let mut preview = state.preview.borrow_mut();
	let Some(preview) = preview.as_mut() else {
		return;
	};
	let mut next = preview.current.clone();
	collect(dialog, &mut next);
	let changed = |key: Key| next.get(key) != preview.current.get(key);
	let scene_changed = fields(state.kind).any(|field| section(field.key) == 3 && changed(field.key));
	let look_changed = changed(Key::Palette) || changed(Key::Light);
	let rebuild = next.text != preview.current.text || scene_changed || look_changed || changed(Key::Seed);
	if rebuild {
		preview.saver = make_saver(state, &next, preview.width, preview.height);
	} else {
		preview.saver.settings = next.clone();
	}
	preview.current = next;
	// The banner has its own seed, so only the look and the scene's own settings change it.
	if look_changed || scene_changed {
		state.banner_stale.set(true);
	}
	if look_changed {
		state.light.set(palette_for(&preview.current).light);
		for n in 0..state.palette_options.len() {
			// SAFETY: a plain repaint request for a control of the dialog.
			unsafe { sys::InvalidateRect(swatch_window(dialog, state.palette_index, n), core::ptr::null(), 0) };
		}
		for number in 0..HEADERS.len() {
			// SAFETY: as above.
			unsafe { sys::InvalidateRect(item(dialog, HEADER_BASE + number as u16), core::ptr::null(), 0) };
		}
	}
}

/// One timer tick: takes up edits, then draws the next preview frame unless the dialog is inactive.
fn tick(dialog: Handle, state: &State) {
	if state.dirty.replace(false) {
		apply_changes(dialog, state);
	}
	if state.banner_stale.replace(false) {
		render_banner(dialog, state);
		// The banner took this tick; the clock restarts so the preview does not jump.
		state.last.set(state.clock.seconds());
		return;
	}
	let now = state.clock.seconds();
	let elapsed = now - state.last.replace(now);
	if !state.active.get() {
		return;
	}
	let mut preview = state.preview.borrow_mut();
	let Some(preview) = preview.as_mut() else {
		return;
	};
	let frame = preview.saver.step(elapsed as f32);
	let pane = item(dialog, PREVIEW_ID);
	// SAFETY: the pane is ours; the context is released here.
	unsafe {
		let dc = sys::GetDC(pane);
		if !dc.is_null() {
			stretch(dc, preview.width as i32, preview.height as i32, frame.w, frame.h, &frame.px);
			sys::ReleaseDC(pane, dc);
		}
	}
	let took = state.clock.seconds() - now;
	let slow = took > SLOW_FRAME;
	if slow != state.slow.get() && (slow || took < SLOW_FRAME / 3.0) {
		state.slow.set(slow);
		// SAFETY: re-arms this dialog's own timer.
		unsafe { sys::SetTimer(dialog, TIMER_ID, if slow { SLOW_MS } else { FRAME_MS }, core::ptr::null()) };
	}
}

/// Builds the banner and the preview, subclasses the swatches and starts the timer. False when the dialog cannot be had.
fn set_up(dialog: Handle, state: &State) -> bool {
	let (banner_width, banner_height) = client_size(item(dialog, BANNER_ID));
	let (width, height) = client_size(item(dialog, PREVIEW_ID));
	if banner_width < 16 || banner_height < 9 || width < 16 || height < 9 {
		return false;
	}
	// SAFETY: the dialog's own font and a copy of it.
	let base = unsafe { sys::SendMessageA(dialog, sys::WM_GETFONT, 0, 0) } as Handle;
	state.bold.set(derived_font(base, sys::FW_BOLD, 100));
	state.title.set(derived_font(base, sys::FW_BOLD, 170));
	state.banner_stale.set(true);
	let settings = state.settings.borrow().clone();
	state.light.set(palette_for(&settings).light);
	*state.preview.borrow_mut() = Some(Preview {
		saver: make_saver(state, &settings, width, height),
		current: settings,
		width,
		height,
	});
	for n in 0..state.palette_options.len() {
		let window = swatch_window(dialog, state.palette_index, n);
		if window.is_null() {
			return false;
		}
		// SAFETY: a control of this dialog; every swatch shares the button class, so the first one's procedure is the original.
		unsafe {
			let previous = sys::GetWindowLongPtr(window, sys::GWL_WNDPROC);
			if previous != swatch_proc as *const () as isize {
				state.button_proc.set(previous);
				sys::SetWindowLongPtr(window, sys::GWL_WNDPROC, swatch_proc as *const () as isize);
			}
		}
	}
	state.last.set(state.clock.seconds());
	// SAFETY: the timer belongs to this dialog and ends with it.
	unsafe { sys::SetTimer(dialog, TIMER_ID, FRAME_MS, core::ptr::null()) };
	true
}

fn new_seed(dialog: Handle, state: &State) {
	let ticks = (state.clock.seconds() * 1000.0) as u32;
	let value = ticks.wrapping_mul(0x9E37_79B1) % state.seed_max.max(2) + 1;
	// SAFETY: the text ends in a NUL.
	unsafe { sys::SetDlgItemTextA(dialog, i32::from(state.seed_id), number_text(value).as_ptr()) };
}

unsafe extern "system" fn dialog_proc(dialog: Handle, message: u32, wparam: usize, lparam: isize) -> isize {
	// SAFETY: the pointer in the user data is the `State` that `show_settings` keeps alive for the whole dialog; the state is only
	// ever borrowed shared, and what changes in it sits in cells.
	unsafe {
		if message == sys::WM_INITDIALOG {
			sys::SetWindowLongPtr(dialog, sys::GWL_USERDATA, lparam);
			let state = &*(lparam as *const State);
			if !state.icon.is_null() {
				sys::SendMessageA(dialog, sys::WM_SETICON, 1, state.icon as isize);
				sys::SendMessageA(dialog, sys::WM_SETICON, 0, state.icon as isize);
			}
			prepare(dialog, state.kind);
			show(dialog, &state.settings.borrow());
			if !set_up(dialog, state) {
				sys::EndDialog(dialog, FALLBACK);
				return 1;
			}
			// Focus the chosen swatch rather than the first control.
			let chosen = chosen_swatch(dialog, state.palette_index, state.palette_options.len()).unwrap_or(0);
			sys::SetFocus(swatch_window(dialog, state.palette_index, chosen));
			state.dirty.set(false);
			return 0;
		}
		let state = sys::GetWindowLongPtr(dialog, sys::GWL_USERDATA) as *const State;
		if state.is_null() {
			return 0;
		}
		let state = &*state;
		match message {
			sys::WM_DRAWITEM => {
				draw_item(state, &*(lparam as *const sys::DrawItemStruct));
				1
			}
			sys::WM_TIMER => {
				tick(dialog, state);
				0
			}
			sys::WM_ACTIVATE => {
				state.active.set(wparam & 0xFFFF != 0);
				0
			}
			sys::WM_HSCROLL => {
				state.dirty.set(true);
				0
			}
			sys::WM_COMMAND => {
				let id = (wparam & 0xFFFF) as u16;
				let first = swatch_id(state.palette_index, 0);
				match id {
					OK_ID => {
						let mut settings = state.settings.borrow_mut();
						collect(dialog, &mut settings);
						let mut store = RegStore::create(state.kind);
						settings.save(&mut store);
						sys::EndDialog(dialog, 1);
					}
					CANCEL_ID => {
						sys::EndDialog(dialog, 0);
					}
					DEFAULTS_ID => {
						show(dialog, &Settings::defaults(state.kind));
						state.dirty.set(true);
					}
					NEW_SEED_ID => {
						new_seed(dialog, state);
						state.dirty.set(true);
					}
					id if id >= first && usize::from(id - first) < state.palette_options.len() => {
						select_swatch(
							dialog,
							state.palette_index,
							state.palette_options.len(),
							usize::from(id - first),
						);
						state.dirty.set(true);
					}
					_ => state.dirty.set(true),
				}
				0
			}
			_ => 0,
		}
	}
}

/// Whether the Afterglow dialog should be offered at all: not under high contrast, and not on a display of 256 colours or fewer,
/// where its colours would be dithered; not on a screen too small for it; and not when the saver's key asks for the plain one (`PlainDialog` set to 1).
pub fn wanted(kind: SaverKind) -> bool {
	if RegStore::open(kind).get_dword("PlainDialog") == Some(1) {
		return false;
	}
	let mut contrast = sys::HighContrast {
		size: core::mem::size_of::<sys::HighContrast>() as u32,
		flags: 0,
		default_scheme: core::ptr::null_mut(),
	};
	// SAFETY: the structure is as large as its size field says.
	let high = unsafe {
		sys::SystemParametersInfoA(
			sys::SPI_GETHIGHCONTRAST,
			contrast.size,
			&mut contrast as *mut sys::HighContrast as *mut core::ffi::c_void,
			0,
		) != 0 && contrast.flags & sys::HCF_HIGHCONTRASTON != 0
	};
	if high {
		return false;
	}
	// SAFETY: a screen context, released at once.
	let depth = unsafe {
		let dc = sys::GetDC(sys::NULL);
		let depth = if dc.is_null() {
			32
		} else {
			sys::GetDeviceCaps(dc, sys::BITSPIXEL) * sys::GetDeviceCaps(dc, sys::PLANES)
		};
		sys::ReleaseDC(sys::NULL, dc);
		depth
	};
	if depth <= 8 {
		return false;
	}
	// The dialog must fit the screen with a taskbar's room to spare, whatever the font size; a smaller screen gets the plain dialog,
	// which is shorter.
	let (width, height) = template_v2::size(kind);
	// SAFETY: plain metric queries.
	let (screen_width, screen_height, chrome) = unsafe {
		(
			sys::GetSystemMetrics(sys::SM_CXSCREEN),
			sys::GetSystemMetrics(sys::SM_CYSCREEN),
			sys::GetSystemMetrics(sys::SM_CYCAPTION) + 2 * sys::GetSystemMetrics(sys::SM_CYDLGFRAME),
		)
	};
	let needed_height = (i32::from(height) * 13 / 8 + chrome) * 11 / 10;
	let needed_width = i32::from(width) * 3 / 2 * 11 / 10;
	needed_height + 32 <= screen_height && needed_width <= screen_width
}

/// Runs the Afterglow dialog to its end. Returns false when it could not be made, so that the caller shows the plain one.
pub fn show_afterglow(kind: SaverKind, owner: Handle, make: MakeScene) -> bool {
	let Some((palette_index, palette_field)) = fields(kind).enumerate().find(|(_, f)| f.key == Key::Palette) else {
		return false;
	};
	let Control::Choice(palette_options) = palette_field.control else {
		return false;
	};
	let Some((seed_index, seed_field)) = fields(kind).enumerate().find(|(_, f)| f.key == Key::Seed) else {
		return false;
	};
	let settings = Settings::load(kind, &RegStore::open(kind));
	// SAFETY: the instance is ours; a missing icon gives null.
	let icon = unsafe { sys::LoadIconA(sys::GetModuleHandleA(core::ptr::null()), sys::ICON_ID as *const u8) };
	let state = State {
		kind,
		make,
		fresh_seed: seed_for(&Settings::defaults(kind)),
		settings: RefCell::new(settings),
		preview: RefCell::new(None),
		banner: RefCell::new(None),
		banner_stale: Cell::new(false),
		clock: Clock::new(),
		last: Cell::new(0.0),
		dirty: Cell::new(false),
		active: Cell::new(true),
		slow: Cell::new(false),
		light: Cell::new(false),
		palette_index,
		palette_options,
		seed_id: crate::template::field_id(seed_index),
		seed_max: seed_field.max,
		icon,
		button_proc: Cell::new(0),
		bold: Cell::new(sys::NULL),
		title: Cell::new(sys::NULL),
	};
	let template = template_v2::build(kind);
	// SAFETY: the template is a complete `DLGTEMPLATE` that outlives the call, and the state outlives the dialog.
	let result = unsafe {
		sys::InitCommonControls();
		let instance = sys::GetModuleHandleA(core::ptr::null());
		sys::DialogBoxIndirectParamA(
			instance,
			template.bytes.as_ptr(),
			owner,
			Some(dialog_proc),
			&state as *const State as isize,
		)
	};
	for font in [state.bold.get(), state.title.get()] {
		if !font.is_null() {
			// SAFETY: fonts made by this dialog, no longer selected anywhere.
			unsafe { sys::DeleteObject(font) };
		}
	}
	result != FALLBACK && result != -1
}
