use egui::ahash::HashMap;
use egui::util::cache::CacheTrait;
use flexim_data_type::FlImage;
use std::any::Any;
use std::sync::Arc;

#[derive(Default)]
pub struct VisualizedImageCache {
    cached_images: HashMap<usize, Arc<FlImage>>,
}

impl VisualizedImageCache {
    pub fn insert(&mut self, id: usize, image: FlImage) {
        self.cached_images.insert(id, Arc::new(image));
    }

    pub fn get(&self, id: usize) -> Option<Arc<FlImage>> {
        self.cached_images.get(&id).map(|t| t.clone())
    }
}

impl CacheTrait for VisualizedImageCache {
    fn update(&mut self) {}

    fn len(&self) -> usize {
        self.cached_images.len()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
