#![cfg(target_os = "wasm32")]

use eframe::Frame;
use egui_extras::install_image_loaders;
use flexim_data_visualize::visualize::{stack_visualize, DataRender, VisualizeState};
use flexim_font::setup_custom_fonts;
use kasm_bindgen_futures::spawn_local;
use std::sync::Arc;

static DATA: &[u8] = include_bytes!("../../assets/content.bin");
const SCROLL_SPEED: f32 = 0.01;

struct App {
    contents: Vec<Arc<DataRender>>,
    state: VisualizeState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, contents: Vec<Arc<DataRender>>) -> Self {
        // This gives us image support:
        install_image_loaders(&cc.egui_ctx);
        setup_custom_fonts(&cc.egui_ctx);

        Self {
            contents,
            state: VisualizeState::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        log::info!("update");
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
            let response = stack_visualize(ui, &mut self.state, &self.contents);

            if response.dragged() {
                self.state.shift -= response.drag_delta() / response.rect.size();
            }
            if response.hovered() {
                ui.input(|input| {
                    self.state.scale += (input.scroll_delta.y * SCROLL_SPEED) as f64;
                });
                self.state.verify();
                log::debug!("scale {:?}", self.state.scale);
            }
        });
    }
}

fn main() {
    eframe::WebLogger::init(log::LevelFilter::Trace).ok();

    log::info!("Starting web app...");
    let runner = eframe::WebRunner::new();
    let contents = bincode::deserialize::<Vec<Arc<DataRender>>>(DATA).unwrap();
    spawn_local(async move {
        let _ = runner
            .start(
                "app",
                eframe::WebOptions::default(),
                Box::new(|cc| Box::new(App::new(cc, contents))),
            )
            .await;
    });
}
