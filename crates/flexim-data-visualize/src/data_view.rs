use crate::visualize::{DataRender, FlDataFrameViewRender};
use egui::{ScrollArea, Ui};
use flexim_data_type::{FlDataFrame, FlDataReference};
use flexim_data_view::{FlDataFrameView, Id};
use flexim_storage::Bag;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub enum DataView {
    FlDataFrameView(FlDataFrameView),
}

impl DataView {
    pub fn id(&self) -> Id {
        match self {
            Self::FlDataFrameView(v) => v.id(),
        }
    }

    pub fn draw(&self, ui: &mut Ui, bag: &Bag) {
        match self {
            Self::FlDataFrameView(v) => v.draw(ui, bag),
        }
    }

    pub fn visualizeable_attributes(&self, bag: &Bag) -> Vec<String> {
        match self {
            Self::FlDataFrameView(v) => v.visualizeable_attributes(bag),
        }
    }

    pub fn create_visualize(&self, attribute: String) -> Arc<DataRender> {
        match self {
            Self::FlDataFrameView(v) => v.create_visualize(attribute),
        }
    }

    pub fn reference(&self) -> FlDataReference {
        match self {
            Self::FlDataFrameView(v) => v.table.data_reference.clone(),
        }
    }
}

pub trait DataViewable {
    fn id(&self) -> Id;
    fn draw(&self, ui: &mut Ui, bag: &Bag);
    fn visualizeable_attributes(&self, bag: &Bag) -> Vec<String>;
    fn create_visualize(&self, attribute: String) -> Arc<DataRender>;
}

impl DataViewable for FlDataFrameView {
    fn id(&self) -> Id {
        self.id
    }

    fn draw(&self, ui: &mut Ui, bag: &Bag) {
        puffin::profile_function!();
        ScrollArea::horizontal()
            .enable_scrolling(true)
            .max_width(ui.available_width())
            .min_scrolled_width(ui.available_width())
            .drag_to_scroll(true)
            .show(ui, |ui| {
                self.table.draw(ui, bag);
            });
    }

    fn visualizeable_attributes(&self, bag: &Bag) -> Vec<String> {
        let dataframe = self.table.dataframe(bag);
        let FlDataFrame {
            value: dataframe,
            special_columns,
            ..
        } = dataframe.as_ref().unwrap().as_ref();
        dataframe
            .fields()
            .iter()
            .filter(|field| {
                special_columns
                    .get(&field.name().to_string())
                    .map_or(false, |v| v.visualizable_attribute())
            })
            .map(|field| field.name().to_string())
            .collect()
    }

    fn create_visualize(&self, attribute: String) -> Arc<DataRender> {
        Arc::new(FlDataFrameViewRender::new(self.clone(), attribute).into())
    }
}
