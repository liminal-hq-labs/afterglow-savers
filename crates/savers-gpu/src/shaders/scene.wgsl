// Pass 3: the sun, the moons and satellites, and the planet over the nebula and stars, one pass over the whole frame. A transcription
// of the CPU scene's `draw_sun`, `draw_planet` and `draw_body`, in the same order: the sun, the bodies behind the planet, the planet
// with its halo, then the bodies in front.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

const PI: f32 = 3.14159265358979;
const TAU: f32 = 6.28318530717959;

// A moon or a satellite, placed on the screen by the host each frame.
struct Body {
	// The centre in pixels (x, y), the depth (z, negative is behind the planet) and the radius in pixels (w, 0 for a satellite).
	place: vec4<f32>,
	// 0 for a moon or 1 for a satellite (x), the moon's index (y).
	kind: vec4<f32>,
	// The moon's tint, or the satellite's colour.
	colour: vec4<f32>,
	// The satellite's pixel.
	pixel: vec4<i32>,
}

struct Params {
	// The render size.
	size: vec4<f32>,
	// The planet's centre in pixels (x, y), its radius (z) and the halo's outer edge in radii (w).
	planet: vec4<f32>,
	// The light's direction (xyz) and the atmosphere's strength (w).
	light: vec4<f32>,
	// The rows of the matrix from view space to the planet's frame.
	rot0: vec4<f32>,
	rot1: vec4<f32>,
	rot2: vec4<f32>,
	// The map's width and height (xy), the ground's spin in map texels (z) and the clouds' (w).
	map: vec4<f32>,
	// 1 or 0 for: smooth ground, cloud shadows, night lights, the sea's sheen.
	flags: vec4<f32>,
	// 1 for the satellites' second texel (Medium and above) in x; the body count in y.
	extra: vec4<f32>,
	city: vec4<f32>,
	cloud_lit: vec4<f32>,
	sun_hue: vec4<f32>,
	// The sun's centre in pixels (x, y), its core radius (z) and its glow's reach (w, 0 for no sun).
	sun: vec4<f32>,
	sun_glow: vec4<f32>,
	sun_hot: vec4<f32>,
	bodies: array<Body, 8>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var backdrop: texture_2d<f32>;
@group(0) @binding(2) var ground_map: texture_2d<f32>;
@group(0) @binding(3) var cloud_map: texture_2d<f32>;
@group(0) @binding(4) var lights_map: texture_2d<f32>;
@group(0) @binding(5) var light_ramp: texture_2d<f32>;
@group(0) @binding(6) var atmosphere_ramp: texture_2d<f32>;
@group(0) @binding(7) var craters: texture_2d_array<f32>;
@group(0) @binding(8) var linear_wrap: sampler;
@group(0) @binding(9) var linear_clamp: sampler;

// A colour from a 256-step ramp at x in [0, 1], as `Ramp::at`.
fn ramp_at(ramp: texture_2d<f32>, x: f32) -> vec3<f32> {
	return textureLoad(ramp, vec2<i32>(i32(saturate1(x) * 255.0), 0), 0).rgb;
}

// A surface colour lit by a light colour, a little over unity so a white light brightens.
fn tint(surface: vec3<f32>, light: vec3<f32>) -> vec3<f32> {
	return min(surface * light * 1.2451, vec3<f32>(1.0));
}

fn shade(c: vec3<f32>, t: f32) -> vec3<f32> {
	if (t >= 0.0) {
		return mix(c, vec3<f32>(1.0), t);
	}
	return mix(c, vec3<f32>(0.0), -t);
}

fn draw_sun(pix: vec3<f32>, frag: vec2<f32>) -> vec3<f32> {
	let reach = params.sun.w;
	if (reach <= 0.0) {
		return pix;
	}
	let core = params.sun.z;
	let d = length(frag - params.sun.xy);
	if (d >= reach) {
		return pix;
	}
	var c: vec3<f32>;
	if (d < core) {
		c = params.sun_hot.rgb;
	} else if (d < core + 1.0) {
		c = mix(params.sun_glow.rgb * (230.0 / 256.0), params.sun_hot.rgb, core + 1.0 - d);
	} else {
		let k = 1.0 - d / reach;
		let level = min(k * k * k * 190.0 + (core / d) * 110.0, 255.0);
		c = params.sun_glow.rgb * (level / 256.0);
	}
	return screen_blend(pix, c);
}

fn draw_body(pix: vec3<f32>, frag: vec2<f32>, body: Body) -> vec3<f32> {
	if (body.kind.x > 0.5) {
		// A satellite: a glint and a blinking light, set on single pixels.
		let offset = vec2<i32>(floor(frag)) - body.pixel.xy;
		let c = body.colour.rgb;
		if (offset.x == 0 && offset.y == 0) {
			return c;
		}
		if (params.extra.x > 0.5) {
			if (offset.x == 1 && offset.y == 0) {
				return c * (120.0 / 256.0);
			}
			if (body.kind.z > 0.5) {
				let arm = (offset.y == 0 && (offset.x == -1 || offset.x == 2 || offset.x == -2))
					|| (offset.x == 0 && (offset.y == 1 || offset.y == -1));
				if (arm) {
					return c * (160.0 / 256.0);
				}
			}
		}
		return pix;
	}
	let radius = body.place.w;
	if (radius < 0.5) {
		return pix;
	}
	let n = (frag - body.place.xy) / radius;
	let d2 = dot(n, n);
	if (d2 >= 1.0) {
		return pix;
	}
	let nz = sqrt(1.0 - d2);
	let light = params.light.xyz;
	let ndl = n.x * light.x + n.y * light.y + nz * light.z;
	let lit = saturate1((ndl + 0.08) / 1.08);
	let crater_texel = textureSampleLevel(craters, linear_clamp, n * 0.5 + vec2<f32>(0.5, 0.5), i32(body.kind.y), 0.0).r;
	let crater = (crater_texel - 0.5) / 1.6;
	var c = shade(body.colour.rgb, crater);
	c = c * ((lit * 235.0 + 8.0) / 256.0);
	let edge = (1.0 - sqrt(d2)) * radius;
	if (edge < 1.0) {
		return mix(pix, c, edge);
	}
	return c;
}

fn draw_planet(pix_in: vec3<f32>, frag: vec2<f32>) -> vec3<f32> {
	var pix = pix_in;
	let radius = params.planet.z;
	let halo = params.planet.w;
	let atmos = params.light.w;
	let light = params.light.xyz;
	let n = (frag - params.planet.xy) / radius;
	let d2 = dot(n, n);
	if (d2 >= 1.0) {
		// Outside the limb: the atmosphere's halo, brightest towards the light.
		let d = sqrt(d2);
		if (d < halo) {
			let falloff = 1.0 - (d - 1.0) / (halo - 1.0);
			let side = saturate1((n.x * light.x + n.y * light.y) / d * 0.6 + 0.55);
			let k = falloff * falloff * falloff * falloff * side * atmos * 1.3;
			if (k > 0.004) {
				pix = screen_blend(pix, ramp_at(atmosphere_ramp, k * 0.9));
			}
		}
		return pix;
	}
	let nz = sqrt(1.0 - d2);
	// Into the planet's frame: y is the axis, so latitude is asin(-p.y) as a map row, and longitude atan2(p.x, p.z).
	let view = vec3<f32>(n, nz);
	let p0 = dot(params.rot0.xyz, view);
	let p1 = dot(params.rot1.xyz, view);
	let p2 = dot(params.rot2.xyz, view);
	let map_width = params.map.x;
	let map_height = params.map.y;
	let row = (asin(clamp(-p1, -1.0, 1.0)) / PI + 0.5) * (map_height - 1.0);
	let lon = atan2(p0, p2);
	let column = lon * (map_width / TAU) + params.map.z;
	let smooth_ground = params.flags.x > 0.5;
	let texels = vec2<i32>(i32(map_width), i32(map_height));
	// Whole-texel reads, as the CPU's unsmoothed ground, the lights and the clouds' rows take: the texel the coordinate is in,
	// longitude wrapped and latitude clamped. (One texture is never read through two samplers, which GL cannot do.)
	let row_texel = clamp(i32(floor(row)), 0, texels.y - 1);
	var ground: vec3<f32>;
	if (smooth_ground) {
		// The CPU's smooth sample puts texel centres on whole numbers.
		let ground_uv = vec2<f32>((column + 0.5) / map_width, (row + 0.5) / map_height);
		ground = textureSampleLevel(ground_map, linear_wrap, ground_uv, 0.0).rgb;
	} else {
		let column_texel = ((i32(floor(column)) % texels.x) + texels.x) % texels.x;
		ground = textureLoad(ground_map, vec2<i32>(column_texel, row_texel), 0).rgb;
	}
	// Light is a colour: warm white by day, orange then rose at the terminator, purple dusk, indigo night.
	let ndl = dot(n, light.xy) + nz * light.z;
	let light_colour = ramp_at(light_ramp, saturate1((ndl + 0.3) * (1.0 / 1.3)));
	var c = tint(ground, light_colour);
	let row_uv_v = (f32(row_texel) + 0.5) / map_height;
	if (ndl < 0.0 && params.flags.z > 0.5) {
		let light_column = ((i32(floor(column)) % texels.x) + texels.x) % texels.x;
		let l = textureLoad(lights_map, vec2<i32>(light_column, row_texel), 0).r;
		if (l > 0.0) {
			c = screen_blend(c, params.city.rgb * (l * saturate1(-ndl * 3.0) * 0.7004));
		}
	}
	// Clouds, lit like the ground, with their shadow a little towards the dark side.
	let cloud_column = lon * (map_width / TAU) + params.map.w;
	let cloud_uv = vec2<f32>((cloud_column + 0.5) / map_width, row_uv_v);
	let cloud = textureSampleLevel(cloud_map, linear_wrap, cloud_uv, 0.0).r;
	if (params.flags.y > 0.5) {
		let shadow_row = floor(row + light.y * 4.0);
		let shadow_uv = vec2<f32>((cloud_column + light.x * -6.0 + 0.5) / map_width, (shadow_row + 0.5) / map_height);
		let shadow = textureSampleLevel(cloud_map, linear_wrap, shadow_uv, 0.0).r;
		c = c * (1.0 - shadow * 0.3502);
	}
	if (cloud > 0.0) {
		c = mix(c, tint(params.cloud_lit.rgb, light_colour), cloud);
	}
	// The sea's sheen: a soft highlight where the light reflects towards the viewer.
	if (params.flags.w > 0.5 && ground.b > ground.r) {
		let hz = light.z + 1.0;
		let hv = (n.x * light.x + n.y * light.y + nz * hz) / sqrt(light.x * light.x + light.y * light.y + hz * hz);
		if (hv > 0.9) {
			let glint = pow(saturate1((hv - 0.9) * 10.0), 3.0) * 70.0 / 256.0;
			c = screen_blend(c, params.sun_hue.rgb * glint);
		}
	}
	// The atmosphere inside the limb.
	let rim = 1.0 - nz;
	let k = rim * rim * rim * atmos * saturate1(ndl * 1.5 + 0.45) * 1.6;
	if (k > 0.003) {
		c = screen_blend(c, ramp_at(atmosphere_ramp, saturate1(k)));
	}
	// Anti-alias the limb over the last pixel.
	let edge = (1.0 - sqrt(d2)) * radius;
	if (edge < 1.0) {
		return mix(pix, c, edge);
	}
	return c;
}

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
	let frag = in.frag.xy;
	var pix = textureLoad(backdrop, vec2<i32>(floor(frag)), 0).rgb;
	pix = draw_sun(pix, frag);
	let count = u32(params.extra.y);
	for (var i = 0u; i < count; i = i + 1u) {
		if (params.bodies[i].place.z < 0.0) {
			pix = draw_body(pix, frag, params.bodies[i]);
		}
	}
	pix = draw_planet(pix, frag);
	for (var i = 0u; i < count; i = i + 1u) {
		if (params.bodies[i].place.z >= 0.0) {
			pix = draw_body(pix, frag, params.bodies[i]);
		}
	}
	return vec4<f32>(pix, 1.0);
}
