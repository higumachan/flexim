use crate::cache::{Poll, VisualizedImageCache};
use egui::{Id, Image, ImageSource, LayerId, Layout, Pos2, Rect, Ui, Vec2};
use egui_tiles::UiResponse;
use flexim_data_type::{FlImage, FlTensor2D};
use image::{DynamicImage, ImageBuffer, Rgb};
use itertools::Itertools;
use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use std::sync::Arc;
use unwrap_ord::UnwrapOrd;

pub trait DataRender {
    fn render(&self, ui: &mut Ui) -> Option<Arc<FlImage>>;
}

pub struct FlImageRender {
    content: Arc<FlImage>,
}

impl FlImageRender {
    pub fn new(content: FlImage) -> Self {
        Self {
            content: Arc::new(content),
        }
    }
}

impl DataRender for FlImageRender {
    fn render(&self, ui: &mut Ui) -> Option<Arc<FlImage>> {
        Some(self.content.clone())
    }
}

pub struct FlTensor2DRender {
    id: usize,
    content: FlTensor2D<f64>,
}

impl FlTensor2DRender {
    pub fn new(id: usize, content: FlTensor2D<f64>) -> Self {
        Self { id, content }
    }
}

impl DataRender for FlTensor2DRender {
    fn render(&self, ui: &mut Ui) -> Option<Arc<FlImage>> {
        ui.memory_mut(|mem| {
            let cache = mem.caches.cache::<VisualizedImageCache>();
            if let Some(image) = cache.get(self.id) {
                if let Poll::Ready(image) = image {
                    Some(image)
                } else {
                    None
                }
            } else {
                let ctx = ui.ctx().clone();
                let id = self.id;
                let content = self.content.clone();
                std::thread::spawn(move || {
                    let cm = scarlet::colormap::ListedColorMap::viridis();
                    let max = content
                        .value
                        .iter()
                        .copied()
                        .max_by_key(|t| UnwrapOrd(*t))
                        .unwrap();
                    let min = content
                        .value
                        .iter()
                        .copied()
                        .min_by_key(|t| UnwrapOrd(*t))
                        .unwrap();
                    let normalize = move |v| (v - min) / (max - min);
                    let transformed: Vec<RGBColor> =
                        cm.transform(content.value.iter().map(|v| normalize(*v)));
                    let pixels: Vec<u8> = transformed
                        .into_iter()
                        .map(|c| [c.int_r(), c.int_g(), c.int_b()])
                        .flatten()
                        .collect();
                    let image_buffer: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_vec(
                        content.value.shape()[1] as u32,
                        content.value.shape()[0] as u32,
                        pixels,
                    )
                    .unwrap();
                    let image = DynamicImage::ImageRgb8(image_buffer);

                    let mut image_png_bytes = Vec::new();
                    image
                        .write_to(&mut image_png_bytes, image::ImageOutputFormat::Png)
                        .unwrap();
                    ctx.memory_mut(|mem| {
                        let cache = mem.caches.cache::<VisualizedImageCache>();
                        cache.insert(id, FlImage::new(image_png_bytes));
                    })
                });
                cache.insert_pending(self.id);
                None
            }
        })
    }
}

pub fn visualize(ui: &mut Ui, name: &str, render: &dyn DataRender) -> UiResponse {
    if let Some(image) = render.render(ui) {
        let image = Image::from_bytes(format!("bytes://{}.png", name), image.value.clone());
        image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();
        ui.add(image);
    } else {
        ui.spinner();
    }
    UiResponse::None
}

pub fn stack_visualize(ui: &mut Ui, stack: &Vec<(String, Arc<dyn DataRender>)>) -> UiResponse {
    if stack.len() == 0 {
        return UiResponse::None;
    }
    let stack = stack
        .iter()
        .map(|(n, s)| s.render(ui).into_iter().map(move |i| (n.clone(), i)))
        .flatten()
        .collect_vec();

    let (name, v) = &stack[0];
    let image = egui::Image::from_bytes(format!("bytes://{}_0.png", name), v.value.clone());
    image
        .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
        .unwrap();
    let response = ui.add(image);
    let rect = response.rect;
    for (i, (name, image)) in stack.iter().enumerate().skip(1) {
        let image = Image::from_bytes(format!("bytes://{}_{}.png", name, i), image.value.clone());
        let image = image.tint(egui::Color32::from_rgba_premultiplied(255, 255, 255, 128));
        image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();
        ui.put(rect, image);
    }
    UiResponse::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
