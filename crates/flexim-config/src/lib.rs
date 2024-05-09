// const ZOOM_UPPER_LIMIT: f32 = 20.0;
// const ZOOM_LOWER_LIMIT: f32 = 0.01;
// const ZOOM_SPEED: f32 = 1.0;
// const SCROLL_SPEED: f32 = 1.0;

use egui::{Context, Id, Ui, Window};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub zoom_upper_limit: f32,
    pub zoom_lower_limit: f32,
    pub zoom_speed: f32,
    pub scroll_speed: f32,
}

impl Config {
    pub fn get_global(ui: &mut Ui) -> Self {
        let config = ui
            .ctx()
            .data_mut(|writer| writer.get_persisted::<Config>(Id::new(CONFIG_WINDOW_ID)))
            .unwrap_or_default();
        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            zoom_upper_limit: 20.0,
            zoom_lower_limit: 0.1,
            zoom_speed: 1.0,
            scroll_speed: 1.0,
        }
    }
}

const CONFIG_WINDOW_ID: &str = "global_config_window";
const ENABLE_CONFIG_WINDOW_ID: &str = "enable config window";

#[derive(Debug, Clone, Default)]
pub struct ConfigWindow {}

impl ConfigWindow {
    pub fn open(ctx: &Context) {
        ctx.memory_mut(|writer| {
            writer
                .data
                .insert_temp(Id::new("enable config window"), true);
        });
    }

    pub fn show(ctx: &Context) {
        let mut config = ctx
            .data_mut(|writer| writer.get_persisted::<Config>(Id::new(CONFIG_WINDOW_ID)))
            .unwrap_or_default();
        let mut active_window = ctx
            .memory(|writer| {
                writer
                    .data
                    .get_temp::<bool>(Id::new(ENABLE_CONFIG_WINDOW_ID))
            })
            .unwrap_or_default();

        Window::new("Config")
            .open(&mut active_window)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Zoom Upper Limit");
                    ui.add(
                        egui::DragValue::new(&mut config.zoom_upper_limit)
                            .clamp_range(0.0..=100.0)
                            .speed(0.01),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Zoom Lower Limit");
                    ui.add(
                        egui::DragValue::new(&mut config.zoom_lower_limit)
                            .clamp_range(0.0..=config.zoom_upper_limit)
                            .speed(0.01),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Zoom Speed");
                    ui.add(
                        egui::DragValue::new(&mut config.zoom_speed)
                            .clamp_range(0.0..=10.0)
                            .speed(0.01),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Scroll Speed");
                    ui.add(
                        egui::DragValue::new(&mut config.scroll_speed)
                            .clamp_range(0.0..=10.0)
                            .speed(0.01),
                    );
                });
            });

        ctx.data_mut(|writer| {
            writer.insert_persisted(Id::new(CONFIG_WINDOW_ID), config);
            writer.insert_temp(Id::new(ENABLE_CONFIG_WINDOW_ID), active_window);
        });
    }
}
