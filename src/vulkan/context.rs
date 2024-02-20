use super::{
    command::CommandList,
    sync::{Fence, Semaphore},
};
use ash::{
    extensions::{
        ext::MetalSurface,
        khr::{AccelerationStructure, Surface, Win32Surface},
    },
    vk,
};
use gpu_allocator::vulkan::Allocator;
use parking_lot::Mutex;
use std::{
    ffi::{c_char, CString},
    mem::ManuallyDrop,
};
use winit::{
    raw_window_handle::{HasDisplayHandle, RawDisplayHandle},
    window::Window,
};

// The Context is the central structure of the Vulkan backend.
// It contains the Vulkan instance, physical device, logical device, queue, and allocator.
// which are explained in the following sections.
pub struct Context {
    // A handle to the Vulkan DLL to load function pointers from.
    pub entry: ash::Entry,
    // A handle to the Vulkan instance which is used to create the logical device.
    pub instance: ash::Instance,
    // A handle to the physical device which is used to create the logical device.
    pub physical: vk::PhysicalDevice,
    // A logiocal device is the interface of which we communicate with a GPU.
    // It is used to create command buffers, allocate memory, and create resources.
    pub device: ash::Device,

    // GPUs have multiple queues which yhou can subnmiut work to. Each queue family has ddifferent capabilities.
    // this is the index of the queue family that we use to submit graphics commands.
    pub queue_family: u32,
    // A handle to the queue that we use to submit graphics commands.
    pub queue: vk::Queue,

    // The allocator is used to allocate memory for resources.
    pub allocator: ManuallyDrop<Mutex<Allocator>>,

    // A handle to the extension that is used to create acceleration structures.
    pub acceleration_structures: ash::extensions::khr::AccelerationStructure,
}

impl Context {
    // Creates a new Vulkan context.
    pub fn new(window: &Window) -> Self {
        let entry = unsafe { ash::Entry::load() }.unwrap();

        // Create the Vulkan instance
        let instance = create_instance(&entry, window);
        // Pick the first physical device
        let physical = pick_physical(&instance);
        // Get the queue family that supports graphics commands
        let queue_family = get_queue_family(&instance, physical);
        // Create the logical device
        let device = create_device(&instance, physical, queue_family);
        // Get the queue that we use to submit graphics commands
        let queue = get_queue(&device, queue_family);

        // Initialize the allocator
        let allocator = {
            let create_info = gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device: physical,
                debug_settings: Default::default(),
                buffer_device_address: true,
                allocation_sizes: Default::default(),
            };
            ManuallyDrop::new(Mutex::new(Allocator::new(&create_info).unwrap()))
        };

        // Create the acceleration structure extension
        let acceleration_structures = AccelerationStructure::new(&instance, &device);

        // Return the context
        Self {
            entry,
            instance,
            physical,
            device,
            queue_family,
            queue,
            allocator,
            acceleration_structures,
        }
    }

    // Submit a list of command buffers to the queue.
    pub fn submit(
        &self,
        submits: &[CommandList],
        wait: Option<&Semaphore>,
        signal: Option<&Semaphore>,
        fence: Option<&Fence>,
    ) {

        // Get the handles of the command buffers
        let mut command_buffers = Vec::with_capacity(submits.len());
        for cmd in submits {
            command_buffers.push(cmd.handle)
        }

        // Get the handles of the semaphores
        let wait_handle = wait.map_or(vk::Semaphore::null(), |s| s.handle);
        let signal_handle = signal.map_or(vk::Semaphore::null(), |s| s.handle);

        // Create the submit info
        let mut submit = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_semaphores(std::slice::from_ref(&wait_handle))
            .signal_semaphores(std::slice::from_ref(&signal_handle))
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::ALL_COMMANDS])
            .build();

        // If the semaphores are not provided, set the count to 0
        if wait.is_none() {
            submit.wait_semaphore_count = 0
        }

        if signal.is_none() {
            submit.signal_semaphore_count = 0
        }

        // Submit the command buffers to the queue
        unsafe {
            self.device
                .queue_submit(
                    self.queue,
                    &[submit],
                    fence.map_or(vk::Fence::null(), |f| f.handle),
                )
                .unwrap();
        }
    }

    // Wait for the device to finish all work.
    pub fn wait_idle(&self) {
        unsafe { self.device.device_wait_idle().unwrap() }
    }
}

impl Drop for Context {
    // Destroys the Vulkan context.
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            ManuallyDrop::drop(&mut self.allocator);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

// Ported from ash-window
// https://crates.io/crates/ash-window
// License : MIT (https://github.com/ash-rs/ash/blob/master/LICENSE-MIT)
pub fn get_window_extensions(window: &Window) -> &'static [*const c_char] {
    let handle = window.display_handle().unwrap().as_raw();

    match handle {
        RawDisplayHandle::AppKit(_) => {
            const METAL: [*const c_char; 2] =
                [Surface::name().as_ptr(), MetalSurface::name().as_ptr()];

            return &METAL;
        }

        RawDisplayHandle::Windows(_) => {
            const WINDOWS: [*const c_char; 2] =
                [Surface::name().as_ptr(), Win32Surface::name().as_ptr()];

            &WINDOWS
        }

        _ => panic!("Unsupported platform"),
    }
}

// Create a Vulkan instance
pub fn create_instance(entry: &ash::Entry, window: &Window) -> ash::Instance {
    // The name of the application
    let app_name = CString::new("duckyboo").unwrap();

    // The information about the application
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&app_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_3);

    // The extensions that are required by the window
    let mut extensions = Vec::new();
    extensions.extend_from_slice(&get_window_extensions(window));

    // Create the Vulkan instance
    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_extension_names(&extensions);

    unsafe { entry.create_instance(&create_info, None).unwrap() }
}

// Pick the first physical device
pub fn pick_physical(instance: &ash::Instance) -> vk::PhysicalDevice {
    unsafe { instance.enumerate_physical_devices().unwrap().remove(0) }
}

// Get the queue family that supports graphics commands
pub fn get_queue_family(instance: &ash::Instance, physical: vk::PhysicalDevice) -> u32 {
    // Query the queue families of the physical device
    let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };

    // Functionally iterate over the queue families and find the first one that supports graphics commands
    let family = families
        .into_iter()
        .zip(0u32..)
        .into_iter()
        .find(|(properties, _family)| properties.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .unwrap()
        .1;

    family
}

// Create a logical device
pub fn create_device(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    family: u32,
) -> ash::Device {
    // Create the queue info
    let queue_infos = [vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(family)
        .queue_priorities(&[1.0])
        .build()];

        // Define the extensions that are required by the device
    let extensions = [
        ash::extensions::khr::Swapchain::name().as_ptr(),
        ash::extensions::khr::AccelerationStructure::name().as_ptr(),
        ash::extensions::khr::DeferredHostOperations::name().as_ptr(),
        vk::KhrRayQueryFn::name().as_ptr(),
        vk::KhrRayTracingPositionFetchFn::name().as_ptr(),
    ];

    // Define the features that are required by the device

    let mut features_1_3 = vk::PhysicalDeviceVulkan13Features::builder()
        .dynamic_rendering(true)
        .synchronization2(true)
        .build();

    let mut features_1_2 = vk::PhysicalDeviceVulkan12Features::builder()
        .buffer_device_address(true)
        .buffer_device_address_capture_replay(true)
        .build();

    let mut features_as = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
        .acceleration_structure(true)
        .acceleration_structure_capture_replay(true)
        .build();

    let mut features_rq = vk::PhysicalDeviceRayQueryFeaturesKHR::builder().ray_query(true);

    let mut features_rqpf = vk::PhysicalDeviceRayTracingPositionFetchFeaturesKHR::builder()
        .ray_tracing_position_fetch(true);

        // Create the logical device
    let create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&extensions)
        .push_next(&mut features_1_2)
        .push_next(&mut features_1_3)
        .push_next(&mut features_as)
        .push_next(&mut features_rq)
        .push_next(&mut features_rqpf);

    unsafe {
        instance
            .create_device(physical, &create_info, None)
            .unwrap()
    }
}

// Get the queue that we use to submit graphics commands
pub fn get_queue(device: &ash::Device, family: u32) -> vk::Queue {
    let graphics = unsafe { device.get_device_queue(family, 0) };
    graphics
}
