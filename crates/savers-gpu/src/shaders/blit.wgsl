// Present pass: the finished frame enlarged (or reduced) to the window's surface through a bilinear filter.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The target's size in pixels.
	size: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let uv = in.frag.xy / params.size.xy;
	return vec4<f32>(textureSampleLevel(source, source_sampler, uv, 0.0).rgb, 1.0);
}
