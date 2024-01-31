use egui::ahash::HashMap;
use egui::util::cache::CacheTrait;
use egui::Id;
use flexim_data_type::FlImage;
use std::any::Any;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Poll<T> {
    Ready(T),
    Pending,
}

#[derive(Default)]
pub struct VisualizedImageCache {
    cached_images: HashMap<Id, Poll<Arc<FlImage>>>,
}

impl VisualizedImageCache {
    pub fn insert(&mut self, id: Id, image: FlImage) {
        self.cached_images.insert(id, Poll::Ready(Arc::new(image)));
    }

    pub fn insert_pending(&mut self, id: Id) {
        self.cached_images.insert(id, Poll::Pending);
    }

    pub fn get(&self, id: Id) -> Option<Poll<Arc<FlImage>>> {
        self.cached_images.get(&id).cloned()
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
