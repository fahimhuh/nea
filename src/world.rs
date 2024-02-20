use crate::input::{Input, Inputs};
use winit::keyboard::KeyCode;

// The Object struct is a representation of an object in the world
// and contains the position, rotation and scale of the object
pub struct Object {
    pub position: glam::Vec3A,
    pub rotation: glam::Vec3A,
    pub scale: glam::Vec3A,
}

// The Camera struct is a representation of the camera in the world
// and contains the position and rotation of the camera (Cameras cannot be scaled)
pub struct Camera {
    pub position: glam::Vec3A,
    pub rotation: glam::Quat,
}

// The RenderSettings struct is a representation of the rendering settings in the world
// and contains the field of view, near and far planes, focal length, aperture, exposure, samples and bounces
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

// The World struct is a representation of the world and contains the camera, rendering settings and objects
pub struct World {
    pub camera: Camera,
    pub settings: RenderSettings,
    pub objects: Vec<Object>,
}

impl World {
    // Create a new world
    pub fn new() -> Self 
    {
        // Create a new camera
        let camera = Camera {
            position: glam::Vec3A::new(0.0, 0.0, -4.0),
            rotation: glam::Quat::IDENTITY,
        };

        // Create a new set of rendering settings with default values
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

        // Calculate the right and forward vectors of the camera given the rotation quaternion
        let right = self.camera.rotation * glam::vec3a(1.0, 0.0, 0.0);
        let forward = self.camera.rotation * glam::vec3a(0.0, 0.0, 1.0);

        // Iterate over the input events from the queue (Remove the events from the queue as we iterate over them)
        for event in inputs.listener.try_iter() {
            match event {
                // Handle keyboard input and move the camera accordingly
                Input::Keyboard(key) => {
                    if key == KeyCode::KeyW {
                        self.camera.position += forward * CAM_SPEED
                    }
                    if key == KeyCode::KeyS {
                        self.camera.position -= forward * CAM_SPEED
                    }

                    if key == KeyCode::KeyD {
                        self.camera.position += right * CAM_SPEED
                    }
                    if key == KeyCode::KeyA {
                        self.camera.position -= right * CAM_SPEED
                    }

                    if key == KeyCode::Space {
                        self.camera.position.y += CAM_SPEED
                    };
                    if key == KeyCode::ShiftLeft {
                        self.camera.position.y -= CAM_SPEED
                    };
                }

                Input::Mouse(delta) => {
                    // Convert the delta from f64s to f32s
                    let movement = delta.as_vec2() * CAM_SENS;

                    // Rotate the camera based on the mouse movement
                    let pitch = glam::Quat::from_rotation_x(-movement.y);
                    let yaw = glam::Quat::from_rotation_y(movement.x);

                    // Rotate the camera
                    self.camera.rotation = pitch * self.camera.rotation * yaw;
                }

                Input::Unknown => (),
            }
        }
    }
}
