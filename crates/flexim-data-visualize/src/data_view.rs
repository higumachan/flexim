use crate::visualize::{DataRender, FlDataFrameViewRender};
use egui::{ScrollArea, Ui};
use flexim_data_type::FlDataFrameRectangle;
use flexim_data_view::FlDataFrameView;
use polars::datatypes::DataType;
use std::sync::Arc;

pub trait DataView {
    fn id(&self) -> usize;
    fn draw(&self, ui: &mut Ui);
    fn visualizeable_attributes(&self) -> Vec<String>;
    fn create_visualize(&self, attribute: String) -> Arc<dyn DataRender>;
}

impl DataView for FlDataFrameView {
    fn id(&self) -> usize {
        self.id
    }

    fn draw(&self, ui: &mut Ui) {
        ScrollArea::horizontal()
            .enable_scrolling(true)
            .show(ui, |ui| {
                self.table.draw(ui);
            });
    }

    fn visualizeable_attributes(&self) -> Vec<String> {
        let dataframe = &self.table.dataframe.value;

        dataframe
            .fields()
            .iter()
            .filter_map(|field| {
                (match &field.dtype {
                    DataType::Struct(inner_field) => {
                        FlDataFrameRectangle::validate_fields(inner_field)
                    }
                    _ => false,
                })
                .then(|| field.name.to_string())
            })
            .collect()
    }

    fn create_visualize(&self, attribute: String) -> Arc<dyn DataRender> {
        Arc::new(FlDataFrameViewRender::new(self.clone(), attribute))
    }
}
