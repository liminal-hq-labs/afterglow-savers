// The Win32 shell of the Afterglow screensavers: the command line, the saver and preview windows, the registry and the settings dialog,
// as a library that each `.scr` program in `src/bin` calls with its own scene.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `no_std`, with no C runtime. The parts that need no Windows (`ansi`, `cmdline`, `template`) build on the host so it can test them;
//! everything that calls Windows is compiled only for the Win32 targets.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod ansi;
pub mod cmdline;
pub mod template;

#[cfg(all(windows, not(test)))]
mod clock;
#[cfg(all(windows, not(test)))]
mod dialog;
#[cfg(all(windows, not(test)))]
mod registry;
#[cfg(all(windows, not(test)))]
mod rt;
#[cfg(all(windows, not(test), not(target_arch = "x86")))]
mod rt_probe;
#[cfg(all(windows, not(test), target_arch = "x86"))]
mod rt_x86;
#[cfg(all(windows, not(test)))]
mod saver;
#[cfg(all(windows, not(test)))]
mod sys;

#[cfg(all(windows, not(test)))]
pub use saver::{MakeScene, run};
#[cfg(all(windows, not(test)))]
pub use sys::ExitProcess;

/// Each saver the Collection can play, with its settings read from its own registry key (SAV-D8).
#[cfg(all(windows, not(test)))]
pub fn member_settings() -> alloc::vec::Vec<savers_core::settings::Settings> {
	use savers_core::scenes::collection::MEMBERS;
	use savers_core::settings::Settings;
	MEMBERS
		.iter()
		.map(|&kind| Settings::load(kind, &registry::RegStore::open(kind)))
		.collect()
}

/// The entry point of a `.scr` program: runs the saver and exits with its code. `$kind` is its `SaverKind`, `$make` its scene maker.
#[cfg(all(windows, not(test)))]
#[macro_export]
macro_rules! screensaver_main {
	($kind:expr, $make:expr) => {
		#[unsafe(no_mangle)]
		pub extern "C" fn start() -> ! {
			let code = $crate::run($kind, $make);
			// SAFETY: ends the process.
			unsafe { $crate::ExitProcess(code) }
		}
	};
}
