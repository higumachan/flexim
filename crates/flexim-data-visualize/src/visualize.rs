use crate::cache::{Poll, VisualizedImageCache};
use std::collections::BTreeSet;
use std::hash::Hash;

use egui::{
    CollapsingHeader, Color32, ComboBox, Context, Id, Image, Painter, Pos2, Rect, Response, Sense,
    Slider, Stroke, Ui, Vec2, Widget,
};

use flexim_data_type::{FlDataFrameRectangle, FlImage, FlTensor2D};
use flexim_data_view::FlDataFrameView;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use itertools::Itertools;

use crate::pallet::pallet;
use anyhow::Context as _;
use downcast_rs::{impl_downcast, Downcast};
use egui::ahash::HashSet;
use egui::load::TexturePoll;
use polars::prelude::*;
use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use std::sync::{Arc, Mutex};
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

pub trait DataRender: Downcast {
    fn id(&self) -> Id;

    fn render(
        &self,
        painter: &mut Painter,
        state: &VisualizeState,

        size: Vec2,
    ) -> anyhow::Result<()>;
    fn config_panel(&self, ui: &mut Ui);
    fn size(&self) -> Vec2;
}
impl_downcast!(DataRender);

pub struct FlImageRender {
    pub content: Arc<FlImage>,
}

impl FlImageRender {
    pub fn new(content: Arc<FlImage>) -> Self {
        Self { content }
    }
}

impl DataRender for FlImageRender {
    fn id(&self) -> Id {
        Id::new("fl_image").with(self.content.id)
    }

    fn render(
        &self,
        painter: &mut Painter,
        state: &VisualizeState,
        size: Vec2,
    ) -> anyhow::Result<()> {
        let image = Image::from_bytes(
            format!("bytes://{}.png", self.content.id),
            self.content.value.clone(),
        );

        draw_image(painter, &image, state.shift, size, Color32::WHITE)
    }

    fn config_panel(&self, ui: &mut Ui) {
        ui.label("FlImage");
    }

    fn size(&self) -> Vec2 {
        Vec2::new(self.content.width as f32, self.content.height as f32)
    }
}

#[derive(Debug)]
pub struct FlTensor2DRenderContext {
    pub transparency: f64,
}

impl Default for FlTensor2DRenderContext {
    fn default() -> Self {
        Self { transparency: 0.5 }
    }
}
pub struct FlTensor2DRender {
    content: Arc<FlTensor2D<f64>>,
    context: Arc<Mutex<FlTensor2DRenderContext>>,
}

impl FlTensor2DRender {
    pub fn new(content: Arc<FlTensor2D<f64>>) -> Self {
        Self {
            content,
            context: Arc::new(Mutex::new(FlTensor2DRenderContext::default())),
        }
    }
}

impl DataRender for FlTensor2DRender {
    fn id(&self) -> Id {
        Id::new("fl_tensor2d").with(self.content.id)
    }

    fn render(
        &self,
        painter: &mut Painter,
        state: &VisualizeState,

        size: Vec2,
    ) -> anyhow::Result<()> {
        let id = Id::new(self.content.id);
        let image = painter.ctx().memory_mut(|mem| {
            let cache = mem.caches.cache::<VisualizedImageCache>();
            if let Some(image) = cache.get(id) {
                if let Poll::Ready(image) = image {
                    Some(image)
                } else {
                    None
                }
            } else {
                let ctx = painter.ctx().clone();
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
                        cache.insert(
                            id,
                            FlImage::new(
                                image_png_bytes,
                                image.width() as usize,
                                image.height() as usize,
                            ),
                        )
                    })
                });
                cache.insert_pending(id);
                None
            }
        });

        if let Some(image) = image {
            let image = Image::from_bytes(
                format!("bytes://{}.png", self.content.id),
                image.value.clone(),
            );

            let transparency = (self.context.lock().unwrap().transparency * 255.0) as u8;
            let tint_color = Color32::from_rgba_premultiplied(
                transparency,
                transparency,
                transparency,
                transparency,
            );

            draw_image(painter, &image, state.shift, size, tint_color)?;
        }

        Ok(())
    }

    fn config_panel(&self, ui: &mut Ui) {
        ui.label("FlTensor2D");
        CollapsingHeader::new("Config").show(ui, |ui| {
            let mut render_context = self.context.lock().unwrap();
            ui.horizontal(|ui| {
                ui.label("Transparency");
                Slider::new(&mut render_context.transparency, 0.0..=1.0).ui(ui);
            });
        });
    }

    fn size(&self) -> Vec2 {
        Vec2::new(
            self.content.value.shape()[1] as f32,
            self.content.value.shape()[0] as f32,
        )
    }
}

#[derive(Debug)]
pub struct FlDataFrameViewRenderContext {
    pub color_scatter_column: Option<String>,
    pub transparency: f64,
    pub normal_thickness: f64,
    pub highlight_thickness: f64,
}

impl Default for FlDataFrameViewRenderContext {
    fn default() -> Self {
        Self {
            color_scatter_column: None,
            transparency: 0.5,
            normal_thickness: 1.0,
            highlight_thickness: 3.0,
        }
    }
}

pub struct FlDataFrameViewRender {
    pub dataframe_view: FlDataFrameView,
    pub column: String,
    render_context: Arc<Mutex<FlDataFrameViewRenderContext>>,
}

impl DataRender for FlDataFrameViewRender {
    fn id(&self) -> Id {
        Id::new("fl_data_frame_view")
            .with(self.dataframe_view.id)
            .with(self.column.as_str())
    }

    fn render(
        &self,
        painter: &mut Painter,
        state: &VisualizeState,
        size: Vec2,
    ) -> anyhow::Result<()> {
        let computed_dataframe = self.dataframe_view.table.computed_dataframe();
        let target_series = self
            .dataframe_view
            .table
            .computed_dataframe()
            .column(self.column.as_str())
            .unwrap()
            .clone();
        let color_series = self
            .render_context
            .lock()
            .unwrap()
            .color_scatter_column
            .as_ref()
            .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone());
        let highlight = {
            let state = self.dataframe_view.table.state();
            let highlight = state.highlight.lock().unwrap();
            computed_dataframe
                .column("__FleximRowId")
                .unwrap()
                .iter()
                .map(|v| {
                    let index = v.extract::<u32>().unwrap() as u64;
                    highlight.contains(&index)
                })
                .collect_vec()
        };
        let rectangles: anyhow::Result<Vec<FlDataFrameRectangle>> =
            target_series.iter().map(TryFrom::try_from).collect();
        let colors =
            color_series.map(|color_series| color_series.iter().map(|v| pallet(v)).collect_vec());

        for (i, rect) in rectangles.unwrap().iter().enumerate() {
            let color = if let Some(colors) = &colors {
                colors[i]
            } else {
                Color32::RED
            };
            let transparent = self.render_context.lock().unwrap().transparency;
            let alpha = 1.0 - transparent;
            let color_array = color
                .to_normalized_gamma_f32()
                .into_iter()
                .map(|c| ((c as f64 * alpha) * 255.0) as u8)
                .collect_vec();
            let color = Color32::from_rgba_premultiplied(
                color_array[0],
                color_array[1],
                color_array[2],
                color_array[3],
            );
            painter.rect_stroke(
                Rect::from_min_max(
                    painter.clip_rect().min
                        + (Vec2::new(rect.x1 as f32, rect.y1 as f32) * state.scale as f32
                            + state.shift),
                    painter.clip_rect().min
                        + (Vec2::new(rect.x2 as f32, rect.y2 as f32) * state.scale as f32
                            + state.shift),
                ),
                0.0,
                Stroke::new(
                    if highlight[i] {
                        self.render_context.lock().unwrap().highlight_thickness
                    } else {
                        self.render_context.lock().unwrap().normal_thickness
                    } as f32,
                    color,
                ),
            );
        }
        Ok(())
    }

    fn config_panel(&self, ui: &mut Ui) {
        ui.label("FlDataFrameView");
        CollapsingHeader::new("Config").show(ui, |ui| {
            let mut render_context = self.render_context.lock().unwrap();
            ui.horizontal(|ui| {
                ui.label("Color Scatter Column");
                let mut columns = self.dataframe_view.table.dataframe.value.get_column_names();
                let columns = columns
                    .into_iter()
                    .filter(|c| c != &self.column)
                    .collect_vec();
                ComboBox::from_label("")
                    .selected_text(
                        render_context
                            .color_scatter_column
                            .as_ref()
                            .map(String::as_str)
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut render_context.color_scatter_column, None, "");
                        for column in columns {
                            ui.selectable_value(
                                &mut render_context.color_scatter_column,
                                Some(column.to_string()),
                                column,
                            );
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Transparency");
                Slider::new(&mut render_context.transparency, 0.0..=1.0).ui(ui);
            });
            ui.horizontal(|ui| {
                ui.label("Highlight Thickness");
                Slider::new(&mut render_context.highlight_thickness, 0.0..=10.0).ui(ui);
            });
            ui.horizontal(|ui| {
                ui.label("Normal Thickness");
                Slider::new(&mut render_context.normal_thickness, 0.0..=10.0).ui(ui);
            });
        });
    }

    fn size(&self) -> Vec2 {
        self.dataframe_view.size
    }
}

impl FlDataFrameViewRender {
    pub fn new(dataframe_view: FlDataFrameView, column: String) -> Self {
        Self {
            dataframe_view,
            column,
            render_context: Arc::new(Mutex::new(FlDataFrameViewRenderContext::default())),
        }
    }

    pub fn id(&self) -> Id {
        let df_string = self.dataframe_view.table.computed_dataframe().to_string();

        Id::new("fl_data_frame_view")
            .with(self.dataframe_view.id)
            .with(self.column.as_str())
            .with(df_string)
            .with(
                self.render_context
                    .lock()
                    .unwrap()
                    .color_scatter_column
                    .clone(),
            )
    }
}

pub fn visualize(
    ui: &mut Ui,
    visualize_state: &mut VisualizeState,
    _name: &str,
    render: &dyn DataRender,
) -> Response {
    ui.centered_and_justified(|ui| {
        let size = render.size();
        let size = size * visualize_state.scale as f32;
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        render.render(&mut painter, visualize_state, size).unwrap();

        if response.dragged() {
            visualize_state.shift += response.drag_delta();
            log::debug!("dragged {:?}", visualize_state.shift);
        }

        response
    })
    .response
}

pub fn stack_visualize(
    ui: &mut Ui,
    visualize_state: &mut VisualizeState,
    stack: &Vec<Arc<dyn DataRender>>,
) -> Response {
    assert_ne!(stack.len(), 0);
    ui.centered_and_justified(|ui| {
        let stack_top = stack.first().unwrap();
        let size = stack_top.size();
        let size = size * visualize_state.scale as f32;
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        stack_top
            .render(&mut painter, visualize_state, size)
            .unwrap();
        for (i, render) in stack.iter().enumerate().skip(1) {
            render.render(&mut painter, visualize_state, size).unwrap();
        }

        if response.dragged() {
            visualize_state.shift += response.drag_delta();
            log::debug!("dragged {:?}", visualize_state.shift);
        }

        response
    })
    .response

    // let stack = stack
    //     .iter()
    //     .map(|s| (s.render(ui).into_iter().map(move |i| (i, s.transparent()))))
    //     .flatten()
    //     .collect_vec();
    //
    // let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click());
    // let (v, _) = &stack[0];
    // let image = egui::Image::from_bytes(format!("bytes://{}.png", v.id), v.value.clone());
    // let image = image.uv(visualize_state.uv_rect());
    // let image = image.sense(Sense::drag());
    // image
    //     .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
    //     .unwrap();
    // let response = ui.add(image);
    // let rect = response.rect;
    // let mut last_image = None;
    // for (_i, (image, transparent)) in stack.iter().enumerate().skip(1) {
    //     if let Some(image) = last_image {
    //         ui.put(rect, image);
    //     }
    //     let image = Image::from_bytes(format!("bytes://{}.png", image.id), image.value.clone());
    //     let image = image.uv(visualize_state.uv_rect());
    //     let alpha = (255.0 * (1.0 - transparent)) as u8;
    //     let image = image.bg_fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 0));
    //     let image = image.tint(dbg!(egui::Color32::from_rgba_premultiplied(
    //         alpha, alpha, alpha, alpha
    //     )));
    //     let _texture = image
    //         .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
    //         .unwrap();
    //
    //     last_image = Some(image);
    // }
    // if let Some(image) = last_image {
    //     let image = image.sense(Sense::drag());
    //     ui.put(rect, image)
    // } else {
    //     response
    // }
}

fn draw_image(
    painter: &mut Painter,
    image: &Image,
    shift: Vec2,
    size: Vec2,
    tint_color: Color32,
) -> anyhow::Result<()> {
    match image
        .load_for_size(painter.ctx(), Vec2::new(512.0, 512.0))
        .context("load image")?
    {
        TexturePoll::Ready { texture } => painter.image(
            texture.id,
            Rect::from_min_size(painter.clip_rect().min + shift, size),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1.0, 1.0)),
            tint_color,
        ),
        TexturePoll::Pending { .. } => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
