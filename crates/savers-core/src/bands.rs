// Row bands: how a pass over rows is shared between threads without changing a byte of the result.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A platform that has threads implements [`Bands`]; the core calls it for every pass whose rows (or columns) are independent, and
//! the default [`Serial`] runs the whole range on the calling thread. The bands write disjoint cells and read only inputs that no
//! band changes during the pass, so the frame is the same byte for byte with any number of bands, in any order (SAV-D51).
//!
//! Writing through the bands needs more than `&mut`, so [`Shared`] wraps a slice for the length of a pass and hands out sub-slices
//! and single cells by index. Each use states why its ranges are disjoint.

use core::marker::PhantomData;
use core::ops::Range;

/// Runs a job over `0..rows`, as disjoint ranges that together cover it, in parallel or not, and returns when all are done.
pub trait Bands: Sync {
	fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync));

	/// How many threads work on a pass (1 when serial); used for the preview tool's report.
	fn threads(&self) -> usize {
		1
	}
}

/// One range, on the calling thread.
pub struct Serial;

impl Bands for Serial {
	fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync)) {
		if rows > 0 {
			job(0..rows);
		}
	}
}

/// The serial default every saver starts with.
pub static SERIAL: Serial = Serial;

/// Chunk `index` of `count` over `rows`: even to within one row, in order, covering `0..rows` with no gap or overlap.
pub fn chunk(rows: usize, count: usize, index: usize) -> Range<usize> {
	let count = count.max(1);
	(rows * index / count)..(rows * (index + 1) / count)
}

/// A slice that several bands write at once, each in its own part. Created from a `&mut` slice, so nothing else touches it while the
/// pass runs.
pub struct Shared<'a, T> {
	ptr: *mut T,
	len: usize,
	_slice: PhantomData<&'a mut [T]>,
}

// SAFETY: a `Shared` only hands out parts through its `unsafe` methods, whose callers promise that no two threads touch the same
// cell; with that, sharing it is no different from sharing disjoint `&mut` sub-slices, which needs `T: Send`.
unsafe impl<T: Send> Sync for Shared<'_, T> {}
unsafe impl<T: Send> Send for Shared<'_, T> {}

impl<'a, T> Shared<'a, T> {
	pub fn new(slice: &'a mut [T]) -> Shared<'a, T> {
		Shared {
			ptr: slice.as_mut_ptr(),
			len: slice.len(),
			_slice: PhantomData,
		}
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// The cells `range` as a mutable slice.
	///
	/// # Safety
	/// No other thread may read or write any cell of `range` while the slice is alive.
	#[allow(clippy::mut_from_ref)]
	pub unsafe fn slice(&self, range: Range<usize>) -> &mut [T] {
		assert!(range.start <= range.end && range.end <= self.len);
		// SAFETY: in bounds, and exclusive by the caller's promise.
		unsafe { core::slice::from_raw_parts_mut(self.ptr.add(range.start), range.end - range.start) }
	}
}

impl<T: Copy> Shared<'_, T> {
	/// Reads cell `i`.
	///
	/// # Safety
	/// No other thread may write cell `i` during the pass.
	#[inline(always)]
	pub unsafe fn get(&self, i: usize) -> T {
		assert!(i < self.len);
		// SAFETY: in bounds; not written elsewhere by the caller's promise.
		unsafe { *self.ptr.add(i) }
	}

	/// Writes cell `i`.
	///
	/// # Safety
	/// No other thread may read or write cell `i` during the pass.
	#[inline(always)]
	pub unsafe fn set(&self, i: usize, value: T) {
		assert!(i < self.len);
		// SAFETY: in bounds; exclusive by the caller's promise.
		unsafe { *self.ptr.add(i) = value }
	}
}

/// A band of a frame's rows `y0..y1`, as a slice of those rows alone.
pub struct Rows<'a> {
	pub px: &'a mut [u32],
	pub w: usize,
	pub y0: usize,
	pub y1: usize,
}

impl Rows<'_> {
	/// Row `y` (a frame row, `y0 <= y < y1`).
	#[inline(always)]
	pub fn row(&mut self, y: usize) -> &mut [u32] {
		let base = (y - self.y0) * self.w;
		&mut self.px[base..base + self.w]
	}

	/// The pixel at `(x, y)`; `y` must be in the band.
	#[inline(always)]
	pub fn get(&self, x: usize, y: usize) -> u32 {
		self.px[(y - self.y0) * self.w + x]
	}

	#[inline(always)]
	pub fn set(&mut self, x: usize, y: usize, c: u32) {
		self.px[(y - self.y0) * self.w + x] = c;
	}

	/// Sets a pixel when it is inside the frame's width and this band's rows.
	#[inline]
	pub fn put(&mut self, x: i32, y: i32, c: u32) {
		if x >= 0 && (x as usize) < self.w && y >= self.y0 as i32 && y < self.y1 as i32 {
			let i = (y as usize - self.y0) * self.w + x as usize;
			self.px[i] = c;
		}
	}

	/// The part of `lo..hi` that lies in this band.
	#[inline(always)]
	pub fn clip(&self, lo: i32, hi: i32) -> Range<i32> {
		lo.max(self.y0 as i32)..hi.min(self.y1 as i32)
	}
}

/// Runs `job` over bands of the rows of a `w`-wide pixel buffer.
pub fn rows(bands: &dyn Bands, px: &mut [u32], w: usize, job: &(dyn Fn(&mut Rows) + Sync)) {
	let h = px.len() / w.max(1);
	let shared = Shared::new(px);
	bands.run(h, &|range: Range<usize>| {
		// SAFETY: the bands cover disjoint row ranges, so these slices do not overlap.
		let slice = unsafe { shared.slice(range.start * w..range.end * w) };
		job(&mut Rows {
			px: slice,
			w,
			y0: range.start,
			y1: range.end,
		});
	});
}

/// Copies `src` into `dst` (resized to match) in bands of rows of width `w`.
pub fn copy(bands: &dyn Bands, dst: &mut alloc::vec::Vec<u32>, src: &[u32], w: usize) {
	dst.resize(src.len(), 0);
	rows(bands, dst, w, &|band| {
		band.px.copy_from_slice(&src[band.y0 * w..band.y1 * w]);
	});
}
