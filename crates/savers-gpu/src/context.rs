// The device: which adapter to draw on, and how to ask for one by name.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt;

/// A device and its queue, with a note of which adapter they came from.
pub struct Gpu {
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub info: wgpu::AdapterInfo,
}

/// Why no device could be made.
#[derive(Debug)]
pub enum GpuError {
	/// No adapter matches (or none exists at all).
	NoAdapter(String),
	/// The adapter refused the device.
	Device(String),
}

impl fmt::Display for GpuError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			GpuError::NoAdapter(why) => write!(f, "no graphics adapter: {why}"),
			GpuError::Device(why) => write!(f, "the graphics adapter would not make a device: {why}"),
		}
	}
}

impl std::error::Error for GpuError {}

fn instance() -> wgpu::Instance {
	wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
}

/// The names of the adapters the machine offers, in the order the driver lists them.
pub fn adapter_names() -> Vec<String> {
	pollster::block_on(instance().enumerate_adapters(wgpu::Backends::all()))
		.iter()
		.map(|adapter| {
			let info = adapter.get_info();
			format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type)
		})
		.collect()
}

impl Gpu {
	/// A device on the first adapter whose name contains `wanted` (ignoring case), or, with `None`, on the environment's choice
	/// (`AFTERGLOW_GPU_ADAPTER`), or else the first discrete or integrated adapter and then anything. Software drivers such as
	/// lavapipe are found by name: `Gpu::new(Some("llvmpipe"))`.
	pub fn new(wanted: Option<&str>) -> Result<Gpu, GpuError> {
		let from_environment = std::env::var("AFTERGLOW_GPU_ADAPTER").ok();
		let wanted = wanted
			.map(str::to_string)
			.or(from_environment)
			.filter(|name| !name.is_empty());
		let instance = instance();
		let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
		if adapters.is_empty() {
			return Err(GpuError::NoAdapter("the instance lists none".into()));
		}
		let pick = match &wanted {
			Some(name) => {
				let name = name.to_lowercase();
				adapters
					.iter()
					.position(|a| a.get_info().name.to_lowercase().contains(&name))
					.ok_or_else(|| GpuError::NoAdapter(format!("none is named like \"{name}\"")))?
			}
			None => adapters
				.iter()
				.position(|a| {
					matches!(
						a.get_info().device_type,
						wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
					)
				})
				.unwrap_or(0),
		};
		let adapter = adapters.swap_remove(pick);
		let info = adapter.get_info();
		let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
			label: Some("afterglow twilight"),
			..Default::default()
		}))
		.map_err(|e| GpuError::Device(e.to_string()))?;
		Ok(Gpu { device, queue, info })
	}

	/// The adapter's name and driver, for a log line.
	pub fn describe(&self) -> String {
		format!("{} ({:?}, {})", self.info.name, self.info.backend, self.info.driver)
	}
}
