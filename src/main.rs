use eframe::emath::Rect;
use egui::ahash::{HashMap, HashMapExt, HashSet};
use egui::emath::RectTransform;
use egui::load::DefaultTextureLoader;
use egui::{Align, Button, DragValue, Grid, Id, Layout, Pos2, ScrollArea, Ui, Vec2, Widget};
use egui_extras::install_image_loaders;
use egui_tiles::{Container, ContainerKind, SimplificationOptions, Tile, TileId, Tree, UiResponse};
use flexim_data_type::{FlData, FlImage, FlTensor2D};
use flexim_data_visualize::visualize::{
    into_visualize, stack_visualize, visualize, DataRender, FlImageRender, FlTensor2DRender,
    VisualizeState,
};
use itertools::Itertools;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Pointer};
use std::sync::Arc;

struct Pane {
    name: String,
    content: Arc<dyn DataRender>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StackId(u64);

#[derive(Clone)]
struct StackTab {
    id: Id,
    contents: Vec<(String, Arc<dyn DataRender>)>,
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

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        tile_id: TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        // スタックタブの場合はデータを重ねて可視化する
        let id = if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
            stack_tab.id
        } else {
            tile_id.egui_id(ui.id())
        };
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
                    visualize(ui, &mut state, &pane.name, pane.content.as_ref())
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
        ],
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
    })
}

fn left_panel(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    ScrollArea::vertical()
        .max_height(ui.available_height() / 2.0)
        .vscroll(true)
        .drag_to_scroll(true)
        .enable_scrolling(true)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.label("Data");
            for d in &app.data {
                ui.with_layout(
                    Layout::left_to_right(Align::Min).with_main_align(Align::Max),
                    |ui| {
                        ui.label(&d.name);

                        if ui.button("+").clicked() {
                            insert_root_tile(&mut app.tree, d);
                        }
                    },
                );
            }
        });
    ui.separator();
}

fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut next_view_nr = 0;
    let mut gen_pane = |name: String, image: Arc<dyn DataRender>| {
        let pane = Pane {
            name,
            content: image,
        };
        next_view_nr += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push({
        let image1 = Arc::new(FlImageRender::new(FlImage::new(
            include_bytes!("../assets/flexim-logo-1.png").to_vec(),
        )));
        let image2 = Arc::new(FlImageRender::new(FlImage::new(
            include_bytes!("../assets/tall.png").to_vec(),
        )));
        let tensor = Arc::new(FlTensor2DRender::new(FlTensor2D::new(
            Array2::from_shape_fn((512, 512), |(y, x)| {
                // center peak gauss
                let x = (x as f64 - 256.0) / 100.0;
                let y = (y as f64 - 256.0) / 100.0;
                (-(x * x + y * y) / 2.0).exp()
            }),
        )));
        let mut children = vec![];
        children.push(tiles.insert_pane(gen_pane("image".to_string(), image1.clone())));
        children.push(tiles.insert_pane(gen_pane("tall".to_string(), image2.clone())));
        // children.push(tiles.insert_pane(gen_pane("tensor".to_string(), tensor)));

        tiles.insert_horizontal_tile(children)
    });

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
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
                    && child_tiles
                        .iter()
                        .all(|(_, t)| t.map(|t| t.is_pane()).unwrap_or(false))
                {
                    for (id, _) in child_tiles.iter() {
                        for (_, t) in child_tiles.iter() {
                            match t {
                                Some(Tile::Pane(p)) => {
                                    stack_tabs
                                        .entry(*id)
                                        .and_modify(|m: &mut Vec<(String, Arc<dyn DataRender>)>| {
                                            m.push((p.name.clone(), p.content.clone()))
                                        })
                                        .or_insert(vec![(p.name.clone(), p.content.clone())]);
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

fn insert_root_tile(tree: &mut Tree<Pane>, data: &ManagedData) {
    dbg!("inserted");
    let tile_id = tree.tiles.insert_pane(Pane {
        name: data.name.clone(),
        content: into_visualize(data.data.as_ref()).unwrap(),
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
    }
}
