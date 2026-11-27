// The scenes, and a builder for any of them by kind.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A single-scene `.scr` calls its scene's constructor directly, so only that scene is linked; the Collection and the preview tool
//! use [`build`], which links them all.

use alloc::boxed::Box;

use crate::scene::Scene;
use crate::settings::{SaverKind, Settings};

pub mod ink;

/// Makes the scene for a saver at a render size.
pub fn build(kind: SaverKind, settings: &Settings, seed: u32, w: usize, h: usize) -> Box<dyn Scene> {
	let _ = kind;
	Box::new(ink::Ink::new(settings, seed, w, h))
}
