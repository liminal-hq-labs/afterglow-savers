// AGTWILIT.SCR: the Afterglow Twilight screensaver.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use savers_core::audio::twilight::TwilightScore;
use savers_core::scenes::twilight::Twilight;
use savers_core::settings::SaverKind;

// Twilight's own constructor, so only that scene is linked.
afterglow_win32::screensaver_main!(
	SaverKind::Twilight,
	|settings, seed, w, h| Box::new(Twilight::new(settings, seed, w, h)),
	|settings, seed, rate| Some(Box::new(TwilightScore::new(settings, seed, rate)))
);
