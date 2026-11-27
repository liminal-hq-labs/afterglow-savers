// AGCRACK.SCR: the Afterglow Cracktro screensaver.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use savers_core::scenes::cracktro::Cracktro;
use savers_core::settings::SaverKind;

// The Cracktro's own constructor, so only that scene is linked.
afterglow_win32::screensaver_main!(SaverKind::Cracktro, |settings, seed, w, h| Box::new(Cracktro::new(
	settings, seed, w, h
)));
