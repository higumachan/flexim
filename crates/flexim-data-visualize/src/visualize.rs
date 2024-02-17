use crate::cache::{Poll, VisualizedImageCache};

use std::io::Cursor;

use egui::{
    CollapsingHeader, Color32, ComboBox, Id, Image, Painter, Pos2, Rect, Response, Sense, Slider,
    Ui, Vec2, Widget,
};

use flexim_data_type::{
    FlData, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSegment, FlDataFrameSpecialColumn,
    FlDataReference, FlImage, FlShapeConvertError,
};
use flexim_data_view::FlDataFrameView;
use image::{DynamicImage, ImageBuffer, Rgb};
use itertools::Itertools;

use crate::pallet::pallet;
use crate::special_columns_visualize::{RenderParameter, SpecialColumnShape};
use anyhow::Context as _;

use egui::load::TexturePoll;
use flexim_table_widget::cache::DataFramePoll;

use flexim_storage::Bag;
use polars::datatypes::DataType;
use polars::prelude::{AnyValue, Field};
use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use unwrap_ord::UnwrapOrd;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizeState {
    pub scale: f32,
    pub shift: Vec2,
}

impl VisualizeState {
    pub fn uv_rect(&self) -> Rect {
        Rect::from_center_size(
            Pos2::new(0.5, 0.5) + self.shift,
            Vec2::new(1.0 / self.scale, 1.0 / self.scale),
        )
    }

    pub fn is_valid(&self) -> bool {
        0.0 <= self.scale
            && self.scale <= 10.0
            && -100000.0 <= self.shift.x
            && self.shift.x <= 100000.0
            && -100000.0 <= self.shift.y
            && self.shift.y <= 100000.0
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
        bag: &Bag,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        match self {
            DataRender::Image(render) => render.render(ui, bag, painter, state),
            DataRender::Tensor2D(render) => render.render(ui, bag, painter, state),
            DataRender::DataFrameView(render) => render.render(ui, bag, painter, state),
        }
    }

    pub fn config_panel(&self, ui: &mut Ui, bag: &Bag) {
        match self {
            DataRender::Image(render) => render.config_panel(ui, bag),
            DataRender::Tensor2D(render) => render.config_panel(ui, bag),
            DataRender::DataFrameView(render) => render.config_panel(ui, bag),
        }
    }
}

pub trait DataRenderable {
    fn id(&self) -> Id;

    fn render(
        &self,
        ui: &mut Ui,
        bag: &Bag,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()>;
    fn config_panel(&self, ui: &mut Ui, bag: &Bag);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlImageRender {
    pub content: FlDataReference,
}

impl FlImageRender {
    pub fn new(content: FlDataReference) -> Self {
        Self { content }
    }
}

impl DataRenderable for FlImageRender {
    fn id(&self) -> Id {
        Id::new("fl_image").with(&self.content)
    }

    fn render(
        &self,
        _ui: &mut Ui,
        bag: &Bag,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        let data = bag.data_by_reference(&self.content)?;

        if let FlData::Image(data) = data {
            let image = Image::from_bytes(format!("bytes://{}.png", data.id), data.value.clone());

            let size = Vec2::new(data.width as f32, data.height as f32) * state.scale;
            draw_image(painter, &image, state.shift, size, Color32::WHITE)
        } else {
            Err(anyhow::anyhow!(
                "mismatched data type expected FlData::Image"
            ))
        }
    }

    fn config_panel(&self, ui: &mut Ui, _bag: &Bag) {
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
    content: FlDataReference,
    context: Arc<Mutex<FlTensor2DRenderContext>>,
}

impl FlTensor2DRender {
    pub fn new(content: FlDataReference) -> Self {
        Self {
            content,
            context: Arc::new(Mutex::new(FlTensor2DRenderContext::default())),
        }
    }
}

impl DataRenderable for FlTensor2DRender {
    fn id(&self) -> Id {
        Id::new("fl_tensor2d").with(&self.content)
    }

    fn render(
        &self,
        _ui: &mut Ui,
        bag: &Bag,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        let data = bag.data_by_reference(&self.content)?;
        if let FlData::Tensor(data) = data {
            let id = Id::new(data.id);
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
                    let content = data.clone();
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
                let image =
                    Image::from_bytes(format!("bytes://{}.png", data.id), image.value.clone());

                let transparency = (self.context.lock().unwrap().transparency * 255.0) as u8;
                let tint_color = Color32::from_rgba_premultiplied(
                    transparency,
                    transparency,
                    transparency,
                    transparency,
                );

                let size = Vec2::new(data.value.shape()[1] as f32, data.value.shape()[0] as f32)
                    * state.scale;
                draw_image(painter, &image, state.shift, size, tint_color)?;
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "mismatched data type expected FlData::Tensor"
            ))
        }
    }

    fn config_panel(&self, ui: &mut Ui, _bag: &Bag) {
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
    pub fill_color_scatter_column: Option<String>,
    pub label_column: Option<String>,
    pub transparency: f64,
    pub fill_transparency: f64,
    pub normal_thickness: f64,
    pub highlight_thickness: f64,
}

impl Default for FlDataFrameViewRenderContext {
    fn default() -> Self {
        Self {
            color_scatter_column: None,
            fill_color_scatter_column: None,
            label_column: None,
            transparency: 0.5,
            fill_transparency: 0.9,
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
        bag: &Bag,
        painter: &mut Painter,
        state: &VisualizeState,
    ) -> anyhow::Result<()> {
        puffin::profile_function!();
        if let Some(DataFramePoll::Ready(computed_dataframe)) =
            self.dataframe_view.table.computed_dataframe(ui, bag)
        {
            let dataframe = self.dataframe_view.table.dataframe(bag)?;
            let special_column = dataframe
                .special_columns
                .get(&self.column)
                .with_context(|| format!("special column not found: {}", self.column))?;
            let target_series = computed_dataframe
                .column(self.column.as_str())
                .unwrap()
                .clone();
            let stroke_color_series = self
                .render_context
                .lock()
                .unwrap()
                .color_scatter_column
                .as_ref()
                .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone());
            let fill_color_series = self
                .render_context
                .lock()
                .unwrap()
                .fill_color_scatter_column
                .as_ref()
                .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone());
            let indices = computed_dataframe
                .column("__FleximRowId")
                .unwrap()
                .iter()
                .map(|v| v.extract::<u32>().unwrap() as u64)
                .collect_vec();

            let highlight = {
                if let Some(state) = self.dataframe_view.table.state(ui, bag) {
                    let state = state.lock().unwrap();
                    let highlight = &state.highlight;
                    Some(
                        computed_dataframe
                            .column("__FleximRowId")
                            .unwrap()
                            .iter()
                            .map(|v| {
                                let index = v.extract::<u32>().unwrap() as u64;
                                highlight.contains(&index)
                            })
                            .collect_vec(),
                    )
                } else {
                    None
                }
            };
            let shapes: Result<Vec<Option<Box<dyn SpecialColumnShape>>>, FlShapeConvertError> =
                target_series
                    .iter()
                    .map(|x| match special_column {
                        FlDataFrameSpecialColumn::Rectangle => {
                            FlDataFrameRectangle::try_from(x.clone())
                                .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>)
                        }
                        FlDataFrameSpecialColumn::Segment => {
                            FlDataFrameSegment::try_from(x.clone())
                                .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>)
                        }
                        _ => Err(FlShapeConvertError::CanNotConvert),
                    })
                    .map(|x| {
                        x.map(Some).or_else(|e| match e {
                            FlShapeConvertError::NullValue => Ok(None),
                            _ => Err(e),
                        })
                    })
                    .collect();
            let shapes = shapes?;
            let stroke_colors = stroke_color_series.map(|color_series| {
                color_series
                    .iter()
                    .map(|value| serise_value_to_color(color_series.field().as_ref(), &value))
                    .collect_vec()
            });
            let fill_colors = fill_color_series.map(|color_series| {
                color_series
                    .iter()
                    .map(|v| serise_value_to_color(color_series.field().as_ref(), &v))
                    .collect_vec()
            });
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
            for (i, shape) in shapes
                .iter()
                .enumerate()
                .filter_map(|(i, x)| Some((i, x.as_ref()?)))
            {
                let color = if let Some(colors) = &stroke_colors {
                    colors[i]
                } else {
                    Color32::RED
                };
                let fill_color = fill_colors.as_ref().map(|colors| colors[i]);

                let label = labels.as_ref().map(|labels| labels[i].as_str());
                let transparent = self.render_context.lock().unwrap().transparency;
                let fill_transparent = self.render_context.lock().unwrap().fill_transparency;

                let thickness = if highlight.as_ref().map_or(false, |h| h[i]) {
                    self.render_context.lock().unwrap().highlight_thickness
                } else {
                    self.render_context.lock().unwrap().normal_thickness
                } as f32;

                let response = shape.render(
                    ui,
                    painter,
                    RenderParameter {
                        stroke_color: calc_transparent_color(color, transparent),
                        stroke_thickness: thickness,
                        label: label.map(|s| s.to_string()),
                        fill_color: fill_color.map(|c| calc_transparent_color(c, fill_transparent)),
                    },
                    state,
                );

                let g = self
                    .dataframe_view
                    .table
                    .state(ui, bag)
                    .context("state not initialized")?;
                let mut state = g.lock().unwrap();
                if let Some(r) = response {
                    if r.hovered() {
                        hovered_index = Some(indices[i]);
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
                    r.on_hover_ui_at_pointer(|ui| {
                        ui.label(format!("index: {}", indices[i]));
                        let dataframe = &self.dataframe_view.table.dataframe(bag).unwrap().value;
                        let row = dataframe.get_row(indices[i] as usize).unwrap();
                        for (c, v) in dataframe.get_column_names().iter().zip(row.0.iter()) {
                            ui.label(format!("{}: {}", c, v));
                        }
                    });
                }
            }
            let g = self
                .dataframe_view
                .table
                .state(ui, bag)
                .context("state not initialized")?;
            let mut state = g.lock().unwrap();
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

    fn config_panel(&self, ui: &mut Ui, bag: &Bag) {
        ui.label("FlDataFrameView");
        CollapsingHeader::new("Config")
            .default_open(true)
            .show(ui, |ui| {
                let mut render_context = self.render_context.lock().unwrap();
                let dataframe = self.dataframe_view.table.dataframe(bag).unwrap();
                let columns = dataframe.value.get_column_names();
                let columns = columns
                    .into_iter()
                    .filter(|c| c != &self.column)
                    .collect_vec();
                ui.horizontal(|ui| {
                    ui.label("Color Scatter Column");

                    ComboBox::from_id_source("Color Scatter Column")
                        .selected_text(render_context.color_scatter_column.as_deref().unwrap_or(""))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut render_context.color_scatter_column, None, "");
                            for &column in &columns {
                                ui.selectable_value(
                                    &mut render_context.color_scatter_column,
                                    Some(column.to_string()),
                                    column,
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Fill Color Scatter Column");
                    ComboBox::from_id_source("Fill Color Scatter Column")
                        .selected_text(
                            render_context
                                .fill_color_scatter_column
                                .as_deref()
                                .unwrap_or(""),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut render_context.fill_color_scatter_column,
                                None,
                                "",
                            );
                            for &column in &columns {
                                ui.selectable_value(
                                    &mut render_context.fill_color_scatter_column,
                                    Some(column.to_string()),
                                    column,
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Label Column");
                    let dataframe = self.dataframe_view.table.dataframe(bag).unwrap();
                    let columns = dataframe.value.get_column_names();
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
                    ui.label("Fill Transparency");
                    Slider::new(&mut render_context.fill_transparency, 0.0..=1.0).ui(ui);
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
    bag: &Bag,
    visualize_state: &mut VisualizeState,
    _name: &str,
    render: &DataRender,
) -> Response {
    ui.centered_and_justified(|ui| {
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        render
            .render(ui, bag, &mut painter, visualize_state)
            .unwrap();

        response
    })
    .response
}

pub fn stack_visualize(
    ui: &mut Ui,
    bag: &Bag,
    visualize_state: &mut VisualizeState,
    stack: &Vec<Arc<DataRender>>,
) -> Response {
    assert_ne!(stack.len(), 0);
    ui.centered_and_justified(|ui| {
        let stack_top = stack.first().unwrap();
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        stack_top
            .render(ui, bag, &mut painter, visualize_state)
            .unwrap();
        for (_i, render) in stack.iter().enumerate().skip(1) {
            render
                .render(ui, bag, &mut painter, visualize_state)
                .unwrap();
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
        TexturePoll::Ready { texture } => {
            painter.image(
                texture.id,
                Rect::from_min_size(painter.clip_rect().min + shift, size),
                Rect::from_min_size(Pos2::ZERO, Vec2::new(1.0, 1.0)),
                tint_color,
            );
        }
        TexturePoll::Pending { .. } => {}
    }
    Ok(())
}

fn calc_transparent_color(color: Color32, transparent: f64) -> Color32 {
    let alpha = 1.0 - transparent;
    let color_array = color
        .to_normalized_gamma_f32()
        .into_iter()
        .map(|c| ((c as f64 * alpha) * 255.0) as u8)
        .collect_vec();
    Color32::from_rgba_premultiplied(
        color_array[0],
        color_array[1],
        color_array[2],
        color_array[3],
    )
}

fn serise_value_to_color(field: &Field, value: &AnyValue) -> Color32 {
    match &field.dtype {
        DataType::Struct(inner_field) => {
            if FlDataFrameColor::validate_fields(inner_field) {
                let color = FlDataFrameColor::try_from(value.clone()).unwrap();
                Color32::from_rgb(color.r as u8, color.g as u8, color.b as u8)
            } else {
                Color32::RED
            }
        }
        _ => pallet(value),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
