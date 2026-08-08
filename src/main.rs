#![allow(unused)]
use std::sync::Arc;

use vulkano::{VulkanLibrary, descriptor_set};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::device::{Device, DeviceCreateInfo, QueueFlags, QueueCreateInfo};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferReadGuard, BufferUsage};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};

use vulkano::pipeline::Pipeline;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;

use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo,
};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};

use vulkano::sync::{self, GpuFuture};

use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo};

use vulkano::pipeline::PipelineBindPoint;

use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::format::Format;

use vulkano::command_buffer::{CopyImageToBufferInfo, ClearColorImageInfo};
use vulkano::format::ClearColorValue;

use image::{ImageBuffer, Rgba};


// #[derive(BufferContents)]
// #[repr(C)]
// struct MyStruct {
//     a: u32,
//     b: u32,
// }

fn main() {
    // Load library and create instance
    let library = VulkanLibrary::new().expect("No local Vulkan library/DLL found");
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            ..Default::default()
        },
    ).expect("Failed to create instance");

    // Identify physical device
    let physical_device = instance
        .enumerate_physical_devices()
        .expect("Could not enumerate devices")
        .next()
        .expect("No devices available");

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

    // Create device
    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo { 
            queue_create_infos: vec![QueueCreateInfo {
                 queue_family_index,
                 ..Default::default()
            }],
            ..Default::default()
        },
    )
    .expect("Failed to create device");

    let queue = queues.next().unwrap();
    let memory_allocator = Arc::new(
        StandardMemoryAllocator::new_default(device.clone())
        );

    // Now we have a virtual device, a queue, and a memory allocator. Let's make a buffer.
    
    // let iter = (0..128).map(|_| 5u8); // an iterator that resolves
    //                                   // to a buffer of value 5 in u8s 128 times
    // let buffer = Buffer::from_iter( // resolve to buffer
    //     memory_allocator.clone(),
    //     BufferCreateInfo {
    //         usage: BufferUsage::UNIFORM_BUFFER,
    //         ..Default::default()
    //     },
    //     AllocationCreateInfo {
    //         memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
    //             | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
    //         ..Default::default()
    //     },
    //     iter,
    // )
    // .unwrap();
    //
    // {
    //     let mut content = buffer.write().unwrap();
    //     content[12] = 83;
    //     content[7] = 3;
    // } // lifetimes c:
    //
    // let guard = buffer.read().unwrap();
    // let inner_ref: &[u8] = &*guard;
    //
    // println!("Thingy: {:?}", inner_ref);
    //

    // let source_content = 0..64;
    //
    // let source = Buffer::from_iter( // resolve to buffer
    //     memory_allocator.clone(),
    //     BufferCreateInfo {
    //         usage: BufferUsage::TRANSFER_SRC,
    //         ..Default::default()
    //     },
    //     AllocationCreateInfo {
    //         memory_type_filter: MemoryTypeFilter::PREFER_HOST
    //             | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
    //         ..Default::default()
    //     },
    //     source_content,
    // )
    // .expect("failed to create source buffer");
    //
    // let destination_content = (0..64).map(|_| 0);
    // let destination = Buffer::from_iter( // resolve to buffer
    //     memory_allocator.clone(),
    //     BufferCreateInfo {
    //         usage: BufferUsage::TRANSFER_DST,
    //         ..Default::default()
    //     },
    //     AllocationCreateInfo {
    //         memory_type_filter: MemoryTypeFilter::PREFER_HOST
    //             | MemoryTypeFilter::HOST_RANDOM_ACCESS,
    //         ..Default::default()
    //     },
    //     destination_content,
    // )
    // .expect("failed to create destination buffer");
    //
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );
    //
    // let mut builder = AutoCommandBufferBuilder::primary(
    //     &command_buffer_allocator, 
    //     queue_family_index, 
    //     CommandBufferUsage::OneTimeSubmit,
    // )
    // .unwrap();
    //
    // builder
    //     .copy_buffer(CopyBufferInfo::buffers(source.clone(), destination.clone()))
    //     .unwrap();
    //
    // let command_buffer = builder.build().unwrap();
    //
    // let future = sync::now(device.clone())
    //     .then_execute(queue.clone(), command_buffer.clone())
    //     .unwrap()
    //     .then_signal_fence_and_flush()
    //     .unwrap();
    //
    // future.wait(None).unwrap();
    //
    // let src_content = source.read().unwrap();
    // let destination_content = destination.read().unwrap();
    // assert_eq!(&*src_content, &*destination_content);
    //
    // println!("Source: {:?}", &*src_content);
    // println!("Dest: {:?}", &*destination_content);
    //
    // println!("Success!");
    //
    // Experimenting with compute operations`

    // let data_iter = 0..65536u32;
    // let data_buffer = Buffer::from_iter(
    //     memory_allocator.clone(), 
    //     BufferCreateInfo { 
    //         usage: BufferUsage::STORAGE_BUFFER,
    //         ..Default::default()
    //     },
    //     AllocationCreateInfo {
    //         memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
    //             | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
    //         ..Default::default()
    //     },
    //     data_iter,
    // )
    // .expect("failed to create buffer");
    //
    // mod cs {
    //     vulkano_shaders::shader!{
    //         ty: "compute",
    //         src: r"
    //             #version 460
    //
    //             layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;
    //
    //             layout(set = 0, binding = 0) buffer Data {
    //                 uint data[];
    //             } buf;
    //
    //             void main() {
    //                 uint idx = gl_GlobalInvocationID.x;
    //                 buf.data[idx] *= 12;
    //             }
    //         ",
    //     }
    // }
    //
    // let shader = cs::load(device.clone()).expect("failed to create shader module");
    //
    // let cs = shader.entry_point("main").unwrap();
    // let stage = PipelineShaderStageCreateInfo::new(cs);
    // let layout = PipelineLayout::new(
    //     device.clone(),
    //     PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
    //         .into_pipeline_layout_create_info(device.clone())
    //         .unwrap(),
    // )
    // .unwrap();
    //
    // let compute_pipeline = ComputePipeline::new(
    //     device.clone(),
    //     None,
    //     ComputePipelineCreateInfo::stage_layout(stage, layout),
    // )
    // .expect("failed to create compute pipeline");
    //
    //
    // // descriptor set layout here
    //
    // let descriptor_set_allocator = 
    //     StandardDescriptorSetAllocator::new(device.clone(), Default::default());
    // let pipeline_layout = compute_pipeline.layout();
    // let descriptor_set_layouts = pipeline_layout.set_layouts();
    //
    // let descriptor_set_layout_index = 0;
    // let descriptor_set_layout = descriptor_set_layouts
    //     .get(descriptor_set_layout_index)
    //     .unwrap();
    // let descriptor_set = PersistentDescriptorSet::new(
    //     &descriptor_set_allocator,
    //     descriptor_set_layout.clone(),
    //     [WriteDescriptorSet::buffer(0, data_buffer.clone())],
    //     [],
    // )
    // .unwrap();
    //
    //
    // let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
    //     &command_buffer_allocator, 
    //     queue.queue_family_index(), 
    //     CommandBufferUsage::OneTimeSubmit,
    // )
    // .unwrap();
    //
    // let work_group_counts = [1024, 1, 1];
    //
    // command_buffer_builder
    //     .bind_pipeline_compute(compute_pipeline.clone())
    //     .unwrap()
    //     .bind_descriptor_sets(PipelineBindPoint::Compute,
    //         compute_pipeline.layout().clone(),
    //         descriptor_set_layout_index as u32,
    //         descriptor_set,
    //     )
    //     .unwrap()
    //     .dispatch(work_group_counts)
    //     .unwrap();
    //
    // let command_buffer = command_buffer_builder.build().unwrap();
    //
    // let future = sync::now(device.clone())
    //     .then_execute(queue.clone(), command_buffer)
    //     .unwrap()
    //     .then_signal_fence_and_flush()
    //     .unwrap();
    //
    // future.wait(None).unwrap();
    //
    // let content = data_buffer.read().unwrap();
    //
    // for (n, val) in content.iter().enumerate() {
    //     assert_eq!(*val, n as u32 * 12);
    // }
    //
    // println!("Everything succeeded!");

    // Image stuff

    let image = Image::new(
        memory_allocator.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::R8G8B8A8_UNORM,
            extent: [1024, 1024, 1],
            usage: ImageUsage::TRANSFER_DST | ImageUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo { 
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
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
        .clear_color_image(ClearColorImageInfo {
            clear_value: ClearColorValue::Float([0.0, 0.0, 1.0, 1.0]),
            ..ClearColorImageInfo::image(image.clone())
        })
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

    resulting_image.save("blue_square.png").unwrap();

    println!("Success!!!");
}
