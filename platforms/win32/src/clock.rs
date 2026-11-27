// Time: the performance counter where the machine has one, else the millisecond tick count.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::sys;

pub struct Clock {
	frequency: i64,
}

impl Clock {
	pub fn new() -> Clock {
		let mut frequency = 0i64;
		// SAFETY: a valid out pointer.
		let ok = unsafe { sys::QueryPerformanceFrequency(&mut frequency) };
		Clock {
			frequency: if ok != 0 && frequency > 0 { frequency } else { 0 },
		}
	}

	/// Seconds since an arbitrary start; it only ever grows (the tick count wraps after 49 days, which a frame loop never sees).
	pub fn seconds(&self) -> f64 {
		if self.frequency > 0 {
			let mut count = 0i64;
			// SAFETY: a valid out pointer.
			unsafe { sys::QueryPerformanceCounter(&mut count) };
			count as f64 / self.frequency as f64
		} else {
			// SAFETY: no arguments.
			f64::from(unsafe { sys::GetTickCount() }) / 1000.0
		}
	}
}
