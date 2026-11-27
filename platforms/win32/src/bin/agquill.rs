// AGQUILL.SCR: the Quill Ghostwriter screensaver, the Easter egg that ships only with Quill (SAV-D20, SAV-D22).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

use savers_core::audio::ghostwriter::GhostwriterScore;
use savers_core::scenes::ghostwriter::Ghostwriter;
use savers_core::settings::SaverKind;

// Ghostwriter's own constructor, so only that scene is linked.
afterglow_win32::screensaver_main!(
	SaverKind::Ghostwriter,
	|settings, seed, w, h| Box::new(Ghostwriter::new(settings, seed, w, h)),
	|settings, seed, rate| Some(Box::new(GhostwriterScore::new(settings, seed, rate)))
);
