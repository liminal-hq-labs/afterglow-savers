// AGMAZE.SCR: the Afterglow Maze screensaver.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use savers_core::scenes::maze::MazeScene;
use savers_core::settings::SaverKind;

// The Maze's own constructor, so only that scene is linked.
afterglow_win32::screensaver_main!(SaverKind::Maze, |settings, seed, w, h| Box::new(MazeScene::new(
	settings, seed, w, h
)));
