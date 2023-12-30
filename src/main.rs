use egui::ahash::{HashMap, HashMapExt, HashSet};
use egui_extras::install_image_loaders;
use egui_tiles::{Container, ContainerKind, SimplificationOptions, Tile, TileId, Tree, UiResponse};
use flexim_data_type::{FlImage, FlTensor2D};
use flexim_data_visualize::visualize::{
    stack_visualize, DataVisualize, FlImageVisualize, FlTensor2DVisualize, StackVisualize,
};
use itertools::Itertools;
use ndarray::Array2;
use std::fmt::{Debug, Formatter, Pointer};
use std::sync::Arc;

struct Pane {
    nr: usize,
    name: String,
    content: Arc<dyn DataVisualize>,
}

#[derive(Clone)]
struct StackTab {
    contents: Vec<Arc<dyn DataVisualize>>,
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
        format!("Pane {}", pane.nr).into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        // スタックタブの場合はデータを重ねて可視化する
        dbg!(tile_id);
        if let Some(stack_tab) = self.stack_tabs.get(&tile_id) {
            stack_visualize(pane.name.as_str(), ui, &stack_tab.contents)
        } else {
            pane.content.visualize(&pane.name, ui)
        }
    }

    fn simplification_options(&self) -> SimplificationOptions {
        let mut opt = SimplificationOptions::default();
        opt.all_panes_must_have_tabs = true;
        opt
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut tree = create_tree();

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        install_image_loaders(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut behavior = TreeBehavior {
                stack_tabs: dbg!(collect_stack_tabs(&tree)),
            };
            tree.ui(&mut behavior, ui);
        });
    })
}

fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut next_view_nr = 0;
    let mut gen_pane = |name: String, image: Arc<dyn DataVisualize>| {
        let pane = Pane {
            nr: next_view_nr,
            name,
            content: image,
        };
        next_view_nr += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push({
        let image1 = Arc::new(FlImageVisualize::new(FlImage::new(
            include_bytes!("../assets/flexim-logo-1.png").to_vec(),
        )));
        let tensor = Arc::new(FlTensor2DVisualize::new(
            0,
            FlTensor2D::new(Array2::from_shape_fn((512, 512), |(y, x)| {
                // center peak gauss
                let x = (x as f64 - 256.0) / 100.0;
                let y = (y as f64 - 256.0) / 100.0;
                (-(x * x + y * y) / 2.0).exp()
            })),
        ));
        let stack = Arc::new(StackVisualize::new(vec![image1.clone(), tensor.clone()]));
        let mut children = vec![];
        children.push(tiles.insert_pane(gen_pane("image".to_string(), image1.clone())));
        children.push(tiles.insert_pane(gen_pane("tensor".to_string(), tensor)));
        children.push(tiles.insert_pane(gen_pane("stack".to_string(), stack)));

        tiles.insert_horizontal_tile(children)
    });

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
}

fn collect_stack_tabs(tree: &Tree<Pane>) -> HashMap<TileId, StackTab> {
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
                                        .and_modify(|m: &mut Vec<Arc<dyn DataVisualize>>| {
                                            m.push(p.content.clone())
                                        })
                                        .or_insert(vec![p.content.clone()]);
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

    HashMap::from_iter(
        stack_tabs
            .into_iter()
            .map(|(k, v)| (k, StackTab { contents: v })),
    )
}
