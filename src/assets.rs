use parking_lot::Mutex;
use std::sync::OnceLock;

static GLOBAL_ASSET_SERVER: OnceLock<Mutex<AssetServer>> = OnceLock::new();

pub struct AssetServer {
    executor: smol::Executor<'static>,
}

impl AssetServer {
    const fn new() -> Self {
        let executor = smol::Executor::new();
        Self { executor }
    }

    pub fn init() {
        let _ = GLOBAL_ASSET_SERVER
            .get_or_init(|| Mutex::new(AssetServer::new()))
            .lock();
    }

    pub fn update() {
        let asset_server = GLOBAL_ASSET_SERVER
            .get_or_init(|| Mutex::new(AssetServer::new()))
            .lock();

        for _ in 0..10 {
            smol::block_on(asset_server.executor.tick());
        }
    }

    pub fn load() {
        let asset_server = GLOBAL_ASSET_SERVER
            .get_or_init(|| Mutex::new(AssetServer::new()))
            .lock();
        let task = asset_server.executor.spawn(load_task());
        task.detach();
    }
}

async fn load_task() {
    let file_request = rfd::AsyncFileDialog::new().pick_file().await;
    let Some(_file) = file_request else {
        log::info!("File loading cancelled");
        return;
    };
}
