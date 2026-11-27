// Pass 1: the nebula, panned against the planet's drift, sampled as the CPU scene does (pixel index, not pixel centre).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The nebula's width and height in texels.
	nebula_size: vec4<f32>,
	// The sample origin in texels (x, y) and the step per output pixel (z, w).
	origin_step: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var nebula: texture_2d<f32>;
@group(0) @binding(2) var nebula_sampler: sampler;

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let pixel = floor(in.frag.xy);
	let at = params.origin_step.xy + pixel * params.origin_step.zw;
	let uv = (at + 0.5) / params.nebula_size.xy;
	return vec4<f32>(textureSampleLevel(nebula, nebula_sampler, uv, 0.0).rgb, 1.0);
}
