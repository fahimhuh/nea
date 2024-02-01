#[derive(Default, Debug)]
pub struct Object {}

#[derive(Default, Debug)]
pub struct Camera {
    pub position: glam::Vec3A,
    pub rotation: glam::Quat,

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
    pub objects: Vec<Object>,
}

impl World {
    pub fn new() -> Self {
        let camera = Camera {
            position: glam::Vec3A::new(0.0, 0.0, -4.0),
            rotation: glam::Quat::IDENTITY,
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
            objects: Vec::default(),
        }
    }

    pub fn update(&mut self) {}
}
