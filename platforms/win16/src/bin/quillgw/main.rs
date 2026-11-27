// QUILLGW.SCR: Quill Ghostwriter for Windows 3.x. A Quill window drifts across a dark desk and types a document that is never saved.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// It follows the Windows 3.x screen saver conventions that SCRNSAVE.LIB gave a saver written in C: the module description starts with
// `SCRNSAVE : ` (Control Panel's Desktop applet lists the file under the rest), the command line is `/s` to run and `/c` to configure
// (`-` works as well as `/`), the window class is `WindowsScreenSaverClass`, any key or click or a real mouse move ends it, and the
// settings live in CONTROL.INI under `[Screen Saver.Quill Ghostwriter]`.
#![no_std]
#![no_main]
mod draw;
mod scene;

use core::ptr::addr_of_mut;
use draw::{BITMAP_H, BITMAP_W, Gdi};
use scene::{Scene, Settings};
use w16::*;

// Exported under the names the stock Windows 3.x savers use, in the order they use them.
export_proc!(SCREENSAVERPROC, saver_proc);
export_proc!(SCREENSAVERCONFIGUREDIALOG, config_dlgproc);

const WM_ERASEBKGND: u32 = 0x0014;
const WM_ACTIVATE: u32 = 0x0006;
const WM_SETCURSOR: u32 = 0x0020;
const WM_CLOSE: u32 = 0x0010;
const WM_TIMER: u32 = 0x0113;
const WM_SYSCOMMAND: u32 = 0x0112;
const WM_KEYDOWN: u32 = 0x0100;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_MBUTTONDOWN: u32 = 0x0207;
const SC_SCREENSAVE: u32 = 0xF140;
const WS_POPUP_VISIBLE: u32 = 0x9000_0000;
const WS_EX_TOPMOST: u32 = 0x0000_0008;
const TICK_MS: u32 = 60;
/// Mouse movement of this many pixels or fewer is noise, not a person.
const MOUSE_SLACK: i32 = 3;
/// The sections and keys of CONTROL.INI.
const SECTION: &str = "Screen Saver.Quill Ghostwriter\0";

/// Bits of the `Acts` setting, numbered as the acts of the Windows 95 edition: act 1 is typing with mistakes, act 6 the visit from the
/// Save box. The other acts need more than a Windows 3.x GDI scene can do.
const ACT_TYPING: u32 = 1 << 0;
const ACT_VISIT: u32 = 1 << 5;

const ID_TYPOS: u32 = 201;
const ID_VISIT: u32 = 202;
const ID_SLOW: u32 = 204;
const ID_FAST: u32 = 206;

struct Saver {
	scene: Scene,
	gdi: Gdi,
	screen_w: i32,
	screen_h: i32,
	mouse_seen: bool,
	mouse_x: i32,
	mouse_y: i32,
	ticks_since_start: u32,
	/// Where the bitmap was last copied to, so the desk can be repainted where it has moved away from.
	last_x: i32,
	last_y: i32,
	/// The window is being destroyed: later messages, among them the deactivation that destroying it brings, are left alone.
	ending: bool,
}

static mut SAVER: Saver = Saver {
	scene: Scene::new(),
	gdi: Gdi::empty(),
	screen_w: 640,
	screen_h: 480,
	mouse_seen: false,
	mouse_x: 0,
	mouse_y: 0,
	ticks_since_start: 0,
	last_x: 0,
	last_y: 0,
	ending: false,
};

static mut MSG: Msg = Msg {
	hwnd: 0,
	message: 0,
	wparam: 0,
	lparam: 0,
	time: 0,
	pt: [0; 2],
};
static mut PS: PaintStruct = PaintStruct {
	hdc: 0,
	erase: 0,
	rect: [0; 4],
	restore: 0,
	inc_update: 0,
	reserved: [0; 16],
};
static mut CLASS: WndClass = WndClass {
	style: 0,
	wndproc: 0,
	cls_extra: 0,
	wnd_extra: 0,
	instance: 0,
	icon: 0,
	cursor: 0,
	background: 0,
	menu: 0,
	class_name: 0,
};

fn saver() -> &'static mut Saver {
	unsafe { &mut *addr_of_mut!(SAVER) }
}

/// A 256-step sine with an amplitude of 127, from a parabola: smooth enough for a drift, and no table to keep in the data segment.
fn sine(phase: u32) -> i32 {
	let p = (phase & 0xFF) as i32;
	let half = if p < 128 { p } else { p - 128 };
	let value = half * (128 - half) * 127 / 4096;
	if p < 128 { value } else { -value }
}

fn read_settings() -> Settings {
	unsafe {
		let ini = cs!("control.ini");
		let section = SECTION.as_ptr() as usize as u32;
		let acts = GetPrivateProfileInt(section, cs!("Acts"), ACT_TYPING | ACT_VISIT, ini);
		Settings {
			typos: acts & ACT_TYPING != 0,
			visit: acts & ACT_VISIT != 0,
			speed: GetPrivateProfileInt(section, cs!("Speed"), 1, ini).min(2) as u8,
		}
	}
}

fn write_settings(settings: &Settings) {
	unsafe {
		let ini = cs!("control.ini");
		let section = SECTION.as_ptr() as usize as u32;
		let acts = (settings.typos as u32 * ACT_TYPING) | (settings.visit as u32 * ACT_VISIT);
		let mut text = [0u8; 4];
		let mut len = 0;
		let mut digits = [0u8; 3];
		let mut value = acts;
		loop {
			digits[len] = b'0' + (value % 10) as u8;
			len += 1;
			value /= 10;
			if value == 0 {
				break;
			}
		}
		for i in 0..len {
			text[i] = digits[len - 1 - i];
		}
		WritePrivateProfileString(section, cs!("Acts"), text.as_ptr() as usize as u32, ini);
		let speed = match settings.speed {
			0 => cs!("0"),
			1 => cs!("1"),
			_ => cs!("2"),
		};
		WritePrivateProfileString(section, cs!("Speed"), speed, ini);
	}
}

/// Draws the scene into the bitmap when it has changed, and copies it to where the drift has taken it.
fn show_frame(hdc: u32) {
	let s = saver();
	if s.scene.dirty {
		s.gdi.render(&s.scene);
		s.scene.dirty = false;
	}
	let room_x = (s.screen_w - BITMAP_W).max(0);
	let room_y = (s.screen_h - BITMAP_H).max(0);
	let x = room_x / 2 + room_x.min(200) / 2 * sine(s.scene.tick * 3) / 127;
	let y = room_y / 2 + room_y.min(180) / 2 * sine(s.scene.tick * 2 + 64) / 127;
	let (old_x, old_y) = (s.last_x, s.last_y);
	if x > old_x {
		s.gdi.desk(hdc, old_x, old_y, x, old_y + BITMAP_H);
	} else if x < old_x {
		s.gdi
			.desk(hdc, x + BITMAP_W, old_y, old_x + BITMAP_W, old_y + BITMAP_H);
	}
	if y > old_y {
		s.gdi.desk(hdc, old_x, old_y, old_x + BITMAP_W, y);
	} else if y < old_y {
		s.gdi
			.desk(hdc, old_x, y + BITMAP_H, old_x + BITMAP_W, old_y + BITMAP_H);
	}
	s.gdi.blit(hdc, x, y);
	s.last_x = x;
	s.last_y = y;
}

fn start(hwnd: u32) -> bool {
	let s = saver();
	unsafe {
		ShowCursor(0);
		s.screen_w = GetSystemMetrics(0) as i32;
		s.screen_h = GetSystemMetrics(1) as i32;
		let hdc = GetDC(hwnd);
		let made = s.gdi.create(hdc);
		ReleaseDC(hwnd, hdc);
		if !made {
			s.gdi.destroy();
			return false;
		}
		s.scene.settings = read_settings();
		s.scene.seed(GetTickCount());
		SetTimer(hwnd, 1, TICK_MS, 0) != 0
	}
}

fn finish(hwnd: u32) {
	unsafe {
		KillTimer(hwnd, 1);
		saver().gdi.destroy();
		ShowCursor(1);
	}
}

/// Ends the saver, once.
fn wake(hwnd: u32) -> u32 {
	let s = saver();
	if !s.ending {
		s.ending = true;
		unsafe {
			DestroyWindow(hwnd);
		}
	}
	0
}

extern "C" fn saver_proc(hwnd: u32, msg: u32, wparam: u32, lparam: u32) -> u32 {
	let s = saver();
	unsafe {
		match msg {
			WM_CREATE => {
				if !start(hwnd) {
					return 0xFFFF_FFFF;
				}
				0
			}
			WM_TIMER => {
				s.ticks_since_start = s.ticks_since_start.saturating_add(1);
				s.scene.step();
				let hdc = GetDC(hwnd);
				show_frame(hdc);
				ReleaseDC(hwnd, hdc);
				0
			}
			WM_PAINT => {
				let ps = addr_of_mut!(PS);
				let hdc = BeginPaint(hwnd, ps as u32);
				s.gdi.desk(hdc, 0, 0, s.screen_w, s.screen_h);
				s.scene.dirty = true;
				show_frame(hdc);
				EndPaint(hwnd, ps as u32);
				0
			}
			WM_ERASEBKGND => 1,
			WM_SETCURSOR => {
				SetCursor(0);
				1
			}
			WM_SYSCOMMAND if wparam & 0xFFF0 == SC_SCREENSAVE => 0,
			WM_ACTIVATE if wparam & 0xFFFF == 0 && s.ticks_since_start > 10 => wake(hwnd),
			WM_KEYDOWN | WM_SYSKEYDOWN | WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
				wake(hwnd)
			}
			WM_MOUSEMOVE => {
				let x = (lparam & 0xFFFF) as i16 as i32;
				let y = (lparam >> 16) as i16 as i32;
				if !s.mouse_seen {
					s.mouse_seen = true;
					s.mouse_x = x;
					s.mouse_y = y;
				} else if (x - s.mouse_x).abs() + (y - s.mouse_y).abs() > MOUSE_SLACK {
					return wake(hwnd);
				}
				0
			}
			WM_CLOSE => wake(hwnd),
			WM_DESTROY => {
				finish(hwnd);
				PostQuitMessage(0);
				0
			}
			_ => DefWindowProc(hwnd, msg, wparam, lparam),
		}
	}
}

extern "C" fn config_dlgproc(hdlg: u32, msg: u32, wparam: u32, _lparam: u32) -> u32 {
	unsafe {
		match msg {
			WM_INITDIALOG => {
				let settings = read_settings();
				CheckDlgButton(hdlg, ID_TYPOS, settings.typos as u32);
				CheckDlgButton(hdlg, ID_VISIT, settings.visit as u32);
				CheckRadioButton(hdlg, ID_SLOW, ID_FAST, ID_SLOW + settings.speed as u32);
				1
			}
			WM_COMMAND => {
				match wparam {
					IDOK => {
						let mut speed = 1;
						for (i, id) in (ID_SLOW..=ID_FAST).enumerate() {
							if IsDlgButtonChecked(hdlg, id) != 0 {
								speed = i as u8;
							}
						}
						write_settings(&Settings {
							typos: IsDlgButtonChecked(hdlg, ID_TYPOS) != 0,
							visit: IsDlgButtonChecked(hdlg, ID_VISIT) != 0,
							speed,
						});
						EndDialog(hdlg, 1);
					}
					IDCANCEL => {
						EndDialog(hdlg, 0);
					}
					_ => {}
				}
				1
			}
			_ => 0,
		}
	}
}

/// What the command line asks for: `/s` or nothing runs the saver, `/c` configures it, and anything else (`/a`, the password change
/// of Windows 95) does nothing, since a Windows 3.x saver has no password.
enum Mode {
	Run,
	Configure,
	Nothing,
}

fn mode(line: &[u8]) -> Mode {
	let mut i = 0;
	while i < line.len() && line[i] == b' ' {
		i += 1;
	}
	if i + 1 >= line.len() || (line[i] != b'/' && line[i] != b'-') {
		return Mode::Run;
	}
	match line[i + 1] | 0x20 {
		b's' => Mode::Run,
		b'c' => Mode::Configure,
		_ => Mode::Nothing,
	}
}

/// With `Log=1` in the saver's CONTROL.INI section, writes the command line Windows passed to `C:\QGWn.LOG`, one file for each start
/// (`LogCount` in the same section says which is next): how the arguments of Control Panel's Setup and Test and of the idle timer are seen.
fn log_command_line(line: &[u8]) {
	unsafe {
		let ini = cs!("control.ini");
		let section = SECTION.as_ptr() as usize as u32;
		if GetPrivateProfileInt(section, cs!("Log"), 0, ini) == 0 {
			return;
		}
		let count = GetPrivateProfileInt(section, cs!("LogCount"), 0, ini);
		let mut path = *b"C:\\QGW0.LOG\0";
		path[6] = b'0' + (count % 10) as u8;
		let mut text = [0u8; 80];
		let mut len = 0;
		for byte in b"command line: ["
			.iter()
			.chain(line.iter())
			.chain(b"]\r\n".iter())
		{
			if len < text.len() {
				text[len] = *byte;
				len += 1;
			}
		}
		write_file(path.as_ptr(), &text[..len]);
		let next = [b'0' + ((count + 1) % 10) as u8, 0];
		WritePrivateProfileString(section, cs!("LogCount"), next.as_ptr() as usize as u32, ini);
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn w16_main(hinst: u32, hprev: u32, _show: u32) -> u32 {
	unsafe {
		let mut line = [0u8; 48];
		let line_len = command_line(&mut line);
		log_command_line(&line[..line_len]);
		match mode(&line[..line_len]) {
			Mode::Nothing => return 0,
			Mode::Configure => {
				DialogBox(
					hinst,
					100,
					0,
					SCREENSAVERCONFIGUREDIALOG as *const () as usize as u32,
				);
				return 0;
			}
			Mode::Run => {}
		}
		if hprev != 0 {
			return 0;
		}
		let class = addr_of_mut!(CLASS);
		(*class).wndproc = far(
			code_selector(),
			SCREENSAVERPROC as *const () as usize as u32,
		);
		(*class).instance = hinst as u16;
		(*class).class_name = far(data_selector(), cs!("WindowsScreenSaverClass"));
		if RegisterClass(class as u32) == 0 {
			return 1;
		}
		let hwnd = CreateWindowEx(
			WS_EX_TOPMOST,
			cs!("WindowsScreenSaverClass"),
			cs!("Quill Ghostwriter"),
			WS_POPUP_VISIBLE,
			0,
			0,
			GetSystemMetrics(0),
			GetSystemMetrics(1),
			0,
			0,
			hinst,
			0,
		);
		if hwnd == 0 {
			return 2;
		}
		let msg = addr_of_mut!(MSG) as u32;
		while GetMessage(msg, 0, 0, 0) != 0 {
			TranslateMessage(msg);
			DispatchMessage(msg);
		}
		(*addr_of_mut!(MSG)).wparam as u32
	}
}
