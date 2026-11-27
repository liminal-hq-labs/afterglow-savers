// What the 32-bit MSVC ABI expects of a program with no C library: the 64-bit division helpers and the stack probe. The helpers are after
// Quill's `rt.rs` through the Liminal Installer's Win32 runtime (see PROVENANCE.md): `compiler_builtins` names them, so they must exist
// even though no code here divides 64-bit numbers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::global_asm;

// The 64-bit division helpers the MSVC ABI expects (`_alldiv` and friends): stdcall, arguments on the stack, result in EDX:EAX. Each wraps a
// Rust routine that divides by shifting and subtracting, since a division here would call the helper again.
fn udivmod(n: u64, d: u64) -> (u64, u64) {
	if d == 0 {
		return (0, n);
	}
	let (mut q, mut r) = (0u64, 0u64);
	let mut i = 64;
	while i > 0 {
		i -= 1;
		r = (r << 1) | ((n >> i) & 1);
		if r >= d {
			r -= d;
			q |= 1 << i;
		}
	}
	(q, r)
}

fn join(lo: u32, hi: u32) -> u64 {
	(u64::from(hi) << 32) | u64::from(lo)
}

extern "C" fn udiv64(al: u32, ah: u32, bl: u32, bh: u32) -> u64 {
	udivmod(join(al, ah), join(bl, bh)).0
}

extern "C" fn urem64(al: u32, ah: u32, bl: u32, bh: u32) -> u64 {
	udivmod(join(al, ah), join(bl, bh)).1
}

extern "C" fn idiv64(al: u32, ah: u32, bl: u32, bh: u32) -> u64 {
	let (a, b) = (join(al, ah) as i64, join(bl, bh) as i64);
	let q = udivmod(a.unsigned_abs(), b.unsigned_abs()).0;
	if (a < 0) != (b < 0) { q.wrapping_neg() } else { q }
}

extern "C" fn irem64(al: u32, ah: u32, bl: u32, bh: u32) -> u64 {
	let (a, b) = (join(al, ah) as i64, join(bl, bh) as i64);
	let r = udivmod(a.unsigned_abs(), b.unsigned_abs()).1;
	if a < 0 { r.wrapping_neg() } else { r }
}

macro_rules! stdcall_wrapper {
	($name:literal, $f:path) => {
		global_asm!(
			concat!(".globl ", $name),
			concat!($name, ":"),
			"push dword ptr [esp + 16]",
			"push dword ptr [esp + 16]",
			"push dword ptr [esp + 16]",
			"push dword ptr [esp + 16]",
			"call {f}",
			"add esp, 16",
			"ret 16",
			f = sym $f,
		);
	};
}

stdcall_wrapper!("__aulldiv", udiv64);
stdcall_wrapper!("__aullrem", urem64);
stdcall_wrapper!("__alldiv", idiv64);
stdcall_wrapper!("__allrem", irem64);

// The stack probe the compiler calls before a frame of a page or more, after the installer's Win32 runtime: EAX holds the frame's size.
// The 32-bit MSVC probe also moves the stack pointer down by that size (the caller does not subtract it), keeping the return address
// on top, and it touches each page on the way so that Windows commits the guard page one page at a time.
global_asm!(
	".globl __chkstk",
	"__chkstk:",
	"push ecx",
	"lea ecx, [esp + 8]",
	"2:",
	"cmp eax, 0x1000",
	"jb 3f",
	"sub ecx, 0x1000",
	"test [ecx], eax",
	"sub eax, 0x1000",
	"jmp 2b",
	"3:",
	"sub ecx, eax",
	"test [ecx], eax",
	"lea eax, [esp + 4]",
	"mov esp, ecx",
	"mov ecx, [eax - 4]",
	"push dword ptr [eax]",
	"ret",
);
