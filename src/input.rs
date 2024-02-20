use crossbeam_channel::{Receiver, Sender};
use winit::{
    event::{DeviceEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

// This enum is used to represent the different types of input that the program can recieve
// It only contains the types of input that are relevant to this project
// and all other inputs recieved from the window are labelled as "Unknown"
pub enum Input {
    Keyboard(KeyCode),
    Mouse(glam::DVec2),
    Unknown,
}

impl Input {
    // This function is used to convert a winit::event::WindowEvent into an Input
    pub fn from_window_event(event: WindowEvent) -> Self {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    Input::Keyboard(code)
                } else {
                    Input::Unknown
                }
            }

            _ => Input::Unknown,
        }
    }

    // This function is used to convert a winit::event::DeviceEvent into an Input
    pub fn from_device_event(event: DeviceEvent) -> Self {
        match event {
            DeviceEvent::MouseMotion { delta } => Input::Mouse(glam::DVec2 {
                x: delta.0,
                y: delta.1,
            }),
            _ => Input::Unknown,
        }
    }
}

// These type aliases are used to make the code more readable
pub type InputListener = Receiver<Input>;
pub type InputBroadcaster = Sender<Input>;

// This struct is used to store the input queue, which is used to store input events
// We use a crossbeam_channel to store the input events, as it is a thread-safe channel
// and can be trivially used as a queue
pub struct Inputs {
    // The broadcaster is used to send input events to the queue
    pub broadcaster: InputBroadcaster,

    // The listener is used to recieve input events from the queue
    pub listener: InputListener,
}

impl Inputs {
    // Create a new input queue
    pub fn new() -> Self {
        let (broadcaster, listener) = crossbeam_channel::unbounded();
        Self {
            broadcaster,
            listener,
        }
    }
}
