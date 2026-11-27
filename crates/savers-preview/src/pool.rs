// A pool of worker threads that shares a pass over rows, for timing the savers with threads on the host.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The same scheme as the Win32 shell's pool (SAV-D51): a pass is cut into more chunks than threads, and the caller and the workers
//! take chunks from one counter until none are left, so a thread that is slow or busy simply takes fewer.

use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use savers_core::bands::{Bands, chunk};

/// How many chunks a pass is cut into for each thread.
const CHUNKS_PER_THREAD: usize = 4;

type Job = *const (dyn Fn(Range<usize>) + Sync);

struct State {
	generation: u64,
	job: Option<Job>,
	rows: usize,
	chunks: usize,
	pending: usize,
	quit: bool,
}

// SAFETY: the job pointer is only read while `run` is waiting for the workers, which keeps what it points to alive.
unsafe impl Send for State {}

struct Inner {
	state: Mutex<State>,
	wake: Condvar,
	done: Condvar,
	next: AtomicUsize,
}

pub struct Pool {
	inner: Arc<Inner>,
	workers: usize,
	handles: Vec<std::thread::JoinHandle<()>>,
}

fn work(inner: &Inner, job: Job, rows: usize, chunks: usize) {
	loop {
		let i = inner.next.fetch_add(1, Ordering::Relaxed);
		if i >= chunks {
			break;
		}
		let range = chunk(rows, chunks, i);
		if !range.is_empty() {
			// SAFETY: `run` keeps the closure alive until every worker has finished with it.
			unsafe { (*job)(range) };
		}
	}
}

impl Pool {
	/// A pool of `threads` threads in all, the caller's included.
	pub fn new(threads: usize) -> Pool {
		let workers = threads.max(1) - 1;
		let inner = Arc::new(Inner {
			state: Mutex::new(State {
				generation: 0,
				job: None,
				rows: 0,
				chunks: 0,
				pending: 0,
				quit: false,
			}),
			wake: Condvar::new(),
			done: Condvar::new(),
			next: AtomicUsize::new(0),
		});
		let handles = (0..workers)
			.map(|_| {
				let inner = inner.clone();
				std::thread::spawn(move || {
					let mut seen = 0;
					loop {
						let (job, rows, chunks) = {
							let mut st = inner.state.lock().unwrap();
							while st.generation == seen && !st.quit {
								st = inner.wake.wait(st).unwrap();
							}
							if st.quit {
								return;
							}
							seen = st.generation;
							(st.job.unwrap(), st.rows, st.chunks)
						};
						work(&inner, job, rows, chunks);
						let mut st = inner.state.lock().unwrap();
						st.pending -= 1;
						if st.pending == 0 {
							inner.done.notify_one();
						}
					}
				})
			})
			.collect();
		Pool { inner, workers, handles }
	}
}

impl Drop for Pool {
	fn drop(&mut self) {
		self.inner.state.lock().unwrap().quit = true;
		self.inner.wake.notify_all();
		for h in self.handles.drain(..) {
			let _ = h.join();
		}
	}
}

impl Bands for Pool {
	fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync)) {
		if rows == 0 {
			return;
		}
		if self.workers == 0 || rows < 2 {
			job(0..rows);
			return;
		}
		let chunks = (self.workers + 1) * CHUNKS_PER_THREAD;
		// SAFETY: the pointer is used only until every worker has reported in below, while `job` is alive.
		let ptr: Job = unsafe { std::mem::transmute::<*const (dyn Fn(Range<usize>) + Sync + '_), Job>(job) };
		self.inner.next.store(0, Ordering::Relaxed);
		{
			let mut st = self.inner.state.lock().unwrap();
			st.job = Some(ptr);
			st.rows = rows;
			st.chunks = chunks;
			st.pending = self.workers;
			st.generation += 1;
		}
		self.inner.wake.notify_all();
		work(&self.inner, ptr, rows, chunks);
		let mut st = self.inner.state.lock().unwrap();
		while st.pending > 0 {
			st = self.inner.done.wait(st).unwrap();
		}
		st.job = None;
	}

	fn threads(&self) -> usize {
		self.workers + 1
	}
}
