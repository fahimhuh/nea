use gltf::Document;
use parking_lot::Mutex;
use std::{
    sync::OnceLock,
    thread::{self, JoinHandle},
};

// Global singleton for the scene loader
// which can be accessed from anywhere in the application
static GLOBAL_SCENE_LOADER: OnceLock<Mutex<SceneLoader>> = OnceLock::new();

// Data structure to hold the scene data
// which is loaded from the glTF file
// into a format that is suitable for the GPU
pub struct SceneData {
    pub objects: Vec<GpuObject>,
}

// The scene loader is responsible for loading the scene data
// It holds a thread handle to the loading task
pub struct SceneLoader {
    load_thread: Option<JoinHandle<anyhow::Result<SceneData>>>,
}

impl SceneLoader {
    // Create a new scene loader
    const fn new() -> Self {
        let load_thread = None;
        SceneLoader { load_thread }
    }

    pub fn init() {
        log::info!("Initialising scene loader");
        let _ = GLOBAL_SCENE_LOADER
            .get_or_init(|| Mutex::new(SceneLoader::new()))
            .lock();
    }

    // Request the scene to be loaded
    pub fn request_load() {
        // Get a handle to the scene loader
        let mut asset_server = GLOBAL_SCENE_LOADER
            .get_or_init(|| Mutex::new(SceneLoader::new()))
            .lock();

        // Check if there is already a scene loading in the background
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

    // Poll the scene loader to see if the scene has been loaded
    pub fn poll() -> Option<SceneData> {
        // Get a handle to the scene loader
        let mut asset_server = GLOBAL_SCENE_LOADER
            .get_or_init(|| Mutex::new(SceneLoader::new()))
            .lock();

        // Check if there is currently a scene loading in the background
        if let Some(join_handle) = &mut asset_server.load_thread {
            // If the scene has finished loading, join the thread and return the result
            if join_handle.is_finished() {
                // Take ownership of the thread handle
                let handle = std::mem::take(&mut asset_server.load_thread).unwrap();
                // Join the thread (Which will be instant as it has already finished)
                let result = handle.join().unwrap();

                // Return the result
                match result {
                    Ok(scene_data) => return Some(scene_data),
                    Err(err) => log::error!("Failed to load scene : {}", err.to_string()),
                }
            }
        }

        None
    }
}

// This function is used to load the scene data from the glTF file
// and is run in a separate thread
fn load_task() -> anyhow::Result<SceneData> {
    // Request the user to select a file
    let file_request = rfd::FileDialog::new().pick_file();
    let Some(file) = file_request else {
        anyhow::bail!("Scene load cancelled")
    };

    log::info!("Loading file..");
    let (document, buffers, images) = gltf::import(&file)?;

    // Call the function to load the scene data
    let objects = load_objects(&document, &buffers);

    Ok(SceneData { objects })
}

// This structure contains the data that will be stored on the GPU for rendering
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

    // Iterate over all the nodes in the glTF file
    for node in document.nodes() {
        // Check if the node has a mesh associated with it
        if let Some(mesh) = node.mesh() {
            // Iterate over all the primitives in the mesh
            for primitive in mesh.primitives() {
                // Get the buffer that contains the data for the primitive
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // Get the transformation matrix of the node
                let transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());

                // Read the vertex positions and indices from the buffer
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

                // Get the material properties of the primitive
                let pbr = primitive.material().pbr_metallic_roughness();

                let base_color = glam::Vec3A::from_slice(&pbr.base_color_factor());
                let roughness = pbr.roughness_factor();
                let metallic = pbr.metallic_factor();
                let emissive = glam::Vec3A::from_array(primitive.material().emissive_factor());

                // Create the GpuObject and add it to the list
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
