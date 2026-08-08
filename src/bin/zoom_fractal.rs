use std::sync::Arc;

use vulkano::{VulkanLibrary};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueFlags, QueueCreateInfo};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::memory::allocator::{StandardMemoryAllocator, AllocationCreateInfo, MemoryTypeFilter};

use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;

use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo,
};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};

use vulkano::sync::{self, GpuFuture};

use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    Pipeline,
    ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo, PipelineBindPoint,
};

use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::image::view::ImageView;
use vulkano::format::Format;

use image::{ImageBuffer, Rgba};

use vulkano::swapchain::Surface;
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};
use winit::window::WindowBuilder;

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


    let (physical_device, queue_family_index) = select_physical_device(
        &instance, 
        &surface, 
        &device_extensions,
    );

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
            ..Default::default()
        },
    )
    .expect("Failed to create device");

    let queue = queues.next().unwrap();

    let (mut swapchain, images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
            image_format,
            image_extent: dimensions.into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::STORAGE, // What the images are going to be used for
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

                // mandelbrot set definition is values of C that diverge in f(z) = z^2 + c as we smoothly
                // iterate on z
                void main() {
                    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
                    vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - vec2(1.0, 0.0);

                    vec2 z = vec2(0.0, 0.0);
                    float i;
                    for (i = 0.0; i < 1.0; i += 0.005) {
                        z = vec2 (
                            z.x * z.x - z.y * z.y + c.x,
                            z.y * z.x + z.x * z.y + c.y 
                        );

                        if (length(z) > 4.0) {
                            break;
                        }
                    }

                    vec4 to_write = vec4(vec3(i), 1.0);
                    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
                }
            ",
        }
    }

    let shader = cs::load(device.clone()).expect("failed to create shader module");

    // Image Stuff
    let image = Image::new(

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


    let cs = shader.entry_point("main").unwrap();
    let stage = PipelineShaderStageCreateInfo::new(cs);
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
            .into_pipeline_layout_create_info(device.clone())
            .unwrap(),
    )
    .unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        None,
        ComputePipelineCreateInfo::stage_layout(stage, layout),
    )
    .expect("failed to create compute pipeline");

    let view = ImageView::new_default(image.clone()).unwrap();

    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();

    let descriptor_set_allocator = 
        StandardDescriptorSetAllocator::new(device.clone(), Default::default());

    let set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        layout.clone(),
        [WriteDescriptorSet::image_view(0, view.clone())], // 0 is the binding
        [],
    )
    .unwrap();

    
    let mut builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    let buf = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST, 
            ..Default::default()
        },
        AllocationCreateInfo { 
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .expect("failed to create buffer");

    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .unwrap()
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0,
            set,
        )
        .unwrap()
        .dispatch([1024 / 8, 1024 / 8, 1])
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
            image.clone(),
            buf.clone(),
        ))    
        .unwrap();

    let command_buffer = builder.build().unwrap();

    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let resulting_image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    resulting_image.save("fractal.png").unwrap();

    println!("Success!!!");

    event_loop.run(|event, _, control_flow| {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            },
            _ => ()
        }
    });

}
