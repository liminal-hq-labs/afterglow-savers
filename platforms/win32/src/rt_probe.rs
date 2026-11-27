// The stack probe of the 64-bit MSVC ABIs, which the compiler calls before a frame of a page or more. Unlike the 32-bit one it does not
// move the stack pointer: it only touches each page of the new frame, in order, so that Windows commits the guard page one page at a time.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::arch::global_asm;

// x86-64: RAX is the frame's size and must survive (the caller subtracts it next); nothing but R10 and R11 may change.
#[cfg(target_arch = "x86_64")]
global_asm!(
	".globl __chkstk",
	"__chkstk:",
	"lea r10, [rsp + 8]",
	"mov r11, rax",
	"2:",
	"cmp r11, 0x1000",
	"jb 3f",
	"sub r10, 0x1000",
	"test [r10], r10",
	"sub r11, 0x1000",
	"jmp 2b",
	"3:",
	"sub r10, r11",
	"test [r10], r10",
	"ret",
);

// ARM64: X15 is the frame's size divided by 16, and nothing but X16 and X17 may change.
#[cfg(target_arch = "aarch64")]
global_asm!(
	".globl __chkstk",
	"__chkstk:",
	"lsl x16, x15, #4",
	"mov x17, sp",
	"2:",
	"sub x17, x17, #4096",
	"subs x16, x16, #4096",
	"ldr xzr, [x17]",
	"b.gt 2b",
	"ret",
);
