use self::objects::GpuObject;
use parking_lot::Mutex;
use std::{
    sync::OnceLock,
    thread::{self, JoinHandle},
};

static GLOBAL_SCENE_LOADER: OnceLock<Mutex<SceneLoader>> = OnceLock::new();

pub struct SceneData {
    pub objects: Vec<GpuObject>,
}

pub struct SceneLoader {
    load_thread: Option<JoinHandle<anyhow::Result<SceneData>>>,
}

impl SceneLoader {
    const fn new() -> Self {
        let load_thread = None;
        SceneLoader { load_thread }
    }

    pub fn init() {
        let _ = GLOBAL_SCENE_LOADER
            .get_or_init(|| Mutex::new(SceneLoader::new()))
            .lock();
    }

    pub fn request_load() {
        let mut asset_server = GLOBAL_SCENE_LOADER
            .get_or_init(|| Mutex::new(SceneLoader::new()))
            .lock();

        if let Some(thread) = std::mem::take(&mut asset_server.load_thread) {
            log::info!(
                "There is already a scene loading in the background, which will be cancelled"
            );

            // Detach the already running thread
            std::mem::drop(thread);
        }

        // And fire up the new one
        let handle = thread::spawn(|| load_task());
        asset_server.load_thread = Some(handle)
    }

    pub fn poll() -> Option<SceneData> {
        let mut asset_server = GLOBAL_SCENE_LOADER
            .get_or_init(|| Mutex::new(SceneLoader::new()))
            .lock();

        if let Some(join_handle) = &mut asset_server.load_thread {
            if join_handle.is_finished() {
                let handle = std::mem::take(&mut asset_server.load_thread).unwrap();
                let result = handle.join().unwrap();

                match result {
                    Ok(scene_data) => return Some(scene_data),
                    Err(err) => log::error!("Failed to load scene : {}", err.to_string()),
                }
            }
        }

        None
    }
}

fn load_task() -> anyhow::Result<SceneData> {
    let file_request = rfd::FileDialog::new().pick_file();
    let Some(file) = file_request else {
        anyhow::bail!("Scene load cancelled")
    };

    log::info!("Loading file..");
    let (document, buffers, images) = gltf::import(&file)?;

    let objects = objects::load_objects(&document, &buffers);

    Ok(SceneData {
        objects,
    })
}

pub mod objects {
    use gltf::Document;

    pub struct GpuObject {
        pub vertices: Vec<f32>,
        pub indices: Vec<u32>,
    
        pub transform: glam::Mat4,
    
        pub base_color: glam::Vec3A,
        pub emissive: glam::Vec3A,
        pub roughness: f32,
        pub metallic: f32,
    }
    
    pub fn load_objects(document: &Document, buffers: &[gltf::buffer::Data]) -> Vec<GpuObject> {
        let mut objects = Vec::new();
    
        for node in document.nodes() {
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    
                    let transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
    
                    let vertices = reader
                        .read_positions()
                        .unwrap()
                        .into_iter()
                        .flatten()
                        .collect::<Vec<f32>>();
    
                    let indices = reader
                        .read_indices()
                        .unwrap()
                        .into_u32()
                        .collect::<Vec<u32>>();
    
                    let pbr = primitive.material().pbr_metallic_roughness();
    
                    let base_color = glam::Vec3A::from_slice(&pbr.base_color_factor());
                    let roughness = pbr.roughness_factor();
                    let metallic = pbr.metallic_factor();
                    let emissive = glam::Vec3A::from_array(primitive.material().emissive_factor());
    
                    let object = GpuObject {
                        vertices,
                        indices,
    
                        transform,
    
                        base_color,
                        emissive,
                        roughness,
                        metallic,
                    };
    
                    objects.push(object);
                }
            }
        }
    
        objects
    }    
}