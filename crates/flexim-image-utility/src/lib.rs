use egui::ahash::HashMap;
use egui::load::{ImageLoadResult, ImageLoader, LoadError};
use egui::{ColorImage, Context, SizeHint};
use std::sync::{Arc, Mutex};

const RAW_BYTES_SCHEME: &str = "raw-bytes";

type Entry = Result<Arc<ColorImage>, String>;

/// Loads an image from raw bytes.
/// special url schema raw-bytes://
#[derive(Default)]
pub struct RawBytesImageLoader {
    cache: Mutex<HashMap<String, Entry>>,
}

impl RawBytesImageLoader {
    pub const ID: &'static str = egui::generate_loader_id!(RawBytesImageLoader);
}

fn is_supported_uri(uri: &str) -> bool {
    uri.starts_with(RAW_BYTES_SCHEME)
}

impl ImageLoader for RawBytesImageLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
        if !is_supported_uri(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
    }

    fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().clear();
    }

    fn byte_size(&self) -> usize {
        todo!()
    }
}
