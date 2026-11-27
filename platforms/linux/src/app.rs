// The window: fullscreen saver, test window, preview, or a child of xscreensaver's window; frame pacing, input and exit rules.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;
use std::time::{Duration, Instant};

use savers_core::settings::Settings;
use savers_gpu::{Gpu, Presenter, TwilightRenderer};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11};
use winit::window::{Fullscreen, Window, WindowId};

use crate::args::{Mode, Options};
use crate::audio::{self, Playback};
use crate::render;

/// Movement of the pointer, in pixels, that ends the saver (SPEC section 4).
const MOVE_TO_EXIT: f64 = 10.0;
/// How long after the window opens the pointer's reports only set the place movement is measured from.
const SETTLE: Duration = Duration::from_millis(500);
/// The size of the `--preview` window.
const PREVIEW_SIZE: (u32, u32) = (320, 180);
/// The bounds of the window a hack is given when its size cannot be read.
const FALLBACK_SIZE: (u32, u32) = (640, 360);

/// Everything one run needs, decided before the window exists.
pub struct Plan {
	pub options: Options,
	pub settings: Settings,
	pub seed: u32,
	pub frame_cap: u32,
	pub sound: bool,
}

/// What `run` ends with.
pub enum Outcome {
	Finished,
	Failed(String),
}

pub fn run(plan: Plan) -> Outcome {
	// A window of xscreensaver's is an X11 window, so a hack talks X11 even in a Wayland session (through Xwayland).
	let mut builder = EventLoop::builder();
	if matches!(plan.options.mode, Mode::Embed(_) | Mode::EmbedFromEnvironment) {
		builder.with_x11();
	}
	let event_loop = match builder.build() {
		Ok(event_loop) => event_loop,
		Err(error) => return Outcome::Failed(format!("no window system (set DISPLAY or WAYLAND_DISPLAY): {error}")),
	};
	event_loop.set_control_flow(ControlFlow::Poll);
	let mut app = App {
		plan,
		running: None,
		failure: None,
	};
	if let Err(error) = event_loop.run_app(&mut app) {
		return Outcome::Failed(error.to_string());
	}
	match app.failure {
		Some(why) => Outcome::Failed(why),
		None => Outcome::Finished,
	}
}

struct App {
	plan: Plan,
	running: Option<Running>,
	failure: Option<String>,
}

struct Running {
	window: Arc<Window>,
	surface: wgpu::Surface<'static>,
	surface_config: wgpu::SurfaceConfiguration,
	gpu: Gpu,
	renderer: TwilightRenderer,
	presenter: Presenter,
	started: Instant,
	frames: u64,
	next_frame: Instant,
	interval: Option<Duration>,
	/// Where pointer movement is measured from, and whether that report came after the window settled.
	anchor: Option<(PhysicalPosition<f64>, bool)>,
	/// Whether any key, button or movement ends this run.
	ends_on_input: bool,
	ends_on_movement: bool,
	log_mark: (Instant, u64),
	verbose: bool,
	freeze: Option<f64>,
	speed: f64,
	_audio: Option<Playback>,
}

impl ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.running.is_some() {
			return;
		}
		match start(event_loop, &self.plan) {
			Ok(running) => self.running = Some(running),
			Err(why) => {
				self.failure = Some(why);
				event_loop.exit();
			}
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		let Some(running) = self.running.as_mut() else {
			return;
		};
		match event {
			WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
			WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed && running.ends_on_input => {
				running.say("ended by a key");
				event_loop.exit()
			}
			WindowEvent::MouseInput {
				state: ElementState::Pressed,
				..
			} if running.ends_on_movement => {
				running.say("ended by a button");
				event_loop.exit()
			}
			WindowEvent::CursorMoved { position, .. } if running.ends_on_movement => {
				// The pointer's first report once the window has settled is where movement is measured from: the reports while
				// it comes up can be stale.
				let settled = running.started.elapsed() >= SETTLE;
				match running.anchor {
					Some((anchor, true)) => {
						let moved = (position.x - anchor.x).hypot(position.y - anchor.y);
						if moved > MOVE_TO_EXIT {
							running.say(&format!("ended by {moved:.0} pixels of movement"));
							event_loop.exit();
						}
					}
					_ => running.anchor = Some((position, settled)),
				}
			}
			WindowEvent::Resized(size) => running.reconfigure(size),
			WindowEvent::RedrawRequested => {
				running.draw();
				if self.plan.options.frames.is_some_and(|limit| running.frames >= limit) {
					event_loop.exit();
				}
			}
			_ => {}
		}
	}

	fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
		let Some(running) = self.running.as_ref() else {
			return;
		};
		let now = Instant::now();
		if running.interval.is_none() || now >= running.next_frame {
			running.window.request_redraw();
			event_loop.set_control_flow(ControlFlow::Poll);
		} else {
			event_loop.set_control_flow(ControlFlow::WaitUntil(running.next_frame));
		}
	}
}

/// The window xscreensaver gave us, from the environment, for `-root`.
fn window_id_from_environment() -> Option<u32> {
	let text = std::env::var("XSCREENSAVER_WINDOW").ok()?;
	let text = text.trim();
	match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
		Some(hex) => u32::from_str_radix(hex, 16).ok(),
		None => text.parse().ok(),
	}
}

/// The size of an X11 window we do not own, read with a short connection of our own.
fn x11_window_size(id: u32) -> Option<(u32, u32)> {
	use x11rb::protocol::xproto::ConnectionExt;
	let (connection, _) = x11rb::rust_connection::RustConnection::connect(None).ok()?;
	let geometry = connection.get_geometry(id).ok()?.reply().ok()?;
	Some((geometry.width as u32, geometry.height as u32))
}

fn start(event_loop: &ActiveEventLoop, plan: &Plan) -> Result<Running, String> {
	let options = &plan.options;
	let mut preview = false;
	let mut attributes = Window::default_attributes().with_title("Afterglow Twilight");
	let mut ends_on_input = true;
	let mut ends_on_movement = false;
	let mut windowed = false;
	let mut mode = options.mode.clone();
	if mode == Mode::EmbedFromEnvironment {
		mode = match window_id_from_environment() {
			Some(id) => Mode::Embed(id),
			None => Mode::Fullscreen,
		};
	}
	match mode {
		Mode::Window(width, height) => {
			attributes = attributes.with_inner_size(PhysicalSize::new(width, height));
			ends_on_movement = options.exit_on_input;
			windowed = true;
		}
		Mode::Preview => {
			attributes = attributes
				.with_inner_size(PhysicalSize::new(PREVIEW_SIZE.0, PREVIEW_SIZE.1))
				.with_resizable(false);
			preview = true;
			windowed = true;
		}
		Mode::Embed(id) => {
			let (width, height) = x11_window_size(id).unwrap_or(FALLBACK_SIZE);
			attributes = attributes
				.with_inner_size(PhysicalSize::new(width, height))
				.with_decorations(false)
				.with_embed_parent_window(id);
			// A child of xscreensaver's window: it ends us, and there is no input to end on.
			ends_on_input = false;
			preview = width < 400;
			windowed = true;
		}
		_ => {
			// Fullscreen on the primary monitor. With no window manager to honour that, the explicit size and place still cover it.
			let monitor = event_loop
				.primary_monitor()
				.or_else(|| event_loop.available_monitors().next());
			attributes = attributes
				.with_decorations(false)
				.with_fullscreen(Some(Fullscreen::Borderless(monitor.clone())));
			if let Some(monitor) = monitor {
				attributes = attributes.with_inner_size(monitor.size()).with_position(monitor.position());
			}
			ends_on_movement = true;
		}
	}
	let window = Arc::new(
		event_loop
			.create_window(attributes)
			.map_err(|error| format!("could not open a window: {error}"))?,
	);
	if !windowed {
		window.set_cursor_visible(false);
	}
	if ends_on_input {
		// A window manager gives the new window the keyboard; with none, ask for it, so a key still ends the run.
		window.focus_window();
	}
	let size = window.inner_size();
	let display = (size.width.max(16), size.height.max(9));

	let (gpu, surface) = Gpu::with_surface(options.adapter.as_deref(), Box::new(window.clone()), |instance| {
		instance.create_surface(window.clone())
	})
	.map_err(|error| error.to_string())?;
	let adapter_note = gpu.describe();
	let capabilities = surface.get_capabilities(&gpu.adapter);
	let format = capabilities
		.formats
		.iter()
		.copied()
		.find(|format| !format.is_srgb())
		.or_else(|| capabilities.formats.first().copied())
		.ok_or("the window's surface offers no format")?;
	let interval = (plan.frame_cap > 0).then(|| Duration::from_secs_f64(1.0 / plan.frame_cap as f64));
	let present_mode = if interval.is_some() && capabilities.present_modes.contains(&wgpu::PresentMode::Mailbox) {
		wgpu::PresentMode::Mailbox
	} else {
		wgpu::PresentMode::Fifo
	};
	let surface_config = wgpu::SurfaceConfiguration {
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
		format,
		color_space: wgpu::SurfaceColorSpace::Auto,
		width: display.0,
		height: display.1,
		present_mode,
		desired_maximum_frame_latency: 2,
		alpha_mode: wgpu::CompositeAlphaMode::Auto,
		view_formats: vec![],
	};
	surface.configure(&gpu.device, &surface_config);

	let (renderer, sizes) = render::build(&gpu, &plan.settings, plan.seed, display, preview);
	let presenter = Presenter::new(&gpu, renderer.finish().output(), format);
	if options.verbose {
		eprintln!(
			"afterglow-twilight-gpu: {adapter_note}; display {}x{}, drawn at {}x{}, finished at {}x{}, {:?}, {}",
			sizes.display.0,
			sizes.display.1,
			sizes.render.0,
			sizes.render.1,
			sizes.output.0,
			sizes.output.1,
			present_mode,
			match plan.frame_cap {
				0 => "vsync".to_string(),
				cap => format!("capped at {cap} fps"),
			}
		);
	}

	let sound = if plan.sound && !preview {
		match audio::start(&plan.settings, plan.seed) {
			Ok(playback) => {
				if options.verbose {
					eprintln!("afterglow-twilight-gpu: sound on {}", playback.description);
				}
				Some(playback)
			}
			Err(why) => {
				if options.verbose {
					eprintln!("afterglow-twilight-gpu: no sound: {why}");
				}
				None
			}
		}
	} else {
		None
	};

	let now = Instant::now();
	Ok(Running {
		window,
		surface,
		surface_config,
		gpu,
		renderer,
		presenter,
		started: now,
		frames: 0,
		next_frame: now,
		interval,
		anchor: None,
		ends_on_input,
		ends_on_movement: ends_on_movement && ends_on_input,
		log_mark: (now, 0),
		verbose: options.verbose,
		freeze: options.at,
		speed: plan.settings.speed() as f64,
		_audio: sound,
	})
}

impl Running {
	fn say(&self, what: &str) {
		if self.verbose {
			eprintln!("afterglow-twilight-gpu: {what}");
		}
	}

	fn reconfigure(&mut self, size: PhysicalSize<u32>) {
		if size.width == 0 || size.height == 0 {
			return;
		}
		self.surface_config.width = size.width;
		self.surface_config.height = size.height;
		self.surface.configure(&self.gpu.device, &self.surface_config);
	}

	/// Draws and presents one frame, then works out when the next is due.
	fn draw(&mut self) {
		let elapsed = self.started.elapsed().as_secs_f64();
		// Scene time runs at the Speed setting; `--at` holds it still for tests.
		let scene_time = self.freeze.unwrap_or(elapsed * self.speed);
		let frame = match self.surface.get_current_texture() {
			wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
			wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
				let size = self.window.inner_size();
				self.reconfigure(size);
				return;
			}
			other => {
				if self.verbose {
					eprintln!("afterglow-twilight-gpu: no frame: {other:?}");
				}
				return;
			}
		};
		let mut encoder = self
			.gpu
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
		self.renderer.encode(&self.gpu, &mut encoder, scene_time, elapsed);
		let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
		self.presenter.encode(
			&self.gpu,
			&mut encoder,
			&view,
			(self.surface_config.width, self.surface_config.height),
		);
		self.gpu.queue.submit([encoder.finish()]);
		self.window.pre_present_notify();
		self.gpu.queue.present(frame);
		self.frames += 1;

		let now = Instant::now();
		if let Some(interval) = self.interval {
			self.next_frame = (self.next_frame + interval).max(now);
		}
		if self.verbose && self.frames.is_multiple_of(30) {
			let (since, count) = self.log_mark;
			let seconds = now.duration_since(since).as_secs_f64();
			eprintln!(
				"afterglow-twilight-gpu: frame {} ({:.1} fps over the last {})",
				self.frames,
				(self.frames - count) as f64 / seconds.max(1e-6),
				self.frames - count
			);
			self.log_mark = (now, self.frames);
		}
	}
}
