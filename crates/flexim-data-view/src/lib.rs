use egui::{ScrollArea, Ui};
use flexim_data_type::{FlData, FlDataFrame};
use flexim_table_widget::FlTable;
use rand::random;
use std::sync::Arc;

pub trait DataView {
    fn draw(&self, ui: &mut Ui);
}

#[derive(Debug)]
pub struct FlDataFrameView {
    id: usize,
    table: FlTable,
}

impl DataView for FlDataFrameView {
    fn draw(&self, ui: &mut Ui) {
        ScrollArea::horizontal()
            .enable_scrolling(true)
            .show(ui, |ui| {
                self.table.draw(ui);
            });
    }
}

impl FlDataFrameView {
    pub fn new(dataframe: Arc<FlDataFrame>) -> Self {
        Self {
            id: gen_id(),
            table: FlTable::new(dataframe),
        }
    }
}

pub trait DataViewCreatable {
    fn data_view_creatable(&self) -> bool;
}

impl DataViewCreatable for FlData {
    fn data_view_creatable(&self) -> bool {
        match self {
            FlData::DataFrame(_) => true,
            _ => false,
        }
    }
}

fn gen_id() -> usize {
    random()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
