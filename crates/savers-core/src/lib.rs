// The Afterglow screensavers' portable core: a software renderer, the scenes, the finishing chain and the settings.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `no_std` with `alloc`, no libm and nothing about any platform (SAV-D3, SAV-D5). A platform makes a scene for a render size,
//! hands it to a [`saver::Saver`], and shows each frame the saver returns.

#![no_std]
// Fixed-point code is shifts after products everywhere (`a * b >> 8`); spelling each with parentheses hides the arithmetic.
#![allow(clippy::precedence)]

extern crate alloc;

pub mod bench;
pub mod colour;
pub mod font;
pub mod frame;
pub mod math;
pub mod noise;
pub mod palette;
pub mod post;
pub mod rng;
pub mod saver;
pub mod scene;
pub mod scenes;
pub mod settings;
