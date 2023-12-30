use egui::{Ui, Vec2};
use egui_tiles::UiResponse;
use flexim_data_type::FlImage;

pub trait DataVisualize {
    fn visualize(&self, name: &str, ui: &mut egui::Ui) -> UiResponse;
}

impl DataVisualize for FlImage {
    fn visualize(&self, name: &str, ui: &mut Ui) -> UiResponse {
        let image = egui::Image::from_bytes(format!("bytes://{}.png", name), self.value.clone());
        image
            .load_for_size(ui.ctx(), Vec2::new(512.0, 512.0))
            .unwrap();
        ui.add(image);
        UiResponse::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
