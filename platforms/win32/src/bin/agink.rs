// AGINK.SCR: the Afterglow Ink screensaver.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use savers_core::scenes::ink::Ink;
use savers_core::settings::SaverKind;

// Ink's own constructor, so only that scene is linked.
afterglow_win32::screensaver_main!(SaverKind::Ink, |settings, seed, w, h| Box::new(Ink::new(
	settings, seed, w, h
)));
