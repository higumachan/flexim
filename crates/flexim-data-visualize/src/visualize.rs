use crate::cache::{Poll, VisualizedImageCache};

use std::io::Cursor;
use std::ops::Deref;

use egui::{
    Align, Align2, Button, CollapsingHeader, Color32, ComboBox, Context, DragValue, FontId, Id,
    Image, Layout, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape, Slider, Stroke, Ui,
    Vec2, Widget,
};

use flexim_data_type::{
    FlData, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSegment, FlDataFrameSpecialColumn,
    FlDataReference, FlImage, FlShapeConvertError,
};
use flexim_data_view::FlDataFrameView;
use image::{DynamicImage, ImageBuffer, Rgb};
use itertools::Itertools;

use crate::pallet::pallet;
use crate::special_columns_visualize::{EdgeAccent, RenderParameter, SpecialColumnShape};
use anyhow::Context as _;

use egui::load::TexturePoll;
use flexim_table_widget::cache::DataFramePoll;

use enum_iterator::all;
use flexim_config::Config;
use flexim_storage::{Bag, BagId};
use flexim_utility::left_and_right_layout;
use geo::{coord, Closest, ClosestPoint, Coord, EuclideanDistance, Line, Vector2DOps};
use polars::datatypes::DataType;
use polars::export::chrono;
use polars::prelude::{AnyValue, Field};
use scarlet::color::RGBColor;
use scarlet::colormap::ColorMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use unwrap_ord::UnwrapOrd;

const PSEUDO_INFINITE: f32 = 100000.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizeState {
    pub id: Id,
    pub current_scale: f32,
    pub shift: Vec2,
    pub origin: Origin,
}

impl VisualizeState {
    pub fn scale(&self) -> Vec2 {
        let y = match self.origin {
            Origin::TopLeft => 1.0,
            Origin::BottomLeft => -1.0,
        };

        Vec2::new(self.current_scale, self.current_scale * y)
    }

    pub fn absolute_to_screen(&self, pos: Vec2) -> Vec2 {
        let scale = self.scale();
        let pos = pos * scale;
        pos + self.shift
    }

    pub fn screen_to_absolute(&self, pos: Vec2) -> Vec2 {
        let scale = self.scale();
        let pos = pos - self.shift;
        pos / scale
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum Origin {
    #[default]
    TopLeft,
    BottomLeft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InnerState {
    current_scale: f32,
    shift: Vec2,
    origin: Origin,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            current_scale: 1.0,
            shift: Vec2::ZERO,
            origin: Origin::default(),
        }
    }
}

impl VisualizeState {
    pub fn load(ctx: &Context, id: Id) -> Self {
        let inner_state =
            ctx.data_mut(|data| data.get_persisted::<InnerState>(id).unwrap_or_default());
        Self {
            id,
            current_scale: inner_state.current_scale,
            shift: inner_state.shift,
            origin: inner_state.origin,
        }
    }

    fn store(&self, ctx: &Context) {
        ctx.data_mut(|data| {
            data.insert_persisted(
                self.id,
                InnerState {
                    current_scale: self.current_scale,
                    shift: self.shift,
                    origin: self.origin,
                },
            )
        });
    }

    pub fn is_valid(&self, ui: &mut Ui) -> bool {
        let config = Config::get_global(ui);
        config.zoom_lower_limit <= self.current_scale
            && self.current_scale <= config.zoom_upper_limit
            && -PSEUDO_INFINITE <= self.shift.x
            && self.shift.x <= PSEUDO_INFINITE
            && -PSEUDO_INFINITE <= self.shift.y
            && self.shift.y <= PSEUDO_INFINITE
    }

    pub fn show_header(&mut self, ui: &mut Ui) {
        let state = self;
        ui.with_layout(
            Layout::left_to_right(Align::Min)
                .with_main_align(Align::Center)
                .with_main_wrap(true),
            |ui| {
                let b = ui.button("-");
                if b.clicked() {
                    state.current_scale -= 0.1;
                }
                let dv = DragValue::new(&mut state.current_scale).speed(0.1).ui(ui);
                if dv.clicked() {
                    state.current_scale = 1.0;
                }

                let b = ui.button("+");
                if b.clicked() {
                    state.current_scale += 0.1;
                }
                if ui
                    .button(match state.origin {
                        Origin::TopLeft => "左上",
                        Origin::BottomLeft => "左下",
                    })
                    .clicked()
                {
                    state.origin = match state.origin {
                        Origin::TopLeft => Origin::BottomLeft,
                        Origin::BottomLeft => Origin::TopLeft,
                    };
                }
                if ui.button("Fit").clicked() {
                    state.fit_to_content(ui);
                }
            },
        );
    }

    fn fit_to_content(&mut self, ui: &mut Ui) {
        if let Some(rects) = self.get_measurable_rectangles(ui.ctx()) {
            if rects.is_empty() {
                return;
            }

            // Calculate center of gravity from rectangle centers
            let (sum_x, sum_y, total_area) =
                rects
                    .iter()
                    .fold((0.0_f32, 0.0_f32, 0.0_f32), |(sx, sy, ta), rect| {
                        let center_x = (rect.min.x + rect.max.x) * 0.5;
                        let center_y = (rect.min.y + rect.max.y) * 0.5;
                        let area = rect.width() * rect.height();
                        (sx + center_x * area, sy + center_y * area, ta + area)
                    });

            if total_area > 0.0 {
                let center = Vec2::new(sum_x / total_area, sum_y / total_area);

                // Calculate the shift needed to center this point
                let screen_center = ui.available_size() * 0.5;
                self.shift = screen_center - (center * self.current_scale);
                self.store(ui.ctx());
            }
        }
    }

    fn get_measurable_segments(&self, ctx: &Context) -> Option<Vec<Line>> {
        ctx.memory_mut(|memory| {
            if let Some(render) = memory.data.get_temp::<Arc<DataRender>>(self.id) {
                render
                    .measurable_segments(
                        ctx,
                        &Bag {
                            id: BagId::new(0),
                            name: String::new(),
                            created_at: chrono::Utc::now(),
                            data_list: vec![],
                            generation_counter: std::collections::HashMap::new(),
                        },
                    )
                    .ok()
            } else {
                None
            }
        })
    }

    fn get_measurable_rectangles(&self, ctx: &Context) -> Option<Vec<Rect>> {
        let segments = self.get_measurable_segments(ctx)?;
        if segments.is_empty() {
            return None;
        }

        // Group segments into rectangles by finding connected segments that form right angles
        let mut rectangles = Vec::new();
        let mut used_segments = vec![false; segments.len()];

        for i in 0..segments.len() {
            if used_segments[i] {
                continue;
            }

            // Find segments that share endpoints and form right angles
            let mut rect_segments = Vec::new();
            let mut current_segment = i;
            let mut found_rect = false;

            for _ in 0..4 {
                if used_segments[current_segment] {
                    break;
                }
                rect_segments.push(current_segment);
                used_segments[current_segment] = true;

                // Find next connected segment
                let current = &segments[current_segment];
                let next_segment = segments
                    .iter()
                    .enumerate()
                    .find(|&(j, segment)| {
                        !used_segments[j]
                            && ((current.end.x == segment.start.x
                                && current.end.y == segment.start.y)
                                || (current.end.x == segment.end.x
                                    && current.end.y == segment.end.y))
                    })
                    .map(|(j, _)| j);

                if let Some(next) = next_segment {
                    current_segment = next;
                } else {
                    break;
                }

                if rect_segments.len() == 4 {
                    found_rect = true;
                    break;
                }
            }

            if found_rect {
                // Convert four segments to Rect
                let (min_x, min_y, max_x, max_y) = rect_segments.iter().fold(
                    (f32::INFINITY, f32::INFINITY, -f32::INFINITY, -f32::INFINITY),
                    |(min_x, min_y, max_x, max_y), &seg_idx| {
                        let segment = &segments[seg_idx];
                        let start_x = segment.start.x;
                        let start_y = segment.start.y;
                        let end_x = segment.end.x;
                        let end_y = segment.end.y;
                        (
                            min_x.min(start_x).min(end_x),
                            min_y.min(start_y).min(end_y),
                            max_x.max(start_x).max(end_x),
                            max_y.max(start_y).max(end_y),
                        )
                    },
                );

                rectangles.push(Rect::from_min_max(
                    Pos2::new(min_x, min_y),
                    Pos2::new(max_x, max_y),
                ));
            }
        }

        if rectangles.is_empty() {
            None
        } else {
            Some(rectangles)
        }
    }

    pub fn show(&mut self, ui: &mut Ui, bag: &Bag, contents: &[Arc<DataRender>]) {
        let old_state = self.clone();
        let config = Config::get_global(ui);

        self.show_header(ui);

        let _response = ui
            .with_layout(Layout::top_down(Align::Min), |ui| {
                let response = {
                    if contents.len() > 1 {
                        stack_visualize(ui, bag, self, contents)
                    } else {
                        visualize(ui, bag, self, contents[0].as_ref())
                    }
                };

                if response.dragged_by(PointerButton::Middle) {
                    self.shift += response.drag_delta();
                }

                if let Some(hover_pos) = response.hover_pos() {
                    let hover_pos = hover_pos - response.rect.min;
                    ui.input(|input| {
                        // スクロール関係
                        {
                            let dy = input.raw_scroll_delta.y;
                            let dx = input.raw_scroll_delta.x;
                            self.shift += egui::vec2(dx, dy) * config.scroll_speed;
                        }
                        // ズーム関係
                        {
                            // https://chat.openai.com/share/e/c46c2795-a9e4-4f23-b04c-fa0b0e8ab818
                            let scale = input.zoom_delta() * config.zoom_speed;
                            let pos = hover_pos;
                            self.current_scale *= scale;
                            self.shift = self.shift * scale
                                + egui::vec2(-scale * pos.x + pos.x, -scale * pos.y + pos.y);
                        }
                    });
                }

                response
            })
            .inner;
        if !self.is_valid(ui) {
            *self = old_state;
        }
        self.store(ui.ctx());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRender {
    Image(FlImageRender),
    Tensor2D(FlTensor2DRender),
    DataFrameView(Box<FlDataFrameViewRender>),
}

impl DataRender {
    pub fn reference(&self) -> FlDataReference {
        match self {
            DataRender::Image(render) => render.content.clone(),
            DataRender::Tensor2D(render) => render.content.clone(),
            DataRender::DataFrameView(render) => render.dataframe_view.table.data_reference.clone(),
        }
    }

    /// 検査可能な線分を返す
    pub fn measurable_segments(&self, ctx: &Context, bag: &Bag) -> anyhow::Result<Vec<Line>> {
        match self {
            DataRender::Image(render) => render.measurable_segments(ctx, bag),
            DataRender::Tensor2D(render) => render.measurable_segments(ctx, bag),
            DataRender::DataFrameView(render) => render.measurable_segments(ctx, bag),
        }
    }
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

    fn measurable_segments(&self, ctx: &Context, bag: &Bag) -> anyhow::Result<Vec<Line>>;
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

            let size = Vec2::new(data.width as f32, data.height as f32) * state.scale();
            draw_image(painter, &image, state.shift, size, Color32::WHITE)
        } else {
            Err(anyhow::anyhow!(
                "mismatched data type expected FlData::Image"
            ))
        }
    }

    fn measurable_segments(&self, _ctx: &Context, bag: &Bag) -> anyhow::Result<Vec<Line>> {
        let data = bag.data_by_reference(&self.content)?;

        if let FlData::Image(data) = data {
            let size = (data.width as f64, data.height as f64);
            Ok(vec![
                Line::new(coord!(x: 0.0, y: 0.0), coord!(x: size.0, y: 0.0)),
                Line::new(coord!(x: 0.0, y: 0.0), coord!(x: 0.0, y: size.1)),
                Line::new(coord!(x: size.0, y: 0.0), coord!(x: size.0, y: size.1)),
                Line::new(coord!(x: 0.0, y: size.1), coord!(x: size.0, y: size.1)),
            ])
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
                    * state.scale();

                let offset = data.offset;
                let offset = Vec2::new(offset.1 as f32, offset.0 as f32) * state.scale();

                draw_image(painter, &image, state.shift + offset, size, tint_color)?;
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "mismatched data type expected FlData::Tensor"
            ))
        }
    }

    fn measurable_segments(&self, _ctx: &Context, bag: &Bag) -> anyhow::Result<Vec<Line>> {
        let data = bag
            .data_by_reference(&self.content)?
            .as_tensor()
            .expect("not tensor");

        let offset = (data.offset.1 as f64, data.offset.0 as f64);
        let size = (data.value.shape()[1] as f64, data.value.shape()[0] as f64);

        Ok(vec![
            Line::new(
                coord!(x: offset.0, y: offset.1),
                coord!(x: size.0 + offset.0, y: offset.1),
            ),
            Line::new(
                coord!(x: offset.0, y: offset.1),
                coord!(x: offset.0, y: size.1 + offset.1),
            ),
            Line::new(
                coord!(x: size.0 + offset.0, y: offset.1),
                coord!(x: size.0 + offset.0, y: size.1 + offset.1),
            ),
            Line::new(
                coord!(x: offset.0, y: size.1 + offset.1),
                coord!(x: size.0 + offset.0, y: size.1 + offset.1),
            ),
        ])
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
    #[serde(default = "default_fill_transparency")]
    pub fill_transparency: f64,
    pub normal_thickness: f64,
    pub highlight_thickness: f64,
    #[serde(default)]
    pub edge_accent_start: EdgeAccent,
    #[serde(default)]
    pub edge_accent_end: EdgeAccent,
}

impl FlDataFrameViewRenderContext {
    pub fn verification(&mut self, columns: &[&str]) {
        if matches!(self.color_scatter_column.as_deref(), Some(c) if !columns.contains(&c)) {
            self.color_scatter_column = None;
        }
        if matches!(self.fill_color_scatter_column.as_deref(), Some(c) if !columns.contains(&c)) {
            self.fill_color_scatter_column = None;
        }
        if matches!(self.label_column.as_deref(), Some(c) if !columns.contains(&c)) {
            self.label_column = None;
        }
    }
}

fn default_fill_transparency() -> f64 {
    0.9
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
            edge_accent_start: EdgeAccent::None,
            edge_accent_end: EdgeAccent::None,
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
        let dataframe = self.dataframe_view.table.dataframe(bag)?;
        let special_column = dataframe
            .special_columns
            .get(&self.column)
            .with_context(|| format!("special column not found: {}", self.column))?;

        let computed_dataframe = if let Some(DataFramePoll::Ready(computed_dataframe)) =
            self.dataframe_view.table.computed_dataframe(ui.ctx(), bag)
        {
            computed_dataframe
        } else {
            dataframe
                .value
                .clone()
                .with_row_index("__FleximRowId".into(), None)
                .unwrap()
        };

        let target_series = computed_dataframe
            .column(self.column.as_str())
            .unwrap()
            .as_series()
            .unwrap()
            .clone();
        let stroke_color_series = self
            .render_context
            .lock()
            .unwrap()
            .color_scatter_column
            .as_ref()
            .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone())
            .map(|c| c.as_series().unwrap().clone());
        let fill_color_series = self
            .render_context
            .lock()
            .unwrap()
            .fill_color_scatter_column
            .as_ref()
            .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone())
            .map(|c| c.as_series().unwrap().clone());
        let indices = computed_dataframe
            .column("__FleximRowId")
            .unwrap()
            .as_series()
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
                        .as_series()
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
                    FlDataFrameSpecialColumn::Segment => FlDataFrameSegment::try_from(x.clone())
                        .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>),
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
            .map(|c| computed_dataframe.column(c.as_str()).unwrap().clone())
            .map(|c| c.as_series().unwrap().clone());
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

            let thickness = if highlight.as_ref().is_some_and(|h| h[i]) {
                self.render_context.lock().unwrap().highlight_thickness
            } else {
                self.render_context.lock().unwrap().normal_thickness
            } as f32;

            let edge_accent_start = self.render_context.lock().unwrap().edge_accent_start;
            let edge_accent_end = self.render_context.lock().unwrap().edge_accent_end;
            let response = shape.render(
                ui,
                painter,
                RenderParameter {
                    stroke_color: calc_transparent_color(color, transparent),
                    stroke_thickness: thickness,
                    label: label.map(|s| s.to_string()),
                    fill_color: fill_color.map(|c| calc_transparent_color(c, fill_transparent)),
                    edge_accent_start,
                    edge_accent_end,
                },
                state,
            );

            if let Some(g) = self.dataframe_view.table.state(ui, bag) {
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
        }
        if let Some(g) = self.dataframe_view.table.state(ui, bag) {
            let mut state = g.lock().unwrap();
            if let Some(hi) = hovered_index {
                state.selected.replace(hi);
            } else {
                state.selected.take();
            }
        }
        Ok(())
    }

    fn measurable_segments(&self, ctx: &Context, bag: &Bag) -> anyhow::Result<Vec<Line>> {
        let dataframe = self.dataframe_view.table.dataframe(bag)?;
        let special_column = dataframe
            .special_columns
            .get(&self.column)
            .with_context(|| format!("special column not found: {}", self.column))?;

        let computed_dataframe = if let Some(DataFramePoll::Ready(computed_dataframe)) =
            self.dataframe_view.table.computed_dataframe(ctx, bag)
        {
            computed_dataframe
        } else {
            dataframe
                .value
                .clone()
                .with_row_index("__FleximRowId".into(), None)
                .unwrap()
        };

        let target_series = computed_dataframe
            .column(self.column.as_str())
            .unwrap()
            .as_series()
            .unwrap()
            .clone();

        let shapes: Result<Vec<Option<Box<dyn SpecialColumnShape>>>, FlShapeConvertError> =
            target_series
                .iter()
                .map(|x| match special_column {
                    FlDataFrameSpecialColumn::Rectangle => {
                        FlDataFrameRectangle::try_from(x.clone())
                            .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>)
                    }
                    FlDataFrameSpecialColumn::Segment => FlDataFrameSegment::try_from(x.clone())
                        .map(|x| Box::new(x) as Box<dyn SpecialColumnShape>),
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

        Ok(shapes
            .iter()
            .filter_map(|x| x.as_ref())
            .flat_map(|x| x.measure_segments())
            .collect_vec())
    }

    fn config_panel(&self, ui: &mut Ui, bag: &Bag) {
        left_and_right_layout(
            ui,
            (),
            |_, ui| {
                ui.label("FlDataFrameView");
            },
            |_, ui| {
                if ui.button("📋").on_hover_text("Copy Config").clicked() {
                    ui.memory_mut(|memory| {
                        let render_context = self.render_context.lock().unwrap();
                        memory.data.insert_temp(
                            Id::new("config clipboard"),
                            serde_json::to_string(render_context.deref()).unwrap(),
                        );
                    });
                }
                let enabled = ui.memory(|memory| {
                    memory
                        .data
                        .get_temp::<String>(Id::new("config clipboard"))
                        .is_some()
                });
                let button = Button::new("📲");
                if ui
                    .add_enabled(enabled, button)
                    .on_hover_text("Paste Config")
                    .clicked()
                {
                    ui.memory(|memory| {
                        let render_context_json = memory
                            .data
                            .get_temp::<String>(Id::new("config clipboard"))
                            .unwrap();
                        let mut render_context = self.render_context.lock().unwrap();
                        *render_context = serde_json::from_str(&render_context_json).unwrap();
                    });
                }
            },
        );

        CollapsingHeader::new("Config")
            .default_open(true)
            .show(ui, |ui| {
                let mut render_context = self.render_context.lock().unwrap();
                let dataframe = self.dataframe_view.table.dataframe(bag).unwrap();
                let columns = dataframe.value.get_column_names();
                let columns = columns
                    .into_iter()
                    .map(|c| c.as_str())
                    .filter(|c| *c != self.column)
                    .collect_vec();

                render_context.verification(&columns);

                ui.horizontal(|ui| {
                    ui.label("Color Scatter Column");
                    ComboBox::from_id_salt("Color Scatter Column")
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
                    ComboBox::from_id_salt("Fill Color Scatter Column")
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
                        .map(|c| c.as_str())
                        .filter(|c| c != &self.column)
                        .collect_vec();
                    ComboBox::from_id_salt("Label Column")
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
                    ui.label("Edge Accent");
                    ComboBox::from_id_salt("Edge Accent Start")
                        .selected_text(render_context.edge_accent_start.to_string())
                        .show_ui(ui, |ui| {
                            for edge_accent in all::<EdgeAccent>() {
                                ui.selectable_value(
                                    &mut render_context.edge_accent_start,
                                    edge_accent,
                                    edge_accent.to_string(),
                                );
                            }
                        });
                    ComboBox::from_id_salt("Edge Accent End")
                        .selected_text(render_context.edge_accent_end.to_string())
                        .show_ui(ui, |ui| {
                            for edge_accent in all::<EdgeAccent>() {
                                ui.selectable_value(
                                    &mut render_context.edge_accent_end,
                                    edge_accent,
                                    edge_accent.to_string(),
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

fn visualize(
    ui: &mut Ui,
    bag: &Bag,
    visualize_state: &mut VisualizeState,
    render: &DataRender,
) -> Response {
    let responses = ui.centered_and_justified(|ui| {
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
        render
            .render(ui, bag, &mut painter, visualize_state)
            .unwrap();

        response
    });

    responses.inner
}

fn stack_visualize(
    ui: &mut Ui,
    bag: &Bag,
    visualize_state: &mut VisualizeState,
    stack: &[Arc<DataRender>],
) -> Response {
    assert_ne!(stack.len(), 0);
    let responses = ui.centered_and_justified(|ui| {
        let stack_top = stack.first().unwrap();
        let (response, mut painter) = ui.allocate_painter(ui.available_size(), Sense::drag());

        stack_top
            .render(ui, bag, &mut painter, visualize_state)
            .unwrap();
        let mut segments = vec![];
        segments.extend(stack_top.measurable_segments(ui.ctx(), bag).unwrap());
        for (_i, render) in stack.iter().enumerate().skip(1) {
            render
                .render(ui, bag, &mut painter, visualize_state)
                .unwrap();
            segments.extend(render.measurable_segments(ui.ctx(), bag).unwrap());
        }

        let tile_origin_pos = response
            .hover_pos()
            .map(|pos| pos - response.rect.min.to_vec2());
        let absolute_pos =
            tile_origin_pos.map(|pos| visualize_state.screen_to_absolute(pos.to_vec2()));

        // 検査を行っている部分のコード
        let command = ui.ctx().input(|input| input.modifiers.command_only());
        if let Some(absolute_pos) = absolute_pos {
            if command {
                inspection(
                    &response.rect,
                    visualize_state,
                    &mut painter,
                    ui,
                    &segments,
                    absolute_pos,
                );
            }
        }

        response
    });

    responses.inner
}

/// 検査モードのUIを描画する関数
fn inspection(
    view_rect: &Rect,
    visualize_state: &VisualizeState,
    painter: &mut Painter,
    ui: &mut Ui,
    segments: &[Line],
    absolute_pos: Vec2,
) {
    // TODO(higumachan): リファクタリングしたい
    // Display coordinates in inspection mode
    let text_pos = view_rect.min + visualize_state.absolute_to_screen(absolute_pos);
    let coord_label = format!("x={:.1}, y={:.1}", absolute_pos.x, absolute_pos.y);

    // Layout the text to measure its size
    let galley = painter.layout_no_wrap(coord_label.clone(), FontId::default(), Color32::BLACK);

    // Create and fill the background rectangle
    let text_rect = Rect::from_min_size(text_pos, galley.size());
    painter.rect_filled(
        text_rect.expand(2.0), // Add padding
        0.0,                   // No corner rounding
        Color32::GREEN,        // Match existing green color usage
    );

    // Draw the text on top
    painter.galley(text_pos, galley, Color32::BLACK);

    // minimum distance
    let config = Config::get_global(ui);

    let d = segments
        .iter()
        .map(|segment| {
            segment.euclidean_distance(&geo::Point::new(
                absolute_pos.x as f64,
                absolute_pos.y as f64,
            ))
        })
        .enumerate()
        .min_by_key(|(_, x)| UnwrapOrd(*x));

    if let Some((pos, min_distance)) = d {
        if min_distance < 5.0 {
            draw_segment(
                painter,
                &segments[pos],
                visualize_state,
                view_rect.min.to_vec2(),
                Stroke::new(config.measure_grid_width, Color32::GREEN),
            );
        }
    }

    if ui.input(|input| input.pointer.primary_clicked()) {
        if let Some((pos, min_distance)) = d {
            if min_distance < 5.0 {
                let segment = segments[pos];
                ui.ctx().memory_mut(|memory| {
                    memory
                        .data
                        .insert_temp(Id::new("measure_selected"), segment);
                });
            }
        }
    }
    if let Some(selected_segment) = ui
        .ctx()
        .memory(|memory| memory.data.get_temp::<Line>(Id::new("measure_selected")))
    {
        draw_segment(
            painter,
            &selected_segment,
            visualize_state,
            view_rect.min.to_vec2(),
            Stroke::new(config.measure_grid_width, Color32::GREEN),
        );

        let extend_lines = segments
            .iter()
            .map(|s| {
                let minus_far_point = s.start - s.delta() * PSEUDO_INFINITE.into();
                let plus_far_point = s.end + s.delta() * PSEUDO_INFINITE.into();
                Line::new(minus_far_point, plus_far_point)
            })
            .collect_vec();

        let nearest_extend_line = extend_lines
            .iter()
            .enumerate()
            .filter(|(_, s)| !same_line_parameter(&selected_segment, s))
            .map(|(pos, segment)| {
                (
                    pos,
                    segment.euclidean_distance(&geo::Point::new(
                        absolute_pos.x as f64,
                        absolute_pos.y as f64,
                    )),
                )
            })
            .min_by_key(|(_, x)| UnwrapOrd(*x));

        let snap_segment = nearest_extend_line
            .filter(|(pos, min_distance)| {
                let vec1 = (selected_segment.end - selected_segment.start)
                    .try_normalize()
                    .unwrap_or_default();
                let vec2 = (segments[*pos].delta()).try_normalize().unwrap_or_default();

                vec1.dot_product(vec2).abs() > 0.1
                    && (*min_distance as f32) < config.grid_snap_distance
            })
            .map(|(pos, _)| &extend_lines[pos]);

        // distance absolute pos and segment
        let (distance, to) = if let Some(ss) = snap_segment {
            let from = Pos2::new(ss.start.x as f32, ss.start.y as f32);
            let from = visualize_state.absolute_to_screen(from.to_vec2()).to_pos2()
                + view_rect.min.to_vec2();
            let from = view_rect.clamp(from);
            let to = Pos2::new(ss.end.x as f32, ss.end.y as f32);
            let to = visualize_state.absolute_to_screen(to.to_vec2()).to_pos2()
                + view_rect.min.to_vec2();
            let to = view_rect.clamp(to);
            painter.add(Shape::dashed_line(
                &[from, to],
                Stroke::new(config.measure_grid_width, Color32::GREEN),
                config.measure_grid_width * 3.0,
                config.measure_grid_width * 3.0,
            ));
            let closest = ss.closest_point(&geo::Point::new(
                absolute_pos.x as f64,
                absolute_pos.y as f64,
            ));
            (
                ss.euclidean_distance(&selected_segment),
                match closest {
                    Closest::SinglePoint(p) | Closest::Intersection(p) => {
                        Pos2::new(p.x() as f32, p.y() as f32)
                    }
                    _ => absolute_pos.to_pos2(),
                },
            )
        } else {
            (
                selected_segment.euclidean_distance(&geo::Point::new(
                    absolute_pos.x as f64,
                    absolute_pos.y as f64,
                )),
                absolute_pos.to_pos2(),
            )
        };

        let closest = selected_segment.closest_point(&geo::Point::new(
            absolute_pos.x as f64,
            absolute_pos.y as f64,
        ));

        match closest {
            Closest::SinglePoint(p) | Closest::Intersection(p) => {
                let from = Pos2::new(p.x() as f32, p.y() as f32);
                let from = visualize_state.absolute_to_screen(from.to_vec2()).to_pos2()
                    + view_rect.min.to_vec2();

                let to = visualize_state.absolute_to_screen(to.to_vec2()).to_pos2()
                    + view_rect.min.to_vec2();

                let cliped_from = view_rect.clamp(from);
                let cliped_to = view_rect.clamp(to);

                let center = (cliped_from + cliped_to.to_vec2()) / 2.0;
                painter.line_segment(
                    [from, to],
                    Stroke::new(config.measure_grid_width, Color32::GREEN),
                );
                let rect = painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    format!("{:.2}", distance),
                    FontId::default(),
                    Color32::BLACK,
                );
                painter.rect_filled(rect, 0.0, Color32::GREEN);
                let _rect = painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    format!("{:.2}", distance),
                    FontId::default(),
                    Color32::BLACK,
                );
            }
            _ => {}
        }
    }
}

fn draw_segment(
    painter: &mut Painter,
    segment: &Line,
    visualize_state: &VisualizeState,
    origin: Vec2,
    stroke: Stroke,
) {
    let from = Pos2::new(segment.start.x as f32, segment.start.y as f32);
    let to = Pos2::new(segment.end.x as f32, segment.end.y as f32);
    let from = visualize_state.absolute_to_screen(from.to_vec2()).to_pos2() + origin;
    let to = visualize_state.absolute_to_screen(to.to_vec2()).to_pos2() + origin;

    painter.line_segment([from, to], stroke);
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
                if let Ok(color) = FlDataFrameColor::try_from(value.clone()) {
                    Color32::from_rgb(color.r as u8, color.g as u8, color.b as u8)
                } else {
                    log::warn!("failed to convert to FlDataFrameColor: {:?}", value);
                    Color32::RED
                }
            } else {
                Color32::RED
            }
        }
        _ => pallet(value),
    }
}

fn segment_extened_line_parameter(segment: &Line) -> (Coord, f32) {
    let delta = segment.delta();
    let normal = Coord::from((delta.y, -delta.x))
        .try_normalize()
        .unwrap_or_default();
    let c = -normal.dot_product(segment.start) as f32;
    (normal, c)
}

fn same_line_parameter(segment1: &Line, segment2: &Line) -> bool {
    let (n1, c1) = segment_extened_line_parameter(segment1);
    let (n2, c2) = segment_extened_line_parameter(segment2);

    ((n1.dot_product(n2)).abs() - 1.0) < 0.0001 && (c1.abs() - c2.abs()).abs() < 0.0001
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn same_line_parameter_test(
            x1 in -1000.0..=1000.0,
            y1 in -1000.0..=1000.0,
            x2 in -1000.0..=1000.0,
            y2 in -1000.0..=1000.0,
            t1 in -1000.0..=1000.0,
            t2 in -1000.0..=1000.0,
        ) {
            prop_assume!(x1 != x2 || y1 != y2);
            let segment1: Line = Line::new(coord!(x: x1, y: y1), coord!(x: x2, y: y2));
            let v = segment1.delta();
            let s = segment1.start;

            let segment2 = Line::new(s + v * t1, s + v * t2);

            prop_assume!(t1 != t2);
            prop_assert!(same_line_parameter(&segment1, &segment2));
        }

        #[test]
        fn not_same_line_parameter_test(
            x1 in -1000.0..=1000.0,
            y1 in -1000.0..=1000.0,
            x2 in -1000.0..=1000.0,
            y2 in -1000.0..=1000.0,
            t1 in -1000.0..=1000.0,
            t2 in -1000.0..=1000.0,
        ) {
            prop_assume!(x1 != x2 || y1 != y2);
            let segment1: Line = Line::new(coord!(x: x1, y: y1), coord!(x: x2, y: y2));
            let normal = Coord::from((segment1.delta().y, -segment1.delta().x))
                .try_normalize()
                .unwrap_or_default();
            let v = normal;
            let s = segment1.start;

            let segment2 = Line::new(s + v * t1, s + v * t2);

            prop_assume!(t1 != t2);
            prop_assert!(!same_line_parameter(&segment1, &segment2));
        }
    }

    #[test]
    fn it_works() {}
}
