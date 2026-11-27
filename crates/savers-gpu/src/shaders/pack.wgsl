// Finish pass 3: the bloom in its own colours and the streaks in their tint, packed into one 8-bit glow at the coarse size.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The bloom gain (x, `bloom * 3`) and the streak gain (y, `streaks * 5`).
	gains: vec4<f32>,
	// The streak tint in levels, 0 to 255.
	tint: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var bloom: texture_2d<f32>;
@group(0) @binding(2) var streak: texture_2d<f32>;

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let at = vec2<i32>(floor(in.frag.xy));
	let red_green_blue = textureLoad(bloom, at, 0).rgb;
	let streak_level = textureLoad(streak, at, 0).r * params.gains.y / 100.0;
	let glow = red_green_blue * params.gains.x / 200.0 + streak_level * params.tint.rgb / 255.0;
	return vec4<f32>(clamp(glow, vec3<f32>(0.0), vec3<f32>(255.0)) / 255.0, 1.0);
}
