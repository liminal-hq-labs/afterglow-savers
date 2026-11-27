// The finishing chain on the graphics card: bloom, streaks, vignette, fade, letterbox and grain, as `savers_core::post`.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use bytemuck::{Pod, Zeroable};
use savers_core::colour::{b, g, r};
use savers_core::post::FinishParams;
use wgpu::util::DeviceExt;

use crate::context::Gpu;

const COMMON: &str = include_str!("shaders/common.wgsl");

/// A full-screen pass: the common vertex stage and a fragment stage, to the given targets.
pub(crate) fn fullscreen_pipeline(
	device: &wgpu::Device,
	label: &str,
	fragment_source: &str,
	targets: &[Option<wgpu::ColorTargetState>],
) -> wgpu::RenderPipeline {
	let source = format!("{COMMON}\n{fragment_source}");
	let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: Some(label),
		source: wgpu::ShaderSource::Wgsl(source.into()),
	});
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some(label),
		layout: None,
		vertex: wgpu::VertexState {
			module: &module,
			entry_point: Some("vs_main"),
			compilation_options: Default::default(),
			buffers: &[],
		},
		primitive: wgpu::PrimitiveState::default(),
		depth_stencil: None,
		multisample: wgpu::MultisampleState::default(),
		fragment: Some(wgpu::FragmentState {
			module: &module,
			entry_point: Some("fs_main"),
			compilation_options: Default::default(),
			targets,
		}),
		multiview_mask: None,
		cache: None,
	})
}

pub(crate) fn target(format: wgpu::TextureFormat) -> Option<wgpu::ColorTargetState> {
	Some(wgpu::ColorTargetState {
		format,
		blend: None,
		write_mask: wgpu::ColorWrites::ALL,
	})
}

/// A 2D texture that can be drawn to and read in a shader.
pub(crate) fn texture(
	device: &wgpu::Device,
	label: &str,
	size: (u32, u32),
	format: wgpu::TextureFormat,
	extra: wgpu::TextureUsages,
) -> wgpu::Texture {
	device.create_texture(&wgpu::TextureDescriptor {
		label: Some(label),
		size: wgpu::Extent3d {
			width: size.0,
			height: size.1,
			depth_or_array_layers: 1,
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format,
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | extra,
		view_formats: &[],
	})
}

pub(crate) fn uniform_buffer<T: Pod>(device: &wgpu::Device, label: &str, value: &T) -> wgpu::Buffer {
	device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some(label),
		contents: bytemuck::bytes_of(value),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	})
}

/// Records one full-screen pass into `targets`.
pub(crate) fn run_pass(
	encoder: &mut wgpu::CommandEncoder,
	label: &str,
	pipeline: &wgpu::RenderPipeline,
	bind_group: &wgpu::BindGroup,
	targets: &[&wgpu::TextureView],
) {
	let attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = targets
		.iter()
		.map(|view| {
			Some(wgpu::RenderPassColorAttachment {
				view,
				depth_slice: None,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
					store: wgpu::StoreOp::Store,
				},
			})
		})
		.collect();
	let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		label: Some(label),
		color_attachments: &attachments,
		depth_stencil_attachment: None,
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
	});
	pass.set_pipeline(pipeline);
	pass.set_bind_group(0, bind_group, &[]);
	pass.draw(0..3, 0..1);
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BrightUniform {
	frame_step: [i32; 4],
	threshold: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlurUniform {
	axis_radius: [i32; 4],
	size: [i32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PackUniform {
	gains: [f32; 4],
	tint: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayUniform {
	size: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OutputUniform {
	size_rows: [f32; 4],
	look: [f32; 4],
	grain: [u32; 4],
}

/// One blur pass: which texture it reads, which it writes, and its uniform.
struct BlurPass {
	label: &'static str,
	bind_group: wgpu::BindGroup,
	/// 0 for the first of each pair of textures, 1 for the second.
	target: usize,
	streak: bool,
}

/// The finishing chain's textures and passes for one render size and output size.
pub struct Finish {
	render_size: (usize, usize),
	output_size: (usize, usize),
	step: usize,
	coarse_size: (usize, usize),
	input: wgpu::Texture,
	input_view: wgpu::TextureView,
	output: wgpu::Texture,
	output_view: wgpu::TextureView,
	bloom: [wgpu::TextureView; 2],
	streak: [wgpu::TextureView; 2],
	glow_view: wgpu::TextureView,
	frame_view: wgpu::TextureView,
	bright_pipeline: wgpu::RenderPipeline,
	bright_buffer: wgpu::Buffer,
	bright_group: wgpu::BindGroup,
	blur_bloom_pipeline: wgpu::RenderPipeline,
	blur_streak_pipeline: wgpu::RenderPipeline,
	blur_passes: Vec<BlurPass>,
	pack_pipeline: wgpu::RenderPipeline,
	pack_buffer: wgpu::Buffer,
	pack_group: wgpu::BindGroup,
	lay_pipeline: wgpu::RenderPipeline,
	lay_group: wgpu::BindGroup,
	output_pipeline: wgpu::RenderPipeline,
	output_buffer: wgpu::Buffer,
	output_group: wgpu::BindGroup,
}

const BLOOM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const STREAK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
const FRAME_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

impl Finish {
	/// Builds the chain for frames of `render_size`, enlarged to `output_size`. `coarse` takes the glow at an eighth of the render
	/// size instead of a quarter, as the CPU does at Low detail.
	pub fn new(gpu: &Gpu, render_size: (usize, usize), output_size: (usize, usize), coarse: bool) -> Finish {
		let device = &gpu.device;
		let step = if coarse { 8 } else { 4 };
		let coarse_size = (render_size.0.div_ceil(step), render_size.1.div_ceil(step));
		let coarse_u32 = (coarse_size.0 as u32, coarse_size.1 as u32);
		let render_u32 = (render_size.0 as u32, render_size.1 as u32);
		let output_u32 = (output_size.0 as u32, output_size.1 as u32);
		let input = texture(
			device,
			"finish input",
			render_u32,
			FRAME_FORMAT,
			wgpu::TextureUsages::COPY_DST,
		);
		let frame = texture(device, "finish frame", render_u32, FRAME_FORMAT, wgpu::TextureUsages::empty());
		let output = texture(
			device,
			"finish output",
			output_u32,
			FRAME_FORMAT,
			wgpu::TextureUsages::COPY_SRC,
		);
		let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
		let bloom_textures = [
			texture(device, "bloom a", coarse_u32, BLOOM_FORMAT, wgpu::TextureUsages::empty()),
			texture(device, "bloom b", coarse_u32, BLOOM_FORMAT, wgpu::TextureUsages::empty()),
		];
		let streak_textures = [
			texture(device, "streak a", coarse_u32, STREAK_FORMAT, wgpu::TextureUsages::empty()),
			texture(device, "streak b", coarse_u32, STREAK_FORMAT, wgpu::TextureUsages::empty()),
		];
		let glow = texture(device, "glow", coarse_u32, FRAME_FORMAT, wgpu::TextureUsages::empty());
		let bloom = [view(&bloom_textures[0]), view(&bloom_textures[1])];
		let streak = [view(&streak_textures[0]), view(&streak_textures[1])];
		let input_view = view(&input);
		let frame_view = view(&frame);
		let output_view = view(&output);
		let glow_view = view(&glow);
		let clamp_linear = device.create_sampler(&wgpu::SamplerDescriptor {
			label: Some("finish linear"),
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});

		let bright_pipeline = fullscreen_pipeline(
			device,
			"bright",
			include_str!("shaders/bright.wgsl"),
			&[target(BLOOM_FORMAT), target(STREAK_FORMAT)],
		);
		let bright_buffer = uniform_buffer(device, "bright", &BrightUniform::zeroed());
		let bright_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("bright"),
			layout: &bright_pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: bright_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&input_view),
				},
			],
		});

		// Bloom: two box blurs each way, about 2% of the height. Streaks: three long horizontal blurs and a short vertical one.
		let blur_bloom_pipeline = fullscreen_pipeline(
			device,
			"blur bloom",
			include_str!("shaders/blur.wgsl"),
			&[target(BLOOM_FORMAT)],
		);
		let blur_streak_pipeline = fullscreen_pipeline(
			device,
			"blur streak",
			include_str!("shaders/blur.wgsl"),
			&[target(STREAK_FORMAT)],
		);
		let bloom_radius = (coarse_size.1 / 24).max(1) as i32;
		let streak_radius = (coarse_size.0 / 7).max(2) as i32;
		let size = [coarse_size.0 as i32, coarse_size.1 as i32, 0, 0];
		let mut blur_passes = Vec::new();
		let mut add_blur = |label: &'static str, streak_pass: bool, axis: [i32; 2], radius: i32, from: usize| {
			let pipeline = if streak_pass {
				&blur_streak_pipeline
			} else {
				&blur_bloom_pipeline
			};
			let views = if streak_pass { &streak } else { &bloom };
			let buffer = uniform_buffer(
				device,
				label,
				&BlurUniform {
					axis_radius: [axis[0], axis[1], radius, 0],
					size,
				},
			);
			let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
				label: Some(label),
				layout: &pipeline.get_bind_group_layout(0),
				entries: &[
					wgpu::BindGroupEntry {
						binding: 0,
						resource: buffer.as_entire_binding(),
					},
					wgpu::BindGroupEntry {
						binding: 1,
						resource: wgpu::BindingResource::TextureView(&views[from]),
					},
				],
			});
			blur_passes.push(BlurPass {
				label,
				bind_group,
				target: 1 - from,
				streak: streak_pass,
			});
		};
		add_blur("bloom rows 1", false, [1, 0], bloom_radius, 0);
		add_blur("bloom columns 1", false, [0, 1], bloom_radius, 1);
		add_blur("bloom rows 2", false, [1, 0], bloom_radius, 0);
		add_blur("bloom columns 2", false, [0, 1], bloom_radius, 1);
		add_blur("streak rows 1", true, [1, 0], streak_radius, 0);
		add_blur("streak rows 2", true, [1, 0], streak_radius, 1);
		add_blur("streak rows 3", true, [1, 0], streak_radius, 0);
		add_blur("streak columns", true, [0, 1], 1, 1);

		let pack_pipeline = fullscreen_pipeline(device, "pack", include_str!("shaders/pack.wgsl"), &[target(FRAME_FORMAT)]);
		let pack_buffer = uniform_buffer(device, "pack", &PackUniform::zeroed());
		let pack_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("pack"),
			layout: &pack_pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: pack_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&bloom[0]),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(&streak[0]),
				},
			],
		});

		let lay_pipeline = fullscreen_pipeline(device, "lay", include_str!("shaders/lay.wgsl"), &[target(FRAME_FORMAT)]);
		let lay_buffer = uniform_buffer(
			device,
			"lay",
			&LayUniform {
				size: [render_size.0 as f32, render_size.1 as f32, 0.0, 0.0],
			},
		);
		let lay_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("lay"),
			layout: &lay_pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: lay_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&input_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(&glow_view),
				},
				wgpu::BindGroupEntry {
					binding: 3,
					resource: wgpu::BindingResource::Sampler(&clamp_linear),
				},
			],
		});

		let output_pipeline = fullscreen_pipeline(device, "output", include_str!("shaders/output.wgsl"), &[target(FRAME_FORMAT)]);
		let output_buffer = uniform_buffer(device, "output", &OutputUniform::zeroed());
		let output_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("output"),
			layout: &output_pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: output_buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&frame_view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&clamp_linear),
				},
			],
		});

		Finish {
			render_size,
			output_size,
			step,
			coarse_size,
			input,
			input_view,
			output,
			output_view,
			bloom,
			streak,
			glow_view,
			frame_view,
			bright_pipeline,
			bright_buffer,
			bright_group,
			blur_bloom_pipeline,
			blur_streak_pipeline,
			blur_passes,
			pack_pipeline,
			pack_buffer,
			pack_group,
			lay_pipeline,
			lay_group,
			output_pipeline,
			output_buffer,
			output_group,
		}
	}

	/// The texture the scene draws into and the chain reads: render size, `Rgba8Unorm`.
	pub fn input(&self) -> &wgpu::Texture {
		&self.input
	}

	pub fn input_view(&self) -> &wgpu::TextureView {
		&self.input_view
	}

	/// The finished frame: output size, `Rgba8Unorm`, with `COPY_SRC` for a readback or a blit to a window.
	pub fn output(&self) -> &wgpu::Texture {
		&self.output
	}

	pub fn output_size(&self) -> (usize, usize) {
		self.output_size
	}

	pub fn render_size(&self) -> (usize, usize) {
		self.render_size
	}

	/// Records the chain for `params` into `encoder`: input to output. `frame_no` moves the grain.
	pub fn encode(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, params: &FinishParams, frame_no: u32) {
		let (rw, rh) = self.render_size;
		let (ow, oh) = self.output_size;
		let threshold = params.threshold.min(250) as f32;
		gpu.queue.write_buffer(
			&self.bright_buffer,
			0,
			bytemuck::bytes_of(&BrightUniform {
				frame_step: [rw as i32, rh as i32, self.step as i32, (self.step / 2) as i32],
				threshold: [threshold, 255.0 - threshold, 0.0, 0.0],
			}),
		);
		let tint = params.streak_tint;
		gpu.queue.write_buffer(
			&self.pack_buffer,
			0,
			bytemuck::bytes_of(&PackUniform {
				gains: [
					(params.bloom.min(100) * 3) as f32,
					(params.streaks.min(100) * 5) as f32,
					0.0,
					0.0,
				],
				tint: [r(tint) as f32, g(tint) as f32, b(tint) as f32, 0.0],
			}),
		);
		// The letterbox rows, as the CPU works them out.
		let (top, bottom) = if params.letterbox > 0.0 {
			let want = (ow as f32 / params.letterbox) as usize;
			if want < oh {
				let bar = (oh - want) / 2;
				(bar, oh - bar)
			} else {
				(0, oh)
			}
		} else {
			(0, oh)
		};
		let aspect = ow as f32 / oh as f32;
		gpu.queue.write_buffer(
			&self.output_buffer,
			0,
			bytemuck::bytes_of(&OutputUniform {
				size_rows: [ow as f32, oh as f32, top as f32, bottom as f32],
				look: [
					params.vignette.min(100) as f32 / 100.0,
					params.fade.clamp(0.0, 1.0),
					aspect.min(1.6) / 1.6,
					0.0,
				],
				grain: [params.grain.min(100) * 28 / 100, frame_no, 0, 0],
			}),
		);

		run_pass(
			encoder,
			"bright",
			&self.bright_pipeline,
			&self.bright_group,
			&[&self.bloom[0], &self.streak[0]],
		);
		for pass in &self.blur_passes {
			let (pipeline, target) = if pass.streak {
				(&self.blur_streak_pipeline, &self.streak[pass.target])
			} else {
				(&self.blur_bloom_pipeline, &self.bloom[pass.target])
			};
			run_pass(encoder, pass.label, pipeline, &pass.bind_group, &[target]);
		}
		run_pass(encoder, "pack", &self.pack_pipeline, &self.pack_group, &[&self.glow_view]);
		run_pass(encoder, "lay", &self.lay_pipeline, &self.lay_group, &[&self.frame_view]);
		run_pass(
			encoder,
			"output",
			&self.output_pipeline,
			&self.output_group,
			&[&self.output_view],
		);
	}

	/// The coarse size the glow is made at.
	pub fn coarse_size(&self) -> (usize, usize) {
		self.coarse_size
	}
}
