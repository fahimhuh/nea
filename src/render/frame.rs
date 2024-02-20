use crate::vulkan::{
    command::{CommandList, CommandPool},
    context::Context,
    display::Display,
    image::{Image, ImageView},
    sync::{Fence, Semaphore},
};

use std::sync::Arc;

// The `Frame` struct contains the Vulkan resources required to render a single frame
// It contains the sychronisation objects required to render the frame, along with the command pool
// as required by the Vulkan specification.
pub struct Frame {
    // The swapchain ready semaphore is used to signal when the swapchain is ready to be presented
    pub swapchain_ready: Semaphore,
    // The rendering finished semaphore is used to signal when the rendering is finished
    pub rendering_finished: Semaphore,
    // The inflight fence is used to signal when the frame is finished rendering
    pub inflight: Fence,
    // The command pool is used to allocate command buffers for the frame
    command_pool: CommandPool,
}

// The `FrameRef` struct is a reference to a `Frame` object, and is used to render a single frame
// It also contains references to the `Context` and `Display` objects, along with the swapchain index
// of the frame as helpers to render the frame.

// The esoteric 'a syntax is used by rust to show that all the objects within the struct are references that
// last as long as the lifetime of the struct, to prevent dangling pointers.

// But in laymans terms, its just a means of preventing memory unsafety.
pub struct FrameRef<'a> {
    pub frame: &'a Frame,
    pub context: &'a Arc<Context>,
    pub display: &'a Display,
    swapchain_index: u32,
}

impl<'a> FrameRef<'a> {
    // This function is used to get the index of the swapchain image that is being rendered
    // and returns it as a usize (which is used more commonly in rust than 32 bit integers)
    pub fn index(&self) -> usize {
        self.swapchain_index as usize
    }

    // This function is used to get the image that is being rendered
    pub fn image(&self) -> &Image {
        &self.display.images[self.index()]
    }

    // This function is used to get the image view that is being rendered
    pub fn image_view(&self) -> &ImageView {
        &self.display.views[self.index()]
    }

    // Allocate a command list from the internal command pool
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

        self.display.present(
            &self.context,
            self.swapchain_index,
            &self.frame.rendering_finished,
        );
    }
}

// The `Frames` struct is an abstraction over the `Frame` struct, and is used to manage the rendering of frames
// it contains a vector of `Frame` objects, along with a counter that is used to keep track of the current frame
// that wraps around when it reaches the end of the vector.
pub struct Frames {
    // The Vulkan API context
    context: Arc<Context>,
    // The window display (which contains the Vulkan Swapchain)
    display: Arc<Display>,
    // The vector of frames
    frames: Vec<Frame>,

    // The counter that is used to keep track of the current frame
    counter: usize,
}

impl Frames {
    pub fn new(context: Arc<Context>, display: Arc<Display>) -> Self {
        // Create an empty list of frames
        let mut frames = Vec::with_capacity(display.frames_in_flight());

        // Initialise the Vulkan objects for each frame
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

    // Gets the next frame from the frames abstraction
    pub fn next(&mut self) -> FrameRef<'_> {
        // Increment the counter and wrap around when it reaches the end of the vector
        self.counter = (self.counter + 1) % self.display.frames_in_flight();

        // Get a mutable reference to the frame
        let frame = self.frames.get_mut(self.counter).unwrap();

        // Wait for the frame to finish rendering and reset the inflight fence
        frame.inflight.wait_and_reset();
        frame.command_pool.reset();

        // And get the index of the swapchain image that is being rendered
        let (index, _suboptimal) = self.display.acquire_next_image(&frame.swapchain_ready);

        // Return a reference to the frame
        FrameRef {
            frame,
            context: &self.context,
            display: &self.display,
            swapchain_index: index,
        }
    }
}
