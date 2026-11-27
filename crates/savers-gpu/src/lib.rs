// Afterglow Twilight on the graphics card: a wgpu renderer drawn from the same data as the CPU scene.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The CPU scene in `savers-core` stays the reference (SAV-D25). This crate takes its camera, maps, ramps and parameters from
//! [`savers_core::twilight_shared`], uploads the maps once as textures, and draws each frame in a handful of passes:
//!
//! | Pass | Shader | Reads | Writes |
//! | --- | --- | --- | --- |
//! | background | `background.wgsl` | nebula | scene A |
//! | stars | `stars.wgsl` (instances) | star buffer | scene A (screen blend) |
//! | scene | `scene.wgsl` | scene A, ground, clouds, lights, ramps, craters | scene B |
//! | bright | `bright.wgsl` | scene B | bloom, streak (coarse size) |
//! | blur | `blur.wgsl` (4 bloom, 4 streak) | bloom, streak | bloom, streak |
//! | pack | `pack.wgsl` | bloom, streak | glow (8-bit) |
//! | lay | `lay.wgsl` | scene B, glow | frame |
//! | output | `output.wgsl` | frame | output (output size) |
//!
//! The last five are the finishing chain of `savers_core::post`, in [`finish::Finish`], which also takes any frame a caller
//! uploads. `docs/gpu-twilight.md` has the design and the parity budget.

pub mod compare;
pub mod context;
pub mod finish;
pub mod present;
pub mod readback;
pub mod twilight;

pub use context::{Gpu, GpuError, adapter_names};
pub use finish::Finish;
pub use present::Presenter;
pub use twilight::{TwilightRenderer, finish_params, shared_for_gpu};
