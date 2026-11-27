// Twilight's scene on the graphics card: the maps uploaded once, the camera, bodies and stars worked out each frame.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use bytemuck::{Pod, Zeroable};
use savers_core::colour::{self, mixf};
use savers_core::frame::Frame;
use savers_core::math::saturate;
use savers_core::noise::noise3;
use savers_core::post::FinishParams;
use savers_core::settings::{Detail, Key, Settings};
use savers_core::twilight_shared::{Shared, camera};
use wgpu::util::DeviceExt;

use crate::context::Gpu;
use crate::finish::{Finish, fullscreen_pipeline, run_pass, target, texture, uniform_buffer};
use crate::readback::read_frame;

/// Texels on a side of each moon's crater texture. A moon is at most about 0.2 of a 0.75-height planet across, so 256 is well past
/// one texel a pixel on a 4K screen.
const CRATER_SIZE: u32 = 256;
const MOON_SLOTS: usize = 8;
/// The most star instances: 700 stars, each a centre and four arms.
const MAX_STAR_INSTANCES: usize = 700 * 5;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BodyUniform {
	place: [f32; 4],
	kind: [f32; 4],
	colour: [f32; 4],
	pixel: [i32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneUniform {
	size: [f32; 4],
	planet: [f32; 4],
	light: [f32; 4],
	rot0: [f32; 4],
	rot1: [f32; 4],
	rot2: [f32; 4],
	map: [f32; 4],
	flags: [f32; 4],
	extra: [f32; 4],
	city: [f32; 4],
	cloud_lit: [f32; 4],
	sun_hue: [f32; 4],
	sun: [f32; 4],
	sun_glow: [f32; 4],
	sun_hot: [f32; 4],
	bodies: [BodyUniform; MOON_SLOTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BackgroundUniform {
	nebula_size: [f32; 4],
	origin_step: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct StarInstance {
	place: [f32; 2],
	colour: [f32; 3],
}

fn rgb(c: u32) -> [f32; 4] {
	[
		colour::r(c) as f32 / 255.0,
		colour::g(c) as f32 / 255.0,
		colour::b(c) as f32 / 255.0,
		1.0,
	]
}

fn sampler(device: &wgpu::Device, label: &str, filter: wgpu::FilterMode, wrap_u: bool) -> wgpu::Sampler {
	device.create_sampler(&wgpu::SamplerDescriptor {
		label: Some(label),
		address_mode_u: if wrap_u {
			wgpu::AddressMode::Repeat
		} else {
			wgpu::AddressMode::ClampToEdge
		},
		address_mode_v: wgpu::AddressMode::ClampToEdge,
		mag_filter: filter,
		min_filter: filter,
		..Default::default()
	})
}

/// Uploads texels as a 2D texture that can be sampled.
fn upload(gpu: &Gpu, label: &str, size: (u32, u32), layers: u32, format: wgpu::TextureFormat, bytes: &[u8]) -> wgpu::Texture {
	gpu.device.create_texture_with_data(
		&gpu.queue,
		&wgpu::TextureDescriptor {
			label: Some(label),
			size: wgpu::Extent3d {
				width: size.0,
				height: size.1,
				depth_or_array_layers: layers,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		},
		wgpu::util::TextureDataOrder::LayerMajor,
		bytes,
	)
}

fn rgba_bytes(pixels: &[u32]) -> Vec<u8> {
	let mut bytes = Vec::with_capacity(pixels.len() * 4);
	for &c in pixels {
		bytes.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
	}
	bytes
}

/// The shared maps and parameters for this edition: the same as the CPU's, with the maps at the size a graphics card should have at
/// the render size (2048 × 1024 for 4K at any detail level, as [`Detail::Ultra`] sizes them), not the CPU's smaller ones at Low
/// and Medium. Use [`Shared::new`] instead to draw exactly the CPU scene's maps, as the parity test does.
pub fn shared_for_gpu(settings: &Settings, seed: u32, render_size: (usize, usize)) -> Shared {
	let (map_width, map_height) = savers_core::twilight_shared::map_size(Detail::Ultra, render_size.0);
	Shared::with_map_size(settings, seed, render_size.0, render_size.1, map_width, map_height)
}

/// What the finishing chain does to one Twilight frame, as `Saver::draw` works it out for this scene. `wall` is the seconds since
/// the saver started, for the fade in from black.
pub fn finish_params(
	settings: &Settings,
	shared: &Shared,
	t: f32,
	wall: f32,
	output: (usize, usize),
	preview: bool,
) -> FinishParams {
	let cam = camera(t);
	let detail = if preview { Detail::Low } else { settings.detail() };
	let mut fade = saturate(wall);
	if settings.get(Key::Fades) == 1 {
		fade *= saturate(cam.fade);
	}
	let wide = output.0 as f32 / output.1 as f32;
	FinishParams {
		bloom: ((settings.get(Key::Bloom) as f32 * 1.2) as u32).min(100),
		streaks: if detail == Detail::Low {
			settings.get(Key::Streaks) / 2
		} else {
			settings.get(Key::Streaks)
		},
		vignette: settings.get(Key::Vignette),
		grain: if preview { 0 } else { settings.get(Key::Grain) },
		letterbox: if settings.get(Key::Letterbox) == 1 && !preview {
			if wide > 1.5 { 2.39 } else { 1.85 }
		} else {
			0.0
		},
		fade,
		threshold: 135,
		streak_tint: mixf(shared.palette.hue(0), 0xFFFFFF, 0.35),
		coarse: detail == Detail::Low,
	}
}

/// The renderer: the planet's textures, the passes that draw it and the finishing chain.
pub struct TwilightRenderer {
	gpu_detail: Detail,
	preview: bool,
	render_size: (usize, usize),
	output: (usize, usize),
	settings: Settings,
	shared: Shared,
	finish: Finish,
	backdrop_view: wgpu::TextureView,
	background_pipeline: wgpu::RenderPipeline,
	background_buffer: wgpu::Buffer,
	background_group: wgpu::BindGroup,
	stars_pipeline: wgpu::RenderPipeline,
	stars_buffer: wgpu::Buffer,
	stars_size_group: wgpu::BindGroup,
	scene_pipeline: wgpu::RenderPipeline,
	scene_buffer: wgpu::Buffer,
	scene_group: wgpu::BindGroup,
	nebula_size: (f32, f32),
	frame_no: u32,
}

impl TwilightRenderer {
	/// Builds the renderer for a scene made from `settings` and `seed`. `shared` must have been made for `render_size` (the
	/// caller chooses its map size: [`Shared::new`] for the CPU's, [`Shared::with_map_size`] for 2048 × 1024 at 4K). The finished
	/// frame is `output` pixels.
	pub fn new(
		gpu: &Gpu,
		settings: &Settings,
		shared: Shared,
		seed: u32,
		render_size: (usize, usize),
		output: (usize, usize),
		preview: bool,
	) -> TwilightRenderer {
		let device = &gpu.device;
		let detail = if preview { Detail::Low } else { settings.detail() };
		let finish = Finish::new(gpu, render_size, output, detail == Detail::Low);
		let render_u32 = (render_size.0 as u32, render_size.1 as u32);

		// The maps, once.
		let (mw, mh) = (shared.map_width as u32, shared.map_height as u32);
		let ground = upload(
			gpu,
			"ground",
			(mw, mh),
			1,
			wgpu::TextureFormat::Rgba8Unorm,
			&rgba_bytes(&shared.ground),
		);
		let clouds = upload(gpu, "clouds", (mw, mh), 1, wgpu::TextureFormat::R8Unorm, &shared.clouds);
		let lights = upload(gpu, "lights", (mw, mh), 1, wgpu::TextureFormat::R8Unorm, &shared.lights);
		let light_ramp = upload(
			gpu,
			"light ramp",
			(256, 1),
			1,
			wgpu::TextureFormat::Rgba8Unorm,
			&rgba_bytes(&shared.light_ramp.c),
		);
		let atmosphere_ramp = upload(
			gpu,
			"atmosphere ramp",
			(256, 1),
			1,
			wgpu::TextureFormat::Rgba8Unorm,
			&rgba_bytes(&shared.atmosphere_ramp.c),
		);
		let nebula = &shared.nebula;
		let nebula_texture = upload(
			gpu,
			"nebula",
			(nebula.w as u32, nebula.h as u32),
			1,
			wgpu::TextureFormat::Rgba8Unorm,
			&rgba_bytes(&nebula.px),
		);
		// Each moon's craters are noise over the face of the disc, the same at every frame: the CPU's `noise3` baked into a
		// texture, so the shader needs no noise of its own.
		let moon_layers = shared.moons.len().max(1) as u32;
		let mut crater_bytes = vec![128u8; (CRATER_SIZE * CRATER_SIZE * moon_layers) as usize];
		for index in 0..shared.moons.len() {
			for y in 0..CRATER_SIZE {
				for x in 0..CRATER_SIZE {
					let nx = (x as f32 + 0.5) / CRATER_SIZE as f32 * 2.0 - 1.0;
					let ny = (y as f32 + 0.5) / CRATER_SIZE as f32 * 2.0 - 1.0;
					let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
					let crater = noise3(nx * 3.0 + index as f32 * 9.0, ny * 3.0, nz * 3.0, seed ^ 0xC0) * 0.25;
					let at = (index as u32 * CRATER_SIZE * CRATER_SIZE + y * CRATER_SIZE + x) as usize;
					crater_bytes[at] = ((crater * 1.6 + 0.5).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
				}
			}
		}
		let craters = upload(
			gpu,
			"craters",
			(CRATER_SIZE, CRATER_SIZE),
			moon_layers,
			wgpu::TextureFormat::R8Unorm,
			&crater_bytes,
		);
		let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
		let craters_view = craters.create_view(&wgpu::TextureViewDescriptor {
			dimension: Some(wgpu::TextureViewDimension::D2Array),
			..Default::default()
		});

		let backdrop = texture(
			device,
			"backdrop",
			render_u32,
			wgpu::TextureFormat::Rgba8Unorm,
			wgpu::TextureUsages::empty(),
		);
		let backdrop_view = view(&backdrop);
		let linear_wrap = sampler(device, "linear wrap", wgpu::FilterMode::Linear, true);
		let linear_clamp = sampler(device, "linear clamp", wgpu::FilterMode::Linear, false);

		// Pass 1: the nebula.
		let background_pipeline = fullscreen_pipeline(
			device,
			"background",
			include_str!("shaders/background.wgsl"),
			&[target(wgpu::TextureFormat::Rgba8Unorm)],
		);
		let background_buffer = uniform_buffer(device, "background", &BackgroundUniform::zeroed());
		let nebula_view = view(&nebula_texture);
		let background_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("background"),
			layout: &background_pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: background_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&nebula_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&linear_clamp),
				},
			],
		});

		// Pass 2: the stars, instanced points blended with the screen blend: src + dst * (1 - src).
		let stars_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("stars"),
			source: wgpu::ShaderSource::Wgsl(include_str!("shaders/stars.wgsl").into()),
		});
		let screen = wgpu::BlendComponent {
			src_factor: wgpu::BlendFactor::One,
			dst_factor: wgpu::BlendFactor::OneMinusSrc,
			operation: wgpu::BlendOperation::Add,
		};
		let star_attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];
		let stars_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("stars"),
			layout: None,
			vertex: wgpu::VertexState {
				module: &stars_module,
				entry_point: Some("vs_main"),
				compilation_options: Default::default(),
				buffers: &[Some(wgpu::VertexBufferLayout {
					array_stride: std::mem::size_of::<StarInstance>() as u64,
					step_mode: wgpu::VertexStepMode::Instance,
					attributes: &star_attributes,
				})],
			},
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			fragment: Some(wgpu::FragmentState {
				module: &stars_module,
				entry_point: Some("fs_main"),
				compilation_options: Default::default(),
				targets: &[Some(wgpu::ColorTargetState {
					format: wgpu::TextureFormat::Rgba8Unorm,
					blend: Some(wgpu::BlendState {
						color: screen,
						alpha: screen,
					}),
					write_mask: wgpu::ColorWrites::ALL,
				})],
			}),
			multiview_mask: None,
			cache: None,
		});
		let stars_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("stars"),
			size: (MAX_STAR_INSTANCES * std::mem::size_of::<StarInstance>()) as u64,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		let stars_size = uniform_buffer(device, "stars size", &[render_size.0 as f32, render_size.1 as f32, 0.0, 0.0]);
		let stars_size_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("stars"),
			layout: &stars_pipeline.get_bind_group_layout(0),
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: stars_size.as_entire_binding(),
			}],
		});

		// Pass 3: the sun, bodies and planet, reading the backdrop and drawing into the finishing chain's input.
		let scene_pipeline = fullscreen_pipeline(
			device,
			"scene",
			include_str!("shaders/scene.wgsl"),
			&[target(wgpu::TextureFormat::Rgba8Unorm)],
		);
		let scene_buffer = uniform_buffer(device, "scene", &SceneUniform::zeroed());
		let ground_view = view(&ground);
		let clouds_view = view(&clouds);
		let lights_view = view(&lights);
		let light_ramp_view = view(&light_ramp);
		let atmosphere_ramp_view = view(&atmosphere_ramp);
		let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("scene"),
			layout: &scene_pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: scene_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&backdrop_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(&ground_view),
				},
				wgpu::BindGroupEntry {
					binding: 3,
					resource: wgpu::BindingResource::TextureView(&clouds_view),
				},
				wgpu::BindGroupEntry {
					binding: 4,
					resource: wgpu::BindingResource::TextureView(&lights_view),
				},
				wgpu::BindGroupEntry {
					binding: 5,
					resource: wgpu::BindingResource::TextureView(&light_ramp_view),
				},
				wgpu::BindGroupEntry {
					binding: 6,
					resource: wgpu::BindingResource::TextureView(&atmosphere_ramp_view),
				},
				wgpu::BindGroupEntry {
					binding: 7,
					resource: wgpu::BindingResource::TextureView(&craters_view),
				},
				wgpu::BindGroupEntry {
					binding: 8,
					resource: wgpu::BindingResource::Sampler(&linear_wrap),
				},
				wgpu::BindGroupEntry {
					binding: 9,
					resource: wgpu::BindingResource::Sampler(&linear_clamp),
				},
			],
		});
		let nebula_size = (nebula.w as f32, nebula.h as f32);

		// The textures stay alive through their views and bind groups.
		TwilightRenderer {
			gpu_detail: detail,
			preview,
			render_size,
			output,
			settings: settings.clone(),
			shared,
			finish,
			backdrop_view,
			background_pipeline,
			background_buffer,
			background_group,
			stars_pipeline,
			stars_buffer,
			stars_size_group,
			scene_pipeline,
			scene_buffer,
			scene_group,
			nebula_size,
			frame_no: 0,
		}
	}

	/// The finishing chain, for its output texture and size.
	pub fn finish(&self) -> &Finish {
		&self.finish
	}

	/// Records and submits the frame for scene time `t` (seconds) and `wall` seconds since the start, ending in the finish
	/// chain's output texture. Returns the number of the frame (the grain's seed).
	pub fn render(&mut self, gpu: &Gpu, t: f64, wall: f64) -> u32 {
		let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("twilight frame"),
		});
		self.encode(gpu, &mut encoder, t, wall);
		gpu.queue.submit([encoder.finish()]);
		self.frame_no
	}

	/// The same, into an encoder the caller submits, such as one that goes on to present the output to a window.
	pub fn encode(&mut self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, t: f64, wall: f64) {
		let t = t as f32;
		let (w, h) = (self.render_size.0 as f32, self.render_size.1 as f32);
		let cam = camera(t);
		let shared = &self.shared;
		let detail = self.gpu_detail;

		// The nebula, panned against the planet's drift for parallax.
		let pan_x = (cam.centre_x - 0.5) * -0.2 + (t * 0.01).sin() * 0.03;
		let pan_y = (cam.centre_y - 0.5) * -0.2;
		let (nw, nh) = self.nebula_size;
		gpu.queue.write_buffer(
			&self.background_buffer,
			0,
			bytemuck::bytes_of(&BackgroundUniform {
				nebula_size: [nw, nh, 0.0, 0.0],
				origin_step: [(0.1 + pan_x * 0.5) * nw, (0.1 + pan_y * 0.5) * nh, nw * 0.8 / w, nh * 0.8 / h],
			}),
		);

		// The stars: a centre pixel each, and four arms on the bright ones.
		let mut instances: Vec<StarInstance> = Vec::with_capacity(MAX_STAR_INSTANCES);
		for s in &shared.stars {
			let x = (s.x + pan_x * 0.7).rem_euclid(1.0) * w;
			let y = (s.y + pan_y * 0.7).rem_euclid(1.0) * h;
			let twinkle = 0.65 + 0.35 * (t * s.twinkle * 2.0 + s.x * 40.0).sin();
			let k = (s.bright * twinkle * 256.0) as u32;
			let (xi, yi) = (x as i32, y as i32);
			if xi < 0 || yi < 0 || xi as f32 >= w || yi as f32 >= h {
				continue;
			}
			let tint = rgb(s.tint);
			let scaled = |k: u32| {
				[
					tint[0] * k as f32 / 256.0,
					tint[1] * k as f32 / 256.0,
					tint[2] * k as f32 / 256.0,
				]
			};
			instances.push(StarInstance {
				place: [xi as f32, yi as f32],
				colour: scaled(k),
			});
			if s.bright > 0.5 {
				// The CPU sets each arm to the centre's colour (star included) screened with a third of the star, so an arm is
				// nearly as bright as the centre: the same colour added over the nebula next to it.
				let (centre, arm) = (scaled(k), scaled(k / 3));
				let colour = [0, 1, 2].map(|c| centre[c] + arm[c] - centre[c] * arm[c]);
				for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
					instances.push(StarInstance {
						place: [(xi + dx) as f32, (yi + dy) as f32],
						colour,
					});
				}
			}
		}
		instances.truncate(MAX_STAR_INSTANCES);
		gpu.queue
			.write_buffer(&self.stars_buffer, 0, bytemuck::cast_slice(&instances));

		// The planet's frame: the light direction in view space, and the axis turned towards the viewer and rolled.
		let (pcx, pcy) = (cam.centre_x * w, cam.centre_y * h);
		let r = cam.radius * h;
		let (tilt, roll) = (cam.tilt, cam.roll);
		let la = cam.light_angle + t * 0.004;
		let light_raw = [la.cos(), -0.3, la.sin()];
		let length = (light_raw[0] * light_raw[0] + light_raw[1] * light_raw[1] + light_raw[2] * light_raw[2]).sqrt();
		let light = [light_raw[0] / length, light_raw[1] / length, light_raw[2] / length];
		let spin = t * shared.spin_rate;
		let cloud_spin = t * shared.spin_rate * 1.35;
		let (mw, mh) = (shared.map_width as f32, shared.map_height as f32);
		let (cr, sr) = (roll.cos(), roll.sin());
		let (ct, st) = (tilt.cos(), tilt.sin());
		let halo = 1.0 + 0.22 * shared.atmosphere;
		let mut bodies = [BodyUniform::zeroed(); MOON_SLOTS];
		let mut count = 0;
		for (i, m) in shared.moons.iter().enumerate() {
			let a = m.phase + t * m.speed;
			let p = [
				a.cos() * m.orbit,
				a.sin() * m.orbit * m.incline.sin(),
				a.sin() * m.orbit * m.incline.cos(),
			];
			let p = rotate_view(p, tilt * 0.3, roll);
			bodies[count] = BodyUniform {
				place: [pcx + p[0] * r, pcy + p[1] * r, p[2], m.radius * r],
				kind: [0.0, i as f32, 0.0, 0.0],
				colour: rgb(m.tint),
				pixel: [0; 4],
			};
			count += 1;
		}
		for (i, s) in shared.satellites.iter().enumerate() {
			let a = s.phase + t * s.speed * if cam.shot == 2 { 1.6 } else { 1.0 };
			let local = [a.cos() * s.orbit, 0.0, a.sin() * s.orbit];
			let p = [local[0], local[2] * s.incline.sin(), local[2] * s.incline.cos()];
			let p = rotate_view(p, s.node, roll);
			let (bx, by) = (pcx + p[0] * r, pcy + p[1] * r);
			let blink = (t * 0.8 + i as f32 * 0.37).rem_euclid(1.0) < 0.12;
			let colour = if blink {
				mixf(shared.palette.sun, 0xFFFFFF, 0.6)
			} else {
				shared.palette.n(300)
			};
			bodies[count] = BodyUniform {
				place: [bx, by, p[2], 0.0],
				kind: [1.0, i as f32, if blink { 1.0 } else { 0.0 }, 0.0],
				colour: rgb(colour),
				pixel: [bx as i32, by as i32, 0, 0],
			};
			count += 1;
		}
		// The sun, behind everything but the stars: a small white-hot disc in a soft glow.
		let mut sun = [0.0; 4];
		if shared.sun && cam.sun_distance > 0.0 {
			let d = (light[0] * light[0] + light[1] * light[1]).sqrt().max(1e-3);
			sun = [
				pcx + light[0] / d * r * cam.sun_distance,
				pcy + light[1] / d * r * cam.sun_distance,
				(h * 0.014).max(1.5),
				h * 0.17,
			];
		}
		let flag = |on: bool| if on { 1.0 } else { 0.0 };
		let palette = &shared.palette;
		let uniform = SceneUniform {
			size: [w, h, 0.0, 0.0],
			planet: [pcx, pcy, r, halo],
			light: [light[0], light[1], light[2], shared.atmosphere],
			rot0: [cr, sr, 0.0, 0.0],
			rot1: [-sr * ct, cr * ct, st, 0.0],
			rot2: [sr * st, -cr * st, ct, 0.0],
			map: [mw, mh, (spin.rem_euclid(1.0)) * mw, (cloud_spin.rem_euclid(1.0)) * mw],
			flags: [
				flag(detail >= Detail::Medium),
				flag(detail >= Detail::Medium),
				flag(detail >= Detail::High),
				flag(detail >= Detail::Ultra),
			],
			extra: [flag(detail >= Detail::Medium), count as f32, 0.0, 0.0],
			city: rgb(mixf(palette.sun, 0xFFFFFF, 0.2)),
			cloud_lit: rgb(mixf(palette.n(200), palette.sun, 0.1)),
			sun_hue: rgb(palette.sun),
			sun,
			sun_glow: rgb(mixf(palette.sun, palette.hue(0), 0.35)),
			sun_hot: rgb(mixf(palette.sun, 0xFFFFFF, 0.75)),
			bodies,
		};
		gpu.queue.write_buffer(&self.scene_buffer, 0, bytemuck::bytes_of(&uniform));

		run_pass(
			encoder,
			"background",
			&self.background_pipeline,
			&self.background_group,
			&[&self.backdrop_view],
		);
		{
			let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("stars"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &self.backdrop_view,
					depth_slice: None,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Load,
						store: wgpu::StoreOp::Store,
					},
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
				multiview_mask: None,
			});
			pass.set_pipeline(&self.stars_pipeline);
			pass.set_bind_group(0, &self.stars_size_group, &[]);
			pass.set_vertex_buffer(0, self.stars_buffer.slice(..));
			pass.draw(0..6, 0..instances.len() as u32);
		}
		run_pass(
			encoder,
			"scene",
			&self.scene_pipeline,
			&self.scene_group,
			&[self.finish.input_view()],
		);

		self.frame_no = self.frame_no.wrapping_add(1);
		let params = finish_params(&self.settings, &self.shared, t, wall as f32, self.output, self.preview);
		self.finish.encode(gpu, encoder, &params, self.frame_no);
	}

	/// Reads the finished frame back as 0x00RRGGBB pixels.
	pub fn read_back(&self, gpu: &Gpu) -> Frame {
		read_frame(gpu, self.finish.output(), self.output.0, self.output.1)
	}
}

/// A point turned into view space: a tilt about x, then a roll about z.
fn rotate_view(p: [f32; 3], tilt: f32, roll: f32) -> [f32; 3] {
	let (ct, st) = (tilt.cos(), tilt.sin());
	let q = [p[0], p[1] * ct - p[2] * st, p[1] * st + p[2] * ct];
	let (cr, sr) = (roll.cos(), roll.sin());
	[q[0] * cr - q[1] * sr, q[0] * sr + q[1] * cr, q[2]]
}
