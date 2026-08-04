use std::sync::Arc;

use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::device::QueueFlags;
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferReadGuard, BufferUsage};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};

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
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    // Now we have a virtual device, a queue, and a memory allocator. Let's make a buffer.
    
    let iter = (0..128).map(|_| 5u8); // an iterator that resolves
                                                                      // to a buffer of value 5 in u8s
                                                                      // 128 times
    let buffer = Buffer::from_iter( // resolve to buffer
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::UNIFORM_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        iter,
    )
    .unwrap();

    {
        let mut content = buffer.write().unwrap();
        content[12] = 83;
        content[7] = 3;
    } // lifetimes c:

    let guard = buffer.read().unwrap();
    let inner_ref: &[u8] = &*guard;

    println!("Thingy: {:?}", inner_ref);

}
