// The Win32 calls the savers make: ANSI entry points only, imported by name (raw-dylib), nothing newer than Windows 95. Only 32-bit x86
// decorates `stdcall` names, so only there is the import name type undecorated. `imports.allow` lists them, and `tools/check-imports.sh`
// keeps each executable to that list.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms, dead_code)]

use core::ffi::c_void;

pub type Handle = *mut c_void;
pub type WndProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

pub const NULL: Handle = core::ptr::null_mut();

// Windows and classes.
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
pub const WS_CAPTION: u32 = 0x00C0_0000;
pub const WS_BORDER: u32 = 0x0080_0000;
pub const WS_SYSMENU: u32 = 0x0008_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_GROUP: u32 = 0x0002_0000;
pub const WS_EX_TOPMOST: u32 = 0x0000_0008;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;
pub const GWL_USERDATA: i32 = -21;
pub const BLACK_BRUSH: i32 = 4;
pub const PM_REMOVE: u32 = 1;
pub const QS_ALLINPUT: u32 = 0x04FF;
pub const WAIT_TIMEOUT: u32 = 258;
pub const COLORONCOLOR: i32 = 3;
pub const DIB_RGB_COLORS: u32 = 0;
pub const SRCCOPY: u32 = 0x00CC_0020;
pub const BI_RGB: u32 = 0;
pub const INFINITE: u32 = 0xFFFF_FFFF;

// Messages.
pub const WM_CREATE: u32 = 0x0001;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_ACTIVATE: u32 = 0x0006;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_ACTIVATEAPP: u32 = 0x001C;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_INITDIALOG: u32 = 0x0110;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_SYSCOMMAND: u32 = 0x0112;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_XBUTTONDOWN: u32 = 0x020B;
pub const WM_NCACTIVATE: u32 = 0x0086;
pub const SC_CLOSE: usize = 0xF060;
pub const SC_NEXTWINDOW: usize = 0xF040;
pub const SC_PREVWINDOW: usize = 0xF050;
pub const SC_SCREENSAVE: usize = 0xF140;

// Controls.
pub const BM_GETCHECK: u32 = 0x00F0;
pub const BM_SETCHECK: u32 = 0x00F1;
pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const CB_SETCURSEL: u32 = 0x014E;
pub const EM_LIMITTEXT: u32 = 0x00C5;
pub const WM_USER: u32 = 0x0400;
pub const TBM_GETPOS: u32 = WM_USER;
pub const TBM_SETPOS: u32 = WM_USER + 5;
pub const TBM_SETRANGE: u32 = WM_USER + 6;
pub const WM_SETICON: u32 = 0x0080;
/// The identifier of every saver's icon resource.
pub const ICON_ID: usize = 101;
pub const IDOK: usize = 1;
pub const IDCANCEL: usize = 2;

#[repr(C)]
pub struct Point {
	pub x: i32,
	pub y: i32,
}

#[repr(C)]
pub struct Rect {
	pub left: i32,
	pub top: i32,
	pub right: i32,
	pub bottom: i32,
}

#[repr(C)]
pub struct Msg {
	pub window: Handle,
	pub message: u32,
	pub wparam: usize,
	pub lparam: isize,
	pub time: u32,
	pub point: Point,
}

#[repr(C)]
pub struct WndClassA {
	pub style: u32,
	pub window_proc: Option<WndProc>,
	pub class_extra: i32,
	pub window_extra: i32,
	pub instance: Handle,
	pub icon: Handle,
	pub cursor: Handle,
	pub background: Handle,
	pub menu_name: *const u8,
	pub class_name: *const u8,
}

#[repr(C)]
pub struct PaintStruct {
	pub dc: Handle,
	pub erase: i32,
	pub paint: Rect,
	pub restore: i32,
	pub increment_update: i32,
	pub reserved: [u8; 32],
}

#[repr(C)]
pub struct CreateStructA {
	pub create_params: *mut c_void,
	pub instance: Handle,
	pub menu: Handle,
	pub parent: Handle,
	pub height: i32,
	pub width: i32,
	pub y: i32,
	pub x: i32,
	pub style: i32,
	pub name: *const u8,
	pub class_name: *const u8,
	pub ex_style: u32,
}

#[repr(C)]
pub struct BitmapInfoHeader {
	pub size: u32,
	pub width: i32,
	pub height: i32,
	pub planes: u16,
	pub bit_count: u16,
	pub compression: u32,
	pub size_image: u32,
	pub x_pels_per_metre: i32,
	pub y_pels_per_metre: i32,
	pub colours_used: u32,
	pub colours_important: u32,
}

// The registry.
pub const HKEY_CURRENT_USER: usize = 0x8000_0001;
pub const KEY_QUERY_VALUE: u32 = 0x0001;
pub const KEY_SET_VALUE: u32 = 0x0002;
pub const KEY_ALL_ACCESS: u32 = 0x000F_003F;
pub const ERROR_SUCCESS: i32 = 0;
pub const REG_SZ: u32 = 1;
pub const REG_DWORD: u32 = 4;
pub const REG_OPTION_NON_VOLATILE: u32 = 0;
pub const HEAP_ZERO_MEMORY: u32 = 8;

#[cfg_attr(
	target_arch = "x86",
	link(name = "kernel32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "kernel32", kind = "raw-dylib"))]
unsafe extern "system" {
	pub fn ExitProcess(code: u32) -> !;
	pub fn GetCommandLineA() -> *const u8;
	pub fn GetModuleHandleA(name: *const u8) -> Handle;
	pub fn GetTickCount() -> u32;
	pub fn QueryPerformanceCounter(count: *mut i64) -> i32;
	pub fn QueryPerformanceFrequency(frequency: *mut i64) -> i32;
	pub fn GetProcessHeap() -> Handle;
	pub fn HeapAlloc(heap: Handle, flags: u32, bytes: usize) -> *mut c_void;
	pub fn HeapReAlloc(heap: Handle, flags: u32, block: *mut c_void, bytes: usize) -> *mut c_void;
	pub fn HeapFree(heap: Handle, flags: u32, block: *mut c_void) -> i32;
}

#[cfg_attr(
	target_arch = "x86",
	link(name = "advapi32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "advapi32", kind = "raw-dylib"))]
unsafe extern "system" {
	pub fn RegCreateKeyExA(
		key: usize,
		sub_key: *const u8,
		reserved: u32,
		class: *const u8,
		options: u32,
		access: u32,
		security: *const c_void,
		result: *mut usize,
		disposition: *mut u32,
	) -> i32;
	pub fn RegOpenKeyExA(key: usize, sub_key: *const u8, options: u32, access: u32, result: *mut usize) -> i32;
	pub fn RegQueryValueExA(
		key: usize,
		name: *const u8,
		reserved: *mut u32,
		kind: *mut u32,
		data: *mut u8,
		size: *mut u32,
	) -> i32;
	pub fn RegSetValueExA(key: usize, name: *const u8, reserved: u32, kind: u32, data: *const u8, size: u32) -> i32;
	pub fn RegCloseKey(key: usize) -> i32;
}

#[cfg_attr(
	target_arch = "x86",
	link(name = "user32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "user32", kind = "raw-dylib"))]
unsafe extern "system" {
	pub fn LoadIconA(instance: Handle, name: *const u8) -> Handle;
	pub fn RegisterClassA(class: *const WndClassA) -> u16;
	pub fn CreateWindowExA(
		ex_style: u32,
		class_name: *const u8,
		title: *const u8,
		style: u32,
		x: i32,
		y: i32,
		width: i32,
		height: i32,
		parent: Handle,
		menu: Handle,
		instance: Handle,
		param: *mut c_void,
	) -> Handle;
	pub fn DestroyWindow(window: Handle) -> i32;
	pub fn DefWindowProcA(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
	pub fn PeekMessageA(msg: *mut Msg, window: Handle, min: u32, max: u32, remove: u32) -> i32;
	pub fn TranslateMessage(msg: *const Msg) -> i32;
	pub fn DispatchMessageA(msg: *const Msg) -> isize;
	pub fn MsgWaitForMultipleObjects(count: u32, handles: *const Handle, wait_all: i32, milliseconds: u32, mask: u32) -> u32;
	pub fn PostMessageA(window: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
	pub fn PostQuitMessage(code: i32);
	pub fn IsWindow(window: Handle) -> i32;
	pub fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
	pub fn GetSystemMetrics(index: i32) -> i32;
	pub fn GetCursorPos(point: *mut Point) -> i32;
	pub fn GetForegroundWindow() -> Handle;
	pub fn SetCursor(cursor: Handle) -> Handle;
	pub fn ShowCursor(show: i32) -> i32;
	pub fn GetDC(window: Handle) -> Handle;
	pub fn ReleaseDC(window: Handle, dc: Handle) -> i32;
	pub fn BeginPaint(window: Handle, paint: *mut PaintStruct) -> Handle;
	pub fn EndPaint(window: Handle, paint: *const PaintStruct) -> i32;
	pub fn SendMessageA(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
	pub fn GetDlgItem(dialog: Handle, id: i32) -> Handle;
	pub fn SetDlgItemTextA(dialog: Handle, id: i32, text: *const u8) -> i32;
	pub fn GetDlgItemTextA(dialog: Handle, id: i32, text: *mut u8, max: i32) -> u32;
	pub fn DialogBoxIndirectParamA(
		instance: Handle,
		template: *const u8,
		owner: Handle,
		dialog_proc: Option<WndProc>,
		param: isize,
	) -> isize;
	pub fn EndDialog(dialog: Handle, result: isize) -> i32;
}

#[cfg(target_arch = "x86")]
#[link(name = "user32", kind = "raw-dylib", import_name_type = "undecorated")]
unsafe extern "system" {
	#[link_name = "SetWindowLongA"]
	pub fn SetWindowLongPtr(window: Handle, index: i32, value: isize) -> isize;
	#[link_name = "GetWindowLongA"]
	pub fn GetWindowLongPtr(window: Handle, index: i32) -> isize;
}

#[cfg(not(target_arch = "x86"))]
#[link(name = "user32", kind = "raw-dylib")]
unsafe extern "system" {
	#[link_name = "SetWindowLongPtrA"]
	pub fn SetWindowLongPtr(window: Handle, index: i32, value: isize) -> isize;
	#[link_name = "GetWindowLongPtrA"]
	pub fn GetWindowLongPtr(window: Handle, index: i32) -> isize;
}

#[cfg_attr(
	target_arch = "x86",
	link(name = "gdi32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "gdi32", kind = "raw-dylib"))]
unsafe extern "system" {
	pub fn StretchDIBits(
		dc: Handle,
		x_dest: i32,
		y_dest: i32,
		width_dest: i32,
		height_dest: i32,
		x_source: i32,
		y_source: i32,
		width_source: i32,
		height_source: i32,
		bits: *const c_void,
		info: *const BitmapInfoHeader,
		usage: u32,
		operation: u32,
	) -> i32;
	pub fn SetStretchBltMode(dc: Handle, mode: i32) -> i32;
	pub fn GetStockObject(object: i32) -> Handle;
}

#[cfg_attr(
	target_arch = "x86",
	link(name = "comctl32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "comctl32", kind = "raw-dylib"))]
unsafe extern "system" {
	pub fn InitCommonControls();
}
