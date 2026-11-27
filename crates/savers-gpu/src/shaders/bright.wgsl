// Finish pass 1: the bright pass at a quarter (or an eighth) of the render size: two samples per block, the part of the luma above
// the threshold kept in colour for the bloom, and its square for the streaks.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The render size (x, y), the block step (z) and half of it (w).
	frame_step: vec4<i32>,
	// The threshold (x) and the span above it (y), in levels.
	threshold: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var scene: texture_2d<f32>;

struct Out {
	@location(0) bloom: vec4<f32>,
	@location(1) streak: vec4<f32>,
}

@fragment
fn fs_main(in: FsIn) -> Out {
	let q = vec2<i32>(floor(in.frag.xy));
	let size = params.frame_step.xy;
	let step = params.frame_step.z;
	let half_step = params.frame_step.w;
	let quarter = half_step / 2;
	let last = size - vec2<i32>(1, 1);
	let p0 = min(q * step + vec2<i32>(quarter, quarter), last);
	let p1 = min(q * step + vec2<i32>(half_step + quarter, half_step + quarter), last);
	let c0 = textureLoad(scene, p0, 0).rgb;
	let c1 = textureLoad(scene, p1, 0).rgb;
	// Sums of two samples in levels, 0 to 510.
	let sum = floor((c0 + c1) * 255.0 + 0.5);
	let luma = floor((sum.r * 77.0 + sum.g * 150.0 + sum.b * 29.0) / 512.0);
	let over = max(luma - params.threshold.x, 0.0);
	let span = params.threshold.y;
	var out: Out;
	out.bloom = vec4<f32>(sum * over / span, 0.0);
	out.streak = vec4<f32>(over * over / span * 2.0, 0.0, 0.0, 0.0);
	return out;
}
