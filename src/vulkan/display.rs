use super::{
    context::Context,
    image::{Image, ImageView},
    sync::Semaphore,
};
use ash::{
    extensions::khr::{Surface, Swapchain, Win32Surface},
    vk,
};
use glam::UVec2;
use std::{ffi::c_void, sync::Arc};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle},
    window::Window,
};

pub struct Display {
    pub surface_loader: Surface,
    pub surface: vk::SurfaceKHR,

    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,

    pub images: Vec<Image>,
    pub views: Vec<ImageView>,

    pub dims: UVec2,
    pub format: vk::Format,

    pub dpi: f32,
}

impl Display {
    const IMAGE_COUNT: u32 = 3;

    pub fn new(context: Arc<Context>, window: &Window) -> Arc<Self> {
        let surface_loader = Surface::new(&context.entry, &context.instance);
        let surface = create_surface(&context.entry, &context.instance, window);

        let swapchain_loader = Swapchain::new(&context.instance, &context.device);

        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(context.physical, surface)
                .unwrap()
        };
        let format = unsafe {
            surface_loader
                .get_physical_device_surface_formats(context.physical, surface)
                .unwrap()[0]
        };

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(Self::IMAGE_COUNT)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::STORAGE)
            .image_extent(capabilities.current_extent)
            .image_array_layers(1)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::MAILBOX)
            .clipped(true);

        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&create_info, None)
                .unwrap()
        };

        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .unwrap()
                .into_iter()
                .map(|i| Image::from_raw(context.clone(), i))
                .collect::<Vec<Image>>()
        };

        let views = images
            .iter()
            .map(|i| {
                ImageView::new(
                    context.clone(),
                    i,
                    format.format,
                    Image::default_subresource(vk::ImageAspectFlags::COLOR),
                )
            })
            .collect::<Vec<ImageView>>();

        let dims = glam::uvec2(
            capabilities.current_extent.width,
            capabilities.current_extent.height,
        );

        let dpi = window.scale_factor() as f32;

        Arc::new(Self {
            surface_loader,
            surface,
            swapchain_loader,
            swapchain,
            images,
            views,
            dims,
            format: format.format,
            dpi,
        })
    }

    pub fn frames_in_flight(&self) -> usize {
        Self::IMAGE_COUNT as usize
    }

    pub fn present(&self, context: &Context, index: u32, wait_semaphore: &Semaphore) {
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(&wait_semaphore.handle))
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(std::slice::from_ref(&index));

        unsafe {
            self.swapchain_loader
                .queue_present(context.queue, &present_info)
                .unwrap()
        };
    }

    pub fn acquire_next_image(&self, signal: &Semaphore) -> (u32, bool) {
        unsafe {
            self.swapchain_loader
                .acquire_next_image(self.swapchain, u64::MAX, signal.handle, vk::Fence::null())
                .unwrap()
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

// Ported from ash-window
// https://crates.io/crates/ash-window
// License : MIT (https://github.com/ash-rs/ash/blob/master/LICENSE-MIT)
pub fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &Window,
) -> vk::SurfaceKHR {
    let display_handle = window.display_handle().unwrap().as_raw();
    let window_handle = window.window_handle().unwrap().as_raw();

    match (display_handle, window_handle) {
        (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(handle)) => {
            let hwnd = std::ptr::NonNull::new(handle.hwnd.get() as *mut c_void).unwrap();
            let hinstance =
                std::ptr::NonNull::new(handle.hinstance.unwrap().get() as *mut c_void).unwrap();
            let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
                .hwnd(hwnd.as_ptr())
                .hinstance(hinstance.as_ptr());
            let surface_fn = Win32Surface::new(entry, instance);
            unsafe { surface_fn.create_win32_surface(&create_info, None).unwrap() }
        }

        #[cfg(target_os = "macos")]
        (RawDisplayHandle::AppKit(_), RawWindowHandle::AppKit(window)) => {
            use raw_window_metal::{appkit, Layer};

            let layer = match unsafe { appkit::metal_layer_from_handle(window) } {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::builder().layer(unsafe { &*layer });
            let surface_fn = MetalSurface::new(entry, instance);
            unsafe {
                surface_fn
                    .create_metal_surface(&surface_desc, None)
                    .unwrap()
            }
        }

        _ => panic!("Unsupported platform"),
    }
}
