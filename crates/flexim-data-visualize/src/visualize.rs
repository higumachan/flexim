use crate::cache::VisualizedImageCache;
use egui::{Id, Image, ImageSource, LayerId, Layout, Pos2, Rect, Ui, Vec2};
use egui_tiles::UiResponse;
use flexim_data_type::{FlImage, FlTensor2D};
use image::{DynamicImage, ImageBuffer, Rgb};
use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use std::sync::Arc;
use unwrap_ord::UnwrapOrd;

pub trait DataRender {
    fn render(&self, ui: &mut Ui) -> Vec<Arc<FlImage>>;
}

pub trait DataVisualize: DataRender {
    fn visualize(&self, name: &str, ui: &mut egui::Ui) -> UiResponse;
}

pub struct FlImageVisualize {
    content: Arc<FlImage>,
}

impl FlImageVisualize {
    pub fn new(content: FlImage) -> Self {
        Self {
            content: Arc::new(content),
        }
    }
}

impl DataRender for FlImageVisualize {
    fn render(&self, ui: &mut Ui) -> Vec<Arc<FlImage>> {
        vec![self.content.clone()]
    }
}

impl DataVisualize for FlImageVisualize {
    fn visualize(&self, name: &str, ui: &mut Ui) -> UiResponse {
        let image = egui::Image::from_bytes(
            format!("bytes://{}.png", name),
            self.render(ui)[0].value.clone(),
        );
        image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();
        ui.add(image);
        UiResponse::None
    }
}

pub struct FlTensor2DVisualize {
    id: usize,
    content: FlTensor2D<f64>,
}

impl FlTensor2DVisualize {
    pub fn new(id: usize, content: FlTensor2D<f64>) -> Self {
        Self { id, content }
    }
}

impl DataRender for FlTensor2DVisualize {
    fn render(&self, ui: &mut Ui) -> Vec<Arc<FlImage>> {
        vec![ui.memory_mut(|mem| {
            let cache = mem.caches.cache::<VisualizedImageCache>();
            if let Some(image) = cache.get(self.id) {
                image
            } else {
                let cm = scarlet::colormap::ListedColorMap::viridis();
                let max = self
                    .content
                    .value
                    .iter()
                    .copied()
                    .max_by_key(|t| UnwrapOrd(*t))
                    .unwrap();
                let min = self
                    .content
                    .value
                    .iter()
                    .copied()
                    .min_by_key(|t| UnwrapOrd(*t))
                    .unwrap();
                let normalize = move |v| (v - min) / (max - min);
                let transformed: Vec<RGBColor> =
                    cm.transform(self.content.value.iter().map(|v| normalize(*v)));
                let pixels: Vec<u8> = transformed
                    .into_iter()
                    .map(|c| [c.int_r(), c.int_g(), c.int_b()])
                    .flatten()
                    .collect();
                let image_buffer: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_vec(
                    self.content.value.shape()[1] as u32,
                    self.content.value.shape()[0] as u32,
                    pixels,
                )
                .unwrap();
                let image = DynamicImage::ImageRgb8(image_buffer);

                let mut image_png_bytes = Vec::new();
                image
                    .write_to(&mut image_png_bytes, image::ImageOutputFormat::Png)
                    .unwrap();
                cache.insert(self.id, FlImage::new(image_png_bytes));
                cache.get(self.id).unwrap()
            }
        })]
    }
}

impl DataVisualize for FlTensor2DVisualize {
    fn visualize(&self, name: &str, ui: &mut Ui) -> UiResponse {
        let image = egui::Image::from_bytes(
            format!("bytes://{}.png", name),
            self.render(ui)[0].value.clone(),
        );
        image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();
        ui.add(image);
        UiResponse::None
    }
}

pub struct StackVisualize {
    stack: Vec<Arc<dyn DataRender>>,
}

impl StackVisualize {
    pub fn new(stack: Vec<Arc<dyn DataRender>>) -> Self {
        Self { stack }
    }
}

impl DataRender for StackVisualize {
    fn render(&self, ui: &mut Ui) -> Vec<Arc<FlImage>> {
        self.stack.iter().map(|s| s.render(ui)).flatten().collect()
    }
}

impl DataVisualize for StackVisualize {
    fn visualize(&self, name: &str, ui: &mut Ui) -> UiResponse {
        if self.stack.len() == 0 {
            return UiResponse::None;
        }
        let image = egui::Image::from_bytes(
            format!("bytes://{}.png", name),
            self.render(ui)[0].value.clone(),
        );
        image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();
        let response = ui.add(image);
        let rect = response.rect;
        for (i, image) in self.render(ui).iter().skip(1).enumerate() {
            let image =
                Image::from_bytes(format!("bytes://{}_{}.png", name, i), image.value.clone());
            let image = image.tint(egui::Color32::from_rgba_premultiplied(255, 255, 255, 128));
            image
                .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
                .unwrap();
            ui.put(rect, image);
        }
        UiResponse::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
