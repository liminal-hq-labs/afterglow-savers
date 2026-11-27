// The saver itself: the full-screen and preview windows, their exit rules, the frame loop and the blit (SPEC sections 2 to 5).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::boxed::Box;
use core::cell::Cell;
use core::ffi::c_void;

use savers_core::frame::Frame;
use savers_core::saver::{Saver, render_size};
use savers_core::scene::Scene;
use savers_core::settings::{Key, SaverKind, Settings};

use crate::clock::Clock;
use crate::cmdline::{self, Mode};
use crate::dialog;
use crate::registry::RegStore;
use crate::sys::{self, Handle};

/// Makes a saver's scene for a render size: the scene constructor, or `savers_core::scenes::build` for the Collection.
pub type MakeScene = fn(&Settings, u32, usize, usize) -> Box<dyn Scene>;

const CLASS_NAME: &[u8] = b"WindowsScreenSaverClass\0";
/// How far the mouse may drift from where the first move found it before the saver ends.
const MOUSE_THRESHOLD: i32 = 10;
/// The preview's frame cap and the size it falls back to when its parent has no size yet.
const PREVIEW_FPS: u32 = 20;
const PREVIEW_FALLBACK: (i32, i32) = (152, 112);

/// What the window procedure needs; the frame loop owns the rest.
struct Shared {
	preview: bool,
	first_move: Cell<Option<(i32, i32)>>,
	/// Whether the window has been active yet: losing activation ends the saver, but a window that was never active (the system sends
	/// a deactivation while it is still being shown, when another window is leaving) has not lost anything.
	was_active: Cell<bool>,
}

fn close(window: Handle) {
	// SAFETY: posting to a window is always allowed.
	unsafe { sys::PostMessageA(window, sys::WM_CLOSE, 0, 0) };
}

unsafe extern "system" fn window_proc(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize {
	// SAFETY: the user data is the `Shared` that `run_window` keeps alive until the window is gone.
	unsafe {
		if message == sys::WM_CREATE {
			let create = &*(lparam as *const sys::CreateStructA);
			sys::SetWindowLongPtr(window, sys::GWL_USERDATA, create.create_params as isize);
			return 0;
		}
		let shared = sys::GetWindowLongPtr(window, sys::GWL_USERDATA) as *const Shared;
		if shared.is_null() {
			return sys::DefWindowProcA(window, message, wparam, lparam);
		}
		let shared = &*shared;
		match message {
			sys::WM_CLOSE => {
				sys::DestroyWindow(window);
				return 0;
			}
			sys::WM_DESTROY => {
				sys::PostQuitMessage(0);
				return 0;
			}
			sys::WM_ERASEBKGND => return 1,
			sys::WM_PAINT => {
				let mut paint: sys::PaintStruct = core::mem::zeroed();
				sys::BeginPaint(window, &mut paint);
				sys::EndPaint(window, &paint);
				return 0;
			}
			_ => {}
		}
		if shared.preview {
			return sys::DefWindowProcA(window, message, wparam, lparam);
		}
		match message {
			sys::WM_SETCURSOR => {
				sys::SetCursor(sys::NULL);
				return 1;
			}
			sys::WM_ACTIVATE | sys::WM_ACTIVATEAPP | sys::WM_NCACTIVATE => {
				let active = if message == sys::WM_ACTIVATE {
					wparam & 0xFFFF != 0
				} else {
					wparam != 0
				};
				if active {
					shared.was_active.set(true);
				} else if shared.was_active.get() {
					close(window);
				}
			}
			sys::WM_KEYDOWN
			| sys::WM_KEYUP
			| sys::WM_SYSKEYDOWN
			| sys::WM_LBUTTONDOWN
			| sys::WM_MBUTTONDOWN
			| sys::WM_RBUTTONDOWN
			| sys::WM_XBUTTONDOWN => close(window),
			sys::WM_MOUSEMOVE => {
				let x = i32::from(lparam as i16);
				let y = i32::from((lparam >> 16) as i16);
				match shared.first_move.get() {
					None => shared.first_move.set(Some((x, y))),
					Some((x0, y0)) => {
						if (x - x0).abs() > MOUSE_THRESHOLD || (y - y0).abs() > MOUSE_THRESHOLD {
							close(window);
						}
					}
				}
			}
			sys::WM_SYSCOMMAND => {
				if matches!(
					wparam & 0xFFF0,
					sys::SC_CLOSE | sys::SC_NEXTWINDOW | sys::SC_PREVWINDOW | sys::SC_SCREENSAVE
				) {
					return 0;
				}
			}
			_ => {}
		}
		sys::DefWindowProcA(window, message, wparam, lparam)
	}
}

/// Draws a 32-bit top-down frame into a device context, enlarged to `width` by `height` pixels.
pub(crate) fn stretch(dc: Handle, width: i32, height: i32, frame_width: usize, frame_height: usize, pixels: &[u32]) {
	let info = sys::BitmapInfoHeader {
		size: core::mem::size_of::<sys::BitmapInfoHeader>() as u32,
		width: frame_width as i32,
		// A negative height is a top-down bitmap, which is how the core's frames are stored.
		height: -(frame_height as i32),
		planes: 1,
		bit_count: 32,
		compression: sys::BI_RGB,
		size_image: 0,
		x_pels_per_metre: 0,
		y_pels_per_metre: 0,
		colours_used: 0,
		colours_important: 0,
	};
	// SAFETY: the context is live; `pixels` holds `frame_width * frame_height` entries (the callers pass a frame's own buffer).
	unsafe {
		sys::SetStretchBltMode(dc, sys::COLORONCOLOR);
		sys::StretchDIBits(
			dc,
			0,
			0,
			width,
			height,
			0,
			0,
			frame_width as i32,
			frame_height as i32,
			pixels.as_ptr() as *const c_void,
			&info,
			sys::DIB_RGB_COLORS,
			sys::SRCCOPY,
		);
	}
}

/// Shows the frame in the window, enlarged to its client area.
fn blit(window: Handle, frame: &Frame) {
	// SAFETY: the window is ours; the rectangle and the DC are used only here.
	unsafe {
		let mut client = sys::Rect {
			left: 0,
			top: 0,
			right: frame.w as i32,
			bottom: frame.h as i32,
		};
		sys::GetClientRect(window, &mut client);
		let dc = sys::GetDC(window);
		if dc.is_null() {
			return;
		}
		stretch(
			dc,
			client.right - client.left,
			client.bottom - client.top,
			frame.w,
			frame.h,
			&frame.px,
		);
		sys::ReleaseDC(window, dc);
	}
}

fn register_class(instance: Handle) {
	let class = sys::WndClassA {
		style: 0,
		window_proc: Some(window_proc),
		class_extra: 0,
		window_extra: 0,
		instance,
		// SAFETY: the instance is ours; a missing icon gives null.
		icon: unsafe { sys::LoadIconA(instance, sys::ICON_ID as *const u8) },
		cursor: sys::NULL,
		// SAFETY: a stock object.
		background: unsafe { sys::GetStockObject(sys::BLACK_BRUSH) },
		menu_name: core::ptr::null(),
		class_name: CLASS_NAME.as_ptr(),
	};
	// SAFETY: the class is fully filled in and its name ends in a NUL.
	unsafe { sys::RegisterClassA(&class) };
}

/// The seed: the setting, or a new one from the tick count and the cursor when the setting is 0.
pub(crate) fn seed_for(settings: &Settings) -> u32 {
	let set = settings.get(Key::Seed);
	if set != 0 {
		return set;
	}
	let mut cursor = sys::Point { x: 0, y: 0 };
	// SAFETY: a valid out pointer.
	let tick = unsafe {
		sys::GetCursorPos(&mut cursor);
		sys::GetTickCount()
	};
	(tick ^ (cursor.x as u32).wrapping_mul(0x9E37_79B1) ^ (cursor.y as u32).wrapping_mul(0x85EB_CA6B)) | 1
}

/// Where the full-screen window goes: the virtual screen, or the primary one where the system has no such metrics.
fn screen_rect() -> (i32, i32, i32, i32) {
	// SAFETY: plain queries.
	unsafe {
		let (x, y) = (
			sys::GetSystemMetrics(sys::SM_XVIRTUALSCREEN),
			sys::GetSystemMetrics(sys::SM_YVIRTUALSCREEN),
		);
		let (w, h) = (
			sys::GetSystemMetrics(sys::SM_CXVIRTUALSCREEN),
			sys::GetSystemMetrics(sys::SM_CYVIRTUALSCREEN),
		);
		if w > 0 && h > 0 {
			(x, y, w, h)
		} else {
			(
				0,
				0,
				sys::GetSystemMetrics(sys::SM_CXSCREEN).max(1),
				sys::GetSystemMetrics(sys::SM_CYSCREEN).max(1),
			)
		}
	}
}

/// Creates the window and runs the frame loop until it is closed. Returns the process exit code.
fn run_window(kind: SaverKind, make: MakeScene, parent: Option<Handle>) -> u32 {
	let settings = Settings::load(kind, &RegStore::open(kind));
	let preview = parent.is_some();
	let (x, y, width, height) = match parent {
		Some(parent) => {
			let mut rect = sys::Rect {
				left: 0,
				top: 0,
				right: 0,
				bottom: 0,
			};
			// SAFETY: a valid out pointer.
			unsafe { sys::GetClientRect(parent, &mut rect) };
			let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
			if w > 0 && h > 0 {
				(0, 0, w, h)
			} else {
				(0, 0, PREVIEW_FALLBACK.0, PREVIEW_FALLBACK.1)
			}
		}
		None => screen_rect(),
	};
	let (render_w, render_h) = render_size(&settings, width as usize, height as usize, preview);
	let scene = make(&settings, seed_for(&settings), render_w, render_h);
	let fps = if preview {
		PREVIEW_FPS.min(settings.get(Key::MaxFps))
	} else {
		settings.get(Key::MaxFps)
	}
	.max(1);
	let mut saver = Saver::new(settings, scene, width as usize, height as usize, preview);

	let shared = Shared {
		preview,
		first_move: Cell::new(None),
		was_active: Cell::new(false),
	};
	// SAFETY: the class name and title end in NULs; `shared` outlives the window, which is gone before this function returns.
	let window = unsafe {
		let instance = sys::GetModuleHandleA(core::ptr::null());
		register_class(instance);
		let title = crate::ansi::to_ansi(kind.title());
		let (ex_style, style) = if preview {
			(0, sys::WS_CHILD | sys::WS_VISIBLE | sys::WS_CLIPCHILDREN)
		} else {
			(
				sys::WS_EX_TOOLWINDOW | sys::WS_EX_TOPMOST,
				sys::WS_POPUP | sys::WS_VISIBLE | sys::WS_CLIPSIBLINGS | sys::WS_CLIPCHILDREN,
			)
		};
		sys::CreateWindowExA(
			ex_style,
			CLASS_NAME.as_ptr(),
			title.as_ptr(),
			style,
			x,
			y,
			width,
			height,
			parent.unwrap_or(sys::NULL),
			sys::NULL,
			instance,
			&shared as *const Shared as *mut c_void,
		)
	};
	if window.is_null() {
		return 1;
	}

	let clock = Clock::new();
	let frame_length = 1.0 / f64::from(fps);
	let mut last = clock.seconds();
	let mut message: sys::Msg = sys::Msg {
		window: sys::NULL,
		message: 0,
		wparam: 0,
		lparam: 0,
		time: 0,
		point: sys::Point { x: 0, y: 0 },
	};
	// SAFETY: every call is given the window and message this function owns.
	unsafe {
		loop {
			while sys::PeekMessageA(&mut message, sys::NULL, 0, 0, sys::PM_REMOVE) != 0 {
				if message.message == sys::WM_QUIT {
					return 0;
				}
				sys::TranslateMessage(&message);
				sys::DispatchMessageA(&message);
			}
			if let Some(parent) = parent
				&& sys::IsWindow(parent) == 0
			{
				sys::DestroyWindow(window);
				continue;
			}
			let now = clock.seconds();
			let elapsed = now - last;
			if elapsed < frame_length {
				// Sleep out the rest of the frame, waking early for input.
				let wait = (((frame_length - elapsed) * 1000.0) as u32).max(1);
				sys::MsgWaitForMultipleObjects(0, core::ptr::null(), 0, wait, sys::QS_ALLINPUT);
				continue;
			}
			last = now;
			let frame = saver.step(elapsed as f32);
			blit(window, frame);
		}
	}
}

/// The program: reads the command line and runs the mode it names. Returns the exit code.
pub fn run(kind: SaverKind, make: MakeScene) -> u32 {
	// SAFETY: the command line is a NUL-terminated string the system keeps alive.
	let line = unsafe {
		let start = sys::GetCommandLineA();
		let mut length = 0;
		while *start.add(length) != 0 {
			length += 1;
		}
		core::slice::from_raw_parts(start, length)
	};
	match cmdline::parse(line) {
		Mode::Full => run_window(kind, make, None),
		Mode::Preview(handle) => {
			let parent = handle as Handle;
			// SAFETY: any handle value may be asked about.
			if handle == 0 || unsafe { sys::IsWindow(parent) } == 0 {
				return 1;
			}
			run_window(kind, make, Some(parent))
		}
		Mode::Settings(owner) => {
			// SAFETY: any handle value may be asked about.
			let owner = unsafe {
				match owner {
					Some(handle) if sys::IsWindow(handle as Handle) != 0 => handle as Handle,
					_ => sys::GetForegroundWindow(),
				}
			};
			dialog::show_settings(kind, owner, make);
			0
		}
		Mode::Plain => {
			dialog::show_settings(kind, sys::NULL, make);
			0
		}
		Mode::Password => 0,
	}
}
