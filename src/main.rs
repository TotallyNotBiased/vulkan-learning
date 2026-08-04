use std::sync::Arc;

use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::device::QueueFlags;
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};


fn main() {
    let library = VulkanLibrary::new().expect("No local Vulkan library/DLL found");
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            ..Default::default()
        },
    ).expect("Failed to create instance");

    let physical_device = instance
        .enumerate_physical_devices()
        .expect("Could not enumerate devices")
        .next()
        .expect("No devices available");

    for family in physical_device.queue_family_properties() {
        println!("Found a queue family with {:?} queue(s)", family.queue_count);
    }

    let queue_family_index = physical_device
        .queue_family_properties()
        .iter()
        .position(|queue_family_properties| {
            queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
        })
        .expect("Couldn't find a graphical queue family") as u32;

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

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let data: i32 = 12;
    let buffer = Buffer::from_data(
        memory_allocator.clone(),
        BufferCreateInfo { 
            usage: BufferUsage::UNIFORM_BUFFER,
            ..Default::default()
            },
            AllocationCreateInfo { memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        data,
    )
    .expect("failed to create buffer");


}
