use std::sync::Arc;

use vulkano::{Validated, VulkanError, VulkanLibrary};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueFlags, QueueCreateInfo};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryTypeFilter};

use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;

use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, BlitImageInfo,
};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};

use vulkano::sync::{self, GpuFuture};

use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::{PushConstantRange, PipelineDescriptorSetLayoutCreateInfo};
use vulkano::pipeline::{
    Pipeline,
    ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo, PipelineBindPoint,
};

use vulkano::image::{Image, ImageCreateInfo, ImageType};
use vulkano::image::view::ImageView;
use vulkano::format::Format;

use vulkano::swapchain;
use vulkano::swapchain::Surface;
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo, SwapchainPresentInfo};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, MouseScrollDelta, WindowEvent};
use winit::window::WindowBuilder;

use vulkano::shader::ShaderStages;

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ZoomDriver {
    pub center: [f64; 2],
    pub zoom: f64,
    pub max_iter: f32,
    pub _packing: u32,
}

struct DragState {
    dragging: bool,
    start_cursor: [f64; 2],
    base_center: [f64; 2],
}
    

fn select_physical_device( // boilerplate from vulkano.rs
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                // Find the first first queue family that is suitable.
                // If none is found, `None` is returned to `filter_map`,
                // which disqualifies this physical device.
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,

            // Note that there exists `PhysicalDeviceType::Other`, however,
            // `PhysicalDeviceType` is a non-exhaustive enum. Thus, one should
            // match wildcard `_` to catch all unknown device types.
            _ => 4,
        })
        .expect("no device available")
}

// fn get_render_pass(device: Arc<Device>, swapchain: &Arc<Swapchain>) -> Arc<RenderPass> {
//     vulkano::single_pass_renderpass!(
//         device,
//         attachments: {
//             color: {
//                 format: swapchain.image_format(),
//                 samples: 1,
//                 load_op: Clear,
//                 store_op: Store,
//             },
//         },
//         pass: {
//             color: [color],
//             depth_stencil: {},
//         },
//     )
//     .unwrap()
// }
//
// fn get_framebuffers(
//     images: &[Arc<Image>],
//     render_pass: &Arc<RenderPass>,
// ) -> Vec<Arc<Framebuffer>> {
//     images
//         .iter()
//         .map(|image| {
//             let view = ImageView::new_default(image.clone()).unwrap();
//             Framebuffer::new(
//                 render_pass.clone(),
//                 FramebufferCreateInfo {
//                     attachments: vec![view],
//                     ..Default::default()
//                 },
//             )
//             .unwrap()
//         })
//         .collect::<Vec<_>>()
// }

fn main() {
    // Winit init stuff
    let event_loop = EventLoop::new();
    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

    // let instance_extensions = Instance

    // Load library and create instance
    let library = VulkanLibrary::new().expect("No local Vulkan library/DLL found");
    let required_extensions = Surface::required_extensions(&event_loop);
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    ).expect("Failed to create instance");

    let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();
    
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };


    let (physical_device, _queue_family_index) = select_physical_device(
        &instance, 
        &surface, 
        &device_extensions,
    );

    let capabilities = physical_device.supported_features();

    // make sure that we have fp64 precision in the shader
    assert!(
        capabilities.shader_float64,
        "not supported"
    );

    let device_features = vulkano::device::Features {
        shader_float64: true,
        ..vulkano::device::Features::empty()
    };

    for family in physical_device.queue_family_properties() {
        println!("Found a queue family with {:?} queue(s)", family.queue_count);
    }

    // Identify device queues
    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .position(|queue_family_properties| {
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        })
        .expect("Couldn't find a graphical queue family") as u32;

    let caps = physical_device
        .surface_capabilities(&surface, Default::default())
        .expect("failed to get surface capabilities");

    let dimensions = window.inner_size();
    let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
    let image_format = physical_device
        .surface_formats(&surface, Default::default())
        .unwrap()[0]
        .0;

    use vulkano::image::ImageUsage;


    // Create device
    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo { 
            queue_create_infos: vec![QueueCreateInfo {
                 queue_family_index,
                 ..Default::default()
            }],
            enabled_extensions: device_extensions,
            enabled_features: device_features,
            ..Default::default()
        },
    )
    .expect("Failed to create device");

    let queue = queues.next().unwrap();

    let (mut swapchain, mut images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
            image_format,
            image_extent: dimensions.into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST, // What the images are going to be used for
            composite_alpha,
            ..Default::default()
        },
    )
    .unwrap();

    let memory_allocator = Arc::new(
        StandardMemoryAllocator::new_default(device.clone())
        );
    
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );


    // Setup finished, now for shader stuff


    mod cs {
        vulkano_shaders::shader!{
            ty: "compute",
            src: r"
                #version 460

                layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

                layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

                layout(push_constant) uniform PushConstants {
                    layout (offset = 0) dvec2 center;
                    layout (offset = 16) double zoom;
                    layout (offset = 24) float max_iter;
                } pc;

                // mandelbrot set definition is values of C that diverge in f(z) = z^2 + c as we smoothly
                // iterate on z
                void main() {
                    dvec2 norm_coordinates = (gl_GlobalInvocationID.xy + dvec2(0.5)) / dvec2(imageSize(img));
                    dvec2 c = pc.center + (norm_coordinates - dvec2(0.5)) * (2.0 * exp2(float(pc.zoom)));

                    dvec2 z = dvec2(0.0, 0.0);
                    float i;
                    for (i = 0.0; i < pc.max_iter; i += 1.0) {
                        z = dvec2(
                            z.x * z.x - z.y * z.y + c.x,
                            z.y * z.x + z.x * z.y + c.y
                        );
                        if (dot(vec2(z), vec2(z)) > 16.0) { // bailout radius 4 squared
                            float log_zn = log(float(dot(z, z))) / 2.0;
                            float nu = log(log_zn / log(2.0)) / log(2.0);
                            i = i - nu;
                            break;
                        }
                    }
                    float t = clamp(i / pc.max_iter, 0.0, 1.0);
                    vec4 to_write = vec4(vec3(t), 1.0);
                    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
                }
            ",
        }
    }

    let shader = cs::load(device.clone()).expect("failed to create shader module");

    let offscreen_image = Image::new(
        memory_allocator.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::R8G8B8A8_UNORM,
            extent: [1024, 1024, 1],
            usage: ImageUsage::STORAGE | ImageUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
    )
    .unwrap();

    let offscreen_view = ImageView::new_default(offscreen_image.clone()).unwrap();


    let cs = shader.entry_point("main").unwrap();
    let stage = PipelineShaderStageCreateInfo::new(cs);

    // here we insert push constants into the layout creation info

    let mut layout_create_info = 
        PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
            .into_pipeline_layout_create_info(device.clone())
            .unwrap();

    // Shader reflection approach
    // layout_create_info.push_constant_ranges = stage
    //     .entry_point
    //     .info()
    //     .push_constant_requirements
    //     .clone()
    //     .into_iter()
    //     .collect();

    layout_create_info.push_constant_ranges = vec![PushConstantRange {
        stages: ShaderStages::COMPUTE,
        offset: 0,
        size: std::mem::size_of::<ZoomDriver>() as u32, // 32
    }];

    let layout = PipelineLayout::new(
        device.clone(),
        layout_create_info,
    )
    .unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        None,
        ComputePipelineCreateInfo::stage_layout(stage, layout),
    )
    .expect("failed to create compute pipeline");

    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();

    let descriptor_set_allocator = 
        StandardDescriptorSetAllocator::new(device.clone(), Default::default());

    let set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        layout.clone(),
        [WriteDescriptorSet::image_view(0, offscreen_view.clone())], // 0 is the binding
        [],
    )
    .unwrap();

    let mut window_resized = false;
    let mut recreate_swapchain = false;
    let mut zoom_driver = ZoomDriver {
        center: [0.0, 0.0],
        zoom: 1.0,
        max_iter: 800.0,
        _packing: 0,
    };

    let mut drag_state = DragState { 
        dragging: false, 
        start_cursor: [0.0, 0.0],
        base_center: [0.0, 0.0],
    };

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            window_resized = true;
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput {
                input: winit::event::KeyboardInput{ virtual_keycode : Some(winit::event::VirtualKeyCode::R), ..} ,
                ..
            },
            .. 
        } => {
            zoom_driver.center = [0.0, 0.0];
            zoom_driver.zoom = 1.0;
            zoom_driver.max_iter = 8.0;
        }
        
        Event::WindowEvent { 
            event: WindowEvent::MouseWheel { delta, .. }, .. 
        } => {
            match delta {
                MouseScrollDelta::LineDelta(_, y) => {
                    zoom_driver.zoom -= y as f64/10.0;
                    zoom_driver.max_iter = 800.0 + zoom_driver.zoom.abs() as f32 * 100.0;
                }
                _ => ()
            }
        }
        Event::WindowEvent { 
            event: WindowEvent::MouseInput { 
                button: winit::event::MouseButton::Left, 
                state: winit::event::ElementState::Pressed,
                ..
            }, 
            .. 
        } => {
            drag_state.dragging = true;
        }
        Event::WindowEvent { 
            event: WindowEvent::MouseInput { 
                button: winit::event::MouseButton::Left, 
                state: winit::event::ElementState::Released,
                ..
            }, 
            .. 
        } => {
            drag_state.dragging = false;
        }
        Event::WindowEvent { 
            event: WindowEvent::CursorMoved { 
                position, ..
            }, 
            ..
        } => {
            if !drag_state.dragging {
                drag_state.start_cursor = [position.x, position.y];
                drag_state.base_center = zoom_driver.center;
            }

            if drag_state.dragging {
                let image_width = 512.0;
                let scale = (2.0 * f64::exp2(zoom_driver.zoom)) / image_width;

                let dx = (drag_state.start_cursor[0] - position.x)*scale;
                let dy = (drag_state.start_cursor[1] - position.y)*scale;
                zoom_driver.center[0] = drag_state.base_center[0] + dx; 
                zoom_driver.center[1] = drag_state.base_center[1] + dy;             
            }

            println!("Position: x: {}, y: {}, dragging: {}, zoom: {:?}", position.x, position.y, drag_state.dragging, zoom_driver.zoom);

        }
        Event::MainEventsCleared => {
            if window_resized || recreate_swapchain {
                recreate_swapchain = false;
                window_resized = false;

                let new_dimensions = window.inner_size();

                let (new_swapchain, new_images) = swapchain
                    .recreate(SwapchainCreateInfo {
                        image_extent: new_dimensions.into(),
                        ..swapchain.create_info()
                    })
                    .expect("failed to recreate swapchain: {e}");
                swapchain = new_swapchain;
                images = new_images;
            }
            
            let (image_i, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None)
                    .map_err(Validated::unwrap)
                {
                    Ok(r) => r,
                    Err(VulkanError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("failed to acquire next image: {e}"),
                };

            if suboptimal {
                recreate_swapchain = true;
            }
            
            let mut builder = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();


            builder
                .bind_pipeline_compute(compute_pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Compute,
                    compute_pipeline.layout().clone(),
                    0,
                    set.clone(),
                )
                .unwrap()
                .push_constants(compute_pipeline.layout().clone(), 0, zoom_driver)
                .unwrap()
                .dispatch([1024 / 8, 1024 / 8, 1])
                .unwrap();

            builder
                .blit_image(BlitImageInfo {
                    filter: vulkano::image::sampler::Filter::Nearest, 
                    ..BlitImageInfo::images(
                        offscreen_image.clone(),
                        images[image_i as usize].clone(),
                    )
                })
                .unwrap();

            let command_buffer = builder.build().unwrap();
            let execution = sync::now(device.clone())
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer) 
                .unwrap()
                .then_swapchain_present(
                    queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i),
                )
                .then_signal_fence_and_flush();

            match execution.map_err(Validated::unwrap) {
                Ok(future) => {
                    // Wait for the GPU to finish.
                    future.wait(None).unwrap();
                }
                Err(VulkanError::OutOfDate) => {
                    recreate_swapchain = true;
                }
                Err(e) => {
                    println!("failed to flush future: {e}");
                }
            }
        }
        _ => (),
    });
}
