use egui::Vec2;
use flexim_data_type::{FlData, FlDataFrame};
use flexim_table_widget::FlTable;

use rand::random;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type Id = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlDataFrameView {
    pub id: Id,
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
        matches!(self, FlData::DataFrame(_))
    }
}

fn gen_id() -> Id {
    random()
}
#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
