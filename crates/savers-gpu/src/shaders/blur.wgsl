// Finish pass 2: a box blur along one axis with the edges clamped, as the CPU's `blur_rows` and `blur_cols`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The step along the blur (x, y) and the radius in texels (z).
	axis_radius: vec4<i32>,
	// The texture size (x, y).
	size: vec4<i32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var source: texture_2d<f32>;

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let at = vec2<i32>(floor(in.frag.xy));
	let axis = params.axis_radius.xy;
	let radius = params.axis_radius.z;
	let last = params.size.xy - vec2<i32>(1, 1);
	var sum = vec4<f32>(0.0);
	for (var i = -radius; i <= radius; i = i + 1) {
		sum = sum + textureLoad(source, clamp(at + axis * i, vec2<i32>(0, 0), last), 0);
	}
	return sum / f32(2 * radius + 1);
}
