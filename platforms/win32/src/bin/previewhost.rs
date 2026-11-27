// A test stand-in for the Display applet's preview pane: makes a small window, starts `<saver> /p <hwnd>` inside it, waits, destroys the
// window and checks the saver notices and exits. Not a shipped file; `scripts/smoke-wine.sh` runs it. Usage: previewhost <saver.scr>
//
// Exit code 0: the saver ran for the whole wait and ended by itself after its parent went. 2: it was gone early. 3: it did not end.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]
#![allow(non_snake_case, clippy::upper_case_acronyms)]

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use afterglow_win32::ExitProcess;
use afterglow_win32::ansi::number_text;

type Handle = *mut c_void;

#[repr(C)]
struct ProcessInformation {
	process: Handle,
	thread: Handle,
	process_id: u32,
	thread_id: u32,
}

#[repr(C)]
struct Message {
	window: Handle,
	message: u32,
	wparam: usize,
	lparam: isize,
	time: u32,
	x: i32,
	y: i32,
}

#[cfg_attr(
	target_arch = "x86",
	link(name = "kernel32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "kernel32", kind = "raw-dylib"))]
unsafe extern "system" {
	fn GetCommandLineA() -> *const u8;
	fn CreateProcessA(
		application: *const u8,
		command_line: *mut u8,
		process_attributes: *const c_void,
		thread_attributes: *const c_void,
		inherit: i32,
		flags: u32,
		environment: *const c_void,
		directory: *const u8,
		startup: *const u32,
		info: *mut ProcessInformation,
	) -> i32;
	fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
	fn TerminateProcess(handle: Handle, code: u32) -> i32;
	fn GetTickCount() -> u32;
	fn Sleep(milliseconds: u32);
}

#[cfg_attr(
	target_arch = "x86",
	link(name = "user32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "user32", kind = "raw-dylib"))]
unsafe extern "system" {
	fn CreateWindowExA(
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
	fn DestroyWindow(window: Handle) -> i32;
	fn PeekMessageA(msg: *mut Message, window: Handle, min: u32, max: u32, remove: u32) -> i32;
	fn TranslateMessage(msg: *const Message) -> i32;
	fn DispatchMessageA(msg: *const Message) -> isize;
}

/// The saver's path: the text after the program's own name.
fn argument(line: &[u8]) -> Vec<u8> {
	let mut i = 0;
	if line.first() == Some(&b'"') {
		i = 1;
		while i < line.len() && line[i] != b'"' {
			i += 1;
		}
		i += 1;
	} else {
		while i < line.len() && line[i] != b' ' {
			i += 1;
		}
	}
	let rest = &line[i.min(line.len())..];
	rest.iter().copied().skip_while(|&b| b == b' ').collect()
}

fn pump_for(milliseconds: u32) {
	// SAFETY: plain message-loop calls on this thread.
	unsafe {
		let until = GetTickCount().wrapping_add(milliseconds);
		let mut message: Message = core::mem::zeroed();
		while until.wrapping_sub(GetTickCount()) < 0x8000_0000 {
			while PeekMessageA(&mut message, core::ptr::null_mut(), 0, 0, 1) != 0 {
				TranslateMessage(&message);
				DispatchMessageA(&message);
			}
			Sleep(10);
		}
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
	// SAFETY: the command line is NUL-terminated; every handle comes from the call before it.
	let code = unsafe {
		let start = GetCommandLineA();
		let mut length = 0;
		while *start.add(length) != 0 {
			length += 1;
		}
		let path = argument(core::slice::from_raw_parts(start, length));
		// WS_OVERLAPPEDWINDOW | WS_VISIBLE, a bit bigger than the Display applet's 152 by 112 preview.
		let parent = CreateWindowExA(
			0,
			c"STATIC".as_ptr().cast(),
			c"preview host".as_ptr().cast(),
			0x10CF_0000,
			100,
			100,
			160,
			142,
			core::ptr::null_mut(),
			core::ptr::null_mut(),
			core::ptr::null_mut(),
			core::ptr::null_mut(),
		);
		let mut command: Vec<u8> = Vec::new();
		command.push(b'"');
		command.extend_from_slice(&path);
		command.extend_from_slice(b"\" /p ");
		let digits = number_text(parent as usize as u32);
		command.extend_from_slice(&digits);
		let mut startup = [0u32; 26];
		startup[0] = if cfg!(target_pointer_width = "64") { 104 } else { 68 };
		let mut info: ProcessInformation = core::mem::zeroed();
		if CreateProcessA(
			core::ptr::null(),
			command.as_mut_ptr(),
			core::ptr::null(),
			core::ptr::null(),
			0,
			0,
			core::ptr::null(),
			core::ptr::null(),
			startup.as_ptr(),
			&mut info,
		) == 0
		{
			ExitProcess(4);
		}
		pump_for(6000);
		let mut state = WaitForSingleObject(info.process, 0);
		if state == 0 {
			// Gone before its parent: it crashed or refused the window.
			ExitProcess(2);
		}
		DestroyWindow(parent);
		pump_for(500);
		state = WaitForSingleObject(info.process, 5000);
		if state != 0 {
			TerminateProcess(info.process, 1);
			3
		} else {
			0
		}
	};
	// SAFETY: ends the process.
	unsafe { ExitProcess(code) }
}
