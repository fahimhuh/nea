use crossbeam_channel::{Receiver, Sender};
use winit::{
    event::{DeviceEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

pub enum Input {
    Keyboard(KeyCode),
    Mouse(glam::DVec2),
    Unknown,
}

impl Input {
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

pub type InputListener = Receiver<Input>;
pub type InputBroadcaster = Sender<Input>;

pub struct Inputs {
    pub broadcaster: InputBroadcaster,
    pub listener: InputListener,
}

impl Inputs {
    pub fn new() -> Self {
        let (broadcaster, listener) = crossbeam_channel::unbounded();
        Self {
            broadcaster,
            listener,
        }
    }
}
