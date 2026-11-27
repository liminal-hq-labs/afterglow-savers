// A test program for the ReactOS guest check: runs each scene's frame loop flat out at 1024 x 768 with the shipped defaults (50% render
// scale, Medium detail) and writes how many frames it drew a second, and how long the scene took to start, to `C:\FPS.TXT`. Not a shipped
// file; `scripts/guest/` copies it into the guest. Exit code 0 when the file was written.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]
#![allow(non_snake_case, clippy::upper_case_acronyms)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;

use afterglow_win32::ExitProcess;
use savers_core::saver::{Saver, render_size};
use savers_core::scenes::build;
use savers_core::settings::{SaverKind, Settings};

type Handle = *mut c_void;

#[cfg_attr(
	target_arch = "x86",
	link(name = "kernel32", kind = "raw-dylib", import_name_type = "undecorated")
)]
#[cfg_attr(not(target_arch = "x86"), link(name = "kernel32", kind = "raw-dylib"))]
unsafe extern "system" {
	fn CreateFileA(
		name: *const u8,
		access: u32,
		share: u32,
		security: *const c_void,
		disposition: u32,
		flags: u32,
		template: Handle,
	) -> Handle;
	fn WriteFile(file: Handle, data: *const u8, length: u32, written: *mut u32, overlapped: *mut c_void) -> i32;
	fn CloseHandle(handle: Handle) -> i32;
	fn QueryPerformanceCounter(count: *mut i64) -> i32;
	fn QueryPerformanceFrequency(frequency: *mut i64) -> i32;
}

fn now() -> f64 {
	let (mut count, mut frequency) = (0i64, 0i64);
	// SAFETY: both point to live integers.
	unsafe {
		QueryPerformanceCounter(&mut count);
		QueryPerformanceFrequency(&mut frequency);
	}
	count as f64 / frequency.max(1) as f64
}

fn push_number(text: &mut String, value: u64) {
	let mut digits = Vec::new();
	let mut rest = value;
	loop {
		digits.push(b'0' + (rest % 10) as u8);
		rest /= 10;
		if rest == 0 {
			break;
		}
	}
	for digit in digits.iter().rev() {
		text.push(*digit as char);
	}
}

fn measure(kind: SaverKind, text: &mut String) {
	let (w, h) = (1024usize, 768usize);
	let settings = Settings::defaults(kind);
	let started = now();
	let (rw, rh) = render_size(&settings, w, h, false);
	let scene = build(kind, &settings, 7, rw, rh);
	let mut saver = Saver::new(settings, scene, w, h, false);
	let start_ms = ((now() - started) * 1000.0) as u64;
	let mut frames = 0u64;
	let begin = now();
	// Frames of a fixed 1/30 s, as the loop gives at its default cap, for five seconds of wall time.
	while now() - begin < 5.0 {
		let _ = saver.step(1.0 / 30.0);
		frames += 1;
	}
	let seconds = now() - begin;
	text.push_str(kind.registry_name());
	text.push_str(" start_ms ");
	push_number(text, start_ms);
	text.push_str(" frames ");
	push_number(text, frames);
	text.push_str(" tenths_fps ");
	push_number(text, (frames as f64 / seconds * 10.0) as u64);
	text.push_str("\r\n");
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> ! {
	let mut text = String::new();
	for kind in [
		SaverKind::Ink,
		SaverKind::Twilight,
		SaverKind::Cracktro,
		SaverKind::Maze,
		SaverKind::Collection,
		SaverKind::Ghostwriter,
	] {
		measure(kind, &mut text);
	}
	let mut code = 1;
	// SAFETY: the name ends in a NUL and the data is as long as the length says.
	unsafe {
		let file = CreateFileA(
			c"C:\\FPS.TXT".as_ptr().cast(),
			0x4000_0000,
			0,
			core::ptr::null(),
			2,
			0x80,
			core::ptr::null_mut(),
		);
		if file as isize != -1 {
			let mut written = 0u32;
			WriteFile(file, text.as_ptr(), text.len() as u32, &mut written, core::ptr::null_mut());
			CloseHandle(file);
			code = 0;
		}
		ExitProcess(code)
	}
}
