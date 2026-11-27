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
	/// The adapter the device came from, for asking a surface what it supports.
	pub adapter: wgpu::Adapter,
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

/// Picks the adapter: one named like `wanted`, else the first discrete or integrated one (that can draw to `surface`, when there
/// is one), else the first.
fn pick_adapter(
	adapters: &mut Vec<wgpu::Adapter>,
	wanted: Option<&str>,
	surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter, GpuError> {
	if adapters.is_empty() {
		return Err(GpuError::NoAdapter("the instance lists none".into()));
	}
	let usable = |adapter: &wgpu::Adapter| surface.is_none_or(|surface| adapter.is_surface_supported(surface));
	let pick = match wanted {
		Some(name) => {
			let name = name.to_lowercase();
			adapters
				.iter()
				.position(|a| a.get_info().name.to_lowercase().contains(&name) && usable(a))
				.ok_or_else(|| GpuError::NoAdapter(format!("none is named like \"{name}\"")))?
		}
		None => adapters
			.iter()
			.position(|a| {
				matches!(
					a.get_info().device_type,
					wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
				) && usable(a)
			})
			.or_else(|| adapters.iter().position(usable))
			.ok_or_else(|| GpuError::NoAdapter("none can draw to the window".into()))?,
	};
	Ok(adapters.swap_remove(pick))
}

fn name_from_environment(wanted: Option<&str>) -> Option<String> {
	wanted
		.map(str::to_string)
		.or_else(|| std::env::var("AFTERGLOW_GPU_ADAPTER").ok())
		.filter(|name| !name.is_empty())
}

fn make_device(adapter: wgpu::Adapter) -> Result<Gpu, GpuError> {
	let info = adapter.get_info();
	let (device, queue) = pollster::block_on(adapter.clone().request_device(&wgpu::DeviceDescriptor {
		label: Some("afterglow twilight"),
		..Default::default()
	}))
	.map_err(|e| GpuError::Device(e.to_string()))?;
	Ok(Gpu {
		device,
		queue,
		info,
		adapter,
	})
}

impl Gpu {
	/// A device on the first adapter whose name contains `wanted` (ignoring case), or, with `None`, on the environment's choice
	/// (`AFTERGLOW_GPU_ADAPTER`), or else the first discrete or integrated adapter and then anything. Software drivers such as
	/// lavapipe are found by name: `Gpu::new(Some("llvmpipe"))`.
	pub fn new(wanted: Option<&str>) -> Result<Gpu, GpuError> {
		let wanted = name_from_environment(wanted);
		let instance = instance();
		let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
		make_device(pick_adapter(&mut adapters, wanted.as_deref(), None)?)
	}

	/// The same, for a window: the instance is made with the window system's display handle, `make_surface` creates the surface
	/// on it, and the adapter chosen is one that can draw to that surface.
	pub fn with_surface(
		wanted: Option<&str>,
		display: Box<dyn wgpu::wgt::WgpuHasDisplayHandle>,
		make_surface: impl FnOnce(&wgpu::Instance) -> Result<wgpu::Surface<'static>, wgpu::CreateSurfaceError>,
	) -> Result<(Gpu, wgpu::Surface<'static>), GpuError> {
		let wanted = name_from_environment(wanted);
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(display));
		let surface = make_surface(&instance).map_err(|e| GpuError::NoAdapter(format!("the window has no surface: {e}")))?;
		let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
		let gpu = make_device(pick_adapter(&mut adapters, wanted.as_deref(), Some(&surface))?)?;
		Ok((gpu, surface))
	}

	/// The adapter's name and driver, for a log line.
	pub fn describe(&self) -> String {
		format!("{} ({:?}, {})", self.info.name, self.info.backend, self.info.driver)
	}
}
