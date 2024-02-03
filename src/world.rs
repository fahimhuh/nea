use crossbeam_channel::Receiver;
use glam::vec3;
use winit::{event::WindowEvent, keyboard::KeyCode};

use crate::input::{Input, Inputs};

#[derive(Default, Debug)]
pub struct Object {}

#[derive(Default, Debug)]
pub struct Camera {
    pub position: glam::Vec3A,
    pub rotation: glam::Quat,
}

pub struct RenderSettings {
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub focal_length: f32,
    pub aperture: f32,
    pub exposure: f32,

    pub samples: u32,
    pub bounces: u32,
}

pub struct World {
    pub camera: Camera,
    pub settings: RenderSettings, 
    pub objects: Vec<Object>,
}

impl World {
    pub fn new() -> Self {
        let camera = Camera {
            position: glam::Vec3A::new(0.0, 0.0, -4.0),
            rotation: glam::Quat::IDENTITY,
        };

        let settings = RenderSettings {
            fov: 60.0,
            near: 0.01,
            far: 100.0,
            focal_length: 16.0,
            aperture: 1.0,
            exposure: 1.0,
            samples: 8,
            bounces: 3,
        };

        Self {
            camera,
            settings,
            objects: Vec::default(),
        }
    }

    pub fn update(&mut self, inputs: &Inputs) {
        const CAM_SPEED: f32 = 0.5;
        const CAM_SENS: f32 = 0.1;

        let right = self.camera.rotation * glam::vec3a(1.0, 0.0, 0.0);
        let forward = self.camera.rotation * glam::vec3a(0.0, 0.0, 1.0);

        for event in inputs.listener.try_iter() {            
            match event {

                Input::Keyboard(key) => {
                    if key == KeyCode::KeyW { self.camera.position += forward * CAM_SPEED }
                    if key == KeyCode::KeyS { self.camera.position -= forward * CAM_SPEED }
                    
                    if key == KeyCode::KeyD { self.camera.position += right * CAM_SPEED }
                    if key == KeyCode::KeyA { self.camera.position -= right * CAM_SPEED }

                    if key == KeyCode::Space { self.camera.position.y += CAM_SPEED };
                    if key == KeyCode::ShiftLeft { self.camera.position.y -= CAM_SPEED };
                    
                },

                Input::Mouse(delta) => {
                    // Convert the delta from f64s to f32s
                    let movement = delta.as_vec2() * CAM_SENS;
                    
                    let pitch = glam::Quat::from_rotation_x(-movement.y);
                    let yaw = glam::Quat::from_rotation_y(movement.x);

                    self.camera.rotation = pitch * self.camera.rotation * yaw;
                    
                },

                Input::Unknown => (),
            }
        }
    }
}
