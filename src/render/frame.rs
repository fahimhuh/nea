use crate::vulkan::{
    command::{CommandList, CommandPool},
    context::Context,
    display::Display,
    image::{Image, ImageView},
    sync::{Fence, Semaphore},
};
use ash::vk;
use std::sync::Arc;

pub struct Frame {
    pub swapchain_ready: Semaphore,
    pub rendering_finished: Semaphore,
    pub inflight: Fence,
    command_pool: CommandPool,
}

pub struct FrameRef<'a> {
    pub frame: &'a Frame,
    pub context: &'a Arc<Context>,
    pub display: &'a Display,
    swapchain_index: u32,
}

impl<'a> FrameRef<'a> {
    pub fn index(&self) -> usize {
        self.swapchain_index as usize
    }

    pub fn image(&self) -> &Image {
        &self.display.images[self.index()]
    }

    pub fn image_view(&self) -> &ImageView {
        &self.display.views[self.index()]
    }

    pub fn allocate_command_list(&self) -> CommandList {
        self.frame.command_pool.allocate()
    }

    pub fn submit(&mut self, cmds: &[CommandList]) {
        self.context.submit(
            cmds,
            Some(&self.frame.swapchain_ready),
            Some(&self.frame.rendering_finished),
            Some(&self.frame.inflight),
        );

        let present = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[self.frame.rendering_finished.handle])
            .swapchains(&[self.display.swapchain])
            .image_indices(&[self.swapchain_index])
            .build();

        unsafe {
            self.display
                .swapchain_loader
                .queue_present(self.context.queue, &present)
                .unwrap();
        }
    }
}

pub struct Frames {
    context: Arc<Context>,
    display: Arc<Display>,
    frames: Vec<Frame>,

    counter: usize,
}

impl Frames {
    pub fn new(context: Arc<Context>, display: Arc<Display>) -> Self {
        let mut frames = Vec::with_capacity(display.frames_in_flight());

        for _ in 0..display.frames_in_flight() {
            let swapchain_ready = Semaphore::new(context.clone());
            let rendering_finished = Semaphore::new(context.clone());
            let inflight = Fence::new(context.clone(), true);

            let command_pool = CommandPool::new(context.clone(), context.queue_family);

            frames.push(Frame {
                swapchain_ready,
                rendering_finished,
                inflight,
                command_pool,
            });
        }

        Self {
            context,
            display,
            frames,
            counter: 0,
        }
    }

    pub fn next(&mut self) -> FrameRef<'_> {
        self.counter = (self.counter + 1) % self.display.frames_in_flight();

        let sync = self.frames.get_mut(self.counter).unwrap();

        sync.inflight.wait_and_reset();
        sync.command_pool.reset();

        let (index, _suboptimal) = self.display.acquire_next_image(&sync.swapchain_ready);

        FrameRef {
            frame: sync,
            context: &self.context,
            display: &self.display,
            swapchain_index: index,
        }
    }
}
