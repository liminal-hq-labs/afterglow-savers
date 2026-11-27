// Copies a texture back to the host as a `Frame`, so a GPU frame can be compared with a CPU one.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use savers_core::frame::Frame;

use crate::context::Gpu;

/// Reads an `Rgba8Unorm` texture (which must have `COPY_SRC`) into a frame of 0x00RRGGBB pixels.
pub fn read_frame(gpu: &Gpu, texture: &wgpu::Texture, width: usize, height: usize) -> Frame {
	let row_bytes = width * 4;
	let padded = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
	let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
		label: Some("readback"),
		size: (padded * height) as u64,
		usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
		mapped_at_creation: false,
	});
	let mut encoder = gpu
		.device
		.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("readback") });
	encoder.copy_texture_to_buffer(
		texture.as_image_copy(),
		wgpu::TexelCopyBufferInfo {
			buffer: &buffer,
			layout: wgpu::TexelCopyBufferLayout {
				offset: 0,
				bytes_per_row: Some(padded as u32),
				rows_per_image: Some(height as u32),
			},
		},
		wgpu::Extent3d {
			width: width as u32,
			height: height as u32,
			depth_or_array_layers: 1,
		},
	);
	gpu.queue.submit([encoder.finish()]);
	let slice = buffer.slice(..);
	slice.map_async(wgpu::MapMode::Read, |result| result.expect("the readback buffer maps"));
	gpu.device
		.poll(wgpu::PollType::wait_indefinitely())
		.expect("the device finishes the copy");
	let mapped = slice.get_mapped_range().expect("the readback buffer is mapped");
	let mut frame = Frame::new(width, height);
	for y in 0..height {
		let row = &mapped[y * padded..y * padded + row_bytes];
		for (x, texel) in row.as_chunks::<4>().0.iter().enumerate() {
			frame.px[y * width + x] = (texel[0] as u32) << 16 | (texel[1] as u32) << 8 | texel[2] as u32;
		}
	}
	drop(mapped);
	buffer.unmap();
	frame
}

/// Uploads a frame into an `Rgba8Unorm` texture of the same size (which must have `COPY_DST`).
pub fn write_frame(gpu: &Gpu, texture: &wgpu::Texture, frame: &Frame) {
	let mut bytes = Vec::with_capacity(frame.px.len() * 4);
	for &c in &frame.px {
		bytes.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]);
	}
	gpu.queue.write_texture(
		texture.as_image_copy(),
		&bytes,
		wgpu::TexelCopyBufferLayout {
			offset: 0,
			bytes_per_row: Some(frame.w as u32 * 4),
			rows_per_image: Some(frame.h as u32),
		},
		wgpu::Extent3d {
			width: frame.w as u32,
			height: frame.h as u32,
			depth_or_array_layers: 1,
		},
	);
}
