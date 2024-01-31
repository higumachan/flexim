use crate::visualize::{DataRender, FlDataFrameViewRender};
use egui::{ScrollArea, Ui};
use flexim_data_type::FlDataFrameRectangle;
use flexim_data_view::{FlDataFrameView, Id};
use polars::datatypes::DataType;
use std::sync::Arc;

pub trait DataView {
    fn id(&self) -> Id;
    fn draw(&self, ui: &mut Ui);
    fn visualizeable_attributes(&self) -> Vec<String>;
    fn create_visualize(&self, attribute: String) -> Arc<DataRender>;
}

impl DataView for FlDataFrameView {
    fn id(&self) -> Id {
        self.id
    }

    fn draw(&self, ui: &mut Ui) {
        puffin::profile_function!();
        ScrollArea::horizontal()
            .enable_scrolling(true)
            .max_width(ui.available_width())
            .min_scrolled_width(ui.available_width())
            .drag_to_scroll(true)
            .show(ui, |ui| {
                self.table.draw(ui);
            });
    }

    fn visualizeable_attributes(&self) -> Vec<String> {
        let dataframe = &self.table.dataframe.value;

        dataframe
            .fields()
            .iter()
            .filter(|field| match &field.dtype {
                DataType::Struct(inner_field) => FlDataFrameRectangle::validate_fields(inner_field),
                _ => false,
            })
            .map(|field| field.name().to_string())
            .collect()
    }

    fn create_visualize(&self, attribute: String) -> Arc<DataRender> {
        Arc::new(FlDataFrameViewRender::new(self.clone(), attribute).into())
    }
}
