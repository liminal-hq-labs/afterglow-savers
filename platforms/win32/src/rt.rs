// What a program with no C library has to supply: the panic handler, the heap, and on the Pentium build the helpers the 32-bit MSVC ABI
// expects (`rt_x86.rs`). The memory routines (`memcpy` and the rest) come from `compiler_builtins`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

use crate::sys;

/// The exit code of a crash, as on every Afterglow platform.
const EXIT_CRASH: u32 = 70;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
	// SAFETY: ends the process.
	unsafe { sys::ExitProcess(EXIT_CRASH) }
}

/// Tells the MSVC linker that floating point is in use somewhere.
#[unsafe(no_mangle)]
pub static _fltused: i32 = 0;

/// The process heap (SAV-D3). `HeapAlloc` returns 8-byte alignment (16 on 64-bit); a stricter request is over-allocated and aligned by
/// hand, with the block's start kept just below the aligned address.
pub struct ProcessHeap;

#[cfg(target_pointer_width = "64")]
const NATURAL: usize = 16;
#[cfg(not(target_pointer_width = "64"))]
const NATURAL: usize = 8;

// SAFETY: every method forwards to the process heap, which is thread safe and returns blocks of the size asked for.
unsafe impl GlobalAlloc for ProcessHeap {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// SAFETY: the process heap is always valid.
		unsafe { self.allocate(layout, 0) }
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		// SAFETY: as `alloc`.
		unsafe { self.allocate(layout, sys::HEAP_ZERO_MEMORY) }
	}

	unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
		// SAFETY: the pointer came from `allocate`, so a stricter alignment kept the block's start just before it.
		unsafe {
			let block = if layout.align() > NATURAL {
				*(pointer as *mut *mut c_void).sub(1)
			} else {
				pointer as *mut c_void
			};
			sys::HeapFree(sys::GetProcessHeap(), 0, block);
		}
	}

	unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		if layout.align() <= NATURAL {
			// SAFETY: a natural-alignment block is the heap's own.
			return unsafe { sys::HeapReAlloc(sys::GetProcessHeap(), 0, pointer as *mut c_void, new_size) as *mut u8 };
		}
		// SAFETY: the new layout has the same alignment and a size the caller vouches for.
		unsafe {
			let fresh = self.alloc(Layout::from_size_align_unchecked(new_size, layout.align()));
			if !fresh.is_null() {
				core::ptr::copy_nonoverlapping(pointer, fresh, layout.size().min(new_size));
				self.dealloc(pointer, layout);
			}
			fresh
		}
	}
}

impl ProcessHeap {
	unsafe fn allocate(&self, layout: Layout, flags: u32) -> *mut u8 {
		// SAFETY: the process heap is always valid; the arithmetic stays inside the block asked for.
		unsafe {
			let heap = sys::GetProcessHeap();
			if layout.align() <= NATURAL {
				return sys::HeapAlloc(heap, flags, layout.size()) as *mut u8;
			}
			let extra = layout.align() + core::mem::size_of::<usize>();
			let block = sys::HeapAlloc(heap, flags, layout.size() + extra) as *mut u8;
			if block.is_null() {
				return block;
			}
			let start = block as usize + core::mem::size_of::<usize>();
			let aligned = (start + layout.align() - 1) & !(layout.align() - 1);
			*(aligned as *mut *mut c_void).sub(1) = block as *mut c_void;
			aligned as *mut u8
		}
	}
}

#[global_allocator]
static HEAP: ProcessHeap = ProcessHeap;
