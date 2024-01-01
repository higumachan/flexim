use crate::cache::{Poll, VisualizedImageCache};
use egui::emath::RectTransform;
use egui::load::TexturePoll;
use egui::scroll_area::State;
use egui::{
    Context, Id, Image, ImageSource, LayerId, Layout, Pos2, Rect, Response, Sense, Ui, Vec2, Widget,
};
use egui_tiles::UiResponse;
use flexim_data_type::{FlData, FlImage, FlTensor2D};
use image::{DynamicImage, ImageBuffer, Rgb};
use itertools::Itertools;
use num_traits::float::Float;
use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use std::sync::Arc;
use unwrap_ord::UnwrapOrd;

#[derive(Debug, Clone)]
pub struct VisualizeState {
    pub scale: f64,
    pub shift: Vec2,
}

impl VisualizeState {
    pub fn uv_rect(&self) -> Rect {
        Rect::from_center_size(
            Pos2::new(0.5, 0.5) + self.shift,
            Vec2::new(1.0 / self.scale as f32, 1.0 / self.scale as f32),
        )
    }

    pub fn verify(&mut self) {
        self.scale = self.scale.clamp(0.01, 10.0);
    }
}

impl Default for VisualizeState {
    fn default() -> Self {
        Self {
            scale: 1.0,
            shift: Vec2::ZERO,
        }
    }
}

pub trait DataRender {
    fn render(&self, ui: &mut Ui) -> Option<Arc<FlImage>>;
}

pub struct FlImageRender {
    content: Arc<FlImage>,
}

impl FlImageRender {
    pub fn new(content: Arc<FlImage>) -> Self {
        Self { content }
    }
}

impl DataRender for FlImageRender {
    fn render(&self, ui: &mut Ui) -> Option<Arc<FlImage>> {
        Some(self.content.clone())
    }
}

pub struct FlTensor2DRender {
    content: Arc<FlTensor2D<f64>>,
}

impl FlTensor2DRender {
    pub fn new(content: Arc<FlTensor2D<f64>>) -> Self {
        Self { content }
    }
}

impl DataRender for FlTensor2DRender {
    fn render(&self, ui: &mut Ui) -> Option<Arc<FlImage>> {
        ui.memory_mut(|mem| {
            let cache = mem.caches.cache::<VisualizedImageCache>();
            if let Some(image) = cache.get(self.content.id) {
                if let Poll::Ready(image) = image {
                    Some(image)
                } else {
                    None
                }
            } else {
                let ctx = ui.ctx().clone();
                let id = self.content.id;
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
                cache.insert_pending(self.content.id);
                None
            }
        })
    }
}

pub fn visualize(
    ui: &mut Ui,
    state: &mut VisualizeState,
    name: &str,
    render: &dyn DataRender,
) -> Response {
    if let Some(image) = render.render(ui) {
        let image = Image::from_bytes(format!("bytes://{}.png", name), image.value.clone());
        let image = image.uv(state.uv_rect());
        let image = image.sense(Sense::drag());
        let _texture = image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();

        ui.add(image)
    } else {
        ui.spinner()
    }
}

pub fn stack_visualize(
    ui: &mut Ui,
    visualize_state: &mut VisualizeState,
    stack: &Vec<(String, Arc<dyn DataRender>)>,
) -> Response {
    assert_ne!(stack.len(), 0);
    let stack = stack
        .iter()
        .map(|(n, s)| s.render(ui).into_iter().map(move |i| (n.clone(), i)))
        .flatten()
        .collect_vec();

    let (name, v) = &stack[0];
    let image = egui::Image::from_bytes(format!("bytes://{}_0.png", name), v.value.clone());
    let image = image.uv(visualize_state.uv_rect());
    let image = image.sense(Sense::drag());
    image
        .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
        .unwrap();
    let response = ui.add(image);
    let rect = response.rect;
    let mut last_image = None;
    for (i, (name, image)) in stack.iter().enumerate().skip(1) {
        if let Some(image) = last_image {
            ui.put(rect, image);
        }
        let image = Image::from_bytes(format!("bytes://{}_{}.png", name, i), image.value.clone());
        let image = image.uv(visualize_state.uv_rect());
        let image = image.tint(egui::Color32::from_rgba_premultiplied(255, 255, 255, 128));
        let _texture = image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();

        last_image = Some(image);
    }
    if let Some(image) = last_image {
        let image = image.sense(Sense::drag());
        ui.put(rect, image)
    } else {
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
