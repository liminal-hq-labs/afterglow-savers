// Finish pass 5: the enlargement to the output size with the vignette, the fade, the letterbox and the grain in one go.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Params {
	// The output size (x, y), the first letterbox row that is picture (z) and the first bar row below it (w).
	size_rows: vec4<f32>,
	// The vignette strength 0 to 1 (x), the fade 0 to 1 (y), the lens aspect factor min(aspect, 1.6) / 1.6 (z).
	look: vec4<f32>,
	// The grain amplitude in levels (x) and the frame number (y).
	grain: vec4<u32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var frame: texture_2d<f32>;
@group(0) @binding(2) var frame_sampler: sampler;

// The vignette's falloff for a squared, weighted distance.
fn vfall(d: f32) -> f32 {
	let f = 1.0 - d;
	return saturate1(f * f);
}

// The same integer hash as the core's `hash32`.
fn hash32(input: u32) -> u32 {
	var x = input;
	x = x ^ (x >> 16u);
	x = x * 0x7FEB352Du;
	x = x ^ (x >> 15u);
	x = x * 0x846CA68Bu;
	x = x ^ (x >> 16u);
	return x;
}

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let size = params.size_rows.xy;
	let row = floor(in.frag.y);
	if (row < params.size_rows.z || row >= params.size_rows.w) {
		return vec4<f32>(0.0, 0.0, 0.0, 1.0);
	}
	var colour = textureSampleLevel(frame, frame_sampler, in.frag.xy / size, 0.0).rgb;
	let v = params.look.x;
	let dx = in.frag.x / size.x * 2.0 - 1.0;
	let dy = in.frag.y / size.y * 2.0 - 1.0;
	let k = vfall(dx * dx * v * 0.55 * params.look.z) * vfall(dy * dy * v * 0.7) * params.look.y;
	colour = colour * k;
	let amp = i32(params.grain.x);
	if (amp != 0) {
		// Triangular grain: two uniform bytes from one hash, per pixel and frame, in about [-255, 255].
		let pixel = vec2<u32>(floor(in.frag.xy));
		let h = hash32(pixel.x ^ hash32(pixel.y ^ hash32(params.grain.y * 0x9E3779B9u)));
		let triangle = i32(h & 255u) + i32((h >> 8u) & 255u) - 255;
		let level = triangle * amp / 256;
		// Strongest in the mid-tones and lighter in the blacks: the weight is read from green.
		let green = floor(colour.g * 255.0 + 0.5);
		let mid = floor(green * (255.0 - green) / 64.0);
		let weight = 96.0 + min(mid, 159.0);
		let shift = floor(f32(level) * weight / 256.0);
		colour = clamp(colour + vec3<f32>(shift / 255.0), vec3<f32>(0.0), vec3<f32>(1.0));
	}
	return vec4<f32>(colour, 1.0);
}
