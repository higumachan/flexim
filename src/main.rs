use egui_extras::install_image_loaders;
use egui_tiles::SimplificationOptions;
use flexim_data_type::FlImage;
use flexim_data_visualize::DataVisualize;
use std::sync::Arc;

struct Pane {
    nr: usize,
    content: Arc<dyn DataVisualize>,
}

struct TreeBehavior {}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        format!("Pane {}", pane.nr).into()
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        // Give each pane a unique color:
        pane.content.visualize("main_image", ui)
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
            let mut behavior = TreeBehavior {};
            tree.ui(&mut behavior, ui);
        });
    })
}

fn create_tree() -> egui_tiles::Tree<Pane> {
    let mut next_view_nr = 0;
    let mut gen_pane = |image: Arc<FlImage>| {
        let pane = Pane {
            nr: next_view_nr,
            content: image,
        };
        next_view_nr += 1;
        pane
    };

    let mut tiles = egui_tiles::Tiles::default();

    let mut tabs = vec![];
    tabs.push({
        let image1 = Arc::new(FlImage::new(
            include_bytes!("../assets/flexim-logo-1.png").to_vec(),
        ));
        let children = (0..2)
            .map(|_| tiles.insert_pane(gen_pane(image1.clone())))
            .collect();
        tiles.insert_horizontal_tile(children)
    });

    let root = tiles.insert_tab_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
}
