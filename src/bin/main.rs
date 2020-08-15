use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::device::{Device, DeviceExtensions, Features, Queue, QueuesIter};
use vulkano::framebuffer::{RenderPass, Subpass};
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain::{ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use vulkan_test::vulkutil;

fn create_window(instance: &Arc<Instance>) -> (EventLoop<()>, Arc<Surface<Window>>) {
	let events_loop = EventLoop::new();
	let surface = WindowBuilder::new()
		.with_title("Vulkan")
		.build_vk_surface(&events_loop, instance.clone())
		.unwrap();
	(events_loop, surface)
}

fn create_swapchain(physical: PhysicalDevice, device: &Arc<Device>, surface: &Arc<Surface<Window>>, queue: &Arc<Queue>)
		-> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
	let caps = surface.capabilities(physical).unwrap();
	// TODO we probably want to actually pick this properly?
	//      Seems to normally be opaque, but shouldn't rely on that.
	let alpha = caps.supported_composite_alpha.iter().next().unwrap();
	println!("Using alpha mode {:?}", alpha);
	// TODO formats?
	let format = caps.supported_formats[0].0;
	println!("Using format {:?}", format);

	let dimensions: [u32; 2] = surface.window().inner_size().into();

	Swapchain::new(
		device.clone(),
		surface.clone(),
		caps.min_image_count,
		format,
		dimensions,
		1,
		ImageUsage::color_attachment(),
		queue,
		SurfaceTransform::Identity,
		alpha,
		PresentMode::Fifo,
		FullscreenExclusive::Default,
		true,
		ColorSpace::SrgbNonLinear
	).unwrap()
}

fn main() {
	let instance = {
		let extensions = vulkano_win::required_extensions();
		Instance::new(None, &extensions, None)
			.expect("Failed to create Vulkan instance.")
	};

	let physical = vulkutil::select_physical_device(&instance);

	let (events_loop, surface) = create_window(&instance);

	// println!("Available queue families:");
	// let queue_families = physical.queue_families();
	// for family in queue_families {
	// 	println!("{} {} {} {} {}",
	// 		family.queues_count(),
	// 		family.supports_graphics(),
	// 		family.supports_compute(),
	// 		family.explicitly_supports_transfers(),
	// 		family.supports_sparse_binding());
	// }

	let queue_family = physical.queue_families()
		.find(|&q| q.supports_graphics())
		.expect("Failed to find a graphical queue family");

	println!("Selected queue family: {:?}", queue_family);

	let device_ext = DeviceExtensions {
		khr_swapchain: true,
		..DeviceExtensions::none()
	};
	let (device, mut queues) = {
		Device::new(physical, &Features ::none(), &device_ext,
					[(queue_family, 0.5)].iter().cloned()).expect("Failed to create device")
	};

	// We only have one queue
	// TODO use multiple queues?
	let queue = queues.next().unwrap();

	let (mut swapchain, images) =
		create_swapchain(physical, &device, &surface, &queue);

	// TODO move this Vertex struct to somewhere more sensible
	//      ideally move all this buffer stuff somewhere more sensible, really.
	#[derive(Default, Debug, Clone)]
	struct Vertex {
		position: [f32; 2],
	}

	let vertex_buffer = {
		vulkano::impl_vertex!(Vertex, position);

		CpuAccessibleBuffer::from_iter(
			device.clone(),
			// TODO pick actual BufferUsage
			BufferUsage::all(),
			false,
			[
				Vertex {position: [-0.5, -0.25]},
				Vertex {position: [0.0, 0.5]},
				Vertex {position: [0.25, -0.1]},
			].iter().cloned()
		).unwrap()
	};

	// TODO figure out where to put shaders
	//      apparently they don't rebuild when changed unless other things in the file are also changed?
	//      irritating if true.
	mod vs {
		vulkano_shaders::shader! {
			ty: "vertex",
			src: "\
#version 450
layout(location = 0) in vec2 position;
void main() {
	gl_Position = vec4(position, 0.0, 1.0);
}"
		}
	}

	mod fs {
		vulkano_shaders::shader! {
			ty: "fragment",
			src: "\
#version 450
layout(location = 0) out vec4 f_color;
void main() {
	f_color = vec4(0.1, 0.25, 1.0, 1.0);
}"
		}
	}

	let vs = vs::Shader::load(device.clone()).unwrap();
	let fs = fs::Shader::load(device.clone()).unwrap();

	let render_pass: Arc<RenderPass<_>> = Arc::new(
		vulkano::single_pass_renderpass!(
			device.clone(),
			attachments: {
				color: {
					// attachment is cleared upon draw
					load: Clear,
					store: Store,
					format: swapchain.format(),
					// no idea what this does, but the vulkano example uses it
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {}
			}
		).unwrap()
	);

	let pipeline = Arc::new(
		GraphicsPipeline::start()
			.vertex_input_single_buffer::<Vertex>()
			.vertex_shader(vs.main_entry_point(), ())
			.triangle_list()
			.viewports_dynamic_scissors_irrelevant(1)
			.fragment_shader(fs.main_entry_point(), ())
			.render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
			.build(device.clone())
			.unwrap()
	);

	events_loop.run(|event, _, control_flow| {
		// TODO this might break things once we're actually rendering to the surface
		//      for now it just serves to prevent max CPU usage on one thread
		*control_flow = ControlFlow::Wait;
		// Commented out to prevent console spam
		// println!("event {:?}", event);
		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
				*control_flow = ControlFlow::Exit;
			},
			Event::WindowEvent { event: WindowEvent::Resized(size), ..} => {
				println!("resized {:?}", size);
			}
			_ => ()
		}
	});
}
