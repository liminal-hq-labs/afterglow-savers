// Puts a finished frame on a window's surface: one full-screen pass with a bilinear filter, so a frame drawn at a render scale
// below 100% is enlarged to the display's native size.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use bytemuck::{Pod, Zeroable};

use crate::context::Gpu;
use crate::finish::{fullscreen_pipeline, target};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlitUniform {
	size: [f32; 4],
}

/// Draws a texture (`Rgba8Unorm`, as the finishing chain's output) over a whole surface texture of `target_format`. The surface
/// format should not be an sRGB one: the frame's values are already display values and are passed through unchanged.
pub struct Presenter {
	pipeline: wgpu::RenderPipeline,
	buffer: wgpu::Buffer,
	group: wgpu::BindGroup,
}

impl Presenter {
	pub fn new(gpu: &Gpu, source: &wgpu::Texture, target_format: wgpu::TextureFormat) -> Presenter {
		let device = &gpu.device;
		let pipeline = fullscreen_pipeline(device, "present", include_str!("shaders/blit.wgsl"), &[target(target_format)]);
		let buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("present"),
			size: size_of::<BlitUniform>() as u64,
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			label: Some("present linear"),
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			..Default::default()
		});
		let view = source.create_view(&wgpu::TextureViewDescriptor::default());
		let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("present"),
			layout: &pipeline.get_bind_group_layout(0),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: buffer.as_entire_binding(),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
			],
		});
		Presenter { pipeline, buffer, group }
	}

	/// Records the pass into `encoder`, drawing onto `target` of `size` pixels.
	pub fn encode(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, size: (u32, u32)) {
		gpu.queue.write_buffer(
			&self.buffer,
			0,
			bytemuck::bytes_of(&BlitUniform {
				size: [size.0 as f32, size.1 as f32, 0.0, 0.0],
			}),
		);
		let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("present"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: target,
				depth_slice: None,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			occlusion_query_set: None,
			multiview_mask: None,
		});
		pass.set_pipeline(&self.pipeline);
		pass.set_bind_group(0, &self.group, &[]);
		pass.draw(0..3, 0..1);
	}
}
