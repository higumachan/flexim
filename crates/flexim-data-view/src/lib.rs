use egui::Vec2;
use flexim_data_type::{FlData, FlDataFrame, FlDataTrait};
use flexim_table_widget::FlTable;

use rand::random;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FlDataFrameView {
    pub id: usize,
    pub size: Vec2,
    pub table: FlTable,
}

impl FlDataFrameView {
    pub fn new(dataframe: Arc<FlDataFrame>, size: Vec2) -> Self {
        Self {
            id: gen_id(),
            size,
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

    #[test]
    fn it_works() {}
}
