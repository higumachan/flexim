use crate::cache::{Poll, VisualizedImageCache};

use std::io::Cursor;

use egui::{
    CollapsingHeader, Color32, ComboBox, Id, Image, Painter, Pos2, Rect, Response, Sense, Slider,
    Ui, Vec2, Widget,
};

use flexim_data_type::{
    FlDataFrameRectangle, FlDataFrameSegment, FlDataFrameSpecialColumn, FlImage, FlTensor2D,
};
use flexim_data_view::FlDataFrameView;
use image::{DynamicImage, ImageBuffer, Rgb};
use itertools::Itertools;

use crate::pallet::pallet;
use crate::special_columns_visualize::SpecialColumnShape;
use anyhow::Context as _;

use egui::load::TexturePoll;
use flexim_table_widget::cache::DataFramePoll;

use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use unwrap_ord::UnwrapOrd;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRender {
    Image(FlImageRender),
    Tensor2D(FlTensor2DRender),
    DataFrameView(Box<FlDataFrameViewRender>),
}

impl From<FlImageRender> for DataRender {
    fn from(render: FlImageRender) -> Self {
        Self::Image(render)
    }
}

impl From<FlTensor2DRender> for DataRender {
    fn from(render: FlTensor2DRender) -> Self {
        Self::Tensor2D(render)
    }
}

impl From<FlDataFrameViewRender> for DataRender {
    fn from(render: FlDataFrameViewRender) -> Self {
        Self::DataFrameView(Box::new(render))
    }
}

impl DataRender {
    pub fn id(&self) -> Id {
        match self {
            DataRender::Image(render) => render.id(),
            DataRender::Tensor2D(render) => render.id(),
            DataRender::DataFrameView(render) => render.id(),
        }
    }

    pub fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        match self {
            DataRender::Image(render) => render.render(ui, painter, state),
            DataRender::Tensor2D(render) => render.render(ui, painter, state),
            DataRender::DataFrameView(render) => render.render(ui, painter, state),
        }
    }

    pub fn config_panel(&self, ui: &mut Ui) {
        match self {
            DataRender::Image(render) => render.config_panel(ui),
            DataRender::Tensor2D(render) => render.config_panel(ui),
            DataRender::DataFrameView(render) => render.config_panel(ui),
        }
    }
}

pub trait DataRenderable {
    fn id(&self) -> Id;

    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()>;
    fn config_panel(&self, ui: &mut Ui);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlImageRender {
    pub content: Arc<FlImage>,
}

impl FlImageRender {
    pub fn new(content: Arc<FlImage>) -> Self {
        Self { content }
    }
}

impl DataRenderable for FlImageRender {
    fn id(&self) -> Id {
        Id::new("fl_image").with(self.content.id)
    }

    fn render(
        &self,
        _ui: &mut Ui,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        let image = Image::from_bytes(
            format!("bytes://{}.png", self.content.id),
            self.content.value.clone(),
        );

        let size =
            Vec2::new(self.content.width as f32, self.content.height as f32) * state.scale as f32;
        draw_image(painter, &image, state.shift, size, Color32::WHITE)
    }

    fn config_panel(&self, ui: &mut Ui) {
        ui.label("FlImage");
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlTensor2DRenderContext {
    pub transparency: f64,
}

impl Default for FlTensor2DRenderContext {
    fn default() -> Self {
        Self { transparency: 0.5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl DataRenderable for FlTensor2DRender {
    fn id(&self) -> Id {
        Id::new("fl_tensor2d").with(self.content.id)
    }

    fn render(
        &self,
        _ui: &mut Ui,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
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
                        .flat_map(|c| [c.int_r(), c.int_g(), c.int_b()])
                        .collect();
                    let image_buffer: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_vec(
                        content.value.shape()[1] as u32,
                        content.value.shape()[0] as u32,
                        pixels,
                    )
                    .unwrap();
                    let image = DynamicImage::ImageRgb8(image_buffer);

                    let mut image_png_bytes = Vec::new();
                    let mut cursor = Cursor::new(&mut image_png_bytes);
                    image
                        .write_to(&mut cursor, image::ImageOutputFormat::Png)
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

            let size = Vec2::new(
                self.content.value.shape()[1] as f32,
                self.content.value.shape()[0] as f32,
            ) * state.scale as f32;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameViewRenderContext {
    pub color_scatter_column: Option<String>,
    pub label_column: Option<String>,
    pub transparency: f64,
    pub normal_thickness: f64,
    pub highlight_thickness: f64,
}

impl Default for FlDataFrameViewRenderContext {
    fn default() -> Self {
        Self {
            color_scatter_column: None,
            label_column: None,
            transparency: 0.5,
            normal_thickness: 1.0,
            highlight_thickness: 3.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameViewRender {
    pub id: Id,
    pub dataframe_view: FlDataFrameView,
    pub column: String,
    render_context: Arc<Mutex<FlDataFrameViewRenderContext>>,
}

impl DataRenderable for FlDataFrameViewRender {
    fn id(&self) -> Id {
        self.id
    }

    fn render(
        &self,
        ui: &mut Ui,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        if let DataFramePoll::Ready(computed_dataframe) =
            self.dataframe_view.table.computed_dataframe(ui)
        {
            let special_column = self
                .dataframe_view
                .table
                .dataframe
                .special_columns
                .get(&self.column)
                .context("special column not found")?;
            let target_series = computed_dataframe
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
            let indices = computed_dataframe
                .column("__FleximRowId")
                .unwrap()
                .iter()
                .map(|v| v.extract::<u32>().unwrap() as u64)
                .collect_vec();
            let highlight = {
                let state = self.dataframe_view.table.state.lock().unwrap();
                let highlight = &state.highlight;
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
            let shapes: anyhow::Result<Vec<Box<dyn SpecialColumnShape>>> = target_series
                .iter()
                .map(|x| match special_column {
                    FlDataFrameSpecialColumn::Rectangle => {
                        FlDataFrameRectangle::try_from(x.clone())
                            .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>)
                    }
                    FlDataFrameSpecialColumn::Segment => FlDataFrameSegment::try_from(x.clone())
                        .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>),
                })
                .collect();
            let shapes = shapes?;
            let colors =
                color_series.map(|color_series| color_series.iter().map(pallet).collect_vec());
            let label_series = self
                .render_context
                .lock()
                .unwrap()
                .label_column
                .as_ref()
                .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone());
            let labels = label_series
                .map(|label_series| label_series.iter().map(|v| v.to_string()).collect_vec());

            let mut hovered_index = None;
            for (i, shape) in shapes.iter().enumerate() {
                let color = if let Some(colors) = &colors {
                    colors[i]
                } else {
                    Color32::RED
                };
                let label = labels.as_ref().map(|labels| labels[i].as_str());
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
                let thickness = if highlight[i] {
                    self.render_context.lock().unwrap().highlight_thickness
                } else {
                    self.render_context.lock().unwrap().normal_thickness
                } as f32;
                let responses = shape.render(ui, painter, color, thickness, label, state);

                let mut state = self.dataframe_view.table.state.lock().unwrap();
                let mut any_hovered = false;
                for r in responses {
                    if r.hovered() {
                        any_hovered = true;
                    }
                    if r.clicked() {
                        let highlight = &mut state.highlight;
                        let index = indices[i];
                        if highlight.contains(&index) {
                            highlight.remove(&index);
                        } else {
                            highlight.insert(index);
                        }
                    }
                }
                if any_hovered {
                    hovered_index = Some(indices[i]);
                }
            }
            let mut state = self.dataframe_view.table.state.lock().unwrap();
            if let Some(hi) = hovered_index {
                state.selected.replace(hi);
            } else {
                state.selected.take();
            }
            Ok(())
        } else {
            ui.label("Loading...");
            Ok(())
        }
    }

    fn config_panel(&self, ui: &mut Ui) {
        ui.label("FlDataFrameView");
        CollapsingHeader::new("Config").show(ui, |ui| {
            let mut render_context = self.render_context.lock().unwrap();
            ui.horizontal(|ui| {
                ui.label("Color Scatter Column");
                let columns = self.dataframe_view.table.dataframe.value.get_column_names();
                let columns = columns
                    .into_iter()
                    .filter(|c| c != &self.column)
                    .collect_vec();
                ComboBox::from_id_source("Color Scatter Column")
                    .selected_text(render_context.color_scatter_column.as_deref().unwrap_or(""))
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
                ui.label("Label Column");
                let columns = self.dataframe_view.table.dataframe.value.get_column_names();
                let columns = columns
                    .into_iter()
                    .filter(|c| c != &self.column)
                    .collect_vec();
                ComboBox::from_id_source("Label Column")
                    .selected_text(render_context.label_column.as_deref().unwrap_or(""))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut render_context.label_column, None, "");
                        for column in columns {
                            ui.selectable_value(
                                &mut render_context.label_column,
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
}

impl FlDataFrameViewRender {
    pub fn new(dataframe_view: FlDataFrameView, column: String) -> Self {
        Self {
            id: Id::new("fl_data_frame_view_render")
                .with(dataframe_view.id)
                .with(column.as_str()),
            dataframe_view,
            column,
            render_context: Arc::new(Mutex::new(FlDataFrameViewRenderContext::default())),
        }
    }
}

pub fn visualize(
    ui: &mut Ui,
    visualize_state: &mut VisualizeState,
    _name: &str,
    render: &DataRender,
) -> Response {
    ui.centered_and_justified(|ui| {
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        render.render(ui, &mut painter, visualize_state).unwrap();

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
    stack: &Vec<Arc<DataRender>>,
) -> Response {
    assert_ne!(stack.len(), 0);
    ui.centered_and_justified(|ui| {
        let stack_top = stack.first().unwrap();
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        stack_top.render(ui, &mut painter, visualize_state).unwrap();
        for (_i, render) in stack.iter().enumerate().skip(1) {
            render.render(ui, &mut painter, visualize_state).unwrap();
        }

        if response.dragged() {
            visualize_state.shift += response.drag_delta();
            log::debug!("dragged {:?}", visualize_state.shift);
        }

        response
    })
    .response
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
