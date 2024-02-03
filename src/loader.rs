use self::{images::GpuImage, objects::GpuObject};
use parking_lot::Mutex;
use std::{
    sync::OnceLock,
    thread::{self, JoinHandle},
};

pub mod images;
pub mod objects;

static GLOBAL_SCENE_LOADER: OnceLock<Mutex<SceneLoader>> = OnceLock::new();

pub struct SceneData {
    pub images: Vec<GpuImage>,
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

    let mut gpu_images = Vec::with_capacity(images.len());
    // Parse the images into a GPU-friendly format
    for image in images {
        let gpu_image = images::parse_image(image)?;
        gpu_images.push(gpu_image)
    }

    let objects = objects::load_objects(&document, &buffers);

    Ok(SceneData {
        images: gpu_images,
        objects,
    })
}
