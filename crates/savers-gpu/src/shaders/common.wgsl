// Shared by every pass: the full-screen triangle and the screen blend.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct FsIn {
	@builtin(position) frag: vec4<f32>,
}

// One triangle that covers the target; the fragment's `frag.xy` is the pixel's centre (x + 0.5, y + 0.5), top row first.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> FsIn {
	var corners = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
	var out: FsIn;
	out.frag = vec4<f32>(corners[index], 0.0, 1.0);
	return out;
}

// 1 - (1 - a)(1 - b) per channel, as the CPU's `screen`.
fn screen_blend(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
	return a + b - a * b;
}

fn saturate1(x: f32) -> f32 {
	return clamp(x, 0.0, 1.0);
}
