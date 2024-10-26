mod initial_data;
mod left_panel;

use std::default::Default;

use eframe::{run_native, Frame};
use egui::ahash::{HashMap, HashMapExt};

use crate::initial_data::initial_data;
use crate::left_panel::left_panel;
use egui::{Context, Id, Response, Ui, ViewportCommand};
use egui_extras::install_image_loaders;
use egui_tiles::{Container, SimplificationOptions, Tile, TileId, Tiles, Tree, UiResponse};
use flexim_config::ConfigWindow;
use flexim_connect::grpc::flexim_connect_server::FleximConnectServer;
use flexim_connect::server::FleximConnectServerImpl;
use flexim_data_type::{
    FlDataFrame, FlDataFrameColor, FlDataFrameRectangle, FlDataFrameSpecialColumn, FlDataReference,
    FlDataType, FlImage, FlObject, FlTensor2D, GenerationSelector,
};
use flexim_data_visualize::visualize::{DataRender, FlImageRender, VisualizeState};
use flexim_font::setup_custom_fonts;
use flexim_layout::pane::{Pane, PaneContent};
use flexim_layout::FlLayout;
use flexim_storage::{Bag, BagId, Storage, StorageQuery};
use itertools::Itertools;
use ndarray::Array2;
use polars::datatypes::StructChunked;
use polars::prelude::{CsvReadOptions, IntoSeries, NamedFrom, SerReader, Series};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Debug, Formatter};
use std::io::Cursor;
use std::sync::{Arc, Mutex, RwLock};
use tonic::transport::Server;

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
                let mut state = VisualizeState::load(ui.ctx(), id);
                let bag = self.current_bag.read().unwrap();
                if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
                    state.show(ui, &bag, &stack_tab.contents);
                } else {
                    state.show(ui, &bag, &[content.clone()]);
                }
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

pub enum UpdateAppEvent {
    ClearBags,
    SwitchBag(BagId),
    InsertTile { title: String, content: PaneContent },
    RemoveTile(TileId),
    UpdateTileVisibility(TileId, bool),
    SwitchLayout(FlLayout),
    RemoveLayout(Id),
    SaveLayout(FlLayout),
}

pub struct App {
    pub tree: Tree<Pane>,
    pub storage: Arc<Storage>,
    pub current_bag_id: Option<BagId>,
    pub current_tile_id: Option<TileId>,
    pub layouts: Vec<FlLayout>,
    events: Arc<Mutex<Vec<UpdateAppEvent>>>,
    panel_context: HashMap<BagId, Tree<Pane>>,
}

impl App {
    pub fn send_event(&self, event: UpdateAppEvent) {
        self.events.lock().unwrap().push(event);
    }
    pub fn current_bag(&self) -> Option<Arc<RwLock<Bag>>> {
        self.storage.get_bag(self.current_bag_id?).ok()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        puffin::profile_scope!("frame");
        {
            egui::SidePanel::left("data viewer").show(ctx, |ui| {
                left_panel(self, ui);
            });
            egui::SidePanel::right("visualize viewer").show(ctx, |ui| {
                right_panel(self, ui);
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                puffin::profile_scope!("center panel");
                if let Some(current_bag) = self.current_bag() {
                    let mut behavior = TreeBehavior {
                        current_bag,
                        stack_tabs: collect_stack_tabs(ui, &self.tree),
                        current_tile_id: &mut self.current_tile_id,
                    };
                    self.tree.ui(&mut behavior, ui);
                }
            });
            ConfigWindow::show(ctx)
        }
        end_of_frame(ctx, self);
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
    let bag_id = initial_data(storage.clone());

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

    let tree = create_tree();
    let app = App {
        tree,
        storage,
        layouts: vec![],
        current_bag_id: Some(bag_id),
        panel_context: HashMap::new(),
        current_tile_id: None,
        events: Arc::new(Mutex::new(vec![])),
    };

    run_native(
        "Flexim",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

fn end_of_frame(ctx: &Context, app: &mut App) {
    let mut events = app.events.lock().unwrap();
    for event in events.drain(0..) {
        match event {
            UpdateAppEvent::ClearBags => {
                app.storage.clear_bags();
                app.current_bag_id = None;
            }
            UpdateAppEvent::SwitchBag(new_bag_id) => {
                if let Some(current_bag_id) = app.current_bag_id {
                    if new_bag_id == current_bag_id {
                        return;
                    }

                    let panel = std::mem::replace(
                        &mut app.tree,
                        app.panel_context.remove(&new_bag_id).unwrap_or_else(|| {
                            Tree::empty(current_bag_id.into_inner().to_string())
                        }),
                    );
                    app.panel_context.insert(current_bag_id, panel);
                }

                app.current_bag_id = Some(new_bag_id);

                let bag = app.storage.get_bag(new_bag_id).unwrap();
                let bag = bag.read().unwrap();
                let bag_name = bag.name.as_str();
                let create_at = bag.created_at.format("%Y-%m-%d %H:%M:%S").to_string();

                ctx.send_viewport_cmd(ViewportCommand::Title(format!(
                    "Flexim - {} {}",
                    bag_name, create_at
                )));
            }
            UpdateAppEvent::InsertTile { content, title } => {
                let tile_id = insert_root_tile(&mut app.tree, &title, content);
                app.current_tile_id = Some(tile_id);
            }
            UpdateAppEvent::RemoveTile(tile_id) => {
                app.tree.tiles.remove(tile_id);
                if app.current_tile_id == Some(tile_id) {
                    app.current_tile_id = None;
                }
            }
            UpdateAppEvent::UpdateTileVisibility(tile_id, visible) => {
                app.tree.tiles.set_visible(tile_id, visible);
            }
            UpdateAppEvent::SwitchLayout(layout) => {
                app.tree = layout.tree;
            }
            UpdateAppEvent::RemoveLayout(id) => {
                app.layouts.retain(|l| l.id != id);
            }
            UpdateAppEvent::SaveLayout(layout) => {
                app.layouts.push(layout);
            }
        }
    }
}

fn right_panel(app: &mut App, ui: &mut Ui) {
    puffin::profile_function!();

    if let Some(bag) = app.current_bag() {
        let bag = bag.read().unwrap();
        if let Some(tile_id) = app.current_tile_id {
            if let Some(tile) = app.tree.tiles.get(tile_id) {
                match tile {
                    Tile::Pane(Pane {
                        content: PaneContent::Visualize(data),
                        ..
                    }) => {
                        data.config_panel(ui, &bag);
                    }
                    Tile::Pane(Pane {
                        content: PaneContent::DataView(data),
                        ..
                    }) => {
                        data.config_panel(ui, &bag);
                    }
                    _ => {}
                }
            } else {
                // log::warn!("tile not found");
            }
        }
    } else {
        ui.label("No bag selected");
    }
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
