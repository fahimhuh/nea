use ash::{
    extensions::{
        ext::MetalSurface,
        khr::{Surface, Win32Surface},
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

pub struct Context {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical: vk::PhysicalDevice,
    pub device: ash::Device,

    pub queue_family: u32,
    pub queue: vk::Queue,

    pub allocator: ManuallyDrop<Mutex<Allocator>>,
}

impl Context {
    pub fn new(window: &Window) -> Self {
        let entry = unsafe { ash::Entry::load() }.unwrap();

        let instance = create_instance(&entry, window);
        let physical = pick_physical(&instance);
        let queue_family = get_queue_family(&instance, physical);
        let device = create_device(&instance, physical, queue_family);
        let queue = get_queue(&device, queue_family);

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

        Self {
            entry,
            instance,
            physical,
            device,
            queue_family,
            queue,
            allocator,
        }
    }

    unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();
        ManuallyDrop::drop(&mut self.allocator);
        self.device.destroy_device(None);
        self.instance.destroy_instance(None);
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.destroy();
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

pub fn create_instance(entry: &ash::Entry, window: &Window) -> ash::Instance {
    let app_name = CString::new("duckyboo").unwrap();

    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&app_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_3);

    let mut extensions = Vec::new();
    extensions.extend_from_slice(&get_window_extensions(window));

    #[allow(unused_mut)]
    let mut flags = vk::InstanceCreateFlags::empty();

    cfg_if::cfg_if! {
        if #[cfg(target_os = "macos")] {
            extensions.push(vk::KhrPortabilityEnumerationFn::name().as_ptr());
            extensions.push(vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
            flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
        }
    }

    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&app_info)
        .enabled_extension_names(&extensions)
        .flags(flags);

    unsafe { entry.create_instance(&create_info, None).unwrap() }
}

pub fn pick_physical(instance: &ash::Instance) -> vk::PhysicalDevice {
    unsafe { instance.enumerate_physical_devices().unwrap().remove(0) }
}

pub fn get_queue_family(instance: &ash::Instance, physical: vk::PhysicalDevice) -> u32 {
    let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };
    let family = families
        .into_iter()
        .zip(0u32..)
        .into_iter()
        .find(|(properties, _family)| properties.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .unwrap()
        .1;

    family
}

pub fn create_device(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    family: u32,
) -> ash::Device {
    let queue_infos = [vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(family)
        .queue_priorities(&[1.0])
        .build()];

    let extensions = [
        ash::extensions::khr::Swapchain::name().as_ptr(),
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        vk::KhrPortabilitySubsetFn::name().as_ptr(),
    ];

    let create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&extensions);

    unsafe {
        instance
            .create_device(physical, &create_info, None)
            .unwrap()
    }
}

pub fn get_queue(device: &ash::Device, family: u32) -> vk::Queue {
    let graphics = unsafe { device.get_device_queue(family, 0) };
    graphics
}
