mod pane;

use crate::pane::{into_pane_content, Pane, PaneContent};
use eframe::emath::Rect;
use egui::ahash::{HashMap, HashMapExt, HashSet};
use egui::emath::RectTransform;
use egui::load::DefaultTextureLoader;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Align, Button, CollapsingHeader, DragValue, Grid, Id, InnerResponse, Layout, Pos2, ScrollArea,
    Ui, Vec2, Widget,
};
use egui_extras::install_image_loaders;
use egui_tiles::{Container, ContainerKind, SimplificationOptions, Tile, TileId, Tree, UiResponse};
use flexim_data_type::{FlData, FlDataFrame, FlDataFrameRectangle, FlImage, FlTensor2D};
use flexim_data_view::{DataViewCreatable, FlDataFrameView};
use flexim_data_visualize::data_view::DataView;
use flexim_data_visualize::data_visualizable::DataVisualizable;
use flexim_data_visualize::visualize::{
    stack_visualize, visualize, DataRender, FlImageRender, FlTensor2DRender, VisualizeState,
};
use itertools::Itertools;
use ndarray::Array2;
use polars::datatypes::StructChunked;
use polars::export::arrow::io::iterator::StreamingIterator;
use polars::prelude::{CsvReader, IntoSeries, NamedFrom, SerReader, Series};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Pointer};
use std::io::Cursor;
use std::sync::Arc;

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
    id: Id,
    contents: Vec<Arc<dyn DataRender>>,
}

impl Debug for StackTab {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackTab")
            .field("contents", &self.contents.len())
            .finish()
    }
}

struct TreeBehavior {
    stack_tabs: HashMap<TileId, StackTab>,
}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        format!("{}", pane.name).into()
    }

    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, pane: &mut Pane) -> UiResponse {
        // スタックタブの場合はデータを重ねて可視化する
        let id = if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
            stack_tab.id
        } else {
            tile_id.egui_id(ui.id())
        };

        match &pane.content {
            PaneContent::Visualize(content) => {
                let mut state = ui
                    .memory_mut(|mem| mem.data.get_persisted::<VisualizeState>(id))
                    .unwrap_or_default();

                let response = ui
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
                            },
                        );

                        let response = if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
                            stack_visualize(ui, &mut state, &stack_tab.contents)
                        } else {
                            visualize(ui, &mut state, &pane.name, content.as_ref())
                        };

                        if response.dragged() {
                            state.shift -= response.drag_delta() / response.rect.size();
                        }

                        response
                    })
                    .inner;

                state.verify();
                ui.memory_mut(|mem| mem.data.insert_persisted(id, state));

                UiResponse::None
            }
            PaneContent::DataView(view) => {
                view.draw(ui);
                UiResponse::None
            }
        }
    }

    fn simplification_options(&self) -> SimplificationOptions {
        let mut opt = SimplificationOptions::default();
        opt.all_panes_must_have_tabs = true;
        opt
    }
}

#[derive(Debug, Clone)]
struct ManagedData {
    pub name: String,
    pub data: Arc<FlData>,
}

impl ManagedData {
    pub fn new(name: String, data: FlData) -> Self {
        Self {
            name,
            data: Arc::new(data),
        }
    }
}

struct App {
    pub tree: Tree<Pane>,
    pub data: Vec<ManagedData>,
    removing_tiles: Vec<TileId>,
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut tree = create_tree();
    let mut app = App {
        tree,
        data: vec![
            ManagedData::new(
                "logo".to_string(),
                FlImage::new(include_bytes!("../assets/flexim-logo-1.png").to_vec()).into(),
            ),
            ManagedData::new(
                "tall".to_string(),
                FlImage::new(include_bytes!("../assets/tall.png").to_vec()).into(),
            ),
            ManagedData::new(
                "gauss".to_string(),
                FlTensor2D::new(Array2::from_shape_fn((512, 512), |(y, x)| {
                    // center peak gauss
                    let x = (x as f64 - 256.0) / 100.0;
                    let y = (y as f64 - 256.0) / 100.0;
                    (-(x * x + y * y) / 2.0).exp()
                }))
                .into(),
            ),
            ManagedData::new("tabledata".to_string(), load_sample_data().into()),
        ],
        removing_tiles: vec![],
    };

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        install_image_loaders(ctx);
        egui::SidePanel::left("data viewer").show(ctx, |ui| {
            left_panel(&mut app, ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut behavior = TreeBehavior {
                stack_tabs: collect_stack_tabs(ui, &app.tree),
            };
            app.tree.ui(&mut behavior, ui);
        });
        end_of_frame(&mut app);
    })
}

fn end_of_frame(app: &mut App) {
    for &tile_id in &app.removing_tiles {
        app.tree.tiles.remove(tile_id);
    }
    app.removing_tiles.clear();
}

fn left_panel(app: &mut App, ui: &mut Ui) {
    data_list_view(app, ui);
    ui.separator();
    data_view_list_view(app, ui);
    ui.separator();
    visualize_list_view(app, ui);
}

fn data_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    ScrollArea::vertical()
        .id_source("data_list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data");
            for d in app.data.clone() {
                left_and_right_layout(
                    ui,
                    app,
                    |_app, ui| {
                        ui.label(&d.name);
                    },
                    |app, ui| {
                        if d.data.is_visualizable() || d.data.data_view_creatable() {
                            if ui.button("+").clicked() {
                                let content = into_pane_content(d.data.as_ref()).unwrap();
                                let tile_id = insert_root_tile(
                                    &mut app.tree,
                                    d.name.as_str(),
                                    content.clone(),
                                );
                            }
                        }
                    },
                )
            }
        });
}

fn data_view_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    let data_views = app
        .tree
        .tiles
        .iter()
        .filter_map(|(tile_id, tile)| match tile {
            Tile::Pane(Pane {
                content: PaneContent::DataView(v),
                name,
            }) => Some(Managed::new(*tile_id, name.clone(), v.clone())),
            _ => None,
        })
        .collect_vec();

    ScrollArea::vertical()
        .id_source("data_view_list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data View");
            for m in &data_views {
                left_and_right_layout(
                    ui,
                    app,
                    |app, ui| {
                        CollapsingHeader::new(&m.name)
                            .id_source(m.tile_id)
                            .show(ui, |ui| {
                                for attr in m.data.visualizeable_attributes() {
                                    left_and_right_layout_dummy(
                                        ui,
                                        app,
                                        |app, ui| {
                                            ui.label(attr.to_string());
                                        },
                                        |app, ui| {
                                            if ui.button("+").clicked() {
                                                let render = m.data.create_visualize(attr.clone());
                                                insert_root_tile(
                                                    &mut app.tree,
                                                    format!("{} viz", m.name).as_str(),
                                                    PaneContent::Visualize(render),
                                                );
                                            }
                                        },
                                    );
                                }
                            });
                    },
                    |app, ui| {
                        let tile_visible = app.tree.tiles.is_visible(m.tile_id);
                        if ui.button(if tile_visible { "👁" } else { "‿" }).clicked() {
                            app.tree.tiles.set_visible(m.tile_id, !tile_visible);
                        }
                        if ui.button("➖").clicked() {
                            app.removing_tiles.push(m.tile_id);
                        }
                    },
                );
            }
        });
}

fn visualize_list_view(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    let visualizes = app
        .tree
        .tiles
        .iter()
        .filter_map(|(tile_id, tile)| match tile {
            Tile::Pane(Pane {
                content: PaneContent::Visualize(v),
                name,
            }) => Some(Managed::new(*tile_id, name.clone(), v.clone())),
            _ => None,
        })
        .collect_vec();

    ScrollArea::vertical()
        .id_source("visualize list")
        .max_height(ui.available_height() / 3.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data Visualize");
            for m in visualizes {
                left_and_right_layout(
                    ui,
                    app,
                    |app, ui| {
                        ui.label(m.name);
                    },
                    |app, ui| {
                        let tile_visible = app.tree.tiles.is_visible(m.tile_id);
                        if ui.button(if tile_visible { "👁" } else { "‿" }).clicked() {
                            app.tree.tiles.set_visible(m.tile_id, !tile_visible);
                        }
                        if ui.button("➖").clicked() {
                            app.removing_tiles.push(m.tile_id);
                        }
                    },
                );
            }
        });
}

fn left_and_right_layout<R>(
    ui: &mut Ui,
    app: &mut App,
    left_content: impl FnOnce(&mut App, &mut Ui) -> R,
    right_content: impl FnOnce(&mut App, &mut Ui) -> R,
) {
    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        left_content(app, ui);
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            right_content(app, ui)
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
    let mut gen_pane = |name: String, image: Arc<dyn DataRender>| {
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
        let image1 = Arc::new(FlImageRender::new(Arc::new(FlImage::new(
            include_bytes!("../assets/flexim-logo-1.png").to_vec(),
        ))));
        let image2 = Arc::new(FlImageRender::new(Arc::new(FlImage::new(
            include_bytes!("../assets/tall.png").to_vec(),
        ))));
        let tensor = Arc::new(FlTensor2DRender::new(Arc::new(FlTensor2D::new(
            Array2::from_shape_fn((512, 512), |(y, x)| {
                // center peak gauss
                let x = (x as f64 - 256.0) / 100.0;
                let y = (y as f64 - 256.0) / 100.0;
                (-(x * x + y * y) / 2.0).exp()
            }),
        ))));
        let mut children = vec![];
        children.push(tiles.insert_pane(gen_pane("image".to_string(), image1.clone())));
        children.push(tiles.insert_pane(gen_pane("tall".to_string(), image2.clone())));

        tiles.insert_horizontal_tile(children)
    });

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("flexim", root, tiles)
}

fn collect_stack_tabs(ui: &mut Ui, tree: &Tree<Pane>) -> HashMap<TileId, StackTab> {
    let mut stack_tabs = HashMap::new();
    for t in tree.tiles.tiles() {
        match t {
            Tile::Container(Container::Tabs(tabs)) => {
                // all tab is pane
                let child_tiles = tabs
                    .children
                    .iter()
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
                                    name,
                                    content: PaneContent::Visualize(content),
                                })) => {
                                    stack_tabs
                                        .entry(*id)
                                        .and_modify(|m: &mut Vec<Arc<dyn DataRender>>| {
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
            _ => {}
        };
    }

    HashMap::from_iter(stack_tabs.into_iter().map(|(k, (v))| {
        (
            k,
            StackTab {
                id: ui.next_auto_id(),
                contents: v,
            },
        )
    }))
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

    let df = df.apply("Face", read_rectangle).unwrap().clone();

    FlDataFrame::new(df)
}

fn read_rectangle(s: &Series) -> Series {
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

    StructChunked::new("Face", &[x1, y1, x2, y2])
        .unwrap()
        .into_series()
}
