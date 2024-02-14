mod left_panel;
mod pane;

use std::default::Default;

use crate::pane::{Pane, PaneContent};

use eframe::{run_native, Frame};
use egui::ahash::{HashMap, HashMapExt};

use crate::left_panel::left_panel;
use egui::{Align, Context, DragValue, Id, Layout, Response, Ui, Widget};
use egui_extras::install_image_loaders;
use egui_tiles::{Container, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};
use flexim_connect::grpc::flexim_connect_server::FleximConnectServer;
use flexim_connect::server::FleximConnectServerImpl;
use flexim_data_type::{
    FlDataFrame, FlDataFrameRectangle, FlDataFrameSpecialColumn, FlDataReference, FlDataType,
    FlImage, FlTensor2D, GenerationSelector,
};
use flexim_data_visualize::visualize::{
    stack_visualize, visualize, DataRender, FlImageRender, VisualizeState,
};
use flexim_font::setup_custom_fonts;
use flexim_storage::{Bag, BagId, Storage, StorageQuery};
use itertools::Itertools;
use ndarray::Array2;
use polars::datatypes::StructChunked;
use polars::prelude::{CsvReader, IntoSeries, NamedFrom, SerReader, Series};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::sync::{Arc, RwLock};
use tonic::transport::Server;

const SCROLL_SPEED: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StackId(u64);

pub struct Managed<D> {
    pub tile_id: TileId,
    pub name: String,
    pub data: D,
}

impl<D> Managed<D> {
    pub fn new(tile_id: TileId, name: String, data: D) -> Self {
        Self {
            tile_id,
            name,
            data,
        }
    }
}

#[derive(Clone)]
struct StackTab {
    contents: Vec<Arc<DataRender>>,
}

impl Debug for StackTab {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackTab")
            .field("contents", &self.contents.len())
            .finish()
    }
}

struct TreeBehavior<'a> {
    current_bag: Arc<RwLock<Bag>>,
    stack_tabs: HashMap<TileId, StackTab>,
    current_tile_id: &'a mut Option<TileId>,
}

impl<'a> egui_tiles::Behavior<Pane> for TreeBehavior<'a> {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.name.clone().into()
    }

    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, pane: &mut Pane) -> UiResponse {
        // スタックタブの場合はデータを重ねて可視化する
        let id = if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
            stack_tab
                .contents
                .iter()
                .fold(Id::new("stack_tab"), |id, content| id.with(content.id()))
        } else {
            Id::new("tab").with(tile_id)
        };

        match &pane.content {
            PaneContent::Visualize(content) => {
                let mut state = ui
                    .memory_mut(|mem| mem.data.get_persisted::<VisualizeState>(id))
                    .unwrap_or_default();

                let _response = ui
                    .with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.with_layout(
                            Layout::left_to_right(Align::Min)
                                .with_main_align(Align::Center)
                                .with_main_wrap(true),
                            |ui| {
                                let b = ui.button("-");
                                if b.clicked() {
                                    state.scale -= 0.1;
                                }
                                let dv = DragValue::new(&mut state.scale).speed(0.1).ui(ui);
                                if dv.clicked() {
                                    state.scale = 1.0;
                                }

                                let b = ui.button("+");
                                if b.clicked() {
                                    state.scale += 0.1;
                                }

                                let b = ui.button("💾");
                                if b.clicked() {
                                    let mut file = std::fs::File::create("content.bin").unwrap();
                                    let mut buf_writer = std::io::BufWriter::new(&mut file);

                                    if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
                                        bincode::serialize_into(
                                            &mut buf_writer,
                                            &stack_tab.contents.clone(),
                                        )
                                        .unwrap();
                                    } else {
                                        bincode::serialize_into(
                                            &mut buf_writer,
                                            &vec![content.clone()],
                                        )
                                        .unwrap();
                                    };
                                }
                            },
                        );

                        let response = {
                            let bag = self.current_bag.read().unwrap();
                            if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
                                stack_visualize(ui, &bag, &mut state, &stack_tab.contents)
                            } else {
                                visualize(ui, &bag, &mut state, &pane.name, content.as_ref())
                            }
                        };

                        if response.dragged() {
                            state.shift -= response.drag_delta() / response.rect.size();
                        }
                        if response.hovered() {
                            ui.input(|input| {
                                state.scale += (input.scroll_delta.y * SCROLL_SPEED) as f64;
                            });
                            state.verify();
                            log::debug!("scale {:?}", state.scale);
                        }

                        response
                    })
                    .inner;

                state.verify();
                ui.memory_mut(|mem| mem.data.insert_persisted(id, state));

                UiResponse::None
            }
            PaneContent::DataView(view) => {
                let bag = self.current_bag.read().unwrap();
                view.draw(ui, &bag);
                UiResponse::None
            }
        }
    }

    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn on_tab_button(
        &mut self,
        _tiles: &Tiles<Pane>,
        tile_id: TileId,
        button_response: Response,
    ) -> Response {
        if button_response.clicked() {
            *self.current_tile_id = Some(tile_id);
        }
        button_response
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FlLayout {
    id: Id,
    name: String,
    tree: Tree<Pane>,
}

impl FlLayout {
    fn new(name: String, tree: Tree<Pane>) -> Self {
        let id = Id::new(name.clone()).with(tree.id());
        Self { id, name, tree }
    }
}

struct App {
    pub tree: Tree<Pane>,
    pub storage: Arc<Storage>,
    pub current_bag_id: BagId,
    pub current_tile_id: Option<TileId>,
    pub layouts: Vec<FlLayout>,
    removing_tiles: Vec<TileId>,
    replace_bag_id: Option<BagId>,
    panel_context: HashMap<BagId, Tree<Pane>>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        puffin::profile_scope!("frame");
        {
            egui::SidePanel::left("data viewer").show(ctx, |ui| {
                let bag = self.storage.get_bag(self.current_bag_id).unwrap();
                let bag = bag.read().unwrap();
                left_panel(self, ui, &bag);
            });
            egui::SidePanel::right("visualize viewer").show(ctx, |ui| {
                let bag = self.storage.get_bag(self.current_bag_id).unwrap();
                let bag = bag.read().unwrap();
                right_panel(self, ui, &bag);
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                puffin::profile_scope!("center panel");
                let current_bag = self.storage.get_bag(self.current_bag_id).unwrap();
                let mut behavior = TreeBehavior {
                    current_bag,
                    stack_tabs: collect_stack_tabs(ui, &self.tree),
                    current_tile_id: &mut self.current_tile_id,
                };
                self.tree.ui(&mut behavior, ui);
            });
        }
        end_of_frame(self);
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    let _puffin_server = puffin_http::Server::new(&server_addr);
    eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
    puffin::set_scopes_on(true);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    let storage = Arc::new(Storage::default());
    let bag_id = storage.create_bag("test".to_string());
    storage
        .insert_data(
            bag_id,
            "logo".to_string(),
            FlImage::new(
                include_bytes!("../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(include_bytes!("../assets/tall.png").to_vec(), 512, 512).into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(include_bytes!("../assets/tall.png").to_vec(), 512, 512).into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "gauss".to_string(),
            FlTensor2D::new(Array2::from_shape_fn((512, 512), |(y, x)| {
                // center peak gauss
                let x = (x as f64 - 256.0) / 100.0;
                let y = (y as f64 - 256.0) / 100.0;
                (-(x * x + y * y) / 2.0).exp()
            }))
            .into(),
        )
        .unwrap();
    storage
        .insert_data(bag_id, "tabledata".to_string(), load_sample_data().into())
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "long_tabledata".to_string(),
            load_long_sample_data().into(),
        )
        .unwrap();

    {
        let storage = storage.clone();
        std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let addr = "[::1]:50051".parse().unwrap();
                let server_impl = FleximConnectServerImpl::new(storage);

                Server::builder()
                    .add_service(FleximConnectServer::new(server_impl))
                    .serve(addr)
                    .await
                    .unwrap();
            });
        });
    }

    let _ = storage.create_bag("test2test2test2test2test2".to_string());
    let bag_id = storage.create_bag("test2test2test2test2test2".to_string());
    storage
        .insert_data(
            bag_id,
            "logo".to_string(),
            FlImage::new(include_bytes!("../assets/tall.png").to_vec(), 512, 512).into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(
                include_bytes!("../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tall".to_string(),
            FlImage::new(
                include_bytes!("../assets/flexim-logo-1.png").to_vec(),
                512,
                512,
            )
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "gauss".to_string(),
            FlTensor2D::new(Array2::from_shape_fn((512, 512), |(y, x)| {
                // center peak gauss
                let x = (x as f64 - 256.0) / 100.0;
                let y = (y as f64 - 256.0) / 100.0;
                (-(x * x + y * y) / 2.0).exp()
            }))
            .into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "tabledata".to_string(),
            load_long_sample_data2().into(),
        )
        .unwrap();
    storage
        .insert_data(
            bag_id,
            "long_tabledata".to_string(),
            load_long_sample_data().into(),
        )
        .unwrap();

    let tree = create_tree();
    let app = App {
        tree,
        storage,
        layouts: vec![],
        current_bag_id: bag_id,
        removing_tiles: vec![],
        replace_bag_id: None,
        panel_context: HashMap::new(),
        current_tile_id: None,
    };

    run_native(
        "Flexim",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            install_image_loaders(&cc.egui_ctx);
            Box::new(app)
        }),
    )
}

fn end_of_frame(app: &mut App) {
    for &tile_id in &app.removing_tiles {
        app.tree.tiles.remove(tile_id);
        if app.current_tile_id == Some(tile_id) {
            app.current_tile_id = None;
        }
    }
    app.removing_tiles.clear();
    if let Some(bag_id) = app.replace_bag_id {
        let panel = std::mem::replace(
            &mut app.tree,
            app.panel_context
                .remove(&bag_id)
                .unwrap_or_else(|| Tree::empty(bag_id.into_inner().to_string())),
        );
        app.panel_context.insert(app.current_bag_id, panel);
        app.current_tile_id = None;
        app.current_bag_id = bag_id;
        app.replace_bag_id = None;
    }
}

fn right_panel(app: &mut App, ui: &mut Ui, bag: &Bag) {
    puffin::profile_function!();
    if let Some(tile_id) = app.current_tile_id {
        if let Some(tile) = app.tree.tiles.get(tile_id) {
            if let Tile::Pane(Pane {
                content: PaneContent::Visualize(data),
                ..
            }) = tile
            {
                data.config_panel(ui, bag);
            }
        } else {
            log::warn!("tile not found");
        }
    }
}

fn left_and_right_layout<Ctx, LR, RR>(
    ui: &mut Ui,
    ctx: &mut Ctx,
    left_content: impl FnOnce(&mut Ctx, &mut Ui) -> LR,
    right_content: impl FnOnce(&mut Ctx, &mut Ui) -> RR,
) {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        right_content(ctx, ui);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            left_content(ctx, ui);
        });
    });
}

// TODO(higumachan): 最終的には消す
fn left_and_right_layout_dummy<R>(
    ui: &mut Ui,
    app: &mut App,
    left_content: impl FnOnce(&mut App, &mut Ui) -> R,
    right_content: impl FnOnce(&mut App, &mut Ui) -> R,
) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        left_content(app, ui);
        right_content(app, ui);
    });
}

fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut next_view_nr = 0;
    let mut gen_pane = |name: String, image: Arc<DataRender>| {
        let pane = Pane {
            name,
            content: PaneContent::Visualize(image),
        };
        next_view_nr += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push({
        let image1 = Arc::<DataRender>::new(
            FlImageRender::new(FlDataReference::new(
                "logo".to_string(),
                GenerationSelector::Latest,
                FlDataType::Image,
            ))
            .into(),
        );
        let image2 = Arc::<DataRender>::new(
            FlImageRender::new(FlDataReference::new(
                "tall".to_string(),
                GenerationSelector::Latest,
                FlDataType::Image,
            ))
            .into(),
        );
        let children = vec![
            tiles.insert_pane(gen_pane("image".to_string(), image1.clone())),
            tiles.insert_pane(gen_pane("tall".to_string(), image2.clone())),
        ];

        tiles.insert_horizontal_tile(children)
    });

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("flexim", root, tiles)
}

fn collect_stack_tabs(_ui: &mut Ui, tree: &Tree<Pane>) -> HashMap<TileId, StackTab> {
    let mut stack_tabs = HashMap::new();
    for t in tree.tiles.tiles() {
        if let Tile::Container(Container::Tabs(tabs)) = t {
            // all tab is pane
            let child_tiles = tabs
                .children
                .iter()
                .filter(|&&c| tree.is_visible(c))
                .map(|&c| (c, tree.tiles.get(c)))
                .collect_vec();
            if child_tiles.len() >= 2
                && child_tiles.iter().all(|(_, t)| {
                    t.map(|t| {
                        matches!(
                            t,
                            Tile::Pane(Pane {
                                content: PaneContent::Visualize(_),
                                ..
                            })
                        )
                    })
                    .unwrap_or(false)
                })
            {
                for (id, _) in child_tiles.iter() {
                    for (_, t) in child_tiles.iter() {
                        match t {
                            Some(Tile::Pane(Pane {
                                name: _,
                                content: PaneContent::Visualize(content),
                            })) => {
                                stack_tabs
                                    .entry(*id)
                                    .and_modify(|m: &mut Vec<Arc<DataRender>>| {
                                        m.push(content.clone())
                                    })
                                    .or_insert(vec![content.clone()]);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
    }

    HashMap::from_iter(
        stack_tabs
            .into_iter()
            .map(|(k, v)| (k, StackTab { contents: v })),
    )
}

fn insert_root_tile(tree: &mut Tree<Pane>, name: &str, pane_content: PaneContent) -> TileId {
    let tile_id = tree.tiles.insert_pane(Pane {
        name: name.to_string(),
        content: pane_content,
    });
    if let Some(root) = tree.root() {
        let root = tree.tiles.get_mut(root).unwrap();
        match root {
            Tile::Container(Container::Tabs(tabs)) => {
                tabs.add_child(tile_id);
            }
            Tile::Container(Container::Linear(linear)) => {
                linear.add_child(tile_id);
            }
            Tile::Container(Container::Grid(grid)) => {
                grid.add_child(tile_id);
            }
            _ => unreachable!("root tile is not pane"),
        }
    } else {
        tree.root = Some(tile_id);
    }
    tile_id
}

fn load_sample_data() -> FlDataFrame {
    let data = Vec::from(include_bytes!("../assets/sample.csv"));
    let data = Cursor::new(data);
    let mut df = CsvReader::new(data).has_header(true).finish().unwrap();

    let mut df = df
        .apply("Face", |s| read_rectangle(s, "Face"))
        .unwrap()
        .clone();

    let df = df
        .apply("Segment", |s| read_segment(s, "Segment"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment".to_string(), FlDataFrameSpecialColumn::Segment),
        ]
        .into_iter()
        .collect(),
    )
}

fn load_long_sample_data() -> FlDataFrame {
    let data = Vec::from(include_bytes!("../assets/long_sample.csv"));
    let data = Cursor::new(data);
    let mut df = CsvReader::new(data).has_header(true).finish().unwrap();

    let mut df = df
        .apply("Face", |s| read_rectangle(s, "Face"))
        .unwrap()
        .clone();

    let df = df
        .apply("Segment", |s| read_segment(s, "Segment"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment".to_string(), FlDataFrameSpecialColumn::Segment),
        ]
        .into_iter()
        .collect(),
    )
}

fn load_long_sample_data2() -> FlDataFrame {
    let data = Vec::from(include_bytes!("../assets/long_sample2.csv"));
    let data = Cursor::new(data);
    let mut df = CsvReader::new(data).has_header(true).finish().unwrap();

    let mut df = df
        .apply("Face1", |s| read_rectangle(s, "Face"))
        .unwrap()
        .clone();

    let df = df
        .apply("Segment1", |s| read_segment(s, "Segment"))
        .unwrap()
        .clone();

    FlDataFrame::new(
        df,
        [
            ("Face1".to_string(), FlDataFrameSpecialColumn::Rectangle),
            ("Segment1".to_string(), FlDataFrameSpecialColumn::Segment),
        ]
        .into_iter()
        .collect(),
    )
}

fn read_rectangle(s: &Series, name: &str) -> Series {
    let mut x1 = vec![];
    let mut y1 = vec![];
    let mut x2 = vec![];
    let mut y2 = vec![];
    for s in s.utf8().unwrap().into_iter() {
        let s: Option<&str> = s;
        if let Some(s) = s {
            let t = serde_json::from_str::<FlDataFrameRectangle>(s).unwrap();
            x1.push(Some(t.x1));
            y1.push(Some(t.y1));
            x2.push(Some(t.x2));
            y2.push(Some(t.y2));
        } else {
            x1.push(None);
            y1.push(None);
            x2.push(None);
            y2.push(None);
        }
    }
    let x1 = Series::new("x1", x1);
    let y1 = Series::new("y1", y1);
    let x2 = Series::new("x2", x2);
    let y2 = Series::new("y2", y2);

    StructChunked::new(name, &[x1, y1, x2, y2])
        .unwrap()
        .into_series()
}

fn read_segment(s: &Series, name: &str) -> Series {
    let mut x1 = vec![];
    let mut y1 = vec![];
    let mut x2 = vec![];
    let mut y2 = vec![];
    for s in s.utf8().unwrap().into_iter() {
        let s: Option<&str> = s;
        if let Some(s) = s {
            let t = serde_json::from_str::<FlDataFrameRectangle>(s).unwrap();
            x1.push(Some(t.x1));
            y1.push(Some(t.y1));
            x2.push(Some(t.x2));
            y2.push(Some(t.y2));
        } else {
            x1.push(None);
            y1.push(None);
            x2.push(None);
            y2.push(None);
        }
    }
    let x1 = Series::new("x1", x1);
    let y1 = Series::new("y1", y1);
    let x2 = Series::new("x2", x2);
    let y2 = Series::new("y2", y2);

    StructChunked::new(name, &[x1, y1, x2, y2])
        .unwrap()
        .into_series()
}
