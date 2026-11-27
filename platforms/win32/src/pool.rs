// The x86-64 build's worker pool: threads that share a frame's passes over rows (SAV-D51). The Pentium build has none.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A pass is cut into more chunks than there are threads, and the calling thread and the workers take chunks from one counter until
//! none are left, so a thread that is slow or busy simply takes fewer. Each worker waits on its own auto-reset event; the last one to
//! finish a pass sets the `done` event the caller waits on. The workers run at the idle priority, so a busy machine is not slowed by
//! them; if a pass has waited [`PATIENCE_MS`] for a worker that the scheduler has not given a turn, the workers are raised to the
//! normal priority for the rest of that pass, so a loaded machine cannot freeze the frame.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ops::Range;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use savers_core::bands::{Bands, chunk};

use crate::sys::{self, Handle};

/// The most workers, so the most threads is eight with the caller's.
pub const MAX_WORKERS: usize = 7;
/// Chunks a pass is cut into for each thread: enough to even out rows that cost different amounts, few enough to stay cheap.
const CHUNKS_PER_THREAD: usize = 4;
/// How long a pass waits for its workers at their idle priority before raising them.
const PATIENCE_MS: u32 = 12;
/// A worker's stack: the scenes' per-pixel loops keep little on it.
const STACK_BYTES: usize = 512 * 1024;

type Job = *const (dyn Fn(Range<usize>) + Sync);

struct Pool {
	go: Vec<Handle>,
	/// How many of `go` have a thread behind them.
	active: AtomicUsize,
	/// The workers' handles, filled in by `start` before any pass runs and read after it by the caller alone.
	threads: UnsafeCell<Vec<Handle>>,
	done: Handle,
	/// The pass: written by the caller before the workers are woken (an event is a full barrier), read by them after.
	job: UnsafeCell<Option<Job>>,
	rows: AtomicUsize,
	chunks: AtomicUsize,
	next: AtomicUsize,
	pending: AtomicUsize,
	quit: AtomicBool,
}

// SAFETY: `job` is written only by the calling thread while no worker is running a pass, and read by the workers only between the
// event that wakes them and the one they set when done; the rest is atomics and handles.
unsafe impl Sync for Pool {}

struct WorkerArg {
	pool: &'static Pool,
	index: usize,
}

impl Pool {
	/// Takes chunks until none are left.
	fn work(&self, job: Job, rows: usize, chunks: usize) {
		loop {
			let i = self.next.fetch_add(1, Ordering::Relaxed);
			if i >= chunks {
				return;
			}
			let range = chunk(rows, chunks, i);
			if !range.is_empty() {
				// SAFETY: `run` keeps the closure alive until `pending` is zero, which is after every worker has left this loop.
				unsafe { (*job)(range) };
			}
		}
	}

	fn set_priority(&self, priority: i32) {
		// SAFETY: `start` is done filling the list, and only the caller reads it.
		for &thread in unsafe { &*self.threads.get() } {
			// SAFETY: the handles are the pool's own live threads.
			unsafe { sys::SetThreadPriority(thread, priority) };
		}
	}
}

unsafe extern "system" fn worker(argument: *mut c_void) -> u32 {
	// SAFETY: the argument is the leaked `WorkerArg` that `start` made for this thread.
	let arg = unsafe { &*(argument as *const WorkerArg) };
	let pool = arg.pool;
	loop {
		// SAFETY: the event is the pool's own.
		unsafe { sys::WaitForSingleObject(pool.go[arg.index], sys::INFINITE) };
		if pool.quit.load(Ordering::Acquire) {
			return 0;
		}
		// SAFETY: the caller wrote the job before setting this worker's event and keeps it until `pending` reaches zero.
		let job = unsafe { (*pool.job.get()).expect("a pass has a job") };
		pool.work(job, pool.rows.load(Ordering::Relaxed), pool.chunks.load(Ordering::Relaxed));
		if pool.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
			// SAFETY: the event is the pool's own.
			unsafe { sys::SetEvent(pool.done) };
		}
	}
}

impl Bands for Pool {
	fn run(&self, rows: usize, job: &(dyn Fn(Range<usize>) + Sync)) {
		if rows == 0 {
			return;
		}
		let active = self.active.load(Ordering::Relaxed);
		if active == 0 || rows < 2 {
			job(0..rows);
			return;
		}
		// SAFETY: only the pointer's lifetime is erased; `job` outlives the pass because this function waits for the workers.
		let ptr: Job = unsafe { core::mem::transmute::<*const (dyn Fn(Range<usize>) + Sync + '_), Job>(job) };
		let chunks = (active + 1) * CHUNKS_PER_THREAD;
		// SAFETY: no worker is running (the last pass ended with `pending` at zero), so nothing else reads `job`.
		unsafe { *self.job.get() = Some(ptr) };
		self.rows.store(rows, Ordering::Relaxed);
		self.chunks.store(chunks, Ordering::Relaxed);
		self.next.store(0, Ordering::Relaxed);
		self.pending.store(active, Ordering::Release);
		for &event in &self.go[..active] {
			// SAFETY: the events are the pool's own.
			unsafe { sys::SetEvent(event) };
		}
		self.work(ptr, rows, chunks);
		let mut raised = false;
		while self.pending.load(Ordering::Acquire) != 0 {
			// SAFETY: the event is the pool's own.
			let waited = unsafe { sys::WaitForSingleObject(self.done, PATIENCE_MS) };
			if waited != sys::WAIT_OBJECT_0 && !raised && self.pending.load(Ordering::Acquire) != 0 {
				self.set_priority(sys::THREAD_PRIORITY_NORMAL);
				raised = true;
			}
		}
		if raised {
			self.set_priority(sys::THREAD_PRIORITY_IDLE);
		}
	}

	fn threads(&self) -> usize {
		self.active.load(Ordering::Relaxed) + 1
	}
}

/// How many processors the machine has.
fn processors() -> usize {
	// SAFETY: a valid out pointer to a zeroed `SystemInfo`.
	let mut info: sys::SystemInfo = unsafe { core::mem::zeroed() };
	unsafe { sys::GetSystemInfo(&mut info) };
	(info.number_of_processors as usize).max(1)
}

/// The number of threads wanted in all, the caller's included: one fewer than the processors, plus the caller, up to eight. A
/// `Threads` value in the registry overrides it: 1 means none, 2 to 8 that many.
pub fn wanted(setting: Option<u32>) -> usize {
	match setting {
		Some(n) if n >= 1 => (n as usize).min(MAX_WORKERS + 1),
		_ => processors().min(MAX_WORKERS + 1),
	}
}

/// Starts a pool of `threads` threads in all (the caller counts as one) that lasts for the run. `None` when one thread is wanted or
/// the system will not make the pool.
pub fn start(threads: usize) -> Option<&'static dyn Bands> {
	let workers = threads.saturating_sub(1).min(MAX_WORKERS);
	if workers == 0 {
		return None;
	}
	// SAFETY: plain calls with null attributes and names; a failed one gives a null handle that is checked.
	let done = unsafe { sys::CreateEventA(core::ptr::null(), 0, 0, core::ptr::null()) };
	if done.is_null() {
		return None;
	}
	let mut go = Vec::with_capacity(workers);
	for _ in 0..workers {
		// SAFETY: as above.
		let event = unsafe { sys::CreateEventA(core::ptr::null(), 0, 0, core::ptr::null()) };
		if event.is_null() {
			return None;
		}
		go.push(event);
	}
	let pool: &'static Pool = Box::leak(Box::new(Pool {
		go,
		active: AtomicUsize::new(0),
		threads: UnsafeCell::new(Vec::with_capacity(workers)),
		done,
		job: UnsafeCell::new(None),
		rows: AtomicUsize::new(0),
		chunks: AtomicUsize::new(0),
		next: AtomicUsize::new(0),
		pending: AtomicUsize::new(0),
		quit: AtomicBool::new(false),
	}));
	for index in 0..workers {
		let arg = Box::leak(Box::new(WorkerArg { pool, index }));
		let mut id = 0u32;
		// SAFETY: the start routine and its leaked argument live for the process.
		let thread = unsafe {
			sys::CreateThread(
				core::ptr::null(),
				STACK_BYTES,
				worker,
				arg as *mut WorkerArg as *mut c_void,
				0,
				&mut id,
			)
		};
		if thread.is_null() {
			// The workers made so far serve; a thread the system would not make is not needed.
			break;
		}
		// SAFETY: only this function touches `threads` until `active` is set, and no pass runs yet.
		unsafe {
			sys::SetThreadPriority(thread, sys::THREAD_PRIORITY_IDLE);
			(*pool.threads.get()).push(thread);
		}
		pool.active.store(index + 1, Ordering::Relaxed);
	}
	if pool.active.load(Ordering::Relaxed) == 0 {
		return None;
	}
	Some(pool)
}
