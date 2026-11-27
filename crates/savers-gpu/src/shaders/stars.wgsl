// Pass 2: the stars, one instance per lit pixel, screen-blended over the nebula.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The render size in pixels.
	size: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;

struct StarIn {
	// The pixel the star lights, as integers.
	@location(0) place: vec2<f32>,
	// The colour to add, already scaled by the star's brightness.
	@location(1) colour: vec3<f32>,
}

struct StarOut {
	@builtin(position) pos: vec4<f32>,
	@location(0) colour: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32, star: StarIn) -> StarOut {
	var quad = array<vec2<f32>, 6>(
		vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
		vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
	);
	let p = star.place + quad[index];
	var out: StarOut;
	out.pos = vec4<f32>(p.x / params.size.x * 2.0 - 1.0, 1.0 - p.y / params.size.y * 2.0, 0.0, 1.0);
	out.colour = star.colour;
	return out;
}

// The pipeline blends with `src + dst * (1 - src)`, which is the screen blend.
@fragment
fn fs_main(in: StarOut) -> @location(0) vec4<f32> {
	return vec4<f32>(in.colour, 1.0);
}
