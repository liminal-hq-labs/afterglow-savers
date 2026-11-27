// Finish pass 4: the glow laid over the frame through a bilinear enlargement, screen-blended.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The render size.
	size: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var glow: texture_2d<f32>;
@group(0) @binding(3) var glow_sampler: sampler;

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let at = vec2<i32>(floor(in.frag.xy));
	let base = textureLoad(scene, at, 0).rgb;
	// The CPU maps output pixel x to coarse coordinate x * (coarse / render) - 0.5 with texel centres on integers, which is the
	// texture coordinate x / render: the pixel's index, not its centre.
	let uv = (in.frag.xy - 0.5) / params.size.xy;
	let glow_colour = textureSampleLevel(glow, glow_sampler, uv, 0.0).rgb;
	return vec4<f32>(screen_blend(base, glow_colour), 1.0);
}
