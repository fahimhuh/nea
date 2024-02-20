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

/// Represents a Vulkan display, which is a window surface and swapchain for rendering to a window.
/// The display is responsible for presenting images to the window.
/// A swapchain is a collection of images that are used as the back buffer for rendering.
pub struct Display {
    /// The surface loader is an instance of the Vulkan surface extension functions.
    pub surface_loader: Surface,
    /// The surface is a handle to the Vulkan surface object.
    pub surface: vk::SurfaceKHR,

    /// The swapchain loader is an instance of the Vulkan swapchain extension functions.
    pub swapchain_loader: Swapchain,
    /// The swapchain is a handle to the Vulkan swapchain object.
    pub swapchain: vk::SwapchainKHR,

    /// The images are the images in the swapchain.
    pub images: Vec<Image>,
    /// The views are the image views for the images in the swapchain.
    pub views: Vec<ImageView>,

    /// The dims are the dimensions of the swapchain images.
    pub dims: UVec2,
    /// The format is the format of the swapchain images.
    pub format: vk::Format,

    /// The dpi is the dots per inch of the window.
    pub dpi: f32,
}

impl Display {
    const IMAGE_COUNT: u32 = 3;

    /// Creates a new display with the specified context and window.
    pub fn new(context: Arc<Context>, window: &Window) -> Arc<Self> {
        // Load the surface extension functions and create the surface.
        let surface_loader = Surface::new(&context.entry, &context.instance);
        let surface = create_surface(&context.entry, &context.instance, window);

        // Load the swapchain extension functions
        let swapchain_loader = Swapchain::new(&context.instance, &context.device);

        // Get the surface capabilities and format
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

        // Create the swapchain
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

        // Get the swapchain images and create image views for them
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

    /// Returns the number of frames in flight for the display.
    pub fn frames_in_flight(&self) -> usize {
        Self::IMAGE_COUNT as usize
    }

    /// Show a swapchain image on the display.
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

    /// Acquires the next image in the swapchain and returns its index and whether it was acquired.
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
        // Destroy the swapchain and surface.
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

        _ => panic!("Unsupported platform"),
    }
}
